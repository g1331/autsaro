/** @file Handle-bearing COM reception and timeout notifications. */
#ifndef RTE_COM_H
#define RTE_COM_H
#include "Rte_Version.h"
#include "Rte_Type.h"
#include "ComStack_Types.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION) ||                                             \
    !defined(COMSTACKTYPE_AR_RELEASE_MINOR_VERSION)
#error "Rte_Com.h -> ComStack_Types.h: missing AUTOSAR release"
#elif (COMSTACKTYPE_AR_RELEASE_MAJOR_VERSION != 4U) ||                                             \
    (COMSTACKTYPE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Rte_Com.h -> ComStack_Types.h: incompatible AUTOSAR release"
#endif
#ifdef __cplusplus
extern "C" {
#endif
/** Refresh receiver values after a successful COM reception.
 * @param[in] handle Configured ComUserCbkHandleId; unknown handles are ignored.
 * @par Return
 * None.
 *
 * Preconditions: RTE started and ECU owner context; otherwise no action.
 *
 * Context: COM callback on the ECU owner task, outside exclusive areas.
 *
 * Reentrancy: not reentrant for the same ECU receiver state.
 *
 * Ownership: No pointer is retained; receiver storage remains owned by the RTE.
 */
void Rte_COMCbk(CbkHandleIdType handle);
/** Mark previously received values as expired after a COM Rx deadline.
 * @param[in] handle Configured ComUserCbkHandleId; unknown handles are ignored.
 * @par Return
 * None.
 *
 * Preconditions: RTE started and ECU owner context; otherwise no action.
 *
 * Context: COM callback on the ECU owner task, outside exclusive areas.
 *
 * Reentrancy: not reentrant for the same ECU receiver state.
 *
 * Ownership: No pointer is retained; receiver storage remains owned by the RTE.
 */
void Rte_COMCbkRxTOut(CbkHandleIdType handle);
#ifdef __cplusplus
}
#endif
#endif
