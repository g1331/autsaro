use autosar_config_core::integration::{InputSource, PlanDependencies, RuntimeCatalog, build_plan};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn compile(directory: &Path, sources: &[&str], output: &str) -> Output {
    let mut command = Command::new("gcc");
    command.args([
        "-std=c99",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-Iinclude",
    ]);
    command
        .args(sources)
        .args(["-o", output])
        .current_dir(directory);
    command
        .output()
        .expect("The configured GCC must be available")
}

fn passed(output: Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rejected(directory: &Path, name: &str, source: &str, expected: &str) {
    fs::write(directory.join(name), source).unwrap();
    let output = compile(directory, &["-c", name], "rejected.o");
    assert!(!output.status.success(), "{name} unexpectedly compiled");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains(expected), "{name}: {diagnostic}");
}

pub fn verify() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let inputs = super::epic4_plan::inputs();
    let plan =
        build_plan(&inputs, &dependencies, &runtime).unwrap_or_else(|issues| panic!("{issues:?}"));
    let contracts = plan
        .component_contract_files()
        .unwrap_or_else(|issues| panic!("{issues:?}"));
    assert_eq!(
        contracts.files(),
        plan.component_contract_files().unwrap().files()
    );
    let mut reversed = inputs.clone();
    reversed.reverse();
    assert_eq!(
        contracts.files(),
        build_plan(&reversed, &dependencies, &runtime)
            .unwrap()
            .component_contract_files()
            .unwrap()
            .files()
    );
    let scratch = super::Scratch::new();
    let delivery = scratch.0.join("contract");
    let preview = contracts.preview(&delivery).unwrap();
    assert!(!delivery.exists(), "Preview wrote an output directory");
    let report = contracts
        .generate_previewed(&delivery, &preview.revision)
        .unwrap();
    assert_eq!(report.files.len(), 7);
    for (name, bytes) in contracts.files() {
        assert_eq!(&fs::read(delivery.join(name)).unwrap(), bytes, "{name}");
    }
    let names: Vec<_> = contracts
        .files()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(names.iter().filter(|name| name.ends_with(".h")).count(), 5);
    assert!(!names.iter().any(|name| name.ends_with(".c")));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(delivery.join("contract.json")).unwrap()).unwrap();
    let owners = metadata["declarationOwners"].as_object().unwrap();
    assert_eq!(owners.len(), 6);
    for symbol in plan
        .description()
        .symbols
        .iter()
        .filter(|symbol| symbol.declaration_owner.starts_with("story-4.11:"))
    {
        let owner = owners[&symbol.symbol].as_str().unwrap();
        let prototype = format!("{}(", symbol.symbol);
        assert!(
            fs::read_to_string(delivery.join(owner))
                .unwrap()
                .contains(&prototype)
        );
        assert_eq!(
            contracts
                .files()
                .iter()
                .filter(|(name, _)| name.ends_with(".h"))
                .map(|(_, bytes)| std::str::from_utf8(bytes)
                    .unwrap()
                    .matches(&prototype)
                    .count())
                .sum::<usize>(),
            1,
            "{} must have exactly one declaration owner",
            symbol.symbol
        );
    }
    let app = fs::read_to_string(delivery.join("include/Rte_EchoApplication.h")).unwrap();
    assert!(app.contains("Std_ReturnType Rte_Read_RxValue_Value(uint32 *data);"));
    assert!(app.contains("Std_ReturnType Rte_Write_TxValue_Value(uint32 data);"));
    assert!(app.contains(
        "Std_ReturnType EchoApplication_ReadData(Dcm_DataElement_ApplicationValueType Data);"
    ));
    assert!(app.contains("void EchoApplication_Periodic(void);"));
    let types = fs::read_to_string(delivery.join("include/Rte_Type.h")).unwrap();
    assert!(types.contains("typedef uint8 Dcm_DataElement_ApplicationValueType[4];"));
    for forbidden in ["valid", "Rte_Invalidate", "OpStatus", "NRC"] {
        assert!(!app.contains(forbidden), "{forbidden}");
    }
    // Compile a consumer independent of the generator and future runtime.
    // Test-private RTE stubs are written beside, never inside the delivery.
    let consumer = scratch.0.join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::create_dir(consumer.join("include")).unwrap();
    for (name, bytes) in contracts
        .files()
        .iter()
        .filter(|(name, _)| name.ends_with(".h"))
    {
        fs::write(consumer.join(name), bytes).unwrap();
    }
    fs::write(
        consumer.join("application.c"),
        r#"#include "Rte_EchoApplication.h"
#include "Rte_EchoApplication.h"
void EchoApplication_Periodic(void)
{
    uint32 data = 0U;
    Std_ReturnType status = Rte_Read_RxValue_Value(&data);
    if (status == E_OK) {
        status = Rte_Write_TxValue_Value(data);
    }
    if (status != E_OK) {
        /* Test consumer detects infrastructure errors without a private valid argument. */
    }
}
Std_ReturnType EchoApplication_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    Data[0] = 1U;
    Data[1] = 2U;
    Data[2] = 3U;
    Data[3] = 4U;
    return E_OK;
}
"#,
    )
    .unwrap();
    fs::write(
        consumer.join("service.c"),
        r#"#include "Rte_DcmService.h"
Std_ReturnType DcmService_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    return Rte_Call_ApplicationValue_ReadData(Data);
}
"#,
    )
    .unwrap();
    fs::write(
        consumer.join("stubs.c"),
        r#"#include "Rte_EchoApplication.h"
#include "Rte_DcmService.h"
#include <stddef.h>
static uint8 stopped = 0U;
Std_ReturnType Rte_Read_RxValue_Value(uint32 *data)
{
    if (data == NULL) { return RTE_E_COM_STOPPED; }
    *data = 42U;
    return E_OK;
}
Std_ReturnType Rte_Write_TxValue_Value(uint32 data)
{
    return (data == 42U) ? E_OK : RTE_E_COM_STOPPED;
}
Std_ReturnType Rte_Call_ApplicationValue_ReadData(Dcm_DataElement_ApplicationValueType Data)
{
    if (stopped != 0U) { return RTE_E_COM_STOPPED; }
    return EchoApplication_ReadData(Data);
}
int main(void)
{
    Dcm_DataElement_ApplicationValueType Data = {0U, 0U, 0U, 0U};
    Std_ReturnType status;
    typedef char four_byte_shape[(sizeof(Data) == 4U) ? 1 : -1];
    four_byte_shape shape = {0};
    EchoApplication_Periodic();
    status = DcmService_ReadData(Data);
    if ((shape[0] != 0) || (status != E_OK) || (Data[0] != 1U) || (Data[1] != 2U) ||
        (Data[2] != 3U) || (Data[3] != 4U)) { return 1; }
    stopped = 1U;
    status = DcmService_ReadData(Data);
    if ((status != RTE_E_COM_STOPPED) || (Data[3] != 4U)) { return 2; }
    if ((RTE_E_NEVER_RECEIVED != 133U) || (RTE_E_MAX_AGE_EXCEEDED != 64U)) { return 3; }
    return 0;
}
"#,
    )
    .unwrap();
    passed(compile(
        &consumer,
        &["application.c", "service.c", "stubs.c"],
        "contract-test.exe",
    ));
    passed(
        Command::new(consumer.join("contract-test.exe"))
            .output()
            .unwrap(),
    );
    for (name, source, expected) in [
        (
            "private-valid.c",
            "#include \"Rte_EchoApplication.h\"\nvoid test(void) { uint32 data=0U; uint8 valid=0U; (void)Rte_Read_RxValue_Value(&data, &valid); }\n",
            "too many arguments",
        ),
        (
            "invalidate.c",
            "#include \"Rte_EchoApplication.h\"\nvoid test(void) { (void)Rte_Invalidate_TxValue_Value(); }\n",
            "implicit declaration",
        ),
        (
            "private-service.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void) { uint8 data[4]={0U}; uint8 nrc=0U; (void)Rte_Call_ApplicationValue_ReadData(0U, data, &nrc); }\n",
            "too many arguments",
        ),
        (
            "wrong-write.c",
            "#include \"Rte_EchoApplication.h\"\nvoid test(void) { uint32 data=0U; (void)Rte_Write_TxValue_Value(&data); }\n",
            "pointer",
        ),
        (
            "wrong-operation.c",
            "#include \"Rte_DcmService.h\"\nvoid test(void) { uint8 data[4]={0U}; (void)Rte_Call_ApplicationValue_WriteData(data); }\n",
            "implicit declaration",
        ),
        (
            "wrong-callback.c",
            "#include \"Rte_EchoApplication.h\"\nStd_ReturnType (*callback)(uint32 *) = &EchoApplication_ReadData;\n",
            "incompatible pointer",
        ),
    ] {
        rejected(&consumer, name, source, expected);
    }
    let snapshot = contracts.files().to_vec();
    let cases: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("core/tests/fixtures/epic4/negative/cases.json")).unwrap(),
    )
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let mut mutated = inputs.clone();
        for override_path in case["overrides"].as_array().unwrap() {
            let path = root
                .join("core/tests/fixtures/epic4")
                .join(override_path.as_str().unwrap());
            let file = path.file_name().unwrap().to_str().unwrap();
            *mutated
                .iter_mut()
                .find(|source| source.logical_path() == file)
                .unwrap() = InputSource::new(file, fs::read(&path).unwrap()).unwrap();
        }
        assert!(
            build_plan(&mutated, &dependencies, &runtime).is_err(),
            "{case}"
        );
        for (name, bytes) in &snapshot {
            assert_eq!(
                &fs::read(delivery.join(name)).unwrap(),
                bytes,
                "{case}: {name}"
            );
        }
    }
    let mut renamed = inputs.clone();
    for source in &mut renamed {
        let text = std::str::from_utf8(source.bytes())
            .unwrap()
            .replace("EchoApplication", "LocalApplication")
            .replace("RxValue", "InputValue")
            .replace("TxValue", "OutputValue")
            .replace("Dcm_DataElement_ApplicationValueType", "SnapshotBytes");
        *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
    }
    let changed = build_plan(&renamed, &dependencies, &runtime)
        .unwrap()
        .component_contract_files()
        .unwrap();
    let header = changed
        .files()
        .iter()
        .find(|(name, _)| name == "include/Rte_LocalApplication.h")
        .unwrap();
    let text = std::str::from_utf8(&header.1).unwrap();
    assert!(text.contains("Rte_Read_InputValue_Value(uint32 *data)"));
    assert!(text.contains("Rte_Write_OutputValue_Value(uint32 data)"));
    assert!(text.contains("LocalApplication_ReadData(SnapshotBytes Data)"));
    let changed_preview = changed.preview(&scratch.0.join("renamed")).unwrap();
    changed
        .generate_previewed(&scratch.0.join("renamed"), &changed_preview.revision)
        .unwrap();
    let renamed_consumer = scratch.0.join("renamed-consumer");
    fs::create_dir(&renamed_consumer).unwrap();
    fs::create_dir(renamed_consumer.join("include")).unwrap();
    for (name, bytes) in changed
        .files()
        .iter()
        .filter(|(name, _)| name.ends_with(".h"))
    {
        fs::write(renamed_consumer.join(name), bytes).unwrap();
    }
    for name in ["application.c", "service.c", "stubs.c"] {
        let text = fs::read_to_string(consumer.join(name))
            .unwrap()
            .replace("EchoApplication", "LocalApplication")
            .replace("RxValue", "InputValue")
            .replace("TxValue", "OutputValue")
            .replace("Dcm_DataElement_ApplicationValueType", "SnapshotBytes");
        fs::write(renamed_consumer.join(name), text).unwrap();
    }
    passed(compile(
        &renamed_consumer,
        &["application.c", "service.c", "stubs.c"],
        "contract-test.exe",
    ));
    passed(
        Command::new(renamed_consumer.join("contract-test.exe"))
            .output()
            .unwrap(),
    );
    for (before, after) in [
        ("EchoApplication", "Type"),
        ("Dcm_DataElement_ApplicationValueType", "RTE_H"),
        ("Dcm_DataElement_ApplicationValueType", "restrict"),
        ("EchoApplication_Periodic", "while"),
        ("EchoApplication_Periodic", "_ReservedRunnable"),
    ] {
        let mut collision = inputs.clone();
        for source in &mut collision {
            let text = std::str::from_utf8(source.bytes())
                .unwrap()
                .replace(before, after);
            *source = InputSource::new(source.logical_path(), text.into_bytes()).unwrap();
        }
        let collision = build_plan(&collision, &dependencies, &runtime).unwrap();
        let issues = collision.component_contract_files().err().unwrap();
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "CONTRACT_NAME_COLLISION"
                    && issue.file.is_some()
                    && issue.object.is_some())
        );
        for (name, bytes) in &snapshot {
            assert_eq!(&fs::read(delivery.join(name)).unwrap(), bytes);
        }
    }
    let second_preview = contracts.preview(&delivery).unwrap();
    let second = contracts
        .generate_previewed(&delivery, &second_preview.revision)
        .unwrap();
    assert!(second.previous_output_directory.is_some());
    for (name, bytes) in &snapshot {
        assert_eq!(&fs::read(delivery.join(name)).unwrap(), bytes);
    }
    let stale = contracts.preview(&delivery).unwrap();
    let target = delivery.join("include/Rte.h");
    let mut edited = fs::read(&target).unwrap();
    edited.extend_from_slice(b"\n/* user edit */\n");
    fs::write(&target, &edited).unwrap();
    assert!(
        contracts
            .generate_previewed(&delivery, &stale.revision)
            .is_err()
    );
    assert_eq!(fs::read(&target).unwrap(), edited);
    fs::write(delivery.join("notes.txt"), b"user notes").unwrap();
    assert!(contracts.preview(&delivery).is_err());
    assert_eq!(fs::read(delivery.join("notes.txt")).unwrap(), b"user notes");
}
