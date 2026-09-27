use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator, schema};
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

fn run(output: &Path) -> Result<(), String> {
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
        let archive = schema::schema_archive(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."));
        let sources = stage.join("sources");
        let (mut alpha, mut beta) = prepare_pair(&sources, archive)?;
        generator::generate_handoff(&mut alpha, &stage.join("Alpha"))?;
        generator::generate_handoff(&mut beta, &stage.join("Beta"))?;
        let canonical_stage = fs::canonicalize(&stage).map_err(|e| e.to_string())?;
        let canonical_sources = fs::canonicalize(&sources).map_err(|e| e.to_string())?;
        if !canonical_sources.starts_with(&canonical_stage) {
            return Err("临时来源目录越界".into());
        }
        fs::remove_dir_all(&sources).map_err(|e| e.to_string())?;
        fs::write(
            stage.join("verify.ps1"),
            include_bytes!("../../../runtime/reference-verify.ps1"),
        )
        .map_err(|e| e.to_string())?;
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
        let mut names = vec![
            "README.md".to_owned(),
            "vectors.json".into(),
            "verify.ps1".into(),
        ];
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
        writeln!(
            hashes,
            "{:x}  reference-files.list",
            Sha256::digest(list.as_bytes())
        )
        .map_err(|e| e.to_string())?;
        fs::write(stage.join("reference-files.list"), list).map_err(|e| e.to_string())?;
        fs::write(stage.join("reference-files.sha256"), hashes).map_err(|e| e.to_string())?;
        fs::rename(&stage, output).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        eprintln!("参考包暂存目录保留供检查: {}", stage.display());
    }
    result
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(output) = args.next() else {
        eprintln!("Usage: package_host_reference <new-output-directory>");
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("Usage: package_host_reference <new-output-directory>");
        std::process::exit(2);
    }
    if let Err(error) = run(&PathBuf::from(output)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
