/** @file Standard CanIf for one Classical CAN controller and immutable routes. */
#include "CanIf.h"
#include "Can.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(CAN_AR_RELEASE_MAJOR_VERSION) || !defined(CAN_AR_RELEASE_MINOR_VERSION)
#error "CanIf.c -> Can.h: missing AUTOSAR release"
#elif (CAN_AR_RELEASE_MAJOR_VERSION != 4U) || (CAN_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.c -> Can.h: incompatible AUTOSAR release"
#endif
#include "Det.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(DET_AR_RELEASE_MAJOR_VERSION) || !defined(DET_AR_RELEASE_MINOR_VERSION)
#error "CanIf.c -> Det.h: missing AUTOSAR release"
#elif (DET_AR_RELEASE_MAJOR_VERSION != 4U) || (DET_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.c -> Det.h: incompatible AUTOSAR release"
#endif
#include "LSduR_CanIf.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(LSDUR_AR_RELEASE_MAJOR_VERSION) || !defined(LSDUR_AR_RELEASE_MINOR_VERSION)
#error "CanIf.c -> LSduR_CanIf.h: missing AUTOSAR release"
#elif (LSDUR_AR_RELEASE_MAJOR_VERSION != 4U) || (LSDUR_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.c -> LSduR_CanIf.h: incompatible AUTOSAR release"
#endif
#include "SchM_CanIf.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "CanIf.c -> SchM_CanIf.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.c -> SchM_CanIf.h: incompatible AUTOSAR release"
#endif
#define CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"
/* CANIF_STATE protects every object below. Init and DeInit clear the configuration/controller state
 * and all Tx ledgers; generation and mode_writes advance instead of resetting, and stops is
 * preserved. */
static const CanIf_ConfigType *configuration
    CANIF_VAR_CLEARED; /**< Init installs or clears; DeInit clears. */
static Can_ControllerStateType controller_mode
    CANIF_VAR_CLEARED; /**< Last indicated mode; Init sets STOPPED or UNINIT. */
static CanIf_PduModeType pdu_mode
    CANIF_VAR_CLEARED; /**< Current channel gating; Init sets OFFLINE. */
/* Tx ledger per L-PDU. Driver calls run outside the area, so a request is
 * reserved before Can_Write and settled after it: `writing` counts reservations whose
 * Can_Write has not returned, `early` the confirmations that already answered one of
 * them, `accepted` settled requests awaiting their result, and `owed` E_NOT_OK results of
 * cancelled requests still to be delivered. A request gets exactly one result. */
static uint32 accepted[32] CANIF_VAR_CLEARED; /**< Accepted requests per L-PDU; Init resets. */
static uint32 owed[32] CANIF_VAR_CLEARED;     /**< Cancelled results owed; Init resets. */
static uint8 writing[32] CANIF_VAR_CLEARED;   /**< Unsettled driver calls; Init resets. */
static uint8 early[32] CANIF_VAR_CLEARED; /**< Confirmations before driver return; Init resets. */
/* Init/DeInit end every request silently; a stop cancels what was accepted before it;
 * every PDU mode write lets a check-then-set request detect a newer decision. */
