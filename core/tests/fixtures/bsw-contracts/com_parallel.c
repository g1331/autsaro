/** Deterministic window oracle for Com (ADR 0018); actual Com, host areas and threads.
 * A dispatcher stops right after Com left COM_STATE, before it calls PduR or a user. The
 * contender (main) then completes Com calls meanwhile: none waits for the dispatcher,
 * every notification and transmission happens exactly once, and the notification recorded
 * last is delivered last. Callbacks re-enter Com, so a held area would abort on the host.
 * Verifies[Com]: SWS_BSW_00134
 */
#include "Com.h"
#include "Com_Internal.h"
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

enum { RECEIVE, MAIN_RX, MAIN_TX, TRIGGER };
enum { ACK = 1, TOUT = 2 };
/* Number of the dispatcher's COM_STATE exit to stop after; zero when disarmed. */
static unsigned armed;
static unsigned paused;
static unsigned released;
static unsigned operation;
static unsigned events[8];
static uint32 seen[8];
static unsigned event_count;
static PduIdType sent_ids[4];
static uint32 sent_values[4];
static unsigned sent_count;
static uint8 payload[4];
static const PduInfoType info = {payload, NULL_PTR, 4u};

static void receive(CbkHandleIdType handle)
{
    uint32 value = 0u;
    assert(handle == 17u && event_count < 8u);
    (void)Com_ReceiveSignal(10u, &value);
    seen[event_count] = value;
    events[event_count++] = ACK;
}
static void timeout(CbkHandleIdType handle)
{
    assert(handle == 17u && event_count < 8u);
    seen[event_count] = 0u;
    events[event_count++] = TOUT;
}
static unsigned rejected;
static Std_ReturnType lower_result = E_OK;
static void rejection(PduIdType id, Std_ReturnType result)
{
    assert(Com_GetStatus() == COM_INIT);
    assert(id == 1u && result == 3u && sent_count == 1u);
    ++rejected;
}
static void replacement_rejection(PduIdType id, Std_ReturnType result)
{
    (void)id;
    (void)result;
    assert(0); /* An in-flight lower call must retain its original observer. */
}
static Std_ReturnType transmit(PduIdType id, const PduInfoType *pdu)
{
    assert(sent_count < 4u && pdu->SduLength == 4u);
    (void)Com_GetStatus();
    sent_ids[sent_count] = id;
    sent_values[sent_count++] = (uint32)pdu->SduDataPtr[0] | ((uint32)pdu->SduDataPtr[1] << 8u);
    return lower_result;
}
static const Com_PduConfigType pdus[] = {{0u, 10u, TRUE, 7u, 30u, 17u},
                                         {1u, 11u, FALSE, 9u, 0u, 0u}};
static const Com_ConfigType configuration = {pdus, 2u, 0u, receive, timeout, transmit, rejection};

void __real_SchM_Exit_Com_COM_STATE(void);
void __wrap_SchM_Exit_Com_COM_STATE(void)
{
    __real_SchM_Exit_Com_COM_STATE();
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
    if (operation == RECEIVE) {
        Com_RxIndication(0u, &info);
    } else if (operation == MAIN_RX) {
        Ecu_ComMainFunctionRx();
    } else if (operation == MAIN_TX) {
        Ecu_ComMainFunctionTx();
    } else {
        assert(Com_TriggerIPDUSend(1u) == E_OK);
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

static void start(void)
{
    Com_Init(&configuration);
    Com_IpduGroupStart(0u, TRUE);
    Com_EnableReceptionDM(0u);
    event_count = 0u;
    sent_count = 0u;
}
static void expire(void)
{
    unsigned period;
    for (period = 0u; period < 30u; ++period) {
        Ecu_ComMainFunctionRx();
    }
}
static void deliver(uint8 value)
{
    payload[0] = value;
    Com_RxIndication(0u, &info);
}
/* The dispatcher has recorded a timeout, not taken it: this reception publishes both. */
static void receive_before_taken(void)
{
    deliver(21u);
    assert(event_count == 2u && events[0] == TOUT && events[1] == ACK && seen[1] == 21u);
}
/* The dispatcher delivers a timeout: a reception is only recorded, and follows it. */
static void receive_while_delivering(void)
{
    deliver(22u);
    assert(event_count == 0u);
    assert(Com_GetStatus() == COM_INIT);
}
/* The dispatcher delivers a reception: a later timeout follows it. */
static void expire_while_delivering(void)
{
    expire();
    assert(event_count == 0u);
}
/* Reinitialization while a reception is delivered: the new one is still owed once. */
static void reinitialize_while_delivering(void)
{
    Com_DeInit();
    assert(Com_GetStatus() == COM_UNINIT);
    Com_Init(&configuration);
    Com_IpduGroupStart(0u, TRUE);
    deliver(24u);
    assert(event_count == 0u);
}
/* The dispatcher packed the Tx PDU: the update and DeInit finish without waiting. */
static void update_and_stop_while_sending(void)
{
    const uint32 value = 0x55u;
    assert(Com_SendSignal(11u, &value) == E_OK);
    Com_DeInit();
    assert(sent_count == 0u);
}

static void replace_observer_while_sending(void)
{
    static const Com_ConfigType replacement = {
        pdus, 2u, 0u, receive, timeout, transmit, replacement_rejection};
    Com_Init(&replacement);
    assert(Com_GetStatus() == COM_INIT && sent_count == 0u);
}

int main(void)
{
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    start();
    deliver(20u);
    event_count = 0u;
    /* Main Rx exits once after recording the timeout; Publish exits after taking it. */
    {
        unsigned period;
        for (period = 0u; period < 29u; ++period) {
            Ecu_ComMainFunctionRx();
        }
    }
    window(1u, MAIN_RX, receive_before_taken);
    assert(event_count == 2u);
    event_count = 0u;
    expire();
    assert(event_count == 1u && events[0] == TOUT);
    event_count = 0u;
    {
        unsigned period;
        for (period = 0u; period < 29u; ++period) {
            Ecu_ComMainFunctionRx();
        }
    }
    window(2u, MAIN_RX, receive_while_delivering);
    assert(event_count == 2u && events[0] == TOUT && events[1] == ACK && seen[1] == 22u);

    /* Com_RxIndication exits after recording, then Publish exits after taking it. */
    event_count = 0u;
    payload[0] = 23u;
    window(2u, RECEIVE, expire_while_delivering);
    assert(event_count == 2u && events[0] == ACK && events[1] == TOUT && seen[0] == 23u);
    event_count = 0u;
    window(2u, RECEIVE, reinitialize_while_delivering);
    assert(event_count == 2u && events[0] == ACK && events[1] == ACK && seen[1] == 24u);
    assert(Com_GetStatus() == COM_INIT);

    /* Main Tx exits once per PDU: the Rx PDU, then the packed Tx PDU. */
    start();
    window(2u, MAIN_TX, update_and_stop_while_sending);
    assert(sent_count == 1u && sent_ids[0] == 1u && sent_values[0] == 9u);
    assert(Com_TriggerIPDUSend(1u) == E_NOT_OK && sent_count == 1u);
    start();
    Ecu_ComMainFunctionTx();
    assert(sent_count == 1u && sent_values[0] == 9u);
    lower_result = 3u;
    start();
    window(2u, MAIN_TX, replace_observer_while_sending);
    assert(rejected == 1u && sent_count == 1u);
    start();
    window(1u, TRIGGER, replace_observer_while_sending);
    assert(rejected == 2u && sent_count == 1u);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
