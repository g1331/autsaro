/** Can driver contract oracle (SWS_CANDriver) with recording CanIf callbacks.
 * The driver and CanIf controller IDs differ, so every callback proves which one it carries. */
#include "Can.h"
#include "Can_Host.h"
#include "Can_Internal.h"
#include "CanIf.h"
#include "SchM_Can.h"
#include "Det.h"
#include "Ecu_Config.h"
#include <assert.h>
/* Verifies: SWS_Can_00415, SWS_Can_00416, SWS_Can_00429, SWS_CAN_00496, SWS_Can_91013 */
/* Verifies: SWS_Can_91003, SWS_Can_00039, SWS_Can_91020 */
typedef char id_is_uint32[(sizeof(Can_IdType) == 4u) ? 1 : -1];
typedef char handle_is_uint16[(sizeof(Can_HwHandleType) == 2u) ? 1 : -1];
typedef char states_are_standard[((CAN_CS_UNINIT == 0) && (CAN_CS_STARTED == 1) &&
                                  (CAN_CS_STOPPED == 2) && (CAN_CS_SLEEP == 3))
                                     ? 1
                                     : -1];
typedef char
    error_states_are_standard[((CAN_ERRORSTATE_ACTIVE == 0) && (CAN_ERRORSTATE_PASSIVE == 1) &&
                               (CAN_ERRORSTATE_BUSOFF == 2))
                                  ? 1
                                  : -1];
