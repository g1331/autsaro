/** @file Single-owner ECU target: lifecycle, input mailbox and output tickets on native hosts. */
#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Ecu_Execution.h"
#include "Os_IntegrationHooks.h"
#include "Os_Backend.h"
#include "Can_Host.h"
#include "CanIf.h"
#include "CanTp.h"
#include "CanTp_Internal.h"
#include "Com.h"
#include "Com_Internal.h"
#include "Dcm.h"
#include "LSduR.h"
#include "Os_Mailbox.h"
#include "PduR.h"
#include "SchM_Can.h"
#include "Rte_Main.h"
#include <string.h>

typedef struct {
    uint64_t epoch;
    uint64_t output_ticket;
    uint32_t can_id;
    PduIdType pdu;
    uint8_t kind;
    uint8_t dlc;
    uint8_t data[8];
} Ecu_Input;
/* C99 checked storage for one complete mailbox payload. */
typedef Ecu_Input Ecu_MailboxInput[(sizeof(Ecu_Input) <= OS_INPUT_PAYLOAD) ? 1 : -1];
typedef struct {
    volatile Os_Atomic32Type state;
    Ecu_OutputRecord record;
    uint64_t driver_token;
} Ecu_OutputSlot;

typedef struct {
    volatile Os_Atomic32Type state;
    Ecu_ProtocolRecord record;
} Ecu_ProtocolSlot;

/* Process-local target: zero-initialized once; Prepare never resets these queues.
 * Atomic lifecycle/native-owner and slot states publish cross-thread payloads.
 * Native identity is immutable after native_owner reaches 2. Output slots advance
 * 0 free -> 1 published -> 2 taken -> 3 confirmation admitted -> 0 owner-retired.
 * Batch ownership advances 0 free -> 1 native writing -> 2 published -> 3 owner
 * executing -> 4 awaiting quiescence -> 5 complete -> 0 native consumed.
 * Ordinary epoch/cycle/output-writer state is accessed only by the ECU owner.
 * Native reader/confirmation indices are accessed only by the bound producer.
 */
static volatile Os_Atomic32Type
    lifecycle; /**< Atomic target lifecycle; Prepare claims zero once. */
/* Accessed only by the native producer after its atomic identity publication. */
static uint64_t native_input_epoch;  /**< Native-only last admitted epoch, ms. */
static uint64_t native_batch_ticket; /**< Native-only last admitted batch identity. */
static volatile Os_Atomic32Type
    batch_state;                 /**< Atomic batch handoff state, 0..5; no Prepare reset. */
static uint64_t batch_epoch;     /**< Published batch epoch, ms; protected by batch_state. */
static uint16_t batch_count;     /**< Published frame count, 0..256; protected by batch_state. */
static uint8_t batch_needs_tick; /**< Native-published tick flag; batch_state handoff. */
static Ecu_BatchFrame batch_frames[ECU_BATCH_CAPACITY]; /**< Payload protected by batch_state. */
static Ecu_BatchCompletion batch_completion;            /**< Result protected by batch_state. */
static Os_HostThreadIdType initialization_thread;       /**< Startup writes once before READY. */
static uint64_t epoch;           /**< Owner-only current logical epoch, ms; initially zero. */
static uint64_t processed_tick;  /**< Owner-only last processed tick ticket; initially zero. */
static uint64_t processed_epoch; /**< Owner-only last processed epoch, ms. */
static unsigned output_retire;   /**< Owner-only next retirement slot; initially zero. */
static unsigned output_pending;  /**< Owner-only outstanding output count; initially zero. */
/* Confirmations taken before their epoch's cycle, retired after it in order. */
static unsigned output_deferred; /**< Owner-only confirmations deferred until cycle. */
static Ecu_OutputSlot outputs[ECU_TARGET_OUTPUT_CAPACITY];             /**< Atomic output slots. */
static Ecu_ProtocolSlot protocol_failures[ECU_TARGET_OUTPUT_CAPACITY]; /**< Atomic failure slots. */
/* Highest generated bootstrap stage that began; retirement undoes only those. */
static uint8 bootstrap_reached; /**< Startup writes; stopped-owner shutdown reads. */

static void fail(void)
{
    (void)Os_HostAtomicExchange(&lifecycle, (Os_Atomic32Type)ECU_TARGET_FAILED);
    ShutdownOS(E_OS_STATE);
}
static Os_Atomic32Type load(volatile Os_Atomic32Type *value) { return Os_HostAtomicLoad(value); }

