/** Deterministic window oracle for the host bus state manager (ADR 0018); actual driver,
 * CanIf, ComM, host areas and threads. A dispatcher stops right after the provider left
 * its area: before CanIf is asked for a controller transition, or before a completed
 * mode reaches ComM. The contender (main) completes provider calls meanwhile: none waits,
 * a repeated request finds the transition already registered, and completed modes reach
 * ComM in order. A third window stops a request after CanIf applied its mode and before
 * publication: ComM receives the actual mode, not the applied one. CanIf and ComM enter strict
 * areas, which abort if the provider held one.
 */
#include "Ecu_HostBusSM.h"
#include "ComM.h"
#include "Can.h"
#include "Can_Host.h"
#include "Can_HostLock.h"
#include "SchM_Can.h"
#include "Ecu_Config.h"
#include <assert.h>
#if defined(_WIN32)
/* Threads only; skip COM/RPC headers whose boolean clashes with Platform_Types.h. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
static CRITICAL_SECTION handshake;
static CONDITION_VARIABLE changed;
static DWORD dispatcher;
static void lock(void) { EnterCriticalSection(&handshake); }
static void unlock(void) { LeaveCriticalSection(&handshake); }
static void wake(void) { WakeAllConditionVariable(&changed); }
static void wait(void) { assert(SleepConditionVariableCS(&changed, &handshake, INFINITE)); }
static int dispatch_thread(void) { return GetCurrentThreadId() == dispatcher; }
#else
#include <pthread.h>
static pthread_mutex_t handshake = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static pthread_t dispatcher;
static void lock(void) { assert(pthread_mutex_lock(&handshake) == 0); }
static void unlock(void) { assert(pthread_mutex_unlock(&handshake) == 0); }
static void wake(void) { assert(pthread_cond_broadcast(&changed) == 0); }
static void wait(void) { assert(pthread_cond_wait(&changed, &handshake) == 0); }
static int dispatch_thread(void) { return pthread_equal(pthread_self(), dispatcher); }
#endif

const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
/* Number of the dispatcher's provider-area exits, or CanIf_SetPduMode returns, to stop
 * after; zero when disarmed. */
static unsigned armed;
static unsigned pdu_armed;
static unsigned paused;
static unsigned released;
static ComM_ModeType dispatched;
static unsigned transitions;
static ComM_ModeType published[8];
static unsigned publish_count;

static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8])
{
    (void)id;
    (void)length;
    (void)data;
    return ECU_OK;
}
static void note(ComM_ModeType mode)
{
    assert(publish_count < 8u);
    published[publish_count++] = mode;
}
static void entered_no(uint8 NetworkId)
{
    assert(NetworkId == 0u);
    note(COMM_NO_COMMUNICATION);
}
static void entered_silent(uint8 NetworkId)
{
    assert(NetworkId == 0u);
    note(COMM_SILENT_COMMUNICATION);
}
static void entered_full(uint8 NetworkId)
{
    assert(NetworkId == 0u);
    note(COMM_FULL_COMMUNICATION);
}
static const CanIf_RxPduConfigType receives[] = {{0x320u, 21u, 1u}};
static const CanIf_TxPduConfigType transmits[] = {{0x321u, 51u, 1u}};
static const CanIf_ConfigType canif = {receives,
                                       1u,
                                       transmits,
                                       1u,
                                       Ecu_HostBusSM_ControllerModeIndication,
                                       Ecu_HostBusSM_ControllerBusOff,
                                       0u,
                                       0u,
                                       0u,
                                       0u};
static const Can_ConfigType can = {sink};
static const Ecu_HostBusSM_ConfigType bussm = {0u, 0u};
static const ComM_UserHandleType users[] = {1u};
/* The Dcm notifications observe what ComM receives from the provider. */
static const ComM_ConfigType comm = {0u,
                                     users,
                                     1u,
                                     1u,
                                     Ecu_HostBusSM_RequestComMode,
                                     Ecu_HostBusSM_GetCurrentComMode,
                                     entered_no,
                                     entered_silent,
                                     entered_full};

Std_ReturnType __real_CanIf_SetControllerMode(uint8 ControllerId,
                                              Can_ControllerStateType ControllerMode);
Std_ReturnType __wrap_CanIf_SetControllerMode(uint8 ControllerId,
                                              Can_ControllerStateType ControllerMode)
{
    ++transitions;
    return __real_CanIf_SetControllerMode(ControllerId, ControllerMode);
}
static void stop_after(unsigned *counter)
{
    lock();
    if ((*counter != 0u) && dispatch_thread()) {
        --*counter;
        if (*counter == 0u) {
            paused = 1u;
            wake();
            while (released == 0u) {
                wait();
            }
        }
    }
    unlock();
}
void __real_Can_HostAreaExit(Can_HostAreaType area);
void __wrap_Can_HostAreaExit(Can_HostAreaType area)
{
    __real_Can_HostAreaExit(area);
    if (area == CAN_HOST_AREA_BUSSM) {
        stop_after(&armed);
    }
}
Std_ReturnType __real_CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest);
Std_ReturnType __wrap_CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest)
{
    const Std_ReturnType result = __real_CanIf_SetPduMode(ControllerId, PduModeRequest);
    stop_after(&pdu_armed);
    return result;
}

