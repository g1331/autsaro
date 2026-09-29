#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Ecu_Config.h"
#include "Os_Windows.h"

typedef struct {
    HANDLE cancel;
    uint64_t started;
} Ecu_HostWatch;

static DWORD WINAPI watch(void *argument) {
    const Ecu_HostWatch *control = (const Ecu_HostWatch *)argument;
    uint64_t elapsed = GetTickCount64() - control->started;
    DWORD waited;
    if (elapsed >= ECU_BATCH_WATCHDOG_MS) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    waited = WaitForSingleObject(control->cancel, ECU_BATCH_WATCHDOG_MS - (DWORD)elapsed);
    if (waited != WAIT_OBJECT_0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    return 0u;
}

static void require(int accepted) {
    if (accepted == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}

static void sequence(Ecu_HostBatch *batch) {
    require(batch->sequence != UINT64_MAX);
    ++batch->sequence;
}

static void pump(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context,
                 const Ecu_HostWatch *control) {
    Ecu_OutputRecord output;
    StatusType status;
    require((GetTickCount64() - control->started) < ECU_BATCH_WATCHDOG_MS);
    require(Ecu_TargetState() == ECU_TARGET_READY);
    status = Ecu_TargetTakeOutput(&output);
    if (status == E_OK) {
        sequence(batch);
        require(sink(&output, batch, E_OK, context) != 0);
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
    } else {
        require(status == E_OS_NOFUNC);
    }
}

static void tick(Ecu_HostBatch *batch, uint64_t at, Ecu_HostSink sink, void *context,
                 const Ecu_HostWatch *control) {
    uint64_t ticket;
    Os_TickCompletion result;
    require(Os_TargetAdvanceOneTick(at, &ticket) == E_OK);
    for (;;) {
        StatusType status;
        pump(batch, sink, context, control);
        status = Os_TargetWaitTick(ticket, 1u, &result);
        if (status == E_OK) {
            require(result.epoch == at);
            sequence(batch);
            return;
        }
        require(status == E_OS_NOFUNC);
    }
}

StatusType Ecu_HostBatchExecute(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context) {
    Ecu_HostWatch control;
    HANDLE monitor;
    Os_TickCompletion previous;
    Ecu_BatchCompletion completed;
    uint64_t at;
    uint64_t ticket;
    uint64_t span;
    uint64_t required;
    uint64_t transport_frames =
        ((uint64_t)ECU_TARGET_DCM_BUFFER_BYTES + UINT64_C(6)) / UINT64_C(7) + UINT64_C(2);
    unsigned diagnostic_inputs = 0u;
    uint16_t index;
    StatusType status;
    if ((batch == NULL) || (sink == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if ((batch->executing == 0u) || (batch->epoch < batch->completed_epoch) ||
        ((batch->epoch - batch->completed_epoch) > ECU_BATCH_MAX_SPAN) ||
        (batch->batch_id == UINT64_MAX)) {
        return E_OS_STATE;
    }
    status = Ecu_TargetValidateFrames(batch->frames, batch->count);
    if (status != E_OK) {
        return status;
    }
    span = batch->epoch - batch->completed_epoch;
    for (index = 0u; index < batch->count; ++index) {
        if (batch->frames[index].id == Ecu_Config.diagnostic->request_can_id) {
            ++diagnostic_inputs;
        }
    }
    /* Reserve enough identities before changing automotive state: every input,
     * tick completion, one Com output per tick, maximum configured transport
     * output per diagnostic input and any older transport tail, plus receipt.
     * Individual Win32/output tickets remain separate correlation identities. */
    required = (uint64_t)batch->count + (UINT64_C(2) * span) +
               ((uint64_t)diagnostic_inputs * transport_frames) + UINT64_C(1);
    if (span != UINT64_C(0)) {
        required += transport_frames;
    }
    if (required > (UINT64_MAX - batch->sequence)) {
        return E_OS_LIMIT;
    }
    status = Os_TargetTickCompletion(batch->completed_epoch, &previous);
    if (status != E_OK) {
        return status;
    }
    control.cancel = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (control.cancel == NULL) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    control.started = GetTickCount64();
    monitor = CreateThread(NULL, 262144u, watch, &control, 0u, NULL);
    if (monitor == NULL) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    at = batch->completed_epoch;
    while ((batch->epoch - at) > UINT64_C(1)) {
        ++at;
        tick(batch, at, sink, context, &control);
    }
    batch->first_input_sequence =
        (batch->count == 0u) ? UINT64_C(0) : batch->sequence + UINT64_C(1);
    batch->sequence += (uint64_t)batch->count;
    batch->input_sequences_reserved = 1u;
    require(Ecu_TargetPostBatch(batch->epoch, batch->frames, batch->count, &ticket) == E_OK);
    if (batch->epoch != batch->completed_epoch) {
        tick(batch, batch->epoch, sink, context, &control);
    }
    for (;;) {
        pump(batch, sink, context, &control);
        status = Ecu_TargetBatchCompletion(ticket, &completed);
        if (status == E_OK) {
            break;
        }
        require(status == E_OS_NOFUNC);
        Sleep(1u);
    }
    require((completed.epoch == batch->epoch) && (completed.input_count == batch->count));
    batch->input_status = completed.input_status;
    status = (completed.input_status == ECU_OK) ? E_OK : E_OS_VALUE;
    require(Ecu_HostBatchComplete(batch) == E_OK);
    require((GetTickCount64() - control.started) < ECU_BATCH_WATCHDOG_MS);
    require(sink(NULL, batch, status, context) != 0);
    require(Os_HostSetEvent(control.cancel) != 0);
    require(WaitForSingleObject(monitor, ECU_BATCH_WATCHDOG_MS) == WAIT_OBJECT_0);
    require(CloseHandle(monitor) != 0);
    require(CloseHandle(control.cancel) != 0);
    return status;
}
