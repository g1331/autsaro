#include "Rte_Process.h"
#ifdef ECU_TARGET_TESTS
void MultiTest_Record(uint8 actor);
#endif
static uint32 committed;
void Process_Periodic(void) {
#ifdef ECU_TARGET_TESTS
    MultiTest_Record(1u);
#endif
    uint32 value = 0u;
    if (Rte_Read_Value_Value(&value) == E_OK) {
        committed = value + 1u;
        (void)Rte_Write_Result_Value(committed);
    }
}
void Process_Transform(uint32 Input, uint32 *Output, uint32 *State) {
#ifdef ECU_TARGET_TESTS
    MultiTest_Record(3u);
#endif
    (void)Input;
    *Output = committed;
    *State = committed;
}
