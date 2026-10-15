/** Deterministic window oracle for ComM (ADR 0018); actual ComM, host areas and threads.
 * A dispatcher stops right after ComM left COMM_STATE, before it calls the bus state
 * manager or publishes a mode. The contender (main) then completes ComM calls meanwhile:
 * none waits for the dispatcher, bus requests are issued once each in order, and the
 * latest actual mode reaches BswM and Dcm after the one in flight. Every observer enters
 * another strict area, which aborts on the host if ComM still held its own.
 * Verifies[ComM]: SWS_BSW_00134
 */
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Internal.h"
#include "BswM_ComM.h"
#include "SchM_BswM.h"
#include "SchM_ComM.h"
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

enum { REQUEST, INDICATE, DEINIT };
enum { DCM = 8u }; /* Added to a mode logged by a Dcm observer. */
/* Number of the dispatcher's COMM_STATE exit to stop after; zero when disarmed. */
static unsigned armed;
static unsigned paused;
static unsigned released;
static unsigned operation;
static ComM_ModeType requested[8];
static unsigned request_count;
static unsigned notified[8];
static unsigned notify_count;
static unsigned depth;
static boolean reenter;
static ComM_ModeType actual = COMM_NO_COMMUNICATION;

/* Proves the caller holds no area: the host aborts when an area is entered inside one. */
static void outside(void)
{
    SchM_Enter_BswM_BSWM_STATE();
    SchM_Exit_BswM_BSWM_STATE();
}
static Std_ReturnType bus_request(NetworkHandleType Channel, ComM_ModeType ComMode)
{
    assert(Channel == 7u && request_count < 8u);
    outside();
    requested[request_count++] = ComMode;
    return E_OK;
}
static Std_ReturnType bus_current(NetworkHandleType Channel, ComM_ModeType *ComMode)
{
    assert(Channel == 7u);
    outside();
    *ComMode = actual;
    return E_OK;
}
static void note(unsigned value)
{
    assert(notify_count < 8u);
    outside();
    notified[notify_count++] = value;
}
static void dcm_no(uint8 NetworkId)
{
    assert(NetworkId == 7u);
    note(DCM + COMM_NO_COMMUNICATION);
}
static void dcm_silent(uint8 NetworkId)
{
    assert(NetworkId == 7u);
    note(DCM + COMM_SILENT_COMMUNICATION);
}
static void dcm_full(uint8 NetworkId)
{
    assert(NetworkId == 7u);
    note(DCM + COMM_FULL_COMMUNICATION);
}
/* BswM double: a reentrant indication is published after this one, never inside it. */
void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType RequestedMode)
{
    assert(Network == 7u && depth == 0u);
    ++depth;
    note(RequestedMode);
    if (reenter == TRUE) {
        reenter = FALSE;
        ComM_BusSM_ModeIndication(7u, COMM_NO_COMMUNICATION);
        assert(notify_count == 1u);
    }
    --depth;
}
static const ComM_UserHandleType users[] = {17u};
static const ComM_ConfigType configuration = {
    7u, users, 1u, 2u, bus_request, bus_current, dcm_no, dcm_silent, dcm_full};

void __real_SchM_Exit_ComM_COMM_STATE(void);
void __wrap_SchM_Exit_ComM_COMM_STATE(void)
{
    __real_SchM_Exit_ComM_COMM_STATE();
    lock();
    if ((armed != 0u) && dispatch_thread()) {
        --armed;
        if (armed == 0u) {
            paused = 1u;
            wake();
            while (released == 0u) {
                wait();
            }
        }
    }
    unlock();
}

static void dispatch(void)
{
    if (operation == REQUEST) {
        assert(ComM_RequestComMode(17u, COMM_FULL_COMMUNICATION) == E_OK);
    } else if (operation == INDICATE) {
        ComM_BusSM_ModeIndication(7u, COMM_FULL_COMMUNICATION);
    } else {
        ComM_DeInit();
    }
}
#if defined(_WIN32)
static DWORD WINAPI run(LPVOID argument)
{
    (void)argument;
    dispatch();
    return 0u;
}
#else
static void *run(void *argument)
{
    (void)argument;
    dispatch();
    return NULL_PTR;
}
#endif

