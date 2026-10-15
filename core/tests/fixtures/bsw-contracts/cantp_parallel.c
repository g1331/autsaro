/** Deterministic window oracle for CanTp (ADR 0018); actual CanTp, host areas and threads.
 * A dispatcher stops right after CanTp left CANTP_STATE, before it calls PduR or LSduR.
 * The contender (main) then completes CanTp calls meanwhile: none waits for the
 * dispatcher, events recorded meanwhile are delivered by the dispatcher exactly once, and
 * results of calls overtaken by Init, a timeout or a cancellation are discarded.
 * Verifies[CanTp]: SWS_BSW_00134
 */
#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
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

enum { RX_NSDU = 4u, TX_NSDU = 6u, RX_NPDU = 2u, TX_NPDU = 3u, UPPER_RX = 21u, UPPER_TX = 23u };
enum { N_AS = 5u, N_CS = 10u, PAD = 0xccu };
enum { MAIN, RECEIVE_SF, RECEIVE_CF, TRANSMIT };
static const CanTp_ConfigType config = {.receive = RX_NSDU,
                                        .transmit = TX_NSDU,
                                        .lower_receive = RX_NPDU,
                                        .lower_transmit = TX_NPDU,
                                        .maximum_length = 256u,
                                        .n_as = N_AS,
                                        .n_ar = 5u,
                                        .n_br = 5u,
                                        .n_bs = 10u,
                                        .n_cr = 10u,
                                        .n_cs = N_CS,
                                        .main_period_ms = 1u,
                                        .wait_frame_max = 0u,
                                        .padding = PAD,
                                        .upper_receive = UPPER_RX,
                                        .upper_transmit = UPPER_TX};
/* Number of the dispatcher's CANTP_STATE exit to stop after; zero when disarmed. */
static unsigned armed;
static unsigned paused;
static unsigned released;
static unsigned operation;
/* Written by both threads only while the other one is stopped or joined. */
static unsigned sent_count;
static uint8 sent[4][8];
static unsigned copy_tx_calls;
static unsigned tx_done;
static Std_ReturnType tx_result;
static unsigned starts;
static uint8 received[32];
static unsigned received_length;
static unsigned rx_done;
static Std_ReturnType rx_result;

Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    assert(TxPduId == TX_NPDU && PduInfoPtr->SduLength == 8u && sent_count < 4u);
    memcpy(sent[sent_count++], PduInfoPtr->SduDataPtr, 8u);
    return E_OK;
}
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id,
                                       const PduInfoType *info,
                                       const RetryInfoType *retry,
                                       PduLengthType *available)
{
    assert(id == UPPER_TX && retry == NULL_PTR);
    memset(info->SduDataPtr, 0x5a, info->SduLength);
    ++copy_tx_calls;
    *available = 0u;
    return BUFREQ_OK;
}
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result)
{
    assert(id == UPPER_TX);
    tx_result = result;
    ++tx_done;
}
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType length,
                                             PduLengthType *available)
{
    assert(id == UPPER_RX && info != NULL_PTR);
    (void)length;
    ++starts;
    received_length = 0u;
    *available = 64u;
    return BUFREQ_OK;
}
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id,
                                       const PduInfoType *info,
                                       PduLengthType *available)
{
    assert(id == UPPER_RX && received_length + info->SduLength <= 32u);
    memcpy(&received[received_length], info->SduDataPtr, info->SduLength);
    received_length += info->SduLength;
    *available = 64u;
    return BUFREQ_OK;
}
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result)
{
    assert(id == UPPER_RX);
    rx_result = result;
    ++rx_done;
}
Std_ReturnType Det_ReportRuntimeError(uint16_t ModuleId,
                                      uint8_t InstanceId,
                                      uint8_t ApiId,
                                      uint8_t ErrorId)
{
    assert(ModuleId == 35u && InstanceId == 0u && ApiId == 0x06u);
    (void)ErrorId;
    return E_OK;
}

