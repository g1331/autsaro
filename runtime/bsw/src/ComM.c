/** @file Single CDD channel ComM for NM variant NONE, without PNC, inhibition or NvM.
 * COMM_STATE protects module data only (ADR 0018): bus requests, the actual mode
 * publication and the bus mode query run outside it.
 */
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Dcm.h"
#include "ComM_EcuM.h"
#include "ComM_Internal.h"
#include "BswM_ComM.h"
#include "SchM_ComM.h"
#define COMM_USER_CAPACITY 32u /**< Users per channel; the generator enforces it. */
#define COMM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "ComM_MemMap.h"
/* Ordered handshake with <Bus>SM_RequestComMode. Entering a main state claims the
 * target here; one caller at a time issues it outside COMM_STATE and settles the
 * result. A target claimed while a call is in flight is issued after that call. */
typedef enum {
    COMM_BUS_ACCEPTED,   /**< The last issued target was accepted. */
    COMM_BUS_DUE,        /**< The target must be issued now. */
    COMM_BUS_REFUSED,    /**< Refused; the main function issues it again. */
    COMM_BUS_CALLING,    /**< The target is on a caller's stack. */
    COMM_BUS_CALLING_DUE /**< On a caller's stack; a newer target follows it. */
} ComM_BusPhaseType;
/* All COMM_STATE. Init resets the channel and the published mode, but preserves an
 * in-flight bus call and publisher so that no second caller overtakes them. */