static uint32 generation CANIF_VAR_CLEARED;  /**< Configuration lifetime; Init increments. */
static uint32 stops CANIF_VAR_CLEARED;       /**< Stop generation; Init preserves. */
static uint32 mode_writes CANIF_VAR_CLEARED; /**< PDU-mode generation; Init increments. */
#define CANIF_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "CanIf.c -> CanIf_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "CanIf.c -> CanIf_MemMap.h: incompatible AUTOSAR release"
#endif
#define CANIF_START_SEC_CODE
#include "CanIf_MemMap.h"
static CANIF_CODE boolean CanIf_Valid(const CanIf_ConfigType *config)
{
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->receive_count <= 32u) && (config->transmit_count <= 32u) &&
        ((config->receive_count == 0u) || (config->receive != NULL_PTR)) &&
        ((config->transmit_count == 0u) || (config->transmit != NULL_PTR)) &&
        (config->controller_mode != NULL_PTR) && (config->bus_off != NULL_PTR)) {
        valid = TRUE;
        for (i = 0u; i < config->receive_count; ++i) {
            uint16 j;
            const CanIf_RxPduConfigType *route = &config->receive[i];
            if ((route->can_id > 0x7ffu) || (route->minimum_length > 8u)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if ((route->can_id == config->receive[j].can_id) ||
                    (route->pdu == config->receive[j].pdu)) {
                    valid = FALSE;
                }
            }
        }
        for (i = 0u; i < config->transmit_count; ++i) {
            uint16 j;
            const CanIf_TxPduConfigType *route = &config->transmit[i];
            if ((route->can_id > 0x7ffu) || (route->maximum_length == 0u) ||
                (route->maximum_length > 8u)) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if (route->pdu == config->transmit[j].pdu) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
/* Called inside the area: start a new lifetime with no request outstanding. */
static CANIF_CODE void CanIf_Reset(void)
{
    uint16 i;
    ++generation;
    ++mode_writes;
    pdu_mode = CANIF_OFFLINE;
    for (i = 0u; i < 32u; ++i) {
        accepted[i] = 0u;
        owed[i] = 0u;
        writing[i] = 0u;
        early[i] = 0u;
    }
}
/* Called inside the area: the controller stopped, so every accepted request fails once
 * (SWS_CANIF_00739); requests still in Can_Write settle against the stop count. */
static CANIF_CODE void CanIf_CancelAccepted(void)
{
    uint16 i;
    ++stops;
    for (i = 0u; i < configuration->transmit_count; ++i) {
        owed[i] += accepted[i];
        accepted[i] = 0u;
    }
}
/* Deliver owed E_NOT_OK results one claim at a time, outside the area. */
static CANIF_CODE void CanIf_DeliverOwed(void)
{
    uint8 more = 1u;
    while (more != 0u) {
        PduIdType id = 0u;
        more = 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        if (configuration != NULL_PTR) {
            uint16 i;
            for (i = 0u; (i < configuration->transmit_count) && (more == 0u); ++i) {
                if (owed[i] != 0u) {
                    --owed[i];
                    id = configuration->transmit[i].pdu;
                    more = 1u;
                }
            }
        }
        SchM_Exit_CanIf_CANIF_STATE();
        if (more != 0u) {
            LSduR_CanIfTxConfirmation(id, E_NOT_OK);
        }
    }
}
/* An invalid set leaves CanIf uninitialized, never on a previous set (SWS_CANIF_00661). */
CANIF_CODE void CanIf_Init(const CanIf_ConfigType *ConfigPtr)
{
    SchM_Enter_CanIf_CANIF_STATE();
    if (CanIf_Valid(ConfigPtr) == TRUE) {
        configuration = ConfigPtr;
        controller_mode = CAN_CS_STOPPED;
    } else {
        configuration = NULL_PTR;
        controller_mode = CAN_CS_UNINIT;
    }
    CanIf_Reset();
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE void CanIf_DeInit(void)
{
    SchM_Enter_CanIf_CANIF_STATE();
    configuration = NULL_PTR;
    controller_mode = CAN_CS_UNINIT;
    CanIf_Reset();
    SchM_Exit_CanIf_CANIF_STATE();
}
/* Called inside the area: the driver controller of a configured CanIf controller. */
static CANIF_CODE uint8 CanIf_Driver(uint8 ControllerId, uint8 *driver)
{
    uint8 valid = 0u;
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller)) {
        *driver = configuration->driver_controller;
        valid = 1u;
    }
    return valid;
}
CANIF_CODE Std_ReturnType CanIf_SetControllerMode(uint8 ControllerId,
                                                  Can_ControllerStateType ControllerMode)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 driver = 0u;
    uint8 valid;
    uint32 decision;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    decision = mode_writes;
    SchM_Exit_CanIf_CANIF_STATE();
    if (valid != 0u) {
        result = Can_SetControllerMode(driver, ControllerMode);
    }
    if (result == E_OK) {
        SchM_Enter_CanIf_CANIF_STATE();
        /* A PDU mode decided while the driver call ran is newer than this one. */
        if (decision == mode_writes) {
            if (ControllerMode == CAN_CS_STOPPED) {
                pdu_mode = CANIF_TX_OFFLINE;
                ++mode_writes;
            } else if (ControllerMode == CAN_CS_SLEEP) {
                pdu_mode = CANIF_OFFLINE;
                ++mode_writes;
            } else {
                /* STARTED leaves the PDU mode to its completion. */
            }
        }
        SchM_Exit_CanIf_CANIF_STATE();
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerMode(uint8 ControllerId,
                                                  Can_ControllerStateType *ControllerModePtr)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 driver = 0u;
    uint8 valid;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    SchM_Exit_CanIf_CANIF_STATE();
    if ((valid != 0u) && (ControllerModePtr != NULL_PTR)) {
        result = Can_GetControllerMode(driver, ControllerModePtr);
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerErrorState(uint8 ControllerId,
                                                        Can_ErrorStateType *ErrorStatePtr)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 driver = 0u;
    uint8 valid;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    SchM_Exit_CanIf_CANIF_STATE();
    if ((valid != 0u) && (ErrorStatePtr != NULL_PTR)) {
        result = Can_GetControllerErrorState(driver, ErrorStatePtr);
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerRxErrorCounter(uint8 ControllerId,
                                                            uint8 *RxErrorCounterPtr)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 driver = 0u;
    uint8 valid;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    SchM_Exit_CanIf_CANIF_STATE();
    if ((valid != 0u) && (RxErrorCounterPtr != NULL_PTR)) {
        result = Can_GetControllerRxErrorCounter(driver, RxErrorCounterPtr);
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetControllerTxErrorCounter(uint8 ControllerId,
                                                            uint8 *TxErrorCounterPtr)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 driver = 0u;
    uint8 valid;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    SchM_Exit_CanIf_CANIF_STATE();
    if ((valid != 0u) && (TxErrorCounterPtr != NULL_PTR)) {
        result = Can_GetControllerTxErrorCounter(driver, TxErrorCounterPtr);
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_SetPduMode(uint8 ControllerId, CanIf_PduModeType PduModeRequest)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 retry = 1u;
    while (retry != 0u) {
        Can_ControllerStateType actual_mode = CAN_CS_UNINIT;
        uint8 driver = 0u;
        uint8 valid;
        uint32 decision;
        uint32 lifetime;
        retry = 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        valid = CanIf_Driver(ControllerId, &driver);
        decision = mode_writes;
        lifetime = generation;
        SchM_Exit_CanIf_CANIF_STATE();
        if (valid != 0u) {
            if ((Can_GetControllerMode(driver, &actual_mode) == E_OK) &&
                (actual_mode == CAN_CS_STARTED) &&
                ((PduModeRequest == CANIF_OFFLINE) || (PduModeRequest == CANIF_TX_OFFLINE) ||
                 (PduModeRequest == CANIF_ONLINE))) {
                SchM_Enter_CanIf_CANIF_STATE();
                if (lifetime != generation) {
                    /* Reinitialized meanwhile: the request belonged to the old lifetime. */
                } else if (decision != mode_writes) {
                    /* A mode change interleaved: decide again against the actual controller. */
                    retry = 1u;
                } else {
                    pdu_mode = PduModeRequest;
                    ++mode_writes;
                    result = E_OK;
                }
                SchM_Exit_CanIf_CANIF_STATE();
            }
        }
    }
    return result;
}
CANIF_CODE Std_ReturnType CanIf_GetPduMode(uint8 ControllerId, CanIf_PduModeType *PduModePtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        (PduModePtr != NULL_PTR)) {
        *PduModePtr = pdu_mode;
        result = E_OK;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
CANIF_CODE Std_ReturnType CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    uint8 errors[2] = {0u, 0u};
    uint8 reserved = 0u;
    uint16 index = 0u;
    Can_HwHandleType hoh = 0u;
    Can_PduType pdu = {TxPduId, 0u, 0u, NULL_PTR};
    uint32 lifetime = 0u;
    uint32 stop = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR)) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            const CanIf_TxPduConfigType *route = &configuration->transmit[i];
            if (route->pdu == TxPduId) {
                if (pdu_mode == CANIF_OFFLINE) {
                    errors[0] = CANIF_E_STOPPED;
                } else if (pdu_mode != CANIF_ONLINE) {
                    /* TX_OFFLINE rejects without CANIF_E_STOPPED (SWS_CANIF_00382, 00489). */
                } else if (PduInfoPtr->SduLength > 8u) {
                    errors[0] = CANIF_E_DATA_LENGTH_MISMATCH;
                    errors[1] = CANIF_E_TXPDU_LENGTH_EXCEEDED;
                } else if (PduInfoPtr->SduLength > route->maximum_length) {
                    errors[0] = CANIF_E_TXPDU_LENGTH_EXCEEDED;
                } else if ((controller_mode == CAN_CS_STARTED) && (owed[i] == 0u) &&
                           (accepted[i] < (UINT32_MAX - 255u)) && (writing[i] < 255u)) {
                    /* Earlier results of this PDU are delivered before a new request. */
                    ++writing[i];
                    reserved = 1u;
                    index = i;
                    hoh = configuration->transmit_hoh;
                    pdu.length = (uint8)PduInfoPtr->SduLength;
                    pdu.id = route->can_id;
                    pdu.sdu = PduInfoPtr->SduDataPtr;
                    lifetime = generation;
                    stop = stops;
                } else {
                    /* Not started, or results of earlier requests are still owed. */
                }
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    if (errors[0] != 0u) {
        (void)Det_ReportRuntimeError(CANIF_MODULE_ID, 0u, CANIF_SID_TRANSMIT, errors[0]);
    }
    if (errors[1] != 0u) {
        (void)Det_ReportRuntimeError(CANIF_MODULE_ID, 0u, CANIF_SID_TRANSMIT, errors[1]);
    }
    if (reserved != 0u) {
        const Std_ReturnType written = Can_Write(hoh, &pdu);
        SchM_Enter_CanIf_CANIF_STATE();
        if (lifetime == generation) {
            --writing[index];
            if (written != E_OK) {
                /* Refused: no result follows, and no confirmation can answer it. */
                if (early[index] > writing[index]) {
                    early[index] = writing[index];
                }
            } else if (early[index] != 0u) {
                --early[index]; /* Its confirmation already arrived. */
            } else if (stop != stops) {
                ++owed[index]; /* Accepted by the driver, then cancelled by a stop. */
            } else {
                ++accepted[index];
            }
        }
        SchM_Exit_CanIf_CANIF_STATE();
        result = (written == E_OK) ? E_OK : E_NOT_OK; /* CAN_BUSY is not a CanIf result. */
        CanIf_DeliverOwed();
    }
    return result;
}
CANIF_CODE void CanIf_TxConfirmation(PduIdType CanTxPduId)
{
    uint8 deliver = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    if (configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < configuration->transmit_count; ++i) {
            if (configuration->transmit[i].pdu == CanTxPduId) {
                uint8 answered = 1u;
                if (accepted[i] != 0u) {
                    --accepted[i];
                } else if (early[i] < writing[i]) {
                    ++early[i]; /* Confirmed before its Can_Write returned. */
                } else {
                    answered = 0u; /* Late: its request already has a result. */
                }
                /* The frame left the controller: a disabled Tx path suppresses its
                 * confirmation (SWS_CANIF_00073, 00489), and STOPPED must not later
                 * report it as failed (SWS_CANIF_00739). */
                deliver = ((answered != 0u) && (pdu_mode == CANIF_ONLINE)) ? 1u : 0u;
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    if (deliver != 0u) {
        LSduR_CanIfTxConfirmation(CanTxPduId, E_OK);
    }
}
CANIF_CODE void CanIf_ControllerModeIndication(uint8 ControllerId,
                                               Can_ControllerStateType ControllerMode)
{
    CanIf_ModeNotificationType notify = NULL_PTR;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (ControllerId == configuration->controller) &&
        ((ControllerMode == CAN_CS_STARTED) || (ControllerMode == CAN_CS_STOPPED) ||
         (ControllerMode == CAN_CS_SLEEP))) {
        controller_mode = ControllerMode;
        if (ControllerMode == CAN_CS_STOPPED) {
            pdu_mode = CANIF_TX_OFFLINE;
            ++mode_writes;
            CanIf_CancelAccepted();
        } else if (ControllerMode == CAN_CS_SLEEP) {
            pdu_mode = CANIF_OFFLINE;
            ++mode_writes;
        } else {
            /* STARTED keeps the PDU mode until the upper layer sets it. */
        }
        notify = configuration->controller_mode;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    CanIf_DeliverOwed();
    if (notify != NULL_PTR) {
        notify(ControllerId, ControllerMode);
    }
}
CANIF_CODE void CanIf_ControllerBusOff(uint8 ControllerId)
{
    CanIf_BusOffNotificationType notify = NULL_PTR;
    Can_ControllerStateType actual_mode = CAN_CS_UNINIT;
    uint8 driver = 0u;
    uint8 valid;
    uint32 lifetime;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    lifetime = generation;
    SchM_Exit_CanIf_CANIF_STATE();
    if (valid != 0u) {
        /* The host driver enters STOPPED on bus-off without a separate mode
         * indication. Converge only from its actual reported transition.
         */
        const uint8 stopped = ((Can_GetControllerMode(driver, &actual_mode) == E_OK) &&
                               (actual_mode == CAN_CS_STOPPED))
                                  ? 1u
                                  : 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        if (lifetime == generation) {
            pdu_mode = CANIF_TX_OFFLINE;
            ++mode_writes;
            if ((stopped != 0u) && (controller_mode != CAN_CS_STOPPED)) {
                controller_mode = CAN_CS_STOPPED;
                CanIf_CancelAccepted();
            }
            notify = configuration->bus_off;
        }
        SchM_Exit_CanIf_CANIF_STATE();
    }
    CanIf_DeliverOwed();
    if (notify != NULL_PTR) {
        notify(ControllerId);
    }
}
CANIF_CODE void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr)
{
    uint8 deliver = 0u;
    uint8 invalid_length = 0u;
    PduIdType id = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((configuration != NULL_PTR) && (Mailbox != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) &&
        (Mailbox->ControllerId == configuration->controller) &&
        /* Standard identifier without FD flag: STANDARD_NO_FD_CAN (SWS_CANIF_00877). */
        (Mailbox->Hoh == configuration->receive_hoh) && (Mailbox->CanId <= 0x7ffu) &&
        (controller_mode == CAN_CS_STARTED) &&
        ((pdu_mode == CANIF_ONLINE) || (pdu_mode == CANIF_TX_OFFLINE))) {
        uint16 i;
        for (i = 0u; i < configuration->receive_count; ++i) {
            const CanIf_RxPduConfigType *route = &configuration->receive[i];
            if (route->can_id == Mailbox->CanId) {
                if ((PduInfoPtr->SduLength < route->minimum_length) ||
                    (PduInfoPtr->SduLength > 8u)) {
                    invalid_length = 1u;
                } else {
                    id = route->pdu;
                    deliver = 1u;
                }
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    if (invalid_length != 0u) {
        (void)Det_ReportRuntimeError(
            CANIF_MODULE_ID, 0u, CANIF_SID_RX_INDICATION, CANIF_E_INVALID_DATA_LENGTH);
    }
    if (deliver != 0u) {
        LSduR_CanIfRxIndication(id, PduInfoPtr);
    }
}
#define CANIF_STOP_SEC_CODE
#include "CanIf_MemMap.h"
