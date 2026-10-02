use super::Scratch;
#[cfg(windows)]
use super::{create_pair, tooling};
#[cfg(windows)]
use autosar_config_core::{Direction, generator};
use std::fs;
#[cfg(windows)]
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
#[cfg(windows)]
use std::process::Stdio;

pub(super) fn standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame() {
    let temp = Scratch::new();
    let harness = temp.0.join("can_standard_api.c");
    let binary = temp.0.join(if cfg!(windows) {
        "can_standard_api.exe"
    } else {
        "can_standard_api"
    });
    fs::write(
        &harness,
r#"#include "Can.h"
#include "CanIf.h"
#include "Os.h"
#include "SchM_Can.h"
#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
static unsigned sent;
static unsigned received;
static unsigned confirmed;
static PduIdType last_confirmed;
static unsigned mode_notifications;
static unsigned bus_off_notifications;
static Can_ControllerStateType last_notified_mode;
static unsigned mode_notification_mismatch;
static unsigned callback_write_busy;
static unsigned probe_rx_reentry;
static EcuStatus nested_rx_result;
static unsigned fail_output;
static unsigned enqueue_on_emit;
EcuStatus Com_AdvanceTime(uint64_t now_ms) { (void)now_ms; return ECU_OK; }
void Dcm_AdvanceTime(uint64_t now_ms) { (void)now_ms; }
EcuStatus CanTp_AdvanceTime(uint64_t now_ms) { (void)now_ms; return ECU_OK; }
EcuStatus Com_TriggerTransmit(size_t frame_index) { (void)frame_index; return ECU_OK; }
static int valid_result;
static int invalid_result;
#ifdef _WIN32
static HANDLE sink_entered;
static HANDLE sink_release;
static EcuStatus flush_result;
static Std_ReturnType blocked_write_result;
#endif
static EcuStatus emit(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    Can_ControllerStateType state = CAN_CS_UNINIT;
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STARTED) return ECU_ERR_CONTROLLER;
    if (id != 0x321u || dlc != 2u || data[0] != 0x12u || data[1] != 0x34u) return ECU_ERR_IO;
#ifdef _WIN32
    if (sink_entered != NULL) {
        SetEvent(sink_entered);
        if (WaitForSingleObject(sink_release, INFINITE) != WAIT_OBJECT_0) return ECU_ERR_IO;
    }
#endif
    if (enqueue_on_emit != 0u) {
        Can_PduType nested = {0u, 2u, 0x321u, (uint8_t *)data};
        enqueue_on_emit = 0u;
        if (Can_Write(0u, &nested) != CAN_BUSY) return ECU_ERR_IO;
    }
    if (fail_output != 0u) return ECU_ERR_IO;
    ++sent;
    return ECU_OK;
}
void CanIf_TxConfirmation(PduIdType handle) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    Can_PduType nested = {handle, 2u, 0x321u, bytes};
    Can_MainFunction_Write();
    if (Can_Write(0u, &nested) == CAN_BUSY) callback_write_busy = 1u;
    last_confirmed = handle;
    ++confirmed;
}
void CanIf_ControllerModeIndication(uint8_t controller_id, Can_ControllerStateType mode) {
    if (controller_id == 0u) {
        Can_ControllerStateType current = CAN_CS_UNINIT;
        if (Can_GetControllerMode(0u, &current) != E_OK || current != mode) {
            ++mode_notification_mismatch;
        }
        last_notified_mode = mode;
        ++mode_notifications;
    }
}
void CanIf_ControllerBusOff(uint8_t controller_id) {
    if (controller_id == 0u) ++bus_off_notifications;
}
EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    Can_ControllerStateType state = CAN_CS_UNINIT;
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STARTED) return ECU_ERR_CONTROLLER;
    if (id != 0x321u || dlc != 2u || data[0] != 0x12u || now_ms != 10u) return ECU_ERR_IO;
    ++received;
    if (probe_rx_reentry != 0u) {
        probe_rx_reentry = 0u;
        Can_MainFunction_Read();
        nested_rx_result = Can_Inject(id, dlc, data, now_ms);
    }
    return ECU_OK;
}
static void send_valid_frames(void) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    unsigned i;
    for (i = 0u; i < 100u; ++i) {
        if (Can_TransmitPdu(0u, 0x321u, 2u, bytes) != ECU_OK) valid_result = 1;
    }
}
static void reject_invalid_frames(void) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    unsigned i;
    for (i = 0u; i < 100u; ++i) {
        if (Can_TransmitPdu(0u, 0x800u, 2u, bytes) != ECU_ERR_FRAME_ID) invalid_result = 1;
    }
}
#ifdef _WIN32
static DWORD WINAPI valid_thread(LPVOID unused) { (void)unused; send_valid_frames(); return 0u; }
static DWORD WINAPI invalid_thread(LPVOID unused) { (void)unused; reject_invalid_frames(); return 0u; }
static DWORD WINAPI flush_thread(LPVOID unused) { (void)unused; flush_result = Can_HostFlush(); return 0u; }
static DWORD WINAPI blocked_write_thread(LPVOID unused) {
    uint8_t bytes[8] = {0x12u, 0x34u};
    Can_PduType pdu = {0u, 2u, 0x321u, bytes};
    (void)unused;
    blocked_write_result = Can_Write(0u, &pdu);
    return 0u;
}
#else
static void *valid_thread(void *unused) { (void)unused; send_valid_frames(); return NULL; }
static void *invalid_thread(void *unused) { (void)unused; reject_invalid_frames(); return NULL; }
#endif
int main(void) {
    const EcuConfig empty_config = {"test", NULL, 0u, NULL, 0u, NULL};
    uint8_t bytes[8] = {0x12u, 0x34u};
    Can_ConfigType config = {emit};
    Can_ConfigType invalid_config = {NULL};
    Can_PduType pdu = {0u, 2u, 0x321u, bytes};
    Can_ControllerStateType state = CAN_CS_UNINIT;
    Can_ErrorStateType error_state = CAN_ERRORSTATE_PASSIVE;
    uint8_t error_counter = 0xa5u;
    Os_Init(&empty_config);
    Can_Init(NULL);
    if (Can_GetControllerMode(0u, &state) != E_NOT_OK) return 1;
    if (Can_GetControllerErrorState(0u, &error_state) != E_NOT_OK) return 47;
    Can_Init(&config);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 2;
    if (Can_GetControllerErrorState(0u, &error_state) != E_OK ||
        error_state != CAN_ERRORSTATE_ACTIVE) return 48;
    if (Can_GetControllerErrorState(1u, &error_state) != E_NOT_OK ||
        Can_GetControllerErrorState(0u, NULL) != E_NOT_OK) return 49;
    if (Can_SetBaudrate(0u, 0u) != E_OK || Can_SetBaudrate(0u, 1u) != E_NOT_OK ||
        Can_SetBaudrate(1u, 0u) != E_NOT_OK) return 50;
    if (Can_GetControllerRxErrorCounter(0u, &error_counter) != E_NOT_OK ||
        Can_GetControllerTxErrorCounter(0u, &error_counter) != E_NOT_OK ||
        error_counter != 0xa5u || Can_CheckWakeup(0u) != E_NOT_OK) return 51;
    Can_DisableControllerInterrupts(0u);
    Can_DisableControllerInterrupts(0u);
    Can_EnableControllerInterrupts(0u);
    Can_EnableControllerInterrupts(0u);
    Can_EnableControllerInterrupts(0u);
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 3;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_NOT_OK) return 13;
    if (Can_SetControllerMode(1u, CAN_CS_STARTED) != E_NOT_OK) return 4;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 5;
    if (mode_notifications != 0u || Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_NOT_OK) return 66;
    Can_SetMode((CanMode)99);
    if (Can_GetMode() != CAN_STARTED || mode_notifications != 0u) return 74;
    if (Os_Advance(1u) != ECU_OK) return 73;
    if (mode_notifications != 1u || last_notified_mode != CAN_CS_STARTED ||
        mode_notification_mismatch != 0u) return 61;
    Can_MainFunction_Wakeup();
    if (mode_notifications != 1u) return 69;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_NOT_OK) return 14;
    if (Can_SetControllerMode(0u, CAN_CS_SLEEP) != E_NOT_OK) return 27;
    if (mode_notifications != 1u) return 65;
    if (Can_Write(1u, &pdu) != E_NOT_OK || Can_Write(0u, NULL) != E_NOT_OK) return 6;
    pdu.id = 0x800u;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 7;
    pdu.id = 0x321u;
    pdu.length = 0u;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 8;
    pdu.length = 2u;
    pdu.swPduHandle = 7u;
    if (Can_Write(0u, &pdu) != E_OK || sent != 0u) return 9;
    Can_Init(NULL);
    Can_Init(&invalid_config);
    Can_Init(&config);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STARTED) return 60;
    if (Can_Write(0u, &pdu) != CAN_BUSY || sent != 0u) return 28;
    if (Can_HostFlush() != ECU_OK || sent != 1u || confirmed != 1u || last_confirmed != 7u ||
        callback_write_busy == 0u) return 29;
    if (Can_Inject(0x321u, 2u, bytes, 10u) != ECU_OK || received != 1u) return 10;
    probe_rx_reentry = 1u;
    if (Can_Inject(0x321u, 2u, bytes, 10u) != ECU_OK || received != 2u ||
        nested_rx_result != ECU_ERR_CAN_BUSY) return 57;
    if (Can_Inject(0x321u, 2u, NULL, 10u) != ECU_ERR_CONFIG || received != 2u) return 12;
    fail_output = 1u;
    if (Can_TransmitPdu(0u, 0x321u, 2u, bytes) != ECU_ERR_IO || sent != 1u || confirmed != 1u) return 20;
    fail_output = 0u;
    Can_SetMode(CAN_BUS_OFF);
    if (bus_off_notifications != 1u) return 62;
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 15;
    if (Can_GetControllerErrorState(0u, &error_state) != E_OK ||
        error_state != CAN_ERRORSTATE_BUSOFF) return 52;
    if (Can_GetMode() != CAN_BUS_OFF) return 21;
    if (Can_TransmitPdu(0u, 0x321u, 2u, bytes) != ECU_ERR_CONTROLLER) return 11;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 16;
    if (mode_notifications != 1u) return 67;
    Can_MainFunction_Wakeup();
    if (last_notified_mode != CAN_CS_STARTED || mode_notifications != 2u) return 63;
    if (Can_GetMode() != CAN_STARTED) return 17;
    if (Can_GetControllerErrorState(0u, &error_state) != E_OK ||
        error_state != CAN_ERRORSTATE_ACTIVE) return 53;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_OK) return 18;
    Can_MainFunction_Wakeup();
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_NOT_OK) return 19;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 22;
    Can_MainFunction_Wakeup();
