pub mod arxml;
pub mod arxml_render;
pub mod definitions;
pub mod execution;
pub mod generator;
pub mod host;
pub mod integration;
pub mod message;
pub mod model;
pub mod prepared;
pub mod project_model;
pub mod resources;
pub mod rules;
pub mod schema;
pub mod target;
#[cfg(feature = "verification-metrics")]
pub mod verification;

pub use arxml::Workspace;
pub use message::LocalizedText;
pub use model::{
    BuildReport, DiagnosticSettings, DiagnosticView, Direction, DtcView, FrameView,
    GenerationPreview, GenerationPreviewFile, GenerationReport, Issue, RunReport, SavePreview,
    SavePreviewFile, SignalView, WorkspaceView,
};
pub use prepared::{
    ApplicationSource, PreparedProject, prepare_ecu_project, prepare_ecu_project_with_applications,
    prepare_host_project,
};
