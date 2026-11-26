/** @file CDD channel ComM for NM variant NONE, without PNC, inhibition or NvM.
 * COMM_STATE protects module data only: bus requests, the actual mode
 * publication and the bus mode query run outside it.
 */
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Dcm.h"
#include "ComM_EcuM.h"
#include "ComM_Internal.h"
#include "BswM_ComM.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(BSWM_AR_RELEASE_MAJOR_VERSION) || !defined(BSWM_AR_RELEASE_MINOR_VERSION)
#error "ComM.c -> BswM_ComM.h: missing AUTOSAR release"
#elif (BSWM_AR_RELEASE_MAJOR_VERSION != 4U) || (BSWM_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.c -> BswM_ComM.h: incompatible AUTOSAR release"
#endif
#include "SchM_ComM.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "ComM.c -> SchM_ComM.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.c -> SchM_ComM.h: incompatible AUTOSAR release"
#endif
#include "Ecu_ProfileConfig.h"
/* SWS_ComM_00418: source release is independent of the imported header producer. */
#define COMM_C_AR_RELEASE_MAJOR_VERSION    4U
#define COMM_C_AR_RELEASE_MINOR_VERSION    10U
#define COMM_C_AR_RELEASE_REVISION_VERSION 0U
#if !defined(COMM_AR_RELEASE_MAJOR_VERSION) || !defined(COMM_AR_RELEASE_MINOR_VERSION) ||          \
    !defined(COMM_AR_RELEASE_REVISION_VERSION)
#error "ComM.c -> ComM.h: missing internal AUTOSAR release"
#elif (COMM_AR_RELEASE_MAJOR_VERSION != COMM_C_AR_RELEASE_MAJOR_VERSION) ||                        \
    (COMM_AR_RELEASE_MINOR_VERSION != COMM_C_AR_RELEASE_MINOR_VERSION) ||                          \
    (COMM_AR_RELEASE_REVISION_VERSION != COMM_C_AR_RELEASE_REVISION_VERSION)
#error "ComM.c -> ComM.h: inconsistent internal AUTOSAR release"
#endif
/* SWS_ComM_00418: source software release is independent of the header owner. */
#define COMM_C_SW_MAJOR_VERSION 0U
#define COMM_C_SW_MINOR_VERSION 1U
#define COMM_C_SW_PATCH_VERSION 0U
#if !defined(COMM_SW_MAJOR_VERSION) || !defined(COMM_SW_MINOR_VERSION) ||                          \
    !defined(COMM_SW_PATCH_VERSION)
#error "ComM.c -> ComM.h: missing internal software version"
#elif (COMM_SW_MAJOR_VERSION != COMM_C_SW_MAJOR_VERSION) ||                                        \
    (COMM_SW_MINOR_VERSION != COMM_C_SW_MINOR_VERSION) ||                                          \
    (COMM_SW_PATCH_VERSION != COMM_C_SW_PATCH_VERSION)
#error "ComM.c -> ComM.h: inconsistent internal software version"
#endif
#define COMM_USER_CAPACITY 32u /**< Maximum configured users across the ECU. */
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
/* Init resets each configured channel, preserving an in-flight call or publisher. */
typedef struct {
    ComM_StateType state;     /**< Figure 7.7 state. */
    boolean allowed;          /**< CommunicationAllowed. */
    boolean diagnostic;       /**< Active diagnostic demand. */
    boolean wakeup;           /**< Wake-up until FULL. */
    uint32 minimum_remaining; /**< Minimum FULL periods. */
    ComM_ModeType target;     /**< Mode of the entered state. */
    ComM_BusPhaseType phase;  /**< Ordered lower-layer request. */
    ComM_ModeType indicated;  /**< Latest actual bus mode. */
    ComM_ModeType published;  /**< Last actual mode claimed for publication. */
} ComM_ChannelStateType;
static const ComM_ConfigType *comm_configuration COMM_VAR_CLEARED; /**< NULL when uninitialized. */
static ComM_ChannelStateType
    comm_channels[COMM_CHANNEL_CAPACITY] COMM_VAR_CLEARED; /**< Per-channel state under COMM_STATE;
                                                              initialized by ComM_Init. */
static ComM_ModeType
    comm_requests[COMM_USER_CAPACITY] COMM_VAR_CLEARED; /**< Latest request per user; ComM_Init sets
                                                           NO. */
static ComM_ModeType
    comm_user_published[COMM_USER_CAPACITY] COMM_VAR_CLEARED; /**< Last indicated user mode;
                                                                 initialized to NO. */
