/** @file Private interrupt vector table of the native port, incl. yield and tick slots. */
#ifndef AUTOSAR_OS_VECTOR_H
#define AUTOSAR_OS_VECTOR_H
#include "Platform_Types.h"
/* Private binding of the patched Win64 and POSIX ports, incl. yield/tick and mailbox slots.
 * All access remains under the port's existing interrupt mutex protocol. */
#define OS_INTERRUPT_VECTOR_COUNT 32u
/* The fixed GCC targets place the table in its own writable section. */
#define OS_VECTOR_TABLE_SECTION __attribute__((section(".os_vec"), aligned(8)))
typedef uint32 (*Os_InterruptHandlerType)(void);
extern Os_InterruptHandlerType Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT];
#endif
