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

pub(super) const ARTI: &str = "arti";
pub(super) const DCM_MODES: &str = "dcm-modes";
pub(super) const OS: &str = "os";

/// Selected assets an emitter renders from instead of delivering verbatim.
const TEMPLATES: [(&str, &str); 4] = [
    ("runtime/os/include/Os_Cfg.h", OS),
    ("runtime/os/src/Os_Arti.c", ARTI),
    ("runtime/bsw/include/Rte_Dcm_Type.h", DCM_MODES),
    ("runtime/bsw/include/SchM_Dcm.h", DCM_MODES),
];

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
        let assets = super::ecu::source_assets(target).map_err(super::ecu::reject)?;
        for (path, asset) in assets {
            match TEMPLATES
                .iter()
                .find(|(template, _)| *template == asset.relative_path)
            {
                Some((_, producer)) => {
                    outputs.templates.insert(path, (asset, producer));
                }
                None => {
                    outputs.register(path, Producer::Asset(asset), Cow::Borrowed(asset.bytes))?
                }
            }
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
            path.into(),
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
            .get(path)
            .filter(|(_, owner)| *owner == producer)
            .ok_or_else(|| PlanDiagnostic::output_missing(path))?;
        std::str::from_utf8(asset.bytes).map_err(|_| PlanDiagnostic::output_missing(path))
    }

    /// Registers the single rendering of a reserved template. Bytes equal to
    /// the template remain the delivered asset.
    pub(super) fn emit_rendered(
        &mut self,
        producer: &'static str,
        path: &str,
        text: String,
    ) -> Result<(), Vec<PlanDiagnostic>> {
        let (asset, _) = self
            .templates
            .remove(path)
            .filter(|(_, owner)| *owner == producer)
            .ok_or_else(|| PlanDiagnostic::output_missing(path))?;
        if text.as_bytes() == asset.bytes {
            self.register(
                path.into(),
                Producer::Asset(asset),
                Cow::Borrowed(asset.bytes),
            )
        } else {
            self.register(
                path.into(),
                Producer::Emitter(producer),
                Cow::Owned(text.into_bytes()),
            )
        }
    }

    pub(super) fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
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

/// One edit of a trusted runtime template.
pub(super) enum Edit<'a> {
    /// The anchor occurs exactly once.
    Once(&'a str, &'a str),
    /// The anchor occurs at least once; every occurrence is replaced.
    Every(&'a str, &'a str),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs() -> EcuOutputs {
        EcuOutputs::new(BuildTarget::LinuxX64ControlledV1).unwrap()
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
        assert_eq!(conflict[0].object.as_deref(), Some("src/Generated.c"));
        // A verbatim asset path and a reserved template path are owned too.
        let asset = outputs.emit("second", "src/Dcm.c", Vec::new()).unwrap_err();
        assert_eq!(asset[0].code, "OUTPUT_PRODUCER_CONFLICT");
        let template = outputs
            .emit("second", "os/include/Os_Cfg.h", Vec::new())
            .unwrap_err();
        assert_eq!(template[0].code, "OUTPUT_PRODUCER_CONFLICT");
    }

    #[test]
    fn templates_are_rendered_once_by_their_emitter() {
        let mut outputs = outputs();
        assert_eq!(
            outputs.template("rte", "os/include/Os_Cfg.h").unwrap_err()[0].code,
            "OUTPUT_MISSING"
        );
        let template = outputs.template(OS, "os/include/Os_Cfg.h").unwrap();
        outputs
            .emit_rendered(OS, "os/include/Os_Cfg.h", format!("{template}/**/\n"))
            .unwrap();
        assert!(
            !outputs
                .original_assets()
                .contains_key("runtime/os/include/Os_Cfg.h")
        );
        let again = outputs
            .emit_rendered(OS, "os/include/Os_Cfg.h", template.into())
            .unwrap_err();
        assert_eq!(again[0].code, "OUTPUT_MISSING");
        // Unchanged bytes stay the delivered asset.
        let arti = outputs.template(ARTI, "os/src/Os_Arti.c").unwrap();
        outputs
            .emit_rendered(ARTI, "os/src/Os_Arti.c", arti.into())
            .unwrap();
        assert_eq!(
            outputs.original_assets()["runtime/os/src/Os_Arti.c"],
            "os/src/Os_Arti.c"
        );
        // The Dcm mode templates were never rendered.
        assert_eq!(outputs.finish().unwrap_err()[0].code, "OUTPUT_MISSING");
    }

    #[test]
    fn template_edits_require_their_anchor() {
        let template = "#ifndef A\n#define A\n#define X 1\n#endif\n";
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
            "#ifndef A\n#define A\n#define X 0\n#define Y\n#endif\n"
        );
        for edit in [
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
