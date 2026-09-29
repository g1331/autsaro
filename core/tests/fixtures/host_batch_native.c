#include "Ecu_Target.h"
#include "Os_Windows.h"
#include <stdio.h>
#include <string.h>

static Ecu_BatchFrame frames[256];
static unsigned signal_outputs;
static unsigned did_outputs;

static void require(int accepted) {
    if (accepted == 0) {
        ShutdownOS(E_OS_STATE);
    }
}

static void outputs(void) {
    Ecu_OutputRecord output;
    StatusType status = Ecu_TargetTakeOutput(&output);
    if (status == E_OK) {
        static const uint8_t expected[4] = {0x78u, 0x56u, 0x34u, 0x12u};
        static const uint8_t did[8] = {7u, 0x62u, 0x12u, 0x34u, 0x12u, 0x34u, 0x56u, 0x78u};
        require((output.epoch == UINT64_C(10)) || (output.epoch == UINT64_C(11)));
        if (output.can_id == 0x321u) {
            require((output.dlc == 4u) && (memcmp(output.data, expected, 4u) == 0));
            ++signal_outputs;
        } else {
            require(printf("batch_did bytes=%02x%02x%02x%02x%02x%02x%02x%02x\n", output.data[0],
                           output.data[1], output.data[2], output.data[3], output.data[4],
                           output.data[5], output.data[6], output.data[7]) >= 0);
            require((output.can_id == 0x708u) && (output.dlc == 8u) &&
                    (memcmp(output.data, did, 8u) == 0));
            ++did_outputs;
        }
        require(Ecu_TargetConfirmOutput(output.ticket, (PduIdType)7u) == E_OS_ID);
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OK);
        require(Ecu_TargetConfirmOutput(output.ticket, output.pdu) == E_OS_ID);
    } else {
        require(status == E_OS_NOFUNC);
    }
}

static Ecu_BatchCompletion wait_batch(uint64_t ticket) {
    Ecu_BatchCompletion result;
    DWORD started = GetTickCount();
    for (;;) {
        StatusType status;
        outputs();
        status = Ecu_TargetBatchCompletion(ticket, &result);
        if (status == E_OK) {
            return result;
        }
        require((status == E_OS_NOFUNC) && ((GetTickCount() - started) < 5000u));
        Sleep(1u);
    }
}

static void tick(uint64_t at) {
    Os_TickCompletion complete;
    uint64_t ticket;
    DWORD started = GetTickCount();
    require(Os_TargetAdvanceOneTick(at, &ticket) == E_OK);
    for (;;) {
        StatusType status;
        outputs();
        status = Os_TargetTickCompletion(ticket, &complete);
        if (status == E_OK) {
            require(complete.epoch == at);
            return;
        }
        require((status == E_OS_NOFUNC) && ((GetTickCount() - started) < 5000u));
        Sleep(1u);
    }
}

static DWORD WINAPI control(void *argument) {
    uint64_t ticket;
    uint64_t untouched = UINT64_C(42);
    Ecu_BatchCompletion complete;
    unsigned index;
    DWORD started = GetTickCount();
    (void)argument;
    while (Ecu_TargetState() != ECU_TARGET_READY) {
        require((GetTickCount() - started) < 5000u);
        Sleep(1u);
    }
    for (index = 0u; index < 256u; ++index) {
        frames[index].id = 0x320u;
        frames[index].dlc = 4u;
        frames[index].data[0] = (uint8_t)index;
    }
    require(Ecu_TargetPostBatch(UINT64_C(0), frames, 257u, &untouched) == E_OS_LIMIT);
    frames[255].dlc = 3u;
    require(Ecu_TargetPostBatch(UINT64_C(0), frames, 256u, &untouched) == E_OS_VALUE);
    frames[255].dlc = 4u;
    require(untouched == UINT64_C(42));
    require(Ecu_TargetPostBatch(UINT64_C(0), frames, 256u, &ticket) == E_OK);
    require(ticket == UINT64_C(1));
    require(Ecu_TargetPostBatch(UINT64_C(0), NULL, 0u, &untouched) == E_OS_STATE);
    require(untouched == UINT64_C(42));
    complete = wait_batch(ticket);
    require((complete.input_count == 256u) && (complete.epoch == UINT64_C(0)) &&
            (complete.input_status == ECU_OK));
    require(Ecu_TargetPostBatch(UINT64_C(0), NULL, 0u, &ticket) == E_OK);
    require(ticket == UINT64_C(2));
    complete = wait_batch(ticket);
    require((complete.input_count == 0u) && (complete.epoch == UINT64_C(0)));
    frames[0].data[0] = 0x78u;
    frames[0].data[1] = 0x56u;
    frames[0].data[2] = 0x34u;
    frames[0].data[3] = 0x12u;
    require(Ecu_TargetPostBatch(UINT64_C(1), frames, 1u, &ticket) == E_OK);
    Sleep(20u);
    require(Ecu_TargetBatchCompletion(ticket, &complete) == E_OS_NOFUNC);
    tick(UINT64_C(1));
    complete = wait_batch(ticket);
    require((complete.input_count == 1u) && (complete.epoch == UINT64_C(1)) &&
            (complete.input_status == ECU_OK));
    for (index = 2u; index < 10u; ++index) {
        tick((uint64_t)index);
    }
    frames[1].id = 0x700u;
    frames[1].dlc = 8u;
    (void)memset(frames[1].data, 0, sizeof(frames[1].data));
    frames[1].data[0] = 3u;
    frames[1].data[1] = 0x22u;
    frames[1].data[2] = 0x12u;
    frames[1].data[3] = 0x34u;
    require(Ecu_TargetPostBatch(UINT64_C(10), frames, 2u, &ticket) == E_OK);
    tick(UINT64_C(10));
    complete = wait_batch(ticket);
    require((complete.input_status == ECU_OK) && (complete.input_count == 2u));
    require((signal_outputs == 1u) && (did_outputs == 1u));
    /* The copied complete request still owns the physical connection until
     * dispatch/confirmation; a later FF cannot steal its receive buffer. */
    frames[0] = frames[1];
    frames[1].data[0] = 0x10u;
    frames[1].data[1] = 9u;
    frames[1].data[2] = 0x22u;
    frames[1].data[3] = 0x12u;
    frames[1].data[4] = 0x34u;
    frames[1].data[5] = 0x12u;
    frames[1].data[6] = 0x34u;
    frames[1].data[7] = 0x12u;
    require(Ecu_TargetPostBatch(UINT64_C(10), frames, 2u, &ticket) == E_OK);
    complete = wait_batch(ticket);
    require(complete.input_status == ECU_ERR_TP_BUSY);
    require((signal_outputs == 1u) && (did_outputs == 2u));
    require(Ecu_TargetPostBatch(UINT64_C(11), frames, 1u, &ticket) == E_OK);
    tick(UINT64_C(11));
    complete = wait_batch(ticket);
    require(complete.input_status == ECU_OK);
    require((signal_outputs == 1u) && (did_outputs == 3u));
    require(printf("native_batch capacity=256 refusal=257 same_epoch=0 future_requires_tick=pass "
                   "confirmations=pass\n") >= 0);
    ShutdownOS(E_OK);
    return 0u;
}

int main(void) {
    HANDLE thread;
    require(Ecu_TargetPrepare() == E_OK);
    thread = CreateThread(NULL, 262144u, control, NULL, 0u, NULL);
    require(thread != NULL);
    require(CloseHandle(thread) != 0);
    StartOS(1u);
    return 94;
}
