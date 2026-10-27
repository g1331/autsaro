#include "Rte_Process.h"
#ifdef ECU_TARGET_TESTS
void PipelineTest_Record(uint8 actor);
static inline void Process_RecordActor(uint8 actor) { PipelineTest_Record(actor); }
#else
static inline void Process_RecordActor(uint8 actor) { (void)actor; }
#endif
#define Process_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Process_MemMap.h"
static uint32 committed Process_VAR_CLEARED_UNSPECIFIED;
#define Process_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Process_MemMap.h"
#define Process_START_SEC_CODE
#include "Process_MemMap.h"
Process_CODE void Process_Periodic(void)
{
    (void)Process_RecordActor(1u);
    uint32 value = 0u;
    if (Rte_Read_Value_Value(&value) == E_OK) {
        committed = value + 1u;
        (void)Rte_Write_Result_Value(committed);
    }
}
Process_CODE void Process_Transform(uint32 Input, uint32 *Output, uint32 *State)
{
    (void)Process_RecordActor(3u);
    (void)Input;
    *Output = committed;
    *State = committed;
}

#define Process_STOP_SEC_CODE
#include "Process_MemMap.h"