#ifdef _WIN32
    {
        HANDLE valid = CreateThread(NULL, 0, valid_thread, NULL, 0, NULL);
        HANDLE invalid = CreateThread(NULL, 0, invalid_thread, NULL, 0, NULL);
        if (valid == NULL || invalid == NULL) return 23;
        if (WaitForSingleObject(valid, INFINITE) != WAIT_OBJECT_0) return 24;
        if (WaitForSingleObject(invalid, INFINITE) != WAIT_OBJECT_0) return 25;
        CloseHandle(valid);
        CloseHandle(invalid);
    }
#else
    {
        pthread_t valid;
        pthread_t invalid;
        if (pthread_create(&valid, NULL, valid_thread, NULL) != 0) return 23;
        if (pthread_create(&invalid, NULL, invalid_thread, NULL) != 0) return 24;
        if (pthread_join(valid, NULL) != 0 || pthread_join(invalid, NULL) != 0) return 25;
    }
#endif
    if (valid_result != 0 || invalid_result != 0 || sent != 101u) return 26;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_OK) return 30;
    Can_MainFunction_Wakeup();
    if (Can_SetControllerMode(0u, CAN_CS_SLEEP) != E_OK) return 31;
    if (last_notified_mode != CAN_CS_STOPPED) return 68;
    Can_MainFunction_Wakeup();
    if (last_notified_mode != CAN_CS_SLEEP || mode_notification_mismatch != 0u) return 64;
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_SLEEP) return 32;
    if (Can_Write(0u, &pdu) != E_NOT_OK) return 33;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_NOT_OK) return 34;
    Can_SetMode(CAN_STARTED);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_SLEEP) return 35;
    if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_OK) return 36;
    Can_MainFunction_Wakeup();
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 37;
    Can_MainFunction_Wakeup();
    enqueue_on_emit = 1u;
    if (Can_Write(0u, &pdu) != E_OK || Can_HostFlush() != ECU_OK) return 38;
    if (Can_HostFlush() != ECU_OK || sent != 102u) return 39;
