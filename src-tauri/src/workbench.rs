use autosar_config_core::Workspace;
use autosar_config_core::execution::ProcessOwner;
use autosar_config_core::integration::{PlanDependencies, PlanDiagnostic, RuntimeCatalog};
use autosar_config_core::project_model::{ActionCapability, RuleCoverage, RuleSetIdentity};
use autosar_config_core::target::{BuildTarget, ExecutionSettings};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub(super) enum Language {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    xsd_archive: Option<PathBuf>,
    mod_archive: Option<PathBuf>,
    execution_tools: Option<ExecutionSettings>,
    build_target: Option<BuildTarget>,
    #[serde(default)]
    appearance: Appearance,
    #[serde(default)]
    language: Language,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OperationView {
    id: u64,
    stage: autosar_config_core::LocalizedText,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Capabilities {
    pub fingerprint: String,
    pub target: BuildTarget,
    pub targets: [BuildTarget; 2],
    pub native_execution: bool,
    pub has_workspace: bool,
    pub xsd_archive: Option<PathBuf>,
    pub mod_archive: Option<PathBuf>,
    pub resource_error: Option<autosar_config_core::LocalizedText>,
    pub execution_tools: Option<ExecutionSettings>,
    pub configured_execution_tools: Option<ExecutionSettings>,
    pub tool_error: Option<autosar_config_core::LocalizedText>,
    pub environment_overrides: Vec<&'static str>,
    pub operation: Option<OperationView>,
    pub rule_set_identity: Option<RuleSetIdentity>,
    pub rule_error: Option<autosar_config_core::LocalizedText>,
    pub rule_coverage: Vec<RuleCoverage>,
    pub definition_fingerprint: Option<String>,
    pub appearance: Appearance,
    pub language: Language,
    pub actions: Vec<ActionCapability>,
    pub verification_mode: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Reply<T> {
    pub value: T,
    pub capabilities: Capabilities,
    pub input_fingerprint: String,
}

struct ActiveOperation {
    view: OperationView,
    owner: Option<Arc<ProcessOwner>>,
}

pub(super) struct Session {
    pub workspace: Option<Arc<Workspace>>,
    pub resources: Option<Arc<PlanDependencies>>,
    pub tools: Option<Arc<ExecutionSettings>>,
    pub target: BuildTarget,
    settings: Settings,
    settings_bytes: Option<Vec<u8>>,
    settings_error: Option<autosar_config_core::LocalizedText>,
    resource_error: Option<autosar_config_core::LocalizedText>,
    tool_error: Option<autosar_config_core::LocalizedText>,
    revision: u64,
    next_operation: u64,
    active: Option<ActiveOperation>,
}

impl Session {
    pub fn invalidate(&mut self) -> Result<(), autosar_config_core::LocalizedText> {
        self.revision =
            self.revision
                .checked_add(1)
                .ok_or(autosar_config_core::product_message!(
                    "backend.workbench.revision_exhausted"
                ))?;
        Ok(())
    }
}

pub(super) struct AppState {
    session: Mutex<Session>,
    // All edits and final commits share this gate. Rendering and native execution do not hold it.
    operation: Mutex<()>,
    settings_file: PathBuf,
    instance: u64,
    pub runtime: Arc<RuntimeCatalog>,
    rule_set_identity: Option<RuleSetIdentity>,
    rule_coverage: Vec<RuleCoverage>,
    rule_error: Option<autosar_config_core::LocalizedText>,
}

pub(super) struct Snapshot {
    pub workspace: Option<Arc<Workspace>>,
    pub resources: Option<Arc<PlanDependencies>>,
    pub tools: Option<Arc<ExecutionSettings>>,
    pub target: BuildTarget,
    fingerprint: String,
}

impl Snapshot {
    pub fn workspace(&self) -> Result<&Workspace, autosar_config_core::LocalizedText> {
        self.workspace.as_deref().ok_or_else(|| {
            autosar_config_core::product_message!("backend.workbench.workspace_required").into()
        })
    }

    pub fn legacy_resources(
        &self,
    ) -> Result<&PlanDependencies, autosar_config_core::LocalizedText> {
        self.resources.as_deref().ok_or_else(|| {
            autosar_config_core::product_message!("backend.workbench.legacy_resources_required")
                .into()
        })
    }
}

pub(super) enum OperationKind {
    Read,
    Edit,
    Open,
    Native,
}

pub(super) struct Operation {
    state: Arc<AppState>,
    id: u64,
    input_fingerprint: String,
    pub snapshot: Snapshot,
    pub owner: Option<Arc<ProcessOwner>>,
}

pub(super) fn diagnostics(issues: Vec<PlanDiagnostic>) -> autosar_config_core::LocalizedText {
    autosar_config_core::LocalizedText::messages(issues.into_iter().flat_map(|issue| {
        [
            autosar_config_core::LocalizedText::evidence(issue.code),
            issue.message,
            issue.remedy,
        ]
    }))
}

fn resources(settings: &Settings) -> Result<PlanDependencies, autosar_config_core::LocalizedText> {
    let xsd = std::env::var_os("AUTOSAR_XSD_ARCHIVE")
        .map(PathBuf::from)
        .or_else(|| settings.xsd_archive.clone())
        .ok_or(autosar_config_core::product_message!(
            "backend.workbench.xsd_required"
        ))?;
    let mod_archive = std::env::var_os("AUTOSAR_MOD_ARCHIVE")
        .map(PathBuf::from)
        .or_else(|| settings.mod_archive.clone())
        .ok_or(autosar_config_core::product_message!(
            "backend.workbench.mod_required"
        ))?;
    let result = PlanDependencies::explicit(xsd, mod_archive)?;
    result.validate().map_err(diagnostics)?;
    Ok(result)
}

fn tools(settings: &Settings) -> Result<ExecutionSettings, autosar_config_core::LocalizedText> {
    let stored = settings.execution_tools.as_ref();
    let path = |name: &str, value: Option<&PathBuf>| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .or_else(|| value.cloned())
            .ok_or_else(|| autosar_config_core::product_message!("backend.workbench.tool_required", "name" => name))
    };
    let result = ExecutionSettings::new(
        path("AUTOSAR_CC", stored.map(|value| &value.compiler))?,
        path("AUTOSAR_OBJDUMP", stored.map(|value| &value.objdump))?,
        path("AUTOSAR_GIT", stored.map(|value| &value.git))?,
        path("AUTOSAR_PYTHON", stored.map(|value| &value.python))?,
    )?;
    for (name, path) in [
        ("compiler", &result.compiler),
        ("objdump", &result.objdump),
        ("git", &result.git),
        ("python", &result.python),
    ] {
        if !path.is_file() {
            return Err(
                autosar_config_core::product_message!("backend.workbench.tool_path_invalid", "name" => name, "path" => path.display()),
            );
        }
    }
    Ok(result)
}

impl AppState {
    pub fn new(settings_file: PathBuf) -> Result<Arc<Self>, autosar_config_core::LocalizedText> {
        let (bytes, settings_error) = match fs::read(&settings_file) {
            Ok(bytes) => (Some(bytes), None),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, None),
            Err(error) => (None, Some(error.to_string().into())),
        };
        let parsed = bytes
            .as_deref()
            .map(serde_json::from_slice::<Settings>)
            .transpose();
        let (settings, settings_error) = match parsed {
            Ok(settings) => (settings.unwrap_or_default(), settings_error),
            Err(error) => (
                Settings::default(),
                Some(
                    autosar_config_core::product_message!("backend.workbench.settings_invalid", "error" => error),
                ),
            ),
        };
        let resolved_resources = resources(&settings);
        let resolved_tools = tools(&settings);
        let target = settings
            .build_target
            .unwrap_or(if cfg!(target_os = "linux") {
                BuildTarget::LinuxX64ControlledV1
            } else {
                BuildTarget::WindowsX64ControlledV1
            });
        let resource_error = resolved_resources.as_ref().err().cloned();
        let rules = autosar_config_core::rules::rule_set_identity().and_then(|identity| {
            autosar_config_core::rules::coverage().map(|coverage| (identity, coverage))
        });
        let (rule_set_identity, rule_coverage, rule_error) = match rules {
            Ok((identity, coverage)) => (Some(identity), coverage, None),
            Err(error) => (None, Vec::new(), Some(error)),
        };
        Ok(Arc::new(Self {
            session: Mutex::new(Session {
                workspace: None,
                resources: resolved_resources.ok().map(Arc::new),
                tools: resolved_tools.as_ref().ok().cloned().map(Arc::new),
                target,
                settings,
                settings_bytes: bytes,
                settings_error,
                resource_error,
                tool_error: resolved_tools.err(),
                revision: 0,
                next_operation: 0,
                active: None,
            }),
            operation: Mutex::new(()),
            settings_file,
            instance: autosar_config_core::execution::monotonic_ns()?,
            runtime: Arc::new(RuntimeCatalog::embedded().map_err(diagnostics)?),
            rule_set_identity,
            rule_coverage,
            rule_error,
        }))
    }

    fn fingerprint(&self, session: &Session) -> String {
        let definition = session
            .workspace
            .as_deref()
            .map(Workspace::definition_fingerprint)
            .transpose();
        let definition = match &definition {
            Ok(Some(value)) => value.as_str(),
            Ok(None) => "no-project",
            Err(_) => "definition-unavailable",
        };
        let rules = self
            .rule_set_identity
            .as_ref()
            .map_or("rules-unavailable", |identity| identity.sha256.as_str());
        format!(
            "{}:{}:{rules}:{definition}",
            self.instance, session.revision
        )
    }

    fn check(
        &self,
        session: &Session,
        fingerprint: &str,
    ) -> Result<(), autosar_config_core::LocalizedText> {
        if self.fingerprint(session) != fingerprint {
            return Err(
                autosar_config_core::product_message!("backend.workbench.stale_delivery").into(),
            );
        }
        Ok(())
    }

    fn capabilities_locked(&self, session: &Session) -> Capabilities {
        Capabilities {
            fingerprint: self.fingerprint(session),
            target: session.target,
            targets: BuildTarget::ALL,
            native_execution: session.target.is_native(),
            has_workspace: session.workspace.is_some(),
            xsd_archive: session
                .resources
                .as_ref()
                .map(|value| value.xsd_archive.clone()),
            mod_archive: session
                .resources
                .as_ref()
                .map(|value| value.mod_archive.clone()),
            resource_error: session
                .settings_error
                .clone()
                .or_else(|| session.resource_error.clone()),
            execution_tools: session.tools.as_deref().cloned(),
            configured_execution_tools: session.settings.execution_tools.clone(),
            tool_error: session.tool_error.clone(),
            environment_overrides: [
                "AUTOSAR_CONFIG_DIR",
                "AUTOSAR_XSD_ARCHIVE",
                "AUTOSAR_MOD_ARCHIVE",
                "AUTOSAR_CC",
                "AUTOSAR_OBJDUMP",
                "AUTOSAR_GIT",
                "AUTOSAR_PYTHON",
            ]
            .into_iter()
            .filter(|name| std::env::var_os(name).is_some())
            .collect(),
            operation: session.active.as_ref().map(|active| active.view.clone()),
            rule_set_identity: self.rule_set_identity.clone(),
            rule_error: self.rule_error.clone(),
            rule_coverage: self.rule_coverage.clone(),
            definition_fingerprint: session
                .workspace
                .as_deref()
                .and_then(|workspace| workspace.definition_fingerprint().ok()),
            appearance: session.settings.appearance,
            language: session.settings.language,
            actions: self.action_capabilities(session),
            verification_mode: cfg!(feature = "native-webdriver"),
        }
    }

    fn action_capabilities(&self, session: &Session) -> Vec<ActionCapability> {
        let rules_available = self.rule_error.is_none();
        let has_workspace = session.workspace.is_some();
        let executing = session.active.is_some();
        let execution_available =
            session.target.is_native() && session.tools.is_some() && session.tool_error.is_none();
        let available =
            |action: &str, ready: bool, reason: fn() -> autosar_config_core::LocalizedText| {
                ActionCapability {
                    action: action.into(),
                    available: ready,
                    reason: (!ready).then(reason),
                }
            };
        vec![
            available("open", !executing, || {
                autosar_config_core::product_message!("backend.workbench.cancel_first")
            }),
            available("create", rules_available && !executing, || {
                autosar_config_core::product_message!(
                    "backend.workbench.rules_or_operation_unavailable"
                )
            }),
            available("source-view", has_workspace, || {
                autosar_config_core::product_message!("backend.workbench.source_required")
            }),
            available(
                "validate",
                has_workspace && rules_available && !executing,
                || {
                    autosar_config_core::product_message!(
                        "backend.workbench.validation_unavailable"
                    )
                },
            ),
            available(
                "edit",
                has_workspace && rules_available && !executing,
                || autosar_config_core::product_message!("backend.workbench.edit_unavailable"),
            ),
            available(
                "save",
                has_workspace && rules_available && !executing,
                || autosar_config_core::product_message!("backend.workbench.save_unavailable"),
            ),
            available(
                "generate",
                has_workspace && rules_available && !executing,
                || {
                    autosar_config_core::product_message!(
                        "backend.workbench.generation_unavailable"
                    )
                },
            ),
            available(
                "preflight",
                has_workspace && rules_available && execution_available && !executing,
                || autosar_config_core::product_message!("backend.workbench.native_unavailable"),
            ),
            available(
                "build",
                has_workspace && rules_available && execution_available && !executing,
                || autosar_config_core::product_message!("backend.workbench.native_unavailable"),
            ),
            available(
                "run",
                has_workspace && rules_available && execution_available && !executing,
                || autosar_config_core::product_message!("backend.workbench.native_unavailable"),
            ),
            available(
                "legacy-import",
                session.resources.is_some() && !executing,
                || {
                    autosar_config_core::product_message!(
                        "backend.workbench.legacy_import_unavailable"
                    )
                },
            ),
        ]
    }

    pub fn definition_cache_root(&self) -> Result<PathBuf, autosar_config_core::LocalizedText> {
        self.settings_file
            .parent()
            .map(|path| path.join("definition-catalogs"))
            .ok_or_else(|| {
                autosar_config_core::product_message!("backend.workbench.settings_parent_missing")
                    .into()
            })
    }

    pub fn read_workspace<T>(
        &self,
        fingerprint: &str,
        action: impl FnOnce(&Workspace) -> Result<T, autosar_config_core::LocalizedText>,
    ) -> Result<Reply<T>, autosar_config_core::LocalizedText> {
        let workspace = {
            let session = self.session.lock().map_err(|_| {
                autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
            })?;
            self.check(&session, fingerprint)?;
            session
                .workspace
                .clone()
                .ok_or(autosar_config_core::product_message!(
                    "backend.workbench.workspace_required"
                ))?
        };
        let value = action(&workspace)?;
        let session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        Ok(Reply {
            value,
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn configure_appearance(
        &self,
        fingerprint: &str,
        appearance: Appearance,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        let mut settings = session.settings.clone();
        settings.appearance = appearance;
        let bytes = self.persist(
            &settings,
            session.settings_bytes.as_deref(),
            session.revision,
        )?;
        session.settings = settings;
        session.settings_bytes = Some(bytes);
        session.settings_error = None;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn configure_language(
        &self,
        fingerprint: &str,
        language: Language,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        let mut settings = session.settings.clone();
        settings.language = language;
        let bytes = self.persist(
            &settings,
            session.settings_bytes.as_deref(),
            session.revision,
        )?;
        // Presentation commits retain the project revision and active operation.
        session.settings = settings;
        session.settings_bytes = Some(bytes);
        session.settings_error = None;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn capabilities(&self) -> Result<Capabilities, autosar_config_core::LocalizedText> {
        let session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        Ok(self.capabilities_locked(&session))
    }
    #[cfg(feature = "native-webdriver")]
    pub fn verification_metrics(
        &self,
        fingerprint: &str,
    ) -> Result<Reply<autosar_config_core::verification::Metrics>, autosar_config_core::LocalizedText>
    {
        let session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        Ok(Reply {
            value: autosar_config_core::verification::metrics(),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn view(
        &self,
    ) -> Result<Reply<autosar_config_core::WorkspaceView>, autosar_config_core::LocalizedText> {
        let session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        let value = session
            .workspace
            .as_deref()
            .ok_or(autosar_config_core::product_message!(
                "backend.workbench.workspace_required"
            ))?
            .view();
        Ok(Reply {
            value,
            input_fingerprint: self.fingerprint(&session),
            capabilities: self.capabilities_locked(&session),
        })
    }

    pub fn begin(
        self: &Arc<Self>,
        fingerprint: &str,
        stage: autosar_config_core::LocalizedText,
        kind: OperationKind,
    ) -> Result<Operation, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        if session.active.is_some() && !matches!(kind, OperationKind::Edit | OperationKind::Open) {
            return Err(autosar_config_core::product_message!(
                "backend.workbench.operation_active"
            )
            .into());
        }
        let previous = if session.active.is_some() {
            session.invalidate()?;
            session.active.take().and_then(|active| active.owner)
        } else {
            None
        };
        drop(session);
        if let Some(owner) = previous {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        if !matches!(kind, OperationKind::Open) && session.workspace.is_none() {
            return Err(autosar_config_core::product_message!(
                "backend.workbench.workspace_required"
            )
            .into());
        }
        if matches!(kind, OperationKind::Edit | OperationKind::Native)
            && let Some(error) = &self.rule_error
        {
            return Err(autosar_config_core::LocalizedText::messages([
                autosar_config_core::product_message!(
                    "backend.workbench.builtin_rules_unavailable"
                ),
                error.clone(),
            ]));
        }
        let snapshot = Snapshot {
            workspace: session.workspace.clone(),
            resources: session.resources.clone(),
            tools: session.tools.clone(),
            target: session.target,
            fingerprint: self.fingerprint(&session),
        };
        if matches!(kind, OperationKind::Native) && !snapshot.target.is_native() {
            return Err(autosar_config_core::product_message!(
                "backend.workbench.target_not_native"
            )
            .into());
        }
        session.next_operation =
            session
                .next_operation
                .checked_add(1)
                .ok_or(autosar_config_core::product_message!(
                    "backend.workbench.operation_id_exhausted"
                ))?;
        let id = session.next_operation;
        drop(session);
        let owner = if matches!(kind, OperationKind::Native) {
            let settings = snapshot
                .tools
                .as_ref()
                .ok_or(autosar_config_core::product_message!(
                    "backend.workbench.native_tools_required"
                ))?;
            Some(Arc::new(ProcessOwner::with_python(&settings.python)?))
        } else {
            None
        };
        self.session
            .lock()
            .map_err(|_| {
                autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
            })?
            .active = Some(ActiveOperation {
            view: OperationView {
                id,
                stage: stage.into(),
            },
            owner: owner.clone(),
        });
        Ok(Operation {
            state: Arc::clone(self),
            id,
            input_fingerprint: fingerprint.into(),
            snapshot,
            owner,
        })
    }

    fn finish(&self, id: u64) {
        if let Ok(mut session) = self.session.lock() {
            if session
                .active
                .as_ref()
                .is_some_and(|active| active.view.id == id)
            {
                session.active = None;
            }
        }
    }

    fn detach(
        &self,
        session: &mut Session,
    ) -> Result<Option<Arc<ProcessOwner>>, autosar_config_core::LocalizedText> {
        session.invalidate()?;
        Ok(session.active.take().and_then(|active| active.owner))
    }

    pub fn cancel(
        &self,
        fingerprint: &str,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn close(
        &self,
        fingerprint: &str,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        session.workspace = None;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    fn persist(
        &self,
        settings: &Settings,
        previous: Option<&[u8]>,
        revision: u64,
    ) -> Result<Vec<u8>, autosar_config_core::LocalizedText> {
        let actual = match fs::read(&self.settings_file) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string().into()),
        };
        if actual.as_deref() != previous {
            return Err(autosar_config_core::product_message!(
                "backend.workbench.settings_changed"
            )
            .into());
        }
        let parent = self
            .settings_file
            .parent()
            .ok_or(autosar_config_core::product_message!(
                "backend.workbench.settings_file_parent_missing"
            ))?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let stage = parent.join(format!(".settings-{}-{revision}.tmp", std::process::id()));
        let bytes = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage)
            .map_err(|error| error.to_string())?;
        let result = file.write_all(&bytes).and_then(|_| file.sync_all());
        drop(file);
        if let Err(error) = result {
            let _ = fs::remove_file(&stage);
            return Err(error.to_string().into());
        }
        if let Err(error) = fs::rename(&stage, &self.settings_file) {
            let _ = fs::remove_file(&stage);
            return Err(error.to_string().into());
        }
        Ok(bytes)
    }

    pub fn configure_resources(
        &self,
        fingerprint: &str,
        xsd: PathBuf,
        mod_archive: PathBuf,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        PlanDependencies::explicit(xsd.clone(), mod_archive.clone())?
            .validate()
            .map_err(diagnostics)?;
        self.configure(
            fingerprint,
            move |settings| {
                settings.xsd_archive = Some(xsd);
                settings.mod_archive = Some(mod_archive);
            },
            true,
        )
    }

    pub fn configure_tools(
        &self,
        fingerprint: &str,
        value: ExecutionSettings,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        for path in [&value.compiler, &value.objdump, &value.git, &value.python] {
            if !path.as_os_str().is_empty() && (!path.is_absolute() || !path.is_file()) {
                return Err(
                    autosar_config_core::product_message!("backend.workbench.configured_tool_path_invalid", "path" => path.display()),
                );
            }
        }
        self.configure(
            fingerprint,
            move |settings| settings.execution_tools = Some(value),
            false,
        )
    }

    fn configure(
        &self,
        fingerprint: &str,
        update: impl FnOnce(&mut Settings),
        validation: bool,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let mut settings = {
            let session = self.session.lock().map_err(|_| {
                autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
            })?;
            self.check(&session, fingerprint)?;
            session.settings.clone()
        };
        update(&mut settings);
        let resolved_resources = if validation {
            Some(Arc::new(resources(&settings)?))
        } else {
            None
        };
        let resolved_tools = if !validation {
            Some(Arc::new(tools(&settings)?))
        } else {
            None
        };
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        // Presentation saves do not advance the engineering fingerprint during validation.
        settings.language = session.settings.language;
        settings.appearance = session.settings.appearance;
        let bytes = self.persist(
            &settings,
            session.settings_bytes.as_deref(),
            session.revision,
        )?;
        if let Some(resources) = resolved_resources {
            if let Some(workspace) = &mut session.workspace {
                let workspace = Arc::make_mut(workspace);
                if workspace.uses_legacy_validation() {
                    workspace.set_legacy_validation_schema(resources.xsd_archive.clone())?;
                }
            }
            session.resources = Some(resources);
            session.resource_error = None;
        }
        if let Some(tools) = resolved_tools {
            session.tools = Some(tools);
            session.tool_error = None;
        }
        session.settings = settings;
        session.settings_bytes = Some(bytes);
        session.settings_error = None;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn select_target(
        &self,
        fingerprint: &str,
        target: BuildTarget,
    ) -> Result<Reply<()>, autosar_config_core::LocalizedText> {
        let _gate = self.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.check(&session, fingerprint)?;
        if let Some(error) = &session.settings_error {
            return Err(error.clone());
        }
        if session.target == target {
            return Ok(Reply {
                value: (),
                capabilities: self.capabilities_locked(&session),
                input_fingerprint: fingerprint.into(),
            });
        }
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        let mut settings = session.settings.clone();
        settings.build_target = Some(target);
        let bytes = self.persist(
            &settings,
            session.settings_bytes.as_deref(),
            session.revision,
        )?;
        session.settings = settings;
        session.settings_bytes = Some(bytes);
        session.target = target;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }
}

impl Operation {
    pub fn native_owner(&self) -> Result<&ProcessOwner, autosar_config_core::LocalizedText> {
        self.owner.as_deref().ok_or_else(|| {
            autosar_config_core::product_message!("backend.workbench.native_owner_missing").into()
        })
    }

    pub fn commit<T>(
        &self,
        action: impl FnOnce(&mut Session) -> Result<T, autosar_config_core::LocalizedText>,
    ) -> Result<Reply<T>, autosar_config_core::LocalizedText> {
        let _gate = self.state.operation.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.commit_lock_poisoned")
        })?;
        let mut session = self.state.session.lock().map_err(|_| {
            autosar_config_core::product_message!("backend.workbench.session_lock_poisoned")
        })?;
        self.state.check(&session, &self.snapshot.fingerprint)?;
        if !session
            .active
            .as_ref()
            .is_some_and(|active| active.view.id == self.id)
        {
            return Err(autosar_config_core::product_message!(
                "backend.workbench.operation_cancelled"
            )
            .into());
        }
        let value = action(&mut session)?;
        session.active = None;
        Ok(Reply {
            value,
            capabilities: self.state.capabilities_locked(&session),
            input_fingerprint: self.input_fingerprint.clone(),
        })
    }

    pub fn publish<T>(
        &self,
        workspace: Workspace,
        changed: bool,
        value: T,
    ) -> Result<Reply<T>, autosar_config_core::LocalizedText> {
        self.commit(move |session| {
            if changed {
                session.invalidate()?;
            }
            session.workspace = Some(Arc::new(workspace));
            Ok(value)
        })
    }

    pub fn publish_configuration(
        &self,
        workspace: Workspace,
        mut outcome: autosar_config_core::project_model::ChangeOutcome,
    ) -> Result<
        Reply<autosar_config_core::project_model::ChangeOutcome>,
        autosar_config_core::LocalizedText,
    > {
        let changed =
            workspace.input_fingerprint()? != self.snapshot.workspace()?.input_fingerprint()?;
        self.commit(move |session| {
            if changed {
                session.invalidate()?;
            }
            session.workspace = Some(Arc::new(workspace));
            outcome.projection.input_fingerprint = self.state.fingerprint(session);
            Ok(outcome)
        })
    }

    pub fn initialize_application(
        &self,
        mut workspace: Workspace,
        preview: &autosar_config_core::arxml::ApplicationInitializationPreview,
    ) -> Result<
        Reply<autosar_config_core::arxml::ApplicationInitializationOutcome>,
        autosar_config_core::LocalizedText,
    > {
        self.commit(move |session| {
            let next_revision =
                session
                    .revision
                    .checked_add(1)
                    .ok_or(autosar_config_core::product_message!(
                        "backend.workbench.revision_exhausted"
                    ))?;
            let mut outcome = workspace.initialize_application_previewed(preview)?;
            session.revision = next_revision;
            session.workspace = Some(Arc::new(workspace));
            outcome.projection.input_fingerprint = self.state.fingerprint(session);
            Ok(outcome)
        })
    }

    pub fn publish_projection(
        &self,
        workspace: Workspace,
        mut projection: autosar_config_core::project_model::ProjectProjection,
    ) -> Result<
        Reply<autosar_config_core::project_model::ProjectProjection>,
        autosar_config_core::LocalizedText,
    > {
        self.commit(move |session| {
            session.invalidate()?;
            session.workspace = Some(Arc::new(workspace));
            projection.input_fingerprint = self.state.fingerprint(session);
            Ok(projection)
        })
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        self.state.finish(self.id);
    }
}

#[cfg(test)]
mod language_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "autosar-language-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn settings(&self) -> PathBuf {
            self.0.join("settings.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn legacy_settings_follow_system_and_language_wire_values_are_explicit() {
        let settings: Settings = serde_json::from_str(r#"{"appearance":"dark"}"#).unwrap();
        assert_eq!(settings.language, Language::System);
        for (wire, language) in [
            ("system", Language::System),
            ("zh-CN", Language::Chinese),
            ("en", Language::English),
        ] {
            assert_eq!(serde_json::to_value(language).unwrap(), wire);
            assert_eq!(
                serde_json::from_value::<Language>(serde_json::json!(wire)).unwrap(),
                language
            );
        }
        assert!(serde_json::from_str::<Language>(r#""fr""#).is_err());
    }

    #[test]
    fn language_commit_preserves_project_fingerprint_drafts_and_active_operation() {
        let fixture = Fixture::new();
        let state = AppState::new(fixture.settings()).unwrap();
        let workspace = Arc::new(Workspace::create(&fixture.0.join("Project"), "Project").unwrap());
        match state.session.lock() {
            Ok(mut session) => session.workspace = Some(workspace.clone()),
            Err(error) => panic!("Test workspace setup failed: {error}"),
        }
        let before = state.capabilities().unwrap().fingerprint;
        let operation = state
            .begin(
                &before,
                autosar_config_core::product_message!("backend.operation.validate"),
                OperationKind::Read,
            )
            .unwrap();
        let operation_id = operation.id;
        let reply = state
            .configure_language(&before, Language::English)
            .unwrap();
        assert_eq!(reply.input_fingerprint, before);
        assert_eq!(reply.capabilities.fingerprint, before);
        assert_eq!(reply.capabilities.language, Language::English);
        assert_eq!(reply.capabilities.operation.unwrap().id, operation_id);
        match state.session.lock() {
            Ok(session) => {
                assert!(Arc::ptr_eq(session.workspace.as_ref().unwrap(), &workspace));
                assert_eq!(session.revision, 0);
            }
            Err(error) => panic!("Test workspace inspection failed: {error}"),
        }
        assert!(operation.commit(|_| Ok(())).is_ok());
        let restarted = AppState::new(fixture.settings()).unwrap();
        assert_eq!(
            restarted.capabilities().unwrap().language,
            Language::English
        );
    }

    #[test]
    fn configuration_commit_retains_concurrent_presentation_saves() {
        let fixture = Fixture::new();
        let initial = Settings {
            xsd_archive: Some(fixture.0.join("configured-xsd.zip")),
            mod_archive: Some(fixture.0.join("configured-mod.zip")),
            execution_tools: None,
            build_target: Some(BuildTarget::WindowsX64ControlledV1),
            appearance: Appearance::Dark,
            language: Language::Chinese,
        };
        fs::write(fixture.settings(), serde_json::to_vec(&initial).unwrap()).unwrap();
        let state = AppState::new(fixture.settings()).unwrap();
        let fingerprint = state.capabilities().unwrap().fingerprint;
        let executable = std::env::current_exe().unwrap();
        let configured_tools = ExecutionSettings::new(
            executable.clone(),
            executable.clone(),
            executable.clone(),
            executable,
        )
        .unwrap();
        let (snapshot_ready, snapshot_taken) = std::sync::mpsc::sync_channel(0);
        let (resume, continue_preparation) = std::sync::mpsc::sync_channel(0);
        let configuring_state = state.clone();
        let configuring_fingerprint = fingerprint.clone();
        let new_tools = configured_tools.clone();
        let configuration = std::thread::spawn(move || {
            configuring_state.configure(
                &configuring_fingerprint,
                move |settings| {
                    settings.execution_tools = Some(new_tools);
                    // Hold preparation after its snapshot, without holding either commit lock.
                    snapshot_ready.send(()).unwrap();
                    continue_preparation.recv().unwrap();
                },
                false,
            )
        });
        snapshot_taken
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        state
            .configure_language(&fingerprint, Language::English)
            .unwrap();
        state
            .configure_appearance(&fingerprint, Appearance::Light)
            .unwrap();
        assert_eq!(state.capabilities().unwrap().fingerprint, fingerprint);
        resume.send(()).unwrap();
        let reply = configuration.join().unwrap().unwrap();
        assert_eq!(reply.capabilities.language, Language::English);
        assert_eq!(
            serde_json::to_value(reply.capabilities.appearance).unwrap(),
            "light"
        );
        assert_eq!(
            reply.capabilities.configured_execution_tools,
            Some(configured_tools.clone())
        );
        let persisted: Settings =
            serde_json::from_slice(&fs::read(fixture.settings()).unwrap()).unwrap();
        assert_eq!(persisted.language, Language::English);
        assert_eq!(serde_json::to_value(persisted.appearance).unwrap(), "light");
        assert_eq!(persisted.execution_tools, Some(configured_tools.clone()));
        assert_eq!(persisted.xsd_archive, initial.xsd_archive);
        assert_eq!(persisted.mod_archive, initial.mod_archive);
        assert_eq!(persisted.build_target, initial.build_target);
        let restarted = AppState::new(fixture.settings())
            .unwrap()
            .capabilities()
            .unwrap();
        assert_eq!(restarted.language, Language::English);
        assert_eq!(serde_json::to_value(restarted.appearance).unwrap(), "light");
        assert_eq!(restarted.configured_execution_tools, Some(configured_tools));
        assert_eq!(restarted.target, BuildTarget::WindowsX64ControlledV1);
    }

    #[test]
    fn external_settings_change_retains_original_file_and_saved_language() {
        let fixture = Fixture::new();
        let original = br#"{"language":"zh-CN"}"#;
        fs::write(fixture.settings(), original).unwrap();
        let state = AppState::new(fixture.settings()).unwrap();
        let fingerprint = state.capabilities().unwrap().fingerprint;
        let external = br#"{"language":"zh-CN","appearance":"dark"}"#;
        fs::write(fixture.settings(), external).unwrap();
        let error = state
            .configure_language(&fingerprint, Language::English)
            .err()
            .unwrap();
        assert_eq!(
            serde_json::to_value(error).unwrap()["key"],
            "backend.workbench.settings_changed"
        );
        assert_eq!(fs::read(fixture.settings()).unwrap(), external);
        let capabilities = state.capabilities().unwrap();
        assert_eq!(capabilities.language, Language::Chinese);
        assert_eq!(capabilities.fingerprint, fingerprint);
    }

    #[test]
    fn failed_settings_stage_keeps_preference_and_original_bytes() {
        let fixture = Fixture::new();
        let original = br#"{"language":"zh-CN"}"#;
        fs::write(fixture.settings(), original).unwrap();
        let state = AppState::new(fixture.settings()).unwrap();
        let fingerprint = state.capabilities().unwrap().fingerprint;
        let stage = fixture
            .0
            .join(format!(".settings-{}-0.tmp", std::process::id()));
        fs::write(&stage, b"external staging file").unwrap();
        assert!(
            state
                .configure_language(&fingerprint, Language::English)
                .is_err()
        );
        assert_eq!(state.capabilities().unwrap().language, Language::Chinese);
        assert_eq!(state.capabilities().unwrap().fingerprint, fingerprint);
        assert_eq!(fs::read(fixture.settings()).unwrap(), original);
        assert_eq!(fs::read(stage).unwrap(), b"external staging file");
    }
}
