use super::*;

impl Workspace {
    pub(super) fn replace_managed(
        &mut self,
        frames: Vec<FrameView>,
        signals: Vec<SignalView>,
        diagnostic: Option<DiagnosticView>,
    ) -> Result<(), crate::message::LocalizedText> {
        let index = self
            .files
            .iter()
            .position(|f| self.is_managed_file(f))
            .ok_or_else(|| {
                crate::product_message!(
                    "backend.arxml.legacy_editor.managed_configuration_unavailable"
                )
            })?;
        let mut frames = frames;
        let mut signals = signals;
        frames.sort_by(|a, b| a.path.cmp(&b.path));
        signals.sort_by(|a, b| a.path.cmp(&b.path));
        let mut issues = validate_profile(&frames, &signals);
        if let Some(diagnostic) = &diagnostic {
            issues.extend(validate_diagnostic(diagnostic, &frames, &signals));
        }
        if let Some(first) = issues.first() {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => first.code
                ),
                first.message.clone(),
            ]));
        }
        let previous = self.files[index].text.clone();
        self.files[index].text = render_profile(&self.name, &frames, &signals, diagnostic.as_ref());
        if let Err(error) = self.refresh() {
            self.files[index].text = previous;
            let _ = self.refresh();
            return Err(error.into());
        }
        // A new Rx frame has no ComSignal/ComTimeout until its first signal is added.
        // Keep the requested timeout in the editing model; validation/save still reject
        // the incomplete on-disk profile if the user stops before adding that signal.
        for frame in &mut self.frames {
            if matches!(frame.direction, Direction::Rx)
                && !signals.iter().any(|signal| signal.frame_path == frame.path)
            {
                frame.timeout_ms = frames
                    .iter()
                    .find(|source| source.path == frame.path)
                    .and_then(|source| source.timeout_ms);
            }
        }
        self.issues.retain(|issue| {
            !(issue.code == "RX_TIMEOUT"
                && frames.iter().any(|frame| {
                    matches!(frame.direction, Direction::Rx)
                        && issue.path.as_deref() == Some(frame.path.as_str())
                        && !signals.iter().any(|signal| signal.frame_path == frame.path)
                }))
        });
        if let Some(issue) = self
            .issues
            .iter()
            .find(|i| matches!(i.severity, Severity::Error))
        {
            let error = crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]);
            self.files[index].text = previous;
            self.refresh()?;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn add_frame(
        &mut self,
        name: String,
        id: u32,
        dlc: u8,
        direction: Direction,
        period_ms: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        if !valid_name(&name) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.frame_name_invalid"
            ));
        }
        let path = format!("/{}/Pdu_{}", self.name, name);
        if self.frames.iter().any(|f| f.path == path) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.frame_name_duplicate"
            ));
        }
        let mut frames = self.frames.clone();
        frames.push(FrameView {
            path,
            name,
            id,
            dlc,
            direction,
            period_ms,
            timeout_ms,
        });
        self.replace_managed(frames, self.signals.clone(), self.diagnostic.clone())?;
        Ok(self.view())
    }

    pub fn add_signal(
        &mut self,
        frame_path: String,
        name: String,
        start_bit: u8,
        length: u8,
        initial_value: u32,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        if !valid_name(&name) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.signal_name_invalid"
            ));
        }
        if !self.frames.iter().any(|f| f.path == frame_path) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.associated_frame_missing"
            ));
        }
        let path = format!("/{}/ComCfg/ComConfig/{}", self.name, name);
        if self.signals.iter().any(|s| s.path == path) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.signal_name_duplicate"
            ));
        }
        let mut signals = self.signals.clone();
        signals.push(SignalView {
            path,
            name,
            frame_path,
            start_bit,
            length,
            initial_value,
        });
        self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        Ok(self.view())
    }
    pub fn configure_diagnostic(
        &mut self,
        settings: DiagnosticSettings,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        let diagnostic = DiagnosticView {
            path: format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/Did", self.name),
            request_id: settings.request_id,
            response_id: settings.response_id,
            s3_ms: settings.s3_ms,
            n_as_ms: settings.n_as_ms.unwrap_or(settings.n_bs_ms),
            n_bs_ms: settings.n_bs_ms,
            n_cr_ms: settings.n_cr_ms,
            did: settings.did,
            signal_paths: settings.signal_paths,
            write_enabled: settings.write_enabled,
            reset_routine_id: settings.reset_routine_id,
            security_enabled: settings.security_enabled,
            dtc: self
                .diagnostic
                .as_ref()
                .and_then(|existing| existing.dtc.clone()),
        };
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }
    pub fn configure_dtc(
        &mut self,
        code: u32,
        monitor_frame_path: String,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        let mut diagnostic = self.diagnostic.clone().ok_or_else(|| {
            crate::product_message!("backend.arxml.legacy_editor.dtc_requires_diagnostic_services")
        })?;
        diagnostic.dtc = Some(DtcView {
            path: format!("/{}/DemCfg/DemConfigSet/DTC", self.name),
            code,
            monitor_frame_path,
        });
        if let Some(issue) = validate_diagnostic(&diagnostic, &self.frames, &self.signals).first() {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]));
        }
        self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
        Ok(self.view())
    }

    pub fn clear_dtc(&mut self) -> Result<WorkspaceView, crate::message::LocalizedText> {
        let mut diagnostic = self.diagnostic.clone().ok_or_else(|| {
            crate::product_message!("backend.arxml.legacy_editor.diagnostic_configuration_missing")
        })?;
        if diagnostic.dtc.is_none() {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.removable_dtc_missing"
            ));
        }
        if diagnostic.security_enabled && !diagnostic.write_enabled {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.dtc_removal_requires_security_disabled"
            ));
        }
        diagnostic.dtc = None;
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), Some(diagnostic))?;
            return Ok(self.view());
        }
        let targets: BTreeSet<_> = [
            format!("/{}/DemCfg", self.name),
            format!("/{}/NvMCfg", self.name),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ClearDiagnosticInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ReadDTCInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsd/Services/ControlDTCSetting",
                self.name
            ),
            format!("/{}/DcmCfg/DcmConfigSet/DcmDsp/ClearDTC", self.name),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsp/ReadDTCInformation",
                self.name
            ),
            format!(
                "/{}/DcmCfg/DcmConfigSet/DcmDsp/ControlDTCSetting",
                self.name
            ),
        ]
        .into_iter()
        .collect();
        let expected = render_profile(
            &self.name,
            &self.frames,
            &self.signals,
            self.diagnostic.as_ref(),
        );
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc
            .descendants()
            .filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && targets.contains(&path_of(*n))
            })
            .map(|n| (path_of(n), structural_node(n)))
            .collect();
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        let row_path = format!("/{}/DcmCfg/DcmConfigSet/DcmDsl/Protocol/UdsCan", self.name);
        let client_path = format!("/{}/DemCfg/DemGeneral/DcmClient", self.name);
        let client_definition = "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow/DcmDemClientRef";
        let mut removed_client_ref: Option<(usize, std::ops::Range<usize>)> = None;
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for row in doc.descendants().filter(|node| {
                node.is_element()
                    && node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && path_of(*node) == row_path
            }) {
                if definition(row).as_deref()
                    != Some(
                        "/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsl/DcmDslProtocol/DcmDslProtocolRow",
                    )
                {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_protocol_definition_mismatch",
                        "path" => row_path
                    ));
                }
                let mut references = row
                    .children()
                    .filter(|node| {
                        node.is_element() && node.tag_name().name() == "REFERENCE-VALUES"
                    })
                    .flat_map(|group| group.children().filter(|node| node.is_element()))
                    .filter(|node| definition(*node).as_deref() == Some(client_definition));
                let link = references.next().ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_client_reference_missing",
                        "path" => row_path
                    )
                })?;
                if references.next().is_some() || removed_client_ref.is_some() {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_client_reference_duplicate",
                        "path" => row_path
                    ));
                }
                let target = link
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "VALUE-REF")
                    .ok_or_else(|| {
                        crate::product_message!(
                            "backend.arxml.legacy_editor.dtc_client_reference_target_missing",
                            "path" => row_path
                        )
                    })?;
                if link.tag_name().name() != "ECUC-REFERENCE-VALUE"
                    || target.attribute("DEST") != Some("ECUC-CONTAINER-VALUE")
                    || target.text() != Some(client_path.as_str())
                {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_client_reference_not_owned",
                        "path" => row_path
                    ));
                }
                let range = link.range();
                patches[index].push(Patch {
                    range: range.clone(),
                    value: String::new(),
                });
                removed_client_ref = Some((index, range));
            }
            for node in doc.descendants().filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && targets.contains(&path_of(*n))
            }) {
                let path = path_of(node);
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_node_not_owned",
                        "path" => path
                    ));
                }
                if !removed.insert(path.clone()) {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_element_duplicate",
                        "path" => path
                    ));
                }
                owned_paths.extend(
                    node.descendants()
                        .filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some())
                        .map(path_of),
                );
                patches[index].push(Patch {
                    range: node.range(),
                    value: String::new(),
                });
            }
        }
        if removed_client_ref.is_none() {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.dtc_unique_client_reference_missing",
                "path" => row_path
            ));
        }
        if removed != targets {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.dtc_nodes_incomplete"
            ));
        }
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| {
                n.is_element()
                    && n.tag_name().name().ends_with("-REF")
                    && n.tag_name().name() != "DEFINITION-REF"
            }) {
                if let Some(target) = node.text()
                    && owned_paths.contains(target)
                    && !node
                        .ancestors()
                        .any(|ancestor| removed.contains(&path_of(ancestor)))
                {
                    if removed_client_ref
                        .as_ref()
                        .is_some_and(|(file_index, range)| {
                            *file_index == index
                                && range.start <= node.range().start
                                && node.range().end <= range.end
                        })
                    {
                        continue;
                    }
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.dtc_external_reference_blocks_removal",
                        "target" => target
                    ));
                }
            }
        }
        self.commit_patches(patches, |workspace| {
            workspace
                .diagnostic
                .as_ref()
                .is_some_and(|d| d.dtc.is_none())
        })?;
        Ok(self.view())
    }

    pub fn clear_diagnostic(&mut self) -> Result<WorkspaceView, crate::message::LocalizedText> {
        if self.diagnostic.is_none() {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.removable_diagnostic_configuration_missing"
            ));
        }
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), self.signals.clone(), None)?;
            return Ok(self.view());
        }
        let mut patches: Vec<Vec<Patch>> = (0..self.files.len()).map(|_| Vec::new()).collect();
        let mut removed = BTreeSet::new();
        let mut owned_paths = BTreeSet::new();
        let mut targets: BTreeSet<_> = [
            format!("/{}/DcmCfg", self.name),
            format!("/{}/CanTpCfg", self.name),
            format!("/{}/NPdu_DiagRequest", self.name),
            format!("/{}/NPdu_DiagResponse", self.name),
            format!("/{}/DcmPdu_DiagRequest", self.name),
            format!("/{}/DcmPdu_DiagResponse", self.name),
        ]
        .into_iter()
        .collect();
        for name in [
            "NPdu_DiagRequest",
            "NPdu_DiagResponse",
            "DcmPdu_DiagRequest",
            "DcmPdu_DiagResponse",
        ] {
            targets.insert(format!("/{}/EcuCCfg/EcucConfigSet/Pdus/{name}", self.name));
        }
        if self.diagnostic.as_ref().is_some_and(|d| d.dtc.is_some()) {
            targets.insert(format!("/{}/DemCfg", self.name));
            targets.insert(format!("/{}/NvMCfg", self.name));
        }
        let canif_targets: BTreeSet<_> = [
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagRequest", self.name),
            format!("/{}/CanIfCfg/CanIfInitCfg/Can_DiagResponse", self.name),
        ]
        .into_iter()
        .collect();
        let expected = render_profile(
            &self.name,
            &self.frames,
            &self.signals,
            self.diagnostic.as_ref(),
        );
        let expected_doc = Document::parse(&expected).map_err(|e| e.to_string())?;
        let expected_nodes: BTreeMap<_, _> = expected_doc
            .descendants()
            .filter(|n| {
                n.is_element()
                    && child_text(*n, "SHORT-NAME").is_some()
                    && (targets.contains(&path_of(*n)) || canif_targets.contains(&path_of(*n)))
            })
            .map(|n| (path_of(n), structural_node(n)))
            .collect();
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                if (matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES"
                        | "ECUC-CONTAINER-VALUE"
                        | "N-PDU"
                        | "DCM-I-PDU"
                ) && targets.contains(&path))
                    || (node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                        && canif_targets.contains(&path))
                {
                    owned_paths.extend(
                        node.descendants()
                            .filter(|n| n.is_element() && child_text(*n, "SHORT-NAME").is_some())
                            .map(path_of),
                    );
                }
            }
        }
        let mut updated_pdu_length_type = false;
        for (index, file) in self.files.iter().enumerate() {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants().filter(|n| n.is_element()) {
                let path = path_of(node);
                if node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                    && definition(node).as_deref()
                        == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection")
                    && path == format!("/{}/EcuCCfg/EcucConfigSet/Pdus", self.name)
                {
                    if updated_pdu_length_type {
                        return Err(crate::product_message!(
                            "backend.arxml.legacy_editor.global_pdu_collection_duplicate"
                        ));
                    }
                    patch_param(
                        node,
                        "PduLengthTypeEnum",
                        "UINT8".into(),
                        &mut patches[index],
                    )?;
                    updated_pdu_length_type = true;
                }
                let owned = (matches!(
                    node.tag_name().name(),
                    "ECUC-MODULE-CONFIGURATION-VALUES"
                        | "ECUC-CONTAINER-VALUE"
                        | "N-PDU"
                        | "DCM-I-PDU"
                ) && targets.contains(&path))
                    || (node.tag_name().name() == "ECUC-CONTAINER-VALUE"
                        && canif_targets.contains(&path));
                if !owned {
                    continue;
                }
                if expected_nodes.get(&path) != Some(&structural_node(node)) {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.diagnostic_node_not_owned",
                        "path" => path
                    ));
                }
                if !removed.insert(path.clone()) {
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.diagnostic_element_duplicate",
                        "path" => path
                    ));
                }
                patches[index].push(Patch {
                    range: node.range(),
                    value: String::new(),
                });
            }
        }
        if !updated_pdu_length_type || removed.len() != targets.len() + canif_targets.len() {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.diagnostic_nodes_incomplete"
            ));
        }
        for file in &self.files {
            let doc = Document::parse(&file.text).map_err(|e| e.to_string())?;
            for node in doc.descendants() {
                let target = if node.is_text() || node.is_comment() {
                    node.text().and_then(|text| {
                        owned_paths.iter().find(|path| text.contains(path.as_str()))
                    })
                } else if node.is_element() {
                    node.attributes().find_map(|attribute| {
                        owned_paths
                            .iter()
                            .find(|path| attribute.value().contains(path.as_str()))
                    })
                } else {
                    None
                };
                if let Some(target) = target {
                    if node
                        .ancestors()
                        .any(|ancestor| removed.contains(&path_of(ancestor)))
                    {
                        continue;
                    }
                    return Err(crate::product_message!(
                        "backend.arxml.legacy_editor.diagnostic_external_content_blocks_removal",
                        "file" => file.path.display(),
                        "path" => path_of(node.parent_element().unwrap_or(node)),
                        "target" => target
                    ));
                }
            }
        }
        self.commit_patches(patches, |workspace| workspace.diagnostic.is_none())?;
        Ok(self.view())
    }

    pub fn update_frame(
        &mut self,
        path: &str,
        changes: Value,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        let old = self
            .frames
            .iter()
            .find(|f| f.path == path)
            .ok_or_else(|| crate::product_message!("backend.arxml.legacy_editor.frame_missing"))?
            .clone();
        let mut frames = self.frames.clone();
        let frame = frames
            .iter_mut()
            .find(|f| f.path == path)
            .ok_or_else(|| crate::product_message!("backend.arxml.legacy_editor.frame_missing"))?;
        if changes
            .get("name")
            .is_some_and(|v| v.as_str() != Some(&frame.name))
        {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.frame_rename_unsupported"
            ));
        }
        if let Some(v) = changes.get("id") {
            frame.id = v
                .as_u64()
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.can_identifier_requires_unsigned_integer"
                    )
                })?
                .try_into()
                .map_err(|_| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.can_identifier_out_of_range"
                    )
                })?;
        }
        if let Some(v) = changes.get("dlc") {
            frame.dlc = v
                .as_u64()
                .ok_or_else(|| {
                    crate::product_message!("backend.arxml.legacy_editor.dlc_requires_integer")
                })?
                .try_into()
                .map_err(|_| {
                    crate::product_message!("backend.arxml.legacy_editor.dlc_out_of_range")
                })?;
        }
        if let Some(v) = changes.get("periodMs") {
            frame.period_ms = if v.is_null() {
                None
            } else {
                Some(
                    v.as_u64()
                        .ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.legacy_editor.period_requires_integer"
                            )
                        })?
                        .try_into()
                        .map_err(|_| {
                            crate::product_message!(
                                "backend.arxml.legacy_editor.period_out_of_range"
                            )
                        })?,
                )
            };
        }
        if let Some(v) = changes.get("timeoutMs") {
            frame.timeout_ms = if v.is_null() {
                None
            } else {
                Some(
                    v.as_u64()
                        .ok_or_else(|| {
                            crate::product_message!(
                                "backend.arxml.legacy_editor.timeout_requires_integer"
                            )
                        })?
                        .try_into()
                        .map_err(|_| {
                            crate::product_message!(
                                "backend.arxml.legacy_editor.timeout_out_of_range"
                            )
                        })?,
                )
            };
        }
        if let Some(v) = changes.get("direction") {
            let current = match frame.direction {
                Direction::Tx => "tx",
                Direction::Rx => "rx",
            };
            if v.as_str() != Some(current) {
                return Err(crate::product_message!(
                    "backend.arxml.legacy_editor.frame_direction_change_unsupported"
                ));
            }
        }
        if let Some(issue) = validate_profile(&frames, &self.signals).first() {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]));
        }
        if let Some(diagnostic) = &self.diagnostic
            && let Some(issue) = validate_diagnostic(diagnostic, &frames, &self.signals).first()
        {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]));
        }
        let updated = frames.iter().find(|f| f.path == path).unwrap().clone();
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(frames, self.signals.clone(), self.diagnostic.clone())?;
        } else {
            self.patch_imported_frame(&old, &updated)?;
        }
        Ok(self.view())
    }

    pub fn update_signal(
        &mut self,
        path: &str,
        changes: Value,
    ) -> Result<WorkspaceView, crate::message::LocalizedText> {
        let old = self
            .signals
            .iter()
            .find(|s| s.path == path)
            .ok_or_else(|| crate::product_message!("backend.arxml.legacy_editor.signal_missing"))?
            .clone();
        let mut signals = self.signals.clone();
        let signal = signals
            .iter_mut()
            .find(|s| s.path == path)
            .ok_or_else(|| crate::product_message!("backend.arxml.legacy_editor.signal_missing"))?;
        if changes
            .get("name")
            .is_some_and(|v| v.as_str() != Some(&signal.name))
            || changes
                .get("framePath")
                .is_some_and(|v| v.as_str() != Some(&signal.frame_path))
        {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.signal_rename_or_move_unsupported"
            ));
        }
        if let Some(v) = changes.get("startBit") {
            signal.start_bit = v
                .as_u64()
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.start_bit_requires_integer"
                    )
                })?
                .try_into()
                .map_err(|_| {
                    crate::product_message!("backend.arxml.legacy_editor.start_bit_out_of_range")
                })?;
        }
        if let Some(v) = changes.get("length") {
            signal.length = v
                .as_u64()
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.bit_length_requires_integer"
                    )
                })?
                .try_into()
                .map_err(|_| {
                    crate::product_message!("backend.arxml.legacy_editor.bit_length_out_of_range")
                })?;
        }
        if let Some(v) = changes.get("initialValue") {
            signal.initial_value = v
                .as_u64()
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.initial_value_requires_integer"
                    )
                })?
                .try_into()
                .map_err(|_| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.initial_value_out_of_range"
                    )
                })?;
        }
        if let Some(issue) = validate_profile(&self.frames, &signals).first() {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]));
        }
        if let Some(diagnostic) = &self.diagnostic
            && let Some(issue) = validate_diagnostic(diagnostic, &self.frames, &signals).first()
        {
            return Err(crate::message::LocalizedText::messages([
                crate::product_message!(
                    "backend.arxml.legacy_editor.validation_issue_code",
                    "code" => issue.code
                ),
                issue.message.clone(),
            ]));
        }
        let updated = signals.iter().find(|s| s.path == path).unwrap().clone();
        if self.files.iter().any(|file| self.is_managed_file(file)) {
            self.replace_managed(self.frames.clone(), signals, self.diagnostic.clone())?;
        } else {
            self.patch_imported_signal(&old, &updated)?;
        }
        Ok(self.view())
    }

    pub(crate) fn handoff_sources(
        &mut self,
    ) -> Result<Vec<HandoffSource>, crate::message::LocalizedText> {
        if self.files.iter().any(|file| file.text != file.saved) {
            return Err(crate::product_message!(
                "backend.arxml.legacy_editor.handoff_requires_saved_sources"
            ));
        }
        self.ensure_sources_current()?;
        self.checked_profile()?;
        let mut sources = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let original_name = file
                .original_name
                .as_deref()
                .or_else(|| file.path.file_name().and_then(|name| name.to_str()))
                .ok_or_else(|| {
                    crate::product_message!(
                        "backend.arxml.legacy_editor.source_filename_not_utf_eight"
                    )
                })?
                .to_owned();
            let doc = Document::parse(&file.saved).map_err(|e| e.to_string())?;
            let package_roots = doc
                .descendants()
                .filter(|node| {
                    node.is_element()
                        && node.tag_name().name() == "AR-PACKAGE"
                        && node.parent_element().is_some_and(|parent| {
                            parent.tag_name().name() == "AR-PACKAGES"
                                && parent.parent_element().is_some_and(|grandparent| {
                                    grandparent.tag_name().name() == "AUTOSAR"
                                })
                        })
                })
                .filter_map(|node| child_text(node, "SHORT-NAME"))
                .collect();
            sources.push(HandoffSource {
                original_name,
                package_roots,
                contents: file.saved.as_bytes().to_vec(),
            });
        }
        sources.sort_by(|left, right| {
            left.contents
                .cmp(&right.contents)
                .then_with(|| left.original_name.cmp(&right.original_name))
        });
        Ok(sources)
    }
}