static boolean comm_publishing COMM_VAR_CLEARED;              /**< Publisher; Init preserves. */
#define COMM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "ComM_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "ComM.c -> ComM_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "ComM.c -> ComM_MemMap.h: incompatible AUTOSAR release"
#endif
#define COMM_START_SEC_CODE
#include "ComM_MemMap.h"
/* Configuration positions index the generated state table; public handles may be sparse. */
static COMM_CODE uint16 ComM_Channel(const ComM_ConfigType *config, NetworkHandleType channel)
{
    uint16 result = COMM_CHANNEL_CAPACITY;
    if (config != NULL_PTR) {
        for (uint16 i = 0u; i < config->channel_count; ++i) {
            if (config->channels[i].channel == channel) {
                result = i;
                break;
            }
        }
    }
    return result;
}

static COMM_CODE boolean ComM_ValidUsers(const ComM_ConfigType *config)
{
    boolean valid = TRUE;
    for (uint16 i = 0u; (i < config->user_count) && valid; ++i) {
        const ComM_UserModeConfigType *user = &config->users[i];
        valid = (user->user != COMM_NOT_USED_USER_ID) && (user->channels != NULL_PTR) &&
                (user->channel_count > 0u) && (user->channel_count <= config->channel_count);
        for (uint16 j = 0u; (j < i) && valid; ++j) {
            valid = config->users[j].user != user->user;
        }
        for (uint16 j = 0u; (j < user->channel_count) && valid; ++j) {
            valid = ComM_Channel(config, user->channels[j]) < COMM_CHANNEL_CAPACITY;
            for (uint16 k = 0u; (k < j) && valid; ++k) {
                valid = user->channels[k] != user->channels[j];
            }
        }
    }
    return valid;
}
static COMM_CODE boolean ComM_Valid(const ComM_ConfigType *config)
{
    boolean valid =
        (config != NULL_PTR) && (config->channels != NULL_PTR) && (config->channel_count != 0u) &&
        (config->channel_count <= COMM_CHANNEL_CAPACITY) && (config->users != NULL_PTR) &&
        (config->user_count > 0u) && (config->user_count <= COMM_USER_CAPACITY) &&
        (config->request != NULL_PTR) && (config->current != NULL_PTR);
    if (valid) {
        for (uint16 i = 0u; (i < config->channel_count) && valid; ++i) {
            const ComM_ChannelConfigType *channel = &config->channels[i];
            valid = ((channel->dcm_no == NULL_PTR) == (channel->dcm_silent == NULL_PTR)) &&
                    ((channel->dcm_no == NULL_PTR) == (channel->dcm_full == NULL_PTR)) &&
                    (ComM_Channel(config, channel->channel) == i);
        }
        valid = valid && ComM_ValidUsers(config);
    }
    return valid;
}
static COMM_CODE uint16 ComM_User(ComM_UserHandleType user)
{
    uint16 index = COMM_USER_CAPACITY;
    if (comm_configuration != NULL_PTR) {
        for (uint16 i = 0u; i < comm_configuration->user_count; ++i) {
            if (comm_configuration->users[i].user == user) {
                index = i;
                break;
            }
        }
    }
    return index;
}
static COMM_CODE boolean ComM_Demand(const ComM_ChannelStateType *state, NetworkHandleType channel)
{
    boolean demand = state->diagnostic || state->wakeup;
    for (uint16 i = 0u; (i < comm_configuration->user_count) && !demand; ++i) {
        const ComM_UserModeConfigType *user = &comm_configuration->users[i];
        if (comm_requests[i] == COMM_FULL_COMMUNICATION) {
            for (uint16 j = 0u; (j < user->channel_count) && !demand; ++j) {
                demand = user->channels[j] == channel;
            }
        }
    }
    return demand;
}
/* Entering a main state requests its mode from the bus state manager
 * (SWS_ComM_00073, SWS_ComM_00069). */
