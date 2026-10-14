/** @file HostBatchV1 execution: admits a batch, drives ticks and writes the receipt. */
#include "Ecu_HostBridge.h"
#include "Ecu_TargetConfig.h"
#include "Ecu_Config.h"
#include "Os_Host.h"
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    Os_HostHandle cancel;
    uint64_t started;
} Ecu_HostWatch;

/* Timing probe for #61 (temporary, enabled by AUTOSAR_TIMING_PROBE). */
static uint64_t probe_started;
static uint64_t probe_at;
static uint64_t probe_tick_started;
static uint64_t probe_max_tick;
static uint64_t probe_span;
static void probe(const char *event, uint64_t value)
{
    const char *path = getenv("AUTOSAR_TIMING_PROBE");
    if (path != NULL) {
        FILE *file = fopen(path, "a");
        if (file != NULL) {
            (void)fprintf(file,
                          "%s=%llu elapsed=%llu at=%llu max_tick=%llu\n",
                          event,
                          (unsigned long long)value,
                          (unsigned long long)(Os_HostMonotonicMs() - probe_started),
                          (unsigned long long)probe_at,
                          (unsigned long long)probe_max_tick);
            (void)fclose(file);
        }
    }
}

static Os_HostThreadResult watch(void *argument)
{
    const Ecu_HostWatch *control = (const Ecu_HostWatch *)argument;
    uint64_t elapsed = Os_HostMonotonicMs() - control->started;
    if (elapsed >= ECU_BATCH_WATCHDOG_MS) {
        probe("abort site=watchdog-start line", __LINE__);
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    if (Os_HostWait(control->cancel, ECU_BATCH_WATCHDOG_MS - (uint32_t)elapsed) == 0) {
        probe("abort site=watchdog line", __LINE__);
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    return 0u;
}

static void require_at(int accepted, unsigned line)
{
    if (accepted == 0) {
        probe("abort site=require line", line);
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}
#define require(accepted) require_at((accepted), __LINE__)

static void sequence(Ecu_HostBatch *batch)
{
    require(batch->sequence != UINT64_MAX);
    ++batch->sequence;
}

static void pump(Ecu_HostBatch *batch,
                 Ecu_HostSink sink,
                 void *context,
                 const Ecu_HostWatch *control)
{
    Ecu_OutputRecord output;
    Ecu_ProtocolRecord failure;
    StatusType status;
    require((Os_HostMonotonicMs() - control->started) < ECU_BATCH_WATCHDOG_MS);
    require(Ecu_TargetState() == ECU_TARGET_READY);
    status = Ecu_TargetTakeProtocolFailure(&failure);
    while (status == E_OK) {
        sequence(batch);
        require(batch->transport_count != UINT16_MAX);
        ++batch->transport_count;
        if (batch->transport_status == ECU_OK) {
            batch->transport_status = failure.status;
            batch->transport_epoch = failure.epoch;
        }
        status = Ecu_TargetTakeProtocolFailure(&failure);
    }
    require(status == E_OS_NOFUNC);
    status = Ecu_TargetTakeOutput(&output);
    if (status == E_OK) {
        sequence(batch);
        require(sink(&output, batch, E_OK, context) != 0);
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
    } else {
        require(status == E_OS_NOFUNC);
    }
}

static void tick(Ecu_HostBatch *batch,
                 uint64_t at,
                 Ecu_HostSink sink,
                 void *context,
                 const Ecu_HostWatch *control)
{
    uint64_t ticket;
    Os_TickCompletion result;
    probe_at = at;
    probe_tick_started = Os_HostMonotonicMs();
    require(Os_TargetAdvanceOneTick(at, &ticket) == E_OK);
    for (;;) {
        StatusType status;
        pump(batch, sink, context, control);
        status = Os_TargetWaitTick(ticket, 1u, &result);
        if (status == E_OK) {
            require(result.epoch == at);
            pump(batch, sink, context, control);
            sequence(batch);
            if ((Os_HostMonotonicMs() - probe_tick_started) > probe_max_tick) {
                probe_max_tick = Os_HostMonotonicMs() - probe_tick_started;
            }
            return;
        }
        require(status == E_OS_NOFUNC);
    }
}

/* Every check that must pass before any automotive state changes. */
static StatusType admit_batch(const Ecu_HostBatch *batch)
{
    StatusType status;
    if ((batch->epoch < batch->completed_epoch) ||
        ((batch->epoch - batch->completed_epoch) > ECU_BATCH_MAX_SPAN) ||
        (batch->batch_id == UINT64_MAX)) {
        status = E_OS_STATE;
    } else {
        status = Ecu_TargetValidateFrames(batch->frames, batch->count);
    }
    if (status == E_OK) {
        const uint64_t span = batch->epoch - batch->completed_epoch;
        const uint64_t transport_frames =
            ((uint64_t)ECU_MAX_PDU_PAYLOAD + UINT64_C(6)) / UINT64_C(7) + UINT64_C(2);
        unsigned diagnostic_inputs = 0u;
        uint16_t index;
        uint64_t required;
        for (index = 0u; index < batch->count; ++index) {
            if (batch->frames[index].id == ECU_TARGET_DIAGNOSTIC_RX_CAN_ID) {
                ++diagnostic_inputs;
            }
        }
        /* Reserve enough identities before changing automotive state: every input,
         * tick completion, one Com output per tick, maximum configured transport
         * output per diagnostic input and any older transport tail, plus receipt.
         * Individual Win32/output tickets remain separate correlation identities. */
        required = (uint64_t)batch->count + (UINT64_C(3) * span) +
                   ((uint64_t)diagnostic_inputs * transport_frames) + UINT64_C(1);
        if (span != UINT64_C(0)) {
            required += transport_frames;
        }
        if (required > (UINT64_MAX - batch->sequence)) {
            status = E_OS_LIMIT;
        }
    }
    return status;
}

StatusType Ecu_HostBatchExecute(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context)
{
    Ecu_HostWatch control;
    Os_HostHandle monitor;
    Os_TickCompletion previous;
    Ecu_BatchCompletion completed;
    uint64_t at;
    uint64_t ticket;
    StatusType status;
    if ((batch == NULL) || (sink == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (batch->executing == 0u) {
        return E_OS_STATE;
    }
    status = admit_batch(batch);
    if (status == E_OK) {
        status = Os_TargetTickCompletion(batch->completed_epoch, &previous);
    }
    if (status != E_OK) {
        /* Nothing ran: the bridge, not its caller, discards the staged commit. */
        batch->executing = 0u;
        batch->count = 0u;
        return status;
    }
    batch->transport_status = ECU_OK;
    batch->transport_epoch = UINT64_C(0);
    batch->transport_count = 0u;
    control.cancel = Os_HostOpenManualEvent();
    if (control.cancel == NULL) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    control.started = Os_HostMonotonicMs();
    probe_started = control.started;
    probe_max_tick = 0u;
    probe_at = batch->completed_epoch;
    probe_span = batch->epoch - batch->completed_epoch;
    monitor = Os_HostSpawnObserverThread(watch, &control);
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
        Os_HostSleepMs(1u);
    }
    require((completed.epoch == batch->epoch) && (completed.input_count == batch->count));
    pump(batch, sink, context, &control);
    batch->input_status = completed.input_status;
    status = ((completed.input_status == ECU_OK) && (batch->transport_status == ECU_OK))
                 ? E_OK
                 : E_OS_VALUE;
    probe("batch span", probe_span);
    require(Ecu_HostBatchComplete(batch) == E_OK);
    require((Os_HostMonotonicMs() - control.started) < ECU_BATCH_WATCHDOG_MS);
    require(sink(NULL, batch, status, context) != 0);
    require(Os_HostSetEvent(control.cancel) != 0);
    require(Os_HostWait(monitor, ECU_BATCH_WATCHDOG_MS) != 0);
    require(Os_HostClose(monitor) != 0);
    require(Os_HostClose(control.cancel) != 0);
    return E_OK;
}
