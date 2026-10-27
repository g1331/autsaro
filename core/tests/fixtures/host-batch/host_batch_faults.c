#include "Ecu_Target.h"
#include "Can_Host.h"
#include "Os_Host.h"
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#else
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#endif
#include <stdio.h>
#include <string.h>

static Ecu_HostBatch batch;
static int mode;
static void require(int accepted);
static size_t pipe_filled;
static unsigned pipe_written;
static volatile Os_Atomic32Type report_actor;
#ifdef _WIN32
static HANDLE pipe_reader;
static HANDLE pipe_writer;
static HANDLE saved_output;
#else
static int pipe_reader;
static int pipe_writer;
static int saved_output;
#endif

/* Called only by the instrumented test copies of the real target/report TUs. */
void HostFaultConfirm(uint64_t ticket, PduIdType pdu)
{
    if (mode >= 7) {
        require(fprintf(stderr,
                        "pipe_confirm ticket=%llu pdu=%u written=%u\n",
                        (unsigned long long)ticket,
                        pdu,
                        pipe_written) >= 0);
        require(fflush(stderr) == 0);
        require(pipe_written != 0u);
    }
}
void HostFaultReport(StatusType reason)
{
    if (mode >= 7) {
        (void)Os_HostAtomicExchange(&report_actor, (Os_Atomic32Type)Os_HostThreadIdentity());
        require(fprintf(stderr, "pipe_report_enter reason=%u\n", reason) >= 0);
        require(fflush(stderr) == 0);
    }
}
void HostFaultNativeWrite(int stream)
{
    if ((mode >= 7) &&
        (Os_HostAtomicLoad(&report_actor) == (Os_Atomic32Type)Os_HostThreadIdentity())) {
        require(fprintf(stderr, "pipe_report_native_write stream=%d\n", stream) >= 0);
        require(fflush(stderr) == 0);
    }
}

static void prepare_pipe(void)
{
    const char byte = 'X';
#ifdef _WIN32
    char name[96];
    int length =
        snprintf(name, sizeof(name), "\\\\.\\pipe\\autosar-output-%lu", GetCurrentProcessId());
    require((length > 0) && ((size_t)length < sizeof(name)));
    pipe_writer = CreateNamedPipeA(name,
                                   PIPE_ACCESS_OUTBOUND,
                                   PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT,
                                   1u,
                                   4096u,
                                   4096u,
                                   0u,
                                   NULL);
    require(pipe_writer != INVALID_HANDLE_VALUE);
    pipe_reader = CreateFileA(name, GENERIC_READ, 0u, NULL, OPEN_EXISTING, 0u, NULL);
    require(pipe_reader != INVALID_HANDLE_VALUE);
    BOOL connected = ConnectNamedPipe(pipe_writer, NULL);
    require((connected != 0) || (GetLastError() == ERROR_PIPE_CONNECTED));
    for (;;) {
        DWORD count = 0u;
        BOOL result = WriteFile(pipe_writer, &byte, 1u, &count, NULL);
        if ((result == 0) || (count == 0u)) {
            require((count == 0u) && ((result != 0) || (GetLastError() == ERROR_NO_DATA)));
            break;
        }
        require(count == 1u);
        ++pipe_filled;
        require(pipe_filled <= 1048576u);
    }
    DWORD state = PIPE_READMODE_BYTE | PIPE_WAIT;
    require(SetNamedPipeHandleState(pipe_writer, &state, NULL, NULL) != 0);
    saved_output = GetStdHandle(STD_OUTPUT_HANDLE);
    require(SetStdHandle(STD_OUTPUT_HANDLE, pipe_writer) != 0);
#else
    int descriptors[2];
    require(pipe(descriptors) == 0);
    pipe_reader = descriptors[0];
    pipe_writer = descriptors[1];
    const int flags = fcntl(pipe_writer, F_GETFL);
    require((flags >= 0) && (fcntl(pipe_writer, F_SETFL, flags | O_NONBLOCK) == 0));
    for (;;) {
        ssize_t count = write(pipe_writer, &byte, 1u);
        if (count < 0) {
            require(errno == EAGAIN);
            break;
        }
        require(count == 1);
        ++pipe_filled;
        require(pipe_filled <= 1048576u);
    }
    require(fcntl(pipe_writer, F_SETFL, flags) == 0);
    saved_output = dup(STDOUT_FILENO);
    require((saved_output >= 0) && (dup2(pipe_writer, STDOUT_FILENO) == STDOUT_FILENO));
#endif
    require(pipe_filled != 0u);
    require(fprintf(stderr, "pipe_full mode=%d bytes=%lu\n", mode, (unsigned long)pipe_filled) >=
            0);
    require(fflush(stderr) == 0);
}

