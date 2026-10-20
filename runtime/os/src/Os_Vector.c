/** @file Interrupt vector table of the native port, placed in its own data section. */
#include "Os_Vector.h"

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/* The fixed GCC target emits writable storage in its own PE section. The
 * native port installs handlers before delivery; this is its actual table,
 * not a parallel description of callbacks. */
Os_InterruptHandlerType Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT] OS_VECTOR_TABLE_SECTION =
    {0}; /**< Port-synchronized vector bindings; no Init reset. */

#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
