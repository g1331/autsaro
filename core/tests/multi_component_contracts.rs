use autosar_config_core::{
    definitions::DefinitionCatalog,
    integration::{InputSource, RuntimeCatalog, ValidatedIntegrationPlan, build_plan_native},
};
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[allow(dead_code)]
#[path = "support/tooling.rs"]
mod tooling;
#[allow(dead_code)]
#[path = "support/workspace.rs"]
mod workspace;
use workspace::Scratch;

fn inputs() -> Vec<InputSource> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi-component");
    let mut sources = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            InputSource::new(
                entry.file_name().to_str().unwrap(),
                std::fs::read(entry.path()).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
    sources
}
fn build(
    sources: &[InputSource],
) -> Result<ValidatedIntegrationPlan, Vec<autosar_config_core::integration::PlanDiagnostic>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    build_plan_native(
        sources,
        &DefinitionCatalog::builtin().unwrap(),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
}
fn change(sources: &mut [InputSource], file: &str, before: &str, after: &str) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    assert!(text.contains(before), "{file}: {before}");
    *source = InputSource::new(file, text.replacen(before, after, 1).into_bytes()).unwrap();
}

#[test]
fn source_derived_multi_contract_is_deterministic_and_keeps_local_identity() {
    let sources = inputs();
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let description = plan.description();
    let multi = description.multi.as_ref().unwrap();
    assert_eq!(description.profile, "singlecore-multi-swc-v1");
    assert!(description.component.is_none());
    assert_eq!(multi.components.len(), 4);
    assert_eq!(multi.connections.len(), 5);
    assert_eq!(multi.network_endpoints.len(), 2);
    assert_eq!(
        description
            .schedule
            .entities
            .iter()
            .filter(|entity| entity.application)
            .map(|entity| entity.symbol.as_str())
            .collect::<Vec<_>>(),
        ["Ingress_Periodic", "Process_Periodic", "Observe_Periodic"]
    );
    for component in &multi.components {
        assert_ne!(component.component, component.instance);
        assert!(component.rte_instance.starts_with("/Configuration/Rte/"));
        if component.component.ends_with("Process") || component.component.ends_with("Observe") {
            assert!(
                !multi
                    .network_endpoints
                    .iter()
                    .any(|endpoint| endpoint.endpoint.instance == component.instance)
            );
        }
    }
    let read_values = multi
        .components
        .iter()
        .flat_map(|component| &component.data_ports)
        .filter(|port| port.name == "Value" && port.read)
        .map(|port| port.initial_value)
        .collect::<Vec<_>>();
    assert_eq!(read_values, [7, 9]);
    let server = description
        .symbols
        .iter()
        .find(|symbol| symbol.symbol == "Process_Transform")
        .unwrap();
    assert_eq!(server.return_type, "void");
    assert_eq!(
        server
            .arguments
            .iter()
            .map(|argument| (argument.native_type.as_str(), argument.direction.as_str()))
            .collect::<Vec<_>>(),
        [("uint32", "IN"), ("uint32 *", "OUT"), ("uint32 *", "INOUT")]
    );
    let files = plan.component_contract_files().unwrap();
    let mut reversed = sources.clone();
    reversed.reverse();
    assert_eq!(
        files.files(),
        build(&reversed)
            .unwrap()
            .component_contract_files()
            .unwrap()
            .files()
    );
    assert!(description.legacy_component().is_err());
    assert!(
        plan.ecu_integration_files(autosar_config_core::target::BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    if let Ok(directory) = std::env::var("AUTOSAR_MULTI_CONTRACT_OUTPUT") {
        let output = Path::new(&directory);
        let preview = files.preview(output).unwrap();
        files.generate_previewed(output, &preview.revision).unwrap();
    }
}

#[test]
fn multi_com_plan_keeps_real_group_timebase_and_notification_identity() {
    let plan = build(&inputs()).unwrap();
    let com = plan.description().com_runtime.as_ref().unwrap();
    assert_eq!(com.callback_header, "Rte_Com.h");
    assert_eq!(com.receive_group.path, "/Configuration/Com/Config/RxGroup");
    assert_eq!(com.receive_group.handle, 0);
    assert_eq!(
        com.receive_group.members,
        ["/Configuration/Com/Config/RxValuePdu"]
    );
    assert_eq!(com.receive_main.symbol, "Com_MainFunctionRx_Rx");
    assert_eq!(com.receive_main.period_ms, 1);
    assert_eq!(com.transmit_main.symbol, "Com_MainFunctionTx_Tx");
    assert_eq!(com.transmit_main.period_ms, 10);
    let reception = &com.receptions[0];
    assert_eq!(reception.signal, "/Configuration/Com/Config/RxValue");
    assert_eq!(
        reception.user_signal,
        "/Configuration/Rte/ComUser/Callbacks/RxValue"
    );
    assert_eq!(reception.callback_handle, 17);
    assert_eq!(reception.first_timeout_ms, 0);
    assert_eq!(reception.timeout_ms, 30);
    assert_eq!(reception.receive_callback, "Rte_COMCbk");
    assert_eq!(reception.timeout_callback, "Rte_COMCbkRxTOut");
    let mut zero = inputs();
    xml_edit(
        &mut zero,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComFirstTimeout"))
            })
        },
        |text| text.replace("<VALUE>0</VALUE>", "<VALUE>0.0e3</VALUE>"),
    );
    assert_eq!(
        build(&zero)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receptions[0]
            .first_timeout_ms,
        0
    );
    let mut renamed = inputs();
    replace_all(
        &mut renamed,
        "/Com/Config/RxGroup",
        "/Com/Config/ReceptionGroup",
    );
    xml_edit(
        &mut renamed,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            text.replace(
                "<SHORT-NAME>RxGroup</SHORT-NAME>",
                "<SHORT-NAME>ReceptionGroup</SHORT-NAME>",
            )
        },
    );
    assert_eq!(
        build(&renamed)
            .unwrap()
            .description()
            .com_runtime
            .as_ref()
            .unwrap()
            .receive_group
            .path,
        "/Configuration/Com/Config/ReceptionGroup"
    );
}

#[test]
fn multi_com_rejects_tx_main_period_different_from_periodic_pdu() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "Transmit10ms"),
        |text| {
            text.replace("Alarm_App", "Alarm_Work")
                .replace("Ev_App", "Ev_Work")
        },
    );
    xml_edit(
        &mut sources,
        "bsw.arxml",
        |node| named(node, "BSW-TIMING-EVENT", "Com_TriggerTransmit_10ms"),
        |text| text.replace("<PERIOD>0.01</PERIOD>", "<PERIOD>0.001</PERIOD>"),
    );
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/ComMainTxTimeBase"))
            })
        },
        |text| text.replace("<VALUE>0.01</VALUE>", "<VALUE>0.001</VALUE>"),
    );
    rejects_in_both(&sources, "COM_TIMEBASE");
}

#[test]
fn multi_com_control_configuration_rejects_inconsistent_group_timebase_and_callbacks() {
    for (field, original, replacement, code) in [
        ("ComSupportedIPduGroups", "1", "0", "COM_RX_GROUP"),
        ("ComIPduGroupHandleId", "0", "1", "COM_RX_GROUP"),
        ("ComMainRxTimeBase", "0.001", "0.002", "COM_TIMEBASE"),
        ("ComMainTxTimeBase", "0.01", "0.001", "COM_TIMEBASE"),
        (
            "ComUserHeaderInclude",
            "Rte_Com.h",
            "Wrong.h",
            "COM_CALLBACK",
        ),
        ("ComUserCallbackName", "Rte_COMCbk", "Wrong", "COM_CALLBACK"),
        (
            "ComUserCallbackType",
            "COM_RX_ACK",
            "COM_TX_ACK",
            "COM_CALLBACK",
        ),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |text| {
                text.replace(
                    &format!("<VALUE>{original}</VALUE>"),
                    &format!("<VALUE>{replacement}</VALUE>"),
                )
            },
        );
        rejects_in_both(&sources, code);
    }
    for (field, code) in [
        ("ComIPduGroupRef", "COM_RX_GROUP"),
        ("ComIPduMainFunctionRef", "COM_RX_GROUP"),
        ("ComUserCallbackRef", "COM_CALLBACK"),
    ] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with(&format!("/{field}")))
                })
            },
            |_| String::new(),
        );
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.has_tag_name("PARAMETER-VALUES")
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with("/ComUserSignal"))
                    })
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    // A live target exists, so rejection proves direction/ownership rather than a dangling ref.
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| {
            text.replace(
                "/Configuration/Com/Config/RxGroup",
                "/Configuration/Com/Config/TxValuePdu",
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComUserCallbackRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_CALLBACK");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxGroup"),
        |text| {
            format!(
                "{text}{}",
                text.replace(
                    "<SHORT-NAME>RxGroup</SHORT-NAME>",
                    "<SHORT-NAME>DuplicateGroup</SHORT-NAME>"
                )
            )
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|text| text.ends_with("/ComIPduGroupRef"))
            })
        },
        |text| format!("{text}{text}"),
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<REFERENCE-VALUES>", r#"<REFERENCE-VALUES><ECUC-REFERENCE-VALUE><DEFINITION-REF DEST="ECUC-REFERENCE-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduGroupRef</DEFINITION-REF><VALUE-REF DEST="ECUC-CONTAINER-VALUE">/Configuration/Com/Config/RxGroup</VALUE-REF></ECUC-REFERENCE-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_RX_GROUP");
}

