//! Object sections and zero-initialization of the generated CAN driver.
use super::*;
use std::fs;
use std::path::Path;

#[test]
fn generated_can_objects_use_mapped_sections_and_clear_state() {
    let scratch = Scratch::new();
    let output = scratch.0.join("samples");
    analysis_samples::generate(&output, &scratch);
    for case in ["standard-ecu", "multi-component"] {
        generated_can_object_sections_and_clear(&output.join(case));
    }
}

fn generated_can_object_sections_and_clear(project: &Path) {
    use std::process::Command;
    let target: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join("target.json")).unwrap()).unwrap();
    let scratch = workspace::Scratch::new();
    let settings = tooling::execution_settings();
    let compile = |source: &Path, output: &Path, object: bool| {
        let mut command = Command::new(&settings.compiler);
        command.current_dir(project);
        for flag in target["compilerFlags"].as_array().unwrap() {
            command.arg(flag.as_str().unwrap());
        }
        for include in target["includePaths"].as_array().unwrap() {
            command.arg("-I").arg(include.as_str().unwrap());
        }
        if object {
            command.arg("-c");
        }
        let result = command.arg(source).arg("-o").arg(output).output().unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            project.display(),
            String::from_utf8_lossy(&result.stderr)
        );
    };
    let object = scratch.0.join("Can.o");
    // Compile the actual generated source with the delivered profile flags and headers.
    compile(&project.join("src/Can.c"), &object, true);
    let symbols = tooling::symbol_sections(&settings.objdump, &object);
    for (symbol, section) in [
        ("Can_Write", ".text"),
        ("Can_Init", ".text"),
        ("Can_MainFunction_Read", ".text"),
        ("can_controller_mode", ".bss"),
        ("can_initialized", ".bss"),
        ("tx_payload", ".bss"),
        ("rx_payload", ".bss"),
        ("driver_lifetime", ".bss"),
    ] {
        assert!(
            symbols
                .iter()
                .any(|(name, actual)| name == symbol && actual == section),
            "{symbol}/{section}: {symbols:?}"
        );
    }
    let source = scratch.0.join("can-clear.c");
    fs::write(&source, format!(r#"#include "{}"
#include <assert.h>
const EcuPolicyConfig Ecu_Policy = {{0}};
void Can_Lock(void) {{}}
void Can_Unlock(void) {{}}
int Can_TryLock(void) {{ return 1; }}
void SchM_Enter_CanIf_CANIF_STATE(void) {{}}
void SchM_Exit_CanIf_CANIF_STATE(void) {{}}
void CanIf_TxConfirmation(PduIdType id) {{ (void)id; }}
void CanIf_ControllerModeIndication(uint8_t id, Can_ControllerStateType mode) {{ (void)id; (void)mode; }}
void CanIf_ControllerBusOff(uint8_t id) {{ (void)id; }}
EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t length, const uint8_t data[8], uint64_t time) {{
    (void)id; (void)length; (void)data; (void)time; return ECU_OK;
}}
Std_ReturnType Det_ReportRuntimeError(uint16_t module, uint8_t instance, uint8_t api, uint8_t error) {{
    (void)module; (void)instance; (void)api; (void)error; return E_OK;
}}
EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t length, const uint8_t data[8]) {{
    (void)sink; (void)handle; (void)id; (void)length; (void)data; return ECU_OK;
}}
int main(void) {{
    size_t i;
    assert(can_controller_mode == 0 && bus_off == 0u && can_initialized == 0u && tx_sink == NULL);
    assert(driver_lifetime == 0u && tx_pending == 0u && tx_in_flight == 0u && tx_cancelled == 0u);
    assert(tx_id == 0u && tx_length == 0u && tx_handle == 0u);
    assert(tx_confirmation_pending == 0u && tx_confirmation_handle == 0u && tx_confirming == 0u);
    assert(interrupt_disable_count == 0u && mode_notification_pending == 0u && mode_notification_processing == 0u);
    assert(pending_controller_mode == 0 && busoff_notification_pending == 0u);
    assert(rx_pending == 0u && rx_processing == 0u && rx_id == 0u && rx_length == 0u && rx_time_ms == 0u);
    assert(rx_overrun_pending == 0u);
#if CAN_HOST_QUEUED_COMPLETION == 1
    assert(host_tx_sequence == 0u && host_tx_cancelled_through == 0u && host_tx_completed_through == 0u);
#endif
    for (i = 0u; i < 8u; ++i) {{ assert(tx_payload[i] == 0u && rx_payload[i] == 0u); }}
    return 0;
}}
"#, project.join("src/Can.c").display().to_string().replace('\\', "/"))).unwrap();
    let executable = tooling::native_binary(&scratch.0, "can-clear");
    // Stubs only close linking: this oracle examines real generated static storage before Init.
    compile(&source, &executable, false);
    let result = Command::new(executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}: {:?}",
        project.display(),
        result
    );
}
