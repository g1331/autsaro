/** Deterministic Dcm windows: real host areas and threads, stopped immediately after
 * SchM_Exit_Dcm_DCM_STATE and before a consumer. The contender must complete in
 * that window; a callback cannot cause overlapping ReadData, stale response bytes,
 * out-of-order modes, or reuse an outstanding lower callback identity.
 * Verifies[Dcm]: SWS_BSW_00134
 * Verifies: SWS_Dcm_00311, SWS_Dcm_01062
 */
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "SchM_Dcm.h"
#include "ComM_Dcm.h"
#include "PduR_Dcm.h"
#include "Can_HostLock.h"
#include <assert.h>
#include <string.h>
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

enum { RX = 17u, TX = 19u, ROUTER = 41u, CHANNEL = 7u };
enum { MAIN, RESET, RECEIVE };
static unsigned armed;
static unsigned paused;
static unsigned released;
static unsigned operation;
/* Both threads access the counters only while the other is paused or joined. */
static unsigned reads;
static unsigned sent_count;
static unsigned rejected_copies;
static unsigned mode_depth;
static unsigned active_calls;
static uint8 published_mode;
static uint8 sent[64];
static PduLengthType sent_length;
static Std_ReturnType read_did(uint8 *data)
{
    ++reads;
    for (unsigned i = 0u; i < 4u; ++i) {
        data[i] = (uint8)(0x50u + reads);
    }
    return E_OK;
}
static void authentication(uint8 value) { assert(value == 0u); }
static const Dcm_ConfigType config = {
    RX, TX, CHANNEL, 0x1234u, read_did, 4u, 50u, 5000u, 5u, 64u, ROUTER, authentication, 0u};