#[test]
fn malformed_multi_contracts_are_rejected_at_real_source_objects() {
    let vectors = [
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Process/Result</TARGET-R-PORT-REF>",
            "direction",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Observe/Value</TARGET-R-PORT-REF>",
            "ownership",
        ),
        (
            "composition.arxml",
            "/Application/Process/Value</TARGET-R-PORT-REF>",
            "/Application/Missing/Value</TARGET-R-PORT-REF>",
            "dangling",
        ),
        (
            "composition.arxml",
            "<SHORT-NAME>ProcessInstance</SHORT-NAME>",
            "<SHORT-NAME>IngressInstance</SHORT-NAME>",
            "duplicate instance",
        ),
        (
            "process.arxml",
            "<SHORT-NAME>Process</SHORT-NAME>",
            "<SHORT-NAME>ingress</SHORT-NAME>",
            "case collision",
        ),
        (
            "process.arxml",
            "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
            "/Types/MissingMapping</DATA-TYPE-MAPPING-REF>",
            "mapping",
        ),
        (
            "types.arxml",
            "<DIRECTION>INOUT</DIRECTION>",
            "<DIRECTION>INVALID</DIRECTION>",
            "parameter direction",
        ),
        (
            "types.arxml",
            "<ARRAY-SIZE>4</ARRAY-SIZE>",
            "<ARRAY-SIZE>5</ARRAY-SIZE>",
            "array length",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>1e-999</ALIVE-TIMEOUT>",
            "timeout underflow",
        ),
        (
            "process.arxml",
            "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
            "<ALIVE-TIMEOUT>invalid</ALIVE-TIMEOUT>",
            "malformed timeout",
        ),
        (
            "process.arxml",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "local freshness",
        ),
        (
            "process.arxml",
            "<VALUE>7</VALUE>",
            "<VALUE>-1</VALUE>",
            "initial value",
        ),
        (
            "process.arxml",
            "<PERIOD>0.01</PERIOD>",
            "<PERIOD>0.02</PERIOD>",
            "period mismatch",
        ),
        (
            "ecuc.arxml",
            "<VALUE>5</VALUE>",
            "<VALUE>4</VALUE>",
            "position",
        ),
        (
            "ecuc.arxml",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>false</VALUE>",
            "RteEventIsMappedToTask</DEFINITION-REF>\n                      <VALUE>true</VALUE>",
            "server task mapping",
        ),
    ];
    for (file, before, after, label) in vectors {
        let mut sources = inputs();
        change(&mut sources, file, before, after);
        let original = sources
            .iter()
            .map(|source| source.bytes().to_vec())
            .collect::<Vec<_>>();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{label} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{label}: {issues:?}"
        );
        assert_eq!(
            original,
            sources
                .iter()
                .map(|source| source.bytes().to_vec())
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_headers_compile_and_link_independent_scalar_and_void_server_signatures() {
    for renamed in [false, true] {
        let mut inputs = multi_operation_inputs();
        if renamed {
            for (before, after) in [
                ("/Application/Process", "/Application/Compute"),
                ("/ProcessInstance", "/ComputeInstance"),
                (">ProcessInstance<", ">ComputeInstance<"),
                (
                    "<SHORT-NAME>Process</SHORT-NAME>",
                    "<SHORT-NAME>Compute</SHORT-NAME>",
                ),
                ("Process_", "Compute_"),
                ("ResultService", "Calculation"),
                ("Transform", "Calculate"),
            ] {
                replace_all(&mut inputs, before, after);
            }
            replace_all(
                &mut inputs,
                "/Application/Compute/Value",
                "/Application/Compute/InputValue",
            );
            xml_edit(
                &mut inputs,
                "process.arxml",
                |node| named(node, "R-PORT-PROTOTYPE", "Value"),
                |original| {
                    original.replace(
                        "<SHORT-NAME>Value</SHORT-NAME>",
                        "<SHORT-NAME>InputValue</SHORT-NAME>",
                    )
                },
            );
        }
        let plan = build(&inputs).unwrap_or_else(|issues| panic!("{issues:?}"));
        let scratch = Scratch::new();
        let directory = scratch.0.join("multi-typed");
        std::fs::create_dir_all(directory.join("include")).unwrap();
        for (file, bytes) in plan.component_contract_files().unwrap().files() {
            std::fs::write(directory.join(file), bytes).unwrap();
        }
        // These definitions and function pointer types are independent of generated metadata.
        let sources = [
            (
                "process.c",
                r#"#include "Rte_Process.h"
static uint32 written;
static uint32 pinged;
void Process_Ping(void) { pinged++; }
void (*const empty_server)(void) = Process_Ping;
void Process_Periodic(void) {}
void Process_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output=Input; *State+=Input; }
void (*const scalar_server)(uint32, uint32 *, uint32 *) = Process_Transform;
Std_ReturnType Rte_Application_Pipeline_ProcessInstance_Read_Value_Value(uint32 *data) { *data=7U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ProcessInstance_Write_Result_Value(uint32 data) { written=data; return E_OK; }
int test_process(void);
int test_process(void) {
    uint32 value=0U, result=0U, state=1U;
    Std_ReturnType (*read_value)(uint32 *)=Rte_Read_Value_Value;
    Std_ReturnType (*write_result)(uint32)=Rte_Write_Result_Value;
    scalar_server(42U,&result,&state);
    empty_server();
    return read_value(&value)==E_OK && value==7U && write_result(43U)==E_OK && written==43U && result==42U && state==43U && pinged==1U ? 0 : 1;
}
"#,
            ),
            (
                "ingress.c",
                r#"#include "Rte_Ingress.h"
static uint32 tx_value, local_value;
void Ingress_Periodic(void) {}
void Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; }
void (*const byte_server)(uint8 *) = Ingress_ReadData;
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Read_RxValue_Value(uint32 *data) { *data=11U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Write_TxValue_Value(uint32 data) { tx_value=data; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_IngressInstance_Write_Value_Value(uint32 data) { local_value=data; return E_OK; }
int test_ingress(void);
int test_ingress(void) {
    uint32 value=0U;
    uint8 bytes[4]={0U,0U,0U,0U};
    byte_server(bytes);
    return Rte_Read_RxValue_Value(&value)==E_OK && value==11U && Rte_Write_TxValue_Value(21U)==E_OK && Rte_Write_Value_Value(31U)==E_OK && tx_value==21U && local_value==31U && bytes[0]==1U && bytes[3]==4U ? 0 : 1;
}
"#,
            ),
            (
                "observe.c",
                r#"#include "Rte_Observe.h"
void Observe_Periodic(void) {}
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Ping(void) { return E_OK; }
Std_ReturnType (*const empty_client)(void) = Rte_Call_ResultService_Ping;
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Read_Value_Value(uint32 *data) { *data=9U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Read_Result_Value(uint32 *data) { *data=19U; return E_OK; }
Std_ReturnType Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output=Input; *State+=Input; return E_OK; }
Std_ReturnType (*const scalar_client)(uint32,uint32 *,uint32 *) = Rte_Call_ResultService_Transform;
int test_observe(void);
int test_observe(void) {
    uint32 result=0U, state=1U, value=0U, read_result=0U;
    return empty_client()==E_OK && scalar_client(42U,&result,&state)==E_OK && result==42U && state==43U && Rte_Read_Value_Value(&value)==E_OK && value==9U && Rte_Read_Result_Value(&read_result)==E_OK && read_result==19U ? 0 : 1;
}
"#,
            ),
            (
                "dcm.c",
                r#"#include "Rte_DcmService.h"
Std_ReturnType Rte_Application_Pipeline_DcmService_Call_DataServices_ApplicationValue_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; return E_OK; }
Std_ReturnType DcmService_ReadData(Dcm_DataElement_ApplicationValueType Data) { return Rte_Call_DataServices_ApplicationValue_ReadData(Data); }
Std_ReturnType (*const byte_client)(uint8 *)=Rte_Call_DataServices_ApplicationValue_ReadData;
Std_ReturnType (*const dcm_bridge)(uint8 *)=DcmService_ReadData;
int test_dcm(void);
int test_dcm(void) {
    uint8 bytes[4]={0U,0U,0U,0U};
    return byte_client(bytes)==E_OK && bytes[0]==1U && bytes[1]==2U && bytes[2]==3U && bytes[3]==4U && dcm_bridge(bytes)==E_OK ? 0 : 1;
}
"#,
            ),
            (
                "main.c",
                r#"int test_process(void);
int test_ingress(void);
int test_observe(void);
int test_dcm(void);
int main(void) { return test_process() || test_ingress() || test_observe() || test_dcm(); }
"#,
            ),
        ];
        for (file, bytes) in sources {
            let (mut file, mut bytes) = (file.to_owned(), bytes.to_owned());
            if renamed {
                for (before, after) in [
                    ("Process", "Compute"),
                    ("ResultService", "Calculation"),
                    ("Transform", "Calculate"),
                ] {
                    file = file.replace(before, after);
                    bytes = bytes.replace(before, after);
                }
                if file == "process.c" {
                    bytes = bytes
                        .replace("Rte_Read_Value_Value", "Rte_Read_InputValue_Value")
                        .replace(
                            "ComputeInstance_Read_Value_Value",
                            "ComputeInstance_Read_InputValue_Value",
                        );
                }
            }
            std::fs::write(directory.join(file), bytes).unwrap();
        }
        let executable = tooling::native_binary(&directory, "typed");
        let mut compiler = std::process::Command::new("gcc");
        compiler
            .current_dir(&directory)
            .args([
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-Iinclude",
                "process.c",
                "ingress.c",
                "observe.c",
                "dcm.c",
                "main.c",
                "-o",
            ])
            .arg(&executable);
        let output = tooling::run_public_command(
            &mut compiler,
            &directory,
            "typed-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = tooling::run_public_command(
            &mut std::process::Command::new(executable),
            &directory,
            "typed-run",
            std::time::Duration::from_secs(30),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}

fn replace_all(sources: &mut [InputSource], before: &str, after: &str) {
    for source in sources {
        let text = std::str::from_utf8(source.bytes()).unwrap();
        *source = InputSource::new(
            source.logical_path(),
            text.replace(before, after).into_bytes(),
        )
        .unwrap();
    }
}

#[test]
fn collisions_are_rejected_with_all_references_still_valid() {
    let vectors = [
        ("Dcm_DataElement_ApplicationValueType", "boolean"),
        (
            "Dcm_DataElement_ApplicationValueType",
            "Std_VersionInfoType",
        ),
        ("Dcm_DataElement_ApplicationValueType", "UINT32_MAX"),
        ("Dcm_DataElement_ApplicationValueType", "uint_fast32_t"),
        ("Process", "ingress"),
        ("Process_Periodic", "Rte_Read_Value_Value"),
        ("Process_Periodic", "Rte_COMCbk"),
        ("Process_Periodic", "Rte_COMCbkRxTOut"),
        ("Process_Periodic", "Com_MainFunctionRx_Rx"),
        ("Process_Periodic", "Com_MainFunctionTx_Tx"),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>uint32</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>Std_ReturnType</SHORT-NAME>",
        ),
        (
            "<SHORT-NAME>Input</SHORT-NAME>",
            "<SHORT-NAME>E_OK</SHORT-NAME>",
        ),
    ];
    for (before, after) in vectors {
        let mut sources = inputs();
        if before == "Process" {
            replace_all(&mut sources, "/Application/Process", "/Application/ingress");
            replace_all(
                &mut sources,
                "<SHORT-NAME>Process</SHORT-NAME>",
                "<SHORT-NAME>ingress</SHORT-NAME>",
            );
        } else {
            replace_all(&mut sources, before, after);
        }
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{after} accepted"));
        assert!(
            issues.iter().any(|issue| matches!(
                issue.code.as_str(),
                "CONTRACT_NAME_COLLISION"
                    | "SYMBOL_NORMALIZATION_COLLISION"
                    | "SYMBOL_PRODUCER_DUPLICATE"
            )),
            "{after}: {issues:?}"
        );
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "REFERENCE_UNRESOLVED"),
            "{after}: dangling references mask collision"
        );
    }
}

