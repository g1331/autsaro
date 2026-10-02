use autosar_config_core::Workspace;
use autosar_config_core::execution::ProcessOwner;
use autosar_config_core::integration::{PlanDependencies, PlanDiagnostic, RuntimeCatalog};
use autosar_config_core::target::{BuildTarget, ExecutionSettings};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    xsd_archive: Option<PathBuf>,
    mod_archive: Option<PathBuf>,
    execution_tools: Option<ExecutionSettings>,
    build_target: Option<BuildTarget>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OperationView {
    id: u64,
    stage: String,
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
    pub resource_error: Option<String>,
    pub execution_tools: Option<ExecutionSettings>,
    pub tool_error: Option<String>,
    pub environment_overrides: Vec<&'static str>,
    pub operation: Option<OperationView>,
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
    settings_error: Option<String>,
    resource_error: Option<String>,
    tool_error: Option<String>,
    revision: u64,
    next_operation: u64,
    active: Option<ActiveOperation>,
}

impl Session {
    pub fn invalidate(&mut self) -> Result<(), String> {
        self.revision = self.revision.checked_add(1).ok_or("工作台修订号已耗尽")?;
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
}

pub(super) struct Snapshot {
    pub workspace: Option<Arc<Workspace>>,
    pub resources: Arc<PlanDependencies>,
    pub tools: Option<Arc<ExecutionSettings>>,
    pub target: BuildTarget,
    fingerprint: String,
}

impl Snapshot {
    pub fn workspace(&self) -> Result<&Workspace, String> {
        self.workspace
            .as_deref()
            .ok_or_else(|| "请先创建或导入 ARXML 项目".into())
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

pub(super) fn diagnostics(issues: Vec<PlanDiagnostic>) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}\n{}", issue.code, issue.message, issue.remedy))
        .collect::<Vec<_>>()
        .join("\n")
}

fn resources(settings: &Settings) -> Result<PlanDependencies, String> {
    let xsd = std::env::var_os("AUTOSAR_XSD_ARCHIVE")
        .map(PathBuf::from)
        .or_else(|| settings.xsd_archive.clone())
        .ok_or("请配置合法的 R24-11 XSD 档案")?;
    let mod_archive = std::env::var_os("AUTOSAR_MOD_ARCHIVE")
        .map(PathBuf::from)
        .or_else(|| settings.mod_archive.clone())
        .ok_or("请配置合法的 R24-11 MOD 档案")?;
    let result = PlanDependencies::explicit(xsd, mod_archive)?;
    result.validate().map_err(diagnostics)?;
    Ok(result)
}

fn tools(settings: &Settings) -> Result<ExecutionSettings, String> {
    let stored = settings.execution_tools.as_ref();
    let path = |name, value: Option<&PathBuf>| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .or_else(|| value.cloned())
            .ok_or_else(|| format!("请配置原生工具 {name}"))
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
            return Err(format!(
                "{name} 不是存在的可执行文件路径: {}",
                path.display()
            ));
        }
    }
    Ok(result)
}

