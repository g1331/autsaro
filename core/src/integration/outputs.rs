//! Output files of one ECU generation, each with exactly one producer.
//!
//! A path is registered once, either by a selected runtime asset delivered
//! verbatim or by the emitter that renders it from the validated plan. The
//! registry has no accessor for registered bytes, so an emitter cannot read
//! back or patch another producer's output; a second producer of a path is
//! rejected before anything is written.
use super::PlanDiagnostic;
use crate::resources::AssetEntry;
use crate::target::BuildTarget;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Delivered metadata of an ECU source project.
pub const INTEGRATION_PATH: &str = "meta/integration.json";
pub(crate) const VERIFICATION_INPUTS_PATH: &str = "meta/verification-inputs.json";

pub(super) const ARTI: &str = "arti";
pub(super) const DCM_MODES: &str = "dcm-modes";
pub(super) const OS: &str = "os";

pub(super) const MEMMAP: &str = "memmap";
pub(super) const RTE: &str = "rte";

pub(super) const ECU: &str = "ecu-config";

/// Trusted inputs reserved at their existing generated output paths.
const TEMPLATES: [(&str, &str, &str); 22] = [
    ("runtime/ecu/templates/Rte.h.in", RTE, "include/Rte.h"),
    ("runtime/os/include/Rte_Type.h", RTE, "include/Rte_Type.h"),
    ("runtime/ecu/include/Rte_Cfg.h", RTE, "include/Rte_Cfg.h"),
    (
        "runtime/ecu/include/Rte_DataHandleType.h",
        RTE,
        "include/Rte_DataHandleType.h",
    ),
    (
        "runtime/ecu/include/Ecu_Config.h",
        ECU,
        "include/Ecu_Config.h",
    ),
    (
        "runtime/ecu/templates/Ecu_TargetConfig.h.in",
        ECU,
        "include/Ecu_TargetConfig.h",
    ),
    (
        "runtime/ecu/src/Ecu_Execution.c",
        ECU,
        "src/Ecu_Execution.c",
    ),
    (
        "runtime/ecu/src/Ecu_Execution.h",
        ECU,
        "src/Ecu_Execution.h",
    ),
    (
        "runtime/ecu/src/Ecu_DetCallout.c",
        ECU,
        "src/Det_Callouts.c",
    ),
    (
        "runtime/ecu/include/Det_Externals.h",
        ECU,
        "include/Det_Externals.h",
    ),
    ("runtime/ecu/include/Rte_Main.h", RTE, "include/Rte_Main.h"),
    ("runtime/ecu/include/Rte_Com.h", RTE, "include/Rte_Com.h"),
    (
        "runtime/ecu/templates/SchM_Main.c.in",
        ECU,
        "src/SchM_Main.c",
    ),
    ("runtime/os/include/Os_Cfg.h", OS, "include/Os_Cfg.h"),
    (
        "runtime/ecu/templates/Ecu_Config.c.in",
        OS,
        "src/Ecu_Config.c",
    ),
    ("runtime/os/src/Os_Arti.c", ARTI, "src/Os_Arti.c"),
    (
        "runtime/bsw/include/Rte_Dcm_Type.h",
        DCM_MODES,
        "include/Rte_Dcm_Type.h",
    ),
    (
        "runtime/bsw/include/SchM_Dcm.h",
        DCM_MODES,
        "include/SchM_Dcm.h",
    ),
    // RTE-generated headers (SWS_Rte_07501, SWS_Rte_07120) fixed for the host target.
    (
        "runtime/bsw/include/SchM_BswM.h",
        RTE,
        "include/SchM_BswM.h",
    ),
    ("runtime/bsw/include/SchM_Can.h", RTE, "include/SchM_Can.h"),
    (
        "runtime/bsw/include/Rte_ComM_Type.h",
        RTE,
        "include/Rte_ComM_Type.h",
    ),
    (
        "runtime/os/include/Rte_Os_Type.h",
        RTE,
        "include/Rte_Os_Type.h",
    ),
];

/// The emitter and logical path of a template asset. Memory Mapping provides
/// every module's `{Mip}_MemMap.h` (SWS_MemMap_00032), not the module itself.
pub(crate) fn template_of(asset: &AssetEntry) -> Option<(&'static str, String)> {
    if let Some((_, producer, path)) = TEMPLATES
        .iter()
        .find(|(template, ..)| *template == asset.relative_path)
    {
        return Some((producer, (*path).into()));
    }
    let name = asset.relative_path.rsplit('/').next()?;
    (asset.relative_path.starts_with("runtime/") && name.ends_with("_MemMap.h"))
        .then(|| (MEMMAP, format!("include/{name}")))
}

