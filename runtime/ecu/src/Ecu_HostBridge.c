/** @file HostBatchV1 execution: admits a batch, drives ticks and writes the receipt. */
#include "Ecu_HostBridge.h"
#include "Ecu_TargetConfig.h"
#include "Ecu_ComRejection.h"
#include "Ecu_Config.h"
#include "Os_Host.h"

typedef struct {
    Os_HostHandleType cancel;
    uint64_t started;
} Ecu_HostWatch;

static Ecu_HostWatch
    host_watch; /**< Sole producer joins its observer before reuse; no Init reset. */

static Os_HostThreadResultType watch(void *argument)
{
    uint64_t elapsed = Os_HostMonotonicMs() - host_watch.started;
    (void)argument;
    if (elapsed >= ECU_BATCH_WATCHDOG_MS) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    if (Os_HostWait(host_watch.cancel, ECU_BATCH_WATCHDOG_MS - (uint32_t)elapsed) == 0) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    return 0u;
}

static void bridge_require(boolean accepted)
{
    if (accepted == FALSE) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
}

static void sequence(Ecu_HostBatch *batch)
{
    bridge_require(batch->sequence != UINT64_MAX);
    ++batch->sequence;
}

static void pump(Ecu_HostBatch *batch,
                 Ecu_HostSink sink,
                 void *context,
                 const Ecu_HostWatch *control)
{
    bridge_require(Ecu_ComDrainRejections(FALSE) != 0);
    Ecu_OutputRecord output;
    Ecu_ProtocolRecord failure;
    StatusType status;
    bridge_require((Os_HostMonotonicMs() - control->started) < ECU_BATCH_WATCHDOG_MS);
    bridge_require(Ecu_TargetState() == ECU_TARGET_READY);
    status = Ecu_TargetTakeProtocolFailure(&failure);
    while (status == E_OK) {
        sequence(batch);
        bridge_require(batch->transport_count != (uint16)UINT16_MAX);
        ++batch->transport_count;
        if (batch->transport_status == ECU_OK) {
            batch->transport_status = failure.status;
            batch->transport_epoch = failure.epoch;
        }
        status = Ecu_TargetTakeProtocolFailure(&failure);
    }
    bridge_require(status == E_OS_NOFUNC);
    status = Ecu_TargetTakeOutput(&output);
    if (status == E_OK) {
        sequence(batch);
        bridge_require(sink(&output, batch, E_OK, context) != 0);
        bridge_require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
    } else {
        bridge_require(status == E_OS_NOFUNC);
    }
}

static void tick(Ecu_HostBatch *batch,
                 uint64_t at,
                 Ecu_HostSink sink,
                 void *context,
                 const Ecu_HostWatch *control)
{
    uint64_t ticket;
    Os_TickCompletionType result;
    bridge_require(Os_TargetAdvanceOneTick(at, &ticket) == E_OK);
    for (;;) {
        StatusType status;
        pump(batch, sink, context, control);
        status = Os_TargetWaitTick(ticket, 1u, &result);
        if (status == E_OK) {
            bridge_require(result.epoch == at);
            pump(batch, sink, context, control);
            sequence(batch);
            return;
        }
        bridge_require(status == E_OS_NOFUNC);
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
            (((uint64_t)ECU_MAX_PDU_PAYLOAD + UINT64_C(6)) / UINT64_C(7)) + UINT64_C(2);
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

/* MR12 RULE 8.7 VIOLATION: the probe delivery retains the host-batch library export. */
StatusType Ecu_HostBatchExecute(Ecu_HostBatch *batch, Ecu_HostSink sink, void *context)
{
    Os_HostHandleType monitor;
    Os_TickCompletionType previous;
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
    host_watch.cancel = Os_HostOpenManualEvent();
    if (host_watch.cancel == NULL) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    host_watch.started = Os_HostMonotonicMs();
    monitor = Os_HostSpawnObserverThread(&watch, NULL);
    if (monitor == NULL) {
        Ecu_TargetAbortNative(E_OS_STATE);
    }
    at = batch->completed_epoch;
    while ((batch->epoch - at) > UINT64_C(1)) {
        ++at;
        tick(batch, at, sink, context, &host_watch);
    }
    batch->first_input_sequence =
        (batch->count == 0u) ? UINT64_C(0) : (batch->sequence + UINT64_C(1));
    batch->sequence += (uint64_t)batch->count;
    batch->input_sequences_reserved = 1u;
    bridge_require(Ecu_TargetPostBatch(batch->epoch, batch->frames, batch->count, &ticket) == E_OK);
    if (batch->epoch != batch->completed_epoch) {
        tick(batch, batch->epoch, sink, context, &host_watch);
    }
    for (;;) {
        pump(batch, sink, context, &host_watch);
        status = Ecu_TargetBatchCompletion(ticket, &completed);
        if (status == E_OK) {
            break;
        }
        bridge_require(status == E_OS_NOFUNC);
        Os_HostSleepMs(1u);
    }
    bridge_require((completed.epoch == batch->epoch) && (completed.input_count == batch->count));
    pump(batch, sink, context, &host_watch);
    batch->input_status = completed.input_status;
    status = ((completed.input_status == ECU_OK) && (batch->transport_status == ECU_OK))
                 ? E_OK
                 : E_OS_VALUE;
    bridge_require(Ecu_HostBatchComplete(batch) == E_OK);
    bridge_require((Os_HostMonotonicMs() - host_watch.started) < ECU_BATCH_WATCHDOG_MS);
    bridge_require(sink(NULL, batch, status, context) != 0);
    bridge_require(Os_HostSetEvent(host_watch.cancel) != 0);
    bridge_require(Os_HostWait(monitor, ECU_BATCH_WATCHDOG_MS) != 0);
    bridge_require(Os_HostClose(monitor) != 0);
    bridge_require(Os_HostClose(host_watch.cancel) != 0);
    return E_OK;
}
