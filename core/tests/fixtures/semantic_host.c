/* Independent consumer of the delivered synchronous profile, not generated expectations. */
#include "Ecu_Runtime.h"
#include "CanIf.h"
#include "CanTp.h"
#include "Com.h"
#include <stdio.h>
#include <string.h>

extern const EcuConfig Ecu_Config;
static uint8_t outputs[16][8];
static uint8_t lengths[16];
static size_t count;
static int remove_confirmation;
static int failed;

static void require_at(int accepted, unsigned line) {
    if (accepted == 0) {
        (void)fprintf(stderr, "synchronous profile assertion: line=%u\n", line);
        failed = 1;
    }
}
#define require(accepted) require_at((accepted), __LINE__)

static EcuStatus sink(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    if ((printf("synchronous_output id=%u dlc=%u\n", id, dlc) < 0) || (fflush(stdout) != 0)) {
        return ECU_ERR_IO;
    }
    if (id == 0x708u) {
        if (count >= 16u) {
            return ECU_ERR_IO;
        }
        memcpy(outputs[count], data, 8u);
        lengths[count] = dlc;
        ++count;
    }
    if (remove_confirmation != 0) {
        /* Actual callback reentrancy removes CanIf before the real driver's confirmation. */
        CanIf_Init(NULL);
    }
    return ECU_OK;
}

static EcuStatus receive(const uint8_t bytes[8], uint64_t now) {
    return Can_Inject(0x700u, 8u, bytes, now);
}

static void frame(size_t index, uint8_t dlc, const uint8_t expected[8]) {
    require((index < count) && (lengths[index] == dlc));
    if (index < count) {
        require(memcmp(outputs[index], expected, dlc) == 0);
    }
}

