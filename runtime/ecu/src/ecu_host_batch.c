#include "Ecu_Target.h"
#include "Os_Windows.h"
#include <fcntl.h>
#include <io.h>
#include <stdio.h>

static Ecu_HostBatch batch;

static int write_output(const Ecu_OutputRecord *output, const Ecu_HostBatch *state,
                        StatusType status, void *context) {
    (void)context;
    if (output == NULL) {
        if (printf("%s batch=%llu epoch=%llu sequence=%llu input_first=%llu inputs=%u status=%u "
                   "input_status=%u\n",
                   (status == E_OK) ? "COMMIT_OK" : "COMMIT_ERROR",
                   (unsigned long long)state->batch_id, (unsigned long long)state->completed_epoch,
                   (unsigned long long)state->sequence,
                   (unsigned long long)state->first_input_sequence, (unsigned)state->count,
                   (unsigned)status, (unsigned)state->input_status) < 0) {
            return 0;
        }
    } else {
        unsigned index;
        if (printf("OUT epoch=%llu sequence=%llu ticket=%llu pdu=%u id=%u dlc=%u data=",
                   (unsigned long long)output->epoch, (unsigned long long)state->sequence,
                   (unsigned long long)output->ticket, (unsigned)output->pdu,
                   (unsigned)output->can_id, (unsigned)output->dlc) < 0) {
            return 0;
        }
        for (index = 0u; index < output->dlc; ++index) {
            if (printf("%02x", (unsigned)output->data[index]) < 0) {
                return 0;
            }
        }
        if (putchar('\n') == EOF) {
            return 0;
        }
    }
    return fflush(stdout) == 0;
}

/* Retain the exact byte count, including embedded NULs, and drain an oversized
 * line before refusing it. fgets/strlen would hide an invalid trailing suffix. */
static int read_line(char line[ECU_BATCH_LINE_CAPACITY], size_t *length) {
    int byte;
    size_t count = 0u;
    int overflow = 0;
    for (;;) {
        byte = getchar();
        if ((byte == EOF) || (byte == (int)'\n')) {
            break;
        }
        if (count < ECU_BATCH_LINE_CAPACITY) {
            line[count] = (char)byte;
            ++count;
        } else {
            overflow = 1;
        }
    }
    if ((byte == EOF) && (ferror(stdin) != 0)) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    if ((byte == EOF) && (count == 0u) && (overflow == 0)) {
        return 0;
    }
    if ((count != 0u) && (line[count - 1u] == '\r')) {
        --count;
    }
    *length = (overflow != 0) ? (ECU_BATCH_LINE_CAPACITY + 1u) : count;
    return 1;
}

static DWORD WINAPI control(void *argument) {
    DWORD started = GetTickCount();
    char line[ECU_BATCH_LINE_CAPACITY];
    size_t length;
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        if ((GetTickCount() - started) >= ECU_BATCH_WATCHDOG_MS) {
            Ecu_TargetAbortNative(E_OS_STATE);
        }
        Sleep(1u);
    }
    Ecu_HostBatchInitialize(&batch);
    if ((_setmode(_fileno(stdin), _O_BINARY) == -1) ||
        (_setmode(_fileno(stdout), _O_BINARY) == -1)) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    if ((printf("READY HostBatchV1\n") < 0) || (fflush(stdout) != 0)) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    while (read_line(line, &length) != 0) {
        Ecu_BatchCommand command;
        StatusType status = Ecu_HostBatchParse(&batch, line, length, &command);
        if ((status == E_OK) && (command == ECU_BATCH_COMMIT)) {
            status = Ecu_HostBatchExecute(&batch, write_output, NULL);
            if ((status != E_OK) && (batch.executing != 0u)) {
                /* Execute returned only an admission refusal; no automotive
                 * work began. Runtime faults close instead of returning here. */
                batch.executing = 0u;
                batch.count = 0u;
            } else {
                continue;
            }
        }
        if (status != E_OK) {
            if ((printf("REJECT status=%u epoch=%llu sequence=%llu\n", (unsigned)status,
                        (unsigned long long)batch.completed_epoch,
                        (unsigned long long)batch.sequence) < 0) ||
                (fflush(stdout) != 0)) {
                Ecu_TargetAbortNative(E_OS_STATE);
            }
        }
    }
    ShutdownOS((batch.open == 0u) ? E_OK : E_OS_VALUE);
    return 0u;
}

int main(void) {
    HANDLE thread;
    if (Ecu_TargetPrepare() != E_OK) {
        return 97;
    }
    thread = CreateThread(NULL, 262144u, control, NULL, 0u, NULL);
    if (thread == NULL) {
        return 96;
    }
    if (CloseHandle(thread) == 0) {
        return 95;
    }
    StartOS(1u);
    return 94;
}
