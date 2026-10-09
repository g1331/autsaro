#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Dcm.h"
#include "ComM_Internal.h"
#include "BswM_ComM.h"
#include "SchM_ComM.h"
#define COMM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "ComM_MemMap.h"
static const ComM_ConfigType *configuration COMM_VAR_CLEARED;
static ComM_ModeType requests[32] COMM_VAR_CLEARED;
static ComM_ModeType current_mode COMM_VAR_CLEARED;
static ComM_StateType channel_state COMM_VAR_CLEARED;
static boolean allowed COMM_VAR_CLEARED;
static boolean diagnostic COMM_VAR_CLEARED;
static uint32 minimum_remaining COMM_VAR_CLEARED;
static Std_ReturnType bus_result COMM_VAR_CLEARED;
#define COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "ComM_MemMap.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
static COMM_CODE boolean ComM_Valid(const ComM_ConfigType *config) {
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->users != NULL_PTR) && (config->user_count > 0u) &&
        (config->user_count <= 32u) && (config->request != NULL_PTR) &&
        (config->current != NULL_PTR) && (config->notification != NULL_PTR)) {
        valid = TRUE;
        for (i = 0u; i < config->user_count; ++i) {
            uint16 j;
            if (config->users[i] == COMM_NOT_USED_USER_ID) {
                valid = FALSE;
            }
            for (j = 0u; j < i; ++j) {
                if (config->users[i] == config->users[j]) {
                    valid = FALSE;
                }
            }
        }
    }
    return valid;
}
static COMM_CODE uint16 ComM_User(ComM_UserHandleType User) {
    uint16 index = 32u;
    if (configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < configuration->user_count; ++i) {
            if (configuration->users[i] == User) {
                index = i;
                break;
            }
        }
    }
    return index;
}
static COMM_CODE boolean ComM_Demand(void) {
    boolean demand = diagnostic;
    uint16 i;
    for (i = 0u; i < configuration->user_count; ++i) {
        if (requests[i] == COMM_FULL_COMMUNICATION) {
            demand = TRUE;
        }
    }
    return demand;
}
static COMM_CODE void ComM_Evaluate(void) {
    const boolean demand = ComM_Demand();
    if ((channel_state == COMM_NO_COM_NO_PENDING_REQUEST) && (demand == TRUE)) {
        channel_state = COMM_NO_COM_REQUEST_PENDING;
    }
    if (channel_state == COMM_NO_COM_REQUEST_PENDING) {
        if (demand == FALSE) {
            channel_state = COMM_NO_COM_NO_PENDING_REQUEST;
        } else if (allowed == TRUE) {
            channel_state = COMM_FULL_COM_NETWORK_REQUESTED;
            minimum_remaining = configuration->minimum_full_ticks;
        } else {
            /* Only REQUEST_PENDING evaluates CommunicationAllowed. */
        }
    } else if (((channel_state == COMM_FULL_COM_READY_SLEEP) ||
                (channel_state == COMM_SILENT_COM)) &&
               (demand == TRUE)) {
        channel_state = COMM_FULL_COM_NETWORK_REQUESTED;
        minimum_remaining = configuration->minimum_full_ticks;
    } else if ((channel_state == COMM_FULL_COM_NETWORK_REQUESTED) && (minimum_remaining == 0u) &&
               (demand == FALSE)) {
        channel_state = COMM_FULL_COM_READY_SLEEP;
    } else {
        /* CDD/NM NONE has no automatic READY_SLEEP -> NO transition. */
    }
}
static COMM_CODE void ComM_RequestBus(void) {
    ComM_ModeType target = COMM_NO_COMMUNICATION;
    if ((channel_state == COMM_FULL_COM_NETWORK_REQUESTED) ||
        (channel_state == COMM_FULL_COM_READY_SLEEP)) {
        target = COMM_FULL_COMMUNICATION;
    } else if (channel_state == COMM_SILENT_COM) {
        target = COMM_SILENT_COMMUNICATION;
    } else {
        /* A pending request with Allowed=false does not open the channel. */
    }
    if ((current_mode != target) || (bus_result != E_OK)) {
        /* E_NOT_OK retains the demand and is retried by the periodic owner.
         * Only the actual completion callback can change current_mode.
         */
        bus_result = configuration->request(configuration->channel, target);
    }
}
COMM_CODE void ComM_Init(const ComM_ConfigType *ConfigPtr) {
    SchM_Enter_ComM_COMM_STATE();
    if (ComM_Valid(ConfigPtr) == TRUE) {
        uint16 i;
        configuration = ConfigPtr;
        for (i = 0u; i < 32u; ++i) {
            requests[i] = COMM_NO_COMMUNICATION;
        }
        current_mode = COMM_NO_COMMUNICATION;
        channel_state = COMM_NO_COM_NO_PENDING_REQUEST;
        allowed = FALSE;
        diagnostic = FALSE;
        minimum_remaining = 0u;
        bus_result = E_NOT_OK;
        ComM_RequestBus(); /* NO entry must disable actual lower communication. */
        /* Default state is not notified to RTE/BswM (SWS_ComM_00313). */
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_DeInit(void) {
    ComM_ModeType actual = COMM_FULL_COMMUNICATION;
    SchM_Enter_ComM_COMM_STATE();
    if ((configuration != NULL_PTR) && (channel_state == COMM_NO_COM_NO_PENDING_REQUEST)) {
        const Std_ReturnType result = configuration->current(configuration->channel, &actual);
        if ((result == E_OK) && (actual == COMM_NO_COMMUNICATION)) {
            configuration = NULL_PTR;
            allowed = FALSE;
            diagnostic = FALSE;
            minimum_remaining = 0u;
        }
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE Std_ReturnType ComM_GetStatus(ComM_InitStatusType *Status) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if (Status != NULL_PTR) {
        *Status = (configuration == NULL_PTR) ? COMM_UNINIT : COMM_INIT;
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_RequestComMode(ComM_UserHandleType User, ComM_ModeType ComMode) {
    Std_ReturnType result = E_NOT_OK;
    uint16 index;
    SchM_Enter_ComM_COMM_STATE();
    index = ComM_User(User);
    if ((index < 32u) &&
        ((ComMode == COMM_NO_COMMUNICATION) || (ComMode == COMM_FULL_COMMUNICATION))) {
        requests[index] = ComMode;
        ComM_Evaluate();
        ComM_RequestBus();
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetRequestedComMode(ComM_UserHandleType User,
                                                  ComM_ModeType *ComMode) {
    Std_ReturnType result = E_NOT_OK;
    uint16 index;
    SchM_Enter_ComM_COMM_STATE();
    index = ComM_User(User);
    if ((index < 32u) && (ComMode != NULL_PTR)) {
        *ComMode = requests[index];
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetCurrentComMode(ComM_UserHandleType User, ComM_ModeType *ComMode) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < 32u) && (ComMode != NULL_PTR)) {
        result = configuration->current(configuration->channel, ComMode);
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetMaxComMode(ComM_UserHandleType User, ComM_ModeType *ComMode) {
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < 32u) && (ComMode != NULL_PTR)) {
        /* Neither mode limitation nor wake-up inhibition is selected. */
        *ComMode = COMM_FULL_COMMUNICATION;
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE void ComM_CommunicationAllowed(NetworkHandleType Channel, boolean Allowed) {
    SchM_Enter_ComM_COMM_STATE();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel)) {
        allowed = Allowed;
        ComM_Evaluate();
        ComM_RequestBus();
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel) {
    SchM_Enter_ComM_COMM_STATE();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel)) {
        diagnostic = TRUE;
        ComM_Evaluate();
        ComM_RequestBus();
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel) {
    SchM_Enter_ComM_COMM_STATE();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel)) {
        diagnostic = FALSE;
        ComM_Evaluate();
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_BusSM_ModeIndication(NetworkHandleType Channel, ComM_ModeType ComMode) {
    SchM_Enter_ComM_COMM_STATE();
    if ((configuration != NULL_PTR) && (Channel == configuration->channel) &&
        (ComMode <= COMM_FULL_COMMUNICATION) && (ComMode != current_mode)) {
        current_mode = ComMode;
        if (ComMode == COMM_NO_COMMUNICATION) {
            channel_state = COMM_NO_COM_NO_PENDING_REQUEST;
            minimum_remaining = 0u;
        } else if (ComMode == COMM_SILENT_COMMUNICATION) {
            channel_state = COMM_SILENT_COM;
        } else {
            /* FULL completion does not restart the NETWORK_REQUESTED timer. */
        }
        BswM_ComM_CurrentMode(Channel, ComMode);
        configuration->notification(Channel, ComMode);
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE ComM_StateType ComM_RunChannel(void) {
    ComM_StateType result;
    SchM_Enter_ComM_COMM_STATE();
    if (configuration != NULL_PTR) {
        if ((channel_state == COMM_FULL_COM_NETWORK_REQUESTED) && (minimum_remaining != 0u)) {
            --minimum_remaining;
        }
        ComM_Evaluate();
        ComM_RequestBus();
    }
    result = channel_state;
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
