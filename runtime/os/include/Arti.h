/** @file R24-11 ARTI tool binding for the controlled x64 host targets. */
#ifndef AUTOSAR_HOST_ARTI_H
#define AUTOSAR_HOST_ARTI_H
#include "Arti_Version.h"
#include "Std_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(STD_TYPES_AR_RELEASE_MAJOR_VERSION) || !defined(STD_TYPES_AR_RELEASE_MINOR_VERSION)
#error "Arti.h -> Std_Types.h: missing AUTOSAR release"
#elif (STD_TYPES_AR_RELEASE_MAJOR_VERSION != 4U) || (STD_TYPES_AR_RELEASE_MINOR_VERSION != 10U)
#error "Arti.h -> Std_Types.h: incompatible AUTOSAR release"
#endif
#include <stddef.h>
/** @brief Maximum number of retained events; overflow is explicitly counted. */
#define ARTI_EVENT_CAPACITY                                                                        \
    4096u /**< Maximum retained tool events; excess reservations are counted as dropped. */
#include "Arti_Types.h"
#include "Os_HostTypes.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(OS_AR_RELEASE_MAJOR_VERSION) || !defined(OS_AR_RELEASE_MINOR_VERSION)
#error "Arti.h -> Os_HostTypes.h: missing AUTOSAR release"
#elif (OS_AR_RELEASE_MAJOR_VERSION != 4U) || (OS_AR_RELEASE_MINOR_VERSION != 10U)
#error "Arti.h -> Os_HostTypes.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Retained tool event; read payloads only after OS shutdown/quiescence, not from reservation
 * count. */
typedef struct {
    const char *context;         /**< Static literal ARTI execution-context name. */
    const char *class_name;      /**< Static literal standard ARTI class name. */
    const char *instance;        /**< Static literal configured module instance name. */
    const char *event;           /**< Static literal standard event name. */
    uint32_t instance_parameter; /**< Captured instance/core payload; core zero on this target. */
    uint32_t event_parameter;    /**< Captured standard uint32 event payload. */
    uintptr_t event_address;     /**< Lossless native address bits when address_valid is nonzero. */
    uint8_t address_valid; /**< Nonzero when event_address carries captured native address bits. */
    uint32_t service_status; /**< Captured service result when status_valid is nonzero. */
    uint8_t status_valid; /**< Nonzero when service_status distinguishes a result from an error. */
    volatile Arti_Atomic32Type
        published; /**< Atomic publication flag, zero until all slot payload fields are written. */
} Arti_Event;
/** Actor-local lossless native address and service-status side data for standard uint32 event
 * payloads. */
typedef struct {
    uintptr_t address;       /**< Captured native address bits; never dereferenced here. */
    uint8_t valid;           /**< Nonzero when address is valid for the next recorded event. */
    uint32_t service_status; /**< Captured service result for the next recorded event. */
    uint8_t status_valid;    /**< Nonzero when service_status is meaningful. */
} Arti_AddressCapture;
/** @brief Lossless x64 side data for standard uint32 pointer payloads.
 * @param[in] address Native address bits captured without dereferencing the address.
 * @par Return
 * None.
 * Preconditions: initialized actor TLS; save/restore scopes are properly nested.
 * Context: current native OS actor, including the native critical section; no blocking or BSW area
 * entry. Reentrancy: actor-local; nested logical actors save and restore the complete capture.
 * Ownership: stores scalar address/status bits only; never dereferences or retains a pointed-to
 * object.
 */
OS_MEMMAP void Arti_CaptureAddress(uintptr_t address);

/** @brief Distinguish a failed getter from a genuine UINT32_MAX result.
 * @param[in] status Captured getter result distinguishing errors from valid payloads.
 * @par Return
 * None.
 * Preconditions: initialized actor TLS; save/restore scopes are properly nested.
 * Context: current native OS actor, including the native critical section; no blocking or BSW area
 * entry. Reentrancy: actor-local; nested logical actors save and restore the complete capture.
 * Ownership: stores scalar address/status bits only; never dereferences or retains a pointed-to
 * object.
 */
OS_MEMMAP void Arti_CaptureServiceStatus(uint32_t status);

/** Save and clear the current actor capture validity for a nested logical caller.
 * @return Prior capture state, to restore after the nested caller.
 * Preconditions: initialized actor TLS; save/restore scopes are properly nested.
 * Context: current native OS actor, including the native critical section; no blocking or BSW area
 * entry. Reentrancy: actor-local; nested logical actors save and restore the complete capture.
 * Ownership: stores scalar address/status bits only; never dereferences or retains a pointed-to
 * object.
 */
OS_MEMMAP Arti_AddressCapture Arti_SaveAddressCapture(void);

/** Restore the suspended logical caller address/status capture.
 * @param[in] capture Snapshot previously returned by Arti_SaveAddressCapture.
 * @par Return
 * None.
 * Preconditions: initialized actor TLS; save/restore scopes are properly nested.
 * Context: current native OS actor, including the native critical section; no blocking or BSW area
 * entry. Reentrancy: actor-local; nested logical actors save and restore the complete capture.
 * Ownership: stores scalar address/status bits only; never dereferences or retains a pointed-to
 * object.
 */
OS_MEMMAP void Arti_RestoreAddressCapture(Arti_AddressCapture capture);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Arti.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Arti.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP extern Arti_Event Arti_Events[ARTI_EVENT_CAPACITY];
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* Native atomics require the host ABI's four-byte cell type. */
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP extern volatile Arti_Atomic32Type Arti_EventCount;

