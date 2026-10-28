#include "Rte_Observe.h"
#ifdef ECU_TARGET_TESTS
void PipelineTest_Record(uint8 actor);
void PipelineTest_CallContext(boolean entered);
void PipelineTest_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status);
static inline void Observe_RecordActor(void) { PipelineTest_Record(2u); }
static inline void Observe_CallContext(boolean entered) { PipelineTest_CallContext(entered); }
static inline void Observe_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status)
{
    PipelineTest_Result(value, state, local, status);
}
#else
static inline void Observe_RecordActor(void) {}
static inline void Observe_CallContext(boolean entered) { (void)entered; }
static inline void Observe_Result(uint32 value, uint32 state, uint32 local, Std_ReturnType status)
{
    (void)value;
    (void)state;
    (void)local;
    (void)status;
}
#endif
#define Observe_START_SEC_CODE
#include "Observe_MemMap.h"
Observe_CODE void Observe_Periodic(void)
{
    (void)Observe_RecordActor();
    /* The connected synchronous accesses only borrow these invocation-local outputs. */
    uint32 value = 0u;
    uint32 observed = 0u;
    uint32 service_value = 0u;
    uint32 service_state = 0u;
    (void)Rte_Read_Value_Value(&value);
    (void)Rte_Read_Result_Value(&observed);
    (void)Observe_CallContext(TRUE);
    const Std_ReturnType call_status =
        Rte_Call_ResultService_Transform(value, &service_value, &service_state);
    (void)Observe_Result(service_value, service_state, observed, call_status);
    (void)Observe_CallContext(FALSE);
}

#define Observe_STOP_SEC_CODE
#include "Observe_MemMap.h"
