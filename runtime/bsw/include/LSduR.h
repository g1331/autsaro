/** @file R24-11 LSduR: immutable one-to-one, no-metadata intra-partition routes. */
#ifndef LSDUR_H
#define LSDUR_H
#include "LSduR_Version.h"
#include "Std_Types.h"
#include "ComStack_Types.h"
/* Module identity (SWS_BSW_00059/00256, Appendix A.2); vendor/release identity is separate. */
#define LSDUR_MODULE_ID 132u
/** Configuration identity (CP_SWS_LSduR_91004). */
typedef uint16 LSduR_PBConfigIdType;
/** Initialization state (CP_SWS_LSduR_91005). */
typedef enum {
    LSDUR_UNINIT = 0, /**< No valid route set has been published. */
    LSDUR_ONLINE = 1  /**< The first valid route set remains published. */
} LSduR_StateType;
/** Implementation route owner; keeps equal upper IDs in separate module namespaces. */
typedef enum {
    LSDUR_UP_PDUR = 0, /**< Source or destination is PDUR. */
    LSDUR_UP_CANTP = 1 /**< Source or destination is CANTP. */
} LSduR_UpperType;
/** Synchronous upper receive callback; descriptors and bytes are borrowed until return. */
typedef void (*LSduR_RxType)(PduIdType id, const PduInfoType *info);
/** Upper completion callback; called outside the router area with the mapped ID and final status.
 */
typedef void (*LSduR_ConfirmationType)(PduIdType id, Std_ReturnType result);
/** Upper buffer-fill callback; info carries input capacity and output bytes/length and is not
 * retained. */
typedef Std_ReturnType (*LSduR_TriggerType)(PduIdType id, PduInfoType *info);
/** Generated one-to-one receive route. Handle fields come from the route PDU references; callbacks
 * are implementation bindings, not independent ECUC parameters. */
typedef struct {
    PduIdType lower;         /**< Lower source handle, in that module's Rx namespace. */
    PduIdType upper;         /**< Mapped upper destination handle, in its Rx namespace. */
    LSduR_RxType indication; /**< Required synchronous upper receive callback. */
} LSduR_RxRouteType;
/** Generated one-to-one transmit route. Handles come from source/destination PDU references; module
 * tags and callbacks are derived implementation bindings. */
typedef struct {
    LSduR_UpperType module; /**< Owner of the upper Tx handle. */
    PduIdType upper;        /**< Source handle, scoped by module. */
    PduIdType lower;        /**< Destination handle, scoped by the lower module. */
    LSduR_ConfirmationType
        confirmation; /**< Required upper result callback for interface routes. */
    LSduR_TriggerType
        trigger; /**< Upper buffer-fill callback; NULL means trigger is unavailable. */
} LSduR_TxRouteType;
/** Implementation configuration (CP_SWS_LSduR_91003). Tables, counts and callback bindings are
 * derived from the validated routing plan; LSduR has no accepted standalone ECUC module. The
 * accepted storage lives for the ECU lifetime. */
typedef struct {
    LSduR_PBConfigIdType id; /**< Generated identity; zero in the selected precompile profile. */
    const LSduR_RxRouteType
        *receive;         /**< Immutable Rx table; NULL only when receive_count is zero. */
    uint16 receive_count; /**< Rx route count, 0..32. */
    const LSduR_TxRouteType
        *transmit;         /**< Immutable Tx table; NULL only when transmit_count is zero. */
    uint16 transmit_count; /**< Tx route count, 0..32. */
} LSduR_PBConfigType;
/** Publish the first valid immutable route set (CP_SWS_LSduR_91006).
 * @param[in] ConfigPtr Generated configuration and tables; at most 32 routes in each table. NULL or
 * invalid sets are ignored.
 * @par Return
 * None.
 * Preconditions: Uninitialized; repeated calls after successful initialization leave the first set
 * unchanged. Context: startup task; not inside an exclusive area. Reentrancy: not reentrant.
 * Ownership: The accepted configuration and all referenced tables/callbacks remain valid for the
 * ECU lifetime.
 */
void LSduR_Init(const LSduR_PBConfigType *ConfigPtr);
/** Read the published configuration identity (CP_SWS_LSduR_91008).
 * @return The configured ID, or zero before initialization. Zero may also be a valid configured ID.
 * Preconditions: None.
 * Context: task or upper-layer callback; not inside an exclusive area.
 * Reentrancy: reentrant.
 * Ownership: None.
 */
LSduR_PBConfigIdType LSduR_GetConfigurationId(void);
#endif
