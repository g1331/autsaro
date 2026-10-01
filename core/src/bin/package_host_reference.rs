use autosar_config_core::target::BuildTarget;
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn prepare_pair(root: &Path, schema_archive: PathBuf) -> Result<(Workspace, Workspace), String> {
    let mut alpha = Workspace::create(&root.join("Alpha"), "Alpha", schema_archive.clone())?;
    let tx = alpha
        .add_frame("Command".into(), 0x321, 2, Direction::Tx, Some(10), None)?
        .frames[0]
        .path
        .clone();
    alpha.add_signal(tx, "SendCount".into(), 3, 8, 5)?;
    let rx = alpha
        .add_frame("Reply".into(), 0x456, 2, Direction::Rx, None, Some(50))?
        .frames
        .into_iter()
        .find(|frame| frame.name == "Reply")
        .ok_or("Reply frame missing")?
        .path;
    alpha.add_signal(rx, "RecvStatus".into(), 0, 8, 0)?;
    let diagnostic_frame = alpha
        .add_frame("DiagData".into(), 0x500, 4, Direction::Tx, Some(1000), None)?
        .frames
        .into_iter()
        .find(|frame| frame.name == "DiagData")
        .ok_or("DiagData frame missing")?
        .path;
    let signal = alpha
        .add_signal(
            diagnostic_frame,
            "DiagnosticValue".into(),
            0,
            32,
            0x11223344,
        )?
        .signals
        .into_iter()
        .find(|item| item.name == "DiagnosticValue")
        .ok_or("DiagnosticValue signal missing")?
        .path;
    alpha.configure_diagnostic(DiagnosticSettings {
        request_id: 0x700,
        response_id: 0x708,
        s3_ms: 5000,
        n_as_ms: Some(200),
        n_bs_ms: 200,
        n_cr_ms: 200,
        did: 0x1234,
        signal_paths: vec![signal],
        write_enabled: false,
        reset_routine_id: None,
        security_enabled: false,
    })?;
    alpha.save()?;

    let mut beta = Workspace::create(&root.join("Beta"), "Beta", schema_archive)?;
    let rx = beta
        .add_frame("Command".into(), 0x321, 2, Direction::Rx, None, Some(40))?
        .frames[0]
        .path
        .clone();
    beta.add_signal(rx, "RecvCount".into(), 3, 8, 0)?;
    let tx = beta
        .add_frame("Reply".into(), 0x456, 2, Direction::Tx, Some(20), None)?
        .frames
        .into_iter()
        .find(|frame| frame.name == "Reply")
        .ok_or("Reply frame missing")?
        .path;
    beta.add_signal(tx, "SendStatus".into(), 0, 8, 7)?;
    beta.save()?;
    Ok((alpha, beta))
}

fn run(output: &Path, target: BuildTarget, archive: PathBuf) -> Result<(), String> {
    if output.exists() {
        return Err("参考包输出目录已存在；请选择新的目录".into());
    }
    let parent = output.parent().ok_or("输出目录须有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = parent.join(format!(".host-reference-{}-{nonce}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = (|| {
        let archive = archive.clone();
        let sources = stage.join("sources");
        let (mut alpha, mut beta) = prepare_pair(&sources, archive)?;
        generator::generate_handoff(&mut alpha, &stage.join("Alpha"), target)?;
        generator::generate_handoff(&mut beta, &stage.join("Beta"), target)?;
        let canonical_stage = fs::canonicalize(&stage).map_err(|e| e.to_string())?;
        let canonical_sources = fs::canonicalize(&sources).map_err(|e| e.to_string())?;
        if !canonical_sources.starts_with(&canonical_stage) {
            return Err("临时来源目录越界".into());
        }
        fs::remove_dir_all(&sources).map_err(|e| e.to_string())?;
        let mut names = vec![
            "README.md".to_owned(),
            "vectors.json".into(),
            "target.json".into(),
        ];
        for name in fs::read_to_string(stage.join("Alpha/files.list"))
            .map_err(|error| error.to_string())?
            .lines()
            .filter(|name| name.starts_with("tools/"))
        {
            let destination = stage.join(name);
            fs::create_dir_all(destination.parent().ok_or("Tool asset has no parent")?)
                .map_err(|error| error.to_string())?;
            fs::write(
                &destination,
                fs::read(stage.join("Alpha").join(name)).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            names.push(name.to_owned());
        }
        fs::write(
            stage.join("target.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "format": "autosar-build-target-v1",
                "target": target.spec().id,
                "profile": "host-reference",
                "members": ["Alpha", "Beta"],
            }))
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        fs::write(
            stage.join("vectors.json"),
            include_bytes!("../../../runtime/reference-vectors.json"),
        )
        .map_err(|e| e.to_string())?;
        fs::write(
            stage.join("README.md"),
            include_bytes!("../../../runtime/reference-README.md"),
        )
        .map_err(|e| e.to_string())?;
        for ecu in ["Alpha", "Beta"] {
            for name in fs::read_to_string(stage.join(ecu).join("files.list"))
                .map_err(|e| e.to_string())?
                .lines()
            {
                names.push(format!("{ecu}/{name}"));
            }
            names.push(format!("{ecu}/files.list"));
            names.push(format!("{ecu}/files.sha256"));
        }
        names.sort();
        let list = names.join("\n") + "\n";
        let mut hashes = String::new();
        for name in &names {
            writeln!(
                hashes,
                "{:x}  {name}",
                Sha256::digest(fs::read(stage.join(name)).map_err(|e| e.to_string())?)
            )
            .map_err(|e| e.to_string())?;
        }
        writeln!(hashes, "{:x}  files.list", Sha256::digest(list.as_bytes()))
            .map_err(|e| e.to_string())?;
        fs::write(stage.join("files.list"), list).map_err(|e| e.to_string())?;
        fs::write(stage.join("files.sha256"), hashes).map_err(|e| e.to_string())?;
        fs::rename(&stage, output).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        eprintln!("参考包暂存目录保留供检查: {}", stage.display());
    }
    result
}

fn arguments() -> Result<(PathBuf, BuildTarget, PathBuf), String> {
    let mut args = std::env::args_os().skip(1);
    let mut output = None;
    let mut target = None;
    let mut archive = None;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--target") if target.is_none() => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or("Missing --target identity")?;
                target = Some(
                    serde_json::from_value(serde_json::Value::String(value))
                        .map_err(|error| error.to_string())?,
                );
            }
            Some("--xsd-archive") if archive.is_none() => {
                archive = Some(PathBuf::from(
                    args.next().ok_or("Missing --xsd-archive path")?,
                ));
            }
            Some(value) if value.starts_with("--") => {
                return Err(format!("Unknown or repeated argument: {value}"));
            }
            _ if output.is_none() => output = Some(PathBuf::from(argument)),
            _ => return Err("Supply only one new output directory".into()),
        }
    }
    let archive = archive
        .or_else(|| std::env::var_os("AUTOSAR_XSD_ARCHIVE").map(PathBuf::from))
        .ok_or("Supply --xsd-archive or AUTOSAR_XSD_ARCHIVE")?;
    if !archive.is_absolute() || !archive.is_file() {
        return Err("The XSD archive must be an existing absolute file".into());
    }
    Ok((
        output.ok_or("Supply the new reference output directory")?,
        target.ok_or("Supply --target windows-x64-controlled-v1|linux-x64-controlled-v1")?,
        archive,
    ))
}

fn main() {
    let result = arguments().and_then(|(output, target, archive)| run(&output, target, archive));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
