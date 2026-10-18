/* Keep the first response CF busy through its N_Cs deadline. The normal PduR
 * configuration and Dcm callback remain in use for all other data and routes. */

#include "PduR_Cfg.h"
#include "Dcm.h"
static PduR_PBConfigType timer_router;
static PduR_TpRouteType timer_route;
static BufReq_ReturnType timer_copy(PduIdType id,
                                    const PduInfoType *info,
                                    const RetryInfoType *retry,
                                    PduLengthType *available)
{
    const uint64_t now = Ecu_TargetNow();
    if ((now >= 11u) && (now < 212u)) {
        return BUFREQ_E_BUSY;
    }
    return Dcm_CopyTxData(id, info, retry, available);
}
int Ecu_TargetTestFailStage(unsigned stage)
{
    if (stage == 5u) {
        timer_router = PduR_Config;
        timer_route = PduR_Config.transport[0];
        timer_route.copy_tx = timer_copy;
        timer_router.transport = &timer_route;
        PduR_Init(&timer_router);
    }
    return 0;
}
void Ecu_TargetTestRetire(unsigned stage) { (void)stage; }
void Ecu_TargetTestShutdown(StatusType reason) { (void)reason; }