static Os_HostThreadResultType drain_pipe(void *argument)
{
    size_t filler = pipe_filled;
    char bytes[256];
    char payload[4096];
    size_t used = 0u;
    (void)argument;
    for (;;) {
        size_t count;
#ifdef _WIN32
        DWORD received = 0u;
        BOOL result = ReadFile(pipe_reader, bytes, sizeof(bytes), &received, NULL);
        if (result == 0) {
            require(GetLastError() == ERROR_BROKEN_PIPE);
            break;
        }
        count = received;
#else
        ssize_t received = read(pipe_reader, bytes, sizeof(bytes));
        if ((received < 0) && (errno == EINTR)) {
            continue;
        }
        require(received >= 0);
        count = (size_t)received;
#endif
        if (count == 0u) {
            break;
        }
        const size_t skipped = (count < filler) ? count : filler;
        for (size_t index = 0u; index < skipped; ++index) {
            require(bytes[index] == 'X');
        }
        filler -= skipped;
        require((count - skipped) <= (sizeof(payload) - used));
        for (size_t index = skipped; index < count; ++index) {
            payload[used] = bytes[index];
            ++used;
        }
    }
    require(filler == 0u);
    require(Os_HostWriteBytes(1, payload, used) != 0);
    return 0u;
}
#ifdef ECU_TARGET_TESTS
static unsigned queued;
int Ecu_TargetTestFailStage(unsigned stage)
{
    if ((mode == 4) && (stage == 8u)) {
        static const uint8_t bytes[8] = {0u};
        for (queued = 0u; queued < 256u; ++queued) {
            require(Can_TransmitPdu(1u, 0x321u, 4u, bytes) == ECU_OK);
        }
        require(printf("output_capacity accepted=256 next=257\n") >= 0);
        require(fflush(stdout) == 0);
        (void)Can_TransmitPdu(1u, 0x321u, 4u, bytes);
        require(0);
    }
    return 0;
}
void Ecu_TargetTestRetire(unsigned stage) { (void)stage; }
void Ecu_TargetTestShutdown(StatusType reason)
{
    require(printf("output_close queued=%u reason=%u\n", queued, reason) >= 0);
    if (fflush(stdout) != 0) {
        Os_HostExit(93u);
    }
}
#endif