/// Emitters name C outputs by the logical `include/` or `src/` path the plan's
/// symbol owners use; both are relative to the package's `generated/`.
fn delivered(path: String) -> String {
    if path.starts_with("include/") || path.starts_with("src/") {
        format!("generated/{path}")
    } else {
        path
    }
}

#[derive(Clone, Copy)]
enum Producer {
    Asset(&'static AssetEntry),
    Emitter(&'static str),
}

impl Producer {
    fn name(self) -> &'static str {
        match self {
            Self::Asset(asset) => asset.relative_path,
            Self::Emitter(name) => name,
        }
    }
}

/// Delivered path to bytes of one ECU generation.
pub(crate) type EcuFiles = BTreeMap<String, Cow<'static, [u8]>>;

pub(crate) struct EcuOutputs {
    files: BTreeMap<String, (Producer, Cow<'static, [u8]>)>,
    templates: BTreeMap<String, (&'static AssetEntry, &'static str)>,
}

impl EcuOutputs {
    /// Registers every asset selected for `target`, reserving template assets
    /// for their emitter.
    pub(super) fn new(target: BuildTarget) -> Result<Self, Vec<PlanDiagnostic>> {
        let mut outputs = Self {
            files: BTreeMap::new(),
            templates: BTreeMap::new(),
        };
        for asset in crate::resources::AssetInventory::embedded().selected(target, "ecu") {
            if let Some((producer, path)) = template_of(asset) {
                let path = delivered(path);
                if outputs.templates.contains_key(&path) {
                    return Err(PlanDiagnostic::output_conflict(&path, producer, producer));
                }
                outputs.templates.insert(path, (asset, producer));
            }
        }
        let assets = super::ecu::source_assets(target).map_err(super::ecu::reject)?;
        for (path, asset) in assets {
            outputs.register(path, Producer::Asset(asset), Cow::Borrowed(asset.bytes))?;
        }
        Ok(outputs)
    }

    fn register(
        &mut self,
        path: String,
        producer: Producer,
        bytes: Cow<'static, [u8]>,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        let existing = self
            .files
            .get(&path)
            .map(|(first, _)| first.name())
            .or_else(|| self.templates.get(&path).map(|(_, emitter)| *emitter));
        if let Some(first) = existing {
            return Err(PlanDiagnostic::output_conflict(
                &path,
                first,
                producer.name(),
            ));
        }
        self.files.insert(path, (producer, bytes));
        Ok(())
    }

    pub(super) fn emit(
        &mut self,
        producer: &'static str,
        path: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        self.register(
            delivered(path.into()),
            Producer::Emitter(producer),
            Cow::Owned(bytes.into()),
        )
    }

    pub(super) fn emit_all(
        &mut self,
        producer: &'static str,
        files: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        files
            .into_iter()
            .try_for_each(|(path, bytes)| self.emit(producer, path, bytes))
    }

    /// The trusted template text reserved for `producer` at `path`.
    pub(super) fn template(
        &self,
        producer: &'static str,
        path: &str,
    ) -> Result<&'static str, Vec<PlanDiagnostic>> {
        let (asset, _) = self
            .templates
            .get(&delivered(path.into()))
            .filter(|(_, owner)| *owner == producer)
            .ok_or_else(|| PlanDiagnostic::output_missing(path))?;
        std::str::from_utf8(asset.bytes).map_err(|_| PlanDiagnostic::output_missing(path))
    }

    /// Registers the single rendering of a reserved template as its emitter's
    /// output, also when the text equals the template.
    pub(super) fn emit_rendered(
        &mut self,
        producer: &'static str,
        path: &str,
        text: String,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        let path = delivered(path.into());
        self.templates
            .remove(&path)
            .filter(|(_, owner)| *owner == producer)
            .ok_or_else(|| PlanDiagnostic::output_missing(&path))?;
        self.register(
            path,
            Producer::Emitter(producer),
            Cow::Owned(text.into_bytes()),
        )
    }

