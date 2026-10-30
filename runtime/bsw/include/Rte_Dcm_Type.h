/** @file Selected Dcm service types and fixed mode declarations of the host
 * RTE. */
#ifndef RTE_DCM_TYPE_H
#define RTE_DCM_TYPE_H
#include "Dcm_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Rte_Dcm_Type.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Rte_Dcm_Type.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
/** SWS_Dcm_00984: standard public Dcm_OpStatusType. */
typedef uint8 Dcm_OpStatusType;
/** SWS_Dcm_00984: First invocation of an operation.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_INITIAL 0x00u
/** SWS_Dcm_00984: Resume an operation that returned pending.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_PENDING 0x01u
/** SWS_Dcm_00984: Cancel a pending operation.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_CANCEL 0x02u
/** SWS_Dcm_00984: Response-pending transmission completed.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_FORCE_RCRRP_OK 0x03u
/** SWS_Dcm_00983: standard public Dcm_ConfirmationStatusType. */
typedef uint8 Dcm_ConfirmationStatusType;
/** SWS_Dcm_00983: Positive response transmitted successfully.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_RES_POS_OK 0x00u
/** SWS_Dcm_00983: Positive response transmission failed.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_RES_POS_NOT_OK 0x01u
/** SWS_Dcm_00983: Negative response transmitted successfully.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_RES_NEG_OK 0x02u
/** SWS_Dcm_00983: Negative response transmission failed.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_RES_NEG_NOT_OK 0x03u
/** SWS_Dcm_00979: standard public Dcm_ProtocolType. */
typedef uint8 Dcm_ProtocolType;
/** SWS_Dcm_00979: OBD protocol over CAN.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_OBD_ON_CAN 0x00u
/** SWS_Dcm_00979: OBD protocol over FlexRay.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_OBD_ON_FLEXRAY 0x01u
/** SWS_Dcm_00979: OBD protocol over IP.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_OBD_ON_IP 0x02u
/** SWS_Dcm_00979: UDS protocol over CAN.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_UDS_ON_CAN 0x03u
/** SWS_Dcm_00979: UDS protocol over FlexRay.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_UDS_ON_FLEXRAY 0x04u
/** SWS_Dcm_00979: UDS protocol over IP.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_UDS_ON_IP 0x05u
/** SWS_Dcm_00979: Response-on-event protocol over CAN.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_ROE_ON_CAN 0x06u
/** SWS_Dcm_00979: Response-on-event protocol over FlexRay.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_ROE_ON_FLEXRAY 0x07u
/** SWS_Dcm_00979: Response-on-event protocol over IP.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_ROE_ON_IP 0x08u
/** SWS_Dcm_00979: Periodic transmission protocol over CAN.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_PERIODICTRANS_ON_CAN 0x09u
/** SWS_Dcm_00979: Periodic transmission protocol over FlexRay.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_PERIODICTRANS_ON_FLEXRAY 0x0Au
/** SWS_Dcm_00979: Periodic transmission protocol over IP.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_PERIODICTRANS_ON_IP 0x0Bu
/** SWS_Dcm_00979: No diagnostic protocol is active.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_NO_ACTIVE_PROTOCOL 0x0Cu
/** SWS_Dcm_00979: UDS protocol over LIN.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_UDS_ON_LIN 0x0Du
/** SWS_Dcm_00979: Supplier protocol slot 1.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_1 0xF0u
/** SWS_Dcm_00979: Supplier protocol slot 2.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_2 0xF1u
/** SWS_Dcm_00979: Supplier protocol slot 3.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_3 0xF2u
/** SWS_Dcm_00979: Supplier protocol slot 4.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_4 0xF3u
/** SWS_Dcm_00979: Supplier protocol slot 5.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_5 0xF4u
/** SWS_Dcm_00979: Supplier protocol slot 6.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_6 0xF5u
/** SWS_Dcm_00979: Supplier protocol slot 7.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_7 0xF6u
/** SWS_Dcm_00979: Supplier protocol slot 8.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_8 0xF7u
/** SWS_Dcm_00979: Supplier protocol slot 9.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_9 0xF8u
/** SWS_Dcm_00979: Supplier protocol slot 10.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_10 0xF9u
/** SWS_Dcm_00979: Supplier protocol slot 11.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_11 0xFAu
/** SWS_Dcm_00979: Supplier protocol slot 12.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_12 0xFBu
/** SWS_Dcm_00979: Supplier protocol slot 13.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_13 0xFCu
/** SWS_Dcm_00979: Supplier protocol slot 14.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_14 0xFDu
/** SWS_Dcm_00979: Supplier protocol slot 15.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SUPPLIER_15 0xFEu
/** SWS_Dcm_00980: standard public Dcm_NegativeResponseCodeType. */
typedef uint8 Dcm_NegativeResponseCodeType;
/** SWS_Dcm_00980: Positive diagnostic result.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_POS_RESP 0x00u
/** SWS_Dcm_00980: Request rejected without a more specific cause. */
#define DCM_E_GENERALREJECT 0x10u
/** SWS_Dcm_00980: Requested service is unavailable. */
#define DCM_E_SERVICENOTSUPPORTED 0x11u
/** SWS_Dcm_00980: Requested subfunction is unavailable. */
#define DCM_E_SUBFUNCTIONNOTSUPPORTED 0x12u
/** SWS_Dcm_00980: Request length or encoding is invalid. */
#define DCM_E_INCORRECTMESSAGELENGTHORINVALIDFORMAT 0x13u
/** SWS_Dcm_00980: Response cannot fit the transport. */
#define DCM_E_RESPONSETOOLONG 0x14u
/** SWS_Dcm_00980: Server is busy; requester may try again.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_BUSYREPEATREQUEST 0x21u
/** SWS_Dcm_00980: Required operating conditions are absent.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_CONDITIONSNOTCORRECT 0x22u
/** SWS_Dcm_00980: Request is out of sequence.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_REQUESTSEQUENCEERROR 0x24u
/** SWS_Dcm_00980: A subnet participant did not respond.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_NORESPONSEFROMSUBNETCOMPONENT 0x25u
/** SWS_Dcm_00980: A detected failure blocks the action.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_FAILUREPREVENTSEXECUTIONOFREQUESTEDACTION 0x26u
/** SWS_Dcm_00980: Requested parameter is outside its supported range. */
#define DCM_E_REQUESTOUTOFRANGE 0x31u
/** SWS_Dcm_00980: Required security access is absent.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_SECURITYACCESSDENIED 0x33u
/** SWS_Dcm_00980: Security key was rejected.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_INVALIDKEY 0x35u
/** SWS_Dcm_00980: Security attempt limit was reached.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_EXCEEDNUMBEROFATTEMPTS 0x36u
/** SWS_Dcm_00980: Security retry delay remains active.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_REQUIREDTIMEDELAYNOTEXPIRED 0x37u
/** SWS_Dcm_00980: Transfer setup was rejected.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_UPLOADDOWNLOADNOTACCEPTED 0x70u
/** SWS_Dcm_00980: Transfer was suspended.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TRANSFERDATASUSPENDED 0x71u
/** SWS_Dcm_00980: Programming operation failed.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_GENERALPROGRAMMINGFAILURE 0x72u
/** SWS_Dcm_00980: Transfer block sequence is invalid.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_WRONGBLOCKSEQUENCECOUNTER 0x73u
/** SWS_Dcm_00980: Subfunction is unavailable in this session.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_SUBFUNCTIONNOTSUPPORTEDINACTIVESESSION 0x7Eu
/** SWS_Dcm_00980: Service is unavailable in this session.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_SERVICENOTSUPPORTEDINACTIVESESSION 0x7Fu
/** SWS_Dcm_00980: Engine speed exceeds the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_RPMTOOHIGH 0x81u
/** SWS_Dcm_00980: Engine speed is below the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_RPMTOOLOW 0x82u
/** SWS_Dcm_00980: Action requires a stopped engine.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_ENGINEISRUNNING 0x83u
/** SWS_Dcm_00980: Action requires a running engine.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_ENGINEISNOTRUNNING 0x84u
/** SWS_Dcm_00980: Engine has not run long enough.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_ENGINERUNTIMETOOLOW 0x85u
/** SWS_Dcm_00980: Temperature exceeds the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TEMPERATURETOOHIGH 0x86u
/** SWS_Dcm_00980: Temperature is below the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TEMPERATURETOOLOW 0x87u
/** SWS_Dcm_00980: Vehicle speed exceeds the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VEHICLESPEEDTOOHIGH 0x88u
/** SWS_Dcm_00980: Vehicle speed is below the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VEHICLESPEEDTOOLOW 0x89u
/** SWS_Dcm_00980: Throttle opening exceeds the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_THROTTLE_PEDALTOOHIGH 0x8Au
/** SWS_Dcm_00980: Throttle opening is below the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_THROTTLE_PEDALTOOLOW 0x8Bu
/** SWS_Dcm_00980: Action requires neutral transmission.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TRANSMISSIONRANGENOTINNEUTRAL 0x8Cu
/** SWS_Dcm_00980: Action requires a selected gear.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TRANSMISSIONRANGENOTINGEAR 0x8Du
/** SWS_Dcm_00980: Action requires the brake switch.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_BRAKESWITCH_NOTCLOSED 0x8Fu
/** SWS_Dcm_00980: Action requires the park position.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_SHIFTERLEVERNOTINPARK 0x90u
/** SWS_Dcm_00980: Converter clutch state blocks the action.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_TORQUECONVERTERCLUTCHLOCKED 0x91u
/** SWS_Dcm_00980: Supply voltage exceeds the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VOLTAGETOOHIGH 0x92u
/** SWS_Dcm_00980: Supply voltage is below the permitted range.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VOLTAGETOOLOW 0x93u
/** SWS_Dcm_00980: Manufacturer-specific condition code 0.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_0 0xF0u
/** SWS_Dcm_00980: Manufacturer-specific condition code 1.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_1 0xF1u
/** SWS_Dcm_00980: Manufacturer-specific condition code 2.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_2 0xF2u
/** SWS_Dcm_00980: Manufacturer-specific condition code 3.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_3 0xF3u
/** SWS_Dcm_00980: Manufacturer-specific condition code 4.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_4 0xF4u
/** SWS_Dcm_00980: Manufacturer-specific condition code 5.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_5 0xF5u
/** SWS_Dcm_00980: Manufacturer-specific condition code 6.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_6 0xF6u
/** SWS_Dcm_00980: Manufacturer-specific condition code 7.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_7 0xF7u
/** SWS_Dcm_00980: Manufacturer-specific condition code 8.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_8 0xF8u
/** SWS_Dcm_00980: Manufacturer-specific condition code 9.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_9 0xF9u
/** SWS_Dcm_00980: Manufacturer-specific condition code A.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_A 0xFAu
/** SWS_Dcm_00980: Manufacturer-specific condition code B.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_B 0xFBu
/** SWS_Dcm_00980: Manufacturer-specific condition code C.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_C 0xFCu
/** SWS_Dcm_00980: Manufacturer-specific condition code D.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_D 0xFDu
/** SWS_Dcm_00980: Manufacturer-specific condition code E.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_E_VMSCNC_E 0xFEu
/** SWS_Dcm_00977: selected SecurityAccess level; only the locked value is
 * active. */
