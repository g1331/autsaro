/** @file Standard CanIf for configured CAN controllers and immutable routes. */
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
#include "Ecu_ProfileConfig.h"
#define CANIF_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "CanIf_MemMap.h"
/* CANIF_STATE protects every object below. Init and DeInit clear the configuration/controller state
 * and all Tx ledgers; generation and mode_writes advance instead of resetting, and stops is
 * preserved. */
static const CanIf_ConfigType *canif_configuration
    CANIF_VAR_CLEARED; /**< Init installs or clears; DeInit clears. */
typedef struct {
    Can_ControllerStateType mode; /**< Latest indicated controller mode. */
    CanIf_PduModeType pdu_mode;   /**< Channel-local admission mode. */
    uint32 stops;                 /**< Number of cancellations, preserved across Init. */
    uint32 mode_writes;           /**< PDU-mode revision. */
} CanIf_ControllerStateType;
static CanIf_ControllerStateType
    canif_controllers[CANIF_CONTROLLER_CAPACITY] CANIF_VAR_CLEARED; /**< Controller-local state,
                                                                       reset by Init and DeInit. */
/* Tx ledger per L-PDU. Driver calls run outside the area, so a request is
 * reserved before Can_Write and settled after it: `writing` counts reservations whose
 * Can_Write has not returned, `early` the confirmations that already answered one of
 * them, `accepted` settled requests awaiting their result, and `owed` E_NOT_OK results of
 * cancelled requests still to be delivered. A request gets exactly one result. */
static uint32 canif_accepted[CANIF_TX_CAPACITY] CANIF_VAR_CLEARED; /**< Accepted requests per L-PDU;
                                                                      Init resets. */
static uint32
    owed[CANIF_TX_CAPACITY] CANIF_VAR_CLEARED; /**< Cancelled results owed; Init resets. */
static uint8
    writing[CANIF_TX_CAPACITY] CANIF_VAR_CLEARED; /**< Unsettled driver calls; Init resets. */
static uint8 early[CANIF_TX_CAPACITY] CANIF_VAR_CLEARED; /**< Confirmations before driver return;
                                                            Init resets. */
/* Init/DeInit end every request silently; a stop cancels what was accepted before it;
 * every PDU mode write lets a check-then-set request detect a newer decision. */
static uint32 canif_generation CANIF_VAR_CLEARED; /**< Configuration lifetime; Init increments. */
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
static CANIF_CODE boolean CanIf_Matches(const CanIf_RxPduConfigType *route, Can_IdType id)
{
    /* SWS_CANIF_00877: for a dual policy ignore only the FD bit. */
    return (route->can_id == id) ||
           (route->accepts_fd && (route->can_id == (id & ~CAN_ID_FD_FLAG)));
}

static CANIF_CODE uint16 CanIf_Controller(const CanIf_ConfigType *config, uint8 id)
{
    uint16 result = CANIF_CONTROLLER_CAPACITY;
    if (config != NULL_PTR) {
        for (uint16 i = 0u; (i < CANIF_CONTROLLER_CAPACITY) && (i < config->controller_count);
             ++i) {
            if (config->controllers[i].controller == id) {
                result = i;
                break;
            }
        }
    }
    return result;
}

static CANIF_CODE boolean CanIf_ValidMapping(uint8 offset,
                                             uint8 pdu_length,
                                             uint8 frame_length,
                                             uint8 maximum)
{
    const uint16 end = (uint16)offset + (uint16)pdu_length;
    boolean valid;
    if (frame_length == 0u) {
        valid = (offset == 0u);
    } else {
        valid = (pdu_length != 0u) && (end <= frame_length) && (frame_length <= maximum);
    }
    return valid;
}

