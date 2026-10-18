//! Process-local reuse of identical sealed HostBatch builds, with private consumers.
use super::{packages, tooling, workspace::Scratch};
use autosar_config_core::target::ExecutionSettings;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

type Tree = BTreeMap<String, Option<Vec<u8>>>;

struct Image {
    bytes: Vec<u8>,
    permissions: fs::Permissions,
}

struct Entry {
    tree: Tree,
    tools: ExecutionSettings,
    image: Arc<OnceLock<Image>>,
}

/// Only the unmodified production HostBatch entry point is eligible. Custom
/// controls and tests of the build/verification boundary keep their own builds.
pub(crate) fn host_batch(project: &Path, output: &Path) -> PathBuf {
    static BUILDS: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
    let tree = packages::package_tree(project);
    let tools = tooling::execution_settings();
    let image = {
        let mut builds = BUILDS.lock().unwrap();
        if let Some(entry) = builds
            .iter()
            .find(|entry| entry.tree == tree && entry.tools == tools)
        {
            Arc::clone(&entry.image)
        } else {
            let image = Arc::new(OnceLock::new());
            builds.push(Entry {
                tree: tree.clone(),
                tools,
                image: Arc::clone(&image),
            });
            image
        }
    };
    let image = image.get_or_init(|| {
        let scratch = Scratch::new();
        let snapshot = scratch.0.join("project");
        for (name, bytes) in &tree {
            let path = snapshot.join(name);
            if let Some(bytes) = bytes {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            } else {
                fs::create_dir_all(path).unwrap();
            }
        }
        let build = scratch.0.join("build");
        let result = tooling::run_public_command(
            &mut tooling::ecu_build_command(&snapshot, &build, "host-batch", None),
            &scratch.0,
            "shared-host-batch-build",
            Duration::from_secs(180),
        );
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let binary = tooling::native_binary(&build, "ecu_host_batch");
        Image {
            bytes: fs::read(&binary).unwrap(),
            permissions: fs::metadata(binary).unwrap().permissions(),
        }
    });
    fs::create_dir_all(output).unwrap();
    let binary = tooling::native_binary(output, "ecu_host_batch");
    fs::write(&binary, &image.bytes).unwrap();
    fs::set_permissions(&binary, image.permissions.clone()).unwrap();
    binary
}
