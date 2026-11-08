/** @file ComM service clients and mode readers linked to real ComM/RTE producers. */
#define RTE_CORE
#include "Rte_Observe.h"
#include "Rte_ComMService.h"
#include "Rte_Main.h"
#include "SchM_ComM.h"
#include "ComM.h"
#include "ComM_Internal.h"
#include "ComM_BswM.h"
#include "BswM_ComM.h"
#include "Ecu_HostBusSM.h"
#include "Com.h"
#include "Ecu_Target.h"
#include "ComM_Cfg.h"
#include "Dcm_ComM.h"
#include "comm-user-config.c"

#define CHECK(condition)                                                                           \
    do {                                                                                           \
        if (!(condition)) {                                                                        \
            return __LINE__;                                                                       \
        }                                                                                          \
    } while (0)

static int owner = 1;
static ComM_ModeType actual;
static uint32 host_notifications;
static uint32 second_notifications;
static boolean reenter;
int Ecu_TargetIsOwner(void) { return owner; }
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    (void)starting;
    return E_OK;
}
Std_ReturnType Ecu_HostBusSM_RequestComMode(NetworkHandleType Channel, ComM_ModeType ComMode)
{
    (void)Channel;
    (void)ComMode;
    return E_OK;
}
Std_ReturnType Ecu_HostBusSM_GetCurrentComMode(NetworkHandleType Channel, ComM_ModeType *ComMode)
{
    (void)Channel;
    *ComMode = actual;
    return E_OK;
}
void BswM_ComM_CurrentMode(NetworkHandleType Network, ComM_ModeType CurrentMode)
{
    (void)Network;
    (void)CurrentMode;
}
void Dcm_ComM_NoComModeEntered(uint8 NetworkId) { (void)NetworkId; }
void Dcm_ComM_SilentComModeEntered(uint8 NetworkId) { (void)NetworkId; }
void Dcm_ComM_FullComModeEntered(uint8 NetworkId) { (void)NetworkId; }
uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_NOT_OK;
}
uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_NOT_OK;
}
Std_ReturnType __real_SchM_Switch_ComM_UM_HostUser(Rte_ModeType_ComMMode mode);
Std_ReturnType __wrap_SchM_Switch_ComM_UM_HostUser(Rte_ModeType_ComMMode mode)
{
    ++host_notifications;
    {
        const Std_ReturnType result = __real_SchM_Switch_ComM_UM_HostUser(mode);
        if (reenter == TRUE) {
            reenter = FALSE;
            ComM_BusSM_ModeIndication(comm_config.channel, COMM_NO_COMMUNICATION);
        }
        return result;
    }
}
Std_ReturnType __real_SchM_Switch_ComM_UM_SecondUser(Rte_ModeType_ComMMode mode);
Std_ReturnType __wrap_SchM_Switch_ComM_UM_SecondUser(Rte_ModeType_ComMMode mode)
{
    ++second_notifications;
    return __real_SchM_Switch_ComM_UM_SecondUser(mode);
}
int main(void)
{
    /* Verifies: SWS_Rte_03793, SWS_Rte_03711, SWS_Rte_03724. */
    CHECK(Rte_Inst_ComMService != NULL_PTR);
    CHECK(sizeof(Rte_Inst_ComMService->Rte_Dummy) == sizeof(uint8));
    /* Verifies: SWS_ComM_00110, SWS_ComM_00079, SWS_ComM_00083, SWS_ComM_00085 */
    /* Verifies: SWS_ComM_01000, SWS_ComM_01011, SWS_ComM_00091, SWS_ComM_00778 */
    /* Verifies: SWS_ComM_00472, SWS_ComM_00675, SWS_ComM_00313, SWS_ComM_01012 */
    ComM_ModeType value = 99u;
    SchM_Init(&Ecu_SchMConfig);
    Rte_Start();
    CHECK(Rte_Call_Observe_HostUser_GetCurrentComMode(&value) == E_NOT_OK && value == 99u);
    ComM_Init(&comm_config);
    CHECK(host_notifications == 0u && second_notifications == 0u);
    ComM_BusSM_ModeIndication(comm_config.channel, COMM_NO_COMMUNICATION);
    CHECK(host_notifications == 0u && second_notifications == 0u);
    CHECK(Rte_Mode_Observe_ModeHostUser_currentMode() == 1u);
    CHECK(Rte_Mode_Observe_ModeSecondUser_currentMode() == 1u);
    CHECK(Rte_Call_Observe_HostUser_RequestComMode(COMM_FULL_COMMUNICATION) == E_OK);
    CHECK(Rte_Call_Observe_SecondUser_GetRequestedComMode(&value) == E_OK && value == 0u);
    CHECK(Rte_Call_Observe_HostUser_GetRequestedComMode(&value) == E_OK && value == 2u);
    CHECK(Rte_Call_Observe_SecondUser_RequestComMode(COMM_FULL_COMMUNICATION) == E_OK);
    CHECK(Rte_Call_Observe_HostUser_RequestComMode(COMM_NO_COMMUNICATION) == E_OK);
    CHECK(Rte_Call_Observe_HostUser_GetRequestedComMode(&value) == E_OK && value == 0u);
    CHECK(Rte_Call_Observe_SecondUser_GetRequestedComMode(&value) == E_OK && value == 2u);
    CHECK(Rte_Call_Observe_HostUser_RequestComMode(COMM_FULL_COMMUNICATION) == E_OK);
    CHECK(Rte_Call_Observe_SecondUser_RequestComMode(COMM_NO_COMMUNICATION) == E_OK);
    {
        ComM_ConfigType rejected = comm_config;
        ComM_UserModeConfigType bad = comm_user_modes[0];
        bad.user = COMM_NOT_USED_USER_ID;
        rejected.user_modes = &bad;
        rejected.user_mode_count = 1u;
        ComM_Init(&rejected);
        CHECK(Rte_Call_Observe_HostUser_GetRequestedComMode(&value) == E_OK && value == 2u);
        rejected.user_mode_count = 3u;
        ComM_Init(&rejected);
        CHECK(Rte_Call_Observe_HostUser_GetRequestedComMode(&value) == E_OK && value == 2u);
    }
    owner = 0;
    value = 99u;
    CHECK(Rte_Call_Observe_HostUser_GetRequestedComMode(&value) == E_NOT_OK && value == 99u);
    CHECK(Rte_Call_Observe_SecondUser_RequestComMode(COMM_FULL_COMMUNICATION) == E_NOT_OK);
    owner = 1;
    CHECK(Rte_Call_Observe_SecondUser_GetRequestedComMode(&value) == E_OK && value == 0u);
    CHECK(Rte_Call_Observe_HostUser_RequestComMode(COMM_SILENT_COMMUNICATION) == E_NOT_OK);
    value = 99u;
    CHECK(Rte_Call_Observe_HostUser_GetMaxComMode(&value) == E_OK && value == 2u);
    CHECK(Rte_Call_Observe_HostUser_GetCurrentPNCComMode(&value) == COMM_E_NO_PNC_ASSIGNED);
    CHECK(value == COMM_E_NO_PNC_ASSIGNED);
    actual = COMM_SILENT_COMMUNICATION;
    CHECK(Rte_Call_Observe_HostUser_GetCurrentComMode(&value) == E_OK && value == 1u);
    for (ComM_ModeType mode = 0u; mode <= COMM_FULL_COMMUNICATION; ++mode) {
        static const uint8 expected[] = {1u, 2u, 0u};
        ComM_BusSM_ModeIndication(comm_config.channel, mode);
        CHECK(Rte_Mode_Observe_ModeHostUser_currentMode() == expected[mode]);
        CHECK(Rte_Mode_Observe_ModeSecondUser_currentMode() == expected[mode]);
    }
    CHECK(host_notifications == 2u && second_notifications == 2u);
    ComM_BusSM_ModeIndication(comm_config.channel, COMM_FULL_COMMUNICATION);
    CHECK(host_notifications == 2u && second_notifications == 2u);
    actual = COMM_NO_COMMUNICATION;
    ComM_BusSM_ModeIndication(comm_config.channel, actual);
    CHECK(Rte_Mode_Observe_ModeHostUser_currentMode() == 1u);
    CHECK(host_notifications == 3u && second_notifications == 3u);
    reenter = TRUE;
    ComM_BusSM_ModeIndication(comm_config.channel, COMM_FULL_COMMUNICATION);
    CHECK(Rte_Mode_Observe_ModeHostUser_currentMode() == 1u);
    CHECK(Rte_Mode_Observe_ModeSecondUser_currentMode() == 1u);
    CHECK(host_notifications == 5u && second_notifications == 5u);
    Rte_Stop();
    value = 99u;
    CHECK(Rte_Call_Observe_HostUser_GetCurrentComMode(&value) != E_OK && value == 99u);
    return 0;
}