/* Runs one dispatcher request stopped after its `exits`-th exit; `contend` completes meanwhile. */
static void window(unsigned exits, unsigned request, void (*contend)(void))
{
#if defined(_WIN32)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    lock();
    armed = exits;
    paused = 0u;
    released = 0u;
    operation = request;
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

/* The dispatcher claimed FULL: a restart claims NO, which only the dispatcher issues. */
static void restart_while_requesting(void)
{
    ComM_InitStatusType status = COMM_UNINIT;
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_INIT);
    ComM_Init(&configuration);
    assert(request_count == 1u); /* Only the first lifetime's NO so far. */
    assert(ComM_RequestComMode(17u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING); /* Allowed reset by Init. */
    assert(request_count == 1u);
}
/* The dispatcher publishes FULL: later modes are recorded, the latest published after. */
static void indicate_while_publishing(void)
{
    ComM_BusSM_ModeIndication(7u, COMM_NO_COMMUNICATION);
    ComM_BusSM_ModeIndication(7u, COMM_SILENT_COMMUNICATION);
    assert(notify_count == 0u);
}
/* The dispatcher queries the actual mode: a request accepted meanwhile keeps ComM. */
static void request_while_deinitializing(void)
{
    assert(ComM_RequestComMode(17u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING);
}
/* DeInit during a publication: the mode in flight completes, the recorded one is dropped. */
static void deinitialize_while_publishing(void)
{
    ComM_InitStatusType status = COMM_INIT;
    ComM_BusSM_ModeIndication(7u, COMM_SILENT_COMMUNICATION);
    ComM_DeInit();
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_UNINIT);
}
/* A restart and DeInit during a bus call: the restart's NO is not issued after DeInit. */
static void deinitialize_while_requesting(void)
{
    ComM_InitStatusType status = COMM_INIT;
    ComM_Init(&configuration);
    ComM_DeInit();
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_UNINIT);
}

int main(void)
{
    unsigned before;
    ComM_InitStatusType status = COMM_UNINIT;
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    ComM_Init(&configuration);
    assert(request_count == 1u && requested[0] == COMM_NO_COMMUNICATION);
    ComM_CommunicationAllowed(7u, TRUE);
    /* Exit 1 records the request and claims FULL; exit 2 takes it for the call. */
    window(2u, REQUEST, restart_while_requesting);
    assert(request_count == 3u && requested[1] == COMM_FULL_COMMUNICATION &&
           requested[2] == COMM_NO_COMMUNICATION);
    assert(ComM_RunChannel() == COMM_NO_COM_REQUEST_PENDING && request_count == 3u);

    /* Exit 1 claims the publisher; exit 2 takes FULL for BswM and Dcm. */
    window(2u, INDICATE, indicate_while_publishing);
    assert(notify_count == 4u && notified[0] == COMM_FULL_COMMUNICATION &&
           notified[1] == DCM + COMM_FULL_COMMUNICATION &&
           notified[2] == COMM_SILENT_COMMUNICATION &&
           notified[3] == DCM + COMM_SILENT_COMMUNICATION);
    /* A reentrant indication from BswM follows the one that caused it. */
    notify_count = 0u;
    reenter = TRUE;
    ComM_BusSM_ModeIndication(7u, COMM_FULL_COMMUNICATION);
    assert(notify_count == 4u && notified[0] == COMM_FULL_COMMUNICATION &&
           notified[1] == DCM + COMM_FULL_COMMUNICATION && notified[2] == COMM_NO_COMMUNICATION &&
           notified[3] == DCM + COMM_NO_COMMUNICATION);

    /* Exit 1 found NO_PENDING_REQUEST; the bus query runs outside the area. */
    ComM_Init(&configuration);
    window(1u, DEINIT, request_while_deinitializing);
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_INIT);
    assert(ComM_RequestComMode(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel() == COMM_NO_COM_NO_PENDING_REQUEST);
    ComM_DeInit();
    assert(ComM_GetStatus(&status) == E_OK && status == COMM_UNINIT);

    ComM_Init(&configuration);
    notify_count = 0u;
    window(2u, INDICATE, deinitialize_while_publishing);
    assert(notify_count == 2u && notified[0] == COMM_FULL_COMMUNICATION &&
           notified[1] == DCM + COMM_FULL_COMMUNICATION);
    ComM_Init(&configuration);
    ComM_CommunicationAllowed(7u, TRUE);
    before = request_count;
    window(2u, REQUEST, deinitialize_while_requesting);
    assert(request_count == before + 1u && requested[before] == COMM_FULL_COMMUNICATION);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