void __real_SchM_Exit_CanTp_CANTP_STATE(void);
void __wrap_SchM_Exit_CanTp_CANTP_STATE(void)
{
    __real_SchM_Exit_CanTp_CANTP_STATE();
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

static void receive(uint8 b0, uint8 b1, uint8 b2)
{
    uint8 frame[8] = {0u, 0u, 0u, PAD, PAD, PAD, PAD, PAD};
    const PduInfoType info = {frame, NULL_PTR, 8u};
    frame[0] = b0;
    frame[1] = b1;
    frame[2] = b2;
    CanTp_RxIndication(RX_NPDU, &info);
}
static Std_ReturnType transmit(void)
{
    const PduInfoType info = {NULL_PTR, NULL_PTR, 5u};
    return CanTp_Transmit(TX_NSDU, &info);
}
static void ticks(unsigned count)
{
    unsigned i;
    for (i = 0u; i < count; ++i) {
        CanTp_MainFunction();
    }
}
static void dispatch(void)
{
    if (operation == MAIN) {
        CanTp_MainFunction();
    } else if (operation == RECEIVE_SF) {
        receive(0x02u, 0x41u, 0x42u);
    } else if (operation == RECEIVE_CF) {
        receive(0x21u, 0x51u, 0x52u);
    } else {
        assert(transmit() == E_OK);
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
    CanTp_Init(&config);
    sent_count = 0u;
    copy_tx_calls = 0u;
    tx_done = 0u;
    starts = 0u;
    rx_done = 0u;
}
/* The dispatcher took an N_As timeout's confirmation: a new request is accepted at once
 * and left to the dispatcher. */
static void transmit_while_confirming(void)
{
    assert(transmit() == E_OK);
    assert(tx_done == 0u && copy_tx_calls == 1u && sent_count == 1u);
}
/* The dispatcher took an SF: Init loses it, and a later SF is only queued. */
static void reinitialize_while_receiving(void)
{
    CanTp_Init(&config);
    receive(0x02u, 0x43u, 0x44u);
    assert(starts == 0u && rx_done == 0u);
}
/* The dispatcher prepared the first copy: N_Cs expires meanwhile. */
static void expire_while_copying(void)
{
    ticks(N_CS);
    assert(tx_done == 0u && copy_tx_calls == 0u);
}
/* The dispatcher accepted a CF: the reception is cancelled meanwhile. */
static void cancel_while_copying(void)
{
    assert(CanTp_CancelReceive(RX_NSDU) == E_OK);
    assert(rx_done == 0u);
}

int main(void)
{
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    /* Main exits after the tick, after taking `running`, and after taking the work. */
    start();
    assert(transmit() == E_OK && sent_count == 1u);
    ticks(N_AS - 1u);
    window(3u, MAIN, transmit_while_confirming);
    assert(tx_done == 1u && tx_result == E_NOT_OK && copy_tx_calls == 2u && sent_count == 1u);
    /* The late confirmation frees the N-PDU for the new SF, which then completes. */
    CanTp_TxConfirmation(TX_NPDU, E_OK);
    ticks(1u);
    assert(sent_count == 2u && tx_done == 1u);
    CanTp_TxConfirmation(TX_NPDU, E_OK);
    assert(tx_done == 2u && tx_result == E_OK);

    /* RxIndication exits after queueing, after taking `running`, and after the frame. */
    start();
    window(3u, RECEIVE_SF, reinitialize_while_receiving);
    assert(starts == 1u && rx_done == 1u && rx_result == E_OK);
    assert(received_length == 2u && received[0] == 0x43u && received[1] == 0x44u);

    /* Transmit exits after the request, after taking `running`, and after the header. */
    start();
    window(3u, TRANSMIT, expire_while_copying);
    assert(copy_tx_calls == 1u && sent_count == 0u && tx_done == 1u && tx_result == E_NOT_OK);

    /* RxIndication of a CF exits a fourth time once the CF is accepted. */
    start();
    receive(0x10u, 20u, 0x31u);
    assert(sent_count == 1u && sent[0][0] == 0x30u);
    CanTp_TxConfirmation(TX_NPDU, E_OK);
    window(4u, RECEIVE_CF, cancel_while_copying);
    assert(rx_done == 1u && rx_result == E_NOT_OK && received_length == 13u);
    receive(0x22u, 0x53u, 0x54u);
    assert(rx_done == 1u);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
