use super::{Scratch, archive, create_pair, epic4_plan, tooling};
use autosar_config_core::{Workspace, generator};
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(windows)]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::process::Stdio;

pub(super) fn saved_handoff_reopens_after_move_and_reproduces_host_sources() {
    let temp = Scratch::new();
    let _ = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let split = temp.0.join("Other/Alpha.arxml");
    fs::create_dir(split.parent().unwrap()).unwrap();
    let xml = fs::read_to_string(&source).unwrap();
    let start = xml
        .find("<I-SIGNAL><SHORT-NAME>ISignal_SendCount</SHORT-NAME>")
        .unwrap();
    let end = start + xml[start..].find("</I-SIGNAL>").unwrap() + "</I-SIGNAL>".len();
    let signal = &xml[start..end];
    fs::write(&source, xml.replacen(signal, "", 1)).unwrap();
    fs::write(&split, format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Alpha</SHORT-NAME><ELEMENTS>{signal}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n")).unwrap();
    let mut original = Workspace::open(vec![source.clone(), split.clone()], archive()).unwrap();
    let output = temp.0.join("Handoff");
    let preview =
        generator::preview_handoff(&mut original, &output, tooling::native_target()).unwrap();
    assert!(
        preview
            .files
            .iter()
            .any(|file| file.path == "inputs/000.arxml")
    );
    generator::generate_handoff_previewed(
        &mut original,
        &output,
        &preview.revision,
        tooling::native_target(),
    )
    .unwrap();
    let moved = temp.0.join("Received/Handoff");
    fs::create_dir(moved.parent().unwrap()).unwrap();
    fs::rename(&output, &moved).unwrap();
    let manifest = fs::read_to_string(moved.join("files.list")).unwrap();
    let listed: Vec<_> = manifest
        .lines()
        .filter(|name| name.starts_with("inputs/"))
        .collect();
    assert_eq!(listed.len(), 2);
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(moved.join("handoff.json")).unwrap()).unwrap();
    let sources = metadata["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert!(
        sources
            .iter()
            .all(|item| item["originalName"] == "Alpha.arxml")
    );
    assert!(
        !fs::read_to_string(moved.join("handoff.json"))
            .unwrap()
            .contains(temp.0.to_str().unwrap())
    );
    let delivered_paths: Vec<_> = sources
        .iter()
        .map(|item| moved.join(item["path"].as_str().unwrap()))
        .collect();
    let delivered_bytes: Vec<_> = delivered_paths
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert!(delivered_bytes.contains(&fs::read(&source).unwrap()));
    assert!(delivered_bytes.contains(&fs::read(&split).unwrap()));
    let mut reopened = generator::open_handoff(&moved, archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let regenerated = temp.0.join("Received/Rebuilt");
    generator::generate_handoff(&mut reopened, &regenerated, tooling::native_target()).unwrap();
    for name in fs::read_to_string(regenerated.join("files.list"))
        .unwrap()
        .lines()
    {
        if name != "README.md" {
            assert_eq!(
                fs::read(regenerated.join(name)).unwrap(),
                fs::read(moved.join(name)).unwrap(),
                "{name}"
            );
        }
    }
    #[cfg(windows)]
    assert!(Path::new(&tooling::build_host(&regenerated).unwrap().binary_path).exists());
    #[cfg(windows)]
    {
        assert!(Path::new(&tooling::build_host(&moved).unwrap().binary_path).exists());
        assert!(generator::open_handoff(&moved, archive()).is_ok());
    }
    assert!(
        fs::read_to_string(moved.join("README.md"))
            .unwrap()
            .contains("handoff.json")
    );
}

pub(super) fn handoff_rejects_dirty_stale_and_modified_output_without_losing_old_package() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let output = temp.0.join("Handoff");
    generator::generate_handoff(&mut ecu, &output, tooling::native_target()).unwrap();
    let original = fs::read(output.join("inputs/000.arxml")).unwrap();
    let frame = ecu.view().frames[0].path.clone();
    ecu.update_frame(&frame, serde_json::json!({"id": 802}))
        .unwrap();
    assert!(
        generator::preview_handoff(&mut ecu, &output, tooling::native_target())
            .unwrap_err()
            .contains("保存")
    );
    assert_eq!(fs::read(output.join("inputs/000.arxml")).unwrap(), original);
    ecu.save().unwrap();
    let external = fs::read_to_string(&source).unwrap().replacen(
        "<VALUE>802</VALUE>",
        "<VALUE>803</VALUE>",
        1,
    );
    fs::write(&source, &external).unwrap();
    assert!(
        generator::generate_handoff(&mut ecu, &output, tooling::native_target())
            .unwrap_err()
            .contains("外部修改")
    );
    assert_eq!(fs::read(output.join("inputs/000.arxml")).unwrap(), original);
    let mut reopened = Workspace::open(vec![source], archive()).unwrap();
    fs::write(output.join("inputs/000.arxml"), b"owner changed this input").unwrap();
    assert!(generator::generate_handoff(&mut reopened, &output, tooling::native_target()).is_err());
    assert_eq!(
        fs::read(output.join("inputs/000.arxml")).unwrap(),
        b"owner changed this input"
    );
}