#if defined(_WIN32)
static DWORD WINAPI run(LPVOID argument)
{
    (void)argument;
    assert(Ecu_HostBusSM_RequestComMode(0u, dispatched) == E_OK);
    return 0u;
}
#else
static void *run(void *argument)
{
    (void)argument;
    assert(Ecu_HostBusSM_RequestComMode(0u, dispatched) == E_OK);
    return NULL_PTR;
}
#endif

/* Stops the dispatcher's request after `count` events of `counter`; `contend` completes
 * meanwhile. */
static void window(unsigned *counter, unsigned count, ComM_ModeType mode, void (*contend)(void))
{
#if defined(_WIN32)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    lock();
    *counter = count;
    paused = 0u;
    released = 0u;
    dispatched = mode;
#if defined(_WIN32)
    thread = CreateThread(NULL, 0u, run, NULL, CREATE_SUSPENDED, &dispatcher);
    assert(thread != NULL);
    assert(ResumeThread(thread) == 1u);
#else
    assert(pthread_create(&thread, NULL_PTR, run, NULL_PTR) == 0);
    dispatcher = thread;
#endif
    while (paused == 0u) {
        wait();
    }
    unlock();
    contend();
    lock();
    released = 1u;
    wake();
    unlock();
#if defined(_WIN32)
    assert(WaitForSingleObject(thread, INFINITE) == WAIT_OBJECT_0);
    assert(CloseHandle(thread) != 0);
#else
    assert(pthread_join(thread, NULL_PTR) == 0);
#endif
}

/* The dispatcher registered START but has not asked CanIf: a repeat is already pending. */
static void repeat_before_transition(void)
{
    ComM_ModeType mode = 99u;
    assert(Ecu_HostBusSM_GetCurrentComMode(0u, &mode) == E_OK && mode == COMM_NO_COMMUNICATION);
    assert(Ecu_HostBusSM_RequestComMode(0u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(transitions == 0u);
}
/* The dispatcher read SILENT for ComM: FULL completes and is published after it. */
static void complete_while_publishing(void)
{
    assert(Ecu_HostBusSM_RequestComMode(0u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(publish_count == 0u);
}
/* The dispatcher applied SILENT, unpublished: the stop completes and is published first. */
static void stop_before_publication(void)
{
    assert(Ecu_HostBusSM_RequestComMode(0u, COMM_NO_COMMUNICATION) == E_OK);
    Can_MainFunction_Mode();
    assert(publish_count == 1u && published[0] == COMM_NO_COMMUNICATION);
}

int main(void)
{
    ComM_ModeType mode = 99u;
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    CanIf_Init(&canif);
    Can_Init(&can);
    Ecu_HostBusSM_Init(&bussm);
    ComM_Init(&comm); /* NO on a stopped controller completes without a transition. */
    assert(transitions == 0u && publish_count == 0u);
    /* Exits: channel lookup, pending check, registration. */
    window(&armed, 3u, COMM_FULL_COMMUNICATION, repeat_before_transition);
    assert(transitions == 1u);
    Can_MainFunction_Mode();
    assert(Ecu_HostBusSM_GetCurrentComMode(0u, &mode) == E_OK && mode == COMM_FULL_COMMUNICATION);
    assert(publish_count == 1u && published[0] == COMM_FULL_COMMUNICATION);
    publish_count = 0u;
    /* Exits: channel lookup, pending check, ApplyMode's lookup, request record, publisher
     * claim, publication taken, and the actual mode read's lookup and pending check. */
    window(&armed, 8u, COMM_SILENT_COMMUNICATION, complete_while_publishing);
    assert(publish_count == 2u && published[0] == COMM_SILENT_COMMUNICATION &&
           published[1] == COMM_FULL_COMMUNICATION);
    assert(Ecu_HostBusSM_GetCurrentComMode(0u, &mode) == E_OK && mode == COMM_FULL_COMMUNICATION);
    assert(transitions == 1u);
    publish_count = 0u;
    /* A stale SILENT would follow NO and leave ComM SILENT on a stopped controller. */
    window(&pdu_armed, 1u, COMM_SILENT_COMMUNICATION, stop_before_publication);
    assert(publish_count == 1u && published[0] == COMM_NO_COMMUNICATION);
    assert(Ecu_HostBusSM_GetCurrentComMode(0u, &mode) == E_OK && mode == COMM_NO_COMMUNICATION);
    assert(transitions == 2u);
    Ecu_HostBusSM_DeInit();
    Can_DeInit();
    CanIf_DeInit();
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
