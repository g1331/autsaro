#include "Ecu_Target.h"
#include "Ecu_TargetConfig.h"
#include "Os_Backend.h"
#include "Can.h"
#include "CanIf.h"
#include "CanTp.h"
#include "Com.h"
#include "Dcm.h"
#include "Dem.h"
#include "LSduR.h"
#include "Os_Mailbox.h"
#include "PduR.h"
#include "Rte.h"
#include "Security.h"
#include "SchM_Can.h"
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
    volatile LONG state;
    Ecu_OutputRecord record;
} Ecu_OutputSlot;

static volatile LONG lifecycle;
static volatile LONG native_owner;
static DWORD native_thread;
/* Accessed only by the native producer after its atomic identity publication. */
static uint64_t native_input_epoch;
static DWORD initialization_thread;
static uint64_t epoch;
static uint64_t processed_tick;
static uint64_t next_output_ticket;
static unsigned output_write;
static unsigned output_read;
static unsigned output_confirm;
static unsigned output_retire;
static unsigned output_pending;
static Ecu_OutputSlot outputs[ECU_TARGET_OUTPUT_CAPACITY];
static uint8_t received;
static uint64_t received_at;

static void fail(void) {
    (void)InterlockedExchange(&lifecycle, (LONG)ECU_TARGET_FAILED);
    ShutdownOS(E_OS_STATE);
}
static LONG load(volatile LONG *value) { return InterlockedCompareExchange(value, 0, 0); }
uint8_t Ecu_TargetState(void) {
    LONG state = load(&lifecycle);
    if ((state == (LONG)ECU_TARGET_READY) && (Os_TargetReady() == 0)) {
        return ECU_TARGET_INITIALIZING;
    }
    return (uint8_t)state;
}
int Ecu_TargetIsOwner(void) {
    if (load(&lifecycle) == (LONG)ECU_TARGET_INITIALIZING) {
        return (initialization_thread != 0u) && (GetCurrentThreadId() == initialization_thread);
    }
    return (Ecu_TargetState() == ECU_TARGET_READY) && (Os_BackendTaskOwner(ECU_TARGET_TASK) == 1);
}
void Ecu_TargetAssertOwner(void) {
    if (Ecu_TargetIsOwner() == 0) {
        fail();
    }
}
uint64_t Ecu_TargetNow(void) {
    Ecu_TargetAssertOwner();
    return epoch;
}
static StatusType native_context(void) {
    LONG owner;
    if (Ecu_TargetState() != ECU_TARGET_READY) {
        return E_OS_STATE;
    }
    if (Ecu_TargetIsOwner() != 0) {
        return E_OS_CALLEVEL;
    }
    owner = InterlockedCompareExchange(&native_owner, 1, 0);
    if (owner == 0) {
        native_thread = GetCurrentThreadId();
        (void)InterlockedExchange(&native_owner, 2);
        return E_OK;
    }
    return ((owner == 2) && (native_thread == GetCurrentThreadId())) ? E_OK : E_OS_ACCESS;
}
StatusType Ecu_TargetPrepare(void) {
    StatusType status;
    if (InterlockedCompareExchange(&lifecycle, (LONG)ECU_TARGET_INITIALIZING, 0) != 0) {
        return E_OS_STATE;
    }
    status = Os_TargetPrepare(&Ecu_OsConfig);
    if (status != E_OK) {
        (void)InterlockedExchange(&lifecycle, (LONG)ECU_TARGET_FAILED);
    }
    return status;
}
static EcuStatus unexpected_sink(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    (void)id;
    (void)dlc;
    (void)data;
    fail();
    return ECU_ERR_IO;
}
static void stage(unsigned number) {
#ifdef ECU_TARGET_TESTS
    if (Ecu_TargetTestFailStage(number) != 0) {
        fail();
    }
#else
    (void)number;
#endif
}
void StartupHook(void) {
    const Can_ConfigType driver = {unexpected_sink};
    Can_ControllerStateType mode;
    initialization_thread = GetCurrentThreadId();
    stage(1u);
    if ((Dem_Init(&Ecu_Config, NULL) != ECU_OK) || (Security_Init(0, NULL, NULL) != ECU_OK)) {
        fail();
    }
    Os_TargetTrace('d');
    stage(2u);
    Can_Init(&driver);
    if ((Can_GetControllerMode(0u, &mode) != E_OK) || (mode != CAN_CS_STOPPED)) {
        fail();
    }
    CanIf_Init(&Ecu_Config);
    Os_TargetTrace('c');
    stage(3u);
    PduR_Init(&Ecu_Config);
    LSduR_Init(Ecu_Config.diagnostic);
    CanTp_Init(Ecu_Config.diagnostic);
    Os_TargetTrace('p');
    stage(4u);
    Com_Init(&Ecu_Config);
    Os_TargetTrace('m');
    stage(5u);
    Dcm_Init(Ecu_Config.diagnostic);
    Os_TargetTrace('g');
    stage(6u);
    if (Ecu_TargetInitializeRte() != E_OK) {
        fail();
    }
    Os_TargetTrace('r');
    stage(7u);
    if ((Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) ||
        (Can_GetControllerMode(0u, &mode) != E_OK) || (mode != CAN_CS_STARTED)) {
        fail();
    }
    Can_MainFunction_Wakeup();
    Os_TargetTrace('s');
    stage(8u);
    (void)InterlockedExchange(&lifecycle, (LONG)ECU_TARGET_READY);
}
void ShutdownHook(StatusType Error) {
    LONG state = (Error == E_OK) ? (LONG)ECU_TARGET_CLOSED : (LONG)ECU_TARGET_FAILED;
    (void)InterlockedExchange(&lifecycle, state);
#ifdef ECU_TARGET_TESTS
    Ecu_TargetTestShutdown(Error);
#endif
}
void Ecu_TargetRecordReceive(uint64_t at) {
    Ecu_TargetAssertOwner();
    received = 1u;
    received_at = at;
}
uint8_t Ecu_TargetReceiveStatus(void) {
    uint8_t result = RTE_E_NEVER_RECEIVED;
    Ecu_TargetAssertOwner();
    if (received != 0u) {
        result = ((epoch >= received_at) && ((epoch - received_at) >= ECU_TARGET_RX_DEADLINE_MS))
                     ? RTE_E_MAX_AGE_EXCEEDED
                     : E_OK;
    }
    return result;
}
StatusType Ecu_TargetPostFrame(uint64_t at, uint32_t id, uint8_t dlc, const uint8_t data[8],
                               uint64_t *ticket) {
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
        Os_TickCompletion completed;
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
EcuStatus Ecu_TargetEnqueueTransmit(PduIdType pdu, uint32_t id, uint8_t dlc,
                                    const uint8_t data[8]) {
    Ecu_OutputSlot *slot = &outputs[output_write];
    Ecu_TargetAssertOwner();
    if ((dlc == 0u) || (dlc > 8u) || (data == NULL) || (id > 0x7ffu) ||
        (next_output_ticket == UINT64_MAX) || (load(&slot->state) != 0)) {
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
    (void)memcpy(slot->record.data, data, dlc);
    ++output_pending;
    output_write = (output_write + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    (void)InterlockedExchange(&slot->state, 1);
    return ECU_OK;
}
StatusType Ecu_TargetTakeOutput(Ecu_OutputRecord *record) {
    StatusType status;
    Ecu_OutputSlot *slot = &outputs[output_read];
    if (record == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = native_context();
    if (status == E_OK) {
        if (InterlockedCompareExchange(&slot->state, 2, 1) != 1) {
            status = E_OS_NOFUNC;
        } else {
            *record = slot->record;
            output_read = (output_read + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
        }
    }
    return status;
}
StatusType Ecu_TargetConfirmOutput(uint64_t ticket, PduIdType pdu) {
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
    if (InterlockedCompareExchange(&slot->state, 3, 2) != 2) {
        return E_OS_STATE;
    }
    input.kind = 2u;
    input.output_ticket = ticket;
    input.pdu = pdu;
    status = Os_TargetPostInput((const uint8_t *)&input, (uint8_t)sizeof(input), &mailbox_ticket);
    if (status == E_OK) {
        output_confirm = (output_confirm + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
    } else if (InterlockedCompareExchange(&slot->state, 2, 3) != 3) {
        fail();
    } else {
        /* Mailbox refusal keeps the taken output available for a retry. */
    }
    return status;
}
static void consume(const Ecu_Input *input) {
    if (input->kind == 1u) {
        EcuStatus status;
        if ((input->epoch < epoch) || ((input->epoch - epoch) > UINT64_C(1))) {
            fail();
        }
        epoch = input->epoch;
        status = Can_Inject(input->can_id, input->dlc, input->data, epoch);
        if ((status == ECU_OK) && (input->can_id == ECU_TARGET_RX_CAN_ID)) {
            Ecu_TargetRecordReceive(epoch);
        }
    } else if (input->kind == 2u) {
        Ecu_OutputSlot *slot = &outputs[output_retire];
        if ((load(&slot->state) != 3) || (slot->record.ticket != input->output_ticket) ||
            (slot->record.pdu != input->pdu) || (output_pending == 0u)) {
            fail();
        }
        --output_pending;
        output_retire = (output_retire + 1u) % ECU_TARGET_OUTPUT_CAPACITY;
        (void)InterlockedExchange(&slot->state, 0);
        CanIf_TxConfirmation(input->pdu);
        if (CanTp_AdvanceTime(epoch) != ECU_OK) {
            fail();
        }
    } else {
        fail();
    }
}
void Ecu_TargetTask(void) {
    const EventMaskType mask = ECU_TARGET_EVENT_WORK | ECU_TARGET_EVENT_APP | ECU_TARGET_EVENT_IO;
    for (;;) {
        EventMaskType events;
        Os_InputRecord record;
        uint64_t ticket;
        uint64_t delivered;
        unsigned count;
        StatusType status;
        Ecu_TargetAssertOwner();
        if ((GetEvent(ECU_TARGET_TASK, &events) != E_OK) || (ClearEvent(events & mask) != E_OK)) {
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
            if (delivered < epoch) {
                fail();
            }
            epoch = delivered;
            Can_MainFunction_Wakeup();
            Os_TargetTrace('w');
            if (CanTp_AdvanceTime(epoch) != ECU_OK) {
                fail();
            }
            Os_TargetTrace('t');
            if (Com_AdvanceTime(epoch) != ECU_OK) {
                fail();
            }
            Os_TargetTrace('c');
            if ((events & ECU_TARGET_EVENT_APP) != 0u) {
                ECU_TARGET_RUN_APPLICATION();
                Os_TargetTrace('a');
                if (ECU_TARGET_TRANSMIT() != ECU_OK) {
                    fail();
                }
                Os_TargetTrace('x');
            }
            Dcm_AdvanceTime(epoch);
            Os_TargetTrace('d');
            processed_tick = ticket;
        } else if ((status != E_OK) && (status != E_OS_NOFUNC)) {
            fail();
        } else {
            /* An IO-only wake consumes callbacks without repeating periodic work. */
        }
        if ((status == E_OK) && (output_pending == 0u) && (Os_TargetCompleteTick(ticket) != E_OK)) {
            fail();
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