typedef uint8 Dcm_SecLevelType;
/** SWS_Dcm_00978: UDS session ID, distinct from the mode declaration value. */
typedef uint8 Dcm_SesCtrlType;
/** Standard UDS values (SWS_Dcm_00977, SWS_Dcm_00978); sessions 2/4 are not
 * selected. */
/** SWS_Dcm_00977: Security level before access is granted. */
#define DCM_SEC_LEV_LOCKED 0u
/** SWS_Dcm_00978: UDS session identifier; availability is
 * configuration-dependent. */
#define DCM_DEFAULT_SESSION 1u
/** SWS_Dcm_00978: UDS session identifier; availability is
 * configuration-dependent.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_PROGRAMMING_SESSION 2u
/** SWS_Dcm_00978: UDS session identifier; availability is
 * configuration-dependent. */
#define DCM_EXTENDED_DIAGNOSTIC_SESSION 3u
/** SWS_Dcm_00978: UDS session identifier; availability is
 * configuration-dependent.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 4u
/** SWS_Dcm_91019: explicit-order session mode; startup/default is 0. */
typedef uint8 Rte_ModeType_DcmDiagnosticSessionControl;
/** SWS_Dcm_91019: session modes and the transition sentinel. */
/** SWS_Dcm_91019: Mode declaration value; distinct from the UDS service
 * encoding. */
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION 0u
/** SWS_Dcm_91019: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_PROGRAMMING_SESSION 1u
/** SWS_Dcm_91019: Mode declaration value; distinct from the UDS service
 * encoding. */
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_EXTENDED_DIAGNOSTIC_SESSION 2u
/** SWS_Dcm_91019: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmDiagnosticSessionControl_DCM_SAFETY_SYSTEM_DIAGNOSTIC_SESSION 3u
/** SWS_Dcm_91019: Mode transition sentinel; not a steady mode.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_TRANSITION_DcmDiagnosticSessionControl 255u
/* Declaration-only modes: their optional diagnostic services are not selected.
 */