    /// Delivers every template reserved for `producer` unchanged.
    pub(super) fn emit_verbatim(
        &mut self,
        producer: &'static str,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        let paths: Vec<_> = self
            .templates
            .iter()
            .filter(|(_, (_, owner))| *owner == producer)
            .map(|(path, _)| path.clone())
            .collect();
        for path in paths {
            let text = self.template(producer, &path)?.to_owned();
            self.emit_rendered(producer, &path, text)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    pub(super) fn contents(&self) -> BTreeMap<&str, &[u8]> {
        self.files
            .iter()
            .map(|(path, (_, bytes))| (path.as_str(), bytes.as_ref()))
            .collect()
    }

    /// Asset path to delivered path for every asset delivered verbatim.
    pub(super) fn original_assets(&self) -> BTreeMap<&'static str, &str> {
        self.files
            .iter()
            .filter_map(|(path, (producer, _))| match producer {
                Producer::Asset(asset) => Some((asset.relative_path, path.as_str())),
                Producer::Emitter(_) => None,
            })
            .collect()
    }

    /// The complete output; a reserved template nobody rendered is an omission.
    pub(super) fn finish(self) -> Result<EcuFiles, Vec<PlanDiagnostic>> {
        if let Some(path) = self.templates.keys().next() {
            return Err(PlanDiagnostic::output_missing(path));
        }
        Ok(self
            .files
            .into_iter()
            .map(|(path, (_, bytes))| (path, bytes))
            .collect())
    }
}

pub(super) fn trusted_asset(path: &str) -> Result<&'static AssetEntry, Vec<PlanDiagnostic>> {
    crate::resources::AssetInventory::embedded()
        .get(path)
        .ok_or_else(|| {
            super::ecu::reject(crate::product_message!(
                "backend.integration.outputs.trusted_asset_missing", "path" => path
            ))
        })
}

pub(super) fn trusted_text(path: &str) -> Result<&'static str, Vec<PlanDiagnostic>> {
    std::str::from_utf8(trusted_asset(path)?.bytes)
        .map_err(|_| PlanDiagnostic::output_missing(path))
}

/// One edit of a trusted runtime template.
pub(super) enum Edit<'a> {
    /// The anchor occurs exactly once.
    Once(&'a str, &'a str),
    /// The anchor occurs at least once; every occurrence is replaced.
    Every(&'a str, &'a str),
    /// The C identifier occurs at least once; every occurrence as a whole token is
    /// replaced, wherever line layout puts it.
    Token(&'a str, &'a str),
    /// Inserted before the closing include guard the template ends with.
    BeforeGuardEnd(&'a str),
}

/// Renders a trusted template in one pass; a missing anchor means the
/// template no longer matches its emitter and is rejected.
#[allow(
    clippy::disallowed_methods,
    reason = "the single renderer of trusted runtime templates"
)]
pub(super) fn render(
    template: &str,
    path: &str,
    edits: &[Edit],
) -> Result<String, Vec<PlanDiagnostic>> {
    let mut text = template.to_owned();
    for edit in edits {
        text = match *edit {
            Edit::Once(anchor, replacement) if text.matches(anchor).count() == 1 => {
                text.replacen(anchor, replacement, 1)
            }
            Edit::Every(anchor, replacement) if text.contains(anchor) => {
                text.replace(anchor, replacement)
            }
            Edit::Token(name, replacement) => match tokens(&text, name, replacement) {
                Some(text) => text,
                None => return Err(PlanDiagnostic::template_anchor(path, name)),
            },
            Edit::BeforeGuardEnd(addition) => match text.strip_suffix("#endif\n") {
                Some(body) => format!("{body}{addition}#endif\n"),
                None => return Err(PlanDiagnostic::template_anchor(path, "#endif")),
            },
            Edit::Once(anchor, _) | Edit::Every(anchor, _) => {
                return Err(PlanDiagnostic::template_anchor(path, anchor));
            }
        };
    }
    Ok(text)
}