#[cfg(windows)]
pub(super) fn source_junction_cannot_be_imported_for_handoff() {
    let temp = Scratch::new();
    let _ = create_pair(&temp.0);
    let alias = temp.0.join("SourceAlias");
    let linked = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(temp.0.join("Alpha"))
        .output()
        .unwrap();
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let result = Workspace::open(vec![alias.join("Alpha.arxml")], archive());
    assert!(result.err().unwrap().contains("重解析点"));
    fs::remove_dir(&alias).unwrap();
    assert!(temp.0.join("Alpha/Alpha.arxml").exists());
}

pub(super) fn delivered_input_removal_or_tampering_cannot_reproduce_host_project() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Handoff");
    generator::generate_handoff(&mut ecu, &output, tooling::native_target()).unwrap();
    let input = output.join("inputs/000.arxml");
    let original = fs::read(&input).unwrap();
    fs::remove_file(&input).unwrap();
    assert!(generator::open_handoff(&output, archive()).is_err());
    let original = String::from_utf8(original).unwrap();
    assert!(original.contains("<VALUE>801</VALUE>"));
    let tampered = original.replacen("<VALUE>801</VALUE>", "<VALUE>9999</VALUE>", 1);
    fs::write(&input, &tampered).unwrap();
    assert!(generator::open_handoff(&output, archive()).is_err());
    assert!(!temp.0.join("BadRebuild").exists());
    assert_eq!(fs::read_to_string(&input).unwrap(), tampered);

    fs::write(&input, original).unwrap();
    let metadata_path = output.join("handoff.json");
    let metadata = fs::read_to_string(&metadata_path).unwrap();
    fs::write(
        &metadata_path,
        metadata.replacen("CP/FO R24-11", "CP/FO R99-99", 1),
    )
    .unwrap();
    let records_path = output.join("files.sha256");
    let seal_metadata = || {
        let digest = format!("{:x}", Sha256::digest(fs::read(&metadata_path).unwrap()));
        let records = fs::read_to_string(&records_path).unwrap();
        let updated = records
            .lines()
            .map(|line| {
                if line.ends_with("  handoff.json") {
                    format!("{digest}  handoff.json")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&records_path, updated).unwrap();
    };
    seal_metadata();
    assert!(generator::open_handoff(&output, archive()).is_err());

    let mut wrong_roots: serde_json::Value = serde_json::from_str(&metadata).unwrap();
    wrong_roots["sources"][0]["packageRoots"] = serde_json::json!(["WrongPackage"]);
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&wrong_roots).unwrap(),
    )
    .unwrap();
    seal_metadata();
    assert!(generator::open_handoff(&output, archive()).is_err());
}