static COMM_CODE void ComM_ClaimBus(ComM_ChannelStateType *state, ComM_ModeType target)
{
    state->target = target;
    state->phase = ((state->phase == COMM_BUS_CALLING) || (state->phase == COMM_BUS_CALLING_DUE))
                       ? COMM_BUS_CALLING_DUE
                       : COMM_BUS_DUE;
}
static COMM_CODE void ComM_Evaluate(uint16 index)
{
    ComM_ChannelStateType *state = &comm_channels[index];
    const ComM_ChannelConfigType *config = &comm_configuration->channels[index];
    const boolean demand = ComM_Demand(state, config->channel);
    if ((state->state == COMM_NO_COM_NO_PENDING_REQUEST) && demand) {
        state->state = COMM_NO_COM_REQUEST_PENDING;
    }
    if (state->state == COMM_NO_COM_REQUEST_PENDING) {
        if (!demand) {
            state->state = COMM_NO_COM_NO_PENDING_REQUEST;
        } else if (state->allowed) {
            state->state = COMM_FULL_COM_NETWORK_REQUESTED;
            state->minimum_remaining = config->minimum_full_ticks;
            state->wakeup = FALSE;
            ComM_ClaimBus(state, COMM_FULL_COMMUNICATION);
        } else {
            /* Only REQUEST_PENDING evaluates CommunicationAllowed. */
        }
    } else if ((state->state == COMM_FULL_COM_READY_SLEEP) && demand) {
        state->state = COMM_FULL_COM_NETWORK_REQUESTED;
        state->minimum_remaining = config->minimum_full_ticks;
    } else if ((state->state == COMM_FULL_COM_NETWORK_REQUESTED) &&
               (state->minimum_remaining == 0u) && !demand) {
        state->state = COMM_FULL_COM_READY_SLEEP;
    } else {
        /* NM NONE has no READY_SLEEP -> NO transition and no SILENT state. */
    }
}
/* The claimant keeps its slot while the lower-layer call runs outside COMM_STATE. */
static COMM_CODE void ComM_IssueBus(NetworkHandleType channel)
{
    boolean owner = TRUE;
    while (owner) {
        ComM_BusRequestType request = NULL_PTR;
        ComM_ChannelStateType *state = NULL_PTR;
        ComM_ModeType target = COMM_NO_COMMUNICATION;
        SchM_Enter_ComM_COMM_STATE();
        const uint16 index = ComM_Channel(comm_configuration, channel);
        if ((index < COMM_CHANNEL_CAPACITY) && (comm_channels[index].phase == COMM_BUS_DUE)) {
            state = &comm_channels[index];
            state->phase = COMM_BUS_CALLING;
            request = comm_configuration->request;
            target = state->target;
        } else {
            owner = FALSE;
        }
        SchM_Exit_ComM_COMM_STATE();
        if (owner) {
            const Std_ReturnType result = request(channel, target);
            SchM_Enter_ComM_COMM_STATE();
            if (state->phase == COMM_BUS_CALLING) {
                state->phase = (result == E_OK) ? COMM_BUS_ACCEPTED : COMM_BUS_REFUSED;
            } else {
                state->phase = COMM_BUS_DUE;
            }
            SchM_Exit_ComM_COMM_STATE();
        }
    }
}
static COMM_CODE void ComM_NotifyDcm(const ComM_ChannelConfigType *config, ComM_ModeType mode)
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
typedef struct {
    ComM_UserModeType notify; /**< Claimed RTE publication. */
    ComM_ModeType mode;       /**< Lowest indicated channel mode. */
} ComM_UserPublicationType;

