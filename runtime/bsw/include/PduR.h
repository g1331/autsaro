/** @file Selected standard PDU routing, immutable one-to-one routes. */
#ifndef PDUR_H
#define PDUR_H
#include "PduR_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "PduR.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "PduR.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
#define PDUR_MODULE_ID 51u
/** Configuration identity (SWS_PduR_00771). */
typedef uint16 PduR_PBConfigIdType;
/** Routing-path group handle (SWS_PduR_00654); no groups are selected. */
typedef uint16 PduR_RoutingPathGroupIdType;
/** Initialization state (SWS_PduR_00742). */
typedef enum {
    PDUR_UNINIT = 0, /**< No valid route set has been published. */
    PDUR_ONLINE = 1  /**< The first valid route set remains published. */
} PduR_StateType;
/** Implementation route owner; keeps equal upper IDs in separate module namespaces. */
typedef enum {
    PDUR_UP_COM = 0, /**< Source or destination is COM. */
    PDUR_UP_DCM = 1  /**< Source or destination is DCM. */
} PduR_UpperType;
/** Implementation lower-module identity; selects a separate handle namespace.
 * Lower module of a transmit route: each owns a separate handle namespace.
 */
typedef enum {
    PDUR_LO_LSDUR = 0, /**< Unbuffered interface route through LSduR. */
    PDUR_LO_CANTP = 1  /**< Transport route through CanTp. */
} PduR_LowerType;
/** Lower transmit callback; borrows info until return and reports request acceptance. */
typedef Std_ReturnType (*PduR_TransmitType)(PduIdType id, const PduInfoType *info);
/** Synchronous upper receive callback; descriptors and bytes are borrowed until return. */
typedef void (*PduR_ReceiveType)(PduIdType id, const PduInfoType *info);
/** Upper completion callback; called outside the router area with the mapped ID and final status.
 */
typedef void (*PduR_ConfirmationType)(PduIdType id, Std_ReturnType result);
/** Upper buffer-fill callback; info carries input capacity and output bytes/length and is not
 * retained. */
typedef Std_ReturnType (*PduR_TriggerType)(PduIdType id, PduInfoType *info);
/** Upper TP admission callback; receives borrowed first-frame data and writes available capacity on
 * acceptance. */
typedef BufReq_ReturnType (*PduR_StartType)(PduIdType id,
                                            const PduInfoType *info,
                                            PduLengthType length,
                                            PduLengthType *available);
/** Upper TP copy callback; consumes borrowed segment data and writes remaining capacity. */
typedef BufReq_ReturnType (*PduR_CopyRxType)(PduIdType id,
                                             const PduInfoType *info,
                                             PduLengthType *available);
/** Upper TP fill callback; info and retry are inputs, destination bytes and available count are
 * outputs. */
typedef BufReq_ReturnType (*PduR_CopyTxType)(PduIdType id,
                                             const PduInfoType *info,
                                             const RetryInfoType *retry,
                                             PduLengthType *available);
/** Implementation unbuffered CanTp/Dcm route, derived from PduR source/destination PDU references.
 * The two directions have independent handle namespaces; each callback is required. */
typedef struct {
    PduIdType lower_rx;                 /**< CanTp source Rx N-SDU handle. */
    PduIdType upper_rx;                 /**< Dcm destination Rx N-SDU handle. */
    PduIdType lower_tx;                 /**< CanTp destination Tx N-SDU handle. */
    PduIdType upper_tx;                 /**< Dcm source Tx N-SDU handle. */
    PduR_StartType start;               /**< Dcm reception-admission callback. */
    PduR_CopyRxType copy_rx;            /**< Dcm segment-consumption callback. */
    PduR_ConfirmationType receive;      /**< Dcm reception-result callback. */
    PduR_CopyTxType copy_tx;            /**< Dcm transmit-buffer callback. */
    PduR_ConfirmationType confirmation; /**< Dcm transmission-result callback. */
} PduR_TpRouteType;
/** Generated one-to-one receive route. Handle fields come from the route PDU references; callbacks
 * are implementation bindings, not independent ECUC parameters. */
