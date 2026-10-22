//! Actual public types, object sections and initial state of analysis samples.
use super::*;
use std::fs;
use std::path::Path;

#[test]
fn generated_analysis_samples_use_selected_types_and_mapped_can_state() {
    let scratch = Scratch::new();
    let output = match std::env::var_os("AUTOSAR_C_ANALYSIS_SAMPLES") {
        Some(path) => std::path::absolute(std::path::PathBuf::from(path)).unwrap(),
        None => {
            let output = scratch.0.join("samples");
            analysis_samples::generate(&output, &scratch);
            output
        }
    };
    for case in ["standard-ecu", "component-pipeline"] {
        generated_can_object_sections_and_clear(&output.join(case));
    }
    // R24-11 SWS_COMTYPE_00005/00008: EcuC-selected unsigned types, exposed
    // through ComStack_Types.h. Numeric expectations are independent of emitters.
    for (case, expected) in [
        ("standard-ecu", "2 65535 2 65535 2\n"),
        ("user-application", "2 65535 2 65535 2\n"),
        ("component-pipeline", "1 255 4 4294967295 4\n"),
        ("no-diagnostic", "1 255 1 255 1\n"),
    ] {
        generated_communication_types(&output.join(case), expected);
    }
}

fn generated_communication_types(project: &Path, expected: &str) {
    use std::process::Command;
    use std::time::Duration;
    let scratch = Scratch::new();
    let source = scratch.0.join("communication-types.c");
    fs::write(
        &source,
        r#"#include "ComStack_Types.h"
#include <stdio.h>
int main(void) {
    PduIdType id = (PduIdType)-1;
    PduLengthType length = (PduLengthType)-1;
    PduInfoType info = {NULL_PTR, NULL_PTR, length};
    return printf("%zu %lu %zu %lu %zu\n", sizeof(id), (unsigned long)id,
                  sizeof(length), (unsigned long)info.SduLength,
                  sizeof(info.SduLength)) < 0 ? 1 : 0;
}
"#,
    )
    .unwrap();
    let target: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join("target.json")).unwrap()).unwrap();
    let executable = tooling::native_binary(&scratch.0, "communication-types");
    let mut command = tooling::c_compiler();
    command.current_dir(project);
    for flag in target["compilerFlags"].as_array().unwrap() {
        command.arg(flag.as_str().unwrap());
    }
    for include in target["includePaths"].as_array().unwrap() {
        command.arg("-I").arg(include.as_str().unwrap());
    }
    command.arg(&source).arg("-o").arg(&executable);
    let compiled = tooling::run_public_command(
        &mut command,
        &scratch.0,
        "compile-types",
        Duration::from_secs(30),
    );
    assert!(compiled.status.success(), "{compiled:?}");
    let result = tooling::run_public_command(
        &mut Command::new(executable),
        &scratch.0,
        "run-types",
        Duration::from_secs(30),
    );
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        String::from_utf8(result.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        expected,
        "{}",
        project.display()
    );
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
    compile(&project.join("bsw/src/Can.c"), &object, true);
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
void SchM_Enter_Can_CAN_STATE(void) {{}}
void SchM_Exit_Can_CAN_STATE(void) {{}}
void CanIf_TxConfirmation(PduIdType id) {{ (void)id; }}
void CanIf_ControllerModeIndication(uint8_t id, Can_ControllerStateType mode) {{ (void)id; (void)mode; }}
void CanIf_ControllerBusOff(uint8_t id) {{ (void)id; }}
void CanIf_RxIndication(const Can_HwType *mailbox, const PduInfoType *info) {{ (void)mailbox; (void)info; }}
Std_ReturnType Det_ReportRuntimeError(uint16_t module, uint8_t instance, uint8_t api, uint8_t error) {{
    (void)module; (void)instance; (void)api; (void)error; return E_OK;
}}
EcuStatus Ecu_ExecutionTransmit(Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t length, const uint8_t data[8]) {{
    (void)sink; (void)handle; (void)id; (void)length; (void)data; return ECU_OK;
}}
int main(void) {{
    uint8 i;
    assert(can_controller_mode == 0 && bus_off == 0u && can_initialized == 0u && tx_sink == NULL_PTR);
    assert(driver_lifetime == 0u && tx_pending == 0u && tx_in_flight == 0u && tx_cancelled == 0u);
    assert(tx_id == 0u && tx_length == 0u && tx_handle == 0u);
    assert(tx_confirmation_pending == 0u && tx_confirmation_handle == 0u && tx_confirming == 0u);
    assert(interrupt_disable_count == 0u && mode_notification_pending == 0u &&
           indication_processing[0] == 0u && indication_processing[1] == 0u);
    assert(pending_controller_mode == 0 && busoff_notification_pending == 0u);
    assert(rx_pending == 0u && rx_processing == 0u && rx_id == 0u && rx_length == 0u);
    assert(rx_overrun_pending == 0u);
#if CAN_HOST_QUEUED_COMPLETION == 1
    assert(host_tx_sequence == 0u && host_tx_cancelled_through == 0u && host_tx_completed_through == 0u);
#endif
    for (i = 0u; i < 8u; ++i) {{ assert(tx_payload[i] == 0u && rx_payload[i] == 0u); }}
    return 0;
}}
"#, project.join("bsw/src/Can.c").display().to_string().replace('\\', "/"))).unwrap();
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
