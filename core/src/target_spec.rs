use serde::{Deserialize, Serialize};

/// Output identity, independent of the operating system running the workbench.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ipc-types", derive(specta::Type))]
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
    "runtime/os/patches/0015-typed-port-state.patch",
    "runtime/os/patches/0016-essential-boolean-controls.patch",
    "runtime/os/patches/0017-portable-header-paths.patch",
    "runtime/os/patches/0018-windows-thread-setup-errors.patch",
    "runtime/os/patches/0019-windows-native-handoff-errors.patch",
    "runtime/os/patches/0020-bounded-queue-front.patch",
    "runtime/os/patches/0021-c99-comment-examples.patch",
    "runtime/os/patches/0022-explicit-function-addresses.patch",
    "runtime/os/patches/0023-preserve-parameter-values.patch",
    "runtime/os/patches/0024-conditional-private-definitions.patch",
    "runtime/os/patches/0025-configured-kernel-declarations.patch",
    "runtime/os/patches/0026-explicit-control-flow-boundaries.patch",
    "runtime/os/patches/0027-port-diagnostic-types.patch",
];
const LINUX_PATCHES: &[&str] = &[
    "runtime/os/patches/0003-activation-ready-policy.patch",
    "runtime/os/patches/0007-controlled-tick.patch",
    "runtime/os/patches/0008-task-hook-boundaries.patch",
    "runtime/os/patches/0015-typed-port-state.patch",
    "runtime/os/patches/0016-essential-boolean-controls.patch",
    "runtime/os/patches/0017-portable-header-paths.patch",
    "runtime/os/patches/0020-bounded-queue-front.patch",
    "runtime/os/patches/0021-c99-comment-examples.patch",
    "runtime/os/patches/0022-explicit-function-addresses.patch",
    "runtime/os/patches/0023-preserve-parameter-values.patch",
    "runtime/os/patches/0024-conditional-private-definitions.patch",
    "runtime/os/patches/0025-configured-kernel-declarations.patch",
    "runtime/os/patches/0026-explicit-control-flow-boundaries.patch",
    "runtime/os/patches/0027-port-diagnostic-types.patch",
    "runtime/os/patches/linux/0001-controlled-posix-port.patch",
    "runtime/os/patches/linux/0002-sequence-persistent-side-effects.patch",
    "runtime/os/patches/linux/0003-port-diagnostic-types.patch",
];
const LINUX_PATCH_INCLUDES: &[(&str, &[&str])] = &[
    (
        "runtime/os/patches/0027-port-diagnostic-types.patch",
        &["tasks.c", "queue.c"],
    ),
    (
        "runtime/os/patches/0026-explicit-control-flow-boundaries.patch",
        &["tasks.c", "queue.c"],
    ),
    (
        "runtime/os/patches/0025-configured-kernel-declarations.patch",
        &["include/portable.h", "include/task.h", "include/queue.h"],
    ),
    (
        "runtime/os/patches/0024-conditional-private-definitions.patch",
        &["tasks.c", "queue.c"],
    ),
    (
        "runtime/os/patches/0023-preserve-parameter-values.patch",
        &["tasks.c"],
    ),
    (
        "runtime/os/patches/0022-explicit-function-addresses.patch",
        &["tasks.c"],
    ),
    (
        "runtime/os/patches/0021-c99-comment-examples.patch",
        &["include/task.h", "include/queue.h", "include/timers.h"],
    ),
    (
        "runtime/os/patches/0020-bounded-queue-front.patch",
        &["queue.c"],
    ),
    (
        "runtime/os/patches/0017-portable-header-paths.patch",
        &["include/deprecated_definitions.h"],
    ),
    (
        "runtime/os/patches/0016-essential-boolean-controls.patch",
        &["tasks.c", "queue.c", "include/list.h", "include/task.h"],
    ),
    (
        "runtime/os/patches/0015-typed-port-state.patch",
        &[
            "tasks.c",
            "include/task.h",
            "include/FreeRTOS.h",
            "queue.c",
            "include/queue.h",
        ],
    ),
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
}

/// Preserve the legacy upstream alias without a case-folded include collision.
pub(crate) fn kernel_path(path: &str) -> Option<&str> {
    path.strip_prefix("third_party/freertos/").map(|name| {
        if name == "include/StackMacros.h" {
            "compat/include/StackMacros.h"
        } else {
            name
        }
    })
}