#[test]
fn missing_and_duplicate_relations_are_rejected_before_emission() {
    let cases = [
        ("process.arxml", "INIT-VALUE", None, false),
        ("process.arxml", "DATA-TYPE-MAPPING-REFS", None, false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), false),
        ("process.arxml", "TIMING-EVENT", Some("Periodic10ms"), true),
        ("ecuc.arxml", "ECUC-CONTAINER-VALUE", Some("Process"), false),
        (
            "ecuc.arxml",
            "ECUC-CONTAINER-VALUE",
            Some("ReadApplicationValue"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            false,
        ),
        (
            "composition.arxml",
            "ASSEMBLY-SW-CONNECTOR",
            Some("IngressProcess"),
            true,
        ),
    ];
    for (file, tag, short_name, duplicate) in cases {
        let mut sources = inputs();
        let source = sources
            .iter_mut()
            .find(|source| source.logical_path() == file)
            .unwrap();
        let text = std::str::from_utf8(source.bytes()).unwrap();
        let document = roxmltree::Document::parse(text).unwrap();
        let node = document
            .descendants()
            .find(|node| {
                node.tag_name().name() == tag
                    && short_name.is_none_or(|name| {
                        node.children().any(|child| {
                            child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name)
                        })
                    })
            })
            .unwrap();
        let range = node.range();
        let replacement = if duplicate {
            format!(
                "{}{}",
                &text[range.clone()],
                text[range.clone()].replacen(
                    &format!("<SHORT-NAME>{}</SHORT-NAME>", short_name.unwrap()),
                    "<SHORT-NAME>Duplicate</SHORT-NAME>",
                    1
                )
            )
        } else {
            String::new()
        };
        let mutated = format!(
            "{}{}{}",
            &text[..range.start],
            replacement,
            &text[range.end..]
        );
        *source = InputSource::new(file, mutated.into_bytes()).unwrap();
        let issues = build(&sources)
            .err()
            .unwrap_or_else(|| panic!("{file}/{tag} duplicate={duplicate} accepted"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.file.is_some() && issue.object.is_some()),
            "{issues:?}"
        );
    }
}

#[test]
fn renamed_component_and_port_contracts_follow_full_source_identity() {
    let mut sources = inputs();
    replace_all(&mut sources, "Ingress", "Gateway");
    replace_all(
        &mut sources,
        "/Application/Process/Value",
        "/Application/Process/InputValue",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SHORT-NAME>Value</SHORT-NAME>",
        "<SHORT-NAME>InputValue</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let multi = plan.description().multi.as_ref().unwrap();
    let gateway = multi
        .components
        .iter()
        .find(|component| component.component == "/Application/Gateway")
        .unwrap();
    assert_eq!(gateway.instance, "/Application/Pipeline/GatewayInstance");
    assert_eq!(gateway.header, "include/Rte_Gateway.h");
    let process = multi
        .components
        .iter()
        .find(|component| component.component == "/Application/Process")
        .unwrap();
    assert_eq!(
        process
            .data_ports
            .iter()
            .find(|port| port.read)
            .unwrap()
            .api_symbol,
        "Rte_Read_InputValue_Value"
    );
    assert!(
        multi
            .connections
            .iter()
            .any(|connection| connection.requester.port == "/Application/Process/InputValue")
    );
    assert!(
        plan.component_contract_files()
            .unwrap()
            .files()
            .iter()
            .any(|(file, _)| file == "include/Rte_Gateway.h")
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn official_and_native_entry_points_produce_the_same_multi_headers() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let plan = build_plan(
        &inputs(),
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    let native = build(&inputs()).unwrap();
    let official = plan.component_contract_files().unwrap();
    let native = native.component_contract_files().unwrap();
    assert_eq!(
        official
            .files()
            .iter()
            .filter(|(file, _)| file.ends_with(".h"))
            .collect::<Vec<_>>(),
        native
            .files()
            .iter()
            .filter(|(file, _)| file.ends_with(".h"))
            .collect::<Vec<_>>()
    );
}

fn subtree(sources: &[InputSource], file: &str, tag: &str, short_name: &str) -> String {
    let source = sources
        .iter()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let node = document
        .descendants()
        .find(|node| {
            node.tag_name().name() == tag
                && node.children().any(|child| {
                    child.tag_name().name() == "SHORT-NAME" && child.text() == Some(short_name)
                })
        })
        .unwrap();
    text[node.range()].to_owned()
}

#[test]
fn every_call_point_and_actual_server_recursion_are_checked() {
    let mut sources = inputs();
    let call = subtree(
        &sources,
        "observe.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "CallTransform",
    );
    let foreign = call
        .replace("CallTransform", "ForeignCall")
        .replace(
            "/Application/Observe/ResultService",
            "/Services/DcmService/DataServices_ApplicationValue",
        )
        .replace(
            "/Types/ScalarService/Transform",
            "/Types/DataServices_ApplicationValue/ReadData",
        );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{foreign}</SERVER-CALL-POINTS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Foreign second call point accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "CALL_POINT_OWNERSHIP"),
        "{issues:?}"
    );

    let mut sources = inputs();
    let port = subtree(
        &sources,
        "observe.arxml",
        "R-PORT-PROTOTYPE",
        "ResultService",
    )
    .replace(
        "<SHORT-NAME>ResultService</SHORT-NAME>",
        "<SHORT-NAME>LoopClient</SHORT-NAME>",
    );
    change(
        &mut sources,
        "process.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    let recursive_call = call.replace("CallTransform", "RecursiveCall").replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "process.arxml",
        "<SYMBOL>Process_Transform</SYMBOL>",
        &format!(
            "<SERVER-CALL-POINTS>{recursive_call}</SERVER-CALL-POINTS><SYMBOL>Process_Transform</SYMBOL>"
        ),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Scalar",
    )
    .replace(
        "<SHORT-NAME>Scalar</SHORT-NAME>",
        "<SHORT-NAME>Recursive</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/ObserveInstance",
        "/Application/Pipeline/ProcessInstance",
    )
    .replace(
        "/Application/Observe/ResultService",
        "/Application/Process/LoopClient",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let issues = build(&sources)
        .err()
        .expect("Recursive server call accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "SERVICE_RECURSION"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "SERVICE_RECURSION")
    );
}

#[test]
fn equal_width_implementation_types_and_network_local_double_binding_are_rejected() {
    let mut sources = inputs();
    let implementation = subtree(
        &sources,
        "types.arxml",
        "IMPLEMENTATION-DATA-TYPE",
        "uint32",
    )
    .replace(
        "<SHORT-NAME>uint32</SHORT-NAME>",
        "<SHORT-NAME>OtherUint32</SHORT-NAME>",
    );
    let mapping = subtree(
        &sources,
        "types.arxml",
        "DATA-TYPE-MAPPING-SET",
        "ApplicationTypes",
    )
    .replace(
        "<SHORT-NAME>ApplicationTypes</SHORT-NAME>",
        "<SHORT-NAME>OtherTypes</SHORT-NAME>",
    )
    .replace(
        "/Types/uint32</IMPLEMENTATION-DATA-TYPE-REF>",
        "/Types/OtherUint32</IMPLEMENTATION-DATA-TYPE-REF>",
    );
    change(
        &mut sources,
        "types.arxml",
        "</ELEMENTS>",
        &format!("{implementation}{mapping}</ELEMENTS>"),
    );
    change(
        &mut sources,
        "process.arxml",
        "/Types/ApplicationTypes</DATA-TYPE-MAPPING-REF>",
        "/Types/OtherTypes</DATA-TYPE-MAPPING-REF>",
    );
    let issues = build(&sources)
        .err()
        .expect("Equal-width distinct implementation types accepted");
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "TYPE_CONFLICT" || issue.code == "SERVICE_TYPE_CONFLICT"),
        "{issues:?}"
    );
    assert!(
        !issues
            .iter()
            .any(|issue| issue.code == "REFERENCE_UNRESOLVED")
    );

    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "/Application/Pipeline/ProcessInstance",
        "/Application/Pipeline/IngressInstance",
    )
    .replace("/Application/Process/Value", "/Application/Ingress/RxValue");
    let original = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    );
    change(&mut sources, "composition.arxml", &original, &connector);
    let issues = build(&sources)
        .err()
        .expect("Local and network producer accepted");
    assert!(
        issues.iter().any(|issue| issue.code == "ENDPOINT_BINDING"),
        "{issues:?}"
    );
    assert!(
        normal_validation(&sources)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "ENDPOINT_BINDING")
    );
}

