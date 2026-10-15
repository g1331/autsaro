/** @file Selected Dcm service types and fixed mode declarations of the host RTE. */
#ifndef RTE_DCM_TYPE_H
#define RTE_DCM_TYPE_H
#include "Std_Types.h"
/** SWS_Dcm_00984: standard public Dcm_OpStatusType. */
typedef uint8 Dcm_OpStatusType;
#define DCM_INITIAL        0x00u
#define DCM_PENDING        0x01u
#define DCM_CANCEL         0x02u
#define DCM_FORCE_RCRRP_OK 0x03u
/** SWS_Dcm_00983: standard public Dcm_ConfirmationStatusType. */
typedef uint8 Dcm_ConfirmationStatusType;
#define DCM_RES_POS_OK     0x00u
#define DCM_RES_POS_NOT_OK 0x01u
#define DCM_RES_NEG_OK     0x02u
#define DCM_RES_NEG_NOT_OK 0x03u
/** SWS_Dcm_00979: standard public Dcm_ProtocolType. */
typedef uint8 Dcm_ProtocolType;
#define DCM_OBD_ON_CAN               0x00u
#define DCM_OBD_ON_FLEXRAY           0x01u
#define DCM_OBD_ON_IP                0x02u
#define DCM_UDS_ON_CAN               0x03u
#define DCM_UDS_ON_FLEXRAY           0x04u
#define DCM_UDS_ON_IP                0x05u
#define DCM_ROE_ON_CAN               0x06u
#define DCM_ROE_ON_FLEXRAY           0x07u
#define DCM_ROE_ON_IP                0x08u
#define DCM_PERIODICTRANS_ON_CAN     0x09u
#define DCM_PERIODICTRANS_ON_FLEXRAY 0x0Au
#define DCM_PERIODICTRANS_ON_IP      0x0Bu
#define DCM_NO_ACTIVE_PROTOCOL       0x0Cu
#define DCM_UDS_ON_LIN               0x0Du
#define DCM_SUPPLIER_1               0xF0u
#define DCM_SUPPLIER_2               0xF1u
#define DCM_SUPPLIER_3               0xF2u
#define DCM_SUPPLIER_4               0xF3u
#define DCM_SUPPLIER_5               0xF4u
#define DCM_SUPPLIER_6               0xF5u
#define DCM_SUPPLIER_7               0xF6u
#define DCM_SUPPLIER_8               0xF7u
#define DCM_SUPPLIER_9               0xF8u
#define DCM_SUPPLIER_10              0xF9u
#define DCM_SUPPLIER_11              0xFAu
#define DCM_SUPPLIER_12              0xFBu
#define DCM_SUPPLIER_13              0xFCu
#define DCM_SUPPLIER_14              0xFDu
#define DCM_SUPPLIER_15              0xFEu
/** SWS_Dcm_00980: standard public Dcm_NegativeResponseCodeType. */
typedef uint8 Dcm_NegativeResponseCodeType;
#define DCM_POS_RESP                                    0x00u
#define DCM_E_GENERALREJECT                             0x10u
#define DCM_E_SERVICENOTSUPPORTED                       0x11u
#define DCM_E_SUBFUNCTIONNOTSUPPORTED                   0x12u
#define DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT     0x13u
#define DCM_E_RESPONSETOOLONG                           0x14u
#define DCM_E_BUSYREPEATREQUEST                         0x21u
#define DCM_E_CONDITIONSNOTCORRECT                      0x22u
#define DCM_E_REQUESTSEQUENCEERROR                      0x24u
#define DCM_E_NORESPONSEFROMSUBNETCOMPONENT             0x25u
#define DCM_E_FAILUREPREVENTSEXECUTIONOFREQUESTEDACTION 0x26u
#define DCM_E_REQUESTOUTOFRANGE                         0x31u
#define DCM_E_SECURITYACCESSDENIED                      0x33u
#define DCM_E_INVALIDKEY                                0x35u
#define DCM_E_EXCEEDNUMBEROFATTEMPTS                    0x36u
#define DCM_E_REQUIREDTIMEDELAYNOTEXPIRED               0x37u
#define DCM_E_UPLOADDOWNLOADNOTACCEPTED                 0x70u
#define DCM_E_TRANSFERDATASUSPENDED                     0x71u
#define DCM_E_GENERALPROGRAMMINGFAILURE                 0x72u
#define DCM_E_WRONGBLOCKSEQUENCECOUNTER                 0x73u
#define DCM_E_SUBFUNCTIONNOTSUPPORTEDINACTIVESESSION    0x7Eu
#define DCM_E_SERVICENOTSUPPORTEDINACTIVESESSION        0x7Fu
#define DCM_E_RPMTOOHIGH                                0x81u
#define DCM_E_RPMTOOLOW                                 0x82u
#define DCM_E_ENGINEISRUNNING                           0x83u
#define DCM_E_ENGINEISNOTRUNNING                        0x84u
#define DCM_E_ENGINERUNTIMETOOLOW                       0x85u
#define DCM_E_TEMPERATURETOOHIGH                        0x86u
#define DCM_E_TEMPERATURETOOLOW                         0x87u
#define DCM_E_VEHICLESPEEDTOOHIGH                       0x88u
#define DCM_E_VEHICLESPEEDTOOLOW                        0x89u
#define DCM_E_THROTTLE_PEDALTOOHIGH                     0x8Au
#define DCM_E_THROTTLE_PEDALTOOLOW                      0x8Bu
#define DCM_E_TRANSMISSIONRANGENOTINNEUTRAL             0x8Cu
#define DCM_E_TRANSMISSIONRANGENOTINGEAR                0x8Du
#define DCM_E_BRAKESWITCH_NOTCLOSED                     0x8Fu
#define DCM_E_SHIFTERLEVERNOTINPARK                     0x90u
#define DCM_E_TORQUECONVERTERCLUTCHLOCKED               0x91u
#define DCM_E_VOLTAGETOOHIGH                            0x92u
#define DCM_E_VOLTAGETOOLOW                             0x93u
#define DCM_E_VMSCNC_0                                  0xF0u
#define DCM_E_VMSCNC_1                                  0xF1u
#define DCM_E_VMSCNC_2                                  0xF2u
#define DCM_E_VMSCNC_3                                  0xF3u
#define DCM_E_VMSCNC_4                                  0xF4u
#define DCM_E_VMSCNC_5                                  0xF5u
#define DCM_E_VMSCNC_6                                  0xF6u
#define DCM_E_VMSCNC_7                                  0xF7u
#define DCM_E_VMSCNC_8                                  0xF8u
#define DCM_E_VMSCNC_9                                  0xF9u
#define DCM_E_VMSCNC_A                                  0xFAu
#define DCM_E_VMSCNC_B                                  0xFBu
#define DCM_E_VMSCNC_C                                  0xFCu
#define DCM_E_VMSCNC_D                                  0xFDu
#define DCM_E_VMSCNC_E                                  0xFEu
/** SWS_Dcm_00977: selected SecurityAccess level; only the locked value is active. */
typedef uint8 Dcm_SecLevelType;
/** SWS_Dcm_00978: UDS session ID, distinct from the mode declaration value. */
typedef uint8 Dcm_SesCtrlType;
/** Standard UDS values (SWS_Dcm_00977, SWS_Dcm_00978); sessions 2/4 are not selected. */
#define DCM_SEC_LEV_LOCKED                   0u
#define DCM_DEFAULT_SESSION                  1u
#define DCM_PROGRAMMING_SESSION              2u
#define DCM_EXTENDED_DIAGNOSTIC_SESSION      3u
#define DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 4u
/** SWS_Dcm_91019: explicit-order session mode; startup/default is 0. */
typedef uint8 Rte_ModeType_DcmDiagnosticSessionControl;
/** SWS_Dcm_91019: session modes and the transition sentinel. */
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION                  0u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_PROGRAMMING_SESSION              1u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION      2u
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 3u
#define RTE_TRANSITION_DcmDiagnosticSessionControl                                255u
/* Declaration-only modes: their optional diagnostic services are not selected. */
/** Standard declaration-only DcmEcuReset mode; its service is not selected. */
typedef uint8 Rte_ModeType_DcmEcuReset;
#define RTE_MODE_DcmEcuReset_DCM_NONE                        0u
#define RTE_MODE_DcmEcuReset_DCM_HARD                        1u
#define RTE_MODE_DcmEcuReset_DCM_KEYONOFF                    2u
#define RTE_MODE_DcmEcuReset_DCM_SOFT                        3u
#define RTE_MODE_DcmEcuReset_DCM_JUMPTOBOOTLOADER            4u
#define RTE_MODE_DcmEcuReset_DCM_JUMPTOSYSSUPPLIERBOOTLOADER 5u
#define RTE_MODE_DcmEcuReset_DCM_EXECUTE                     6u
#define RTE_TRANSITION_DcmEcuReset                           255u
/** Standard declaration-only DcmModeRapidPowerShutDown mode; its service is not selected. */
typedef uint8 Rte_ModeType_DcmModeRapidPowerShutDown;
#define RTE_MODE_DcmModeRapidPowerShutDown_DCM_ENABLE_RAPIDPOWERSHUTDOWN  0u
#define RTE_MODE_DcmModeRapidPowerShutDown_DCM_DISABLE_RAPIDPOWERSHUTDOWN 1u
#define RTE_TRANSITION_DcmModeRapidPowerShutDown                          255u
/** Standard declaration-only DcmControlDTCSetting mode; its service is not selected. */
typedef uint8 Rte_ModeType_DcmControlDTCSetting;
#define RTE_MODE_DcmControlDTCSetting_DCM_ENABLEDTCSETTING  0u
#define RTE_MODE_DcmControlDTCSetting_DCM_DISABLEDTCSETTING 1u
#define RTE_TRANSITION_DcmControlDTCSetting                 255u
/** Standard declaration-only DcmSecurityAccess mode; its service is not selected. */
typedef uint8 Rte_ModeType_DcmSecurityAccess;
#define RTE_MODE_DcmSecurityAccess_DCM_SEC_LEV_LOCKED 0u
#define RTE_TRANSITION_DcmSecurityAccess              255u
#endif