impl AppState {
    pub fn new(settings_file: PathBuf) -> Result<Arc<Self>, String> {
        let (bytes, settings_error) = match fs::read(&settings_file) {
            Ok(bytes) => (Some(bytes), None),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, None),
            Err(error) => (None, Some(error.to_string())),
        };
        let parsed = bytes
            .as_deref()
            .map(serde_json::from_slice::<Settings>)
            .transpose();
        let (settings, settings_error) = match parsed {
            Ok(settings) => (settings.unwrap_or_default(), settings_error),
            Err(error) => (
                Settings::default(),
                Some(format!("设置文件无效，原文件保留: {error}")),
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
        }))
    }

    fn fingerprint(&self, session: &Session) -> String {
        format!("{}:{}", self.instance, session.revision)
    }

    fn check(&self, session: &Session, fingerprint: &str) -> Result<(), String> {
        if self.fingerprint(session) != fingerprint {
            return Err("STALE_DELIVERY: 当前项目、规范、工具或目标已变化；旧操作未提交".into());
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
        }
    }

    pub fn capabilities(&self) -> Result<Capabilities, String> {
        let session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        Ok(self.capabilities_locked(&session))
    }

    pub fn view(&self) -> Result<Reply<autosar_config_core::WorkspaceView>, String> {
        let session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        let value = session
            .workspace
            .as_deref()
            .ok_or("请先创建或导入 ARXML 项目")?
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
        stage: &str,
        kind: OperationKind,
    ) -> Result<Operation, String> {
        let _gate = self.operation.lock().map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        self.check(&session, fingerprint)?;
        if session.active.is_some() && !matches!(kind, OperationKind::Edit | OperationKind::Open) {
            return Err("工作台已有运行中的操作".into());
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
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        if !matches!(kind, OperationKind::Open) && session.workspace.is_none() {
            return Err("请先创建或导入 ARXML 项目".into());
        }
        if let Some(error) = session
            .settings_error
            .as_ref()
            .or(session.resource_error.as_ref())
        {
            return Err(error.clone());
        }
        let snapshot = Snapshot {
            workspace: session.workspace.clone(),
            resources: session.resources.clone().ok_or("请先配置合法规范档案")?,
            tools: session.tools.clone(),
            target: session.target,
            fingerprint: self.fingerprint(&session),
        };
        if matches!(kind, OperationKind::Native) && !snapshot.target.is_native() {
            return Err("未执行：本机不支持所选目标执行".into());
        }
        session.next_operation = session
            .next_operation
            .checked_add(1)
            .ok_or("操作序号已耗尽")?;
        let id = session.next_operation;
        drop(session);
        let owner = if matches!(kind, OperationKind::Native) {
            let settings = snapshot
                .tools
                .as_ref()
                .ok_or("请先配置原生执行工具；尚未预检")?;
            Some(Arc::new(ProcessOwner::with_python(&settings.python)?))
        } else {
            None
        };
        self.session.lock().map_err(|_| "工作区状态锁损坏")?.active = Some(ActiveOperation {
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

    fn detach(&self, session: &mut Session) -> Result<Option<Arc<ProcessOwner>>, String> {
        session.invalidate()?;
        Ok(session.active.take().and_then(|active| active.owner))
    }

    pub fn cancel(&self, fingerprint: &str) -> Result<Reply<()>, String> {
        let _gate = self.operation.lock().map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        Ok(Reply {
            value: (),
            capabilities: self.capabilities_locked(&session),
            input_fingerprint: fingerprint.into(),
        })
    }

    pub fn close(&self, fingerprint: &str) -> Result<Reply<()>, String> {
        let _gate = self.operation.lock().map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
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
    ) -> Result<Vec<u8>, String> {
        let actual = match fs::read(&self.settings_file) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if actual.as_deref() != previous {
            return Err("设置已被外部修改；原文件未覆盖".into());
        }
        let parent = self.settings_file.parent().ok_or("设置文件须有父目录")?;
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
            return Err(error.to_string());
        }
        if let Err(error) = fs::rename(&stage, &self.settings_file) {
            let _ = fs::remove_file(&stage);
            return Err(error.to_string());
        }
        Ok(bytes)
    }

    pub fn configure_resources(
        &self,
        fingerprint: &str,
        xsd: PathBuf,
        mod_archive: PathBuf,
    ) -> Result<Reply<()>, String> {
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
    ) -> Result<Reply<()>, String> {
        let ExecutionSettings {
            compiler,
            objdump,
            git,
            python,
        } = value;
        let value = ExecutionSettings::new(compiler, objdump, git, python)?;
        for path in [&value.compiler, &value.objdump, &value.git, &value.python] {
            if !path.is_file() {
                return Err(format!("工具路径不存在: {}", path.display()));
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
    ) -> Result<Reply<()>, String> {
        let mut settings = {
            let session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
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
        let _gate = self.operation.lock().map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        self.check(&session, fingerprint)?;
        let owner = self.detach(&mut session)?;
        drop(session);
        if let Some(owner) = owner {
            owner.cancel()?;
        }
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
        let bytes = self.persist(
            &settings,
            session.settings_bytes.as_deref(),
            session.revision,
        )?;
        if let Some(resources) = resolved_resources {
            if let Some(workspace) = &mut session.workspace {
                Arc::make_mut(workspace).set_validation_schema(resources.xsd_archive.clone());
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
    ) -> Result<Reply<()>, String> {
        let _gate = self.operation.lock().map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
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
        let mut session = self.session.lock().map_err(|_| "工作区状态锁损坏")?;
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
    pub fn native_owner(&self) -> Result<&ProcessOwner, String> {
        self.owner
            .as_deref()
            .ok_or_else(|| "原生操作没有已配置的进程 owner".into())
    }

    pub fn commit<T>(
        &self,
        action: impl FnOnce(&mut Session) -> Result<T, String>,
    ) -> Result<Reply<T>, String> {
        let _gate = self
            .state
            .operation
            .lock()
            .map_err(|_| "工作台提交锁损坏")?;
        let mut session = self.state.session.lock().map_err(|_| "工作区状态锁损坏")?;
        self.state.check(&session, &self.snapshot.fingerprint)?;
        if !session
            .active
            .as_ref()
            .is_some_and(|active| active.view.id == self.id)
        {
            return Err("STALE_DELIVERY: 操作已被取消；旧结果未提交".into());
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
    ) -> Result<Reply<T>, String> {
        self.commit(move |session| {
            if changed {
                session.invalidate()?;
            }
            session.workspace = Some(Arc::new(workspace));
            Ok(value)
        })
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        self.state.finish(self.id);
    }
}
