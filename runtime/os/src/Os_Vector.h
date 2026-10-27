/** @file Private interrupt vector table of the native port, incl. yield and tick slots. */
#ifndef AUTOSAR_OS_VECTOR_H
#define AUTOSAR_OS_VECTOR_H
#include "Os_Version.h"
#include "Os_PortTypes.h"
/* Private binding of the patched Win64 and POSIX ports, incl. yield/tick and mailbox slots.
 * All access remains under the port's existing interrupt mutex protocol. */
#define OS_INTERRUPT_VECTOR_COUNT 32u
#define OS_START_SEC_VAR_INIT_VECTOR_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern Os_InterruptHandlerType Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT];
#define OS_STOP_SEC_VAR_INIT_VECTOR_PTR
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Vector.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Vector.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