#[test]
fn unsigned_contract_rejects_conflicting_base_encodings_with_valid_references() {
    for native in ["uint8", "uint32"] {
        for encoding in ["2C", "IEEE754"] {
            let mut sources = inputs();
            change(
                &mut sources,
                "types.arxml",
                &format!(
                    "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
                &format!(
                    "<BASE-TYPE-ENCODING>{encoding}</BASE-TYPE-ENCODING>\n          <NATIVE-DECLARATION>{native}</NATIVE-DECLARATION>"
                ),
            );
            let issues = build(&sources)
                .err()
                .expect("conflicting encoding rejected");
            assert!(
                issues.iter().any(|issue| issue.code == "TYPE_CONFLICT"),
                "{issues:?}"
            );
            assert!(
                !issues.iter().any(|issue| issue.code.contains("UNRESOLVED")),
                "{issues:?}"
            );
        }
    }
}

fn normal_validation(
    sources: &[InputSource],
) -> autosar_config_core::project_model::ScopeValidation {
    let files = sources
        .iter()
        .map(|source| {
            (
                Path::new(source.logical_path()),
                std::str::from_utf8(source.bytes()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    DefinitionCatalog::builtin()
        .unwrap()
        .validate_documents(&files)
        .unwrap()
}

fn xml_edit(
    sources: &mut [InputSource],
    file: &str,
    select: impl Fn(roxmltree::Node<'_, '_>) -> bool,
    replacement: impl Fn(&str) -> String,
) {
    let source = sources
        .iter_mut()
        .find(|source| source.logical_path() == file)
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let range = document
        .descendants()
        .find(|node| select(*node))
        .unwrap()
        .range();
    let changed = format!(
        "{}{}{}",
        &text[..range.start],
        replacement(&text[range.clone()]),
        &text[range.end..]
    );
    *source = InputSource::new(file, changed.into_bytes()).unwrap();
}

fn named(node: roxmltree::Node<'_, '_>, tag: &str, name: &str) -> bool {
    node.tag_name().name() == tag
        && node
            .children()
            .any(|child| child.tag_name().name() == "SHORT-NAME" && child.text() == Some(name))
}

fn parameter(sources: &mut [InputSource], owner: &str, definition: &str, new_value: &str) {
    xml_edit(
        sources,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "VALUE"
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.tag_name().name() == "DEFINITION-REF"
                            && child
                                .text()
                                .is_some_and(|text| text.ends_with(&format!("/{definition}")))
                    })
                })
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", owner))
        },
        |_| format!("<VALUE>{new_value}</VALUE>"),
    );
}

fn rejects_in_both(sources: &[InputSource], code: &str) {
    let issues = build(sources)
        .err()
        .unwrap_or_else(|| panic!("{code} accepted"));
    assert!(issues.iter().any(|issue| issue.code == code), "{issues:?}");
    let validation = normal_validation(sources);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == code && issue.file.is_some() && issue.path.is_some()),
        "{code}: {:?}",
        validation.diagnostics
    );
}

#[test]
fn normal_definition_validation_closes_multi_schedule_routes_types_and_handles() {
    let validation = normal_validation(&inputs());
    assert!(
        validation.diagnostics.is_empty(),
        "{:?}",
        validation.diagnostics
    );
    for (owner, definition, value, code) in [
        ("Observe", "RtePositionInTask", "5", "SCHEDULE_NOT_UNIQUE"),
        (
            "ReadApplicationValue",
            "RteEventIsMappedToTask",
            "true",
            "SERVICE_ASYNC_MAPPING",
        ),
        ("RxValue", "CanIfRxPduId", "2", "CAN_ID_CONFLICT"),
        (
            "RxValueSource",
            "PduRSrcPduUpTxConf",
            "false",
            "PDU_ROUTE_CONFIRMATION",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, definition, value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "composition.arxml",
        "/Application/Process/Value</TARGET-R-PORT-REF>",
        "/Application/Process/Result</TARGET-R-PORT-REF>",
    );
    rejects_in_both(&sources, "REFERENCE_DEST");
    let mut sources = inputs();
    change(
        &mut sources,
        "types.arxml",
        "<BASE-TYPE-ENCODING>NONE</BASE-TYPE-ENCODING>",
        "<BASE-TYPE-ENCODING>2C</BASE-TYPE-ENCODING>",
    );
    rejects_in_both(&sources, "TYPE_CONFLICT");
    let mut sources = inputs();
    parameter(&mut sources, "TxValue", "CanIfTxPduId", "0");
    assert!(
        build(&sources).is_ok(),
        "same handle in different directions must be allowed"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn network_freshness_and_runnable_execution_constraints_are_explicit() {
    for (before, after) in [
        (
            "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
        ),
        (
            "<HANDLE-TIMEOUT-TYPE>NONE</HANDLE-TIMEOUT-TYPE>",
            "<HANDLE-TIMEOUT-TYPE>REPLACE</HANDLE-TIMEOUT-TYPE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "ingress.arxml", before, after);
        rejects_in_both(&sources, "NETWORK_FRESHNESS_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    assert!(build(&sources).is_ok());
    change(
        &mut sources,
        "process.arxml",
        "<MINIMUM-START-INTERVAL>0",
        "<MINIMUM-START-INTERVAL>0.001",
    );
    rejects_in_both(&sources, "MINIMUM_START_INTERVAL_UNSUPPORTED");
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<EVENTS>",
        "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
    );
    change(
        &mut sources,
        "process.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
    );
    let issues = build(&sources).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.file.as_deref() == Some("process.arxml")),
        "{issues:?}"
    );
    // The bounded normal grammar rejects this official declaration before semantic planning.
    assert!(!normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn dcm_bridge_requires_actual_service_identity_and_follows_renames() {
    for (before, after) in [
        (
            "<SERVICE-KIND>DIAGNOSTIC-COMMUNICATION-MANAGER</SERVICE-KIND>",
            "<SERVICE-KIND>COM-MANAGER</SERVICE-KIND>",
        ),
        (
            "<IS-SERVICE>true</IS-SERVICE>",
            "<IS-SERVICE>false</IS-SERVICE>",
        ),
    ] {
        let mut sources = inputs();
        change(&mut sources, "types.arxml", before, after);
        rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    }
    let mut sources = inputs();
    replace_all(
        &mut sources,
        "/Services/DcmService/DataServices_ApplicationValue",
        "/Services/DcmService/IncorrectPort",
    );
    change(
        &mut sources,
        "services.arxml",
        "<SHORT-NAME>DataServices_ApplicationValue</SHORT-NAME>",
        "<SHORT-NAME>IncorrectPort</SHORT-NAME>",
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
    let mut sources = inputs();
    replace_all(&mut sources, "ApplicationValue", "VehicleValue");
    replace_all(&mut sources, "DcmService", "DiagnosticClient");
    // The application provider port is identified by its connector, not by the DID name.
    replace_all(
        &mut sources,
        "/Application/Ingress/VehicleValue",
        "/Application/Ingress/SnapshotRead",
    );
    change(
        &mut sources,
        "ingress.arxml",
        "<SHORT-NAME>VehicleValue</SHORT-NAME>",
        "<SHORT-NAME>SnapshotRead</SHORT-NAME>",
    );
    let plan = build(&sources).unwrap();
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DiagnosticClient/DataServices_VehicleValue"
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn network_routing_is_checked_without_optional_did() {
    let mut sources = inputs();
    for name in [
        "ApplicationValue",
        "ApplicationDid",
        "DcmService",
        "DiagRequest",
        "DiagResponse",
        "DiagRequestSource",
        "DiagRequestDestination",
        "DiagResponseSource",
        "DiagResponseDestination",
    ] {
        xml_edit(
            &mut sources,
            "ecuc.arxml",
            |node| {
                named(node, "ECUC-CONTAINER-VALUE", name)
                    && (!name.starts_with("Diag")
                        || node.children().any(|child| {
                            child.tag_name().name() == "DEFINITION-REF"
                                && child.text().is_some_and(|text| text.contains("/PduR/"))
                        }))
            },
            |_| String::new(),
        );
    }
    xml_edit(
        &mut sources,
        "composition.arxml",
        |node| named(node, "SW-COMPONENT-PROTOTYPE", "DcmService"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "composition.arxml",
        |node| named(node, "ASSEMBLY-SW-CONNECTOR", "Diagnostic"),
        |_| String::new(),
    );
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| {
            node.tag_name().name() == "COMPONENT-IREF"
                && node
                    .descendants()
                    .any(|child| child.text() == Some("/Application/Pipeline/DcmService"))
        },
        |_| String::new(),
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(plan.description().diagnostic.is_none());
    assert_eq!(plan.description().routes.len(), 2);
    assert!(normal_validation(&sources).diagnostics.is_empty());
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "RxValue")
                && node.children().any(|child| {
                    child
                        .text()
                        .is_some_and(|text| text.ends_with("/PduRRoutingPath"))
                })
        },
        |_| String::new(),
    );
    rejects_in_both(&sources, "PDU_ROUTE_NOT_UNIQUE");
}

#[test]
fn ordinary_application_client_sharing_did_server_is_not_the_dcm_bridge() {
    let mut sources = inputs();
    let port = subtree(
        &sources,
        "services.arxml",
        "R-PORT-PROTOTYPE",
        "DataServices_ApplicationValue",
    );
    let call = subtree(
        &sources,
        "services.arxml",
        "SYNCHRONOUS-SERVER-CALL-POINT",
        "ReadApplicationValue",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "observe.arxml",
        "</PORTS>",
        &format!("{port}</PORTS>"),
    );
    change(
        &mut sources,
        "observe.arxml",
        "</SERVER-CALL-POINTS>",
        &format!("{call}</SERVER-CALL-POINTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>ApplicationDiagnostic</SHORT-NAME>",
    )
    .replace(
        "/Application/Pipeline/DcmService",
        "/Application/Pipeline/ObserveInstance",
    )
    .replace("/Services/DcmService", "/Application/Observe");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description().diagnostic.as_ref().unwrap().client_port,
        "/Services/DcmService/DataServices_ApplicationValue"
    );
    assert_eq!(
        plan.description()
            .multi
            .as_ref()
            .unwrap()
            .connections
            .iter()
            .filter(|connection| connection.service
                && connection.provider.port == "/Application/Ingress/ApplicationValue")
            .count(),
        2
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn two_real_dcm_requesters_are_rejected_without_dangling_references() {
    let mut sources = inputs();
    let component = subtree(
        &sources,
        "services.arxml",
        "SERVICE-SW-COMPONENT-TYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "services.arxml",
        "</ELEMENTS>",
        &format!("{component}</ELEMENTS>"),
    );
    let prototype = subtree(
        &sources,
        "composition.arxml",
        "SW-COMPONENT-PROTOTYPE",
        "DcmService",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</COMPONENTS>",
        &format!("{prototype}</COMPONENTS>"),
    );
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "Diagnostic",
    )
    .replace(
        "<SHORT-NAME>Diagnostic</SHORT-NAME>",
        "<SHORT-NAME>SecondDiagnostic</SHORT-NAME>",
    )
    .replace("DcmService", "DcmSecond");
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    let mapping = "<COMPONENT-IREF><CONTEXT-COMPOSITION-REF DEST=\"ROOT-SW-COMPOSITION-PROTOTYPE\">/Extract/ReferenceExtract/RootComposition</CONTEXT-COMPOSITION-REF><TARGET-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/DcmSecond</TARGET-COMPONENT-REF></COMPONENT-IREF>";
    change(
        &mut sources,
        "extract.arxml",
        "</COMPONENT-IREFS>",
        &format!("{mapping}</COMPONENT-IREFS>"),
    );
    let rte_instance = subtree(&sources, "ecuc.arxml", "ECUC-CONTAINER-VALUE", "DcmService");
    change(
        &mut sources,
        "ecuc.arxml",
        &rte_instance,
        &format!(
            "{}{}",
            rte_instance,
            rte_instance.replace("DcmService", "DcmSecond")
        ),
    );
    rejects_in_both(&sources, "SERVICE_CLIENT_MISSING");
}

#[cfg(feature = "official-oracles")]
#[test]
fn execution_constraint_rejections_use_officially_valid_arxml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for exclusive_area in [false, true] {
        let mut sources = inputs();
        if exclusive_area {
            change(
                &mut sources,
                "process.arxml",
                "<EVENTS>",
                "<EXCLUSIVE-AREAS><EXCLUSIVE-AREA><SHORT-NAME>Lock</SHORT-NAME></EXCLUSIVE-AREA></EXCLUSIVE-AREAS><EVENTS>",
            );
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-ENTER-EXCLUSIVE-AREA-REF DEST=\"EXCLUSIVE-AREA\">/Application/Process/Behavior/Lock</CAN-ENTER-EXCLUSIVE-AREA-REF></CAN-ENTER-EXCLUSIVE-AREA-REFS><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        } else {
            change(
                &mut sources,
                "process.arxml",
                "<CAN-BE-INVOKED-CONCURRENTLY>false",
                "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>false",
            );
        }
        let files = sources
            .iter()
            .map(|source| {
                (
                    Path::new(source.logical_path()),
                    std::str::from_utf8(source.bytes()).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let issues = autosar_config_core::schema::validate_files(
            &autosar_config_core::schema::schema_archive(root),
            &files,
        )
        .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert!(build(&sources).is_err());
        assert!(!normal_validation(&sources).diagnostics.is_empty());
    }
}

#[test]
fn normal_definition_validation_keeps_duplicate_producer_diagnostics() {
    let mut sources = inputs();
    let connector = subtree(
        &sources,
        "composition.arxml",
        "ASSEMBLY-SW-CONNECTOR",
        "IngressProcess",
    )
    .replace(
        "<SHORT-NAME>IngressProcess</SHORT-NAME>",
        "<SHORT-NAME>SecondProducer</SHORT-NAME>",
    );
    change(
        &mut sources,
        "composition.arxml",
        "</CONNECTORS>",
        &format!("{connector}</CONNECTORS>"),
    );
    rejects_in_both(&sources, "MULTIPLE_PRODUCERS");
}

fn multi_operation_inputs() -> Vec<InputSource> {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "CLIENT-SERVER-OPERATION", "Transform"),
        |original| {
            format!(
                "{original}<CLIENT-SERVER-OPERATION><SHORT-NAME>Ping</SHORT-NAME></CLIENT-SERVER-OPERATION>"
            )
        },
    );
    for (file, tag) in [
        ("process.arxml", "SERVER-COM-SPEC"),
        ("observe.arxml", "CLIENT-COM-SPEC"),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| node.tag_name().name() == tag,
            |original| format!("{original}{}", original.replace("Transform", "Ping")),
        );
    }
    for (file, tag, name) in [
        (
            "process.arxml",
            "OPERATION-INVOKED-EVENT",
            "InvokeTransform",
        ),
        ("process.arxml", "RUNNABLE-ENTITY", "ServerTransform"),
        (
            "observe.arxml",
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "CallTransform",
        ),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| named(node, tag, name),
            |original| format!("{original}{}", original.replace("Transform", "Ping")),
        );
    }
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "ReadApplicationValue")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Process"))
        },
        |original| {
            format!(
                "{original}{}",
                original
                    .replace("ReadApplicationValue", "InvokePing")
                    .replace("InvokeTransform", "InvokePing")
            )
        },
    );
    sources
}

#[test]
fn client_server_sets_close_in_both_directions_and_empty_interfaces_are_rejected() {
    for file in ["process.arxml", "observe.arxml"] {
        for foreign in [false, true] {
            let mut sources = inputs();
            let tag = if file == "process.arxml" {
                "SERVER-COM-SPEC"
            } else {
                "CLIENT-COM-SPEC"
            };
            xml_edit(
                &mut sources,
                file,
                |node| node.tag_name().name() == tag,
                |original| {
                    format!(
                        "{original}{}",
                        if foreign {
                            original.replace(
                                "/Types/ScalarService/Transform",
                                "/Types/DataServices_ApplicationValue/ReadData",
                            )
                        } else {
                            original.into()
                        }
                    )
                },
            );
            rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
        }
    }
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "OPERATION-INVOKED-EVENT", "InvokeTransform"),
        |original| {
            format!(
                "{original}<OPERATION-INVOKED-EVENT><SHORT-NAME>InvokeUnbound</SHORT-NAME><START-ON-EVENT-REF DEST=\"RUNNABLE-ENTITY\">/Application/Process/Behavior/Unbound</START-ON-EVENT-REF><OPERATION-IREF><CONTEXT-P-PORT-REF DEST=\"P-PORT-PROTOTYPE\">/Application/Process/ResultService</CONTEXT-P-PORT-REF><TARGET-PROVIDED-OPERATION-REF DEST=\"CLIENT-SERVER-OPERATION\">/Types/DataServices_ApplicationValue/ReadData</TARGET-PROVIDED-OPERATION-REF></OPERATION-IREF></OPERATION-INVOKED-EVENT>"
            )
        },
    );
    change(
        &mut sources,
        "process.arxml",
        "</RUNNABLES>",
        "<RUNNABLE-ENTITY><SHORT-NAME>Unbound</SHORT-NAME><CAN-BE-INVOKED-CONCURRENTLY>false</CAN-BE-INVOKED-CONCURRENTLY><SYMBOL>Process_Unbound</SYMBOL></RUNNABLE-ENTITY></RUNNABLES>",
    );
    rejects_in_both(&sources, "EVENT_OWNERSHIP");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "types.arxml",
        |node| named(node, "CLIENT-SERVER-INTERFACE", "ScalarService"),
        |_| {
            "<CLIENT-SERVER-INTERFACE><SHORT-NAME>ScalarService</SHORT-NAME><IS-SERVICE>false</IS-SERVICE></CLIENT-SERVER-INTERFACE>".into()
        },
    );
    for (file, tag) in [
        ("process.arxml", "PROVIDED-COM-SPECS"),
        ("observe.arxml", "REQUIRED-COM-SPECS"),
        ("process.arxml", "OPERATION-IREF"),
        ("observe.arxml", "OPERATION-IREF"),
    ] {
        xml_edit(
            &mut sources,
            file,
            |node| {
                node.tag_name().name() == tag
                    && (tag == "OPERATION-IREF"
                        || node.ancestors().any(|ancestor| {
                            named(ancestor, "P-PORT-PROTOTYPE", "ResultService")
                                || named(ancestor, "R-PORT-PROTOTYPE", "ResultService")
                        }))
            },
            |_| String::new(),
        );
    }
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
}