/* Owner-only writer; a full queue closes the target instead of dropping a record. */
static void record_protocol_failure(EcuStatus status)
{
    static unsigned
        protocol_write; /**< Owner-only next failure publication slot; initially zero. */
    Ecu_ProtocolSlot *slot = &protocol_failures[protocol_write];
    if (load(&slot->state) != 0) {
        fail();
    }
    slot->record.epoch = epoch;
    slot->record.status = status;
    protocol_write = (protocol_write + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    (void)Os_HostAtomicExchange(&slot->state, 1);
}
void Ecu_TargetReportTransportTimeout(void)
{
    if (Ecu_TargetIsOwner() == 0) {
        fail();
    } else {
        record_protocol_failure(ECU_ERR_TP_TIMEOUT);
    }
}
uint8_t Ecu_TargetState(void)
{
    Os_Atomic32Type state = load(&lifecycle);
    if (state == (Os_Atomic32Type)ECU_TARGET_READY) {
        if (Os_TargetReady() == 0) {
            state = (Os_Atomic32Type)ECU_TARGET_INITIALIZING;
        }
    }
    return (uint8_t)state;
}
int Ecu_TargetIsOwner(void)
{
    int owned = 0;
    if (load(&lifecycle) == (Os_Atomic32Type)ECU_TARGET_INITIALIZING) {
        if (initialization_thread != 0u) {
            owned = (Os_HostThreadIdentity() == initialization_thread) ? 1 : 0;
        }
    } else {
        if (Ecu_TargetState() == ECU_TARGET_READY) {
            owned = (Os_BackendTaskOwner(ECU_TARGET_TASK) == 1) ? 1 : 0;
        }
    }
    return owned;
}
void Ecu_TargetAssertOwner(void)
{
    if (Ecu_TargetIsOwner() == 0) {
        fail();
    }
}
Std_ReturnType Ecu_TargetCheckLifecycleContext(boolean starting)
{
    const Os_HookPhaseType phase = Os_HookContext();
    return (((starting == TRUE) && (phase == OS_HOOK_STARTUP)) ||
            ((starting == FALSE) && (phase == OS_HOOK_SHUTDOWN)))
               ? E_OK
               : E_NOT_OK;
}
/* MR12 RULE 8.7 VIOLATION: Retain the owner-checked clock export for delivered control programs;
 * sample programs without the unused execution wrapper still report it. See retained-clock.md. */
uint64_t Ecu_TargetNow(void)
{
    Ecu_TargetAssertOwner();
    return epoch;
}
static StatusType native_context(void)
{
    static volatile Os_Atomic32Type
        native_owner;                         /**< Atomic binding: 0 unbound, 1 writing, 2 ready. */
    static Os_HostThreadIdType native_thread; /**< Native identity published by native_owner. */
    Os_Atomic32Type owner;
    StatusType status = E_OS_ACCESS;
    if (Ecu_TargetState() != ECU_TARGET_READY) {
        return E_OS_STATE;
    }
    if (Ecu_TargetIsOwner() != 0) {
        return E_OS_CALLEVEL;
    }
    owner = Os_HostAtomicCompareExchange(&native_owner, 1, 0);
    if (owner == 0) {
        native_thread = Os_HostThreadIdentity();
        (void)Os_HostAtomicExchange(&native_owner, 2);
        return E_OK;
    }
    if (owner == 2) {
        if (native_thread == Os_HostThreadIdentity()) {
            status = E_OK;
        }
    }
    return status;
}

StatusType Ecu_TargetTakeProtocolFailure(Ecu_ProtocolRecord *record)
{
    static unsigned
        protocol_read; /**< Native-only next failure consumption slot; initially zero. */
    StatusType status;
    Ecu_ProtocolSlot *slot = &protocol_failures[protocol_read];
    if (record == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_context();
    if (status == E_OK) {
        if (Os_HostAtomicCompareExchange(&slot->state, 2, 1) == 1) {
            *record = slot->record;
            (void)Os_HostAtomicExchange(&slot->state, 0);
            protocol_read = (protocol_read + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
        } else {
            status = E_OS_NOFUNC;
        }
    }
    return status;
}
StatusType Ecu_TargetPrepare(void)
{
    StatusType status;
    if (Os_HostAtomicCompareExchange(&lifecycle,
                                     (Os_Atomic32Type)ECU_TARGET_INITIALIZING,
                                     (Os_Atomic32Type)ECU_TARGET_UNPREPARED) !=
        (Os_Atomic32Type)ECU_TARGET_UNPREPARED) {
        return E_OS_STATE;
    }
    status = Os_TargetPrepare(&Ecu_OsConfig);
    if (status != E_OK) {
        (void)Os_HostAtomicExchange(&lifecycle, (Os_Atomic32Type)ECU_TARGET_FAILED);
    }
    return status;
}
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
static void stage(unsigned number)
{
    if (Ecu_TargetTestFailStage(number) != 0) {
        fail();
    }
}
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
void StartupHook(void)
{
    initialization_thread = Os_HostThreadIdentity();
    if (Ecu_StackBootstrap(&bootstrap_reached) != E_OK) {
        fail();
        return;
    }
    (void)Os_HostAtomicExchange(&lifecycle, (Os_Atomic32Type)ECU_TARGET_READY);
}

#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"

#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
void ShutdownHook(StatusType Error)
{
    Os_Atomic32Type state =
        (Error == E_OK) ? (Os_Atomic32Type)ECU_TARGET_CLOSED : (Os_Atomic32Type)ECU_TARGET_FAILED;
    Ecu_StackRetire(bootstrap_reached);
    (void)Os_HostAtomicExchange(&lifecycle, state);
    Ecu_TargetTestShutdown(Error);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
void ShutdownHook(StatusType Error)
{
    Os_Atomic32Type state =
        (Error == E_OK) ? (Os_Atomic32Type)ECU_TARGET_CLOSED : (Os_Atomic32Type)ECU_TARGET_FAILED;
    Ecu_StackRetire(bootstrap_reached);
    (void)Os_HostAtomicExchange(&lifecycle, state);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
StatusType Ecu_TargetPostFrame(
    uint64_t at, uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t *ticket)
{
    Ecu_Input input = {0};
    StatusType status;
    if ((data == NULL) || (ticket == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if ((id > 0x7ffu) || (dlc == 0u) || (dlc > 8u)) {
        return E_OS_VALUE;
    }
    status = native_context();
    if (status == E_OK) {
        Os_TickCompletionType completed;
        if (at < native_input_epoch) {
            return E_OS_VALUE;
        }
        status = Os_TargetTickCompletion(at, &completed);
        if ((status == E_OS_ID) && (at != UINT64_C(0))) {
            status = Os_TargetTickCompletion(at - UINT64_C(1), &completed);
        }
        if (status != E_OK) {
            return (status == E_OS_ID) ? E_OS_VALUE
                                       : ((status == E_OS_NOFUNC) ? E_OS_STATE : status);
        }
        input.kind = 1u;
        input.epoch = at;
        input.can_id = id;
        input.dlc = dlc;
        (void)memcpy(input.data, data, dlc);
        status = Os_TargetPostInput((const uint8_t *)&input, (uint8_t)sizeof(input), ticket);
        if (status == E_OK) {
            native_input_epoch = at;
        }
    }
    return status;
}
static StatusType batch_time(uint64_t at, Os_TickCompletionType *completed)
{
    StatusType status;
    if (at < native_input_epoch) {
        return E_OS_VALUE;
    }
    status = Os_TargetTickCompletion(at, completed);
    if ((status == E_OS_ID) && (at != UINT64_C(0))) {
        status = Os_TargetTickCompletion(at - UINT64_C(1), completed);
    }
    return (status == E_OS_ID) ? E_OS_VALUE : ((status == E_OS_NOFUNC) ? E_OS_STATE : status);
}
static int frame_shape(const Ecu_BatchFrame *frame)
{
    return Ecu_StackFrameShape(frame->id, frame->dlc);
}
StatusType Ecu_TargetValidateFrames(const Ecu_BatchFrame *frames, uint16_t count)
{
    uint16_t index;
    if ((frames == NULL) && (count != 0u)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if (count > ECU_BATCH_CAPACITY) {
        return E_OS_LIMIT;
    }
    for (index = 0u; index < count; ++index) {
        if (frame_shape(&frames[index]) == 0) {
            return E_OS_VALUE;
        }
    }
    return E_OK;
}
void Ecu_TargetAbortNative(StatusType reason)
{
    StatusType failure = (reason == E_OK) ? E_OS_STATE : reason;
    (void)Os_HostAtomicExchange(&lifecycle, (Os_Atomic32Type)ECU_TARGET_FAILED);
    Os_BackendRequestShutdown(failure);
    /* Fault controllers normally close first; blocked host IO must not keep
     * this failed process alive after the native watchdog has expired. */
    Os_HostSleepMs(600u);
    Os_HostExit((uint32_t)failure);
}
StatusType Ecu_TargetPostBatch(uint64_t at,
                               const Ecu_BatchFrame *frames,
                               uint16_t count,
                               uint64_t *ticket)
{
    Os_TickCompletionType completed;
    Ecu_Input input = {0};
    uint64_t mailbox_ticket;
    StatusType status;
    if (ticket == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = Ecu_TargetValidateFrames(frames, count);
    if (status != E_OK) {
        return status;
    }
    status = native_context();
    if (status != E_OK) {
        return status;
    }
    status = batch_time(at, &completed);
    if (status != E_OK) {
        return status;
    }
    if (native_batch_ticket == UINT64_MAX) {
        return E_OS_LIMIT;
    }
    if (Os_HostAtomicCompareExchange(&batch_state, 1, 0) != 0) {
        return E_OS_STATE;
    }
    batch_epoch = at;
    batch_count = count;
    batch_needs_tick = (at != completed.epoch) ? 1u : 0u;
    if (count != 0u) {
        (void)memcpy((void *)batch_frames, (const void *)frames, (size_t)count * sizeof(frames[0]));
    }
    batch_completion.ticket = native_batch_ticket + UINT64_C(1);
    batch_completion.epoch = at;
    batch_completion.input_count = count;
    batch_completion.input_status = ECU_OK;
    input.kind = 3u;
    input.epoch = at;
    input.output_ticket = batch_completion.ticket;
    (void)Os_HostAtomicExchange(&batch_state, 2);
    status = Os_TargetPostInput((const uint8_t *)&input, (uint8_t)sizeof(input), &mailbox_ticket);
    if (status == E_OK) {
        native_batch_ticket = batch_completion.ticket;
        native_input_epoch = at;
        *ticket = native_batch_ticket;
    } else {
        (void)Os_HostAtomicExchange(&batch_state, 0);
    }
    return status;
}
StatusType Ecu_TargetBatchCompletion(uint64_t ticket, Ecu_BatchCompletion *result)
{
    StatusType status;
    if (result == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_context();
    if (status != E_OK) {
        return status;
    }
    if ((ticket == UINT64_C(0)) || (ticket != native_batch_ticket)) {
        return E_OS_ID;
    }
    if (load(&batch_state) != 5) {
        return E_OS_NOFUNC;
    }
    *result = batch_completion;
    (void)Os_HostAtomicExchange(&batch_state, 0);
    return E_OK;
}
sint32 Os_IntegrationTimingAuthorized(void) { return (Ecu_SchMTimingActive() == TRUE) ? 1 : 0; }
void Os_IntegrationOnWaiting(TaskType id, EventMaskType pending, EventMaskType predicate)
{
    if ((id == ECU_TARGET_TASK) && (pending == 0u) && ((predicate & ECU_TARGET_EVENT_IO) != 0u)) {
        if ((Os_MailboxQuiescent() != 0) && (output_pending == 0u)) {
            if ((load(&batch_state) == 4) &&
                ((batch_needs_tick == 0u) || (processed_tick == batch_epoch))) {
                (void)Os_HostAtomicCompareExchange(&batch_state, 5, 4);
            }
        }
    }
}
EcuStatus Ecu_TargetEnqueueTransmit(PduIdType pdu, uint32_t id, uint8_t dlc, const uint8_t data[8])
{
    static uint64_t
        next_output_ticket;       /**< Owner-only monotonic output identity; initially zero. */
    static unsigned output_write; /**< Owner-only next publication slot; initially zero. */
    Ecu_OutputSlot *slot = &outputs[output_write];
    if (Ecu_TargetIsOwner() == 0) {
        return ECU_ERR_IO;
    }
    boolean writable =
        (dlc <= 8u) && (data != NULL) && (id <= 0x7ffu) && (next_output_ticket != UINT64_MAX);
    if (writable) {
        writable = load(&slot->state) == 0;
    }
    if (!writable) {
        fail();
        return ECU_ERR_IO;
    }
    slot->driver_token = Can_HostTransmitToken();
    if (slot->driver_token == UINT64_C(0)) {
        fail();
        return ECU_ERR_IO;
    }
    ++next_output_ticket;
    slot->record.ticket = next_output_ticket;
    slot->record.epoch = epoch;
    slot->record.pdu = pdu;
    slot->record.can_id = id;
    slot->record.dlc = dlc;
    (void)memset(slot->record.data, 0, sizeof(slot->record.data));
    if (dlc != 0u) {
        (void)memcpy(slot->record.data, data, dlc);
    }
    ++output_pending;
    output_write = (output_write + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    (void)Os_HostAtomicExchange(&slot->state, 1);
    /* The tick cannot complete before this confirmation; a failed wake only delays it
     * to the host's timed wait, and a signal failure already closes the target. */
    (void)Os_TargetNotifyNative();
    return ECU_OK;
}
StatusType Ecu_TargetTakeOutput(Ecu_OutputRecord *record)
{
    static unsigned output_read; /**< Native-only next take slot; initially zero. */
    StatusType status;
    Ecu_OutputSlot *slot = &outputs[output_read];
    if (record == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_context();
    if (status == E_OK) {
        if (Os_HostAtomicCompareExchange(&slot->state, 2, 1) != 1) {
            status = E_OS_NOFUNC;
        } else {
            *record = slot->record;
            output_read = (output_read + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
        }
    }
    return status;
}
StatusType Ecu_TargetConfirmOutput(uint64_t ticket, PduIdType pdu)
{
    static unsigned output_confirm; /**< Native-only next confirmation slot; initially zero. */
    Ecu_OutputSlot *slot = &outputs[output_confirm];
    Ecu_Input input = {0};
    uint64_t mailbox_ticket;
    StatusType status = native_context();
    if (status != E_OK) {
        return status;
    }
    if ((load(&slot->state) != 2) || (slot->record.ticket != ticket) || (slot->record.pdu != pdu)) {
        return E_OS_ID;
    }
    if (Os_HostAtomicCompareExchange(&slot->state, 3, 2) != 2) {
        return E_OS_STATE;
    }
    input.kind = 2u;
    input.output_ticket = ticket;
    input.pdu = pdu;
    status = Os_TargetPostInput((const uint8_t *)&input, (uint8_t)sizeof(input), &mailbox_ticket);
    if (status == E_OK) {
        output_confirm = (output_confirm + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    } else if (Os_HostAtomicCompareExchange(&slot->state, 2, 3) != 3) {
        fail();
    } else {
        /* Mailbox refusal keeps the taken output available for a retry. */
    }
    return status;
}
/* Completes the oldest confirmed output; its ticket and PDU are already proven. */
static void retire_output(void)
{
    Ecu_OutputSlot *slot = &outputs[output_retire];
    const uint64_t driver_token = slot->driver_token;
    const PduIdType pdu = slot->record.pdu;
    --output_pending;
    output_retire = (output_retire + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    (void)Os_HostAtomicExchange(&slot->state, 0);
    /* The output ticket proof establishes real write/flush completion.
     * A cancelled token is discarded even after the same PDU ID is reused. */
    if (Can_HostCompleteTransmit(pdu, driver_token) == CAN_BUSY) {
        fail();
    }
    Can_MainFunction_Write();
}
static void consume(const Ecu_Input *input)
{
    if (input->kind == 1u) {
        if ((input->epoch < epoch) || ((input->epoch - epoch) > UINT64_C(1))) {
            fail();
        }
        epoch = input->epoch;
        /* Single-frame control has no receipt; only batches report input status. */
        (void)Can_Inject(input->can_id, input->dlc, input->data, epoch);
        Ecu_ComReceptionBeforeMain(input->epoch > processed_epoch);
        Ecu_CanTpReceptionBeforeMain(input->epoch > processed_epoch);
        Can_MainFunction_Read();
        Ecu_CanTpReceptionBeforeMain(FALSE);
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
        stage(11u);
#endif
        if (Can_HostFlush() != ECU_OK) {
            fail();
        }
    } else if (input->kind == 2u) {
        Ecu_OutputSlot *slot =
            &outputs[(output_retire + output_deferred) % ECU_TARGET_OUTPUT_CAPACITY];
        if ((load(&slot->state) != 3) || (slot->record.ticket != input->output_ticket) ||
            (slot->record.pdu != input->pdu) || (output_deferred >= output_pending)) {
            fail();
        }
        if (epoch > processed_epoch) {
            /* A frame sent before this epoch's cycle completes on the bus after
             * it, whenever the host thread happens to confirm it. */
            ++output_deferred;
        } else {
            retire_output();
        }
    } else if (input->kind == 3u) {
        uint16_t index;
        if ((load(&batch_state) != 2) || (input->output_ticket != batch_completion.ticket) ||
            (input->epoch != batch_epoch) || (batch_epoch < epoch) ||
            ((batch_epoch - epoch) > UINT64_C(1))) {
            fail();
        }
        (void)Os_HostAtomicExchange(&batch_state, 3);
        epoch = batch_epoch;
        for (index = 0u; index < batch_count; ++index) {
            EcuStatus status = Can_Inject(
                batch_frames[index].id, batch_frames[index].dlc, batch_frames[index].data, epoch);
            Ecu_ComReceptionBeforeMain(batch_epoch > processed_epoch);
            Ecu_CanTpReceptionBeforeMain(batch_epoch > processed_epoch);
            Can_MainFunction_Read();
            Ecu_CanTpReceptionBeforeMain(FALSE);
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
            stage(11u);
#endif
            if (Can_HostFlush() != ECU_OK) {
                fail();
            }
            if ((status != ECU_OK) && (batch_completion.input_status == ECU_OK)) {
                batch_completion.input_status = status;
            }
        }
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
        stage(12u);
#endif
        (void)Os_HostAtomicExchange(&batch_state, 4);
    } else {
        fail();
    }
}
void Ecu_TargetTask(void)
{
    const EventMaskType mask = ECU_TARGET_EVENT_WORK | ECU_TARGET_EVENT_APP | ECU_TARGET_EVENT_IO;
    for (;;) {
        EventMaskType events;
        Os_InputRecordType record;
        uint64_t ticket;
        uint64_t delivered;
        unsigned count;
        StatusType status;
        Ecu_TargetAssertOwner();
        status = GetEvent(ECU_TARGET_TASK, &events);
        if (status == E_OK) {
            status = ClearEvent(events & mask);
        }
        if (status != E_OK) {
            fail();
        }
        status = Os_TargetTakeInput(&record);
        while (status == E_OK) {
            Ecu_MailboxInput input;
            if (record.length != sizeof(input[0])) {
                fail();
            }
            (void)memcpy((void *)&input[0], (const void *)record.data, sizeof(input[0]));
            consume(&input[0]);
            status = Os_TargetTakeInput(&record);
        }
        if (status != E_OS_NOFUNC) {
            fail();
        }
        status = Os_TargetCurrentTick(&ticket, &delivered);
        if ((status == E_OK) && (ticket != processed_tick)) {
            EventMaskType tick_events;
            /* A tick may arrive while this Task drains copied inputs. Its
             * alarm events are published before the delivered ticket, so
             * refresh the hints instead of losing this tick's application. */
            StatusType event_status = GetEvent(ECU_TARGET_TASK, &tick_events);
            if (event_status == E_OK) {
                event_status =
                    ClearEvent(tick_events & (ECU_TARGET_EVENT_WORK | ECU_TARGET_EVENT_APP));
            }
            if (event_status != E_OK) {
                fail();
            }
            events |= tick_events;
            if (delivered < epoch) {
                fail();
            }
            epoch = delivered;
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
            stage(9u);
#endif
            if (Ecu_StackRunCycle(events) != E_OK) {
                fail();
            }
#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)
            stage(10u);
#endif
            processed_tick = ticket;
            processed_epoch = delivered;
            while (output_deferred > 0u) {
                --output_deferred;
                retire_output();
            }
        } else if ((status != E_OK) && (status != E_OS_NOFUNC)) {
            fail();
        } else {
            /* No new tick; an already processed tick is not run again. */
        }
        if ((status == E_OK) && (output_pending == 0u)) {
            if (Os_TargetCompleteTick(ticket) != E_OK) {
                fail();
            }
        }
        if (Os_TargetInputCount(&count) != E_OK) {
            fail();
        }
        if (count == 0u) {
            if (WaitEvent(mask) != E_OK) {
                fail();
            }
        }
    }
}
