/** @file Interrupt vector table of the native port, placed in its own data section. */
#include "Os_Vector.h"
/* The fixed GCC target emits writable storage in its own PE section. The
 * native port installs handlers before delivery; this is its actual table,
 * not a parallel description of callbacks. */
Os_InterruptHandler Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT] OS_VECTOR_TABLE_SECTION = {
    0}; /**< Port-synchronized vector bindings; no Init reset. */