#[test]
fn every_operation_has_its_own_spec_call_and_event_and_void_is_valid() {
    let sources = multi_operation_inputs();
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap();
    let files = plan.component_contract_files().unwrap();
    assert!(
        std::str::from_utf8(
            &files
                .files()
                .iter()
                .find(|(path, _)| path == "include/Rte_Process.h")
                .unwrap()
                .1
        )
        .unwrap()
        .contains("void Process_Ping(void);")
    );
    for (file, tag, code) in [
        ("process.arxml", "SERVER-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        ("observe.arxml", "CLIENT-COM-SPEC", "SERVICE_TYPE_CONFLICT"),
        (
            "observe.arxml",
            "SYNCHRONOUS-SERVER-CALL-POINT",
            "SERVICE_CLIENT_MISSING",
        ),
        (
            "process.arxml",
            "OPERATION-INVOKED-EVENT",
            "SERVICE_CLIENT_MISSING",
        ),
    ] {
        let mut changed = sources.clone();
        xml_edit(
            &mut changed,
            file,
            |node| {
                node.tag_name().name() == tag
                    && node.descendants().any(|child| {
                        child
                            .text()
                            .is_some_and(|text| text.ends_with("/Ping") || text == "InvokePing")
                    })
            },
            |_| String::new(),
        );
        if tag == "OPERATION-INVOKED-EVENT" {
            xml_edit(
                &mut changed,
                "ecuc.arxml",
                |node| named(node, "ECUC-CONTAINER-VALUE", "InvokePing"),
                |_| String::new(),
            );
            xml_edit(
                &mut changed,
                "process.arxml",
                |node| named(node, "RUNNABLE-ENTITY", "ServerPing"),
                |_| String::new(),
            );
        }
        rejects_in_both(&changed, code);
    }
}

#[test]
fn argument_policy_explicit_task_flags_offset_and_concurrency_are_checked() {
    let mut sources = inputs();
    change(&mut sources, "types.arxml", "USE-ARGUMENT-TYPE", "USE-VOID");
    rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    for prefix in ["Rte", "RteBsw"] {
        for remove in [false, true] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "ecuc.arxml",
                |node| {
                    node.tag_name().name() == "ECUC-NUMERICAL-PARAM-VALUE"
                        && node.children().any(|child| {
                            child.text().is_some_and(|text| {
                                text.ends_with(&format!("/{prefix}EventIsMappedToTask"))
                            })
                        })
                        && node.children().any(|child| child.text() == Some("true"))
                },
                |original| {
                    if remove {
                        String::new()
                    } else {
                        original.replace("<VALUE>true</VALUE>", "<VALUE>false</VALUE>")
                    }
                },
            );
            rejects_in_both(&sources, "INSTANCE_MAPPING");
        }
    }
    for name in ["Periodic", "ServerTransform"] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "process.arxml",
            |node| named(node, "RUNNABLE-ENTITY", name),
            |original| {
                original.replace(
                    "<CAN-BE-INVOKED-CONCURRENTLY>false",
                    "<CAN-BE-INVOKED-CONCURRENTLY>true",
                )
            },
        );
        rejects_in_both(&sources, "REENTRANCY_UNSUPPORTED");
    }
    let mut sources = inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    assert!(build(&sources).is_ok());
    assert!(normal_validation(&sources).diagnostics.is_empty());
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    rejects_in_both(&sources, "PERIOD_UNSUPPORTED");
}

