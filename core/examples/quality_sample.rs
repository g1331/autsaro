use autosar_config_core::{Direction, Workspace, generator, schema};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("missing output root")?);
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut workspace = Workspace::create(
        &root.join("workspace"),
        "QualitySample",
        schema::schema_archive(&repository),
    )?;
    let frame = workspace
        .add_frame("Status".into(), 0x321, 2, Direction::Tx, Some(10), None)?
        .frames[0]
        .path
        .clone();
    workspace.add_signal(frame, "Counter".into(), 0, 8, 0)?;
    workspace.save()?;
    generator::generate(&mut workspace, &root.join("generated"))?;
    Ok(())
}