static CANIF_CODE boolean CanIf_ValidRx(const CanIf_ConfigType *config)
{
    boolean valid = (boolean)TRUE;
    for (uint16 i = 0u; (i < config->receive_count) && valid; ++i) {
        const CanIf_RxPduConfigType *route = &config->receive[i];
        const boolean fd_only = (route->can_id & CAN_ID_FD_FLAG) != 0u;
        const uint8 maximum =
            (fd_only || route->accepts_fd) ? CAN_FD_MAX_LENGTH : CAN_CLASSICAL_MAX_LENGTH;
        valid = Can_IsValidId(route->can_id) && ((!fd_only) || (!route->accepts_fd)) &&
                (route->minimum_length <= maximum) &&
                CanIf_ValidMapping(
                    route->frame_offset, route->minimum_length, route->frame_length, maximum) &&
                (CanIf_Controller(config, route->controller) < CANIF_CONTROLLER_CAPACITY);
        for (uint16 j = 0u; (j < i) && valid; ++j) {
            const CanIf_RxPduConfigType *other = &config->receive[j];
            valid = (route->pdu != other->pdu) && ((route->controller != other->controller) ||
                                                   ((!CanIf_Matches(route, other->can_id)) &&
                                                    (!CanIf_Matches(other, route->can_id))));
        }
    }
    return valid;
}

static CANIF_CODE boolean CanIf_ValidTx(const CanIf_ConfigType *config)
{
    boolean valid = (boolean)TRUE;
    for (uint16 i = 0u; (i < config->transmit_count) && valid; ++i) {
        const CanIf_TxPduConfigType *route = &config->transmit[i];
        const uint8 maximum =
            ((route->can_id & CAN_ID_FD_FLAG) != 0u) ? CAN_FD_MAX_LENGTH : CAN_CLASSICAL_MAX_LENGTH;
        valid = Can_IsValidId(route->can_id) && (route->maximum_length != 0u) &&
                (route->maximum_length <= maximum) &&
                CanIf_ValidMapping(
                    route->frame_offset, route->maximum_length, route->frame_length, maximum) &&
                (CanIf_Controller(config, route->controller) < CANIF_CONTROLLER_CAPACITY);
        for (uint16 j = 0u; (j < i) && valid; ++j) {
            valid = route->pdu != config->transmit[j].pdu;
        }
    }
    return valid;
}

static CANIF_CODE boolean CanIf_Valid(const CanIf_ConfigType *config)
{
    boolean valid = (config != NULL_PTR) && (config->receive_count <= CANIF_RX_CAPACITY) &&
                    (config->transmit_count <= CANIF_TX_CAPACITY) &&
                    ((config->receive_count == 0u) || (config->receive != NULL_PTR)) &&
                    ((config->transmit_count == 0u) || (config->transmit != NULL_PTR)) &&
                    (config->controller_mode != NULL_PTR) && (config->bus_off != NULL_PTR) &&
                    (config->controllers != NULL_PTR) && (config->controller_count != 0u) &&
                    (config->controller_count <= CANIF_CONTROLLER_CAPACITY);
    if (valid) {
        for (uint16 i = 0u; (i < config->controller_count) && valid; ++i) {
            const CanIf_ControllerConfigType *controller = &config->controllers[i];
            for (uint16 j = 0u; (j < i) && valid; ++j) {
                valid = (controller->controller != config->controllers[j].controller) &&
                        (controller->driver_controller != config->controllers[j].driver_controller);
            }
        }
        valid = valid && CanIf_ValidRx(config) && CanIf_ValidTx(config);
    }
    return valid;
}
/* Called inside the area: start a new lifetime with no request outstanding. */
static CANIF_CODE void CanIf_Reset(void)
{
    uint16 i;
    ++canif_generation;
    for (i = 0u; i < CANIF_CONTROLLER_CAPACITY; ++i) {
        ++canif_controllers[i].mode_writes;
        canif_controllers[i].pdu_mode = CANIF_OFFLINE;
        canif_controllers[i].mode = CAN_CS_STOPPED;
    }
    for (i = 0u; i < CANIF_TX_CAPACITY; ++i) {
        canif_accepted[i] = 0u;
        owed[i] = 0u;
        writing[i] = 0u;
        early[i] = 0u;
    }
}
/* Called inside the area: the controller stopped, so every accepted request fails once
 * (SWS_CANIF_00739); requests still in Can_Write settle against the stop count. */