typedef struct {
    PduIdType lower;             /**< Lower source handle, in that module's Rx namespace. */
    PduIdType upper;             /**< Mapped upper destination handle, in its Rx namespace. */
    PduR_ReceiveType indication; /**< Required synchronous upper receive callback. */
} PduR_RxRouteType;
/** Generated one-to-one transmit route. Handles come from source/destination PDU references; module
 * tags and callbacks are derived implementation bindings. */
typedef struct {
    PduR_UpperType module;       /**< Owner of the upper Tx handle. */
    PduIdType upper;             /**< Source handle, scoped by module. */
    PduR_LowerType lower_module; /**< Destination module; separates LSduR and CanTp IDs. */
    PduIdType lower;             /**< Destination handle, scoped by the lower module. */
    PduR_TransmitType transmit;  /**< Required lower request callback; no buffering in PduR. */
    PduR_ConfirmationType confirmation; /**< Required upper result callback for interface routes. */
    PduR_TriggerType trigger; /**< Upper buffer-fill callback; NULL means trigger is unavailable. */
} PduR_TxRouteType;
/** Implementation configuration (SWS_PduR_00743). Tables, counts and callback bindings are derived
 * from the validated routing plan; LSduR has no accepted standalone ECUC module. The accepted
 * storage lives for the ECU lifetime. */
typedef struct {
    PduR_PBConfigIdType id; /**< PduRConfigurationId; zero is a valid configured identity. */
    const PduR_RxRouteType
        *receive;         /**< Immutable Rx table; NULL only when receive_count is zero. */
    uint16 receive_count; /**< Rx route count, 0..32. */
    const PduR_TxRouteType
        *transmit;         /**< Immutable Tx table; NULL only when transmit_count is zero. */
    uint16 transmit_count; /**< Tx route count, 0..32. */
    const PduR_TpRouteType
        *transport;         /**< Immutable TP table; NULL only when transport_count is zero. */
    uint16 transport_count; /**< TP route count, 0..32. */
} PduR_PBConfigType;
#define PDUR_START_SEC_CODE
#include "PduR_MemMap.h"
/** Publish the first valid immutable route set (SWS_PduR_00334).
 * @param[in] ConfigPtr Generated configuration and tables; at most 32 routes in each table. NULL or
 * invalid sets are ignored.
 * @par Return
 * None.
 * Preconditions: Uninitialized; repeated calls after successful initialization leave the first set
 * unchanged. Context: startup task; not inside an exclusive area. Reentrancy: not reentrant.
 * Ownership: The accepted configuration and all referenced tables/callbacks remain valid for the
 * ECU lifetime.
 */
PDUR_CODE void PduR_Init(const PduR_PBConfigType *ConfigPtr);
/** Read the published configuration identity (SWS_PduR_00341).
 * @return The configured ID, or zero before initialization. Zero may also be a valid configured ID.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
PDUR_CODE PduR_PBConfigIdType PduR_GetConfigurationId(void);
/** No routing groups are selected, so every request returns without action (SWS_PduR_00615/00647).
 * @param[in] id Any routing-path group ID; no groups are configured.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
PDUR_CODE void PduR_EnableRouting(PduR_RoutingPathGroupIdType id);
/** No routing groups are selected, so every request returns without action (SWS_PduR_00617/00716).
 * @param[in] id Any routing-path group ID; no groups are configured.
 * @param[in] initialize TRUE or FALSE; unused because no routing-path group exists.
 * @par Return
 * None.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
PDUR_CODE void PduR_DisableRouting(PduR_RoutingPathGroupIdType id, boolean initialize);
#define PDUR_STOP_SEC_CODE
#include "PduR_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "PduR.h -> PduR_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "PduR.h -> PduR_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
