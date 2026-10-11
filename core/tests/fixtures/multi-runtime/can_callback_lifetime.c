/** Deterministic admission-window oracle; real driver, CanIf, routing and SchM. */
#include "Can.h"
#include "CanIf.h"
#include "Can_HostLock.h"
#include "SchM_Can.h"
#include "SchM_CanIf.h"
#include "SchM_Dcm.h"
#include "SchM_CanTp.h"
#include "SchM_ComM.h"
#include "SchM_BswM.h"
#include "Ecu_HostBusSM.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "PduR.h"
#include "PduR_CanIf.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>
#if defined(_WIN32)
#include <windows.h>
static CRITICAL_SECTION handshake;
static CONDITION_VARIABLE changed;
static DWORD dispatcher;
static DWORD contender;
static void lock(void) { EnterCriticalSection(&handshake); }
static void unlock(void) { LeaveCriticalSection(&handshake); }
static void wake(void) { WakeAllConditionVariable(&changed); }
static void wait(void) { assert(SleepConditionVariableCS(&changed, &handshake, INFINITE)); }
static int dispatch_thread(void) { return GetCurrentThreadId() == dispatcher; }
static int contend_thread(void) { return GetCurrentThreadId() == contender; }
#else
#include <pthread.h>
static pthread_mutex_t handshake = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static pthread_t dispatcher;
static pthread_t contender;
static void lock(void) { assert(pthread_mutex_lock(&handshake) == 0); }
static void unlock(void) { assert(pthread_mutex_unlock(&handshake) == 0); }
static void wake(void) { assert(pthread_cond_broadcast(&changed) == 0); }
static void wait(void) { assert(pthread_cond_wait(&changed, &handshake) == 0); }
static int dispatch_thread(void) { return pthread_equal(pthread_self(), dispatcher); }
static int contend_thread(void) { return pthread_equal(pthread_self(), contender); }
#endif
static unsigned paused;
static unsigned attempted;
static unsigned blocked;
static unsigned released;
static unsigned armed;
static unsigned skip_unlocks;
static unsigned operation;
static unsigned cancellation;
static unsigned generation;
static unsigned callbacks[2];
static unsigned reenter;
static Can_ControllerStateType last_mode;
static const uint8 bytes[8] = {42u};
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
/* Observe actual native contention, so release cannot race a merely scheduled waiter. */
#if defined(_WIN32)
extern void(WINAPI *__real___imp_EnterCriticalSection)(LPCRITICAL_SECTION);
static void WINAPI enter_native(LPCRITICAL_SECTION mutex) {
    if (mutex != &handshake) {
        int probe;
        lock();
        probe = paused && contend_thread();
        unlock();
        if (probe) {
            if (TryEnterCriticalSection(mutex))
                return;
            lock();
            blocked = 1u;
            wake();
            unlock();
        }
    }
    __real___imp_EnterCriticalSection(mutex);
}
void(WINAPI *__wrap___imp_EnterCriticalSection)(LPCRITICAL_SECTION) = enter_native;
#else
#include <errno.h>
int __real_pthread_mutex_lock(pthread_mutex_t *mutex);
int __wrap_pthread_mutex_lock(pthread_mutex_t *mutex) {
    if (mutex != &handshake) {
        int probe;
        lock();
        probe = paused && contend_thread();
        unlock();
        if (probe) {
            int result = pthread_mutex_trylock(mutex);
            if (result != EBUSY)
                return result;
            lock();
            blocked = 1u;
            wake();
            unlock();
        }
    }
    return __real_pthread_mutex_lock(mutex);
}
#endif
void __real_Can_Unlock(void);
void __real_SchM_Enter_CanIf_CANIF_STATE(void);
/* Stop after the driver unlock, before the actual CanIf call, with no sleeps. */
void __wrap_Can_Unlock(void) {
    __real_Can_Unlock();
    lock();
    if (armed && dispatch_thread() && skip_unlocks) {
        --skip_unlocks;
    } else if (armed && dispatch_thread()) {
        armed = 0u;
        paused = 1u;
        wake();
        while (!released)
            wait();
    }
    unlock();
}
void __wrap_SchM_Enter_CanIf_CANIF_STATE(void) {
    lock();
    if (paused && contend_thread()) {
        attempted = 1u;
        wake();
    }
    unlock();
    __real_SchM_Enter_CanIf_CANIF_STATE();
}
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    assert(id == 0x321u && length == 1u && data[0] == 42u);
    return ECU_OK;
}
static const Can_ConfigType can = {sink};
static void observed(void) {
    /* Another thread proves the DRIVER mutex is free at the dispatch window. */
    ++callbacks[generation];
    if (reenter) {
        reenter = 0u;
        Can_SetMode(CAN_STOPPED);
        Can_DeInit();
        Can_Init(&can);
        Can_SetMode(CAN_STARTED);
    }
}
static void mode(uint8 id, Can_ControllerStateType value) {
    assert(id == 0u);
    last_mode = value;
    observed();
}
static void busoff(uint8 id) {
    assert(id == 0u);
    observed();
}
static void rx(PduIdType id, const PduInfoType *info) {
    assert(id == 17u && info->SduLength == 1u && info->SduDataPtr[0] == 42u);
    observed();
}
static void tx(PduIdType id, Std_ReturnType status) {
    assert(id == 18u);
    if (status == E_OK)
        observed();
}
static const CanIf_RxPduConfigType receives[] = {{0x320u, 21u, 1u}};
static const CanIf_TxPduConfigType transmits[] = {{0x321u, 51u, 1u}};
static const CanIf_ConfigType canif = {receives, 1u, transmits, 1u, mode, busoff, 0u, 0u, 0u, 0u};
static const LSduR_RxRouteType ls_rx[] = {{21u, 31u, PduR_CanIfRxIndication}};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, NULL_PTR}};
static const LSduR_PBConfigType ls = {29u, ls_rx, 1u, ls_tx, 1u};
static const PduR_RxRouteType pdur_rx[] = {{31u, 17u, rx}};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 18u, 41u, LSduR_PduRTransmit, tx, NULL_PTR}};
static const PduR_PBConfigType pdur = {23u, pdur_rx, 1u, pdur_tx, 1u, NULL_PTR, 0u};
static const Ecu_HostBusSM_ConfigType bussm = {0u, 0u};
static void initialize(void) {
    Can_Init(&can);
    CanIf_Init(&canif);
    LSduR_Init(&ls);
    PduR_Init(&pdur);
    Ecu_HostBusSM_Init(&bussm);
    Can_SetMode(CAN_STARTED);
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
}
static void dispatch(void) {
#if defined(_WIN32)
    dispatcher = GetCurrentThreadId();
#else
    dispatcher = pthread_self();
#endif
    if (operation == 0u)
        Can_SetMode(CAN_STARTED);
    else if (operation == 1u)
        Can_MainFunction_Mode();
    else if (operation == 2u) {
#if CAN_BUSOFF_POLLING == 1
        Can_MainFunction_BusOff();
#else
        Can_SetMode(CAN_BUS_OFF);
#endif
    } else if (operation == 3u) {
#if CAN_TX_POLLING == 1
        Can_MainFunction_Write();
#else
        assert(Can_HostFlush() == ECU_OK);
#endif
    } else if (operation == 4u) {
#if CAN_RX_POLLING == 1
        Can_MainFunction_Read();
#else
        assert(Can_Inject(0x320u, 1u, bytes, 0u) == ECU_OK);
#endif
    } else {
        Can_ControllerStateType actual;
        assert(CanIf_GetControllerMode(0u, &actual) == E_OK);
    }
}
static void cancel(void) {
    Can_ControllerStateType actual;
#if defined(_WIN32)
    contender = GetCurrentThreadId();
#else
    contender = pthread_self();
#endif
    if (operation == 5u) {
        assert(Ecu_HostBusSM_RequestComMode(0u, COMM_NO_COMMUNICATION) == E_OK);
    } else if (operation == 10u) {
        Det_Init(NULL_PTR);
    } else if (operation >= 6u) {
        if (operation == 6u)
            SchM_Enter_Dcm_DCM_STATE();
        else if (operation == 7u)
            SchM_Enter_CanTp_CANTP_STATE();
        else if (operation == 8u)
            SchM_Enter_ComM_COMM_STATE();
        else
            SchM_Enter_BswM_BSWM_STATE();
        assert(CanIf_GetControllerMode(0u, &actual) == E_OK);
        if (operation == 6u)
            SchM_Exit_Dcm_DCM_STATE();
        else if (operation == 7u)
            SchM_Exit_CanTp_CANTP_STATE();
        else if (operation == 8u)
            SchM_Exit_ComM_COMM_STATE();
        else
            SchM_Exit_BswM_BSWM_STATE();
    } else {
        Can_SetMode(CAN_STOPPED);
        if (cancellation) {
            Can_DeInit();
            CanIf_DeInit();
            generation = 1u;
            initialize();
        }
    }
}
#if defined(_WIN32)
static DWORD WINAPI run_dispatch(LPVOID arg) {
    (void)arg;
    dispatch();
    return 0u;
}
static DWORD WINAPI run_cancel(LPVOID arg) {
    (void)arg;
    cancel();
    return 0u;
}
#else
static void *run_dispatch(void *arg) {
    (void)arg;
    dispatch();
    return NULL_PTR;
}
static void *run_cancel(void *arg) {
    (void)arg;
    cancel();
    return NULL_PTR;
}
#endif
int main(void) {
    const PduInfoType info = {(uint8 *)bytes, NULL_PTR, 1u};
    unsigned before;
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    Det_Init(NULL_PTR);
    for (operation = 0u; operation < 11u; ++operation) {
        for (cancellation = 0u; cancellation < 2u; ++cancellation) {
#if defined(_WIN32)
            HANDLE first, second;
#else
            pthread_t first, second;
#endif
            generation = 0u;
            initialize();
            if (operation == 1u) {
                Can_SetMode(CAN_STOPPED);
                assert(Can_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
            } else if (operation == 2u) {
#if CAN_BUSOFF_POLLING == 1
                Can_SetMode(CAN_BUS_OFF);
#endif
            } else if (operation == 3u) {
                assert(CanIf_Transmit(51u, &info) == E_OK);
#if CAN_TX_POLLING == 1
                assert(Can_HostFlush() == ECU_OK);
#endif
            } else if (operation == 4u) {
#if CAN_RX_POLLING == 1
                assert(Can_Inject(0x320u, 1u, bytes, 0u) == ECU_OK);
#endif
            }
            before = callbacks[1];
            paused = attempted = blocked = released = 0u;
            skip_unlocks = 0u;
#if CAN_TX_POLLING == 0
            if (operation == 3u)
                skip_unlocks = 2u;
#endif
            armed = 1u;
#if defined(_WIN32)
            first = CreateThread(NULL, 0u, run_dispatch, NULL, 0u, NULL);
            assert(first != NULL);
#else
            assert(pthread_create(&first, NULL_PTR, run_dispatch, NULL_PTR) == 0);
#endif
            lock();
            while (!paused)
                wait();
            unlock();
#if defined(_WIN32)
            second = CreateThread(NULL, 0u, run_cancel, NULL, 0u, NULL);
            assert(second != NULL);
#else
            assert(pthread_create(&second, NULL_PTR, run_cancel, NULL_PTR) == 0);
#endif
            lock();
            while (!attempted || !blocked)
                wait();
            /* Neither the dispatcher nor its dependency waiter owns DRIVER. */
            assert(Can_TryLock() == 1);
            __real_Can_Unlock();
            released = 1u;
            wake();
            unlock();
#if defined(_WIN32)
            assert(WaitForSingleObject(first, INFINITE) == WAIT_OBJECT_0);
            assert(WaitForSingleObject(second, INFINITE) == WAIT_OBJECT_0);
            assert(CloseHandle(first) && CloseHandle(second));
#else
            assert(pthread_join(first, NULL_PTR) == 0);
            assert(pthread_join(second, NULL_PTR) == 0);
#endif
            if (operation < 5u) {
                if (cancellation) {
                    assert(last_mode == CAN_CS_STARTED && callbacks[1] == before + 1u);
                    assert(CanIf_Transmit(51u, &info) == E_OK);
                    assert(Can_HostFlush() == ECU_OK);
                    Can_MainFunction_Write();
                    assert(callbacks[1] == before + 2u);
                } else
                    assert(last_mode == CAN_CS_STOPPED);
            }
            Can_SetMode(CAN_STOPPED);
            Can_DeInit();
        }
    }
    initialize();
    reenter = 1u;
    assert(CanIf_Transmit(51u, &info) == E_OK);
    assert(Can_HostFlush() == ECU_OK);
    Can_MainFunction_Write();
    assert(!reenter && Can_GetMode() == CAN_STARTED);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#else
    assert(pthread_cond_destroy(&changed) == 0);
    assert(pthread_mutex_destroy(&handshake) == 0);
#endif
    return 0;
}
