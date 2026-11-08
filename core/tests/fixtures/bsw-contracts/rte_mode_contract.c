/** Independent consumer of synchronized BSW/SWC modes. */
#define RTE_CORE
#include "Rte_Ingress.h"
#include "Rte_Observe.h"
#include "Rte_Process.h"
#include "SchM_ComM.h"
#include "SchM_Dcm.h"
#include "Rte_Main.h"
#include "Ecu_Target.h"
#include "Com.h"
#define CHECK(condition)                                                                           \
    do {                                                                                           \
        if (!(condition)) {                                                                        \
            return __LINE__;                                                                       \
        }                                                                                          \
    } while (0)

/* Context adapter and unrelated COM dependencies for the direct RTE consumer. */
int Ecu_TargetIsOwner(void) { return 1; }
uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_NOT_OK;
}
uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr)
{
    (void)SignalId;
    (void)SignalDataPtr;
    return E_NOT_OK;
}

Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    return starting == TRUE ? E_OK : E_NOT_OK;
}

#if defined(SWC_MODE_PUBLISHER)
#define PUBLISH_PRIMARY   Rte_Switch_Process_Primary_Mode
#define PUBLISH_SECONDARY Rte_Switch_Process_Secondary_Mode
#else
#define PUBLISH_PRIMARY   SchM_Switch_ComM_Primary
#define PUBLISH_SECONDARY SchM_Switch_ComM_Secondary
#endif

int main(void)
{
    SchM_Init(&Ecu_SchMConfig);
    CHECK(Rte_Mode_Ingress_First_Mode() == 3u);
    CHECK(Rte_Mode_Observe_First_Mode() == 3u);
    CHECK(Rte_Mode_Observe_Second_Mode() == 3u);
    CHECK(PUBLISH_PRIMARY(7u) == SCHM_E_OK);
    CHECK(Rte_Mode_Ingress_First_Mode() == 7u);
    CHECK(Rte_Mode_Observe_First_Mode() == 7u);
    CHECK(Rte_Mode_Observe_Second_Mode() == 3u);
    CHECK(PUBLISH_SECONDARY(7u) == SCHM_E_OK);
    CHECK(PUBLISH_PRIMARY(3u) == SCHM_E_OK);
    CHECK(Rte_Mode_Ingress_First_Mode() == 3u);
    CHECK(Rte_Mode_Observe_First_Mode() == 3u);
    CHECK(Rte_Mode_Observe_Second_Mode() == 7u);
    CHECK(SchM_Mode_Dcm_DcmDiagnosticSessionControl() == 0u);
    CHECK(Rte_Mode_Ingress_Session_diagnosticSession() == 0u);
    CHECK(SchM_Switch_Dcm_DcmDiagnosticSessionControl(1u) == SCHM_E_OK);
    CHECK(SchM_Mode_Dcm_DcmDiagnosticSessionControl() == 1u);
    CHECK(Rte_Mode_Ingress_Session_diagnosticSession() == 1u);
    CHECK(Rte_Mode_Observe_Second_Mode() == 7u);
    return 0;
}