#[test]
fn contract_provenance_preserves_complete_rule_and_extension_identities() {
    let plan = build(&inputs()).unwrap();
    let files = plan.component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(path, _)| path == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        json["ruleSetIdentity"],
        serde_json::to_value(&plan.description().rule_set_identity).unwrap()
    );
    assert_eq!(
        json["requiredExtensionDefinitions"],
        serde_json::to_value(&plan.description().required_extension_definitions).unwrap()
    );
    assert_eq!(
        json["validationDependencies"],
        serde_json::to_value(&plan.description().validation_dependencies).unwrap()
    );
}

#[cfg(feature = "official-oracles")]
#[test]
fn zero_offset_and_multiple_operations_are_accepted_by_official_and_native_inputs() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut sources = multi_operation_inputs();
    change(
        &mut sources,
        "process.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0</OFFSET><PERIOD>0.01</PERIOD>",
    );
    let official = build_plan(
        &sources,
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let native = build(&sources).unwrap();
    let official_files = official.component_contract_files().unwrap();
    let native_files = native.component_contract_files().unwrap();
    assert_eq!(
        official_files
            .files()
            .iter()
            .filter(|(path, _)| path.ends_with(".h"))
            .collect::<Vec<_>>(),
        native_files
            .files()
            .iter()
            .filter(|(path, _)| path.ends_with(".h"))
            .collect::<Vec<_>>()
    );
    change(
        &mut sources,
        "process.arxml",
        "<OFFSET>0</OFFSET>",
        "<OFFSET>0.001</OFFSET>",
    );
    let errors = build_plan(
        &sources,
        &PlanDependencies::from_repository(root),
        &RuntimeCatalog::from_repository(root).unwrap(),
    )
    .err()
    .unwrap();
    assert!(
        errors
            .iter()
            .any(|issue| issue.code == "PERIOD_UNSUPPORTED"),
        "{errors:?}"
    );
}

#[test]
fn unsupported_edit_and_handoff_preserve_live_inputs_and_existing_output() {
    use autosar_config_core::{Workspace, integration::IntegrationEdit, target::BuildTarget};
    let scratch = Scratch::new();
    let source_directory = scratch.0.join("source");
    std::fs::create_dir(&source_directory).unwrap();
    let inputs = inputs();
    let paths: Vec<_> = inputs
        .iter()
        .map(|source| {
            let path = source_directory.join(source.logical_path());
            std::fs::write(&path, source.bytes()).unwrap();
            path
        })
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let mut workspace = Workspace::open(paths.clone()).unwrap();
    let before = serde_json::to_value(&workspace.project_projection("same").unwrap()).unwrap();
    let errors = workspace
        .edit_integration(
            &runtime,
            IntegrationEdit {
                application_period_ms: Some(20),
                ..IntegrationEdit::default()
            },
        )
        .err()
        .unwrap();
    assert!(errors.iter().any(|issue| issue.code == "EDIT_UNSUPPORTED"));
    assert_eq!(
        before,
        serde_json::to_value(&workspace.project_projection("same").unwrap()).unwrap()
    );
    let plan = build(&inputs).unwrap();
    let contract = plan.component_contract_files().unwrap();
    let output = scratch.0.join("contract");
    let preview = contract.preview(&output).unwrap();
    contract
        .generate_previewed(&output, &preview.revision)
        .unwrap();
    assert!(
        plan.ecu_handoff_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    assert!(
        plan.ecu_integration_files(BuildTarget::LinuxX64ControlledV1)
            .is_err()
    );
    for (file, bytes) in contract.files() {
        assert_eq!(&std::fs::read(output.join(file)).unwrap(), bytes);
    }
    for (source, path) in inputs.iter().zip(paths) {
        assert_eq!(std::fs::read(path).unwrap(), source.bytes());
    }
}

#[test]
fn type_categories_are_required_with_valid_references_in_both_entrances() {
    for category in [None, Some("VALUE")] {
        let mut sources = inputs();
        xml_edit(
            &mut sources,
            "types.arxml",
            |node| node.tag_name().name() == "IMPLEMENTATION-DATA-TYPE-ELEMENT",
            |original| {
                original.replace(
                    "<CATEGORY>TYPE_REFERENCE</CATEGORY>",
                    &category
                        .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                        .unwrap_or_default(),
                )
            },
        );
        rejects_in_both(&sources, "SERVICE_TYPE_CONFLICT");
    }
    for base in ["Base_uint8", "Base_uint32"] {
        for category in [None, Some("VARIABLE_LENGTH")] {
            let mut sources = inputs();
            xml_edit(
                &mut sources,
                "types.arxml",
                |node| named(node, "SW-BASE-TYPE", base),
                |original| {
                    original.replace(
                        "<CATEGORY>FIXED_LENGTH</CATEGORY>",
                        &category
                            .map(|value| format!("<CATEGORY>{value}</CATEGORY>"))
                            .unwrap_or_default(),
                    )
                },
            );
            rejects_in_both(&sources, "TYPE_CONFLICT");
        }
    }
}

#[test]
fn flat_signal_context_and_ecu_members_are_unique_and_nonempty() {
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "DATA-ELEMENT-IREF",
        |original| {
            original.replace("</CONTEXT-COMPONENT-REF>", "</CONTEXT-COMPONENT-REF><CONTEXT-COMPONENT-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/Pipeline/ProcessInstance</CONTEXT-COMPONENT-REF>")
        },
    );
    rejects_in_both(&sources, "REFERENCE_UNRESOLVED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "COMPONENT-IREF",
        |original| format!("{original}{original}"),
    );
    rejects_in_both(&sources, "ECU_MAPPING");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "extract.arxml",
        |node| node.tag_name().name() == "SWC-TO-ECU-MAPPING",
        |original| {
            format!(
                "{original}<SWC-TO-ECU-MAPPING><SHORT-NAME>Empty</SHORT-NAME><ECU-INSTANCE-REF DEST=\"ECU-INSTANCE\">/Extract/ReferenceEcu</ECU-INSTANCE-REF></SWC-TO-ECU-MAPPING>"
            )
        },
    );
    rejects_in_both(&sources, "ECU_MAPPING");
}

#[test]
fn multi_network_zero_timeout_matches_without_changing_runtime_scope() {
    let mut sources = inputs();
    change(
        &mut sources,
        "ingress.arxml",
        "<ALIVE-TIMEOUT>0.03</ALIVE-TIMEOUT>",
        "<ALIVE-TIMEOUT>0</ALIVE-TIMEOUT>",
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0");
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let plan = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        plan.description()
            .signals
            .iter()
            .find(|signal| signal.receive)
            .unwrap()
            .deadline_ms,
        Some(0)
    );
    parameter(&mut sources, "RxValue", "ComTimeout", "0.02");
    rejects_in_both(&sources, "TIMEOUT_CONFLICT");
}

fn legacy_inputs() -> Vec<InputSource> {
    let scratch = Scratch::new();
    let preview = autosar_config_core::Workspace::preview_project_creation(
        &scratch.0.join("legacy"),
        "Legacy",
        "standard-ecu-v1",
    )
    .unwrap();
    let mut sources: Vec<_> = preview
        .files
        .into_iter()
        .filter(|file| file.path.ends_with(".arxml"))
        .map(|file| InputSource::new(file.path, file.contents.into_bytes()).unwrap())
        .collect();
    sources.sort_by(|left, right| left.logical_path().cmp(right.logical_path()));
    sources
}