#[cfg(windows)]
pub(super) fn moved_reference_bundle_verifies_offline_and_rejects_wrong_vector() {
    let temp = Scratch::new();
    let bundle = temp.0.join("Reference");
    let mut producer = Command::new(env!("CARGO_BIN_EXE_package_host_reference"));
    producer
        .arg(&bundle)
        .args(["--target", tooling::native_target().spec().id])
        .arg("--xsd-archive")
        .arg(archive());
    let producer = tooling::run_public_command(
        &mut producer,
        &temp.0,
        "reference-package",
        std::time::Duration::from_secs(60),
    );
    assert!(
        producer.status.success(),
        "{}",
        String::from_utf8_lossy(&producer.stderr)
    );
    let moved = temp.0.join("Received reference with spaces");
    fs::rename(&bundle, &moved).unwrap();
    let verify = |suffix: &str| {
        let report = temp.0.join(format!("{suffix}.json"));
        let mut command =
            tooling::ecu_verify_command(&moved, &temp.0.join(format!("reference-build-{suffix}")));
        command.arg("--report-path").arg(&report);
        let result = tooling::run_public_command(
            &mut command,
            &temp.0,
            suffix,
            std::time::Duration::from_secs(180),
        );
        let report_text = fs::read_to_string(&report).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(report_text.trim_start_matches('\u{feff}')).unwrap();
        (result.status.success(), value)
    };
    let (passed, result) = verify("pass");
    assert!(passed, "{result}");
    assert_eq!(result["status"], "passed");
    assert!(
        result["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"] == "Alpha diagnostic rejection then recovery")
    );

    let empty_path = temp.0.join("NoCompiler");
    fs::create_dir(&empty_path).unwrap();
    let report = temp.0.join("no-compiler.json");
    let mut no_compiler = tooling::ecu_verify_command(&moved, &temp.0.join("no-compiler-build"));
    no_compiler
        .arg("--report-path")
        .arg(&report)
        .env("AUTOSAR_CC", empty_path.join("gcc.exe"));
    let no_compiler = tooling::run_public_command(
        &mut no_compiler,
        &temp.0,
        "missing-compiler",
        std::time::Duration::from_secs(60),
    );
    assert!(!no_compiler.status.success());
    let result: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_eq!(result["status"], "failed");

    let report_alias = temp.0.join("ReportAlias");
    let linked = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&report_alias)
        .arg(&moved)
        .output()
        .unwrap();
    assert!(linked.status.success());
    let mut linked_report =
        tooling::ecu_verify_command(&moved, &temp.0.join("linked-report-build"));
    linked_report
        .arg("--report-path")
        .arg(report_alias.join("new-directory/result.json"));
    let linked_report = tooling::run_public_command(
        &mut linked_report,
        &temp.0,
        "linked-report",
        std::time::Duration::from_secs(60),
    );
    assert!(!linked_report.status.success());
    assert!(!moved.join("new-directory").exists());
    fs::remove_dir(report_alias).unwrap();

    let vector_path = moved.join("vectors.json");
    let original = fs::read_to_string(&vector_path).unwrap();
    fs::write(
        &vector_path,
        original.replacen("X 801 2 B001", "X 801 2 DEADBEEF", 1),
    )
    .unwrap();
    let hash_path = moved.join("files.sha256");
    let seal_vectors = || {
        let digest = format!("{:x}", Sha256::digest(fs::read(&vector_path).unwrap()));
        let records = fs::read_to_string(&hash_path).unwrap();
        let updated = records
            .lines()
            .map(|line| {
                if line.ends_with("  vectors.json") {
                    format!("{digest}  vectors.json")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&hash_path, updated).unwrap();
    };
    seal_vectors();
    let (passed, result) = verify("wrong-vector");
    assert!(!passed);
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"].as_str().unwrap().contains("expected"),
        "{result}"
    );

    let mut slow: serde_json::Value = serde_json::from_str(&original).unwrap();
    slow["timeoutMs"] = serde_json::json!(1);
    slow["cases"][0]["input"] = serde_json::json!("G 0\n".repeat(20_000));
    fs::write(&vector_path, serde_json::to_vec_pretty(&slow).unwrap()).unwrap();
    seal_vectors();
    let (passed, result) = verify("timeout");
    assert!(!passed);
    assert_eq!(result["status"], "failed");
    assert!(
        result["processStatus"] == "timeout"
            || result["error"].as_str().unwrap().contains("deadline"),
        "{result}"
    );

    fs::remove_file(moved.join("Beta/inputs/000.arxml")).unwrap();
    let (passed, result) = verify("missing-input");
    assert!(!passed);
    assert_eq!(result["status"], "failed");
}

pub(super) fn regeneration_preserves_user_edits_to_generated_files() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    for name in [
        "src/Com.c",
        "Ecu_Config.c",
        "README.md",
        "tools/ecu-tool.py",
        "files.list",
        "files.sha256",
    ] {
        let changed = b"user edited generated output";
        let file = output.join(name);
        let original = fs::read(&file).unwrap();
        fs::write(&file, changed).unwrap();
        assert!(
            generator::generate(&mut ecu, &output, tooling::native_target()).is_err(),
            "{name}"
        );
        assert_eq!(fs::read(&file).unwrap(), changed, "{name}");
        fs::write(file, original).unwrap();
    }
}

