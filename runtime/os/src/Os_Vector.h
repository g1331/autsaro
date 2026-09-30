#ifndef AUTOSAR_EPIC4_OS_VECTOR_H
#define AUTOSAR_EPIC4_OS_VECTOR_H
#include <stdint.h>
/* Private fixed Win64 port binding, including yield/tick and mailbox slots.
 * All access remains under the port's existing interrupt mutex protocol. */
#define OS_INTERRUPT_VECTOR_COUNT 32u
typedef uint32_t (*Os_InterruptHandler)(void);
extern Os_InterruptHandler Os_InterruptVectorTable[OS_INTERRUPT_VECTOR_COUNT];
#endif