void ComM_DCM_ActiveDiagnostic(NetworkHandleType channel)
{
    assert(channel == CHANNEL);
    ++active_calls;
    assert(Dcm_SetActiveDiagnostic(TRUE) == E_OK);
}
void ComM_DCM_InactiveDiagnostic(NetworkHandleType channel)
{
    assert(channel == CHANNEL);
    assert(Dcm_SetActiveDiagnostic(TRUE) == E_OK);
}
Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(
    Rte_ModeType_DcmDiagnosticSessionControl mode)
{
    assert(++mode_depth == 1u);
    Can_HostAreaEnter(CAN_HOST_AREA_SCHM_MODE);
    published_mode = mode;
    Can_HostAreaExit(CAN_HOST_AREA_SCHM_MODE);
    --mode_depth;
    return E_OK;
}
Std_ReturnType PduR_DcmTransmit(PduIdType id, const PduInfoType *info)
{
    PduLengthType available = 999u;
    PduInfoType copy = {sent, NULL_PTR, info->SduLength};
    assert(id == ROUTER && info->SduLength <= sizeof(sent));
    ++sent_count;
    if (Dcm_CopyTxData(TX, &copy, NULL_PTR, &available) == BUFREQ_OK) {
        assert(available == 0u);
        sent_length = info->SduLength;
    } else {
        assert(available == 999u);
        ++rejected_copies;
    }
    return E_OK;
}
void __real_SchM_Exit_Dcm_DCM_STATE(void);
void __wrap_SchM_Exit_Dcm_DCM_STATE(void)
{
    __real_SchM_Exit_Dcm_DCM_STATE();
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
static void receive(boolean extended)
{
    uint8 bytes[3] = {0x22u, 0x12u, 0x34u};
    PduLengthType available = 999u;
    PduInfoType info = {bytes, NULL_PTR, 3u};
    if (extended == TRUE) {
        bytes[0] = 0x10u;
        bytes[1] = 0x83u;
        info.SduLength = 2u;
    }
    assert(Dcm_StartOfReception(RX, &info, info.SduLength, &available) == BUFREQ_OK);
    assert(Dcm_CopyRxData(RX, &info, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(RX, E_OK);
}
static void start(void)
{
    Dcm_Init(&config);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    reads = 0u;
    sent_count = 0u;
    rejected_copies = 0u;
    sent_length = 0u;
}
static void dispatch(void)
{
    if (operation == RECEIVE) {
        receive(FALSE);
        return;
    }
    if (operation == MAIN) {
        Dcm_MainFunction();
    } else {
        assert(Dcm_ResetToDefaultSession() == E_OK);
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
static void restart_while_reading(void)
{
    assert(reads == 0u);
    Dcm_Init(&config);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    receive(FALSE);
    Dcm_MainFunction();
    assert(reads == 0u && sent_count == 0u);
}
static void restart_while_sending(void)
{
    assert(reads == 1u && sent_count == 0u);
    Dcm_Init(&config);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    receive(FALSE);
    Dcm_MainFunction();
    assert(reads == 1u && sent_count == 0u);
}
static void change_session_while_publishing(void)
{
    receive(TRUE);
    Dcm_MainFunction();
    Dcm_SesCtrlType session = 0u;
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 1u);
    assert(published_mode == 2u); /* Pending publisher still owns the old effect. */
}
static void finish_while_active_publication_is_pending(void)
{
    const unsigned before = reads;
    Dcm_MainFunction();
    assert(reads == before && sent_count == 0u);
}
static void close_no_com(void) { Dcm_ComM_NoComModeEntered(CHANNEL); }
static void close_silent_com(void) { Dcm_ComM_SilentComModeEntered(CHANNEL); }
int main(void)
{
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    start();
    receive(FALSE);
    /* Main exits after claiming the executor, copying the request, then before ReadData. */
    window(3u, MAIN, restart_while_reading);
    assert(reads == 1u && sent_count == 0u);
    Dcm_MainFunction();
    assert(reads == 2u && sent_count == 1u && sent_length == 7u && sent[3] == 0x52u);
    Dcm_TpTxConfirmation(TX, E_OK);

    start();
    receive(FALSE);
    /* The sixth exit commits the PduR admission before the external call. */
    window(6u, MAIN, restart_while_sending);
    assert(reads == 1u && sent_count == 1u && rejected_copies == 1u);
    Dcm_MainFunction();
    assert(reads == 2u && sent_count == 1u);
    Dcm_TpTxConfirmation(TX, E_OK); /* Retires only the old lower identity. */
    Dcm_MainFunction();
    assert(sent_count == 2u && rejected_copies == 1u && sent[3] == 0x52u);
    Dcm_TpTxConfirmation(TX, E_OK);

    start();
    receive(TRUE);
    Dcm_MainFunction();
    assert(published_mode == 2u && sent_count == 0u);
    /* Reset exits after its state change, publisher claim, inactive effect and session effect. */
    window(4u, RESET, change_session_while_publishing);
    Dcm_MainFunction();
    Dcm_SesCtrlType session = 0u;
    assert(Dcm_GetSesCtrlType(&session) == E_OK && session == 3u && published_mode == 2u);

    /* A mode notification revokes a reserved, not yet committed lower admission.
     * Verifies: SWS_Dcm_00148, SWS_Dcm_00152, SWS_Dcm_00153
     */
    start();
    receive(FALSE);
    window(5u, MAIN, close_no_com);
    assert(sent_count == 0u);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    Dcm_MainFunction();
    assert(sent_count == 1u);
    Dcm_TpTxConfirmation(TX, E_OK);
    start();
    receive(FALSE);
    window(5u, MAIN, close_silent_com);
    assert(sent_count == 0u);
    Dcm_ComM_FullComModeEntered(CHANNEL);
    Dcm_MainFunction();
    assert(sent_count == 1u);
    Dcm_TpTxConfirmation(TX, E_OK);
    /* Verifies: SWS_Dcm_01376, SWS_Dcm_01377
     * Keep every request notification; Main cannot overtake its publisher.
     */
    start();
    active_calls = 0u;
    window(5u, RECEIVE, finish_while_active_publication_is_pending);
    assert(active_calls == 1u && reads == 0u);
    Dcm_MainFunction();
    Dcm_TpTxConfirmation(TX, E_OK);
    receive(FALSE);
    Dcm_MainFunction();
    Dcm_TpTxConfirmation(TX, E_OK);
    assert(active_calls == 2u);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
