use autosar_config_core::{Direction, Workspace, schema};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct Scratch(pub(super) PathBuf);
impl Scratch {
    pub(super) fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("autosar-config-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(super) fn archive() -> PathBuf {
    schema::schema_archive(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}
pub(super) fn create_pair(root: &Path) -> (Workspace, Workspace) {
    let zip = archive();
    let mut a = Workspace::create(&root.join("Alpha"), "Alpha", zip.clone()).unwrap();
    let tx = a
        .add_frame("Command".into(), 0x321, 2, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    a.add_signal(tx, "SendCount".into(), 3, 8, 5).unwrap();
    let rx = a
        .add_frame("Reply".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|f| f.name == "Reply")
        .unwrap()
        .path;
    a.add_signal(rx, "RecvStatus".into(), 0, 8, 0).unwrap();
    assert!(a.validate().unwrap().issues.is_empty());
    a.save().unwrap();
    let mut b = Workspace::create(&root.join("Beta"), "Beta", zip).unwrap();
    let rx = b
        .add_frame("Command".into(), 0x321, 2, Direction::Rx, None, Some(40))
        .unwrap()
        .frames[0]
        .path
        .clone();
    b.add_signal(rx, "RecvCount".into(), 3, 8, 0).unwrap();
    let tx = b
        .add_frame("Reply".into(), 0x456, 2, Direction::Tx, Some(20), None)
        .unwrap()
        .frames
        .into_iter()
        .find(|f| f.name == "Reply")
        .unwrap()
        .path;
    b.add_signal(tx, "SendStatus".into(), 0, 8, 7).unwrap();
    assert!(b.validate().unwrap().issues.is_empty());
    b.save().unwrap();
    (a, b)
}