#[cfg(windows)]
pub(super) fn generated_handoff_builds_and_runs_after_moving_without_the_workbench() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let delivered = temp.0.join("Delivered ECU with spaces");
    fs::rename(&output, &delivered).unwrap();
    let build_directory = temp.0.join("native-host-build");
    let build = tooling::run_public_command(
        &mut tooling::ecu_build_command(&delivered, &build_directory, "host", None),
        &temp.0,
        "moved-host-build",
        std::time::Duration::from_secs(180),
    );
    assert!(
        build.status.success(),
        "{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    let binary = tooling::native_binary(&build_directory, "ecu_host");
    let logs = temp.0.join("host-process-logs");
    fs::create_dir(&logs).unwrap();
    let mut spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![binary.as_os_str().to_owned()],
        temp.0.clone(),
        Vec::new(),
        std::time::Duration::from_secs(15),
        logs,
    )
    .unwrap();
    spec.stdin_stream = true;
    let owner = autosar_config_core::execution::ProcessOwner::new().unwrap();
    let mut process = owner.spawn(spec, None).unwrap();
    process.write_stdin(b"T 10\n").unwrap();
    process.close_stdin();
    let result = process.wait().unwrap();
    assert!(result.success(), "{result:?}");
    assert_eq!(
        fs::read_to_string(&result.stdout).unwrap().trim(),
        "X 801 2 2800"
    );

    fs::write(&binary, b"owner binary").unwrap();
    let repeated = tooling::run_public_command(
        &mut tooling::ecu_build_command(&delivered, &build_directory, "host", None),
        &temp.0,
        "owner-output-preserved",
        std::time::Duration::from_secs(60),
    );
    assert!(!repeated.status.success());
    assert_eq!(fs::read(&binary).unwrap(), b"owner binary");
}

pub(super) fn regeneration_rejects_missing_proof_and_unlisted_user_content() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let config = fs::read(output.join("Ecu_Config.c")).unwrap();

    let proof = output.join("files.sha256");
    let proof_contents = fs::read(&proof).unwrap();
    fs::remove_file(&proof).unwrap();
    assert!(
        generator::generate(&mut ecu, &output, tooling::native_target())
            .unwrap_err()
            .contains("完整性记录")
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);

    fs::write(&proof, b"invalid proof\n").unwrap();
    assert!(generator::generate(&mut ecu, &output, tooling::native_target()).is_err());
    fs::write(&proof, proof_contents).unwrap();
    let manifest = output.join("files.list");
    let manifest_contents = fs::read(&manifest).unwrap();
    fs::remove_file(&manifest).unwrap();
    assert!(
        generator::generate(&mut ecu, &output, tooling::native_target())
            .unwrap_err()
            .contains("文件清单")
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
    fs::write(&manifest, manifest_contents).unwrap();
    let extra = output.join("notes.txt");
    fs::write(&extra, b"user content").unwrap();
    assert!(generator::generate(&mut ecu, &output, tooling::native_target()).is_err());
    assert_eq!(fs::read(&extra).unwrap(), b"user content");
    fs::remove_file(&extra).unwrap();
    let extra_dir = output.join("user-data");
    fs::create_dir(&extra_dir).unwrap();
    assert!(generator::generate(&mut ecu, &output, tooling::native_target()).is_err());
    assert!(extra_dir.is_dir());
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
}

pub(super) fn regeneration_rejects_a_missing_generated_file() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let missing = output.join("include/Can.h");
    fs::remove_file(&missing).unwrap();
    let manifest = fs::read(output.join("files.list")).unwrap();

    assert!(generator::generate(&mut ecu, &output, tooling::native_target()).is_err());
    assert!(!missing.exists());
    assert_eq!(fs::read(output.join("files.list")).unwrap(), manifest);
}

#[cfg(windows)]
pub(super) fn generation_rejects_junction_output_without_touching_its_target() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let owner = temp.0.join("OwnerData");
    fs::create_dir(&owner).unwrap();
    fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();
    let output = temp.0.join("Generated");
    let junction = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&output)
        .arg(&owner)
        .output()
        .unwrap();
    assert!(
        junction.status.success(),
        "{}",
        String::from_utf8_lossy(&junction.stderr)
    );

    assert!(
        generator::generate(&mut ecu, &output, tooling::native_target())
            .unwrap_err()
            .contains("重解析点")
    );
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
    assert!(!owner.join("Ecu_Config.c").exists());
    fs::remove_dir(&output).unwrap();
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
}

