#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Rte_EchoApplication.h"
#include "Can.h"
#include "Com.h"
#include "SchM_Can.h"
#include "Os_Windows.h"
#include <stdio.h>
#include <string.h>

static Ecu_HostBatch batch;
static unsigned mode;
static volatile LONG periods;
static unsigned can_outputs;
static unsigned did_outputs;
static unsigned session_outputs;
typedef struct {
    unsigned phase;
    uint64_t at;
    uint32_t value;
    Std_ReturnType status;
    Ecu_ApplicationState snapshot;
} Observation;
static Observation observations[8];
static volatile LONG observation_count;
static unsigned observation_read;

static void require(int accepted) {
    if (accepted == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}
static void observe(const Observation *record) {
    LONG count = InterlockedCompareExchange(&observation_count, 0, 0);
    require((count >= 0) && (count < 8));
    observations[count] = *record;
    (void)InterlockedExchange(&observation_count, count + 1);
}
static void report_observations(void) {
    unsigned count = (unsigned)InterlockedCompareExchange(&observation_count, 0, 0);
    while (observation_read < count) {
        const Observation *record = &observations[observation_read];
        if (record->phase == 9u) {
            require(printf("read epoch=%llu value=%08lx status=%u\n",
                           (unsigned long long)record->at, (unsigned long)record->value,
                           (unsigned)record->status) >= 0);
        } else {
            require(printf("snapshot at=%llu value=%08lx committed=%llu read=%u write=%u\n",
                           (unsigned long long)record->at, (unsigned long)record->snapshot.value,
                           (unsigned long long)record->snapshot.epoch,
                           (unsigned)record->snapshot.read_status,
                           (unsigned)record->snapshot.write_status) >= 0);
        }
        ++observation_read;
    }
}
static void controller(Can_ControllerStateType wanted) {
    Can_ControllerStateType actual;
    require(Can_SetControllerMode(0u, wanted) == E_OK);
    Can_MainFunction_Wakeup();
    require((Can_GetControllerMode(0u, &actual) == E_OK) && (actual == wanted));
}
int Ecu_TargetTestFailStage(unsigned stage) {
    if (stage == 9u) {
        Observation record = {0};
        uint32 value = UINT32_C(0xffffffff);
        Std_ReturnType status = Rte_Read_RxValue_Value(&value);
        uint64_t at = Ecu_TargetNow();
        if (mode == 0u) {
            require((value == 0u) && (status == RTE_E_NEVER_RECEIVED));
        } else if ((mode == 1u) && (at == UINT64_C(30))) {
            require((value == UINT32_C(0x12345678)) && (status == RTE_E_MAX_AGE_EXCEEDED));
        } else if ((at == UINT64_C(30)) || ((mode == 3u) && (at == UINT64_C(20)))) {
            require((value == UINT32_C(0x0a0b0c0d)) && (status == E_OK));
        } else {
            require((value == UINT32_C(0x12345678)) && (status == E_OK));
        }
        record.phase = 9u;
        record.at = at;
        record.value = value;
        record.status = status;
        observe(&record);
        require(Rte_Read_RxValue_Value(NULL) == RTE_E_COM_STOPPED);
        require(Ecu_ApplicationInspect(NULL) == E_NOT_OK);
        if ((mode == 3u) && (at == UINT64_C(20))) {
            controller(CAN_CS_STOPPED);
        }
    } else if (stage == 10u) {
        Observation record = {0};
        Ecu_ApplicationState state;
        uint64_t at = Ecu_TargetNow();
        require(Ecu_ApplicationInspect(&state) == E_OK);
        if ((mode == 3u) && (at == UINT64_C(20))) {
            require((state.value == UINT32_C(0x12345678)) && (state.epoch == UINT64_C(10)) &&
                    (state.read_status == RTE_E_COM_STOPPED) &&
                    (state.write_status == COM_SERVICE_NOT_AVAILABLE));
            controller(CAN_CS_STARTED);
        } else {
            uint32_t expected =
                ((mode == 0u) || ((mode == 1u) && (at == UINT64_C(30))))
                    ? UINT32_C(0)
                    : ((at == UINT64_C(30)) ? UINT32_C(0x0a0b0c0d) : UINT32_C(0x12345678));
            require((state.value == expected) && (state.epoch == at) &&
                    (state.write_status == E_OK));
        }
        record.phase = 10u;
        record.at = at;
        record.snapshot = state;
        observe(&record);
        (void)InterlockedIncrement(&periods);
    } else {
        /* Startup stages use the actual initialization; no synthetic failure. */
    }
    return 0;
}
void Ecu_TargetTestShutdown(StatusType reason) {
    require(printf("application_close reason=%u\n", (unsigned)reason) >= 0);
}
static int sink(const Ecu_OutputRecord *output, const Ecu_HostBatch *state, StatusType status,
                void *context) {
    static const uint8_t zero[4] = {0u, 0u, 0u, 0u};
    static const uint8_t original_can[4] = {0x78u, 0x56u, 0x34u, 0x12u};
    static const uint8_t original_did[4] = {0x12u, 0x34u, 0x56u, 0x78u};
    static const uint8_t new_can[4] = {0x0du, 0x0cu, 0x0bu, 0x0au};
    static const uint8_t new_did[4] = {0x0au, 0x0bu, 0x0cu, 0x0du};
    const uint8_t *expected;
    (void)context;
    if (output == NULL) {
        require(status == E_OK);
        report_observations();
        return printf("commit epoch=%llu sequence=%llu\n",
                      (unsigned long long)state->completed_epoch,
                      (unsigned long long)state->sequence) >= 0 &&
               fflush(stdout) == 0;
    }
    if (output->can_id == 0x321u) {
        expected = ((mode == 0u) || ((mode == 1u) && (output->epoch == UINT64_C(30))))
                       ? zero
                       : ((output->epoch == UINT64_C(30)) ? new_can : original_can);
        require((output->dlc == 4u) && (memcmp(output->data, expected, 4u) == 0));
        ++can_outputs;
    } else {
        require((output->can_id == 0x708u) && (output->dlc == 8u));
        if (output->data[1] == 0x50u) {
            static const uint8_t session[8] = {6u, 0x50u, 3u, 0u, 0x32u, 1u, 0xf4u, 0u};
            require((mode == 4u) && (memcmp(output->data, session, 8u) == 0));
            ++session_outputs;
        } else {
            expected = ((output->epoch == UINT64_C(0)) || (mode == 0u) ||
                        ((mode == 1u) && (output->epoch == UINT64_C(30))))
                           ? zero
                           : ((output->epoch == UINT64_C(30)) ? new_did : original_did);
            require((output->data[0] == 7u) && (output->data[1] == 0x62u) &&
                    (output->data[2] == 0x12u) && (output->data[3] == 0x34u) &&
                    (memcmp(&output->data[4], expected, 4u) == 0));
            ++did_outputs;
        }
    }
    require(printf("application_output epoch=%llu id=%lu data=", (unsigned long long)output->epoch,
                   (unsigned long)output->can_id) >= 0);
    for (unsigned i = 0u; i < output->dlc; ++i) {
        require(printf("%02x", (unsigned)output->data[i]) >= 0);
    }
    return putchar('\n') != EOF && fflush(stdout) == 0;
}
static void execute(const char *commands) {
    const char *next = commands;
    while (*next != '\0') {
        const char *end = strchr(next, '\n');
        Ecu_BatchCommand command;
        require(end != NULL);
        require(Ecu_HostBatchParse(&batch, next, (size_t)(end - next), &command) == E_OK);
        if (command == ECU_BATCH_COMMIT) {
            require(Ecu_HostBatchExecute(&batch, sink, NULL) == E_OK);
        }
        next = end + 1;
    }
}
static DWORD WINAPI control(void *argument) {
    Ecu_ApplicationState untouched = {UINT32_C(0xabcdef01), UINT64_C(77), 99u, 98u};
    DWORD started = GetTickCount();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((GetTickCount() - started) < 5000u);
        Sleep(1u);
    }
    require(Ecu_ApplicationInspect(&untouched) == E_NOT_OK);
    require((untouched.value == UINT32_C(0xabcdef01)) && (untouched.epoch == UINT64_C(77)) &&
            (untouched.read_status == 99u) && (untouched.write_status == 98u));
    Ecu_HostBatchInitialize(&batch);
    if (mode == 4u) {
        execute("BEGIN 0\nRX 1792 8 0210030000000000\nCOMMIT\n");
    }
    execute("BEGIN 0\nRX 1792 8 0322123400000000\nCOMMIT\n");
    if (mode == 0u) {
        execute("BEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\n");
        require((can_outputs == 1u) && (did_outputs == 2u));
        require(InterlockedCompareExchange(&periods, 0, 0) == 1);
    } else {
        execute("BEGIN 0\nRX 800 4 78563412\nCOMMIT\n");
        execute("BEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\n");
        if (mode == 1u) {
            execute("BEGIN 30\nRX 1792 8 0322123400000000\nCOMMIT\n");
        } else if ((mode == 2u) || (mode == 3u)) {
            if (mode == 3u) {
                execute("BEGIN 20\nRX 800 4 0d0c0b0a\nRX 1792 8 0322123400000000\nCOMMIT\n");
            }
            execute("BEGIN 30\nRX 800 4 0d0c0b0a\nRX 1792 8 0322123400000000\nCOMMIT\n");
            execute("BEGIN 30\nRX 1792 8 0322123400000000\nCOMMIT\n");
        }
        require(can_outputs == ((mode == 4u) ? 1u : 3u));
        require(did_outputs ==
                ((mode == 1u) ? 3u : ((mode == 4u) ? 2u : ((mode == 3u) ? 5u : 4u))));
        require(InterlockedCompareExchange(&periods, 0, 0) == ((mode == 4u) ? 1 : 3));
    }
    require(session_outputs == ((mode == 4u) ? 1u : 0u));
    require(printf("application_loop mode=%u status_epoch_value_can_did=pass\n", mode) >= 0);
    ShutdownOS(E_OK);
    return 0u;
}
int main(int argc, char **argv) {
    HANDLE thread;
    if (argc != 2) {
        return 98;
    }
    if (strcmp(argv[1], "initial") == 0) {
        mode = 0u;
    } else if (strcmp(argv[1], "stale") == 0) {
        mode = 1u;
    } else if (strcmp(argv[1], "deadline") == 0) {
        mode = 2u;
    } else if (strcmp(argv[1], "write_fail") == 0) {
        mode = 3u;
    } else if (strcmp(argv[1], "extended") == 0) {
        mode = 4u;
    } else {
        return 98;
    }
    require(Ecu_TargetPrepare() == E_OK);
    thread = CreateThread(NULL, 262144u, control, NULL, 0u, NULL);
    require(thread != NULL);
    require(CloseHandle(thread) != 0);
    StartOS(1u);
    return 94;
}