/* Inside COMM_STATE; use the actual modes even if another bus publication is queued. */
static COMM_CODE uint16 ComM_ClaimUsers(ComM_UserPublicationType *publications)
{
    uint16 count = 0u;
    for (uint16 i = 0u; i < comm_configuration->user_count; ++i) {
        const ComM_UserModeConfigType *user = &comm_configuration->users[i];
        ComM_ModeType lowest = COMM_FULL_COMMUNICATION;
        for (uint16 j = 0u; j < user->channel_count; ++j) {
            const uint16 index = ComM_Channel(comm_configuration, user->channels[j]);
            if (comm_channels[index].indicated < lowest) {
                lowest = comm_channels[index].indicated;
            }
        }
        if (comm_user_published[i] != lowest) {
            comm_user_published[i] = lowest;
            if (user->notify != NULL_PTR) {
                publications[count].notify = user->notify;
                publications[count].mode = lowest;
                ++count;
            }
        }
    }
    return count;
}
/* The one publisher reconciles channel indications and dispatches outside COMM_STATE. */
static COMM_CODE void ComM_Publish(void)
{
    boolean owner = TRUE;
    while (owner) {
        ComM_UserPublicationType users[COMM_USER_CAPACITY];
        const ComM_ChannelConfigType *channel = NULL_PTR;
        uint16 count = 0u;
        ComM_ModeType mode = COMM_NO_COMMUNICATION;
        SchM_Enter_ComM_COMM_STATE();
        if (comm_configuration != NULL_PTR) {
            for (uint16 i = 0u; i < comm_configuration->channel_count; ++i) {
                if (comm_channels[i].published != comm_channels[i].indicated) {
                    comm_channels[i].published = comm_channels[i].indicated;
                    channel = &comm_configuration->channels[i];
                    mode = comm_channels[i].published;
                    count = ComM_ClaimUsers(users);
                    break;
                }
            }
        }
        if (channel == NULL_PTR) {
            comm_publishing = FALSE;
            owner = FALSE;
        }
        SchM_Exit_ComM_COMM_STATE();
        if (owner) {
            for (uint16 i = 0u; i < count; ++i) {
                users[i].notify(users[i].mode);
            }
            BswM_ComM_CurrentMode(channel->channel, mode);
            ComM_NotifyDcm(channel, mode);
        }
    }
}
static COMM_CODE void ComM_ResetChannel(ComM_ChannelStateType *state)
{
    state->state = COMM_NO_COM_NO_PENDING_REQUEST;
    state->allowed = FALSE;
    state->diagnostic = FALSE;
    state->wakeup = FALSE;
    state->minimum_remaining = 0u;
    state->indicated = COMM_NO_COMMUNICATION;
    if (!comm_publishing) {
        state->published = COMM_NO_COMMUNICATION;
    }
    ComM_ClaimBus(state, COMM_NO_COMMUNICATION);
}
COMM_CODE void ComM_Init(const ComM_ConfigType *ConfigPtr)
{
    if (ComM_Valid(ConfigPtr) == TRUE) {
        SchM_Enter_ComM_COMM_STATE();
        comm_configuration = ConfigPtr;
        for (uint16 i = 0u; i < ConfigPtr->user_count; ++i) {
            comm_requests[i] = COMM_NO_COMMUNICATION;
            if (!comm_publishing) {
                comm_user_published[i] = COMM_NO_COMMUNICATION;
            }
        }
        for (uint16 i = 0u; i < ConfigPtr->channel_count; ++i) {
            ComM_ResetChannel(&comm_channels[i]);
        }
        SchM_Exit_ComM_COMM_STATE();
        for (uint16 i = 0u; i < ConfigPtr->channel_count; ++i) {
            ComM_IssueBus(ConfigPtr->channels[i].channel);
        }
    }
}
static COMM_CODE boolean ComM_NoPendingRequest(const ComM_ConfigType *config)
{
    boolean idle = config != NULL_PTR;
    if (idle) {
        for (uint16 i = 0u; (i < config->channel_count) && idle; ++i) {
            idle = comm_channels[i].state == COMM_NO_COM_NO_PENDING_REQUEST;
        }
    }
    return idle;
}
static COMM_CODE boolean ComM_AllBusesNoCom(const ComM_ConfigType *config)
{
    boolean idle = TRUE;
    for (uint16 i = 0u; (i < config->channel_count) && idle; ++i) {
        ComM_ModeType mode = COMM_FULL_COMMUNICATION;
        idle = (config->current(config->channels[i].channel, &mode) == E_OK) &&
               (mode == COMM_NO_COMMUNICATION);
    }
    return idle;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00147) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE void ComM_DeInit(void)
{
    const ComM_ConfigType *config = NULL_PTR;
    SchM_Enter_ComM_COMM_STATE();
    if (ComM_NoPendingRequest(comm_configuration)) {
        config = comm_configuration;
    }
    SchM_Exit_ComM_COMM_STATE();
    if ((config != NULL_PTR) && ComM_AllBusesNoCom(config)) {
        SchM_Enter_ComM_COMM_STATE();
        /* A request accepted during the query can keep ComM. */
        if ((comm_configuration == config) && ComM_NoPendingRequest(config)) {
            comm_configuration = NULL_PTR;
        }
        SchM_Exit_ComM_COMM_STATE();
    }
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00242) that no
 * module of the selected program calls; see the ComM deviation record. */
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
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00619) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE Std_ReturnType ComM_GetInhibitionStatus(NetworkHandleType Channel,
                                                  ComM_InhibitionStatusType *Status)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if (ComM_Channel(comm_configuration, Channel) < COMM_CHANNEL_CAPACITY && (Status != NULL_PTR)) {
        /* Wake-up inhibition and mode limitation are disabled. */
        *Status = 0u;
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00552) that no
 * module of the selected program calls; see the ComM deviation record. */
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
    const ComM_UserModeConfigType *user = NULL_PTR;
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_User(User);
    if ((index < COMM_USER_CAPACITY) &&
        ((ComMode == COMM_NO_COMMUNICATION) || (ComMode == COMM_FULL_COMMUNICATION))) {
        user = &comm_configuration->users[index];
        comm_requests[index] = ComMode;
        for (uint16 i = 0u; i < user->channel_count; ++i) {
            ComM_Evaluate(ComM_Channel(comm_configuration, user->channels[i]));
        }
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    if (user != NULL_PTR) {
        for (uint16 i = 0u; i < user->channel_count; ++i) {
            ComM_IssueBus(user->channels[i]);
        }
    }
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00079) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE Std_ReturnType ComM_GetRequestedComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    uint16 index;
    SchM_Enter_ComM_COMM_STATE();
    index = ComM_User(User);
    if ((index < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        *ComMode = comm_requests[index];
        result = E_OK;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00083) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE Std_ReturnType ComM_GetCurrentComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    const ComM_UserModeConfigType *user = NULL_PTR;
    ComM_BusGetType current = NULL_PTR;
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_User(User);
    if ((index < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        user = &comm_configuration->users[index];
        current = comm_configuration->current;
    }
    SchM_Exit_ComM_COMM_STATE();
    if (user != NULL_PTR) {
        ComM_ModeType lowest = COMM_FULL_COMMUNICATION;
        result = E_OK;
        for (uint16 i = 0u; (i < user->channel_count) && (result == E_OK); ++i) {
            ComM_ModeType mode = COMM_NO_COMMUNICATION;
            result = current(user->channels[i], &mode);
            if (mode < lowest) {
                lowest = mode;
            }
        }
        if (result == E_OK) {
            *ComMode = lowest;
        }
    }
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_91002) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE Std_ReturnType ComM_GetCurrentPNCComMode(ComM_UserHandleType User, ComM_ModeType *ComMode)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_ComM_COMM_STATE();
    if ((ComM_User(User) < COMM_USER_CAPACITY) && (ComMode != NULL_PTR)) {
        /* PncSupport=false gives every configured static user zero PNCs. SWS_ComM_01023
         * hands the code out as ComMode and SWS_ComM_91002 lists it as return value. */
        *ComMode = COMM_E_NO_PNC_ASSIGNED;
        result = COMM_E_NO_PNC_ASSIGNED;
    }
    SchM_Exit_ComM_COMM_STATE();
    return result;
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00085) that no
 * module of the selected program calls; see the ComM deviation record. */
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
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if (index < COMM_CHANNEL_CAPACITY) {
        comm_channels[index].allowed = Allowed;
        ComM_Evaluate(index);
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus(Channel);
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_ComM_00275) that no
 * module of the selected program calls; see the ComM deviation record. */
