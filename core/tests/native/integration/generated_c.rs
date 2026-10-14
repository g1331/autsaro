//! Generated contract headers compiled and linked by independent consumers.
use super::*;
use std::path::Path;

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn generated_headers_compile_and_link_independent_scalar_and_void_server_signatures() {
    for renamed in [false, true] {
        let mut inputs = pipeline_operation_inputs();
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
            sections.contains(".bss.rte.Observe.VAR_CLEARED_UNSPECIFIED"),
            "{sections}"
        );
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
        let compiled = tooling::ecu_build_command(&project, &build, "host-batch", None)
            .output()
            .unwrap();
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
    void (*array_server)(Dcm_DataElement_ApplicationValueType) = &Ingress_ReadData;
    scalar_server(23u, &output, &state);
    array_server(data);
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
