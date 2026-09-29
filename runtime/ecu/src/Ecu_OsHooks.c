#include "Os_Target.h"

/* Standard configured callbacks are bounded and do not perform host I/O.
 * Detailed ARTI object/state binding is delivered by story 4.19. */
void ErrorHook(StatusType Error) {
    (void)Error;
    Os_TargetTrace('e');
}
void PreTaskHook(void) { Os_TargetTrace('p'); }
void PostTaskHook(void) { Os_TargetTrace('q'); }