COMM_CODE void ComM_EcuM_WakeUpIndication(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if ((index < COMM_CHANNEL_CAPACITY) &&
        (comm_channels[index].state == COMM_NO_COM_NO_PENDING_REQUEST)) {
        /* Wake-up remains pending until this channel is allowed to request FULL. */
        comm_channels[index].wakeup = TRUE;
        ComM_Evaluate(index);
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus(Channel);
}
COMM_CODE void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if (index < COMM_CHANNEL_CAPACITY) {
        comm_channels[index].diagnostic = TRUE;
        ComM_Evaluate(index);
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus(Channel);
}
COMM_CODE void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel)
{
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if (index < COMM_CHANNEL_CAPACITY) {
        comm_channels[index].diagnostic = FALSE;
        ComM_Evaluate(index);
    }
    SchM_Exit_ComM_COMM_STATE();
}
COMM_CODE void ComM_BusSM_ModeIndication(NetworkHandleType Channel, ComM_ModeType ComMode)
{
    boolean owner = FALSE;
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if ((index < COMM_CHANNEL_CAPACITY) && (ComMode <= COMM_FULL_COMMUNICATION)) {
        comm_channels[index].indicated = ComMode;
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
COMM_CODE ComM_StateType ComM_RunChannel(NetworkHandleType Channel)
{
    ComM_StateType result = COMM_NO_COM_NO_PENDING_REQUEST;
    SchM_Enter_ComM_COMM_STATE();
    const uint16 index = ComM_Channel(comm_configuration, Channel);
    if (index < COMM_CHANNEL_CAPACITY) {
        ComM_ChannelStateType *state = &comm_channels[index];
        if ((state->state == COMM_FULL_COM_NETWORK_REQUESTED) && (state->minimum_remaining != 0u)) {
            --state->minimum_remaining;
        }
        ComM_Evaluate(index);
        if (state->phase == COMM_BUS_REFUSED) {
            state->phase = COMM_BUS_DUE;
        }
        result = state->state;
    }
    SchM_Exit_ComM_COMM_STATE();
    ComM_IssueBus(Channel);
    return result;
}
#define COMM_STOP_SEC_CODE
#include "ComM_MemMap.h"