static const ComM_ConfigType *comm_configuration COMM_VAR_CLEARED;  /**< NULL when uninitialized. */
static ComM_ModeType requests[COMM_USER_CAPACITY] COMM_VAR_CLEARED; /**< Per user. */
static ComM_StateType channel_state COMM_VAR_CLEARED;               /**< Figure 7.7 state. */
static boolean allowed COMM_VAR_CLEARED;                            /**< CommunicationAllowed. */
static boolean diagnostic COMM_VAR_CLEARED;                         /**< Dcm active diagnostic. */
static boolean wakeup_pending COMM_VAR_CLEARED;                     /**< Wake-up until FULL. */
static uint32 minimum_remaining COMM_VAR_CLEARED;                   /**< Minimum FULL periods. */
static ComM_ModeType bus_target COMM_VAR_CLEARED;     /**< Mode of the entered state. */
static ComM_BusPhaseType bus_phase COMM_VAR_CLEARED;  /**< Init preserves a call. */
static ComM_ModeType indicated COMM_VAR_CLEARED;      /**< Latest actual bus mode. */
static ComM_ModeType comm_published COMM_VAR_CLEARED; /**< Last mode sent to users. */
static boolean comm_publishing COMM_VAR_CLEARED;      /**< Publisher; Init preserves. */
#define COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "ComM_MemMap.h"
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
static COMM_CODE boolean ComM_Valid(const ComM_ConfigType *config)
{
    boolean valid = FALSE;
    uint16 i;
    if ((config != NULL_PTR) && (config->users != NULL_PTR) && (config->user_count > 0u) &&
        (config->user_count <= COMM_USER_CAPACITY) && (config->request != NULL_PTR) &&
        (config->current != NULL_PTR) &&
        ((config->dcm_no == NULL_PTR) == (config->dcm_silent == NULL_PTR)) &&
        ((config->dcm_no == NULL_PTR) == (config->dcm_full == NULL_PTR))) {
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
static COMM_CODE uint16 ComM_User(ComM_UserHandleType User)
{
    uint16 index = COMM_USER_CAPACITY;
    if (comm_configuration != NULL_PTR) {
        uint16 i;
        for (i = 0u; i < comm_configuration->user_count; ++i) {
            if (comm_configuration->users[i] == User) {
                index = i;
                break;
            }
        }
    }
    return index;
}
static COMM_CODE boolean ComM_Demand(void)
{
    boolean demand = diagnostic || wakeup_pending;
    uint16 i;
    for (i = 0u; i < comm_configuration->user_count; ++i) {
        if (requests[i] == COMM_FULL_COMMUNICATION) {
            demand = TRUE;
        }
    }
    return demand;
}
/* Entering a main state requests its mode from the bus state manager
 * (SWS_ComM_00073, SWS_ComM_00069). */
static COMM_CODE void ComM_ClaimBus(ComM_ModeType target)
{
    bus_target = target;
    bus_phase = ((bus_phase == COMM_BUS_CALLING) || (bus_phase == COMM_BUS_CALLING_DUE))
                    ? COMM_BUS_CALLING_DUE
                    : COMM_BUS_DUE;
}
static COMM_CODE void ComM_Evaluate(void)
{
    const boolean demand = ComM_Demand();
    if ((channel_state == COMM_NO_COM_NO_PENDING_REQUEST) && demand) {
        channel_state = COMM_NO_COM_REQUEST_PENDING;
    }
    if (channel_state == COMM_NO_COM_REQUEST_PENDING) {
        if (!demand) {
            channel_state = COMM_NO_COM_NO_PENDING_REQUEST;
        } else if (allowed) {
            channel_state = COMM_FULL_COM_NETWORK_REQUESTED;
            minimum_remaining = comm_configuration->minimum_full_ticks;
            wakeup_pending = FALSE;
            ComM_ClaimBus(COMM_FULL_COMMUNICATION);
        } else {
            /* Only REQUEST_PENDING evaluates CommunicationAllowed. */
        }
    } else if ((channel_state == COMM_FULL_COM_READY_SLEEP) && demand) {
        channel_state = COMM_FULL_COM_NETWORK_REQUESTED;
        minimum_remaining = comm_configuration->minimum_full_ticks;
    } else if ((channel_state == COMM_FULL_COM_NETWORK_REQUESTED) && (minimum_remaining == 0u) &&
               !demand) {
        channel_state = COMM_FULL_COM_READY_SLEEP;
    } else {
        /* NM NONE has no READY_SLEEP -> NO transition and no SILENT state. */
    }
}
/* Issue claimed bus targets in order; a caller that finds a call in flight leaves its
 * target to that call's owner. */
static COMM_CODE void ComM_IssueBus(void)
{
    boolean owner = TRUE;
    while (owner) {
        ComM_BusRequestType request = NULL_PTR;
        NetworkHandleType channel = 0u;
        ComM_ModeType target = COMM_NO_COMMUNICATION;
        SchM_Enter_ComM_COMM_STATE();
        if ((comm_configuration != NULL_PTR) && (bus_phase == COMM_BUS_DUE)) {
            bus_phase = COMM_BUS_CALLING;
            request = comm_configuration->request;
            channel = comm_configuration->channel;
            target = bus_target;
        } else {
            owner = FALSE;
        }
        SchM_Exit_ComM_COMM_STATE();
        if (owner) {
            const Std_ReturnType result = request(channel, target);
            SchM_Enter_ComM_COMM_STATE();
            if (bus_phase == COMM_BUS_CALLING) {
                /* A refusal keeps the target; the main function retries it. */
                bus_phase = (result == E_OK) ? COMM_BUS_ACCEPTED : COMM_BUS_REFUSED;
            } else {
                bus_phase = COMM_BUS_DUE;
            }
            SchM_Exit_ComM_COMM_STATE();
        }
    }
}
static COMM_CODE void ComM_NotifyDcm(const ComM_ConfigType *config, ComM_ModeType mode)
{
    if (config->dcm_full != NULL_PTR) {
        if (mode == COMM_FULL_COMMUNICATION) {
            config->dcm_full(config->channel);
        } else if (mode == COMM_SILENT_COMMUNICATION) {
            config->dcm_silent(config->channel);
        } else {
            config->dcm_no(config->channel);
        }
    }
}
/* Called by the claimed publisher only; publishes until the latest indication is out. */
static COMM_CODE void ComM_Publish(void)
{
    boolean owner = TRUE;
    while (owner) {
        const ComM_ConfigType *config = NULL_PTR;
        ComM_ModeType mode = COMM_NO_COMMUNICATION;
        SchM_Enter_ComM_COMM_STATE();
        if ((comm_configuration != NULL_PTR) && (comm_published != indicated)) {
            comm_published = indicated;
            config = comm_configuration;
            mode = comm_published;
        } else {
            comm_publishing = FALSE;
            owner = FALSE;
        }
        SchM_Exit_ComM_COMM_STATE();
        if (owner) {
            /* RTE user ports are not generated (see the ComM standards page). */
            BswM_ComM_CurrentMode(config->channel, mode);
            ComM_NotifyDcm(config, mode);
        }
    }
}
COMM_CODE void ComM_Init(const ComM_ConfigType *ConfigPtr)
{
    if (ComM_Valid(ConfigPtr) == TRUE) {
        uint16 i;
        SchM_Enter_ComM_COMM_STATE();
        comm_configuration = ConfigPtr;
        for (i = 0u; i < COMM_USER_CAPACITY; ++i) {
            requests[i] = COMM_NO_COMMUNICATION;
        }
        channel_state = COMM_NO_COM_NO_PENDING_REQUEST;
        allowed = FALSE;
        diagnostic = FALSE;
        wakeup_pending = FALSE;
        minimum_remaining = 0u;
        /* The default NO is not notified (SWS_ComM_00313). */
        indicated = COMM_NO_COMMUNICATION;
        comm_published = COMM_NO_COMMUNICATION;
        ComM_ClaimBus(COMM_NO_COMMUNICATION);
        SchM_Exit_ComM_COMM_STATE();
        ComM_IssueBus();
    }
}
COMM_CODE void ComM_DeInit(void)
{
    const ComM_ConfigType *config = NULL_PTR;
    ComM_ModeType actual = COMM_FULL_COMMUNICATION;
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if (channel_state == COMM_NO_COM_NO_PENDING_REQUEST) {
        config = comm_configuration;
    }
    SchM_Exit_ComM_COMM_STATE();
    if (config != NULL_PTR) {
        result = config->current(config->channel, &actual);
    }
    if ((result == E_OK) && (actual == COMM_NO_COMMUNICATION)) {
        SchM_Enter_ComM_COMM_STATE();
        /* A request accepted during the query left NO_PENDING_REQUEST and keeps ComM. */
        if ((comm_configuration == config) && (channel_state == COMM_NO_COM_NO_PENDING_REQUEST)) {
            comm_configuration = NULL_PTR;
        }
        SchM_Exit_ComM_COMM_STATE();
    }
}
COMM_CODE Std_ReturnType ComM_GetStatus(ComM_InitStatusType *Status)
{
    Std_ReturnType result = E_NOT_OK;
    if (Status != NULL_PTR) {
        SchM_Enter_ComM_COMM_STATE();
        *Status = (comm_configuration == NULL_PTR) ? COMM_UNINIT : COMM_INIT;
        SchM_Exit_ComM_COMM_STATE();
        result = E_OK;
    }
    return result;
}
COMM_CODE Std_ReturnType ComM_GetInhibitionStatus(NetworkHandleType Channel,
                                                  ComM_InhibitionStatusType *Status)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel) &&
        (Status != NULL_PTR)) {
        /* Wake-up inhibition and mode limitation are disabled. */
        *Status = 0u;
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_SetECUGroupClassification(ComM_InhibitionStatusType Status)
{
    Std_ReturnType result = E_NOT_OK;
    /* Only the disabled inhibition features read the classification. */
    (void)Status;
    SchM_Enter_ComM_COMM_STATE();
    if (comm_configuration != NULL_PTR) {
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_RequestComMode(ComM_UserHandleType User, ComM_ModeType ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    uint16 index;
    SchM_Enter_ComM_COMM_STATE();
    index = ComM_User(User);
    if ((index < COMM_USER_CAPACITY) &&
        ((ComMode == COMM_NO_COMMUNICATION) || (ComMode == COMM_FULL_COMMUNICATION))) {
        requests[index] = ComMode;
        ComM_Evaluate();
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetRequestedComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    uint16 index;
    SchM_Enter_ComM_COMM_STATE();
    index = ComM_User(User);
    if ((index < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        *ComMode = requests[index];
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetCurrentComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const ComM_ConfigType *config = NULL_PTR;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        config = comm_configuration;
    }
    SchM_Exit_ComM_COMM_STATE();
    if (config != NULL_PTR) {
        result = config->current(config->channel, ComMode);
    }
    return result;
}
COMM_CODE Std_ReturnType ComM_GetCurrentPNCComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        /* PncSupport=false gives every configured static user zero PNCs.
         * The API and service Possible Errors put this in the return status;
         * the output mode remains untouched on this refusal.
         */
        result = COMM_E_NO_PNC_ASSIGNED;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE Std_ReturnType ComM_GetMaxComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        /* Neither mode limitation nor wake-up inhibition is selected. */
        *ComMode = COMM_FULL_COMMUNICATION;
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
COMM_CODE void ComM_CommunicationAllowed(NetworkHandleType Channel, boolean Allowed)
{
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel)) {
        allowed = Allowed;
        ComM_Evaluate();
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus();
}
COMM_CODE void ComM_EcuM_WakeUpIndication(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel) &&
        (channel_state == COMM_NO_COM_NO_PENDING_REQUEST)) {
        /* A wake-up is a pending request of its own. Ordinary NO requests must
         * not cancel it while communication is still waiting for permission.
         * The selected single channel has SynchronousWakeUp=false and NM NONE.
         */
        wakeup_pending = TRUE;
        ComM_Evaluate();
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus();
}
COMM_CODE void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel)) {
        diagnostic = TRUE;
        ComM_Evaluate();
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus();
}
COMM_CODE void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel)) {
        diagnostic = FALSE;
        ComM_Evaluate();
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_BusSM_ModeIndication(NetworkHandleType Channel, ComM_ModeType ComMode)
{
    boolean owner = FALSE;
    SchM_Enter_ComM_COMM_STATE();
    if ((comm_configuration != NULL_PTR) && (Channel == comm_configuration->channel) &&
        (ComMode <= COMM_FULL_COMMUNICATION)) {
        indicated = ComMode;
        if (!comm_publishing) {
            comm_publishing = TRUE;
            owner = TRUE;
        }
    }
    SchM_Exit_ComM_COMM_STATE();
    if (owner) {
        ComM_Publish();
    }
}
COMM_CODE ComM_StateType ComM_RunChannel(void)
{
    ComM_StateType result;
    SchM_Enter_ComM_COMM_STATE();
    if (comm_configuration != NULL_PTR) {
        if ((channel_state == COMM_FULL_COM_NETWORK_REQUESTED) && (minimum_remaining != 0u)) {
            --minimum_remaining;
        }
        ComM_Evaluate();
        if (bus_phase == COMM_BUS_REFUSED) {
            bus_phase = COMM_BUS_DUE;
        }
    }
    result = channel_state;
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus();
    return result;
}
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
