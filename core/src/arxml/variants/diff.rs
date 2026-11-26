use super::{resolve::resolve, *};
use crate::project_model::ValueKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDifference {
    pub path: String,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticChange {
    Added,
    Removed,
    Value,
    Reference,
    Origin,
    State,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticDifference {
    pub target: LayerTarget,
    pub changes: Vec<SemanticChange>,
    pub before: Option<SemanticEntry>,
    pub after: Option<SemanticEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDifference {
    pub sources: Vec<SourceDifference>,
    pub semantics: Vec<SemanticDifference>,
    pub definitions_changed: bool,
}

fn origin_keys(row: &SemanticEntry) -> BTreeSet<(&[String], &str)> {
    row.sources
        .iter()
        .map(|origin| (origin.layer.as_slice(), origin.file.as_str()))
        .collect()
}

fn action_kind(action: Option<&LayerAction>) -> Option<&'static str> {
    match action {
        Some(LayerAction::Set {
            default_origin: Some(_),
            ..
        }) => Some("default"),
        Some(LayerAction::Set { .. }) => Some("set"),
        Some(LayerAction::Delete) => Some("delete"),
        Some(LayerAction::Exclude) => Some("exclude"),
        None => None,
    }
}

impl Workspace {
    /// Returns effective rows with their portable identities and origin locations.
    pub fn project_semantics(
        &self,
    ) -> Result<BTreeMap<LayerTarget, SemanticEntry>, Vec<PlanDiagnostic>> {
        let failure = |message| vec![diagnostic("PROJECT_DIFF", "layers", message)];
        self.ensure_sources_current().map_err(failure)?;
        let (_, layers) = self.project_parts().map_err(failure)?;
        let sources = self.original_layer_sources().map_err(failure)?;
        resolve(layers, &sources, &self.catalog).map(|resolution| resolution.semantics)
    }

    pub fn compare_project(&self, after: &Self) -> Result<ProjectDifference, Vec<PlanDiagnostic>> {
        let diagnostic = |message| vec![diagnostic("PROJECT_DIFF", "layers", message)];
        let (_, before_layers) = self.project_parts().map_err(diagnostic)?;
        let (_, after_layers) = after.project_parts().map_err(diagnostic)?;
        let before_sources = self.original_layer_sources().map_err(diagnostic)?;
        let after_sources = after.original_layer_sources().map_err(diagnostic)?;
        let before = resolve(before_layers, &before_sources, &self.catalog)?.semantics;
        let current = resolve(after_layers, &after_sources, &after.catalog)?.semantics;
        let mut sources = BTreeMap::new();
        for source in &before_sources {
            sources.insert(
                source.logical_path().to_owned(),
                (Some(source.bytes().to_owned()), None),
            );
        }
        for source in &after_sources {
            sources
                .entry(source.logical_path().to_owned())
                .or_default()
                .1 = Some(source.bytes().to_owned());
        }
        sources.insert(
            "workbench-project.json".into(),
            (
                self.project
                    .as_ref()
                    .map(|project| project.current.as_bytes().to_vec()),
                after
                    .project
                    .as_ref()
                    .map(|project| project.current.as_bytes().to_vec()),
            ),
        );
        let sources = sources
            .into_iter()
            .filter(|(_, (before, after))| before != after)
            .map(|(path, (before, after))| SourceDifference {
                path,
                before,
                after,
            })
            .collect();
        let targets: BTreeSet<_> = before.keys().chain(current.keys()).collect();
        let mut semantics = Vec::new();
        for target in targets {
            let left = before.get(target);
            let right = current.get(target);
            let mut changes = Vec::new();
            match (left, right) {
                (Some(left), Some(right)) => {
                    if left.values != right.values {
                        let reference = target
                            .field
                            .as_deref()
                            .and_then(|id| after.catalog.get(id))
                            .is_some_and(|definition| {
                                definition.kind == Some(ValueKind::Reference)
                            });
                        changes.push(if reference {
                            SemanticChange::Reference
                        } else {
                            SemanticChange::Value
                        });
                    }
                    if left.values == right.values && origin_keys(left) != origin_keys(right) {
                        changes.push(SemanticChange::Origin);
                    }
                    if action_kind(left.operation.as_ref()) != action_kind(right.operation.as_ref())
                    {
                        changes.push(SemanticChange::State);
                        if left.values.is_empty() != right.values.is_empty()
                            && target.field.is_none()
                        {
                            changes.push(if right.values.is_empty() {
                                SemanticChange::Removed
                            } else {
                                SemanticChange::Added
                            });
                        }
                    }
                }
                (None, Some(_)) => changes.push(SemanticChange::Added),
                (Some(_), None) => changes.push(SemanticChange::Removed),
                (None, None) => {}
            }
            if !changes.is_empty() {
                semantics.push(SemanticDifference {
                    target: target.clone(),
                    changes,
                    before: left.cloned(),
                    after: right.cloned(),
                });
            }
        }
        Ok(ProjectDifference {
            sources,
            semantics,
            definitions_changed: self.definition_fingerprint().map_err(diagnostic)?
                != after.definition_fingerprint().map_err(diagnostic)?,
        })
    }
}
