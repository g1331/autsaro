#include "Os_Vector.h"
/* The fixed GCC target emits writable storage in its own PE section. The
 * native port installs handlers before delivery; this is its actual table,
 * not a parallel description of callbacks. */
Os_InterruptHandler Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT]
    __attribute__((section(".os_vec"), aligned(8))) = {0};