pub(super) fn generation_rejects_trailing_dot_alias_without_touching_owner_directory() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let owner = temp.0.join("OwnerData");
    fs::create_dir(&owner).unwrap();
    fs::write(owner.join("sentinel.txt"), b"owner content").unwrap();

    assert!(generator::generate(&mut ecu, &owner.join("."), tooling::native_target()).is_err());
    assert_eq!(
        fs::read(owner.join("sentinel.txt")).unwrap(),
        b"owner content"
    );
    assert!(!owner.join("OwnerData").exists());
}

pub(super) fn regeneration_keeps_previous_output_tree() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    let stage_collision = temp
        .0
        .join(format!(".autosar-config-stage-{}-0", std::process::id()));
    fs::create_dir(&stage_collision).unwrap();
    fs::write(
        stage_collision.join("owner.txt"),
        b"keep staged owner content",
    )
    .unwrap();
    let initial = generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert!(initial.previous_output_directory.is_none());
    assert_eq!(
        fs::read(stage_collision.join("owner.txt")).unwrap(),
        b"keep staged owner content"
    );
    let old_config = fs::read(output.join("Ecu_Config.c")).unwrap();
    let old_manifest = fs::read(output.join("files.list")).unwrap();
    let collision = temp
        .0
        .join(format!(".autosar-config-backup-{}-0", std::process::id()));
    fs::create_dir(&collision).unwrap();
    fs::write(collision.join("owner.txt"), b"do not replace").unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    let regenerated = generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let previous = PathBuf::from(regenerated.previous_output_directory.unwrap());
    assert!(previous.is_absolute() && !previous.starts_with(&collision));
    assert_eq!(fs::read(previous.join("Ecu_Config.c")).unwrap(), old_config);
    assert_eq!(fs::read(previous.join("files.list")).unwrap(), old_manifest);
    assert_eq!(
        fs::read(collision.join("owner.txt")).unwrap(),
        b"do not replace"
    );
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), old_config);

    let next = generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert_ne!(
        next.previous_output_directory.as_deref(),
        Some(previous.to_str().unwrap())
    );
    assert_eq!(fs::read(previous.join("Ecu_Config.c")).unwrap(), old_config);
}

#[cfg(windows)]
pub(super) fn regeneration_preserves_built_binary_until_owner_moves_it() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&output).unwrap().binary_path;
    let original = fs::read(&binary).unwrap();
    let config = fs::read(output.join("Ecu_Config.c")).unwrap();
    let manifest = fs::read(output.join("files.list")).unwrap();
    let proof = fs::read(output.join("files.sha256")).unwrap();

    let replacement = generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert!(replacement.previous_output_directory.is_some());
    assert_eq!(fs::read(&binary).unwrap(), original);
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), config);
    assert_eq!(fs::read(output.join("files.list")).unwrap(), manifest);
    assert_eq!(fs::read(output.join("files.sha256")).unwrap(), proof);
}

#[cfg(windows)]
pub(super) fn rebuild_rejects_existing_binary_without_overwriting_owner_bytes() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    let generated = generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let output = PathBuf::from(generated.output_directory);
    let binary = tooling::build_host(&output).unwrap().binary_path;
    fs::write(&binary, b"owner modified binary").unwrap();

    assert!(tooling::build_host(&output).is_err());
    assert_eq!(fs::read(&binary).unwrap(), b"owner modified binary");
    let archived = temp.0.join("OwnerBinary.exe");
    fs::rename(&binary, &archived).unwrap();
    let rebuilt = generator::build(
        &output,
        &temp.0.join("Rebuilt"),
        &tooling::execution_settings(),
    )
    .unwrap()
    .binary_path;
    let mut process = Command::new(rebuilt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\n")
        .unwrap();
    let result = process.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "X 801 2 B001"
    );
    assert_eq!(fs::read(archived).unwrap(), b"owner modified binary");
}

