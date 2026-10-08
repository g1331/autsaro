use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Output identity, independent of the operating system running the workbench.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum BuildTarget {
    #[serde(rename = "windows-x64-controlled-v1")]
    WindowsX64ControlledV1,
    #[serde(rename = "linux-x64-controlled-v1")]
    LinuxX64ControlledV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetSpec {
    pub id: &'static str,
    pub abi: &'static str,
    pub native_port: &'static str,
    pub toolchain_asset: &'static str,
    pub kernel_patches: &'static [&'static str],
    pub kernel_patch_includes: &'static [(&'static str, &'static [&'static str])],
    pub link_libraries: &'static [&'static str],
    pub binary_name: &'static str,
    pub object_format: &'static str,
    pub required_sections: &'static [&'static str],
    pub logical_clock: &'static str,
}

const WINDOWS_PATCHES: &[&str] = &[
    "runtime/os/patches/0001-controlled-host-lifecycle.patch",
    "runtime/os/patches/0002-native-stack.patch",
    "runtime/os/patches/0003-activation-ready-policy.patch",
    "runtime/os/patches/0004-windows-event-namespace.patch",
    "runtime/os/patches/0005-category2-resource-gate.patch",
    "runtime/os/patches/0006-input-handler-ownership.patch",
    "runtime/os/patches/0007-controlled-tick.patch",
    "runtime/os/patches/0008-task-hook-boundaries.patch",
    "runtime/os/patches/0009-interrupt-source-control.patch",
    "runtime/os/patches/0010-native-interrupt-identities.patch",
    "runtime/os/patches/0011-isr-phase-before-mutex-release.patch",
    "runtime/os/patches/0012-nested-interrupt-dispatch.patch",
    "runtime/os/patches/0013-interrupt-source-repetition.patch",
    "runtime/os/patches/0014-relocatable-interrupt-vectors.patch",
];
const LINUX_PATCHES: &[&str] = &[
    "runtime/os/patches/0003-activation-ready-policy.patch",
    "runtime/os/patches/0007-controlled-tick.patch",
    "runtime/os/patches/0008-task-hook-boundaries.patch",
    "runtime/os/patches/linux/0001-controlled-posix-port.patch",
];
const LINUX_PATCH_INCLUDES: &[(&str, &[&str])] = &[
    (
        "runtime/os/patches/0003-activation-ready-policy.patch",
        &["tasks.c", "include/task.h", "include/FreeRTOS.h"],
    ),
    (
        "runtime/os/patches/0007-controlled-tick.patch",
        &["tasks.c"],
    ),
    (
        "runtime/os/patches/0008-task-hook-boundaries.patch",
        &["tasks.c"],
    ),
];

impl BuildTarget {
    pub const ALL: [Self; 2] = [Self::WindowsX64ControlledV1, Self::LinuxX64ControlledV1];

    pub const fn spec(self) -> TargetSpec {
        match self {
            Self::WindowsX64ControlledV1 => TargetSpec {
                id: "windows-x64-controlled-v1",
                abi: "x86_64-w64-mingw32",
                native_port: "FreeRTOS/MSVC-MingW + Windows host adapter",
                toolchain_asset: "runtime/os/toolchain.json",
                kernel_patches: WINDOWS_PATCHES,
                kernel_patch_includes: &[],
                link_libraries: &["winmm", "bcrypt"],
                binary_name: "ecu_host_batch.exe",
                object_format: "PE32+ x86-64",
                required_sections: &[".text", ".data", ".rdata", ".tls", ".os_vec", ".os_code"],
                logical_clock: "explicit controlled 1ms tick",
            },
            Self::LinuxX64ControlledV1 => TargetSpec {
                id: "linux-x64-controlled-v1",
                abi: "x86_64-linux-gnu",
                native_port: "FreeRTOS/GCC Posix + Linux host adapter",
                toolchain_asset: "runtime/os/toolchain-linux.json",
                kernel_patches: LINUX_PATCHES,
                kernel_patch_includes: LINUX_PATCH_INCLUDES,
                link_libraries: &["pthread"],
                binary_name: "ecu_host_batch",
                object_format: "ELF64 x86-64",
                required_sections: &[".text", ".data", ".bss", ".os_vec", ".os_code"],
                logical_clock: "explicit controlled 1ms tick",
            },
        }
    }

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