#ifdef _WIN32
    {
        HANDLE output;
        HANDLE writer;
        sink_entered = CreateEvent(NULL, TRUE, FALSE, NULL);
        sink_release = CreateEvent(NULL, TRUE, FALSE, NULL);
        if (sink_entered == NULL || sink_release == NULL) return 40;
        if (Can_Write(0u, &pdu) != E_OK) return 41;
        output = CreateThread(NULL, 0, flush_thread, NULL, 0, NULL);
        if (output == NULL || WaitForSingleObject(sink_entered, 1000u) != WAIT_OBJECT_0) return 42;
        writer = CreateThread(NULL, 0, blocked_write_thread, NULL, 0, NULL);
        if (writer == NULL) return 43;
        if (WaitForSingleObject(writer, 500u) != WAIT_OBJECT_0 || blocked_write_result != CAN_BUSY) {
            SetEvent(sink_release);
            return 44;
        }
        SetEvent(sink_release);
        if (WaitForSingleObject(output, 1000u) != WAIT_OBJECT_0 || flush_result != ECU_OK) return 45;
        CloseHandle(writer);
        CloseHandle(output);
        CloseHandle(sink_entered);
        CloseHandle(sink_release);
        sink_entered = NULL;
        sink_release = NULL;
    }
    if (sent != 103u) return 46;