static CANIF_CODE void CanIf_CancelAccepted(uint8 controller, CanIf_ControllerStateType *state)
{
    uint16 i;
    ++state->stops;
    for (i = 0u; (i < CANIF_TX_CAPACITY) && (i < canif_configuration->transmit_count); ++i) {
        if (canif_configuration->transmit[i].controller == controller) {
            owed[i] += canif_accepted[i];
            canif_accepted[i] = 0u;
        }
    }
}
/* Deliver owed E_NOT_OK results one claim at a time, outside the area. */
static CANIF_CODE void CanIf_DeliverOwed(uint8 controller)
{
    uint8 more = 1u;
    while (more != 0u) {
        PduIdType id = 0u;
        more = 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        if (canif_configuration != NULL_PTR) {
            uint16 i;
            i = 0u;
            while ((i < CANIF_TX_CAPACITY) && (i < canif_configuration->transmit_count) &&
                   (more == 0u)) {
                if ((owed[i] != 0u) &&
                    (canif_configuration->transmit[i].controller == controller)) {
                    --owed[i];
                    id = canif_configuration->transmit[i].pdu;
                    more = 1u;
                }
                ++i;
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
        canif_configuration = ConfigPtr;
    } else {
        canif_configuration = NULL_PTR;
    }
    CanIf_Reset();
    SchM_Exit_CanIf_CANIF_STATE();
}
CANIF_CODE void CanIf_DeInit(void)
{
    SchM_Enter_CanIf_CANIF_STATE();
    canif_configuration = NULL_PTR;
    CanIf_Reset();
    SchM_Exit_CanIf_CANIF_STATE();
}
/* Called inside the area: the driver controller of a configured CanIf controller. */
static CANIF_CODE uint8 CanIf_Driver(uint8 ControllerId, uint8 *driver)
{
    uint8 valid = 0u;
    const uint16 index = CanIf_Controller(canif_configuration, ControllerId);
    if (index < CANIF_CONTROLLER_CAPACITY) {
        *driver = canif_configuration->controllers[index].driver_controller;
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
    uint32 decision = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    valid = CanIf_Driver(ControllerId, &driver);
    const uint16 slot = CanIf_Controller(canif_configuration, ControllerId);
    if (slot < CANIF_CONTROLLER_CAPACITY) {
        decision = canif_controllers[slot].mode_writes;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    if (valid != 0u) {
        result = Can_SetControllerMode(driver, ControllerMode);
    }
    if ((result == E_OK) && (slot < CANIF_CONTROLLER_CAPACITY)) {
        SchM_Enter_CanIf_CANIF_STATE();
        /* A PDU mode decided while the driver call ran is newer than this one. */
        CanIf_ControllerStateType *state = &canif_controllers[slot];
        if (decision == state->mode_writes) {
            if (ControllerMode == CAN_CS_STOPPED) {
                state->pdu_mode = CANIF_TX_OFFLINE;
                ++state->mode_writes;
            } else if (ControllerMode == CAN_CS_SLEEP) {
                state->pdu_mode = CANIF_OFFLINE;
                ++state->mode_writes;
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
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
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
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
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
        uint32 decision = 0u;
        uint32 lifetime;
        retry = 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        valid = CanIf_Driver(ControllerId, &driver);
        const uint16 slot = CanIf_Controller(canif_configuration, ControllerId);
        if (slot < CANIF_CONTROLLER_CAPACITY) {
            decision = canif_controllers[slot].mode_writes;
        }
        lifetime = canif_generation;
        SchM_Exit_CanIf_CANIF_STATE();
        if ((valid != 0u) && (slot < CANIF_CONTROLLER_CAPACITY)) {
            if ((Can_GetControllerMode(driver, &actual_mode) == E_OK) &&
                (actual_mode == CAN_CS_STARTED) &&
                ((PduModeRequest == CANIF_OFFLINE) || (PduModeRequest == CANIF_TX_OFFLINE) ||
                 (PduModeRequest == CANIF_ONLINE))) {
                SchM_Enter_CanIf_CANIF_STATE();
                if (lifetime != canif_generation) {
                    /* Reinitialized meanwhile: the request belonged to the old lifetime. */
                } else if (decision != canif_controllers[slot].mode_writes) {
                    /* A mode change interleaved: decide again against the actual controller. */
                    retry = 1u;
                } else {
                    canif_controllers[slot].pdu_mode = PduModeRequest;
                    ++canif_controllers[slot].mode_writes;
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
    const uint16 slot = CanIf_Controller(canif_configuration, ControllerId);
    if ((slot < CANIF_CONTROLLER_CAPACITY) && (PduModePtr != NULL_PTR)) {
        *PduModePtr = canif_controllers[slot].pdu_mode;
        result = E_OK;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    return result;
}
typedef struct {
    Can_PduType pdu;      /**< Driver request. */
    Can_HwHandleType hoh; /**< Driver transmit handle. */
    uint16 route;         /**< Ledger index, or CANIF_TX_CAPACITY when refused. */
    uint16 slot;          /**< Owner's controller state index. */
    uint32 lifetime;      /**< Configuration lifetime at admission. */
    uint32 stops;         /**< Owner's stop revision at admission. */
    uint8 frame_offset;   /**< Mapped byte offset captured at admission. */
    uint8 frame_length;   /**< Mapped frame length captured at admission. */
    uint8 errors[2];      /**< Runtime errors delivered outside CANIF_STATE. */
} CanIf_TxAdmissionType;

/* Called under CANIF_STATE with an installed configuration and a valid route index. */
static CANIF_CODE void CanIf_ReserveTx(CanIf_TxAdmissionType *request,
                                       uint16 index,
                                       const PduInfoType *info)
{
    const CanIf_TxPduConfigType *route = &canif_configuration->transmit[index];
    const uint16 slot = CanIf_Controller(canif_configuration, route->controller);
    if (slot < CANIF_CONTROLLER_CAPACITY) {
        const CanIf_ControllerStateType *state = &canif_controllers[slot];
        if (state->pdu_mode == CANIF_OFFLINE) {
            request->errors[0] = CANIF_E_STOPPED;
        } else if (state->pdu_mode != CANIF_ONLINE) {
            /* TX_OFFLINE rejects without CANIF_E_STOPPED (SWS_CANIF_00382, 00489). */
        } else if ((info->SduLength > CAN_FD_MAX_LENGTH) ||
                   (((route->can_id & CAN_ID_FD_FLAG) == 0u) &&
                    (info->SduLength > CAN_CLASSICAL_MAX_LENGTH))) {
            request->errors[0] = CANIF_E_DATA_LENGTH_MISMATCH;
            request->errors[1] = CANIF_E_TXPDU_LENGTH_EXCEEDED;
        } else if (info->SduLength > route->maximum_length) {
            request->errors[0] = CANIF_E_TXPDU_LENGTH_EXCEEDED;
        } else if ((state->mode == CAN_CS_STARTED) && (owed[index] == 0u) &&
                   (canif_accepted[index] < (UINT32_MAX - 255u)) && (writing[index] < 255u)) {
            ++writing[index];
            request->route = index;
            request->slot = slot;
            request->hoh = route->hoh;
            request->pdu.length = (uint8)info->SduLength;
            request->pdu.id = route->can_id;
            request->pdu.sdu = info->SduDataPtr;
            request->frame_offset = route->frame_offset;
            request->frame_length = route->frame_length;
            request->lifetime = canif_generation;
            request->stops = state->stops;
        } else {
            /* Not started, or results of earlier requests are still owed. */
        }
    }
}

static CANIF_CODE void CanIf_SettleTx(const CanIf_TxAdmissionType *request, Std_ReturnType written)
{
    const uint16 index = request->route;
    SchM_Enter_CanIf_CANIF_STATE();
    if (request->lifetime == canif_generation) {
        --writing[index];
        if (written != E_OK) {
            /* Refused: no result follows, and no confirmation can answer it. */
            if (early[index] > writing[index]) {
                early[index] = writing[index];
            }
        } else if (early[index] != 0u) {
            --early[index]; /* Its confirmation already arrived. */
        } else if (request->stops != canif_controllers[request->slot].stops) {
            ++owed[index]; /* Accepted by the driver, then cancelled by a stop. */
        } else {
            ++canif_accepted[index];
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
}

CANIF_CODE Std_ReturnType CanIf_Transmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    CanIf_TxAdmissionType request = {{TxPduId, 0u, 0u, NULL_PTR},
                                     0u,
                                     CANIF_TX_CAPACITY,
                                     CANIF_CONTROLLER_CAPACITY,
                                     0u,
                                     0u,
                                     0u,
                                     0u,
                                     {0u, 0u}};
    uint8 controller = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    if ((canif_configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR)) {
        for (uint16 i = 0u; (i < CANIF_TX_CAPACITY) && (i < canif_configuration->transmit_count);
             ++i) {
            const CanIf_TxPduConfigType *route = &canif_configuration->transmit[i];
            if (route->pdu == TxPduId) {
                CanIf_ReserveTx(&request, i, PduInfoPtr);
                controller = route->controller;
                break;
            }
        }
    }
    SchM_Exit_CanIf_CANIF_STATE();
    for (uint8 i = 0u; i < 2u; ++i) {
        if (request.errors[i] != 0u) {
            (void)Det_ReportRuntimeError(
                CANIF_MODULE_ID, 0u, CANIF_SID_TRANSMIT, request.errors[i]);
        }
    }
    if (request.route < CANIF_TX_CAPACITY) {
        uint8 frame[CAN_FD_MAX_LENGTH];
        if (request.frame_length != 0u) {
            for (uint8 byte = 0u; byte < request.frame_length; ++byte) {
                frame[byte] = 0u;
            }
            for (uint8 byte = 0u; byte < request.pdu.length; ++byte) {
                frame[request.frame_offset + byte] = request.pdu.sdu[byte];
            }
            request.pdu.sdu = frame;
            request.pdu.length = request.frame_length;
        }
        const Std_ReturnType written = Can_Write(request.hoh, &request.pdu);
        CanIf_SettleTx(&request, written);
        result = (written == E_OK) ? E_OK : E_NOT_OK; /* CAN_BUSY is not a CanIf result. */
        CanIf_DeliverOwed(controller);
    }
    return result;
}
CANIF_CODE void CanIf_TxConfirmation(PduIdType CanTxPduId)
{
    uint8 deliver = 0u;
    SchM_Enter_CanIf_CANIF_STATE();
    if (canif_configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; (i < CANIF_TX_CAPACITY) && (i < canif_configuration->transmit_count); ++i) {
            const CanIf_TxPduConfigType *route = &canif_configuration->transmit[i];
            if (route->pdu == CanTxPduId) {
                const uint16 slot = CanIf_Controller(canif_configuration, route->controller);
                uint8 answered = 1u;
                if (canif_accepted[i] != 0u) {
                    --canif_accepted[i];
                } else if (early[i] < writing[i]) {
                    ++early[i]; /* Confirmed before its Can_Write returned. */
                } else {
                    answered = 0u; /* Late: its request already has a result. */
                }
                /* The frame left the controller: a disabled Tx path suppresses its
                 * confirmation (SWS_CANIF_00073, 00489), and STOPPED must not later
                 * report it as failed (SWS_CANIF_00739). */
                deliver = ((answered != 0u) && (slot < CANIF_CONTROLLER_CAPACITY) &&
                           (canif_controllers[slot].pdu_mode == CANIF_ONLINE))
                              ? 1u
                              : 0u;
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
    const uint16 slot = CanIf_Controller(canif_configuration, ControllerId);
    if ((slot < CANIF_CONTROLLER_CAPACITY) &&
        ((ControllerMode == CAN_CS_STARTED) || (ControllerMode == CAN_CS_STOPPED) ||
         (ControllerMode == CAN_CS_SLEEP))) {
        CanIf_ControllerStateType *state = &canif_controllers[slot];
        state->mode = ControllerMode;
        if (ControllerMode == CAN_CS_STOPPED) {
            state->pdu_mode = CANIF_TX_OFFLINE;
            ++state->mode_writes;
            CanIf_CancelAccepted(ControllerId, state);
        } else if (ControllerMode == CAN_CS_SLEEP) {
            state->pdu_mode = CANIF_OFFLINE;
            ++state->mode_writes;
        } else {
            /* STARTED keeps the PDU mode until the upper layer sets it. */
        }
        notify = canif_configuration->controller_mode;
    }
    SchM_Exit_CanIf_CANIF_STATE();
    CanIf_DeliverOwed(ControllerId);
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
    const uint16 slot = CanIf_Controller(canif_configuration, ControllerId);
    lifetime = canif_generation;
    SchM_Exit_CanIf_CANIF_STATE();
    if ((valid != 0u) && (slot < CANIF_CONTROLLER_CAPACITY)) {
        /* The host driver enters STOPPED on bus-off without a separate mode
         * indication. Converge only from its actual reported transition.
         */
        const uint8 stopped = ((Can_GetControllerMode(driver, &actual_mode) == E_OK) &&
                               (actual_mode == CAN_CS_STOPPED))
                                  ? 1u
                                  : 0u;
        SchM_Enter_CanIf_CANIF_STATE();
        if (lifetime == canif_generation) {
            CanIf_ControllerStateType *state = &canif_controllers[slot];
            state->pdu_mode = CANIF_TX_OFFLINE;
            ++state->mode_writes;
            if ((stopped != 0u) && (state->mode != CAN_CS_STOPPED)) {
                state->mode = CAN_CS_STOPPED;
                CanIf_CancelAccepted(ControllerId, state);
            }
            notify = canif_configuration->bus_off;
        }
        SchM_Exit_CanIf_CANIF_STATE();
    }
    CanIf_DeliverOwed(ControllerId);
    if (notify != NULL_PTR) {
        notify(ControllerId);
    }
}
CANIF_CODE void CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr)
{
    uint8 deliver = 0u;
    uint8 invalid_length = 0u;
    PduIdType id = 0u;
    PduInfoType logical = {NULL_PTR, NULL_PTR, 0u};
    SchM_Enter_CanIf_CANIF_STATE();
    const uint16 slot = (Mailbox == NULL_PTR)
                            ? CANIF_CONTROLLER_CAPACITY
                            : CanIf_Controller(canif_configuration, Mailbox->ControllerId);
    if ((slot < CANIF_CONTROLLER_CAPACITY) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR) && (canif_controllers[slot].mode == CAN_CS_STARTED) &&
        ((canif_controllers[slot].pdu_mode == CANIF_ONLINE) ||
         (canif_controllers[slot].pdu_mode == CANIF_TX_OFFLINE))) {
        uint16 i;
        for (i = 0u; i < canif_configuration->receive_count; ++i) {
            const CanIf_RxPduConfigType *route = &canif_configuration->receive[i];
            if ((route->controller == Mailbox->ControllerId) && (route->hoh == Mailbox->Hoh) &&
                CanIf_Matches(route, Mailbox->CanId)) {
                const uint8 minimum =
                    (route->frame_length == 0u) ? route->minimum_length : route->frame_length;
                if ((PduInfoPtr->SduLength < minimum) ||
                    (!Can_IsWireLength(Mailbox->CanId, PduInfoPtr->SduLength))) {
                    invalid_length = 1u;
                } else {
                    id = route->pdu;
                    logical = *PduInfoPtr;
                    if (route->frame_length != 0u) {
                        logical.SduDataPtr = &PduInfoPtr->SduDataPtr[route->frame_offset];
                        logical.SduLength = route->minimum_length;
                    }
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
        LSduR_CanIfRxIndication(id, &logical);
    }
}
#define CANIF_STOP_SEC_CODE
#include "CanIf_MemMap.h"
