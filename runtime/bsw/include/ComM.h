/** @file Single-channel CDD/NM NONE manager, no PNC or inhibition/NvM.
 * Configuration remains immutable for the initialized lifetime. All users map
 * to this channel; users belong to host BSW, without SW-C mode ports.
 */
#ifndef COMM_H
#define COMM_H
#include "ComStack_Types.h"
#include "Rte_ComM_Type.h"
#define COMM_E_MODE_LIMITATION 2u
#define COMM_E_MULTIPLE_PNC_ASSIGNED 3u
#define COMM_E_NO_PNC_ASSIGNED 4u
typedef enum { COMM_UNINIT = 0, COMM_INIT = 1 } ComM_InitStatusType;
typedef uint8 ComM_StateType;
#define COMM_NO_COM_NO_PENDING_REQUEST 0u
#define COMM_NO_COM_REQUEST_PENDING 1u
#define COMM_FULL_COM_NETWORK_REQUESTED 2u
#define COMM_FULL_COM_READY_SLEEP 3u
#define COMM_SILENT_COM 4u
typedef Std_ReturnType (*ComM_BusRequestType)(NetworkHandleType Channel, ComM_ModeType ComMode);
typedef Std_ReturnType (*ComM_BusGetType)(NetworkHandleType Channel, ComM_ModeType *ComMode);
typedef void (*ComM_ModeNotificationType)(NetworkHandleType Channel, ComM_ModeType ComMode);
typedef struct {
    NetworkHandleType channel;
    const ComM_UserHandleType *users;
    uint16 user_count;
    uint32 minimum_full_ticks;
    ComM_BusRequestType request;
    ComM_BusGetType current;
    ComM_ModeNotificationType notification;
    ComM_InhibitionStatusType ecu_group_classification;
} ComM_ConfigType;
void ComM_Init(const ComM_ConfigType *ConfigPtr);
/** DeInit only succeeds in NO_COM_NO_PENDING_REQUEST and actual NO mode. */
void ComM_DeInit(void);
Std_ReturnType ComM_GetStatus(ComM_InitStatusType *Status);
Std_ReturnType ComM_GetInhibitionStatus(NetworkHandleType Channel,
                                        ComM_InhibitionStatusType *Status);
Std_ReturnType ComM_SetECUGroupClassification(ComM_InhibitionStatusType Status);
Std_ReturnType ComM_RequestComMode(ComM_UserHandleType User, ComM_ModeType ComMode);
Std_ReturnType ComM_GetRequestedComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
Std_ReturnType ComM_GetCurrentComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
Std_ReturnType ComM_GetCurrentPNCComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
Std_ReturnType ComM_GetMaxComMode(ComM_UserHandleType User, ComM_ModeType *ComMode);
void ComM_BusSM_ModeIndication(NetworkHandleType Channel, ComM_ModeType ComMode);
#endif