/// `text` with every identifier token `name` replaced, or `None` without one.
fn tokens(text: &str, name: &str, replacement: &str) -> Option<String> {
    let identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    let mut found = false;
    while let Some(at) = rest.find(name) {
        output.push_str(&rest[..at]);
        let before = output.chars().next_back();
        let after = rest[at + name.len()..].chars().next();
        if before.is_some_and(identifier) || after.is_some_and(identifier) {
            output.push_str(name);
        } else {
            output.push_str(replacement);
            found = true;
        }
        rest = &rest[at + name.len()..];
    }
    output.push_str(rest);
    found.then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs() -> EcuOutputs {
        EcuOutputs::new(BuildTarget::LinuxX64ControlledV1).unwrap()
    }

    #[test]
    fn schm_template_preserves_both_partition_uses_and_requires_one_mode_insertion() {
        let outputs = outputs();
        let path = "src/SchM_Main.c";
        let template = outputs.template(ECU, path).unwrap();
        let text = render(
            template,
            path,
            &[
                Edit::Every("@PARTITION@", "65535"),
                Edit::Once("@MODES@", "/* modes */\n"),
            ],
        )
        .unwrap();
        assert!(text.contains("= {65535u};"));
        assert!(text.contains("ConfigPtr->partition != 65535u"));
        assert!(text.ends_with("/* modes */\n"));
        assert!(!text.contains('@'));
        let duplicate = format!("{template}@MODES@");
        for source in [duplicate.as_str(), "missing template anchors"] {
            let error = render(source, path, &[Edit::Once("@MODES@", "")]).unwrap_err();
            assert_eq!(error[0].code, "TEMPLATE_ANCHOR");
            assert_eq!(error[0].object.as_deref(), Some(path));
        }
    }

    #[test]
    fn a_second_producer_of_a_path_is_rejected() {
        let mut outputs = outputs();
        outputs
            .emit("first", "src/Generated.c", b"a".to_vec())
            .unwrap();
        let conflict = outputs
            .emit("second", "src/Generated.c", b"a".to_vec())
            .unwrap_err();
        assert_eq!(conflict[0].code, "OUTPUT_PRODUCER_CONFLICT");
        assert_eq!(
            conflict[0].object.as_deref(),
            Some("generated/src/Generated.c")
        );
        // A verbatim asset path and a reserved template path are owned too.
        let asset = outputs.emit("second", "NOTICE", Vec::new()).unwrap_err();
        assert_eq!(asset[0].code, "OUTPUT_PRODUCER_CONFLICT");
        let template = outputs
            .emit("second", "include/Os_Cfg.h", Vec::new())
            .unwrap_err();
        assert_eq!(template[0].code, "OUTPUT_PRODUCER_CONFLICT");
    }

    #[test]
    fn templates_are_rendered_once_by_their_emitter() {
        let mut outputs = outputs();
        assert_eq!(
            outputs.template("rte", "include/Os_Cfg.h").unwrap_err()[0].code,
            "OUTPUT_MISSING"
        );
        let template = outputs.template(OS, "include/Os_Cfg.h").unwrap();
        outputs
            .emit_rendered(OS, "include/Os_Cfg.h", format!("{template}/**/\n"))
            .unwrap();
        let again = outputs
            .emit_rendered(OS, "include/Os_Cfg.h", template.into())
            .unwrap_err();
        assert_eq!(again[0].code, "OUTPUT_MISSING");
        // Unchanged bytes are still the emitter's output, never the asset.
        let arti = outputs.template(ARTI, "src/Os_Arti.c").unwrap();
        outputs
            .emit_rendered(ARTI, "src/Os_Arti.c", arti.into())
            .unwrap();
        outputs.emit_verbatim(MEMMAP).unwrap();
        let original = outputs.original_assets();
        for asset in [
            "runtime/os/include/Os_Cfg.h",
            "runtime/os/src/Os_Arti.c",
            "runtime/os/include/Os_MemMap.h",
            "runtime/bsw/include/Can_MemMap.h",
        ] {
            assert!(!original.contains_key(asset), "{asset}");
        }
        assert!(
            outputs
                .paths()
                .any(|path| path == "generated/include/Can_MemMap.h")
        );
        assert!(
            !outputs
                .paths()
                .any(|path| path.starts_with("bsw/include/") && path.ends_with("_MemMap.h"))
        );
        // The Dcm mode templates were never rendered.
        assert_eq!(outputs.finish().unwrap_err()[0].code, "OUTPUT_MISSING");
    }

    #[test]
    fn template_edits_require_their_anchor() {
        let template = r#"#ifndef A
#define A
#define X 1
#endif
"#;
        assert_eq!(
            render(
                template,
                "A.h",
                &[
                    Edit::Once("X 1", "X 0"),
                    Edit::BeforeGuardEnd("#define Y\n")
                ]
            )
            .unwrap(),
            r#"#ifndef A
#define A
#define X 0
#define Y
#endif
"#
        );
        assert_eq!(
            render(
                r#"f(a, Os, 0u);
g(Os_X,
  Os,
  OsAlt);
"#,
                "a.c",
                &[Edit::Token("Os", "Rt")]
            )
            .unwrap(),
            r#"f(a, Rt, 0u);
g(Os_X,
  Rt,
  OsAlt);
"#
        );
        for edit in [
            Edit::Token("Os", ""),
            Edit::Once("Z", ""),
            Edit::Every("Z", ""),
            Edit::Once("#define", ""),
        ] {
            assert_eq!(
                render(template, "A.h", &[edit]).unwrap_err()[0].code,
                "TEMPLATE_ANCHOR"
            );
        }
        assert_eq!(
            render("int x;\n", "a.c", &[Edit::BeforeGuardEnd("")]).unwrap_err()[0].code,
            "TEMPLATE_ANCHOR"
        );
    }
}