OS_MEMMAP extern volatile Arti_Atomic32Type Arti_EventsDropped;

OS_MEMMAP extern volatile Arti_Atomic32Type Arti_DevelopmentError;
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
/* MR12 RULE 2.5 VIOLATION: published ARTI selectors retain their recorder-facing numeric
 * representation. */
#define ARTI_E_INIT_FAILED   0x01u /**< SWS_Arti_91002: ARTI initialization failure. */
#define ARTI_E_PARAM_POINTER 0x02u /**< SWS_Arti_91002: invalid API output pointer. */
/* MR12 RULE 2.5 VIOLATION: published ARTI selectors retain their recorder-facing numeric
 * representation. */
#define ARTI_STOPWATCH_FLAT 0x00u /**< SWS_Arti_91000: stopwatch evaluation ignores nesting. */
/* MR12 RULE 2.5 VIOLATION: published ARTI selectors retain their recorder-facing numeric
 * representation. */
#define ARTI_STOPWATCH_NESTED                                                                      \
    0x01u /**< SWS_Arti_91000: stopwatch evaluation accounts for nesting. */

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** @brief Initialize before OS actors start; resets only the tool buffer.
 * @par Return
 * None.
 * Preconditions: no OS actors exist; reinitialization during execution is outside the contract.
 * Context: native startup before actor creation, outside exclusive areas.
 * Reentrancy: non-reentrant with recording, reading or another initialization.
 * Ownership: resets event publication/count/error cells and the calling actor capture validity;
 * payload bytes need not be cleared.
 */
OS_MEMMAP void Arti_Init(void);

/** @brief Read the host ARTI binding software version.
 * @param[out] versioninfo Writable version result; NULL raises ARTI_E_PARAM_POINTER
 * in the host binding's debugger-visible development-error cell.
 * @par Return
 * None.
 * Preconditions: versioninfo is writable when non-NULL; NULL records the development error.
 * Context: native or automotive OS actor, outside BSW exclusive areas.
 * Reentrancy: reentrant with separate outputs; the development-error publication is atomic.
 * Ownership: borrows versioninfo only until return, writes it when non-NULL and retains no pointer.
 */
OS_MEMMAP static inline void Arti_GetVersionInfo(Std_VersionInfoType *versioninfo)
{
    if (versioninfo != NULL) {
        versioninfo->vendorID = 0u;
        /* BSWGeneral Appendix A.2 fixes the module ID; vendor identity is unassigned. */
        versioninfo->moduleID = ARTI_MODULE_ID;
        versioninfo->sw_major_version = ARTI_SW_MAJOR_VERSION;
        versioninfo->sw_minor_version = ARTI_SW_MINOR_VERSION;
        versioninfo->sw_patch_version = ARTI_SW_PATCH_VERSION;
    } else {
        (void)InterlockedExchange(&Arti_DevelopmentError, (LONG)ARTI_E_PARAM_POINTER);
    }
}

/** @brief Tool callback; arguments contain only captured values and literals.
 * @param[in] context Literal ARTI execution context.
 * @param[in] class_name Literal standard ARTI class.
 * @param[in] instance Literal configured module SHORT-NAME.
 * @param[in] event Literal standard event name.
 * @param[in] instance_parameter Hardware core index.
 * @param[in] event_parameter Captured event payload.
 * @par Return
 * None.
 * Preconditions: initialized buffer and process-lifetime string literals; readers wait for OS
 * shutdown/quiescence. Context: OS tracing actor, including its existing native critical section;
 * no blocking or BSW area entry. Reentrancy: reentrant; bounded atomic slot reservation, overflow
 * explicitly counted. Ownership: retains literal string pointers for the tool buffer lifetime;
 * captured payloads are copied.
 */
OS_MEMMAP void Arti_Record(const char *context,
                           const char *class_name,
                           const char *instance,
                           const char *event,
                           uint32_t instance_parameter,
                           uint32_t event_parameter);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

/* The standard contract requires literal tokens here. Stringification retains
 * those identities without expanding a caller's unrelated macro definitions.
 * Each numeric argument is evaluated exactly once. */
/* MR12 RULE 20.10 VIOLATION: trace emission retains class and event name construction. */
/** Record standard literal ARTI identities with captured scalar payloads.
 * @param[in] context Literal execution-context token; stringified without evaluation.
 * @param[in] class_name Literal ARTI class token; stringified without evaluation.
 * @param[in] instance Literal module instance token; stringified without evaluation.
 * @param[in] instance_parameter Scalar core/instance payload, evaluated once.
 * @param[in] event Literal standard event token; stringified without evaluation.
 * @param[in] event_parameter Captured scalar event payload, evaluated once.
 * @return No value; expands to the void Arti_Record call.
 * Preconditions: ARTI initialized and scalar payloads captured before expansion; no concurrent
 * buffer reader. Context: OS trace site, including its existing native critical section; outside
 * BSW exclusive areas. Reentrancy: reentrant recording with atomic slot reservation. Ownership:
 * retains static stringified literals and copies scalar payloads; no application object retained.
 */
#define ARTI_TRACE(context, class_name, instance, instance_parameter, event, event_parameter)      \
    Arti_Record(#context, #class_name, #instance, #event, (instance_parameter), (event_parameter))
#endif
