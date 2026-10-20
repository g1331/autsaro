/** @file Private interrupt vector table of the native port, incl. yield and tick slots. */
#ifndef AUTOSAR_OS_VECTOR_H
#define AUTOSAR_OS_VECTOR_H
#include "Platform_Types.h"
/* Private binding of the patched Win64 and POSIX ports, incl. yield/tick and mailbox slots.
 * All access remains under the port's existing interrupt mutex protocol. */
#define OS_INTERRUPT_VECTOR_COUNT 32u
typedef uint32 (*Os_InterruptHandlerType)(void);
#define OS_START_SEC_VAR_INIT_VECTOR_PTR
#include "Os_MemMap.h"
OS_MEMMAP extern Os_InterruptHandlerType Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT];
#define OS_STOP_SEC_VAR_INIT_VECTOR_PTR
#include "Os_MemMap.h"
#endif
