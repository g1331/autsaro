//! Generated contract headers compiled and linked by independent consumers.
use super::*;
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_headers_compile_and_link_independent_scalar_and_void_server_signatures() {
    for renamed in [false, true] {
        let mut inputs = pipeline_operation_inputs();
        if renamed {
            replace_all(&mut inputs, "/Types/uint32", "/Types/ValueType");
            xml_edit(
                &mut inputs,
                "types.arxml",
                |node| named(node, "IMPLEMENTATION-DATA-TYPE", "uint32"),
                |text| {
                    text.replace(
                        "<SHORT-NAME>uint32</SHORT-NAME>",
                        "<SHORT-NAME>ValueType</SHORT-NAME>",
                    )
                },
            );
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
        let directory = scratch.0.join("typed");
        std::fs::create_dir_all(directory.join("include")).unwrap();
        for (file, bytes) in plan.component_contract_files().unwrap().files() {
            let path = directory.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
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
Std_ReturnType Rte_Read_Process_Value_Value(uint32 *data) { *data=7U; return E_OK; }
Std_ReturnType Rte_Write_Process_Result_Value(uint32 data) { written=data; return E_OK; }
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
Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; return E_OK; }
Std_ReturnType (*const byte_server)(uint8 *) = Ingress_ReadData;
Std_ReturnType Rte_Read_Ingress_RxValue_Value(uint32 *data) { *data=11U; return E_OK; }
Std_ReturnType Rte_Write_Ingress_TxValue_Value(uint32 data) { tx_value=data; return E_OK; }
Std_ReturnType Rte_Write_Ingress_Value_Value(uint32 data) { local_value=data; return E_OK; }
int test_ingress(void);
int test_ingress(void) {
    uint32 value=0U;
    uint8 bytes[4]={0U,0U,0U,0U};
    if (byte_server(bytes) != E_OK) { return 1; }
    return Rte_Read_RxValue_Value(&value)==E_OK && value==11U && Rte_Write_TxValue_Value(21U)==E_OK && Rte_Write_Value_Value(31U)==E_OK && tx_value==21U && local_value==31U && bytes[0]==1U && bytes[3]==4U ? 0 : 1;
}
"#,
            ),
            (
                "observe.c",
                r#"#include "Rte_Observe.h"
void Observe_Periodic(void) {}
Std_ReturnType Rte_Call_Observe_ResultService_Ping(void) { return E_OK; }
Std_ReturnType (*const empty_client)(void) = Rte_Call_ResultService_Ping;
Std_ReturnType Rte_Read_Observe_Value_Value(uint32 *data) { *data=9U; return E_OK; }
Std_ReturnType Rte_Read_Observe_Result_Value(uint32 *data) { *data=19U; return E_OK; }
Std_ReturnType Rte_Call_Observe_ResultService_Transform(uint32 Input, uint32 *Output, uint32 *State) { *Output=Input; *State+=Input; return E_OK; }
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
Std_ReturnType Rte_Call_DcmService_DataServices_ApplicationValue_ReadData(Dcm_DataElement_ApplicationValueType Data) { Data[0]=1U; Data[1]=2U; Data[2]=3U; Data[3]=4U; return E_OK; }
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
                            "Rte_Read_Compute_Value_Value",
                            "Rte_Read_Compute_InputValue_Value",
                        );
                }
            }
            std::fs::write(directory.join(file), bytes).unwrap();
        }
        let executable = tooling::native_binary(&directory, "typed");
        let mut compiler = tooling::c_compiler();
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
        // Compile the actual generated producer too: the consumer definitions
        // above validate independent signatures, not this implementation's bodies.
        std::fs::create_dir_all(directory.join("src")).unwrap();
        let produced = prepared_pipeline_files(&plan);
        let metadata = std::str::from_utf8(
            &produced
                .iter()
                .find(|(file, _)| file == "generated/descriptions/Host_Implementation.arxml")
                .unwrap()
                .1,
        )
        .unwrap();
        assert!(metadata.contains(if renamed {
            "<SHORT-NAME>Compute_CODE</SHORT-NAME>"
        } else {
            "<SHORT-NAME>Process_CODE</SHORT-NAME>"
        }));
        let partition = plan
            .description()
            .communication_runtime
            .as_ref()
            .unwrap()
            .partition
            .path
            .rsplit('/')
            .next()
            .unwrap();
        assert!(metadata.contains(&format!(
            "<SHORT-NAME>{partition}_CALLOUT_CODE</SHORT-NAME>"
        )));
        for (file, bytes) in produced {
            // Static BSW and host headers come from the repository runtime below.
            if let Some(file) = file
                .strip_prefix("generated/")
                .filter(|file| file.starts_with("include/") || *file == "src/Rte.c")
            {
                let target = directory.join(file);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::write(target, bytes).unwrap();
            }
        }
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let settings = tooling::execution_settings();
        let mut compiler = std::process::Command::new(&settings.compiler);
        compiler.current_dir(&directory).args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-Iinclude",
        ]);
        for include in [
            "runtime/ecu/include",
            "runtime/os/include",
            "runtime/bsw/include",
            "runtime/host/include",
        ] {
            compiler.arg("-I").arg(repository.join(include));
        }
        compiler.args(["-c", "src/Rte.c", "-o", "Rte.o"]);
        let output = tooling::run_public_command(
            &mut compiler,
            &directory,
            "rte-producer-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = tooling::run_public_command(
            std::process::Command::new(&settings.objdump)
                .current_dir(&directory)
                .args(["-h", "Rte.o"]),
            &directory,
            "rte-producer-sections",
            std::time::Duration::from_secs(30),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let sections = String::from_utf8_lossy(&output.stdout);
        assert!(sections.contains(".rte_code"), "{sections}");
        assert!(
            sections.contains(".data.rte.Observe.VAR_INIT_UNSPECIFIED")
                && sections.contains(".bss.rte.Ingress.VAR_CLEARED_UNSPECIFIED"),
            "{sections}"
        );
        let nm = std::env::var_os("AUTOSAR_NM").expect("Set AUTOSAR_NM to the controlled nm");
        let output = tooling::run_public_command(
            std::process::Command::new(nm)
                .current_dir(&directory)
                .arg(if cfg!(windows) {
                    "--target=pe-x86-64"
                } else {
                    "--target=elf64-x86-64"
                })
                .args(["-g", "--defined-only", "Rte.o"]),
            &directory,
            "rte-public-symbols",
            std::time::Duration::from_secs(30),
        );
        assert!(output.status.success(), "{output:?}");
        let symbols: Vec<_> = std::str::from_utf8(&output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .collect();
        for component in [
            "Ingress",
            if renamed { "Compute" } else { "Process" },
            "Observe",
            "DcmService",
        ] {
            assert!(symbols.contains(&format!("Rte_Inst_{component}").as_str()));
            assert!(!symbols.contains(&format!("Rte_Instance_{component}").as_str()));
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_rte_com_header_links_independent_handle_callbacks_and_real_dispatch() {
    // Two Rx receivers at once (handles 43 and 61, neither the template's 17).
    let plan = build(&two_receiver_inputs()).unwrap();
    let scratch = Scratch::new();
    for (path, bytes) in prepared_pipeline_files(&plan) {
        let path = scratch.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(&scratch.0, "rte-com-consumer");
    let mut compiler = tooling::c_compiler();
    compiler
        .current_dir(&scratch.0)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .args([
            "-Igenerated/include",
            "-Ibsw/include",
            "-Ibsw/src",
            "-Iintegration/include",
            "-Iintegration/src",
            "-Ios/include",
        ])
        .args([
            "generated/src/Rte.c",
            "bsw/src/Com.c",
            // COM enters its own state area; link the delivered host provider.
            "integration/src/SchM_CanIf_Host.c",
            "integration/src/Can_HostLock.c",
            "app/Ingress/Ingress.c",
            "app/Process/Process.c",
            "app/Observe/Observe.c",
        ])
        .arg(root.join("core/tests/fixtures/bsw-contracts/rte_com_contract.c"))
        .arg("-Wl,--wrap=Com_ReceiveSignal");
    if cfg!(target_os = "linux") {
        compiler.arg("-pthread");
    }
    compiler.arg("-o").arg(&executable);
    let output = tooling::run_public_command(
        &mut compiler,
        &scratch.0,
        "rte-com-compile",
        std::time::Duration::from_secs(60),
    );
    assert!(output.status.success(), "{output:?}");
    let output = tooling::run_public_command(
        &mut std::process::Command::new(executable),
        &scratch.0,
        "rte-com-run",
        std::time::Duration::from_secs(30),
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

/// The generated SchM lifecycle gates order, partition, single use and the
/// shutdown-only Deinit (`schm_contract.c`).
#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_schm_lifecycle_refuses_misordered_repeated_and_foreign_calls() {
    let plan = build(&inputs()).unwrap();
    let scratch = Scratch::new();
    for (path, bytes) in prepared_pipeline_files(&plan) {
        let path = scratch.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let executable = tooling::native_binary(&scratch.0, "schm-lifecycle");
    let mut compiler = tooling::c_compiler();
    compiler
        .current_dir(&scratch.0)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .args([
            "-Igenerated/include",
            "-Ibsw/include",
            "-Iintegration/include",
            "-Iintegration/src",
            "-Ios/include",
        ])
        // The Dcm mode switches in SchM_Main.c enter the delivered host area.
        .args([
            "generated/src/SchM_Main.c",
            "integration/src/Can_HostLock.c",
        ])
        .arg(root.join("core/tests/fixtures/bsw-contracts/schm_contract.c"));
    if cfg!(target_os = "linux") {
        compiler.arg("-pthread");
    }
    compiler.arg("-o").arg(&executable);
    let output = tooling::run_public_command(
        &mut compiler,
        &scratch.0,
        "schm-lifecycle-compile",
        std::time::Duration::from_secs(60),
    );
    assert!(output.status.success(), "{output:?}");
    let output = tooling::run_public_command(
        &mut std::process::Command::new(executable),
        &scratch.0,
        "schm-lifecycle-run",
        std::time::Duration::from_secs(30),
    );
    assert!(output.status.success(), "{output:?}");
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn initialized_scaffolds_compile_link_and_initialize_only_out_arguments() {
    use autosar_config_core::prepared::prepare_ecu_project_for_workspace;
    for arguments in [
        [("Input", "value"), ("Output", "data"), ("State", "status")],
        [
            ("Input", "Ecu_sample"),
            ("Output", "Os_sample"),
            ("State", "xTask_sample"),
        ],
    ] {
        let scratch = Scratch::new();
        let live = scratch.0.join("live");
        let mut sources = inputs();
        for (before, after) in arguments {
            change(
                &mut sources,
                "types.arxml",
                &format!("<SHORT-NAME>{before}</SHORT-NAME>"),
                &format!("<SHORT-NAME>{after}</SHORT-NAME>"),
            );
        }
        let mut workspace = live_pipeline_workspace(&live, &sources);
        let preview = workspace.preview_application_initialization().unwrap();
        workspace
            .initialize_application_previewed(&preview)
            .unwrap();
        let plan = workspace.saved_integration_plan().unwrap();
        let project = scratch.0.join("sealed");
        prepare_ecu_project_for_workspace(&workspace, &plan, tooling::native_target(), true)
            .unwrap()
            .generate(&project)
            .unwrap();
        let build = scratch.0.join("build");
        // Only the independent C consumer runs here; keep its TSan scaffold prerequisite plain.
        let mut build_command = if std::env::var("AUTOSAR_SANITIZE").as_deref() == Ok("thread") {
            tooling::ecu_production_build_command(&project, &build, "host-batch", None)
        } else {
            tooling::ecu_build_command(&project, &build, "host-batch", None)
        };
        let compiled = build_command.output().unwrap();
        assert!(
            compiled.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&compiled.stdout),
            String::from_utf8_lossy(&compiled.stderr)
        );
        let harness = scratch.0.join("scaffold_contract.c");
        std::fs::write(
            &harness,
            r#"#define RTE_CORE
#include "Rte_Process.h"
#include "Rte_Ingress.h"
#include <assert.h>
int main(void) {
    uint32 output = 99u;
    uint32 state = 17u;
    Dcm_DataElement_ApplicationValueType data = {1u, 2u, 3u, 4u};
    void (*scalar_server)(uint32, uint32 *, uint32 *) = &Process_Transform;
    Std_ReturnType (*array_server)(Dcm_DataElement_ApplicationValueType) = &Ingress_ReadData;
    scalar_server(23u, &output, &state);
    assert(array_server(data) == E_OK);
    assert(output == 0u);
    assert(state == 17u);
    assert((data[0] == 0u) && (data[1] == 0u) && (data[2] == 0u) && (data[3] == 0u));
    return 0;
}
"#,
        )
        .unwrap();
        let binary = tooling::native_binary(&scratch.0, "scaffold_contract");
        let compiled = tooling::c_compiler()
            .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(
                ["generated/include", "bsw/include"]
                    .map(|include| format!("-I{}", project.join(include).display())),
            )
            .arg(project.join("app/Process/Process.c"))
            .arg(project.join("app/Ingress/Ingress.c"))
            .arg(project.join("app/Observe/Observe.c"))
            .arg(&harness)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        assert!(
            std::process::Command::new(binary)
                .status()
                .unwrap()
                .success()
        );
    }
}

#[test]
fn generated_rte_unconnected_and_first_publication_preserve_values_and_outputs() {
    for (unconnected, never_received) in [(false, true), (true, true), (false, false)] {
        let mut sources = inputs();
        for file in ["process.arxml", "observe.arxml"]
            .into_iter()
            .filter(|_| never_received)
        {
            change(
                &mut sources,
                file,
                "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
                "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
            );
        }
        if !never_received {
            change(
                &mut sources,
                "ingress.arxml",
                "<HANDLE-NEVER-RECEIVED>true</HANDLE-NEVER-RECEIVED>",
                "<HANDLE-NEVER-RECEIVED>false</HANDLE-NEVER-RECEIVED>",
            );
        }
        if unconnected {
            for connector in ["IngressProcess", "IngressObserve", "Scalar"] {
                xml_edit(
                    &mut sources,
                    "composition.arxml",
                    |node| named(node, "ASSEMBLY-SW-CONNECTOR", connector),
                    |_| String::new(),
                );
            }
        }
        let plan = build(&sources).unwrap();
        let scratch = Scratch::new();
        for (path, bytes) in prepared_pipeline_files(&plan) {
            let path = scratch.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let executable = tooling::native_binary(&scratch.0, "rte-consumer");
        let cpp = scratch.0.join("callback.cpp");
        std::fs::write(&cpp, "#include \"Rte_Com.h\"\nvoid callback_linkage(void);\nvoid callback_linkage(void) { void (*volatile received)(CbkHandleIdType) = Rte_COMCbk; void (*volatile expired)(CbkHandleIdType) = Rte_COMCbkRxTOut; (void)received; (void)expired; }\n").unwrap();
        let object = scratch.0.join("callback.o");
        let output = tooling::run_public_command(
            tooling::c_compiler()
                .current_dir(&scratch.0)
                .args([
                    "-x",
                    "c++",
                    "-std=c++11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pedantic",
                    "-Igenerated/include",
                    "-Ibsw/include",
                    "-c",
                ])
                .arg(&cpp)
                .arg("-o")
                .arg(&object),
            &scratch.0,
            "cpp-callback-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(output.status.success(), "{output:?}");
        let mut compiler = tooling::c_compiler();
        compiler
            .current_dir(&scratch.0)
            .args([
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-Igenerated/include",
                "-Ibsw/include",
                "-Iintegration/include",
                "-Ios/include",
            ])
            .arg("generated/src/Rte.c")
            .arg(root.join("core/tests/fixtures/bsw-contracts/rte_contract.c"));
        if unconnected {
            compiler.arg("-DUNCONNECTED_PORTS");
        }
        if !never_received {
            compiler.arg("-DHANDLE_NEVER_DISABLED");
        }
        compiler.arg(&object).arg("-o").arg(&executable);
        let output = tooling::run_public_command(
            &mut compiler,
            &scratch.0,
            "rte-consumer-compile",
            std::time::Duration::from_secs(60),
        );
        assert!(output.status.success(), "{output:?}");
        let output = tooling::run_public_command(
            &mut std::process::Command::new(executable),
            &scratch.0,
            "rte-consumer-run",
            std::time::Duration::from_secs(30),
        );
        assert!(output.status.success(), "{output:?}");
    }
}

#[test]
fn application_headers_enforce_runnable_visibility_memmap_and_imported_releases() {
    // Verifies: SWS_Rte_01005, SWS_Rte_01006, SWS_Rte_02751, SWS_Rte_05078
    // Verifies: SWS_Rte_06713, SWS_Rte_07120, SWS_Rte_07124, SWS_Rte_07194
    // Verifies: SWS_Rte_07692, SWS_MemMap_00027, SWS_MemMap_00029
    let plan = build(&inputs()).unwrap();
    let scratch = Scratch::new();
    for (path, bytes) in prepared_pipeline_files(&plan) {
        let path = scratch.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let check = |label: &str, source: &str, cpp: bool, accepted: bool, diagnostic: &str| {
        let path = scratch.0.join(format!("{label}.c"));
        std::fs::write(&path, source).unwrap();
        let mut compiler = tooling::c_compiler();
        compiler
            .current_dir(&scratch.0)
            .args([
                "-x",
                if cpp { "c++" } else { "c" },
                if cpp { "-std=c++11" } else { "-std=c99" },
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-fsyntax-only",
                "-Igenerated/include",
                "-Ibsw/include",
                "-Iintegration/include",
                "-Ios/include",
            ])
            .arg(&path);
        let output = tooling::run_public_command(
            &mut compiler,
            &scratch.0,
            label,
            std::time::Duration::from_secs(60),
        );
        assert_eq!(
            output.status.success(),
            accepted,
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if !accepted {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(diagnostic),
                "{label}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    };
    // Verifies: SWS_MemMap_00003, SWS_MemMap_00007, SWS_MemMap_00011
    for (header, prefix, code_keyword, variable_keyword) in [
        ("Can_MemMap.h", "CAN", "CAN_CODE", "CAN_VAR_CLEARED"),
        ("BswM_MemMap.h", "BSWM", "BSWM_CODE", "BSWM_VAR_CLEARED"),
        ("CanIf_MemMap.h", "CANIF", "CANIF_CODE", "CANIF_VAR_CLEARED"),
        ("CanTp_MemMap.h", "CANTP", "CANTP_CODE", "CANTP_VAR_CLEARED"),
        ("Com_MemMap.h", "COM", "COM_CODE", "COM_VAR_CLEARED"),
        ("ComM_MemMap.h", "COMM", "COMM_CODE", "COMM_VAR_CLEARED"),
        ("Dcm_MemMap.h", "DCM", "DCM_CODE", "DCM_VAR_CLEARED"),
        ("Det_MemMap.h", "DET", "DET_CODE", "DET_VAR_CLEARED"),
        ("LSduR_MemMap.h", "LSDUR", "LSDUR_CODE", "LSDUR_VAR_CLEARED"),
        ("PduR_MemMap.h", "PDUR", "PDUR_CODE", "PDUR_VAR_CLEARED"),
        (
            "Ecu_HostBusSM_MemMap.h",
            "ECU_HOSTBUSSM",
            "ECU_HOSTBUSSM_CODE",
            "ECU_HOSTBUSSM_VAR_CLEARED",
        ),
        ("Os_MemMap.h", "OS", "OS_MEMMAP", "OS_MEMMAP"),
        (
            "Rte_MemMap.h",
            "RTE_Rte",
            "RTE_Rte_CODE",
            "RTE_Rte_VAR_CLEARED_UNSPECIFIED",
        ),
        (
            "Ingress_MemMap.h",
            "Ingress",
            "Ingress_CODE",
            "Ingress_VAR_CLEARED_UNSPECIFIED",
        ),
    ] {
        for cpp in [false, true] {
            check(
                &format!("memmap-{prefix}-paired-{cpp}"),
                &format!(
                    "#define {prefix}_START_SEC_CODE\n#include \"{header}\"\n\
                     {code_keyword} void mapping_code(void) {{}}\n\
                     #define {prefix}_STOP_SEC_CODE\n#include \"{header}\"\n\
                     #if defined({prefix}_START_SEC_CODE) || defined({prefix}_STOP_SEC_CODE) || defined({code_keyword})\n\
                     #error Code section was not consumed\n#endif\n\
                     #define {prefix}_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"{header}\"\n\
                     {variable_keyword} int mapping_state;\n\
                     #define {prefix}_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"{header}\"\n\
                     #if defined({prefix}_START_SEC_VAR_CLEARED_UNSPECIFIED) || defined({prefix}_STOP_SEC_VAR_CLEARED_UNSPECIFIED) || defined({variable_keyword})\n\
                     #error Variable section was not consumed\n#endif\n"
                ),
                cpp,
                true,
                "",
            );
            let diagnostic = format!(
                "{} requires exactly one supported marker",
                header.trim_end_matches(".h")
            );
            check(
                &format!("memmap-{prefix}-missing-{cpp}"),
                &format!("#include \"{header}\"\nint mapping_control;\n"),
                cpp,
                false,
                if prefix == "OS" {
                    "Unsupported or missing OS memory mapping marker"
                } else {
                    &diagnostic
                },
            );
            check(
                &format!("memmap-{prefix}-multiple-{cpp}"),
                &format!(
                    "#define {prefix}_START_SEC_CODE\n#define {prefix}_STOP_SEC_CODE\n\
                     #include \"{header}\"\nint mapping_control;\n"
                ),
                cpp,
                false,
                if prefix == "OS" {
                    "Conflicting OS memory mapping markers"
                } else {
                    &diagnostic
                },
            );
        }
    }
    for cpp in [false, true] {
        check(
            if cpp { "cpp-contract" } else { "c-contract" },
            "#define RTE_RUNNABLEAPI_Periodic\n#include \"Rte_Ingress.h\"\n#ifndef Rte_Read_RxValue_Value\n#error Missing runnable read\n#endif\n#if Rte_InitValue_RxValue_Value != 0UL\n#error Wrong initial value\n#endif\nvoid (*entry)(void) = RTE_RUNNABLE_Periodic;\nRte_Instance instance(void);\nRte_Instance instance(void) { return Rte_Inst_Ingress; }\n",
            cpp,
            true,
            "",
        );
    }
    for cpp in [false, true] {
        check(
            if cpp {
                "cpp-data-handle-types"
            } else {
                "c-data-handle-types"
            },
            "#include \"Rte_DataHandleType.h\"\nDcm_DataElement_ApplicationValueType data;\n",
            cpp,
            true,
            "",
        );
        check(
            if cpp {
                "cpp-mode-types"
            } else {
                "c-mode-types"
            },
            "#include \"SchM_Dcm_Type.h\"\nRte_ModeType_DcmDiagnosticSessionControl session = RTE_MODE_DcmDiagnosticSessionControl_DCM_DEFAULT_SESSION;\nRte_ModeType_DcmAuthenticationState_Physical authentication = RTE_MODE_DcmAuthenticationState_Physical_DCM_DEAUTHENTICATED;\n#if RTE_TRANSITION_DcmDiagnosticSessionControl != 255u\n#error Wrong transition\n#endif\n",
            cpp,
            true,
            "",
        );
    }
    check(
        "cpp-module-interlinks",
        "#include \"SchM_Can.h\"\n#include \"SchM_CanIf.h\"\n#include \"SchM_CanTp.h\"\n#include \"SchM_Com.h\"\n#include \"SchM_ComM.h\"\n#include \"SchM_BswM.h\"\n#include \"SchM_Dcm.h\"\n#include \"SchM_Det.h\"\n#include \"SchM_PduR.h\"\n#include \"SchM_LSduR.h\"\n#if defined(COM_START_SEC_CODE) || defined(COM_STOP_SEC_CODE) || defined(Com_START_SEC_CODE) || defined(Com_STOP_SEC_CODE) || defined(DCM_START_SEC_CODE) || defined(DCM_STOP_SEC_CODE) || defined(Dcm_START_SEC_CODE) || defined(Dcm_STOP_SEC_CODE)\n#error Unconsumed module section marker\n#endif\n",
        true,
        true,
        "",
    );
    check(
        "multiple-app-headers",
        "#include \"Rte_Ingress.h\"\n#include \"Rte_Process.h\"\n",
        false,
        false,
        "Multiple application header",
    );
    check(
        "private-cds-object",
        "#include \"Rte_Ingress.h\"\nconst Rte_CDS_Ingress *storage = &Rte_Instance_Ingress;\n",
        false,
        false,
        "Rte_Instance_Ingress",
    );
    check(
        "unselected-runnable-api",
        "#define RTE_RUNNABLEAPI_ServerReadData\n#include \"Rte_Ingress.h\"\nStd_ReturnType (*hidden)(uint32 *) = Rte_Read_Ingress_RxValue_Value;\n",
        false,
        false,
        "Rte_Read_Ingress_RxValue_Value",
    );
    for (label, source, error) in [
        (
            "unknown-swc-section",
            "#define Ingress_START_SEC_UNKNOWN\n#include \"Ingress_MemMap.h\"\n",
            "Unknown SWC memory section",
        ),
        (
            "unmatched-swc-section",
            "#define Ingress_STOP_SEC_CODE\n#include \"Ingress_MemMap.h\"\n",
            "Unmatched SWC memory section",
        ),
        (
            "nested-swc-section",
            "#define Ingress_START_SEC_CODE\n#include \"Ingress_MemMap.h\"\n#define Ingress_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include \"Ingress_MemMap.h\"\n",
            "Nested SWC memory section",
        ),
        (
            "unknown-rte-section",
            "#define RTE_Ingress_START_SEC_UNKNOWN\n#include \"Rte_MemMap.h\"\n",
            "Unknown RTE memory section",
        ),
    ] {
        check(label, source, false, false, error);
    }
    for (header, prefix) in [
        ("bsw/include/Com_Version.h", "COM"),
        ("os/include/Os_Version.h", "OS"),
        ("bsw/include/Std_Types.h", "STD_TYPES"),
        ("bsw/include/Platform_Types.h", "PLATFORM"),
        ("bsw/include/ComStack_Version.h", "COMSTACKTYPE"),
    ] {
        let path = scratch.0.join(header);
        let original = std::fs::read_to_string(&path).unwrap();
        let token = original
            .lines()
            .find(|line| {
                line.split_whitespace().nth(1)
                    == Some(&format!("{prefix}_AR_RELEASE_MINOR_VERSION"))
            })
            .unwrap()
            .to_owned();
        assert!(original.contains(&token));
        std::fs::write(&path, original.replace(&token, &token.replace("10U", "9U"))).unwrap();
        check(
            &format!("wrong-release-{prefix}"),
            "#include \"generated/src/Rte.c\"\n",
            false,
            false,
            "Incompatible",
        );
        std::fs::write(path, original).unwrap();
    }
}

#[test]
fn bsw_description_native_widths_and_added_api_signatures_match_public_headers() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files(&plan);
    let scratch = Scratch::new();
    for (path, bytes) in &files {
        let target = scratch.0.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let description = files
        .iter()
        .find(|(path, _)| path == "generated/descriptions/Host_Implementation.arxml")
        .unwrap();
    let document =
        roxmltree::Document::parse(std::str::from_utf8(&description.1).unwrap()).unwrap();
    let mut consumer = String::from(
        r#"#include <limits.h>
#include "Arti.h"
#include "Os.h"
#include "Can.h"
#include "CanIf.h"
#include "CanTp.h"
#include "Com.h"
#include "ComM.h"
#include "Dcm.h"
#include "PduR.h"
#include "LSduR.h"
void (*can_disable)(uint8) = Can_DisableControllerInterrupts;
Std_ReturnType (*canif_count)(uint8, uint8 *) = CanIf_GetControllerRxErrorCounter;
Std_ReturnType (*cancel_tx)(PduIdType) = CanTp_CancelTransmit;
uint8 (*invalidate)(Com_SignalIdType) = Com_InvalidateSignal;
Std_ReturnType (*request_mode)(ComM_UserHandleType, ComM_ModeType) = ComM_RequestComMode;
Std_ReturnType (*security_level)(Dcm_SecLevelType *) = Dcm_GetSecurityLevel;
void (*disable_route)(PduR_RoutingPathGroupIdType, boolean) = PduR_DisableRouting;
LSduR_PBConfigIdType (*routing_configuration)(void) = LSduR_GetConfigurationId;
"#,
    );
    for (index, native) in [
        "Arti_AddressCapture",
        "AlarmBaseRefType",
        "TaskRefType",
        "Com_StatusType",
        "ComM_InitStatusType *",
        "ComM_UserHandleType",
        "PduR_PBConfigIdType",
        "Dcm_SecLevelType *",
    ]
    .into_iter()
    .enumerate()
    {
        let base = document
            .descendants()
            .find(|node| {
                node.has_tag_name("SW-BASE-TYPE")
                    && node.children().any(|child| {
                        child.has_tag_name("NATIVE-DECLARATION") && child.text() == Some(native)
                    })
            })
            .unwrap_or_else(|| panic!("missing {native}"));
        let bits = base
            .children()
            .find(|node| node.has_tag_name("BASE-TYPE-SIZE"))
            .unwrap()
            .text()
            .unwrap()
            .parse::<u32>()
            .unwrap();
        consumer.push_str(&format!(
            "typedef char width_{index}[(sizeof({native}) * CHAR_BIT == {bits}u) ? 1 : -1];\n"
        ));
    }
    std::fs::write(scratch.0.join("consumer.c"), consumer).unwrap();
    let mut compiler = tooling::c_compiler();
    compiler.current_dir(&scratch.0).args([
        "-std=c99",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-Igenerated/include",
        "-Ibsw/include",
        "-Ios/include",
        "-Iintegration/include",
        "-c",
        "consumer.c",
        "-o",
        "consumer.o",
    ]);
    let output = tooling::run_public_command(
        &mut compiler,
        &scratch.0,
        "bsw-description-consumer",
        std::time::Duration::from_secs(60),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
