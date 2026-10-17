use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use crate::target_spec::{BuildTarget, TargetSpec};

impl BuildTarget {
    pub fn is_native(self) -> bool {
        match self {
            Self::WindowsX64ControlledV1 => cfg!(all(windows, target_arch = "x86_64")),
            Self::LinuxX64ControlledV1 => cfg!(all(target_os = "linux", target_arch = "x86_64")),
        }
    }
}

/// External executable paths are supplied by the caller; constructing settings
/// never launches a process or probes the host environment.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecutionSettings {
    pub compiler: PathBuf,
    pub objdump: PathBuf,
    pub git: PathBuf,
    pub python: PathBuf,
}

impl ExecutionSettings {
    pub fn new(
        compiler: PathBuf,
        objdump: PathBuf,
        git: PathBuf,
        python: PathBuf,
    ) -> Result<Self, crate::message::LocalizedText> {
        for (name, path) in [
            ("compiler", &compiler),
            ("objdump", &objdump),
            ("git", &git),
            ("python", &python),
        ] {
            if !path.is_absolute()
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(
                    crate::product_message!("backend.target.normalized_path", "name" => name, "path" => path.display()),
                );
            }
        }
        Ok(Self {
            compiler,
            objdump,
            git,
            python,
        })
    }

    pub fn from_environment() -> Result<Self, crate::message::LocalizedText> {
        let required = |name: &str| {
            std::env::var_os(name).map(PathBuf::from).ok_or_else(
                || crate::product_message!("backend.target.executable_environment", "name" => name),
            )
        };
        Self::new(
            required("AUTOSAR_CC")?,
            required("AUTOSAR_OBJDUMP")?,
            required("AUTOSAR_GIT")?,
            required("AUTOSAR_PYTHON")?,
        )
    }

    pub fn python_from_environment() -> Option<PathBuf> {
        std::env::var_os("AUTOSAR_PYTHON")
            .map(PathBuf::from)
            .filter(|path| Path::new(path).is_absolute())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_executable_path_is_structured_without_changing_the_path() {
        let relative = PathBuf::from("用户/compiler");
        let error = ExecutionSettings::new(
            relative.clone(),
            relative.clone(),
            relative.clone(),
            relative.clone(),
        )
        .unwrap_err();
        let wire = serde_json::to_value(&error).unwrap();
        assert_eq!(wire["key"], "backend.target.normalized_path");
        assert_eq!(wire["params"]["name"], "compiler");
        assert_eq!(wire["params"]["path"], relative.display().to_string());
    }
}
