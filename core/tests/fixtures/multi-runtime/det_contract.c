/** Independent R24-11 DET consumer and behavioral expectations. */
#include "Det.h"
#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#if defined(_WIN32)
#include <signal.h>
#include <stdlib.h>
static void observed_abort(int signal_number) { _Exit((signal_number == SIGABRT) ? 86 : 87); }
#endif

static unsigned events[16];
static unsigned event_count;
static boolean nested;
static boolean recurse_development;

static void record(unsigned value) {
    assert(event_count < 16u);
    events[event_count] = value;
    ++event_count;
}
static Std_ReturnType first(uint16 module, uint8 instance, uint8 api, uint8 error) {
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    record(1u);
    if (nested == TRUE) {
        nested = FALSE;
        assert(Det_ReportRuntimeError(60u, 2u, 5u, 7u) == E_OK);
    }
    return E_NOT_OK;
}
static Std_ReturnType second(uint16 module, uint8 instance, uint8 api, uint8 error) {
    assert(module == 60u);
    assert(instance == 2u);
    assert(api == 5u);
    assert(error == 7u);
    record(2u);
    return E_OK;
}
static Std_ReturnType development_first(uint16 module, uint8 instance, uint8 api, uint8 error) {
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
static Std_ReturnType development_second(uint16 module, uint8 instance, uint8 api, uint8 error) {
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

int main(int argc, char **argv) {
#if defined(_WIN32)
    /* Observe this process's real abort before the CRT starts WER or a dialog. */
    (void)_set_error_mode(_OUT_TO_STDERR);
    if (signal(SIGABRT, observed_abort) == SIG_ERR) {
        return 88;
    }
#endif
    /* These expectations come from SWS_Det_00008/00009/00010/01001,
     * 00014/00018/00024/00026/00208/00501/00503, independent of inventory.
     */
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
    assert(fputs("runtime-ok\n", stdout) >= 0);
    return 0;
}
