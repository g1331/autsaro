/** Independent native concurrency oracle: real driver and routed confirmations.
 * No standard-module substitutes, sleeps, timing scheduler or injected success.
 */
#include "Com.h"
#include "PduR.h"
#include "PduR_Com.h"
#include "PduR_CanIf.h"
#include "LSduR.h"
#include "LSduR_PduR.h"
#include "CanIf.h"
#include "Can.h"
#include "SchM_Can.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

#define ROUNDS 1000u
/* Three-phase barrier is test coordination only, independent of module state. */
typedef struct {
#if defined(_WIN32)
    CRITICAL_SECTION lock;
    CONDITION_VARIABLE changed;
#else
    pthread_mutex_t lock;
    pthread_cond_t changed;
#endif
    unsigned arrived;
    unsigned generation;
} Barrier;
static Barrier barrier;
static void rendezvous(void) {
    unsigned generation;
#if defined(_WIN32)
    EnterCriticalSection(&barrier.lock);
#else
    assert(pthread_mutex_lock(&barrier.lock) == 0);
#endif
    generation = barrier.generation;
    ++barrier.arrived;
    if (barrier.arrived == 3u) {
        barrier.arrived = 0u;
        ++barrier.generation;
#if defined(_WIN32)
        WakeAllConditionVariable(&barrier.changed);
#else
        assert(pthread_cond_broadcast(&barrier.changed) == 0);
#endif
    } else {
        while (generation == barrier.generation) {
#if defined(_WIN32)
            assert(SleepConditionVariableCS(&barrier.changed, &barrier.lock, INFINITE) != 0);
#else
            assert(pthread_cond_wait(&barrier.changed, &barrier.lock) == 0);
#endif
        }
    }
#if defined(_WIN32)
    LeaveCriticalSection(&barrier.lock);
#else
    assert(pthread_mutex_unlock(&barrier.lock) == 0);
#endif
}
typedef struct {
    unsigned index;
    unsigned accepted;
    unsigned rejected;
} Worker;
static Worker workers[2] = {{0u, 0u, 0u}, {1u, 0u, 0u}};
/* Callbacks run under the actual common CanIf/driver resource. Read only after
 * both producer threads reach the submitted barrier; never protect with a
 * second lock that could invert driver callback ordering.
 */
static unsigned successes[2];
static unsigned failures[2];
static unsigned frames[2];
static unsigned offline_errors;
static unsigned mode_indications;
const EcuPolicyConfig Ecu_Policy = {ECU_TX_SYNCHRONOUS,
                                    ECU_RX_BEFORE_DEADLINE,
                                    0u,
                                    0u,
                                    ECU_WAIT_ABORT,
                                    0u,
                                    0u,
                                    NULL_PTR,
                                    0u,
                                    50u,
                                    5000u};
static unsigned index_for(PduIdType id) {
    assert(id == 1u || id == 2u);
    return (id == 1u) ? 0u : 1u;
}
static void confirmed(PduIdType id, Std_ReturnType result) {
    const unsigned index = index_for(id);
    Com_TxConfirmation(id, result);
    if (result == E_OK) {
        ++successes[index];
    } else {
        assert(result == E_NOT_OK);
        ++failures[index];
    }
}
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8]) {
    unsigned index;
    CanIf_PduModeType mode = CANIF_OFFLINE;
    assert(id == 0x456u || id == 0x457u);
    index = (id == 0x456u) ? 0u : 1u;
    assert(length == 4u);
    assert(data[0] == (uint8)(index + 1u) && data[1] == 0x5au && data[2] == 0xa5u &&
           data[3] == 0xffu);
    /* Actual output callback re-enters the same resource, proving recursion. */
    assert(CanIf_GetPduMode(0u, &mode) == E_OK && mode == CANIF_ONLINE);
    ++frames[index];
    return ECU_OK;
}
static void mode(uint8 id, Can_ControllerStateType state) {
    assert(id == 0u && (state == CAN_CS_STARTED || state == CAN_CS_STOPPED));
    ++mode_indications;
}
static void bus_off(uint8 id) { assert(id == 0u); }
static Std_ReturnType error(uint16 module, uint8 instance, uint8 api, uint8 code) {
    assert(module == 60u && instance == 0u && api == 0x49u && code == 70u);
    ++offline_errors;
    return E_NOT_OK;
}
static const Det_ErrorHookType hooks[] = {error};
static const Det_ConfigType det = {NULL_PTR, 0u, hooks, 1u};
static const Com_PduConfigType com_pdus[] = {{1u, 11u, FALSE, 0u, 0u, 0u},
                                             {2u, 12u, FALSE, 0u, 0u, 0u}};
static const Com_ConfigType com = {com_pdus, 2u, 0u, NULL_PTR, NULL_PTR, PduR_ComTransmit};
static const PduR_TxRouteType pdur_tx[] = {
    {PDUR_UP_COM, 1u, 41u, LSduR_PduRTransmit, confirmed, Com_TriggerTransmit},
    {PDUR_UP_COM, 2u, 42u, LSduR_PduRTransmit, confirmed, Com_TriggerTransmit}};
