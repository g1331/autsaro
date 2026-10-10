#include "Rte_Ingress.h"
#ifdef ECU_TARGET_TESTS
#include "Ecu_Target.h"
static uint32 test_counts[10];
static uint8 test_next_actor;
static boolean test_observe_call;
void MultiTest_Record(uint8 actor);
void MultiTest_CallContext(boolean entered);
void MultiTest_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status);
void MultiTest_Copy(uint32 result[10]);
void MultiTest_Record(uint8 actor) {
    if ((actor >= 4u) || (Ecu_TargetIsOwner() == 0)) {
        ++test_counts[5];
    } else {
        ++test_counts[actor];
        if (test_next_actor != actor) {
            ++test_counts[4];
        }
        test_next_actor = (uint8)((actor + 1u) % 4u);
        if ((actor == 3u) && (test_observe_call == FALSE)) {
            ++test_counts[5];
        }
    }
}
void MultiTest_CallContext(boolean entered) { test_observe_call = entered; }
void MultiTest_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status) {
    test_counts[6] = value;
    test_counts[7] = state;
    test_counts[8] = local;
    test_counts[9] = status;
}
void MultiTest_Copy(uint32 result[10]) {
    uint8 index;
    for (index = 0u; index < 10u; ++index) {
        result[index] = test_counts[index];
    }
}
#endif
static uint32 committed;
void Ingress_Periodic(void) {
#ifdef ECU_TARGET_TESTS
    MultiTest_Record(0u);
#endif
    uint32 value = committed;
    const Std_ReturnType status = Rte_Read_RxValue_Value(&value);
    if ((status == E_OK) || (status == RTE_E_MAX_AGE_EXCEEDED)) {
        committed = value;
        (void)Rte_Write_TxValue_Value(value);
        (void)Rte_Write_Value_Value(value);
    }
}
void Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data) {
    Data[0] = (uint8)(committed >> 24u);
    Data[1] = (uint8)(committed >> 16u);
    Data[2] = (uint8)(committed >> 8u);
    Data[3] = (uint8)committed;
}
