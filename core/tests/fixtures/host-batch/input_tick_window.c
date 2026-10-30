/* Hold the first empty drain across real native input publication and tick
 * delivery before returning NOFUNC. No automotive state
 * or OS result is fabricated; the cached NOFUNC was obtained before publication. */
#include "Os_Host.h"
#include "Os_HostIo.h"
#include "Os_Mailbox.h"

static volatile Os_Atomic32Type window;

StatusType __real_Os_TargetTakeInput(Os_InputRecordType *record);
StatusType __real_Os_TargetPostInput(const uint8 *data, uint8 length, uint64 *ticket);
void __real_vApplicationTickHook(void);

StatusType __wrap_Os_TargetTakeInput(Os_InputRecordType *record)
{
    StatusType status = __real_Os_TargetTakeInput(record);
    if ((status == E_OS_NOFUNC) && (Os_HostAtomicCompareExchange(&window, 1, 0) == 0)) {
        static const char witness[] = "WINDOW input=published tick=delivered\n";
        while (Os_HostAtomicLoad(&window) != 3) {
            /* The real producer and tick ISR finish inside this drain window. */
        }
        if (Os_HostWriteBytes(0, witness, sizeof(witness) - 1u) == 0) {
            status = E_OS_STATE;
        }
    }
    return status;
}

StatusType __wrap_Os_TargetPostInput(const uint8 *data, uint8 length, uint64 *ticket)
{
    StatusType status;
    if (Os_HostAtomicLoad(&window) < 2) {
        while (Os_HostAtomicLoad(&window) != 1) {
            /* Wait for the owner to have observed an empty mailbox. */
        }
        status = __real_Os_TargetPostInput(data, length, ticket);
        (void)Os_HostAtomicExchange(&window, 2);
    } else {
        status = __real_Os_TargetPostInput(data, length, ticket);
    }
    return status;
}

void __wrap_vApplicationTickHook(void)
{
    __real_vApplicationTickHook();
    if (Os_HostAtomicLoad(&window) == 2) {
        (void)Os_HostAtomicExchange(&window, 3);
    }
}
