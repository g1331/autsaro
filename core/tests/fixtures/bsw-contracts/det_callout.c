/** Independent consumer of the actual generated Det notification and public headers. */
#include "Det.h"
#include "Det_Externals.h"
#include "Ecu_Target.h"
#include <assert.h>

static unsigned timeouts;
void Ecu_TargetReportTransportTimeout(void) { ++timeouts; }

int main(void)
{
    /* Verifies: SWS_Det_00035, SWS_Det_00180, SWS_Det_00184 */
    /* Verifies[Det]: SWS_BSW_00135, SWS_BSW_00136, SWS_BSW_00254 */
    static const Det_ErrorHookType hooks[] = {Ecu_DetRuntimeCallout};
    static const Det_ConfigType config = {NULL_PTR, 0u, hooks, 1u};
    assert(Ecu_DetRuntimeCallout(0u, 0u, 0u, 0u) == E_OK);
    assert(timeouts == 0u);
    Det_Init(&config);
    assert(Det_ReportRuntimeError(35u, 0u, 0x06u, 0xc0u) == E_OK);
    assert(timeouts == 1u);
    assert(Det_ReportRuntimeError(35u, 0u, 0x06u, 0xd0u) == E_OK);
    assert(timeouts == 2u);
    assert(Det_ReportRuntimeError(34u, 0u, 0x06u, 0xc0u) == E_OK);
    assert(Det_ReportRuntimeError(35u, 0u, 0x05u, 0xc0u) == E_OK);
    assert(Det_ReportRuntimeError(35u, 0u, 0x06u, 0xc1u) == E_OK);
    assert(timeouts == 2u);
    return 0;
}
