#include "Os_Arti.h"
#include "Os_Backend.h"
#define main control_main
#include "ecu_control.c"
#undef main
#include "Arti_ExpressionProbe.h"

static const char *expected_os;
static unsigned observations;
static uintptr_t alarm_base_address;
int Ecu_TargetTestFailStage(unsigned stage) {
    if (stage == 9u) {
        TickType counter = UINT64_MAX;
        Os_ActivationInfo actual;
        const CONTEXT *context = Os_ArtiTaskContexts[0];
        ++observations;
        require(*Os_ArtiOsReady != 0 && *Os_ArtiAppMode == 1u);
        require(GetCounterValue(0u, &counter) == E_OK && counter == observations * UINT64_C(10));
        require(Os_ArtiCounters[0] == counter);
        require(Os_TargetInspectActivation(0u, &actual) == E_OK && actual.state == RUNNING);
        require(Os_ArtiTasks[0].state == RUNNING &&
                Os_ArtiTasks[0].priority == actual.effective_priority);
        require(Os_ArtiTasks[0].activations == actual.count && actual.count == 1u);
        require(Os_ArtiTasks[0].events == actual.events);
        require(Os_ArtiTaskStacks[0] != NULL && Os_ArtiTaskStacks[0]->role == 'T');
        require(Os_ArtiStacks[0].role == 'S' &&
                Os_ArtiStacks[0].sp >= Os_ArtiStacks[0].reserve_low);
        require(Os_ArtiStacks[0].sp < Os_ArtiStacks[0].high);
        require(Os_ArtiTaskStacks[0]->context_valid == 1u && context != NULL);
        require(context->Rsp >= Os_ArtiTaskStacks[0]->reserve_low &&
                context->Rsp < Os_ArtiTaskStacks[0]->high);
        require(context->Rip != 0u && Os_ArtiNativeContextSize == sizeof(CONTEXT));
        if (Ecu_OsConfig.time->alarm_count != 0u) {
            static AlarmBaseType base;
            require(Ecu_OsConfig.time->alarm_count == 2u);
            require(Os_ArtiAlarms[0].active == 1u && Os_ArtiAlarms[0].cycle == 10u &&
                    Os_ArtiAlarms[0].remaining == 10u);
            require(Os_ArtiAlarms[1].active == 1u && Os_ArtiAlarms[1].cycle == 1u &&
                    Os_ArtiAlarms[1].remaining == 1u);
            require(GetAlarmBase(0u, &base) == E_OK && base.maxallowedvalue == 65535u &&
                    base.ticksperbase == 1u && base.mincycle == 1u);
            alarm_base_address = (uintptr_t)&base;
        } else {
            ScheduleTableStatusType status = 99u;
            require(Ecu_OsConfig.time->schedule_table_count == 2u);
            require(GetScheduleTableStatus(0u, &status) == E_OK && status == SCHEDULETABLE_RUNNING);
            require(Os_ArtiScheduleTables[0].status == SCHEDULETABLE_RUNNING &&
                    Os_ArtiScheduleTables[0].remaining == 10u);
            require(Os_ArtiScheduleTables[1].status == SCHEDULETABLE_RUNNING &&
                    Os_ArtiScheduleTables[1].remaining == 1u);
        }
        if (Ecu_OsConfig.resource_count != 0u) {
            require(GetResource(RES_SCHEDULER) == E_OK && Os_ArtiResourceOwners[0] == 1u);
            require(ReleaseResource(RES_SCHEDULER) == E_OK && Os_ArtiResourceOwners[0] == 0u);
        }
        counter = UINT64_MAX;
        require(GetCounterValue(UINT32_MAX, &counter) == E_OS_ID && counter == UINT64_MAX);
        arti_expression_probe();
    }
    return 0;
}
static void arti_require_at(int accepted, unsigned line) {
    if (accepted == 0) {
        fprintf(stderr, "ARTI consumer assertion failed after actor shutdown: line=%u\n", line);
        ExitProcess(7u);
    }
}
#define arti_require(accepted) arti_require_at((accepted), __LINE__)
void Ecu_TargetTestShutdown(StatusType reason) {
    unsigned start = 0u, waits = 0u, releases = 0u, errors = 0u;
    unsigned getters = 0u, rejected = 0u, alarm_addresses = 0u, isr_start = 0u, isr_stop = 0u;
    arti_require(reason == E_OK && observations == 2u && *Os_ArtiOsReady == 0);
    arti_require(Arti_EventCount > 0 && Arti_EventCount <= (long)ARTI_EVENT_CAPACITY &&
                 Arti_EventsDropped == 0);
    for (long i = 0L; i < Arti_EventCount; ++i) {
        const Arti_Event *event = &Arti_Events[i];
        arti_require(event->published == 1L);
        arti_require(strcmp(event->instance, expected_os) == 0 && event->instance_parameter == 0u);
        if (strcmp(event->event, "OsTask_Start") == 0) {
            arti_require(event->event_parameter == 0u);
            ++start;
        } else if (strcmp(event->event, "OsTask_Wait") == 0) {
            ++waits;
        } else if (strcmp(event->event, "OsTask_Release") == 0) {
            ++releases;
        } else if (strcmp(event->event, "OsHook_ErrorHook_Start") == 0) {
            arti_require(event->event_parameter == 3u);
            ++errors;
        } else if (strcmp(event->event, "OsServiceCall_GetCounterValue_Return") == 0) {
            if (event->event_parameter == UINT32_MAX) {
                arti_require(event->status_valid == 1u && event->service_status == 3u);
                ++rejected;
            } else {
                arti_require(event->event_parameter == 10u || event->event_parameter == 20u);
                ++getters;
            }
        } else if (strcmp(event->event, "OsServiceCall_GetAlarmBase_Return") == 0) {
            if (event->address_valid != 1u || event->event_address != alarm_base_address) {
                fprintf(
                    stderr,
                    "ARTI pointer record index=%ld valid=%u actual=%llu expected=%llu status=%u\n",
                    i, event->address_valid, (unsigned long long)event->event_address,
                    (unsigned long long)alarm_base_address, event->service_status);
            }
            arti_require(event->address_valid == 1u && event->event_address == alarm_base_address);
            arti_require(event->event_parameter == (uint32_t)alarm_base_address);
            ++alarm_addresses;
        } else if (strcmp(event->class_name, "AR_CP_OS_CAT2ISR") == 0) {
            arti_require(event->event_parameter == 30u);
            if (strcmp(event->event, "OsCat2Isr_Start") == 0) {
                ++isr_start;
            } else {
                arti_require(strcmp(event->event, "OsCat2Isr_Stop") == 0);
                ++isr_stop;
            }
        }
        /* SetEvent is used only by this ECU's internal Alarm/Table/mailbox
         * actions, so those calls must not masquerade as application services. */
        arti_require(strcmp(event->event, "OsServiceCall_SetEvent_Start") != 0);
    }
    arti_require(start > 20u && waits >= 20u && releases >= 20u);
    arti_require(errors == 2u && rejected == 2u && getters == 2u);
    arti_require(isr_start > 0u && isr_start == isr_stop);
    arti_require(alarm_addresses == (Ecu_OsConfig.time->alarm_count != 0u ? 2u : 0u));
    printf("arti_consumer os=%s observations=2 getters=2 errors=2 internal_services=0 dropped=0\n",
           expected_os);
}
int main(int argc, char **argv) {
    if (argc != 2) {
        return 7;
    }
    expected_os = argv[1];
    return control_main();
}
