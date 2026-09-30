/* Real generated owner calls this test seam before its periodic application. */
#define main independent_main
#include "ecu_control.c"
#undef main

static unsigned expected_enabled, scheduler_calls;

int Ecu_TargetTestFailStage(unsigned stage) {
    if (stage == 9u) {
        Os_ActivationInfo info;
        StatusType status = GetResource(RES_SCHEDULER);
        require(status == ((expected_enabled != 0u) ? E_OK : E_OS_ID));
        require(Os_TargetInspectActivation(ECU_TARGET_TASK, &info) == E_OK);
        require(info.resource_count == expected_enabled);
        require(info.effective_priority == 1u);
        status = ReleaseResource(RES_SCHEDULER);
        require(status == ((expected_enabled != 0u) ? E_OK : E_OS_ID));
        require(Os_TargetInspectActivation(ECU_TARGET_TASK, &info) == E_OK);
        require(info.resource_count == 0u && info.effective_priority == 1u);
        ++scheduler_calls;
    }
    return 0;
}
void Ecu_TargetTestShutdown(StatusType reason) {
    printf("scheduler_consumer enabled=%u calls=%u reason=%u\n", expected_enabled, scheduler_calls,
           reason);
    if (reason == E_OK && scheduler_calls != 2u) {
        ExitProcess(E_OS_STATE);
    }
}
int main(int argc, char **argv) {
    if (argc != 2) {
        return 98;
    }
    expected_enabled = (strcmp(argv[1], "true") == 0) ? 1u : 0u;
    if (Ecu_OsConfig.resource_count != expected_enabled) {
        return 97;
    }
    if (expected_enabled != 0u) {
        const Os_ResourceConfig *resource = Ecu_OsConfig.resources;
        if (resource == NULL || resource->id != RES_SCHEDULER || resource->ceiling != 1u ||
            resource->task_access != 1u || resource->isr_access != 0u) {
            return 96;
        }
        for (unsigned i = 0u; i < 3u; ++i) {
            Os_ResourceConfig wrong = *resource;
            Os_TargetConfig config = Ecu_OsConfig;
            config.resources = &wrong;
            if (i == 0u) {
                wrong.ceiling = 0u;
            } else if (i == 1u) {
                wrong.task_access = 0u;
            } else {
                wrong.isr_access = UINT32_C(4);
            }
            if (Os_TargetPrepare(&config) != E_OS_VALUE) {
                return 95;
            }
        }
    } else if (Ecu_OsConfig.resources != NULL) {
        return 94;
    }
    return independent_main();
}