fn legacy_execution_variants() -> Vec<Vec<InputSource>> {
    let mut result = Vec::new();
    let mut sources = legacy_inputs();
    change(
        &mut sources,
        "application.arxml",
        "<PERIOD>0.01</PERIOD>",
        "<OFFSET>0.001</OFFSET><PERIOD>0.01</PERIOD>",
    );
    result.push(sources);
    for file in ["application.arxml", "services.arxml"] {
        let source = legacy_inputs();
        let text = std::str::from_utf8(
            source
                .iter()
                .find(|source| source.logical_path() == file)
                .unwrap()
                .bytes(),
        )
        .unwrap();
        let names: Vec<_> = roxmltree::Document::parse(text)
            .unwrap()
            .descendants()
            .filter(|node| node.tag_name().name() == "RUNNABLE-ENTITY")
            .map(|node| {
                node.children()
                    .find(|child| child.tag_name().name() == "SHORT-NAME")
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        for name in names {
            let mut sources = legacy_inputs();
            xml_edit(
                &mut sources,
                file,
                |node| named(node, "RUNNABLE-ENTITY", &name),
                |original| {
                    original.replace("<CAN-BE-INVOKED-CONCURRENTLY>", "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL><CAN-BE-INVOKED-CONCURRENTLY>")
                },
            );
            result.push(sources);
        }
    }
    result
}

#[test]
fn legacy_native_rejects_nonzero_execution_constraints_and_keeps_exact_zero() {
    for (index, sources) in legacy_execution_variants().into_iter().enumerate() {
        let expected = if index == 0 {
            "OFFSET_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        let issues = build(&sources)
            .err()
            .expect("legacy execution constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
        let mut zero = sources;
        replace_all(&mut zero, "<OFFSET>0.001</OFFSET>", "<OFFSET>0e1</OFFSET>");
        replace_all(
            &mut zero,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0.000</MINIMUM-START-INTERVAL>",
        );
        assert!(build(&zero).is_ok());
        assert!(normal_validation(&zero).diagnostics.is_empty());
    }
}

#[cfg(feature = "official-oracles")]
#[test]
fn legacy_official_legal_inputs_reject_nonzero_execution_constraints() {
    use autosar_config_core::integration::{PlanDependencies, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    for (index, sources) in legacy_execution_variants().into_iter().enumerate() {
        let expected = if index == 0 {
            "OFFSET_UNSUPPORTED"
        } else {
            "MINIMUM_START_INTERVAL_UNSUPPORTED"
        };
        let issues = build_plan(&sources, &dependencies, &runtime)
            .err()
            .expect("legacy constraint accepted");
        assert!(
            issues.iter().any(|issue| issue.code == expected),
            "{issues:?}"
        );
    }
}

#[test]
fn serialized_symbol_consumers_include_all_runnable_accesses_and_client_callers() {
    let mut sources = inputs();
    let accesses = format!(
        "<DATA-RECEIVE-POINT-BY-ARGUMENTS>{}</DATA-RECEIVE-POINT-BY-ARGUMENTS>",
        subtree(&sources, "process.arxml", "VARIABLE-ACCESS", "RValue")
    );
    xml_edit(
        &mut sources,
        "process.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "ServerTransform"),
        |original| original.replace("<SYMBOL>", &format!("{accesses}<SYMBOL>")),
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "RUNNABLE-ENTITY", "Periodic"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic</SHORT-NAME>",
                        "<SHORT-NAME>ExtraPeriodic</SHORT-NAME>"
                    )
                    .replace(
                        "<SYMBOL>Observe_Periodic</SYMBOL>",
                        "<SYMBOL>Observe_ExtraPeriodic</SYMBOL>"
                    )
            )
        },
    );
    xml_edit(
        &mut sources,
        "observe.arxml",
        |node| named(node, "TIMING-EVENT", "Periodic10ms"),
        |original| {
            format!(
                "{original}{}",
                original
                    .replace("Periodic10ms", "Extra10ms")
                    .replace("/Behavior/Periodic", "/Behavior/ExtraPeriodic")
            )
        },
    );
    parameter(&mut sources, "Transmit10ms", "RteBswPositionInTask", "8");
    parameter(&mut sources, "Dcm", "RteBswPositionInTask", "9");
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "Periodic10ms")
                && node
                    .ancestors()
                    .any(|ancestor| named(ancestor, "ECUC-CONTAINER-VALUE", "Observe"))
        },
        |original| {
            format!(
                "{original}{}",
                original
                    .replace(
                        "<SHORT-NAME>Periodic10ms</SHORT-NAME>",
                        "<SHORT-NAME>Extra10ms</SHORT-NAME>"
                    )
                    .replace(
                        "/Observe/Behavior/Periodic10ms",
                        "/Observe/Behavior/Extra10ms"
                    )
                    .replace("<VALUE>6</VALUE>", "<VALUE>7</VALUE>")
            )
        },
    );
    assert!(normal_validation(&sources).diagnostics.is_empty());
    let files = build(&sources).unwrap().component_contract_files().unwrap();
    let json: serde_json::Value = serde_json::from_slice(
        &files
            .files()
            .iter()
            .find(|(file, _)| file == "contract.json")
            .unwrap()
            .1,
    )
    .unwrap();
    for (symbol, expected) in [
        (
            "Rte_Application_Pipeline_ProcessInstance_Read_Value_Value",
            vec![
                "/Application/Process/Behavior/Periodic",
                "/Application/Process/Behavior/ServerTransform",
            ],
        ),
        (
            "Rte_Application_Pipeline_ObserveInstance_Call_ResultService_Transform",
            vec![
                "/Application/Observe/Behavior/ExtraPeriodic",
                "/Application/Observe/Behavior/Periodic",
            ],
        ),
    ] {
        let actual = json["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["symbol"] == symbol)
            .unwrap()["consumers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry.as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected.into_iter().collect());
    }
}

#[test]
fn normal_legacy_definition_validation_rejects_only_new_nonzero_execution_constraints() {
    assert!(normal_validation(&legacy_inputs()).diagnostics.is_empty());
    let expected = [
        (
            "OFFSET_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/Periodic10ms",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/PeriodicRunnable",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "application.arxml",
            "/Application/EchoApplication/Behavior/ReadDataRunnable",
        ),
        (
            "MINIMUM_START_INTERVAL_UNSUPPORTED",
            "services.arxml",
            "/Services/DcmService/Behavior/DcmReadData",
        ),
    ];
    let cases = legacy_execution_variants();
    assert_eq!(cases.len(), expected.len());
    for (mut sources, (code, file, path)) in cases.into_iter().zip(expected) {
        let validation = normal_validation(&sources);
        assert!(
            validation.diagnostics.iter().any(|issue| issue.code == code
                && issue.file.as_deref() == Some(file)
                && issue.path.as_deref() == Some(path)),
            "{validation:?}"
        );
        let issues = build(&sources).err().unwrap();
        assert!(issues.iter().any(|issue| issue.code == code), "{issues:?}");
        replace_all(&mut sources, "<OFFSET>0.001</OFFSET>", "<OFFSET>0</OFFSET>");
        replace_all(
            &mut sources,
            "<MINIMUM-START-INTERVAL>0.001</MINIMUM-START-INTERVAL>",
            "<MINIMUM-START-INTERVAL>0</MINIMUM-START-INTERVAL>",
        );
        assert!(normal_validation(&sources).diagnostics.is_empty());
        assert!(build(&sources).is_ok());
    }
    // A capability flag does not change the actual historical graph.
    // Definition editing stays valid; native generation keeps its own refusal.
    let mut unrelated = legacy_inputs();
    change(
        &mut unrelated,
        "application.arxml",
        "<SUPPORTS-MULTIPLE-INSTANTIATION>false",
        "<SUPPORTS-MULTIPLE-INSTANTIATION>true",
    );
    let validation = normal_validation(&unrelated);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(
        validation.status,
        autosar_config_core::project_model::ValidationStatus::Unsupported
    );
    assert!(validation.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    assert!(
        build(&unrelated)
            .err()
            .unwrap()
            .iter()
            .any(|issue| issue.code == "MULTIPLE_INSTANCES")
    );
}

#[test]
fn communication_types_and_polling_periods_are_source_derived() {
    let plan = build(&inputs()).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = plan.description().communication_runtime.as_ref().unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT16");
    assert_eq!(json["pduLengthType"], "UINT16");
    assert_eq!(
        runtime.can.read_write_period,
        "/Configuration/Can/General/Polling"
    );
    assert_eq!(runtime.can.read_write_period_ms, 1);
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 0);
    assert_eq!(runtime.can.transmit_handle, 1);
    assert_eq!(runtime.can.busoff_period_ms, 1);
    assert_eq!(runtime.can.mode_period_ms, 1);
    assert!(runtime.can.receive_polling && runtime.can.transmit_polling);
    assert!(runtime.can.busoff_polling && runtime.can.wakeup_polling);
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT32");
    replace_all(&mut sources, "/General/Polling", "/General/PollCycle");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Polling</SHORT-NAME>",
        "<SHORT-NAME>PollCycle</SHORT-NAME>",
    );
    parameter(&mut sources, "Receive", "CanObjectId", "1");
    parameter(&mut sources, "Transmit", "CanObjectId", "0");
    let renamed = build(&sources).unwrap_or_else(|issues| panic!("{issues:?}"));
    let runtime = renamed
        .description()
        .communication_runtime
        .as_ref()
        .unwrap();
    let json = serde_json::to_value(runtime).unwrap();
    assert_eq!(json["pduIdType"], "UINT8");
    assert_eq!(json["pduLengthType"], "UINT32");
    assert!(runtime.can.read_write_period.ends_with("/PollCycle"));
    assert_eq!(runtime.can.controller_id, 0);
    assert_eq!(runtime.can.can_if_controller_id, 0);
    assert_eq!(runtime.can.receive_handle, 1);
    assert_eq!(runtime.can.transmit_handle, 0);
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn multi_communication_rejects_truncation_and_unbound_polling_in_both_entries() {
    for (owner, field, new_value, code) in [
        (
            "Polling",
            "CanMainFunctionPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionBusoffPeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "General",
            "CanMainFunctionModePeriod",
            "0.002",
            "CAN_POLLING_TIMEBASE",
        ),
        (
            "Controller",
            "CanBusoffProcessing",
            "INTERRUPT",
            "CAN_PROCESSING",
        ),
        (
            "General",
            "CanDevErrorDetect",
            "true",
            "CAN_FEATURE_UNSUPPORTED",
        ),
        ("Transmit", "CanObjectId", "0", "CAN_HARDWARE_HANDLES"),
        ("Transmit", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Receive", "CanObjectId", "17", "CAN_HARDWARE_HANDLES"),
        ("Controller", "CanControllerId", "7", "CAN_CONTROLLER_ID"),
        ("Controller", "CanIfCtrlId", "9", "CAN_CONTROLLER_ID"),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduIdTypeEnum", "UINT8");
    parameter(&mut sources, "RxValue", "CanIfRxPduId", "256");
    rejects_in_both(&sources, "COMMUNICATION_HANDLE_RANGE");
    let mut sources = inputs();
    parameter(&mut sources, "Pdus", "PduLengthTypeEnum", "UINT8");
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| named(node, "ECUC-CONTAINER-VALUE", "RxValuePdu"),
        |text| {
            format!(
                "{}\n{}",
                text,
                text.replace(
                    "<SHORT-NAME>RxValuePdu</SHORT-NAME>",
                    "<SHORT-NAME>AdditionalPdu</SHORT-NAME>"
                )
                .replace("<VALUE>4</VALUE>", "<VALUE>256</VALUE>")
            )
        },
    );
    rejects_in_both(&sources, "COMMUNICATION_LENGTH_RANGE");
    let mut sources = inputs();
    change(
        &mut sources,
        "ecuc.arxml",
        "/Configuration/Can/General/Polling</VALUE-REF>",
        "/Configuration/Can/General</VALUE-REF>",
    );
    rejects_in_both(&sources, "CAN_POLLING_REFERENCE");
}

#[test]
fn mode_configuration_preserves_channel_users_rules_and_static_callouts() {
    let plan = build(&inputs()).unwrap();
    let mode = plan.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Host");
    assert_eq!(mode.channel_handle, 0);
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Host");
    assert_eq!((mode.period_ms, mode.minimum_full_ms), (1, 5));
    assert_eq!(mode.users.len(), 1);
    assert_eq!(mode.ecu_group_classification, 3);
    assert_eq!(mode.users[0].handle, 0);
    assert_eq!(mode.include, "Ecu_HostBusSM.h");
    assert_eq!(mode.initial_mode, 0);
    assert_eq!(
        mode.rules.iter().map(|rule| rule.mode).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(mode.dcm_connections.len(), 1);
    assert!(mode.rules.iter().all(|rule| {
        rule.callout
            .starts_with("Ecu_HostBusSM_ApplyMode(0u, COMM_")
    }));
    let mut sources = inputs();
    replace_all(&mut sources, "/Config/Host/", "/Config/Vehicle/");
    change(
        &mut sources,
        "ecuc.arxml",
        "<SHORT-NAME>Host</SHORT-NAME>",
        "<SHORT-NAME>Vehicle</SHORT-NAME>",
    );
    replace_all(&mut sources, "/Config/Host<", "/Config/Vehicle<");
    parameter(&mut sources, "Vehicle", "ComMChannelId", "7");
    parameter(&mut sources, "HostUser", "ComMUserIdentifier", "19");
    parameter(&mut sources, "General", "ComMEcuGroupClassification", "1");
    replace_all(
        &mut sources,
        "Ecu_HostBusSM_ApplyMode(0u,",
        "Ecu_HostBusSM_ApplyMode(7u,",
    );
    let renamed = build(&sources).unwrap();
    let mode = renamed.description().mode_runtime.as_ref().unwrap();
    assert_eq!(mode.channel, "/Configuration/ComM/Config/Vehicle");
    assert_eq!(mode.main_symbol, "ComM_MainFunction_Vehicle");
    assert_eq!(mode.channel_handle, 7);
    assert_eq!(mode.users[0].handle, 19);
    assert_eq!(mode.ecu_group_classification, 1);
    assert!(normal_validation(&sources).diagnostics.is_empty());
}

#[test]
fn selected_mode_configuration_rejects_unbound_features_and_dynamic_callouts() {
    for (owner, field, new_value, code) in [
        (
            "Host",
            "ComMBusType",
            "COMM_BUS_TYPE_CAN",
            "MODE_BUS_PROVIDER",
        ),
        (
            "Host",
            "ComMCDDBusPrefix",
            "OtherBusSM",
            "MODE_BUS_PROVIDER",
        ),
        (
            "NetworkManagement",
            "ComMNmVariant",
            "FULL",
            "MODE_NM_VARIANT",
        ),
        ("Host", "ComMMainFunctionPeriod", "0.002", "MODE_TIMEBASE"),
        (
            "CurrentMode",
            "BswMRequestProcessing",
            "BSWM_DEFERRED",
            "MODE_PROCESSING",
        ),
        (
            "Includes",
            "BswMUserIncludeFile",
            "../Ecu_HostBusSM.h",
            "MODE_INCLUDE",
        ),
        (
            "NoRule",
            "BswMNestedExecutionOnly",
            "true",
            "MODE_FEATURE_UNSUPPORTED",
        ),
        (
            "NoList",
            "BswMActionListExecution",
            "BSWM_TRIGGER",
            "MODE_RULE",
        ),
        (
            "FullAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, currentMode)",
            "MODE_STATIC_CALLOUT",
        ),
        (
            "SilentAction",
            "BswMUserCalloutFunction",
            "Ecu_HostBusSM_ApplyMode(0u, COMM_FULL_COMMUNICATION)",
            "MODE_STATIC_CALLOUT",
        ),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, owner, field, new_value);
        rejects_in_both(&sources, code);
    }
}

#[test]
fn required_dcm_channel_reference_has_only_verified_historical_compatibility() {
    use autosar_config_core::project_model::ValidationStatus;
    let legacy = legacy_inputs();
    let validation = normal_validation(&legacy);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(validation.status, ValidationStatus::Unsupported);
    assert!(validation.coverage.iter().any(|rule| rule.rule_id
        == "native.definition.legacy-dcm-mode-dependency"
        && !rule.supported));
    let plan = build(&legacy).unwrap();
    assert_eq!(
        plan.description().profile,
        autosar_config_core::integration::PROFILE
    );
    assert!(plan.description().mode_runtime.is_none());
    assert!(
        plan.ecu_integration_files(autosar_config_core::target::BuildTarget::LinuxX64ControlledV1)
            .is_ok()
    );
    let mut multi = inputs();
    xml_edit(
        &mut multi,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "ECUC-REFERENCE-VALUE"
                && node.children().any(|child| {
                    child.tag_name().name() == "DEFINITION-REF"
                        && child
                            .text()
                            .is_some_and(|text| text.ends_with("/DcmDslProtocolComMChannelRef"))
                })
        },
        |_| String::new(),
    );
    assert!(
        normal_validation(&multi)
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY")
    );
    assert!(build(&multi).is_err());
    // An explicit new mode module is a new configuration, even when the
    // remaining files retain the historical single-component shape.
    for module in ["ComM", "BswM"] {
        let mut sources = legacy.clone();
        change(
            &mut sources,
            "ecuc.arxml",
            "</ELEMENTS>",
            &format!(
                "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>{module}</SHORT-NAME><DEFINITION-REF DEST=\"ECUC-MODULE-DEF\">/AUTOSAR/EcucDefs/{module}</DEFINITION-REF></ECUC-MODULE-CONFIGURATION-VALUES></ELEMENTS>"
            ),
        );
        assert!(
            normal_validation(&sources)
                .diagnostics
                .iter()
                .any(|issue| issue.code == "MULTIPLICITY"
                    && issue.witness.as_ref().is_some_and(|witness| format!(
                        "{:?}",
                        witness.constraint
                    )
                    .contains("DcmDslProtocolComMChannelRef")))
        );
    }
    let source = legacy
        .iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    let text = std::str::from_utf8(source.bytes()).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let dcm = document
        .descendants()
        .find(|node| named(*node, "ECUC-MODULE-CONFIGURATION-VALUES", "Dcm"))
        .unwrap();
    let standalone = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Configuration</SHORT-NAME><ELEMENTS>{}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>",
        &text[dcm.range()]
    );
    let standalone = [InputSource::new("dcm.arxml", standalone.into_bytes()).unwrap()];
    let validation = normal_validation(&standalone);
    assert!(
        validation
            .diagnostics
            .iter()
            .any(|issue| issue.code == "MULTIPLICITY"
                && issue.witness.as_ref().is_some_and(|witness| format!(
                    "{:?}",
                    witness.constraint
                )
                .contains("DcmDslProtocolComMChannelRef"))),
        "{:?}",
        validation.diagnostics
    );
}

