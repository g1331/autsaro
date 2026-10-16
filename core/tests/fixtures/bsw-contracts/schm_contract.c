/** 生成的 SchM 生命周期：只有生命周期阶段为测试适配，SchM_Main.c 为真实 producer。
 * Init 每进程只接受一次且须为本分区配置；Start 须在 Init 之后，StartTiming 须在
 * Start 之后；Deinit 只在关闭阶段生效，之后不再启动计时实体，也不能重新初始化。
 * Verifies: SWS_Rte_07574、07586、07270、04546、04549、07274
 */
#include "Rte_Main.h"
#include "Ecu_Target.h"
#include <assert.h>

/* 0: neither hook; 1: StartupHook; 2: ShutdownHook. */
static int phase;

Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    return (((starting == TRUE) && (phase == 1)) || ((starting == FALSE) && (phase == 2)))
               ? E_OK
               : E_NOT_OK;
}

int main(void)
{
    const SchM_ConfigType other = {Ecu_SchMConfig.partition + 1u};
    /* Outside StartupHook nothing is accepted and Init is not consumed. */
    SchM_Init(&Ecu_SchMConfig);
    SchM_Start();
    assert(Ecu_SchMReady() == FALSE);
    /* Shutdown before the first valid Init must not consume the allocation. */
    phase = 2;
    SchM_Deinit();
    SchM_Deinit();
    assert((Ecu_SchMReady() == FALSE) && (Ecu_SchMTimingActive() == FALSE));
    phase = 1;
    /* Order: Start needs Init, StartTiming needs Start. */
    SchM_StartTiming();
    SchM_Start();
    assert((Ecu_SchMReady() == FALSE) && (Ecu_SchMTimingActive() == FALSE));
    SchM_Init(NULL_PTR);
    SchM_Init(&other);
    SchM_Start();
    assert(Ecu_SchMReady() == FALSE);
    SchM_Init(&Ecu_SchMConfig);
    SchM_StartTiming();
    assert(Ecu_SchMTimingActive() == FALSE);
    SchM_Start();
    assert((Ecu_SchMReady() == TRUE) && (Ecu_SchMTimingActive() == FALSE));
    SchM_StartTiming();
    assert(Ecu_SchMTimingActive() == TRUE);
    /* Repeated startup calls preserve the active timing phase. */
    SchM_Init(&Ecu_SchMConfig);
    SchM_Start();
    SchM_StartTiming();
    assert((Ecu_SchMReady() == TRUE) && (Ecu_SchMTimingActive() == TRUE));
    /* Deinit is a shutdown service only. */
    SchM_Deinit();
    assert(Ecu_SchMTimingActive() == TRUE);
    phase = 2;
    SchM_Deinit();
    assert((Ecu_SchMReady() == FALSE) && (Ecu_SchMTimingActive() == FALSE));
    /* Init, Start and StartTiming are called once per process. */
    phase = 1;
    SchM_Init(&Ecu_SchMConfig);
    SchM_Start();
    SchM_StartTiming();
    assert((Ecu_SchMReady() == FALSE) && (Ecu_SchMTimingActive() == FALSE));
    return 0;
}
