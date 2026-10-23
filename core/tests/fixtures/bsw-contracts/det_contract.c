/** Independent R24-11 DET consumer and behavioral expectations. */
#include "Det.h"
#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

static unsigned events[16];
static unsigned event_count;
static boolean nested;
static boolean recurse_development;
static boolean reinitialize_runtime;

static void record(unsigned value)
{
    assert(event_count < 16u);
    events[event_count] = value;
    ++event_count;
}
static Std_ReturnType first(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    record(1u);
    if (reinitialize_runtime == TRUE) {
        reinitialize_runtime = FALSE;
        Det_Init(NULL_PTR);
    }
    if (nested == TRUE) {
        nested = FALSE;
        assert(Det_ReportRuntimeError(60u, 2u, 5u, 7u) == E_OK);
    }
    return E_NOT_OK;
}
static Std_ReturnType second(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    record(2u);
    return E_OK;
}
static Std_ReturnType development_first(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    assert(fputs("development-first\n", stdout) >= 0);
    assert(fflush(stdout) == 0);
    if (recurse_development == TRUE) {
        (void)Det_ReportError(60u, 2u, 5u, 7u);
    }
    return E_NOT_OK;
}
static Std_ReturnType development_second(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    assert(fputs("development-second\n", stdout) >= 0);
    assert(fflush(stdout) == 0);
    return E_OK;
}
static const Det_ErrorHookType runtime_hooks[] = {first, second};
static const Det_ErrorHookType development_hooks[] = {development_first, development_second};
static const Det_ConfigType config = {development_hooks, 2u, runtime_hooks, 2u};

int main(int argc, char **argv)
{
    /* R24-11 BSWGeneral Appendix A.2 and the module API tables, independent of runtime. */
    assert(DET_MODULE_ID == 15u);
    /* Verifies: SWS_Det_00008, SWS_Det_00009, SWS_Det_00010, SWS_Det_01001 */
    /* Verifies: SWS_Det_00014, SWS_Det_00018, SWS_Det_00019, SWS_Det_00020 */
    /* Verifies: SWS_Det_00024, SWS_Det_00025, SWS_Det_00026, SWS_Det_00035 */
    /* Verifies: SWS_Det_00207, SWS_Det_00208, SWS_Det_00501, SWS_Det_00503 */
    void (*initialize)(const Det_ConfigType *) = Det_Init;
    void (*start)(void) = Det_Start;
    Std_ReturnType (*runtime_report)(uint16, uint8, uint8, uint8) = Det_ReportRuntimeError;
    Std_ReturnType (*development_report)(uint16, uint8, uint8, uint8) = Det_ReportError;
    assert(argc == 2);
    assert(NULL_PTR == NULL);
    assert(sizeof(uint8) == 1u && sizeof(uint16) == 2u && sizeof(uint32) == 4u);
    assert(CPU_TYPE == CPU_TYPE_64 && CPU_BIT_ORDER == LSB_FIRST);
    assert(CPU_BYTE_ORDER == LOW_BYTE_FIRST);
    assert(TRUE == true && FALSE == false);
    start();
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 0u);
    if (strcmp(argv[1], "before-init") == 0) {
        (void)development_report(60u, 2u, 5u, 7u);
        return 90;
    }
    if (strcmp(argv[1], "invalid-config") == 0) {
        const Det_ConfigType invalid = {NULL, 1u, NULL, 0u};
        initialize(&invalid);
        return 91;
    }
    if (strcmp(argv[1], "invalid-runtime-config") == 0) {
        const Det_ConfigType invalid = {NULL, 0u, NULL, 1u};
        initialize(&invalid);
        return 91;
    }
    if (strcmp(argv[1], "invalid-hook-entry") == 0) {
        const Det_ErrorHookType invalid_hooks[] = {NULL_PTR};
        const Det_ConfigType invalid = {invalid_hooks, 1u, NULL, 0u};
        initialize(&invalid);
        return 91;
    }
    if (strcmp(argv[1], "development-without-hooks") == 0) {
        initialize(NULL_PTR);
        (void)development_report(60u, 2u, 5u, 7u);
        return 92;
    }
    initialize(&config);
    if (strcmp(argv[1], "development") == 0) {
        (void)development_report(60u, 2u, 5u, 7u);
        return 92;
    }
    if (strcmp(argv[1], "recursive-development") == 0) {
        recurse_development = TRUE;
        (void)development_report(60u, 2u, 5u, 7u);
        return 93;
    }
    assert(strcmp(argv[1], "runtime") == 0);
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 2u);
    assert(events[0] == 1u && events[1] == 2u);
    nested = TRUE;
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 6u);
    assert(events[2] == 1u && events[3] == 1u);
    assert(events[4] == 2u && events[5] == 2u);
    initialize(NULL_PTR);
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 6u);
    initialize(&config);
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 8u);
    assert(events[6] == 1u && events[7] == 2u);
    reinitialize_runtime = TRUE;
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 10u && events[8] == 1u && events[9] == 2u);
    assert(runtime_report(60u, 2u, 5u, 7u) == E_OK);
    assert(event_count == 10u);
    assert(fputs("runtime-ok\n", stdout) >= 0);
    return 0;
}