int main(void) {
    static const uint8_t read_did[8] = {3u, 0x22u, 0x12u, 0x34u, 0u, 0u, 0u, 0u};
    static const uint8_t denied[8] = {3u, 0x7fu, 0x22u, 0x31u, 0u, 0u, 0u, 0u};
    static const uint8_t extended[8] = {2u, 0x10u, 3u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t session_reply[8] = {6u, 0x50u, 3u, 0u, 50u, 0u, 50u, 0u};
    static const uint8_t value[8] = {7u, 0x62u, 0x12u, 0x34u, 0x11u, 0x22u, 0x33u, 0x44u};
    static const uint8_t ordered[8] = {5u, 0x22u, 0xf1u, 0x86u, 0x12u, 0x34u, 0u, 0u};
    static const uint8_t first[8] = {0x10u, 10u, 0x62u, 0xf1u, 0x86u, 3u, 0x12u, 0x34u};
    static const uint8_t tail[8] = {0x21u, 0x11u, 0x22u, 0x33u, 0x44u, 0u, 0u, 0u};
    static const uint8_t wait[8] = {0x31u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t clear[8] = {0x30u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    static const uint8_t three[8] = {7u, 0x22u, 0xf1u, 0x86u, 0xf1u, 0x86u, 0xf1u, 0x86u};
    static const uint8_t three_first[8] = {0x10u, 10u, 0x62u, 0xf1u, 0x86u, 3u, 0xf1u, 0x86u};
    static const uint8_t three_tail[8] = {0x21u, 3u, 0xf1u, 0x86u, 3u, 0u, 0u, 0u};
    static const uint8_t request_first[8] = {0x10u, 9u, 0x22u, 0xf1u, 0x86u, 0xf1u, 0x86u, 0xf1u};
    static const uint8_t request_tail[8] = {0x21u, 0x86u, 0xf1u, 0x86u, 0u, 0u, 0u, 0u};
    static const uint8_t info[8] = {3u, 0x22u, 0xf1u, 0x86u, 0u, 0u, 0u, 0u};
    static const uint8_t info_reply[8] = {4u, 0x62u, 0xf1u, 0x86u, 3u, 0u, 0u, 0u};
    static const uint8_t too_long[8] = {3u, 0x7fu, 0x22u, 0x14u, 0u, 0u, 0u, 0u};
    uint8_t large_request[87];
    uint8_t fragment[8];
    size_t offset;
    size_t index;
    uint8_t sequence;
    size_t tx;
    uint64_t now;
    require(Ecu_Init(&Ecu_Config, sink, NULL, NULL, NULL) == ECU_OK);
    for (tx = 0u; tx < Ecu_Config.frame_count; ++tx) {
        if (Ecu_Config.frames[tx].id == 0x500u) {
            break;
        }
    }
    require(tx < Ecu_Config.frame_count);
    require(Com_TriggerTransmit(tx) == ECU_OK);
    remove_confirmation = 1;
    require(Com_TriggerTransmit(tx) == ECU_ERR_IO);
    remove_confirmation = 0;
    CanIf_Init(&Ecu_Config);
    CanIf_ControllerModeIndication(0u, CAN_CS_STARTED);
    require(Com_TriggerTransmit(tx) == ECU_OK);

    require(receive(read_did, 0u) == ECU_OK);
    frame(0u, 4u, denied);
    require(receive(extended, 1u) == ECU_OK);
    frame(1u, 7u, session_reply);
    require(receive(read_did, 2u) == ECU_OK);
    frame(2u, 8u, value);
    require(receive(ordered, 3u) == ECU_OK);
    frame(3u, 8u, first);
    now = 3u + Ecu_Config.diagnostic->n_bs_ms - 1u;
    require(receive(wait, now) == ECU_OK);
    require(CanTp_AdvanceTime(now + 1u) == ECU_OK);
    require(receive(clear, now + 1u) == ECU_OK);
    frame(4u, 5u, tail);
    require(receive(three, now + 2u) == ECU_OK);
    frame(5u, 8u, three_first);
    require(receive(clear, now + 2u) == ECU_OK);
    frame(6u, 5u, three_tail);

    require(receive(request_first, now + 3u) == ECU_OK);
    require(receive(request_tail, now + 3u + Ecu_Config.diagnostic->n_cr_ms) == ECU_ERR_TP_TIMEOUT);
    require(count == 8u); /* Only the actual FC was emitted; expired payload never reaches DCM. */
    require(receive(info, now + 4u + Ecu_Config.diagnostic->n_cr_ms) == ECU_OK);
    frame(8u, 5u, info_reply);
    require(count == 9u);
    /* Forty-three valid DIDs fit the Rx buffer but exceed the actual 256-byte response buffer. */
    large_request[0] = 0x22u;
    for (index = 1u; index < sizeof(large_request); ++index) {
        large_request[index] = ((index & 1u) != 0u) ? 0x12u : 0x34u;
    }
    fragment[0] = 0x10u;
    fragment[1] = (uint8_t)sizeof(large_request);
    memcpy(&fragment[2], large_request, 6u);
    now += Ecu_Config.diagnostic->n_cr_ms + 5u;
    require(receive(fragment, now) == ECU_OK);
    offset = 6u;
    sequence = 1u;
    while (offset < sizeof(large_request)) {
        size_t length = sizeof(large_request) - offset;
        if (length > 7u) {
            length = 7u;
        }
        memset(fragment, 0, sizeof(fragment));
        fragment[0] = (uint8_t)(0x20u | sequence);
        memcpy(&fragment[1], &large_request[offset], length);
        ++now;
        require(receive(fragment, now) == ECU_OK);
        offset += length;
        sequence = (uint8_t)((sequence + 1u) & 0x0fu);
    }
    frame(10u, 4u, too_long);
    require(receive(info, now + 1u) == ECU_OK);
    frame(11u, 5u, info_reply);
    require(count == 12u);
    if (failed != 0) {
        return 1;
    }
    puts("SEMANTIC_HOST PASS confirmation/io session/p2 order/three-did dlc/wait deadline/recovery "
         "overflow/recovery");
    return 0;
}