static const PduR_PBConfigType pdur = {23u, NULL_PTR, 0u, pdur_tx, 2u, NULL_PTR, 0u};
static const LSduR_TxRouteType ls_tx[] = {
    {LSDUR_UP_PDUR, 41u, 51u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit},
    {LSDUR_UP_PDUR, 42u, 61u, PduR_CanIfTxConfirmation, PduR_CanIfTriggerTransmit}};
static const LSduR_PBConfigType ls = {29u, NULL_PTR, 0u, ls_tx, 2u};
static const CanIf_TxPduConfigType tx[] = {{0x456u, 51u, 4u}, {0x457u, 61u, 4u}};
static const CanIf_ConfigType canif = {NULL_PTR, 0u, tx, 2u, mode, bus_off};
static const Can_ConfigType can = {sink};
static void produce(Worker *worker) {
    unsigned round;
    uint8 bytes[4] = {(uint8)(worker->index + 1u), 0x5au, 0xa5u, 0xffu};
    const PduInfoType info = {bytes, NULL_PTR, 4u};
    const PduIdType id = (worker->index == 0u) ? 51u : 61u;
    for (round = 0u; round < ROUNDS; ++round) {
        rendezvous();
        if (CanIf_Transmit(id, &info) == E_OK) {
            ++worker->accepted;
        } else {
            ++worker->rejected;
        }
        rendezvous();
        rendezvous();
    }
}
#if defined(_WIN32)
static DWORD WINAPI run(LPVOID argument) {
    produce(argument);
    return 0u;
}
#else
static void *run(void *argument) {
    produce(argument);
    return NULL_PTR;
}
#endif
int main(void) {
    unsigned round;
    unsigned index;
    unsigned previous_accepted = 0u;
#if defined(_WIN32)
    HANDLE threads[2];
    InitializeCriticalSection(&barrier.lock);
    InitializeConditionVariable(&barrier.changed);
#else
    pthread_t threads[2];
    assert(pthread_mutex_init(&barrier.lock, NULL_PTR) == 0);
    assert(pthread_cond_init(&barrier.changed, NULL_PTR) == 0);
#endif
    Det_Init(&det);
    Com_Init(&com);
    PduR_Init(&pdur);
    LSduR_Init(&ls);
    CanIf_Init(&canif);
    Can_Init(&can);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
    Can_MainFunction_Wakeup();
    assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
    for (index = 0u; index < 2u; ++index) {
#if defined(_WIN32)
        threads[index] = CreateThread(NULL, 0u, run, &workers[index], 0u, NULL);
        assert(threads[index] != NULL);
#else
        assert(pthread_create(&threads[index], NULL_PTR, run, &workers[index]) == 0);
#endif
    }
    for (round = 0u; round < ROUNDS; ++round) {
        const boolean stop = ((round % 4u) >= 2u);
        unsigned accepted;
        rendezvous();
        if ((round % 4u) == 1u) {
            assert(Can_HostFlush() == ECU_OK); /* May confirm while producers execute. */
        } else if ((round % 4u) == 2u) {
            assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
            Can_MainFunction_Wakeup(); /* Stop interleaves with active different-PDU writers. */
        }
        rendezvous();
        accepted = workers[0].accepted + workers[1].accepted;
        if ((round % 4u) == 3u) {
            /* Defer completion: exactly one producer accepted, other gets real BUSY. */
            assert(accepted == previous_accepted + 1u);
            assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
            Can_MainFunction_Wakeup();
        }
        if (stop == FALSE) {
            assert(Can_HostFlush() == ECU_OK);
        }
        if ((round % 4u) == 0u) {
            assert(accepted == previous_accepted + 1u);
        }
        for (index = 0u; index < 2u; ++index) {
            assert(successes[index] + failures[index] == workers[index].accepted);
            assert(frames[index] == successes[index]);
            assert(workers[index].accepted + workers[index].rejected == round + 1u);
        }
        previous_accepted = accepted;
        if (stop == TRUE) {
            assert(Can_HostFlush() == ECU_OK);
            assert(CanIf_SetControllerMode(0u, CAN_CS_STARTED) == E_OK);
            Can_MainFunction_Wakeup();
            assert(CanIf_SetPduMode(0u, CANIF_ONLINE) == E_OK);
        }
        rendezvous();
    }
#if defined(_WIN32)
    assert(WaitForMultipleObjects(2u, threads, TRUE, INFINITE) == WAIT_OBJECT_0);
    assert(CloseHandle(threads[0]) != 0 && CloseHandle(threads[1]) != 0);
    DeleteCriticalSection(&barrier.lock);
#else
    assert(pthread_join(threads[0], NULL_PTR) == 0 && pthread_join(threads[1], NULL_PTR) == 0);
    assert(pthread_cond_destroy(&barrier.changed) == 0);
    assert(pthread_mutex_destroy(&barrier.lock) == 0);
#endif
    assert(successes[0] + successes[1] >= 500u);
    assert(failures[0] + failures[1] >= 250u);
    assert(mode_indications == 1001u);
    /* Offline refusals may vary with scheduling; exact accepted-result identity does not. */
    assert(offline_errors <= workers[0].rejected + workers[1].rejected);
    assert(CanIf_SetControllerMode(0u, CAN_CS_STOPPED) == E_OK);
    Can_MainFunction_Wakeup();
    Com_DeInit();
    CanIf_DeInit();
    Can_DeInit();
    return 0;
}