#[test]
fn historical_profile_identity_uses_bindings_instead_of_runnable_capabilities() {
    let mut concurrent = legacy_inputs();
    change(
        &mut concurrent,
        "application.arxml",
        "<CAN-BE-INVOKED-CONCURRENTLY>false",
        "<CAN-BE-INVOKED-CONCURRENTLY>true",
    );
    let validation = normal_validation(&concurrent);
    assert!(validation.diagnostics.is_empty());
    assert_eq!(
        validation.status,
        autosar_config_core::project_model::ValidationStatus::Unsupported
    );
    let issues = build(&concurrent).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == "REENTRANCY_UNSUPPORTED"),
        "{issues:?}"
    );
    assert!(!issues.iter().any(|issue| issue.code == "MULTIPLICITY"));
    let mut wrong_rte = legacy_inputs();
    xml_edit(
        &mut wrong_rte,
        "ecuc.arxml",
        |node| {
            node.tag_name().name() == "VALUE-REF"
                && node.parent().is_some_and(|parent| {
                    parent.children().any(|child| {
                        child.tag_name().name() == "DEFINITION-REF"
                            && child.text().is_some_and(|text| {
                                text.ends_with("/RteSoftwareComponentInstanceRef")
                            })
                    })
                })
        },
        |_| {
            "<VALUE-REF DEST=\"SW-COMPONENT-PROTOTYPE\">/Application/ReferenceComposition/DcmService</VALUE-REF>".into()
        },
    );
    let mut unmapped = legacy_inputs();
    xml_edit(
        &mut unmapped,
        "extract.arxml",
        |node| {
            node.tag_name().name() == "COMPONENT-IREF"
                && node.children().any(|child| {
                    child.tag_name().name() == "TARGET-COMPONENT-REF"
                        && child.text() == Some("/Application/ReferenceComposition/EchoApplication")
                })
        },
        |_| String::new(),
    );
    for sources in [wrong_rte, unmapped] {
        let validation = normal_validation(&sources);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|issue| issue.code == "MULTIPLICITY"
                    && issue.witness.as_ref().is_some_and(|witness| format!(
                        "{:?}",
                        witness.constraint
                    )
                    .contains("DcmDslProtocolComMChannelRef"))),
            "{:?}",
            validation.diagnostics
        );
        assert!(
            !validation
                .coverage
                .iter()
                .any(|rule| rule.rule_id == "native.definition.legacy-dcm-mode-dependency")
        );
    }
}

#[test]
fn multi_internal_active_session_did_cannot_be_replaced_by_application_source() {
    let mut sources = inputs();
    parameter(
        &mut sources,
        "ApplicationDid",
        "DcmDspDidIdentifier",
        "61830",
    );
    rejects_in_both(&sources, "DIAGNOSTIC_IDENTIFIER");
    let issues = build(&sources).err().expect("reserved DID refused");
    let message = issues
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let validation = normal_validation(&sources);
    let message = validation
        .diagnostics
        .iter()
        .find(|issue| issue.code == "DIAGNOSTIC_IDENTIFIER")
        .unwrap()
        .message
        .to_string();
    assert!(
        message.contains("0xF186") && message.contains("reserved") && message.contains("internal"),
        "{message}"
    );
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../src/messages.json")).unwrap();
    assert!(
        catalog["zh-CN"]["backend.integration.diagnostic.did_identifier_reserved"]
            .as_str()
            .unwrap()
            .contains("保留")
    );
}

#[test]
fn selected_dcm_subfunction_availability_matches_actual_service_dispatch() {
    for (service, available) in [
        ("SessionControl", "false"),
        ("TesterPresent", "false"),
        ("ReadDataByIdentifier", "true"),
    ] {
        let mut sources = inputs();
        parameter(&mut sources, service, "DcmDsdSidTabSubfuncAvail", available);
        rejects_in_both(&sources, "SERVICE_UNSUPPORTED");
    }
}

#[test]
fn selected_com_manual_trigger_requires_zero_minimum_delay_and_no_callout() {
    let mut sources = inputs();
    parameter(&mut sources, "Transmit", "ComMinimumDelayTime", "0.001");
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
    let mut sources = inputs();
    xml_edit(
        &mut sources,
        "ecuc.arxml",
        |node| {
            named(node, "ECUC-CONTAINER-VALUE", "TxValuePdu")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with("/ComIPdu"))
                })
        },
        |text| {
            text.replacen("<PARAMETER-VALUES>", r#"<PARAMETER-VALUES><ECUC-TEXTUAL-PARAM-VALUE><DEFINITION-REF DEST="ECUC-FUNCTION-NAME-DEF">/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComIPduCallout</DEFINITION-REF><VALUE>ApplicationCallout</VALUE></ECUC-TEXTUAL-PARAM-VALUE>"#, 1)
        },
    );
    rejects_in_both(&sources, "COM_FEATURE_UNSUPPORTED");
}