typedef char codes_are_standard[((CAN_BUSY == 2u) && (CAN_E_DATALOST == 1u)) ? 1 : -1];
static unsigned frames;
static uint32 frame_id;
static uint8 frame_length;
static uint8 frame_data[8];
static unsigned modes;
static uint8 mode_controller;
static Can_ControllerStateType last_mode;
static unsigned busoffs;
static uint8 busoff_controller;
static unsigned confirmations;
static PduIdType confirmed;
static unsigned receptions;
static Can_HwType mailbox;
static uint8 received_length;
static uint8 received[8];
static unsigned lost;
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
static EcuStatus sink(uint32 id, uint8 length, const uint8 data[8])
{
    uint8 i;
    frame_id = id;
    frame_length = length;
    for (i = 0u; i < 8u; ++i) {
        frame_data[i] = data[i];
    }
    ++frames;
    return ECU_OK;
}
/* Verifies: SWS_Can_00234 */
void __wrap_CanIf_ControllerModeIndication(uint8 ControllerId,
                                           Can_ControllerStateType ControllerMode)
{
    mode_controller = ControllerId;
    last_mode = ControllerMode;
    ++modes;
}
void __wrap_CanIf_ControllerBusOff(uint8 ControllerId)
{
    Can_ControllerStateType actual = CAN_CS_STARTED;
    /* STOPPED is reached before the indication (SWS_Can_00020). */
    assert(Can_GetControllerMode(3u, &actual) == E_OK && actual == CAN_CS_STOPPED);
    busoff_controller = ControllerId;
    ++busoffs;
}
void __wrap_CanIf_TxConfirmation(PduIdType CanTxPduId)
{
    confirmed = CanTxPduId;
    ++confirmations;
}
void __wrap_CanIf_RxIndication(const Can_HwType *Mailbox, const PduInfoType *PduInfoPtr)
{
    uint8 i;
    assert(Mailbox != NULL_PTR && PduInfoPtr != NULL_PTR && PduInfoPtr->MetaDataPtr == NULL_PTR);
    mailbox = *Mailbox;
    received_length = (uint8)PduInfoPtr->SduLength;
    for (i = 0u; i < received_length; ++i) {
        received[i] = PduInfoPtr->SduDataPtr[i];
    }
    ++receptions;
}
static Std_ReturnType report_runtime(uint16 module, uint8 instance, uint8 api, uint8 error)
{
    assert(module == CAN_MODULE_ID && instance == 0u && api == 0x08u && error == CAN_E_DATALOST);
    ++lost;
    return E_OK;
}
static const Can_ConfigType can = {sink};
static Can_ControllerStateType mode_of(void)
{
    Can_ControllerStateType state = CAN_CS_UNINIT;
    assert(Can_GetControllerMode(3u, &state) == E_OK);
    return state;
}
static void polled_mode(Can_ControllerStateType expected)
{
    const unsigned before = modes;
    Can_MainFunction_Mode();
    assert(modes == before + 1u && mode_controller == 5u && last_mode == expected);
}
int main(void)
{
    /* Standard signatures (Verifies: SWS_Can_00223, SWS_Can_91002, SWS_Can_00230); input
     * pointers are const (Verifies[Can]: SWS_BSW_00186). */
    void (*init)(const Can_ConfigType *) = Can_Init;
    void (*deinit)(void) = Can_DeInit;
    Std_ReturnType (*set_mode)(uint8, Can_ControllerStateType) = Can_SetControllerMode;
    /* Verifies: SWS_Can_91014, SWS_Can_91004, SWS_Can_00511, SWS_Can_00516 */
    Std_ReturnType (*get_mode)(uint8, Can_ControllerStateType *) = Can_GetControllerMode;
    Std_ReturnType (*error_state)(uint8, Can_ErrorStateType *) = Can_GetControllerErrorState;
    Std_ReturnType (*rx_counter)(uint8, uint8 *) = Can_GetControllerRxErrorCounter;
    Std_ReturnType (*tx_counter)(uint8, uint8 *) = Can_GetControllerTxErrorCounter;
    /* Verifies: SWS_Can_00231, SWS_Can_00232, SWS_Can_00360, SWS_Can_00233 */
    void (*disable)(uint8) = Can_DisableControllerInterrupts;
    void (*enable)(uint8) = Can_EnableControllerInterrupts;
    Std_ReturnType (*check_wakeup)(uint8) = Can_CheckWakeup;
    Std_ReturnType (*write)(Can_HwHandleType, const Can_PduType *) = Can_Write;
    /* Verifies: SWS_Can_00225, SWS_Can_00226, SWS_Can_00227, SWS_Can_00228 */
    /* Verifies: SWS_Can_00368 */
    void (*main_functions[])(void) = {Can_MainFunction_Write,
                                      Can_MainFunction_Read,
                                      Can_MainFunction_BusOff,
                                      Can_MainFunction_Wakeup,
                                      Can_MainFunction_Mode};
    static const Det_ErrorHookType callouts[] = {report_runtime};
    static const Det_ConfigType det = {NULL_PTR, 0u, callouts, 1u};
    uint8 bytes[8] = {1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u};
    const uint8 incoming[8] = {9u, 10u, 11u, 12u, 13u, 14u, 15u, 16u};
    Can_PduType pdu = {18u, 8u, 0x123u, bytes};
    Can_ControllerStateType state = CAN_CS_SLEEP;
    Can_ErrorStateType error = CAN_ERRORSTATE_PASSIVE;
    uint8 counter = 0xa5u;
    uint8 i;
    Det_Init(&det);

    /* Verifies: SWS_Can_00103 */
    assert(get_mode(3u, &state) == E_NOT_OK && state == CAN_CS_SLEEP);
    /* Scheduled functions of the uninitialized driver return without action
     * (Verifies[Can]: SWS_BSW_00037). */
    assert(Can_Inject(0x321u, 1u, incoming, 0u) == ECU_ERR_CONTROLLER);
    for (i = 0u; i < 5u; ++i) {
        main_functions[i]();
    }
    assert(modes == 0u && busoffs == 0u && confirmations == 0u && receptions == 0u && lost == 0u);
    assert(set_mode(3u, CAN_CS_STARTED) == E_NOT_OK);
    assert(write(1u, &pdu) == E_NOT_OK);
    init(NULL_PTR);
    assert(get_mode(3u, &state) == E_NOT_OK);

    /* Verifies: SWS_Can_00246, SWS_Can_00245, SWS_Can_00259, SWS_Can_00250 */
    /* Verifies[Can]: SWS_BSW_00071 */
    init(&can);
    assert(mode_of() == CAN_CS_STOPPED && modes == 0u);
    /* Verifies: SWS_Can_91008, SWS_Can_91015 */
    assert(error_state(3u, &error) == E_OK && error == CAN_ERRORSTATE_ACTIVE);
    assert(get_mode(5u, &state) == E_NOT_OK && error_state(5u, &error) == E_NOT_OK);
    assert(get_mode(3u, NULL_PTR) == E_NOT_OK && error_state(3u, NULL_PTR) == E_NOT_OK);
    /* The virtual controller has no error counters. */
    assert(rx_counter(3u, &counter) == E_NOT_OK && tx_counter(3u, &counter) == E_NOT_OK);
    assert(counter == 0xa5u);
    assert(check_wakeup(3u) == E_NOT_OK);
    disable(3u);
    enable(3u);
    enable(3u);
    assert(write(1u, &pdu) == E_NOT_OK && frames == 0u);

    /* Software transitions, indicated by the mode main function
     * (Verifies: SWS_Can_00017, SWS_Can_00261, SWS_Can_00370, SWS_Can_00373, SWS_Can_00369). */
    assert(set_mode(3u, CAN_CS_STARTED) == E_OK && modes == 0u);
    polled_mode(CAN_CS_STARTED);
    assert(mode_of() == CAN_CS_STARTED);
    Can_MainFunction_Mode();
    assert(modes == 1u);
    /* Verifies: SWS_Can_00409, SWS_Can_00411 */
    assert(set_mode(3u, CAN_CS_STARTED) == E_NOT_OK && set_mode(3u, CAN_CS_SLEEP) == E_NOT_OK);
    assert(set_mode(5u, CAN_CS_STOPPED) == E_NOT_OK && set_mode(3u, CAN_CS_UNINIT) == E_NOT_OK);
    assert(mode_of() == CAN_CS_STARTED);
    Can_MainFunction_Mode();
    assert(modes == 1u);

    /* Invalid requests (Verifies: SWS_Can_00218, SWS_Can_00486). */
    assert(write(0u, &pdu) == E_NOT_OK && write(1u, NULL_PTR) == E_NOT_OK);
    pdu.sdu = NULL_PTR;
    assert(write(1u, &pdu) == E_NOT_OK);
    pdu.sdu = bytes;
    pdu.length = 9u;
    assert(write(1u, &pdu) == E_NOT_OK);
    pdu.length = 8u;
    pdu.id = 0x800u;
    assert(write(1u, &pdu) == E_NOT_OK);
    pdu.id = 0x80000123u;
    assert(write(1u, &pdu) == E_NOT_OK && frames == 0u);
    /* The FD bit is not evaluated without an FD configuration; the copied frame is
     * classic, byte 0 first (Verifies: SWS_Can_00212, SWS_Can_00011, SWS_Can_00059). */
    pdu.id = 0x40000123u;
    /* Verifies: SWS_Can_00275, SWS_Can_00213 */
    assert(write(1u, &pdu) == E_OK);
    bytes[0] = 99u;
    pdu.swPduHandle = 19u;
    assert(write(1u, &pdu) == CAN_BUSY);
    assert(Can_HostFlush() == ECU_OK && frames == 1u && frame_id == 0x123u && frame_length == 8u);
    for (i = 0u; i < 8u; ++i) {
        assert(frame_data[i] == (uint8)(i + 1u));
    }
    /* Verifies: SWS_Can_00276, SWS_Can_00016, SWS_Can_00031 */
    assert(confirmations == 0u);
    Can_MainFunction_Write();
    assert(confirmations == 1u && confirmed == 18u);
    Can_MainFunction_Write();
    assert(confirmations == 1u);
    /* A zero-length L-PDU is valid. */
    pdu.length = 0u;
    assert(write(1u, &pdu) == E_OK && Can_HostFlush() == ECU_OK);
    assert(frames == 2u && frame_length == 0u);
    Can_MainFunction_Write();
    assert(confirmations == 2u && confirmed == 19u);

    /* Polled reception (Verifies: SWS_Can_00108, SWS_Can_00396, SWS_Can_00279,
     * SWS_Can_00060, SWS_Can_00501). */
    assert(Can_Inject(0x321u, 8u, incoming, 0u) == ECU_OK && receptions == 0u);
    Can_MainFunction_Read();
    assert(receptions == 1u && mailbox.CanId == 0x321u && mailbox.Hoh == 0u);
    assert(mailbox.ControllerId == 5u && received_length == 8u);
    for (i = 0u; i < 8u; ++i) {
        assert(received[i] == incoming[i]);
    }
    Can_MainFunction_Read();
    assert(receptions == 1u);
    /* An overrun of an accepted frame is a uint8 runtime error reported to Det
     * (Verifies: SWS_Can_00395) (Verifies[Can]: SWS_BSW_00219, 00222). */
    assert(Can_Inject(0x321u, 1u, incoming, 0u) == ECU_OK);
    assert(Can_InternalRxIndication(0x322u, 1u, incoming, 0u) == ECU_ERR_CAN_BUSY && lost == 0u);
    Can_MainFunction_Read();
    assert(lost == 1u && receptions == 2u && mailbox.CanId == 0x321u && received_length == 1u);

    /* STOPPED cancels pending messages (Verifies: SWS_Can_00263, SWS_Can_00282). */
    pdu.length = 8u;
    assert(write(1u, &pdu) == E_OK);
    assert(Can_Inject(0x321u, 1u, incoming, 0u) == ECU_OK);
    assert(set_mode(3u, CAN_CS_STOPPED) == E_OK);
    assert(Can_HostFlush() == ECU_OK && frames == 2u);
    Can_MainFunction_Write();
    Can_MainFunction_Read();
    assert(confirmations == 2u && receptions == 2u);
    polled_mode(CAN_CS_STOPPED);
    assert(write(1u, &pdu) == E_NOT_OK && Can_Inject(0x321u, 1u, incoming, 0u) != ECU_OK);
    /* The state machine loops STOPPED to STOPPED. */
    assert(set_mode(3u, CAN_CS_STOPPED) == E_OK);
    polled_mode(CAN_CS_STOPPED);

    /* Logical sleep (Verifies: SWS_Can_00258, SWS_Can_00290, SWS_Can_00404, SWS_Can_00405,
     * SWS_Can_00267). */
    assert(set_mode(3u, CAN_CS_SLEEP) == E_OK);
    polled_mode(CAN_CS_SLEEP);
    assert(mode_of() == CAN_CS_SLEEP && set_mode(3u, CAN_CS_STARTED) == E_NOT_OK);
    assert(set_mode(3u, CAN_CS_SLEEP) == E_OK);
    polled_mode(CAN_CS_SLEEP);
    assert(write(1u, &pdu) == E_NOT_OK && Can_Inject(0x321u, 1u, incoming, 0u) != ECU_OK);
    assert(set_mode(3u, CAN_CS_STOPPED) == E_OK);
    /* The Wakeup main function also polls the mode transition (SWS_Can_00370). */
    Can_MainFunction_Wakeup();
    assert(modes == 6u && last_mode == CAN_CS_STOPPED && mode_of() == CAN_CS_STOPPED);

    /* STARTED reuses the configuration set by Can_Init (Verifies: SWS_Can_00384). */
    assert(set_mode(3u, CAN_CS_STARTED) == E_OK);
    polled_mode(CAN_CS_STARTED);
    assert(write(1u, &pdu) == E_OK && Can_HostFlush() == ECU_OK && frames == 3u);
    Can_MainFunction_Write();
    assert(confirmations == 3u);

    /* Bus-off stops the controller, cancels pending messages and is polled; recovery
     * waits for software (Verifies: SWS_Can_00020, SWS_Can_00272, SWS_Can_00273,
     * SWS_Can_00274, SWS_Can_00109). */
    assert(write(1u, &pdu) == E_OK);
    Can_SetMode(CAN_BUS_OFF);
    assert(mode_of() == CAN_CS_STOPPED && busoffs == 0u);
    assert(error_state(3u, &error) == E_OK && error == CAN_ERRORSTATE_BUSOFF);
    Can_MainFunction_BusOff();
    assert(busoffs == 1u && busoff_controller == 5u);
    Can_MainFunction_BusOff();
    assert(busoffs == 1u);
    assert(Can_HostFlush() == ECU_OK && frames == 3u);
    Can_MainFunction_Write();
    Can_MainFunction_Mode();
    assert(confirmations == 3u && mode_of() == CAN_CS_STOPPED && write(1u, &pdu) == E_NOT_OK);
    assert(set_mode(3u, CAN_CS_STARTED) == E_OK);
    polled_mode(CAN_CS_STARTED);
    assert(error_state(3u, &error) == E_OK && error == CAN_ERRORSTATE_ACTIVE);

    /* De-initialization (Verifies: SWS_Can_91009, SWS_Can_91010). */
    deinit();
    assert(mode_of() == CAN_CS_STARTED);
    assert(set_mode(3u, CAN_CS_STOPPED) == E_OK);
    polled_mode(CAN_CS_STOPPED);
    deinit();
    assert(get_mode(3u, &state) == E_NOT_OK && set_mode(3u, CAN_CS_STARTED) == E_NOT_OK);
    init(&can);
    assert(mode_of() == CAN_CS_STOPPED);
    return 0;
}
