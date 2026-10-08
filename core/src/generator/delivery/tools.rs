use super::{HandoffMetadata, digest, insert_file};
use crate::prepared::PreparedFile;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeMap;

const HELPER: &[u8] = include_bytes!("../../../../tools/python/src/ecu_tools/workbench_v2.py");
const INVENTORY: &[u8] =
    include_bytes!("../../../../tools/python/src/ecu_tools/workbench-v2-assets.json");
include!(concat!(env!("OUT_DIR"), "/delivery_helper.rs"));
const WRAPPER: &str = "tools/ecu-tool.py";
const LEGACY_ENTRY: &str = "tools/legacy-ecu-tool.py";
const HELPER_PATH: &str = "tools/workbench_v2.py";
const INVENTORY_PATH: &str = "tools/workbench-v2-assets.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolInventory {
    format: String,
    version: String,
    files: Vec<ToolIdentity>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolIdentity {
    path: String,
    sha256: String,
}

pub(crate) fn add<'a>(
    files: &mut BTreeMap<String, PreparedFile<'a>>,
    metadata: &HandoffMetadata,
) -> Result<(), crate::LocalizedText> {
    let inventory: ToolInventory = serde_json::from_slice(INVENTORY)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    if inventory.format != "autosar-workbench-build-tool-inventory-v1"
        || inventory.version != "1.0.0"
        || inventory.files.len() != 1
        || inventory.files[0].path != "workbench_v2.py"
        || inventory.files[0].sha256 != TRUSTED_HELPER_SHA256
        || digest(HELPER) != TRUSTED_HELPER_SHA256
    {
        return Err(crate::product_message!(
            "backend.delivery.tool_inventory_corrupt"
        ));
    }
    let original = files
        .get(WRAPPER)
        .ok_or_else(|| crate::product_message!("backend.delivery.standalone_tool_missing"))?;
    let source = original
        .source
        .ok_or_else(|| crate::product_message!("backend.delivery.standalone_producer_missing"))?;
    if source.bytes != original.bytes.as_ref() {
        return Err(crate::product_message!(
            "backend.delivery.standalone_tool_modified"
        ));
    }
    let original_bytes = original.bytes.clone();
    insert_file(files, LEGACY_ENTRY.into(), original_bytes)?;
    insert_file(files, HELPER_PATH.into(), Cow::Borrowed(HELPER))?;
    insert_file(files, INVENTORY_PATH.into(), Cow::Borrowed(INVENTORY))?;
    let policy = super::ownership::ledger(files, metadata)?
        .files
        .into_iter()
        .map(|entry| {
            let mut value =
                serde_json::json!({"owner": entry.owner, "producerId": entry.producer_id});
            if let Some(snapshot) = entry.snapshot_of {
                value["snapshotOf"] = serde_json::Value::String(snapshot);
            }
            (entry.path, value)
        })
        .collect::<BTreeMap<_, _>>();
    let payload: BTreeMap<_, _> = files
        .iter()
        .filter(|(path, _)| path.as_str() != WRAPPER)
        .map(|(path, file)| (path.as_str(), digest(&file.bytes)))
        .collect();
    let tool_bytes: BTreeMap<_, _> = files
        .iter()
        .filter(|(path, _)| path.starts_with("tools/") && path.as_str() != WRAPPER)
        .map(|(path, file)| {
            (
                path.as_str(),
                serde_json::json!({"sha256": digest(&file.bytes), "size": file.bytes.len()}),
            )
        })
        .collect();
    let metadata_json = serde_json::to_string(metadata)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let policy_json = serde_json::to_string(&policy)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let tool_json = serde_json::to_string(&tool_bytes)
        .map_err(|error| crate::LocalizedText::from(error.to_string()))?;
    let wrapper = format!(
        r#""""Native v2 guard before the unchanged sealed standalone engineering tool."""
import argparse
import hashlib
import json
import os
import runpy
import stat
import sys
from pathlib import Path

sys.dont_write_bytecode = True
os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / 'tools'))
EXPECTED_METADATA = json.loads({metadata_json:?})
EXPECTED_POLICY = json.loads({policy_json:?})
EXPECTED_PAYLOAD = json.loads({payload_json:?})
TRUSTED_TOOLS = json.loads({tool_json:?})

def run():
    for name, identity in TRUSTED_TOOLS.items():
        path = ROOT / name
        for part in (path, *path.parents):
            info = part.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
                raise ValueError('Linked native tool path is refused: ' + name)
        if not path.is_file() or path.stat().st_size != identity['size']:
            raise ValueError('Native tool size/type differs: ' + name)
        with path.open('rb') as source:
            actual = hashlib.sha256(source.read(identity['size'] + 1)).hexdigest()
        if actual != identity['sha256']:
            raise ValueError('Native tool bytes differ from compiled producer: ' + name)
    original = runpy.run_path(str(ROOT / 'tools/legacy-ecu-tool.py'))['main']
    if '-h' in sys.argv or '--help' in sys.argv:
        return original()
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--project', action='append', type=Path)
    args, _ = parser.parse_known_args()
    if args.project is None or len(args.project) != 1:
        raise ValueError('Exactly one explicit native --project is required')
    if args.project[0].resolve(strict=True) != ROOT:
        raise ValueError('Use the engineering tool belonging to the selected immutable source package')
    from workbench_v2 import validate
    validate(args.project[0], EXPECTED_METADATA, EXPECTED_POLICY, EXPECTED_PAYLOAD)
    result = original()
    validate(args.project[0], EXPECTED_METADATA, EXPECTED_POLICY, EXPECTED_PAYLOAD)
    return result

if __name__ == '__main__':
    try:
        result = run()
    except (OSError, ValueError, TypeError, KeyError, RuntimeError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        result = 1
    raise SystemExit(result)
"#
    );
    let entry = files.get_mut(WRAPPER).unwrap();
    entry.bytes = Cow::Owned(wrapper.into_bytes());
    entry.source = None;
    Ok(())
}