static void require(int accepted)
{
    if (accepted == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}
static int sink(const Ecu_OutputRecord *output,
                const Ecu_HostBatch *state,
                StatusType status,
                void *context)
{
    (void)status;
    (void)context;
    if (mode >= 7) {
        require(fprintf(stderr,
                        "pipe_write_enter stage=%s ticket=%llu\n",
                        output == NULL ? "receipt" : "output",
                        output == NULL ? 0ull : (unsigned long long)output->ticket) >= 0);
        require(fflush(stderr) == 0);
        require(write_output(output, state, status, context) != 0);
        ++pipe_written;
        require(fprintf(stderr,
                        "pipe_write_complete stage=%s\n",
                        output == NULL ? "receipt" : "output") >= 0);
        require(fflush(stderr) == 0);
        return 1;
    }
    if (output != NULL) {
        require(fprintf(stderr,
                        "fault_output ticket=%llu mode=%d\n",
                        (unsigned long long)output->ticket,
                        mode) >= 0);
        require(fflush(stderr) == 0);
        if ((mode == 5) || (mode == 6)) {
            /* Use the exact production writer, with a real failed host stream. */
            int written = write_output(output, state, status, context);
            require(written == 0);
            require(fprintf(stderr,
                            "physical_%s failed no_receipt ticket=%llu\n",
                            (mode == 5) ? "write" : "closed",
                            (unsigned long long)output->ticket) >= 0);
            return 0;
        }
        if (mode == 1) {
            return 0;
        }
        if (mode == 2) {
            for (;;) {
                Os_HostSleepMs(1000u);
            }
        }
    } else {
        require(printf("fault_commit epoch=%llu sequence=%llu\n",
                       (unsigned long long)state->completed_epoch,
                       (unsigned long long)state->sequence) >= 0);
        require(fflush(stdout) == 0);
    }
    return 1;
}
static Os_HostThreadResultType control(void *argument)
{
    Ecu_BatchCommand command;
    uint64_t started = Os_HostMonotonicMs();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((Os_HostMonotonicMs() - started) < 5000u);
        Os_HostSleepMs(1u);
    }
    Ecu_HostBatchInitialize(&batch);
    if (mode == 3) {
        batch.sequence = UINT64_MAX;
        require(Ecu_HostBatchParse(&batch, "BEGIN 0", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OS_LIMIT);
        require(batch.completed_epoch == UINT64_C(0));
        Ecu_HostBatchInitialize(&batch);
        batch.sequence = UINT64_MAX - UINT64_C(1);
        require(Ecu_HostBatchParse(&batch, "BEGIN 1", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        require(Ecu_HostBatchExecute(&batch, sink, NULL) == E_OS_LIMIT);
        {
            Os_TickCompletionType initial;
            require(Os_TargetTickCompletion(UINT64_C(0), &initial) == E_OK);
            require(initial.epoch == UINT64_C(0));
        }
        Ecu_HostBatchInitialize(&batch);
        batch.sequence = UINT64_MAX - UINT64_C(1);
        require(Ecu_HostBatchParse(&batch, "BEGIN 0", 7u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        require(Ecu_HostBatchExecute(&batch, sink, NULL) == E_OK);
        require(batch.sequence == UINT64_MAX);
        require(printf("sequence_limit before_state_change=pass final_identity=pass\n") >= 0);
        require(fflush(stdout) == 0);
        ShutdownOS(E_OK);
    } else {
        require(Ecu_HostBatchParse(&batch, "BEGIN 10", 8u, &command) == E_OK);
        require(Ecu_HostBatchParse(&batch, "COMMIT", 6u, &command) == E_OK);
        if ((mode == 5) || (mode == 6)) {
#ifdef _WIN32
            HANDLE reader;
            HANDLE writer;
            require(CreatePipe(&reader, &writer, NULL, 0u) != 0);
            require(CloseHandle(reader) != 0);
            if (mode == 6) {
                require(CloseHandle(writer) != 0);
                writer = INVALID_HANDLE_VALUE;
            }
            require(SetStdHandle(STD_OUTPUT_HANDLE, writer) != 0);
#else
            if (mode == 5) {
                require(freopen("/dev/full", "w", stdout) != NULL);
            } else {
                require(close(STDOUT_FILENO) == 0);
            }
#endif
        }
        Os_HostHandleType drainer = NULL;
        if (mode >= 7) {
            prepare_pipe();
            if (mode == 8) {
                drainer = Os_HostSpawnObserverThread(drain_pipe, NULL);
                require(drainer != NULL);
            }
        }
        (void)Ecu_HostBatchExecute(&batch, sink, NULL);
        if (mode == 8) {
#ifdef _WIN32
            require(SetStdHandle(STD_OUTPUT_HANDLE, saved_output) != 0);
            require(CloseHandle(pipe_writer) != 0);
#else
            require(dup2(saved_output, STDOUT_FILENO) == STDOUT_FILENO);
            require(close(saved_output) == 0);
            require(close(pipe_writer) == 0);
#endif
            require(Os_HostWait(drainer, ECU_BATCH_WATCHDOG_MS) != 0);
            require(Os_HostClose(drainer) != 0);
#ifdef _WIN32
            require(CloseHandle(pipe_reader) != 0);
#else
            require(close(pipe_reader) == 0);
#endif
            ShutdownOS(E_OK);
        }
        /* A failed/blocked output must close, not report a completed batch. */
        require(0);
    }
    return 0u;
}
int main(int argc, char **argv)
{
    Os_HostHandleType thread;
    if (argc != 2) {
        return 98;
    }
    if (strcmp(argv[1], "fail") == 0) {
        mode = 1;
    } else if (strcmp(argv[1], "block") == 0) {
        mode = 2;
    } else if (strcmp(argv[1], "write") == 0) {
        mode = 5;
    } else if (strcmp(argv[1], "closed") == 0) {
        mode = 6;
    } else if (strcmp(argv[1], "sequence") == 0) {
        mode = 3;
    } else if (strcmp(argv[1], "pipe-block") == 0) {
        mode = 7;
    } else if (strcmp(argv[1], "pipe-drain") == 0) {
        mode = 8;
#ifdef ECU_TARGET_TESTS
    } else if (strcmp(argv[1], "overflow") == 0) {
        mode = 4;
#endif
    } else {
        return 98;
    }
    require(Ecu_TargetPrepare() == E_OK);
    if (mode != 4) {
        thread = Os_HostSpawnThread(control, NULL);
        require(thread != NULL);
        require(Os_HostClose(thread) != 0);
    }
    StartOS(1u);
    return 94;
}
