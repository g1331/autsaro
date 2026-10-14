/** Deterministic window oracle for the routers (ADR 0018); actual PduR, LSduR and SchM.
 * A dispatcher stops right after PduR or LSduR left its area and before it calls the next
 * module. The contender (main) then completes Init and routing through the same areas, and
 * every request still has exactly one result. CanIf_Transmit is wrapped. The areas guard
 * only module data (Verifies[PduR]: SWS_BSW_00134; Verifies[LSduR]: SWS_BSW_00134).
 */
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "LSduR_CanIf.h"
#include "SchM_PduR.h"
#include "SchM_LSduR.h"
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

enum { NOWHERE, AFTER_PDUR, AFTER_LSDUR };
enum { TRANSMIT, RECEIVE, CONFIRM };
static unsigned armed;
static unsigned paused;
static unsigned released;
static unsigned operation;
static Std_ReturnType outcome;
static unsigned transmits;
static unsigned receptions;
static unsigned confirmations;
static PduIdType last_id;
static const PduInfoType *last_info;
static Std_ReturnType last_result;
static uint8 first_bytes[8];
static uint8 second_bytes[8];
static const PduInfoType first = {first_bytes, NULL_PTR, 8u};
static const PduInfoType second = {second_bytes, NULL_PTR, 8u};

Std_ReturnType __wrap_CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    ++transmits;
    last_id = TxPduId;
    last_info = PduInfoPtr;
    return E_OK;
}
static void com_rx(PduIdType id, const PduInfoType *pdu)
{
    ++receptions;
    last_id = id;
    last_info = pdu;
}
static void com_conf(PduIdType id, Std_ReturnType result)
{
    ++confirmations;
    last_id = id;
    last_result = result;
}

/* Stop the dispatcher once per window, after the router's real exit, with no sleeps. */
static void pause_at(unsigned area)
{
    lock();
    if ((armed == area) && dispatch_thread()) {
        armed = NOWHERE;
        paused = 1u;
        wake();
        while (released == 0u) {
            wait();
        }
    }
    unlock();
}
void __real_SchM_Exit_PduR_PDUR_STATE(void);
void __real_SchM_Exit_LSduR_LSDUR_STATE(void);
void __wrap_SchM_Exit_PduR_PDUR_STATE(void)
{
    __real_SchM_Exit_PduR_PDUR_STATE();
    pause_at(AFTER_PDUR);
}
void __wrap_SchM_Exit_LSduR_LSDUR_STATE(void)
{
    __real_SchM_Exit_LSduR_LSDUR_STATE();
    pause_at(AFTER_LSDUR);
}

static void dispatch(void)
{
    if (operation == TRANSMIT) {
        outcome = PduR_ComTransmit(5u, &first);
    } else if (operation == RECEIVE) {
        LSduR_CanIfRxIndication(5u, &first);
    } else {
        LSduR_CanIfTxConfirmation(5u, E_OK);
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

static const LSduR_RxRouteType ls_rx[] = {{5u, 25u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 35u, 5u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 1u};
static const PduR_RxRouteType pdur_rx[] = {{25u, 5u, com_rx}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 5u, PDUR_LO_LSDUR, 35u, LSduR_PduRTransmit, com_conf, NULL_PTR}};
static const PduR_PBConfigType pdur = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
static const PduR_PBConfigType other = {24u, NULL_PTR, 0u, NULL_PTR, 0u, NULL_PTR, 0u};
static const LSduR_PBConfigType ls_other = {30u, NULL_PTR, 0u, NULL_PTR, 0u};

/* Runs one dispatcher request stopped at `area`; `contend` must complete meanwhile. */
static void window(unsigned area, unsigned request, void (*contend)(void))
{
#if defined(_WIN32)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    lock();
    armed = area;
    paused = 0u;
    released = 0u;
    operation = request;
    outcome = E_NOT_OK;
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

/* The dispatcher read UNINIT; Init and a full request complete inside its window. */
static void initialize(void)
{
    LSduR_Init(&ls);
    PduR_Init(&pdur);
    assert(PduR_ComTransmit(5u, &second) == E_OK);
    assert(transmits == 1u && last_id == 5u && last_info == &second);
}
static void receive(void)
{
    LSduR_CanIfRxIndication(5u, &second);
    assert(receptions == 1u && last_id == 5u && last_info == &second);
    LSduR_Init(&ls_other);
    PduR_Init(&other);
}
static void confirm(void)
{
    PduR_CanIfTxConfirmation(35u, E_NOT_OK);
    assert(confirmations == 1u && last_id == 5u && last_result == E_NOT_OK);
    LSduR_Init(&ls_other);
    PduR_Init(&other);
}
static void transmit(void)
{
    assert(PduR_ComTransmit(5u, &second) == E_OK);
    assert(transmits == 1u && last_info == &second);
}

int main(void)
{
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    /* An in-flight request keeps the state it read: UNINIT, so no call and no success. */
    window(AFTER_PDUR, TRANSMIT, initialize);
    assert(outcome == E_NOT_OK && transmits == 1u);
    /* Requests read ONLINE; the contender's own results and ignored Inits come first. */
    window(AFTER_LSDUR, RECEIVE, receive);
    assert(receptions == 2u && last_id == 5u && last_info == &first);
    window(AFTER_PDUR, CONFIRM, confirm);
    assert(confirmations == 2u && last_id == 5u && last_result == E_OK);
    transmits = 0u;
    window(AFTER_LSDUR, TRANSMIT, transmit);
    assert(outcome == E_OK && transmits == 2u && last_id == 5u && last_info == &first);
    assert(PduR_GetConfigurationId() == 23u && LSduR_GetConfigurationId() == 29u);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