/** SWS_Dcm_91021: declaration-only mode; its service is not selected.
 * MR12 RULE 2.3 VIOLATION: retain this public type when unused.
 */
typedef uint8 Rte_ModeType_DcmEcuReset;
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_NONE 0u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_HARD 1u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_KEYONOFF 2u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_SOFT 3u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_JUMPTOBOOTLOADER 4u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_JUMPTOSYSSUPPLIERBOOTLOADER 5u
/** SWS_Dcm_91021: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmEcuReset_DCM_EXECUTE 6u
/** SWS_Dcm_91021: Mode transition sentinel; not a steady mode.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_TRANSITION_DcmEcuReset 255u
/** SWS_Dcm_91023: declaration-only mode; its service is not selected.
 * MR12 RULE 2.3 VIOLATION: retain this public type when unused.
 */
typedef uint8 Rte_ModeType_DcmModeRapidPowerShutDown;
/** SWS_Dcm_91023: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmModeRapidPowerShutDown_DCM_ENABLE_RAPIDPOWERSHUTDOWN 0u
/** SWS_Dcm_91023: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmModeRapidPowerShutDown_DCM_DISABLE_RAPIDPOWERSHUTDOWN 1u
/** SWS_Dcm_91023: Mode transition sentinel; not a steady mode.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_TRANSITION_DcmModeRapidPowerShutDown 255u
/** SWS_Dcm_91027: declaration-only mode; its service is not selected.
 * MR12 RULE 2.3 VIOLATION: retain this public type when unused.
 */
typedef uint8 Rte_ModeType_DcmControlDTCSetting;
/** SWS_Dcm_91027: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmControlDTCSetting_DCM_ENABLEDTCSETTING 0u
/** SWS_Dcm_91027: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmControlDTCSetting_DCM_DISABLEDTCSETTING 1u
/** SWS_Dcm_91027: Mode transition sentinel; not a steady mode.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_TRANSITION_DcmControlDTCSetting 255u
/** SWS_Dcm_91031: declaration-only mode; its service is not selected.
 * MR12 RULE 2.3 VIOLATION: retain this public type when unused.
 */
typedef uint8 Rte_ModeType_DcmSecurityAccess;
/** SWS_Dcm_91031: Mode declaration value; distinct from the UDS service
 * encoding.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_MODE_DcmSecurityAccess_DCM_SEC_LEV_LOCKED 0u
/** SWS_Dcm_91031: Mode transition sentinel; not a steady mode.
 * MR12 RULE 2.5 VIOLATION: retain the public preprocessing contract when unused.
 */
#define RTE_TRANSITION_DcmSecurityAccess 255u
#endif