pub(super) fn build_rejects_changed_generated_inputs_before_compiling() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    for (case, path, change) in [
        ("changed source", "Ecu_Config.c", "append"),
        ("changed manifest", "files.list", "append"),
        ("extra source", "src/Owner.c", "create"),
        ("missing header", "include/Can.h", "remove"),
    ] {
        let output = temp.0.join(case);
        generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
        let target = output.join(path);
        match change {
            "append" => {
                let mut content = fs::read(&target).unwrap();
                content.extend_from_slice(b"\n/* external edit */\n");
                fs::write(&target, content).unwrap();
            }
            "create" => fs::write(&target, b"int owner(void) { return 1; }\n").unwrap(),
            "remove" => fs::remove_file(&target).unwrap(),
            _ => unreachable!(),
        }
        let before = if target.exists() {
            Some(fs::read(&target).unwrap())
        } else {
            None
        };
        let error = tooling::build_host(&output).unwrap_err();
        assert!(error.contains("拒绝构建"), "{case}: {error}");
        assert!(
            !output
                .join(if cfg!(windows) {
                    "ecu_host.exe"
                } else {
                    "ecu_host"
                })
                .exists(),
            "{case}"
        );
        assert_eq!(
            target.exists().then(|| fs::read(&target).unwrap()),
            before,
            "{case}"
        );
    }
}

#[cfg(windows)]
pub(super) fn build_rejects_source_changed_during_compilation_before_installing_binary() {
    use std::time::{Duration, Instant};

    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let target = output.join("Ecu_Config.c");
    let build = temp.0.join("NativeBuild");
    let project = output.clone();
    let destination = build.clone();
    let settings = tooling::execution_settings();
    let worker = std::thread::spawn(move || generator::build(&project, &destination, &settings));
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let compiling = fs::read_dir(&build)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with("compile-"));
        if compiling {
            break;
        }
        assert!(
            !worker.is_finished(),
            "The real compiler exited before the mutation gate"
        );
        assert!(
            Instant::now() < deadline,
            "The real compiler did not reach staging"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    fs::OpenOptions::new()
        .append(true)
        .open(&target)
        .unwrap()
        .write_all(b"\n/* changed during build */\n")
        .unwrap();
    let result = worker.join().unwrap();
    assert!(result.is_err(), "A changed source must not install a build");
    assert!(!build.join("ecu_host.exe").exists());
    assert!(
        fs::read_to_string(&target)
            .unwrap()
            .contains("changed during build")
    );
}

#[cfg(windows)]
pub(super) fn untouched_output_regenerates_changed_config_and_runs_the_new_schedule() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("Generated");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let original = fs::read(output.join("Ecu_Config.c")).unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), original);

    let binary = tooling::build_host(&output).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin
        .take()
        .unwrap()
        .write_all(b"S 0 54\nT 10\nT 15\n")
        .unwrap();
    let result = ecu.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "X 801 2 B001"
    );
}

pub(super) fn generation_preview_is_read_only_and_confirmed_files_match() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("PreviewOutput");
    fs::create_dir(&output).unwrap();
    let preview = generator::preview_generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert!(fs::read_dir(&output).unwrap().next().is_none());
    assert!(preview.files.iter().all(|file| file.status == "new"));
    assert!(preview.files.iter().any(|file| file.path == "Ecu_Config.c"
        && file.after.as_ref().unwrap().contains("const EcuConfig")));
    generator::generate_previewed(
        &mut ecu,
        &output,
        &preview.revision,
        tooling::native_target(),
    )
    .unwrap();
    for file in &preview.files {
        assert_eq!(
            fs::read_to_string(output.join(&file.path)).unwrap(),
            file.after.as_deref().unwrap()
        );
    }

    let first = fs::read(output.join("Ecu_Config.c")).unwrap();
    let frame = ecu
        .view()
        .frames
        .iter()
        .find(|frame| frame.name == "Command")
        .unwrap()
        .path
        .clone();
    ecu.update_frame(&frame, serde_json::json!({"periodMs": 15}))
        .unwrap();
    ecu.save().unwrap();
    let changed = generator::preview_generate(&mut ecu, &output, tooling::native_target()).unwrap();
    assert_eq!(
        changed
            .files
            .iter()
            .find(|file| file.path == "Ecu_Config.c")
            .unwrap()
            .status,
        "changed"
    );
    assert_eq!(fs::read(output.join("Ecu_Config.c")).unwrap(), first);
    assert!(
        generator::generate_previewed(
            &mut ecu,
            &output,
            &preview.revision,
            tooling::native_target()
        )
        .unwrap_err()
        .contains("预览已失效")
    );
    generator::generate_previewed(
        &mut ecu,
        &output,
        &changed.revision,
        tooling::native_target(),
    )
    .unwrap();
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), first);
}

