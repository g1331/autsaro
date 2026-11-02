/* Actual generated startup must publish LSduR's immutable routes. */
#include "Ecu_Target.h"
#include "Os_Host.h"
#include "LSduR_Cfg.h"
#include "LSduR_CanIf.h"
#include "LSduR_PduR.h"
#include "LSduR_CanTp.h"
#include <assert.h>

static Os_HostThreadResultType observe(void *argument)
{
    uint64_t started = Os_HostMonotonicMs();
    uint16 i;
    uint8 data[8] = {0};
    PduInfoType info = {data, NULL_PTR, sizeof(data)};
    boolean found = FALSE;
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        assert((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    for (i = 0u; i < LSduR_Config.transmit_count; ++i) {
        const LSduR_TxRouteType *route = &LSduR_Config.transmit[i];
        if (route->module == LSDUR_UP_PDUR) {
            assert(LSduR_CanIfTriggerTransmit(route->lower, &info) == E_OK);
            assert(info.SduLength == 4u);
            found = TRUE;
        }
    }
    assert(found == TRUE);
    assert(LSduR_PduRTransmit((PduIdType)255u, &info) == E_NOT_OK);
    assert(LSduR_CanTpTransmit((PduIdType)255u, &info) == E_NOT_OK);
    assert(LSduR_CanIfTriggerTransmit((PduIdType)255u, &info) == E_NOT_OK);
    assert(LSduR_PduRTransmit(0u, NULL_PTR) == E_NOT_OK);
    assert(LSduR_CanTpTransmit(0u, NULL_PTR) == E_NOT_OK);
    assert(LSduR_CanIfTriggerTransmit(0u, NULL_PTR) == E_NOT_OK);
    ShutdownOS(E_OK);
    return 0u;
}

int main(void)
{
    Os_HostHandleType thread;
    assert(Ecu_TargetPrepare() == E_OK);
    thread = Os_HostSpawnThread(&observe, NULL);
    assert(thread != NULL);
    assert(Os_HostClose(thread) != 0);
    StartOS(OSDEFAULTAPPMODE);
    return 1;
}
