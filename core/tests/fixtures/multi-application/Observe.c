#include "Rte_Observe.h"
#ifdef ECU_TARGET_TESTS
void MultiTest_Record(uint8 actor);
void MultiTest_CallContext(boolean entered);
void MultiTest_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status);
#endif
static uint32 observed;
static uint32 service_value;
static uint32 service_state;
void Observe_Periodic(void) {
#ifdef ECU_TARGET_TESTS
    MultiTest_Record(2u);
#endif
    uint32 value = 0u;
    (void)Rte_Read_Value_Value(&value);
    (void)Rte_Read_Result_Value(&observed);
#ifdef ECU_TARGET_TESTS
    MultiTest_CallContext(TRUE);
#endif
    const Std_ReturnType call_status =
        Rte_Call_ResultService_Transform(value, &service_value, &service_state);
#ifndef ECU_TARGET_TESTS
    (void)call_status;
#endif
#ifdef ECU_TARGET_TESTS
    MultiTest_Result(service_value, service_state, observed, call_status);
    MultiTest_CallContext(FALSE);
#endif
}
