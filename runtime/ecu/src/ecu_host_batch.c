/** @file Host batch ECU executable speaking HostBatchV1 over standard streams. */
#include "Ecu_HostBridge.h"
#include "Os_Host.h"
#include "Os_HostIo.h"

#ifndef _WIN32
#include <errno.h>
#include <unistd.h>
#endif

/* The only input consumer owns this private platform read boundary. */
static int read_byte(char *byte)
{
#ifdef _WIN32
    DWORD count = 0u;
    int result = -1;
    if (ReadFile(GetStdHandle(STD_INPUT_HANDLE), byte, 1u, &count, NULL) != 0) {
        result = (int)count;
    } else if (GetLastError() == (DWORD)ERROR_BROKEN_PIPE) {
        result = 0;
    } else {
        /* Invalid handles and other read failures are not EOF. */
    }
    return result;
#else
    ssize_t count;
    do {
        errno = 0;
        count = read(STDIN_FILENO, byte, 1u);
    } while ((count < 0) && (errno == EINTR));
    return (count < 0) ? -1 : (int)count;
#endif
}

static void receipt(Os_HostTextType *text, const Ecu_HostBatch *state, StatusType status)
{
    Os_HostTextAppend(text, (status == E_OK) ? "COMMIT_OK" : "COMMIT_ERROR");
    Os_HostTextField(text, " batch=", state->batch_id);
    Os_HostTextField(text, " epoch=", state->completed_epoch);
    Os_HostTextField(text, " sequence=", state->sequence);
    Os_HostTextField(text, " input_first=", state->first_input_sequence);
    Os_HostTextField(text, " inputs=", state->count);
    Os_HostTextField(text, " status=", status);
    Os_HostTextField(text, " input_status=", state->input_status);
    Os_HostTextField(text, " transport_status=", state->transport_status);
    Os_HostTextField(text, " transport_epoch=", state->transport_epoch);
    Os_HostTextField(text, " transport_count=", state->transport_count);
}

static int write_output(const Ecu_OutputRecord *output,
                        const Ecu_HostBatch *state,
                        StatusType status,
                        void *context)
{
    char storage[512];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    (void)context;
    if (output == NULL) {
        receipt(&text, state, status);
    } else if ((output->dlc > sizeof(output->data)) || (output->can_id > 0x7ffu)) {
        text.failed = 1;
    } else {
        size_t index;
        Os_HostTextField(&text, "OUT epoch=", output->epoch);
        Os_HostTextField(&text, " sequence=", state->sequence);
        Os_HostTextField(&text, " ticket=", output->ticket);
        Os_HostTextField(&text, " pdu=", output->pdu);
        Os_HostTextField(&text, " id=", output->can_id);
        Os_HostTextField(&text, " dlc=", output->dlc);
        Os_HostTextAppend(&text, " data=");
        for (index = 0u; index < output->dlc; ++index) {
            Os_HostTextHex(&text, output->data[index], 2u, 0);
        }
    }
    Os_HostTextByte(&text, '\n');
    return Os_HostTextWrite(&text);
}

static int write_rejection(const Ecu_HostBatch *state, StatusType status)
{
    char storage[128];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "REJECT status=", status);
    Os_HostTextField(&text, " epoch=", state->completed_epoch);
    Os_HostTextField(&text, " sequence=", state->sequence);
    Os_HostTextByte(&text, '\n');
    return Os_HostTextWrite(&text);
}

/* Retain the exact byte count, including embedded NULs, and drain an oversized
 * line before refusing it. fgets/strlen would hide an invalid trailing suffix. */
static int read_line(char line[ECU_BATCH_LINE_CAPACITY], size_t *length)
{
    char byte;
    int received;
    size_t count = 0u;
    int overflow = 0;
    for (;;) {
        received = read_byte(&byte);
        if ((received != 1) || (byte == '\n')) {
            break;
        }
        if (count < ECU_BATCH_LINE_CAPACITY) {
            line[count] = byte;
            ++count;
        } else {
            overflow = 1;
        }
    }
    if (received < 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    if ((received == 0) && (count == 0u) && (overflow == 0)) {
        return 0;
    }
    if ((count != 0u) && (line[count - 1u] == '\r')) {
        --count;
    }
    *length = (overflow != 0) ? (ECU_BATCH_LINE_CAPACITY + 1u) : count;
    return 1;
}

static Os_HostThreadResultType control(void *argument)
{
    static Ecu_HostBatch batch; /**< Native control only; HostBatchInitialize zeros before use. */
    uint64_t started = Os_HostMonotonicMs();
    char line[ECU_BATCH_LINE_CAPACITY];
    size_t length;
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        if ((Os_HostMonotonicMs() - started) >= ECU_BATCH_WATCHDOG_MS) {
            Ecu_TargetAbortNative(E_OS_STATE);
        }
        Os_HostSleepMs(1u);
    }
    Ecu_HostBatchInitialize(&batch);
    if (Os_HostWriteBytes(0, "READY HostBatchV1\n", sizeof("READY HostBatchV1\n") - 1u) == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    while (read_line(line, &length) != 0) {
        Ecu_BatchCommand command;
        StatusType status = Ecu_HostBatchParse(&batch, line, length, &command);
        if ((status == E_OK) && (command == ECU_BATCH_COMMIT)) {
            /* E_OK: the receipt is written. Otherwise nothing ran. */
            status = Ecu_HostBatchExecute(&batch, &write_output, NULL);
        }
        if (status != E_OK) {
            if (write_rejection(&batch, status) == 0) {
                Ecu_TargetAbortNative(E_OS_STATE);
            }
        }
    }
    ShutdownOS((batch.open == 0u) ? E_OK : E_OS_VALUE);
    return 0u;
}

int main(void)
{
    Os_HostHandleType thread;
    if (Ecu_TargetPrepare() != E_OK) {
        return 97;
    }
    thread = Os_HostSpawnThread(&control, NULL);
    if (thread == NULL) {
        return 96;
    }
    if (Os_HostClose(thread) == 0) {
        return 95;
    }
    StartOS(OSDEFAULTAPPMODE);
    return 94;
}
