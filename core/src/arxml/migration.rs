use super::*;

struct PduReference {
    system_path: String,
    system_dest: String,
    global_path: String,
}

fn closing_gap(node: Node<'_, '_>, text: &str, closing: &str) -> Option<usize> {
    let range = node.range();
    text[range.clone()].ends_with(closing).then_some(range.end - closing.len())
}

impl Workspace {
    pub(super) fn stage_legacy_pdus(&mut self) -> Result<(), Issue> {
        let module_path = format!("/{}/ComCfg", self.name);
        let fallback = || migration_issue(&self.files[0], module_path.clone(),
            "旧版 PDU 配置不是可完整识别的工具所有形状；原文件保持只读");
        if self.frames.is_empty() || self.signals.is_empty() ||
            self.issues.iter().any(|issue| matches!(issue.severity, Severity::Error) &&
                issue.code != "PDU_LEGACY_READ_ONLY" &&
                !(issue.code == "PDU_UNSUPPORTED" && issue.path.as_deref() == Some(module_path.as_str()))) {
            return Err(fallback());
        }
        let expected = render_profile(&self.name, &self.frames, &self.signals, self.diagnostic.as_ref());
        let expected_doc = Document::parse(&expected).map_err(|_| fallback())?;
        let config_path = format!("{module_path}/ComConfig");
        let canif_path = format!("/{}/CanIfCfg", self.name);
        let ecuc_path = format!("/{}/EcuCCfg", self.name);
        let project_root = format!("/{}", self.name);
        let project_prefix = format!("{project_root}/");
        let global_prefix = format!("{ecuc_path}/EcucConfigSet/Pdus/");
        let expected_node = |path: &str, tag: &str| expected_doc.descendants().find(|node|
            node.is_element() && node.tag_name().name() == tag && path_of(*node) == path);
        let general = expected_node(&format!("{module_path}/ComGeneral"), "ECUC-CONTAINER-VALUE")
            .ok_or_else(fallback)?;
        let ecuc = expected_node(&ecuc_path, "ECUC-MODULE-CONFIGURATION-VALUES")
            .ok_or_else(fallback)?;
        let mut global_bindings = BTreeMap::new();
        for node in expected_doc.descendants().filter(|node| node.is_element() &&
            node.tag_name().name() == "ECUC-CONTAINER-VALUE" &&
            definition(*node).as_deref() == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")) {
            let binding = tool_global_pdu(node, &self.files[0]).map_err(|_| fallback())?;
            if global_bindings.insert(path_of(node), binding.system_path).is_some() { return Err(fallback()); }
        }
        if global_bindings.len() != self.frames.len() + if self.diagnostic.is_some() { 4 } else { 0 } {
            return Err(fallback());
        }
        let system_paths: BTreeSet<_> = global_bindings.values().map(String::as_str).collect();
        let system_kinds: BTreeMap<_, _> = expected_doc.descendants().filter(|node| node.is_element() &&
            matches!(node.tag_name().name(), "I-SIGNAL-I-PDU" | "N-PDU" | "DCM-I-PDU"))
            .map(|node| (path_of(node), node.tag_name().name().to_owned())).collect();
        let mut expected_refs = BTreeMap::new();
        for node in expected_doc.descendants().filter(|node| node.is_element() && node.tag_name().name() == "ECUC-REFERENCE-VALUE") {
            let Some(value) = node.children().find(|child| child.is_element() && child.tag_name().name() == "VALUE-REF") else { continue; };
            let Some(global_path) = value.text().filter(|target| target.starts_with(&global_prefix)) else { continue; };
            let Some(system_path) = global_bindings.get(global_path) else { return Err(fallback()); };
            let Some(system_dest) = system_kinds.get(system_path) else { return Err(fallback()); };
            let Some(owner) = node.parent_element().and_then(|group| group.parent_element()) else { return Err(fallback()); };
            let Some(definition) = definition(node) else { return Err(fallback()); };
            let reference = PduReference { system_path: system_path.clone(), system_dest: system_dest.clone(), global_path: global_path.into() };
            if expected_refs.insert((path_of(owner), definition), reference).is_some() { return Err(fallback()); }
        }
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut seen_refs = BTreeSet::new();
        let mut patched_values = BTreeSet::new();
        let mut com_file = None;
        let mut canif_file = None;
        let mut seen_npdu = BTreeSet::new();
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|_| fallback())?;
            for node in doc.descendants().filter(|node| node.is_element()) {
                let tag = node.tag_name().name();
                if tag == "VARIATION-POINT" {
                    let path = path_of(node);
                    if path == project_root || path.starts_with(&project_prefix) ||
                        node.parent_element().is_some_and(|parent| parent.tag_name().name() == "AUTOSAR") {
                        return Err(migration_issue(file, path, "旧版 PDU 配置存在未解析变体"));
                    }
                }
                if tag == "ECUC-MODULE-CONFIGURATION-VALUES" {
                    let path = path_of(node);
                    if path == ecuc_path || definition(node).as_deref() == Some("/AUTOSAR/EcucDefs/EcuC") {
                        return Err(migration_issue(file, path, "已有 EcuC 模块；禁止修改用户所有的 ECUC"));
                    }
                    if path == module_path {
                        if com_file.replace(index).is_some() || !module_definition(node, "/AUTOSAR/EcucDefs/Com") {
                            return Err(migration_issue(file, path, "Com 模块不唯一或父定义不受支持"));
                        }
                        let roots = child_containers(node);
                        if roots.len() != 1 || path_of(roots[0]) != config_path ||
                            definition(roots[0]).as_deref() != Some("/AUTOSAR/EcucDefs/Com/ComConfig") {
                            return Err(migration_issue(file, path, "旧 Com 只允许唯一 ComConfig 且无 ComGeneral"));
                        }
                        let containers = node.children().find(|child| child.is_element() && child.tag_name().name() == "CONTAINERS")
                            .ok_or_else(|| migration_issue(file, path.clone(), "Com 容器缺失"))?;
                        let position = closing_gap(containers, &file.text, "</CONTAINERS>")
                            .ok_or_else(|| migration_issue(file, path.clone(), "Com 容器不能安全增补"))?;
                        patches[index].push(Patch { range: position..position, value: expected[general.range()].into() });
                        patches[index].push(Patch { range: node.range().end..node.range().end,
                            value: expected[ecuc.range()].into() });
                    }
                    if path == canif_path {
                        if canif_file.replace(index).is_some() || !module_definition(node, "/AUTOSAR/EcucDefs/CanIf") {
                            return Err(migration_issue(file, path, "CanIf 模块不唯一或父定义不受支持"));
                        }
                    }
                }
                if tag == "ECUC-REFERENCE-VALUE" {
                    let Some(owner) = node.parent_element().and_then(|group| group.parent_element()) else { return Err(fallback()); };
                    let key = (path_of(owner), definition(node).unwrap_or_default());
                    if let Some(target) = expected_refs.get(&key) {
                        if !seen_refs.insert(key.clone()) { return Err(migration_issue(file, key.0, "旧 PDU 引用重复")); }
                        let patch = legacy_reference_patch(node, file, &key.1, &target.system_path,
                            &target.system_dest, &target.global_path)?;
                        patched_values.insert((index, patch.range.start));
                        patches[index].push(patch);
                    }
                }
                if tag == "ECUC-CONTAINER-VALUE" &&
                    definition(node).as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu") {
                    let path = path_of(node);
                    if !tool_com_parent(node, &module_path, &config_path) { return Err(migration_issue(file, path, "ComIPdu 所有者不受支持")); }
                    let Some(reference) = expected_node(&path, "ECUC-CONTAINER-VALUE") else { return Err(migration_issue(file, path, "旧 ComIPdu 不能匹配生成模型")); };
                    let old_params = node.children().find(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
                        .ok_or_else(|| migration_issue(file, path.clone(), "ComIPdu 缺少旧参数组"))?;
                    if old_params.children().filter(|child| child.is_element()).count() != 2 {
                        return Err(migration_issue(file, path, "ComIPdu 必需字段部分迁移或含未知参数"));
                    }
                    let mut additions = String::new();
                    for parameter in reference.children().filter(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
                        .flat_map(|group| group.children().filter(|child| child.is_element())) {
                        if definition(parameter).as_deref().is_some_and(|name| name.ends_with("/ComIPduSignalProcessing") || name.ends_with("/ComIPduType")) {
                            additions.push_str(&expected[parameter.range()]);
                        }
                    }
                    let position = closing_gap(old_params, &file.text, "</PARAMETER-VALUES>")
                        .ok_or_else(|| migration_issue(file, path.clone(), "ComIPdu 参数组不能安全增补"))?;
                    patches[index].push(Patch { range: position..position, value: additions });
                    if let Some(expected_tx) = reference.descendants().find(|child| child.is_element() &&
                        definition(*child).as_deref() == Some("/AUTOSAR/EcucDefs/Com/ComConfig/ComIPdu/ComTxIPdu")) {
                        let tx_path = path_of(expected_tx);
                        let old_tx = node.descendants().find(|child| child.is_element() && path_of(*child) == tx_path)
                            .ok_or_else(|| migration_issue(file, path.clone(), "旧 ComTxIPdu 缺失"))?;
                        if old_tx.children().any(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES") {
                            return Err(migration_issue(file, tx_path, "ComTxIPdu 含未知或部分迁移参数"));
                        }
                        let expected_params = expected_tx.children().find(|child| child.is_element() && child.tag_name().name() == "PARAMETER-VALUES")
                            .ok_or_else(fallback)?;
                        let subcontainers = old_tx.children().find(|child| child.is_element() && child.tag_name().name() == "SUB-CONTAINERS")
                            .ok_or_else(|| migration_issue(file, path.clone(), "ComTxIPdu 子容器缺失"))?;
                        patches[index].push(Patch { range: subcontainers.range().start..subcontainers.range().start,
                            value: expected[expected_params.range()].into() });
                    }
                }
                if tag == "N-PDU" && self.diagnostic.is_some() &&
                    expected_node(&path_of(node), "N-PDU").is_some() {
                    let path = path_of(node);
                    if !seen_npdu.insert(path.clone()) || node.attributes().len() != 0 || node.descendants().any(|child| child.is_comment()) ||
                        node.children().filter(|child| child.is_element()).map(|child| child.tag_name().name()).collect::<Vec<_>>()
                            != ["SHORT-NAME", "HAS-DYNAMIC-LENGTH", "LENGTH"] ||
                        child_text(node, "HAS-DYNAMIC-LENGTH").as_deref() != Some("true") ||
                        child_text(node, "LENGTH").as_deref() != Some("8") {
                        return Err(migration_issue(file, path, "N-PDU 含未知长度字段或变体"));
                    }
                    let dynamic = node.children().find(|child| child.is_element() && child.tag_name().name() == "HAS-DYNAMIC-LENGTH")
                        .ok_or_else(fallback)?;
                    patches[index].push(Patch { range: dynamic.range(), value: String::new() });
                }
            }
            for value in doc.descendants().filter(|node| node.is_element() && node.tag_name().name() == "VALUE-REF") {
                if value.text().is_some_and(|target| system_paths.contains(target)) &&
                    !patched_values.contains(&(index, value.range().start)) {
                    return Err(migration_issue(file, path_of(value.parent_element().and_then(|group| group.parent_element()).unwrap_or(value)),
                        "外部 ECUC 引用依赖旧系统 PDU，拒绝迁移"));
                }
            }
        }
        if com_file.is_none() || canif_file.is_none() || seen_refs.len() != expected_refs.len() ||
            seen_npdu.len() != if self.diagnostic.is_some() { 2 } else { 0 } {
            return Err(fallback());
        }
        let mut preview: Vec<_> = self.files.iter().map(|file| file.text.clone()).collect();
        for (text, file_patches) in preview.iter_mut().zip(patches.iter_mut()) {
            apply_patches(text, file_patches).map_err(|_| fallback())?;
        }
        for owned_path in [&module_path, &canif_path] {
            let expected_module = expected_node(owned_path, "ECUC-MODULE-CONFIGURATION-VALUES").ok_or_else(fallback)?;
            let source_index = if owned_path == &module_path { com_file.unwrap() } else { canif_file.unwrap() };
            let doc = Document::parse(&preview[source_index]).map_err(|_| fallback())?;
            let actual = doc.descendants().find(|node| node.is_element() &&
                node.tag_name().name() == "ECUC-MODULE-CONFIGURATION-VALUES" && path_of(*node) == *owned_path)
                .ok_or_else(fallback)?;
            if structural_node(actual) != structural_node(expected_module) {
                return Err(migration_issue(&self.files[source_index], owned_path.clone(),
                    "旧版 Com/CanIf 含未知扩展、字段、引用或顺序；拒绝丢失用户内容"));
            }
        }
        let validation = schema::validate_files(&self.schema_zip, &self.files.iter().enumerate()
            .map(|(index, file)| (file.path.as_path(), preview[index].as_str())).collect::<Vec<_>>())
            .map_err(|_| fallback())?;
        if let Some(issue) = validation.first() {
            return Err(migration_issue(&self.files[0], module_path,
                format!("迁移后的 ARXML 不符合 XSD: {}", issue.message)));
        }
        let frame_count = self.frames.len();
        self.commit_patches(patches, |workspace| workspace.frames.len() == frame_count)
            .map_err(|error| migration_issue(&self.files[0], format!("/{}/ComCfg", self.name), error))?;
        Ok(())
    }
}