#endif
    if (Can_Write(0u, &pdu) != E_OK) return 58;
    {
        unsigned before_cancel = confirmed;
        if (Can_SetControllerMode(0u, CAN_CS_STOPPED) != E_OK) return 54;
        Can_DeInit();
        if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 72;
        Can_MainFunction_Wakeup();
        if (Can_HostFlush() != ECU_OK || confirmed != before_cancel) return 59;
    }
    Can_DeInit();
    if (Can_GetControllerMode(0u, &state) != E_NOT_OK || Can_Write(0u, &pdu) != E_NOT_OK) return 55;
    Can_Init(&config);
    if (Can_GetControllerMode(0u, &state) != E_OK || state != CAN_CS_STOPPED) return 56;
    if (Can_SetControllerMode(0u, CAN_CS_STARTED) != E_OK) return 70;
    {
        unsigned before_override = mode_notifications;
        Can_SetMode(CAN_BUS_OFF);
        Can_MainFunction_Wakeup();
        if (mode_notifications != before_override || bus_off_notifications != 2u) return 71;
    }
    return 0;
}
"#,
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let build = Command::new("gcc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-pthread")
        .arg(format!("-I{}", root.join("runtime/include").display()))
        .arg(format!("-I{}", root.join("runtime/host/include").display()))
        .arg(format!("-I{}", root.join("runtime/src").display()))
        .arg(root.join("runtime/host/src/Can_Execution.c"))
        .arg(root.join("runtime/src/Can.c"))
        .arg(root.join("runtime/src/Can_HostLock.c"))
        .arg(root.join("runtime/src/Os.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

pub(super) fn standard_canif_rx_callback_keeps_nested_host_status_and_time_separate() {
    let temp = Scratch::new();
    let harness = temp.0.join("canif_rx_callback.c");
    let binary = temp.0.join(if cfg!(windows) {
        "canif_rx_callback.exe"
    } else {
        "canif_rx_callback"
    });
    fs::write(
        &harness,
        r#"#include "CanIf.h"
#include "Can.h"
#include "LSduR.h"
#include "Os.h"

static const EcuReceiveRoute receive_routes[] = {{0x321u, 2u, 0u, 0u, ECU_ROUTE_COM}};
const EcuReceiveRoute *const Ecu_ReceiveRoutes = receive_routes;
const size_t Ecu_ReceiveRouteCount = 1u;
static const EcuTransmitRoute transmit_routes[] = {
    {0u, 0u, ECU_ROUTE_COM},
    {1u, 1u, ECU_ROUTE_CANTP},
};
const EcuTransmitRoute *const Ecu_TransmitRoutes = transmit_routes;
const size_t Ecu_TransmitRouteCount = 2u;
static int nested_time_ok;
static int outer_time_ok;
static int depth;
static unsigned tx_confirmation_count;
static PduIdType tx_confirmed_id;
static unsigned transmit_count;
uint64_t Os_Now(void) { return 99u; }
EcuStatus Can_TransmitPdu(PduIdType pdu_id, uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    (void)pdu_id; (void)id; (void)dlc; (void)data;
    ++transmit_count;
    return ECU_OK;
}
void LSduR_CanIfTxConfirmation(PduIdType pdu_id, Std_ReturnType result) {
    if (result == E_OK) { tx_confirmed_id = pdu_id; ++tx_confirmation_count; }
}
EcuStatus LSduR_CanTpRxIndication(uint8_t dlc, const uint8_t data[8], uint64_t now_ms) {
    (void)dlc; (void)data; (void)now_ms; return ECU_ERR_CONFIG;
}
EcuStatus LSduR_CanIfRxIndication(size_t rx_pdu_id, const uint8_t data[8], uint64_t now_ms) {
    Can_HwType mailbox = {0x321u, 0u, 0u};
    uint8_t bytes[2] = {0x12u, 0x34u};
    PduInfoType pdu = {bytes, NULL, 2u};
    (void)rx_pdu_id; (void)data;
    if (depth != 0) {
        nested_time_ok = now_ms == 99u;
        return ECU_ERR_FRAME_DLC;
    }
    outer_time_ok = now_ms == 10u;
    depth = 1;
    CanIf_RxIndication(&mailbox, &pdu);
    depth = 0;
    return ECU_OK;
}
int main(void) {
    const EcuFrameConfig frame = {0x321u, 2u, 0u, 0u, 0u, 0u, 1u};
    const EcuConfig config = {"test", &frame, 1u, NULL, 0u, NULL};
    const EcuFrameConfig tx_frame = {0x321u, 2u, 1u, 0u, 0u, 10u, 0u};
    const EcuDiagnosticConfig diagnostic = {.response_can_id = 0x456u, .tx_pdu_id = 1u};
    const EcuConfig tx_config = {"tx", &tx_frame, 1u, NULL, 0u, &diagnostic};
    const uint8_t data[8] = {0x12u, 0x34u};
    CanIf_Init(&config);
    if (CanIf_HostRxIndication(0x321u, 2u, data, 10u) != ECU_OK) return 1;
    if (outer_time_ok == 0 || nested_time_ok == 0) return 2;
    CanIf_TxConfirmation(0u);
    if (tx_confirmation_count != 0u) return 3;
    CanIf_Init(&tx_config);
    outer_time_ok = 0;
    nested_time_ok = 0;
    if (CanIf_HostRxIndication(0x321u, 2u, data, 10u) != ECU_OK ||
        outer_time_ok != 0 || nested_time_ok != 0) return 15;
    if (CanIf_TransmitDiagnostic(2u, data) != ECU_ERR_CONTROLLER || transmit_count != 0u) return 12;
    if (CanIf_Transmit(0u, data) != ECU_ERR_CONTROLLER || transmit_count != 0u) return 6;
    CanIf_ControllerModeIndication(1u, CAN_CS_STARTED);
    if (CanIf_Transmit(0u, data) != ECU_ERR_CONTROLLER ||
        CanIf_TransmitDiagnostic(2u, data) != ECU_ERR_CONTROLLER || transmit_count != 0u) return 7;
    CanIf_ControllerModeIndication(0u, CAN_CS_STARTED);
    if (CanIf_Transmit(0u, data) != ECU_OK || transmit_count != 1u) return 8;
    if (CanIf_TransmitDiagnostic(2u, data) != ECU_OK || transmit_count != 2u) return 13;
    CanIf_ControllerBusOff(0u);
    if (CanIf_Transmit(0u, data) != ECU_ERR_CONTROLLER ||
        CanIf_TransmitDiagnostic(2u, data) != ECU_ERR_CONTROLLER || transmit_count != 2u) return 9;
    CanIf_ControllerModeIndication(0u, CAN_CS_STARTED);
    if (CanIf_Transmit(0u, data) != ECU_OK || transmit_count != 3u) return 10;
    if (CanIf_TransmitDiagnostic(2u, data) != ECU_OK || transmit_count != 4u) return 14;
    CanIf_ControllerModeIndication(0u, CAN_CS_STOPPED);
    if (CanIf_Transmit(0u, data) != ECU_ERR_CONTROLLER ||
        CanIf_TransmitDiagnostic(2u, data) != ECU_ERR_CONTROLLER || transmit_count != 4u) return 11;
    CanIf_TxConfirmation(2u);
    if (tx_confirmation_count != 0u) return 4;
    CanIf_TxConfirmation(0u);
    if (tx_confirmation_count != 1u || tx_confirmed_id != 0u) return 5;
    return 0;
}
"#,
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let build = Command::new("gcc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-pthread")
        .arg(format!("-I{}", root.join("runtime/include").display()))
        .arg(format!("-I{}", root.join("runtime/host/include").display()))
        .arg(format!("-I{}", root.join("runtime/src").display()))
        .arg(root.join("runtime/host/src/Ecu_Clock.c"))
        .arg(root.join("runtime/src/CanIf.c"))
        .arg(root.join("runtime/src/Can_HostLock.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

pub(super) fn cantp_failed_frame_confirmation_ends_host_session() {
    let temp = Scratch::new();
    let harness = temp.0.join("cantp_failed_confirmation.c");
    let binary = temp.0.join(if cfg!(windows) {
        "cantp_failed_confirmation.exe"
    } else {
        "cantp_failed_confirmation"
    });
    fs::write(
        &harness,
        r#"#include "CanTp.h"
#include "LSduR.h"
#include "Os.h"
#include "PduR.h"
#include <stddef.h>
const EcuPolicyConfig Ecu_Policy = {.rx_time_order = ECU_TIME_BEFORE_RX,
                                  .wait_when_wft_max_zero = ECU_WAIT_RESTART};
static unsigned completion_count;
static EcuStatus completion_status;
static unsigned fail_frame;
EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]) {
    (void)dlc; (void)data;
    CanTp_TxConfirmation(0u, fail_frame != 0u ? E_NOT_OK : E_OK);
    return ECU_OK;
}

EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length) {
    size_t i;
    (void)offset;
    for (i = 0u; i < length; ++i) destination[i] = 0u;
    return ECU_OK;
}
void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms) {
    (void)now_ms;
    completion_status = status;
    ++completion_count;
}
void PduR_CanTpRxAbort(void) {}
EcuStatus PduR_CanTpRxIndication(uint64_t now_ms) { (void)now_ms; return ECU_OK; }
EcuStatus PduR_CanTpStartOfReception(size_t length) { (void)length; return ECU_OK; }
EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length) {
    (void)data; (void)length; return ECU_OK;
}
int main(void) {
    static const EcuDiagnosticConfig config = {0};
    CanTp_Init(&config);
    fail_frame = 1u;
    if (CanTp_Transmit(8u, 0u) != ECU_ERR_IO) return 1;
    if (completion_count != 1u || completion_status != ECU_ERR_IO) return 2;
    fail_frame = 0u;
    if (CanTp_Transmit(8u, 6u) != ECU_OK || completion_count != 1u) return 3;
    return 0;
}
"#,
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let build = Command::new("gcc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg(format!("-I{}", root.join("runtime/include").display()))
        .arg(format!("-I{}", root.join("runtime/host/include").display()))
        .arg(root.join("runtime/src/CanTp.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

pub(super) fn cantp_async_confirmation_respects_n_as_and_late_release() {
    let temp = Scratch::new();
    let harness = temp.0.join("cantp_async_confirmation.c");
    let binary = temp.0.join(if cfg!(windows) {
        "cantp_async_confirmation.exe"
    } else {
        "cantp_async_confirmation"
    });
    fs::write(
        &harness,
        r#"#include "CanTp.h"
#include "LSduR.h"
#include "PduR.h"
#include <stddef.h>
const EcuPolicyConfig Ecu_Policy = {.rx_time_order = ECU_TIME_BEFORE_RX,
                                  .wait_when_wft_max_zero = ECU_WAIT_RESTART};
static unsigned completion_count;
static EcuStatus completion_status;
static unsigned frame_count;
static unsigned rx_abort_count;
EcuStatus LSduR_CanTpTransmit(uint8_t dlc, const uint8_t data[8]) {
    (void)dlc; (void)data;
    ++frame_count;
    return ECU_OK;
}
EcuStatus PduR_CanTpCopyTxData(size_t offset, uint8_t *destination, size_t length) {
    size_t i;
    (void)offset;
    for (i = 0u; i < length; ++i) destination[i] = 0u;
    return ECU_OK;
}
void PduR_CanTpTxConfirmation(EcuStatus status, uint64_t now_ms) {
    (void)now_ms;
    completion_status = status;
    ++completion_count;
}
void PduR_CanTpRxAbort(void) { ++rx_abort_count; }
EcuStatus PduR_CanTpRxIndication(uint64_t now_ms) { (void)now_ms; return ECU_OK; }
EcuStatus PduR_CanTpStartOfReception(size_t length) { (void)length; return ECU_OK; }
EcuStatus PduR_CanTpCopyRxData(const uint8_t *data, size_t length) {
    (void)data; (void)length; return ECU_OK;
}
int main(void) {
    static const EcuDiagnosticConfig config = {.n_as_ms = 5u, .n_bs_ms = 10u,
                                              .n_cr_ms = 10u, .tx_pdu_id = 3u};
    const uint8_t flow_control[8] = {0x30u, 0u, 0u, 0u, 0u, 0u, 0u, 0u};
    const uint8_t first_frame[8] = {0x10u, 0x08u, 0u, 0u, 0u, 0u, 0u, 0u};
    CanTp_Init(&config);
    if (CanTp_Transmit(4u, 0u) != ECU_OK || frame_count != 1u || completion_count != 0u) return 1;
    CanTp_TxConfirmation(2u, E_OK);
    if (CanTp_AdvanceTime(1u) != ECU_OK || completion_count != 0u) return 2;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(2u) != ECU_OK || completion_count != 1u ||
        completion_status != ECU_OK) return 3;
    if (CanTp_Transmit(4u, 3u) != ECU_OK || frame_count != 2u) return 4;
    if (CanTp_AdvanceTime(8u) != ECU_ERR_TP_TIMEOUT || completion_count != 2u ||
        completion_status != ECU_ERR_TP_TIMEOUT) return 5;
    if (CanTp_Transmit(4u, 8u) != ECU_ERR_TP_BUSY) return 6;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(9u) != ECU_OK || completion_count != 2u) return 7;
    if (CanTp_Transmit(4u, 10u) != ECU_OK || frame_count != 3u) return 8;
    CanTp_TxConfirmation(3u, E_NOT_OK);
    if (CanTp_AdvanceTime(11u) != ECU_ERR_IO || completion_count != 3u ||
        completion_status != ECU_ERR_IO) return 9;
    if (CanTp_Transmit(8u, 12u) != ECU_OK || frame_count != 4u) return 10;
    if (CanTp_RxIndication(3u, flow_control, 13u) != ECU_ERR_TP_FLOW) return 11;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(14u) != ECU_OK || completion_count != 3u) return 12;
    if (CanTp_RxIndication(3u, flow_control, 15u) != ECU_OK || frame_count != 5u) return 13;
    if (CanTp_AdvanceTime(19u) != ECU_OK || completion_count != 3u) return 14;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(19u) != ECU_OK || completion_count != 4u ||
        completion_status != ECU_OK) return 15;
    if (CanTp_Transmit(8u, 20u) != ECU_OK || frame_count != 6u) return 16;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(21u) != ECU_OK || completion_count != 4u) return 17;
    if (CanTp_AdvanceTime(31u) != ECU_ERR_TP_TIMEOUT || completion_count != 5u ||
        completion_status != ECU_ERR_TP_TIMEOUT) return 18;
    if (CanTp_Transmit(8u, 40u) != ECU_OK || frame_count != 7u) return 19;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(41u) != ECU_OK) return 20;
    if (CanTp_RxIndication(8u, first_frame, 42u) != ECU_OK || frame_count != 8u) return 21;
    if (CanTp_AdvanceTime(47u) != ECU_ERR_TP_TIMEOUT || rx_abort_count != 1u ||
        completion_count != 5u) return 22;
    CanTp_TxConfirmation(3u, E_OK);
    if (CanTp_AdvanceTime(48u) != ECU_OK || completion_count != 5u) return 23;
    if (CanTp_Transmit(4u, 49u) != ECU_ERR_TP_BUSY) return 24;
    if (CanTp_AdvanceTime(51u) != ECU_ERR_TP_TIMEOUT || completion_count != 6u ||
        completion_status != ECU_ERR_TP_TIMEOUT) return 25;
    return 0;
}
"#,
    )
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let build = Command::new("gcc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg(format!("-I{}", root.join("runtime/include").display()))
        .arg(format!("-I{}", root.join("runtime/host/include").display()))
        .arg(root.join("runtime/src/CanTp.c"))
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[cfg(windows)]
pub(super) fn generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    fs::create_dir(&out_a).unwrap();
    fs::create_dir(&out_b).unwrap();
    generator::generate(&mut a, &out_a, tooling::native_target()).unwrap();
    generator::generate(&mut b, &out_b, tooling::native_target()).unwrap();
    let mut names: Vec<String> = fs::read_to_string(out_a.join("files.list"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    names.extend(["files.list".into(), "files.sha256".into()]);
    let original: Vec<_> = names
        .iter()
        .map(|name| fs::read(out_a.join(name)).unwrap())
        .collect();
    generator::generate(&mut a, &out_a, tooling::native_target()).unwrap();
    for (name, expected) in names.iter().zip(&original) {
        assert_eq!(
            &fs::read(out_a.join(name)).unwrap(),
            expected,
            "identical ARXML changed generated file {name}"
        );
    }
    let exe_a = tooling::build_host(&out_a).unwrap().binary_path;
    let exe_b = tooling::build_host(&out_b).unwrap().binary_path;
    let mut tx = Command::new(exe_a)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    tx.stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\n")
        .unwrap();
    let actual = String::from_utf8(tx.wait_with_output().unwrap().stdout).unwrap();
    assert!(
        actual
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 2 B001"),
        "golden LSB0 bit packing: {actual}"
    );
    let mut rx = Command::new(exe_b)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    rx.stdin
        .take()
        .unwrap()
        .write_all(b"T 10\nR 801 2 B001\nG 0\n")
        .unwrap();
    let received = String::from_utf8(rx.wait_with_output().unwrap().stdout).unwrap();
    assert!(
        received
            .lines()
            .any(|line| line.trim_end_matches('\r') == "V 0 54 1"),
        "independent received value: {received}"
    );
    let result = tooling::run_hosts(&out_a, &out_b).unwrap();
    assert!(result.passed, "host bus failed: {}", result.log);
    assert!(result.events.iter().any(|e| e.contains("BUS_OFF")));
    assert!(result.events.iter().any(|e| e.contains("DLC")));
}

#[cfg(windows)]
pub(super) fn host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let tx = a
        .add_frame("Mismatch".into(), 0x600, 2, Direction::Tx, Some(10), None)
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Mismatch")
        .unwrap()
        .path;
    a.add_signal(tx, "ExtraTx".into(), 0, 8, 1).unwrap();
    let rx = b
        .add_frame("Mismatch".into(), 0x600, 1, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Mismatch")
        .unwrap()
        .path;
    b.add_signal(rx, "ExtraRx".into(), 0, 8, 0).unwrap();
    a.save().unwrap();
    b.save().unwrap();
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    generator::generate(&mut a, &out_a, tooling::native_target()).unwrap();
    generator::generate(&mut b, &out_b, tooling::native_target()).unwrap();
    tooling::build_host(&out_a).unwrap();
    tooling::build_host(&out_b).unwrap();
    let result = tooling::run_hosts(&out_a, &out_b).unwrap();
    assert!(
        !result.passed && result.log.contains("FRAME_DLC"),
        "ID-matched DLC mismatch was ignored: {}",
        result.log
    );
}