pub(super) fn generation_preview_rejects_changed_existing_output() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let output = temp.0.join("PreviewOutput");
    generator::generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let preview = generator::preview_generate(&mut ecu, &output, tooling::native_target()).unwrap();
    let original = fs::read(output.join("Ecu_Config.c")).unwrap();
    fs::write(output.join("Ecu_Config.c"), b"owner change").unwrap();
    assert!(
        generator::generate_previewed(
            &mut ecu,
            &output,
            &preview.revision,
            tooling::native_target()
        )
        .is_err()
    );
    assert_eq!(
        fs::read(output.join("Ecu_Config.c")).unwrap(),
        b"owner change"
    );
    assert_ne!(fs::read(output.join("Ecu_Config.c")).unwrap(), original);
}

pub(super) fn generation_preview_rejects_user_files_binaries_and_bad_proofs_before_writing() {
    let temp = Scratch::new();
    let (mut ecu, _) = create_pair(&temp.0);
    let user_output = temp.0.join("UserOutput");
    fs::create_dir(&user_output).unwrap();
    fs::write(user_output.join("owner.txt"), b"keep me").unwrap();
    assert!(generator::preview_generate(&mut ecu, &user_output, tooling::native_target()).is_err());
    assert_eq!(fs::read(user_output.join("owner.txt")).unwrap(), b"keep me");

    let built_output = temp.0.join("BuiltOutput");
    generator::generate(&mut ecu, &built_output, tooling::native_target()).unwrap();
    let binary = built_output.join(if cfg!(windows) {
        "ecu_host.exe"
    } else {
        "ecu_host"
    });
    fs::write(&binary, b"owner binary").unwrap();
    assert!(
        generator::preview_generate(&mut ecu, &built_output, tooling::native_target()).is_err()
    );
    assert_eq!(fs::read(&binary).unwrap(), b"owner binary");

    let changed_output = temp.0.join("ChangedOutput");
    generator::generate(&mut ecu, &changed_output, tooling::native_target()).unwrap();
    fs::write(changed_output.join("Ecu_Config.c"), b"owner edit").unwrap();
    assert!(
        generator::preview_generate(&mut ecu, &changed_output, tooling::native_target()).is_err()
    );
    assert_eq!(
        fs::read(changed_output.join("Ecu_Config.c")).unwrap(),
        b"owner edit"
    );
}

#[cfg(any(windows, target_os = "linux"))]
pub(super) fn requested_native_preflight_failure_never_installs_source() {
    use autosar_config_core::integration::PlanDependencies;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let scratch = Scratch::new();
    let output = scratch.0.join("source-not-installed");
    let inputs = scratch.0.join("inputs");
    fs::create_dir(&inputs).unwrap();
    let sources = epic4_plan::inputs();
    let mut arguments = vec![
        std::ffi::OsString::from("--target"),
        std::ffi::OsString::from(tooling::native_target().spec().id),
        "--xsd-archive".into(),
        dependencies.xsd_archive.into_os_string(),
        "--mod-archive".into(),
        dependencies.mod_archive.into_os_string(),
        "--output".into(),
        output.as_os_str().to_owned(),
    ];
    for source in &sources {
        let path = inputs.join(source.logical_path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source.bytes()).unwrap();
        arguments.extend(["--input".into(), path.into_os_string()]);
    }
    let mut preview = Command::new(env!("CARGO_BIN_EXE_generate_epic4_ecu"));
    preview.args(&arguments);
    let result = tooling::run_public_command(
        &mut preview,
        &scratch.0,
        "pure-cli-preview",
        std::time::Duration::from_secs(60),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let preview: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(preview["preflight"]["status"], "not_run");
    let revision = preview["revision"].as_str().unwrap();
    let mut generate = Command::new(env!("CARGO_BIN_EXE_generate_epic4_ecu"));
    generate
        .args(&arguments)
        .args(["--write", "--revision", revision, "--preflight"])
        .env("AUTOSAR_CC", scratch.0.join("missing-compiler"));
    let result = tooling::run_public_command(
        &mut generate,
        &scratch.0,
        "failed-cli-preflight",
        std::time::Duration::from_secs(60),
    );
    assert!(
        !result.status.success(),
        "Failed native preflight was reported as successful"
    );
    assert!(
        !output.exists(),
        "Failed preflight installed a source package"
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["preflight"]["status"], "failed");
    for source in &sources {
        assert_eq!(
            fs::read(inputs.join(source.logical_path())).unwrap(),
            source.bytes()
        );
    }
}
