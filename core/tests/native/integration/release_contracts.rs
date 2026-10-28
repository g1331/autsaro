//! Public release identities and isolated, dependency-specific preprocessor refusals.
use super::*;
use std::path::Path;

fn compile(root: &Path, original: &Path, source: &str, label: &str, accepted: bool, error: &str) {
    let input = root.join("release-consumer.c");
    std::fs::write(&input, source).unwrap();
    let mut command = tooling::c_compiler();
    command
        .current_dir(root)
        .args(["-fsyntax-only", "-I.", "-Iinclude"]);
    if !source.contains("#define RTE_CORE") {
        command.arg("-DRTE_CORE");
    }
    if root.join("target.json").exists() {
        let target: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("target.json")).unwrap()).unwrap();
        for flag in target["compilerFlags"].as_array().unwrap() {
            command.arg(flag.as_str().unwrap());
        }
        for path in target["includePaths"].as_array().unwrap() {
            command.arg("-I").arg(path.as_str().unwrap());
        }
    } else {
        command.args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"]);
    }
    command
        .arg("-I")
        .arg(original.parent().unwrap())
        .arg(&input);
    let output = tooling::run_public_command(
        &mut command,
        root,
        label,
        std::time::Duration::from_secs(60),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.success(), accepted, "{label}: {stderr}");
    if !accepted {
        let errors: Vec<_> = stderr
            .lines()
            .filter(|line| line.contains("error:"))
            .collect();
        assert_eq!(errors.len(), 1, "{label}: {stderr}");
        assert!(errors[0].contains(error), "{label}: {stderr}");
    }
}

fn install(files: &[(String, Vec<u8>)], root: &Path) {
    for (path, bytes) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    if root.join("target.json").exists() {
        let target: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("target.json")).unwrap()).unwrap();
        for patch in target["kernelPatches"].as_array().unwrap() {
            let mut git = std::process::Command::new(tooling::execution_settings().git);
            git.current_dir(root.join("kernel"))
                .env("GIT_CEILING_DIRECTORIES", root)
                .args([
                    "-c",
                    "core.autocrlf=false",
                    "apply",
                    "--ignore-space-change",
                ]);
            for include in patch["includes"].as_array().unwrap() {
                git.arg(format!("--include={}", include.as_str().unwrap()));
            }
            git.arg(
                root.join(
                    patch["path"]
                        .as_str()
                        .unwrap()
                        .trim_start_matches("runtime/"),
                ),
            );
            let result = tooling::run_public_command(
                &mut git,
                root,
                "release-consumer-kernel-patch",
                std::time::Duration::from_secs(30),
            );
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

// The five pre-existing RTE imports retain their original dependency-specific diagnostics.
fn import_diagnostic(consumer: &str, dependency: &str, missing: bool) -> String {
    let legacy = if consumer == "Rte.c" {
        match dependency {
            "Com.h" => Some("Com"),
            "Os.h" => Some("Os"),
            "Std_Types.h" => Some("Std_Types"),
            "Platform_Types.h" => Some("Platform"),
            "ComStack_Types.h" => Some("ComStack_Types"),
            _ => None,
        }
    } else {
        None
    };
    match legacy {
        Some(module) => format!(
            "{} {module} AUTOSAR release",
            if missing { "Missing" } else { "Incompatible" }
        ),
        None => format!(
            "{consumer} -> {dependency}: {} AUTOSAR release",
            if missing { "missing" } else { "incompatible" }
        ),
    }
}

// Poison only the intended include edge, then restore it before another consumer
// can observe it. The real source is compiled, including declarations and bodies;
// a different guard or syntax error cannot satisfy the single exact diagnostic.
fn isolated(source: &str, consumer: &str, dependency: &str, prefix: &str, poison: &str) -> String {
    let error = import_diagnostic(consumer, dependency, true);
    let marker = source
        .find(&error)
        .unwrap_or_else(|| panic!("missing edge {error}"));
    let start = source[..marker].rfind("#if ").unwrap();
    let end = marker + source[marker..].find("#endif").unwrap() + "#endif".len();
    let save = ["MAJOR", "MINOR", "REVISION"].map(|part| format!(
        "#undef {prefix}_AR_RELEASE_{part}_VERSION\n#define {prefix}_AR_RELEASE_{part}_VERSION {}U\n",
        match part { "MAJOR" => 4, "MINOR" => 10, _ => 0 }
    )).join("");
    format!(
        "{}{}{}\n{}{}",
        &source[..start],
        poison,
        &source[start..end],
        save,
        &source[end..]
    )
}

#[test]
fn direct_external_headers_refuse_only_missing_or_wrong_major_minor() {
    // Verifies[Can]: SWS_BSW_00036
    // Verifies[CanIf]: SWS_BSW_00036
    // Verifies[CanTp]: SWS_BSW_00036
    // Verifies[Com]: SWS_BSW_00036
    // Verifies[ComM]: SWS_BSW_00036
    // Verifies[BswM]: SWS_BSW_00036
    // Verifies[Dcm]: SWS_BSW_00036
    // Verifies[PduR]: SWS_BSW_00036
    // Verifies[Os]: SWS_BSW_00036
    // Verifies[Rte]: SWS_BSW_00036
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files_for_target(&plan, native_delivery_target());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    let edges = [
        ("CanIf_PBcfg.c", "Rte_MemMap.h", "MEMMAP"),
        ("PduR_PBcfg.c", "Rte_MemMap.h", "MEMMAP"),
        ("Ecu_RuntimeConfig.c", "Rte_MemMap.h", "MEMMAP"),
        ("Rte.c", "Com.h", "COM"),
        ("Rte.c", "Os.h", "OS"),
        ("Rte.c", "Std_Types.h", "STD_TYPES"),
        ("Rte.c", "Platform_Types.h", "PLATFORM"),
        ("Rte.c", "ComStack_Types.h", "COMSTACKTYPE"),
        ("CanIf_PBcfg.c", "Can_Cfg.h", "CAN"),
        ("Ecu_RuntimeConfig.c", "BswM.h", "BSWM"),
        ("Ecu_RuntimeConfig.c", "BswM_ComM.h", "BSWM"),
        ("Ecu_RuntimeConfig.c", "CanIf.h", "CANIF"),
        ("Ecu_RuntimeConfig.c", "CanIf_Cfg.h", "CANIF"),
        ("Ecu_RuntimeConfig.c", "CanTp.h", "CANTP"),
        ("Ecu_RuntimeConfig.c", "CanTp_Cfg.h", "CANTP"),
        ("Ecu_RuntimeConfig.c", "Can_Cfg.h", "CAN"),
        ("Ecu_RuntimeConfig.c", "Com.h", "COM"),
        ("Ecu_RuntimeConfig.c", "ComM.h", "COMM"),
        ("Ecu_RuntimeConfig.c", "ComM_BswM.h", "COMM"),
        ("Ecu_RuntimeConfig.c", "ComM_Cfg.h", "COMM"),
        ("Ecu_RuntimeConfig.c", "ComM_Internal.h", "COMM"),
        ("Ecu_RuntimeConfig.c", "Com_Cfg.h", "COM"),
        ("Ecu_RuntimeConfig.c", "Com_Internal.h", "COM"),
        ("Ecu_RuntimeConfig.c", "Dcm.h", "DCM"),
        ("Ecu_RuntimeConfig.c", "Dcm_Cfg.h", "DCM"),
        ("Ecu_RuntimeConfig.c", "Dcm_ComM.h", "DCM"),
        ("Ecu_RuntimeConfig.c", "Det.h", "DET"),
        ("Ecu_RuntimeConfig.c", "Det_Externals.h", "DET"),
        ("Ecu_RuntimeConfig.c", "LSduR.h", "LSDUR"),
        ("Ecu_RuntimeConfig.c", "LSduR_CanTp.h", "LSDUR"),
        ("Ecu_RuntimeConfig.c", "LSduR_Cfg.h", "LSDUR"),
        ("Ecu_RuntimeConfig.c", "LSduR_PduR.h", "LSDUR"),
        ("Ecu_RuntimeConfig.c", "PduR.h", "PDUR"),
        ("Ecu_RuntimeConfig.c", "PduR_CanIf.h", "PDUR"),
        ("Ecu_RuntimeConfig.c", "PduR_CanTp.h", "PDUR"),
        ("Ecu_RuntimeConfig.c", "PduR_Cfg.h", "PDUR"),
        ("Ecu_RuntimeConfig.c", "PduR_Com.h", "PDUR"),
        ("Ecu_RuntimeConfig.c", "Rte_Cfg.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "Rte_Com.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "Rte_Main.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "SchM_Can.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "SchM_CanTp.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "SchM_Com.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "SchM_ComM.h", "RTE"),
        ("Ecu_RuntimeConfig.c", "SchM_Dcm.h", "RTE"),
        ("Ecu_TargetConfig.h", "Os_Cfg.h", "OS"),
        ("Ecu_TargetConfig.h", "Rte_MemMap.h", "MEMMAP"),
        ("Ecu_HostBusSM.h", "ComM.h", "COMM"),
        ("Ecu_HostBusSM.h", "CanIf.h", "CANIF"),
        ("Ecu_HostBusSM.h", "Ecu_HostBusSM_MemMap.h", "MEMMAP"),
        ("Os_HostWindows.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Windows.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Windows.h", "Std_Types.h", "STD_TYPES"),
        ("PduR_PBcfg.c", "CanIf_Cfg.h", "CANIF"),
        ("PduR_PBcfg.c", "CanTp.h", "CANTP"),
        ("PduR_PBcfg.c", "CanTp_Cfg.h", "CANTP"),
        ("PduR_PBcfg.c", "Com.h", "COM"),
        ("PduR_PBcfg.c", "Com_Cfg.h", "COM"),
        ("PduR_PBcfg.c", "Dcm.h", "DCM"),
        ("PduR_PBcfg.c", "Dcm_Cfg.h", "DCM"),
        ("PduR_PBcfg.c", "LSduR.h", "LSDUR"),
        ("PduR_PBcfg.c", "LSduR_Cfg.h", "LSDUR"),
        ("PduR_PBcfg.c", "LSduR_PduR.h", "LSDUR"),
        ("Rte.c", "Com_Cfg.h", "COM"),
        ("Rte.c", "Rte_Dcm_Type.h", "DCM"),
        ("Rte.c", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_Ingress.h", "Ingress_MemMap.h", "MEMMAP"),
        ("Rte_Os.h", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_OsService.c", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_Process.h", "Process_MemMap.h", "MEMMAP"),
        ("SchM_Main.c", "Rte_MemMap.h", "MEMMAP"),
        ("Arti.c", "Os_Host.h", "OS"),
        ("Arti.c", "Os_MemMap.h", "MEMMAP"),
        ("Arti.h", "Os_MemMap.h", "MEMMAP"),
        ("Arti.h", "Std_Types.h", "STD_TYPES"),
        ("BswM.c", "BswM_MemMap.h", "MEMMAP"),
        ("BswM.c", "SchM_BswM.h", "RTE"),
        ("BswM.h", "ComM.h", "COMM"),
        ("Can.c", "CanIf.h", "CANIF"),
        ("Can.c", "Can_MemMap.h", "MEMMAP"),
        ("Can.c", "Det.h", "DET"),
        ("Can.c", "SchM_Can.h", "RTE"),
        ("Can.h", "Can_MemMap.h", "MEMMAP"),
        ("BswM.h", "BswM_MemMap.h", "MEMMAP"),
        ("BswM_ComM.h", "BswM_MemMap.h", "MEMMAP"),
        ("CanIf.h", "CanIf_MemMap.h", "MEMMAP"),
        ("CanTp.h", "CanTp_MemMap.h", "MEMMAP"),
        ("Com.h", "Com_MemMap.h", "MEMMAP"),
        ("ComM.h", "ComM_MemMap.h", "MEMMAP"),
        ("ComM_BswM.h", "ComM_MemMap.h", "MEMMAP"),
        ("ComM_Dcm.h", "ComM_MemMap.h", "MEMMAP"),
        ("ComM_EcuM.h", "ComM_MemMap.h", "MEMMAP"),
        ("Dcm.h", "Dcm_MemMap.h", "MEMMAP"),
        ("Dcm_ComM.h", "Dcm_MemMap.h", "MEMMAP"),
        ("Det.h", "Det_MemMap.h", "MEMMAP"),
        ("LSduR.h", "LSduR_MemMap.h", "MEMMAP"),
        ("LSduR_CanIf.h", "LSduR_MemMap.h", "MEMMAP"),
        ("LSduR_CanTp.h", "LSduR_MemMap.h", "MEMMAP"),
        ("LSduR_PduR.h", "LSduR_MemMap.h", "MEMMAP"),
        ("PduR.h", "PduR_MemMap.h", "MEMMAP"),
        ("PduR_CanIf.h", "PduR_MemMap.h", "MEMMAP"),
        ("PduR_CanTp.h", "PduR_MemMap.h", "MEMMAP"),
        ("PduR_Com.h", "PduR_MemMap.h", "MEMMAP"),
        ("PduR_Dcm.h", "PduR_MemMap.h", "MEMMAP"),
        ("Can_Internal.h", "Can_MemMap.h", "MEMMAP"),
        ("CanTp_Internal.h", "CanTp_MemMap.h", "MEMMAP"),
        ("Com_Internal.h", "Com_MemMap.h", "MEMMAP"),
        ("ComM_Internal.h", "ComM_MemMap.h", "MEMMAP"),
        ("Det_Host.h", "Det_MemMap.h", "MEMMAP"),
        ("Rte_Main.h", "Rte_MemMap.h", "MEMMAP"),
        ("Det_Externals.h", "Std_Types.h", "STD_TYPES"),
        ("Det_Externals.h", "Det_MemMap.h", "MEMMAP"),
        ("Det_Callouts.c", "Det_Externals.h", "DET"),
        ("Det_Callouts.c", "Det_MemMap.h", "MEMMAP"),
        ("SchM_BswM.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Can.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_CanIf.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_CanTp.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Com.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_ComM.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Dcm.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Det.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_LSduR.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_PduR.h", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_CanIf_Host.c", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Mode_Host.c", "Rte_MemMap.h", "MEMMAP"),
        ("SchM_Diagnostic_Host.c", "Rte_MemMap.h", "MEMMAP"),
        ("CanIf_Cfg.h", "Rte_MemMap.h", "MEMMAP"),
        ("PduR_Cfg.h", "Rte_MemMap.h", "MEMMAP"),
        ("LSduR_Cfg.h", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_Com.h", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_Ingress.h", "Rte_MemMap.h", "MEMMAP"),
        ("Rte_Process.h", "Rte_MemMap.h", "MEMMAP"),
        ("Can.h", "Std_Types.h", "STD_TYPES"),
        ("CanIf.c", "Can.h", "CAN"),
        ("CanIf.c", "CanIf_MemMap.h", "MEMMAP"),
        ("CanIf.c", "Det.h", "DET"),
        ("CanIf.c", "LSduR_CanIf.h", "LSDUR"),
        ("CanIf.c", "SchM_CanIf.h", "RTE"),
        ("CanIf.h", "Can_GeneralTypes.h", "CAN"),
        ("CanIf.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("CanIf.h", "Std_Types.h", "STD_TYPES"),
        ("CanTp.c", "CanTp_MemMap.h", "MEMMAP"),
        ("CanTp.c", "Det.h", "DET"),
        ("CanTp.c", "LSduR_CanTp.h", "LSDUR"),
        ("CanTp.c", "PduR_CanTp.h", "PDUR"),
        ("CanTp.c", "SchM_CanTp.h", "RTE"),
        ("CanTp.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("CanTp.h", "Std_Types.h", "STD_TYPES"),
        ("Can_CfgTypes.h", "Platform_Types.h", "PLATFORM"),
        ("Can_GeneralTypes.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Com.c", "Com_MemMap.h", "MEMMAP"),
        ("Com.c", "SchM_Com.h", "RTE"),
        ("Com.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Com.h", "Std_Types.h", "STD_TYPES"),
        ("ComM.c", "BswM_ComM.h", "BSWM"),
        ("ComM.c", "ComM_MemMap.h", "MEMMAP"),
        ("ComM.c", "SchM_ComM.h", "RTE"),
        ("ComM.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("ComM.h", "Rte_ComM_Type.h", "RTE"),
        ("Dcm.c", "ComM_Dcm.h", "COMM"),
        ("Dcm.c", "Dcm_MemMap.h", "MEMMAP"),
        ("Dcm.c", "PduR_Dcm.h", "PDUR"),
        ("Dcm.c", "SchM_Dcm.h", "RTE"),
        ("Dcm.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Dcm.h", "Std_Types.h", "STD_TYPES"),
        ("Dcm_ComM.h", "Std_Types.h", "STD_TYPES"),
        ("Ecu_ProfileConfig.h", "CanIf_Cfg.h", "CANIF"),
        ("Ecu_ProfileConfig.h", "Can_Cfg.h", "CAN"),
        ("Ecu_ProfileConfig.h", "Std_Types.h", "STD_TYPES"),
        ("Os.c", "Os_MemMap.h", "MEMMAP"),
        ("Os.h", "Os_MemMap.h", "MEMMAP"),
        ("Os.h", "Rte_Os_Type.h", "RTE"),
        ("Os_Arti.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Arti.h", "Arti.h", "ARTI"),
        ("Os_Arti.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_ArtiServices.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Backend.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Backend.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Cfg.h", "Std_Types.h", "STD_TYPES"),
        ("Os_Error.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Hooks.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Host.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_HostLinux.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_HostLinux.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_HostServices.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Interrupt.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Mailbox.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Mailbox.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Schedule.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Stack.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Stack.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_StackLinux.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Target.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Time.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Time.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_Types.h", "Rte_Os_Type.h", "RTE"),
        ("Os_Types.h", "Std_Types.h", "STD_TYPES"),
        ("Os_Vector.c", "Os_MemMap.h", "MEMMAP"),
        ("Os_Vector.h", "Os_MemMap.h", "MEMMAP"),
        ("Os_PortTypes.h", "Platform_Types.h", "PLATFORM"),
        ("PduR.c", "Det.h", "DET"),
        ("PduR.c", "PduR_MemMap.h", "MEMMAP"),
        ("PduR.c", "SchM_PduR.h", "RTE"),
        ("PduR.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("PduR.h", "Std_Types.h", "STD_TYPES"),
        ("PduR_CanTp.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Rte.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Rte.h", "Std_Types.h", "STD_TYPES"),
        ("Rte_Cfg.h", "Std_Types.h", "STD_TYPES"),
        ("Rte_Com.h", "ComStack_Types.h", "COMSTACKTYPE"),
        ("Rte_ComM_Type.h", "Std_Types.h", "STD_TYPES"),
        ("Rte_Dcm_Type.h", "Rte_Type.h", "RTE"),
        ("Rte_Dcm_Type.h", "Std_Types.h", "STD_TYPES"),
        ("Rte_Ingress_Type.h", "Rte_Dcm_Type.h", "DCM"),
        ("Rte_Os.h", "Os.h", "OS"),
        ("Rte_Process_Type.h", "Rte_Dcm_Type.h", "DCM"),
        ("Rte_Type.h", "Std_Types.h", "STD_TYPES"),
        ("SchM_BswM.h", "BswM_MemMap.h", "MEMMAP"),
        ("SchM_Can.h", "Can_MemMap.h", "MEMMAP"),
        ("SchM_CanTp.h", "CanTp_MemMap.h", "MEMMAP"),
        ("SchM_Dcm.h", "Dcm_MemMap.h", "MEMMAP"),
        ("SchM_Dcm.h", "Rte_Dcm_Type.h", "DCM"),
        ("SchM_Dcm_Type.h", "Rte_Dcm_Type.h", "DCM"),
        ("Std_Types.h", "Platform_Types.h", "PLATFORM"),
    ];
    for (consumer, dependency, prefix) in edges {
        if cfg!(windows)
            && ["Os_HostLinux.h", "Os_HostLinux.c", "Os_StackLinux.c"].contains(&consumer)
        {
            continue;
        }
        if !cfg!(windows) && ["Os_Windows.h", "Os_HostWindows.c", "Os_Stack.c"].contains(&consumer)
        {
            continue;
        }
        let (path, bytes) = files
            .iter()
            .find(|(path, _)| path.ends_with(&format!("/{consumer}")))
            .unwrap_or_else(|| panic!("missing consumer {consumer}"));
        let text = std::str::from_utf8(bytes).unwrap();
        for part in ["MAJOR", "MINOR"] {
            for absent in [true, false] {
                let name = format!("{prefix}_AR_RELEASE_{part}_VERSION");
                let poison = if absent {
                    format!("#undef {name}\n")
                } else {
                    format!("#undef {name}\n#define {name} 99U\n")
                };
                let source = isolated(text, consumer, dependency, prefix, &poison);
                let label = format!("{consumer}-{dependency}-{part}-{absent}");
                let diagnostic = import_diagnostic(consumer, dependency, absent);
                compile(
                    &scratch.0,
                    &scratch.0.join(path),
                    &source,
                    &label,
                    false,
                    &diagnostic,
                );
            }
        }
        let poison = format!(
            "#undef {prefix}_AR_RELEASE_REVISION_VERSION\n#define {prefix}_AR_RELEASE_REVISION_VERSION 99U\n#undef {prefix}_VENDOR_ID\n#define {prefix}_VENDOR_ID 43U\n#undef {prefix}_SW_MAJOR_VERSION\n#define {prefix}_SW_MAJOR_VERSION 7U\n"
        );
        let source = isolated(text, consumer, dependency, prefix, &poison);
        compile(
            &scratch.0,
            &scratch.0.join(path),
            &source,
            "compatible-external-revision-vendor-software",
            true,
            "",
        );
    }
}

#[test]
fn public_headers_publish_their_owner_release_in_ecu_and_contract_deliveries() {
    let plan = build(&inputs()).unwrap();
    let ecu = prepared_pipeline_files_for_target(&plan, native_delivery_target());
    let contract = plan.component_contract_files().unwrap();
    for files in [&ecu[..], contract.files()] {
        let scratch = Scratch::new();
        install(files, &scratch.0);
        for (path, _) in files {
            if !path.ends_with(".h") || path.ends_with("_MemMap.h") {
                continue;
            }
            let name = Path::new(path).file_name().unwrap().to_str().unwrap();
            if path.starts_with("integration/") && name != "Det_Externals.h" {
                continue;
            }
            let prefix = if name.starts_with("SchM_") || name.starts_with("Rte_") || name == "Rte.h"
            {
                if name == "Rte_Dcm_Type.h" {
                    "DCM"
                } else {
                    "RTE"
                }
            } else if name.starts_with("ComStack_") {
                "COMSTACKTYPE"
            } else if name == "Std_Types.h" {
                "STD_TYPES"
            } else if name == "Platform_Types.h" {
                "PLATFORM"
            } else {
                let owner = name.split(['_', '.']).next().unwrap();
                match owner {
                    "Can" => "CAN",
                    "CanIf" => "CANIF",
                    "CanTp" => "CANTP",
                    "Com" => "COM",
                    "ComM" => "COMM",
                    "BswM" => "BSWM",
                    "Dcm" => "DCM",
                    "PduR" => "PDUR",
                    "LSduR" => "LSDUR",
                    "Det" => "DET",
                    "Os" => "OS",
                    "Arti" => "ARTI",
                    "MemMap" => "MEMMAP",
                    _ => continue, // Actual host helpers have no AUTOSAR module identity.
                }
            };
            // Fixed MOD DOC-REVISION literals; independent of the producer and ABI catalog.
            let source = format!(
                "#include \"{path}\"\n#if !defined({prefix}_AR_RELEASE_MAJOR_VERSION) || !defined({prefix}_AR_RELEASE_MINOR_VERSION) || !defined({prefix}_AR_RELEASE_REVISION_VERSION)\n#error Missing owner release\n#endif\n#if ({prefix}_AR_RELEASE_MAJOR_VERSION != 4U) || ({prefix}_AR_RELEASE_MINOR_VERSION != 10U) || ({prefix}_AR_RELEASE_REVISION_VERSION != 0U)\n#error Wrong owner release\n#endif\n"
            );
            let (major, minor) = if prefix == "ARTI" { (1, 0) } else { (0, 1) };
            let source = format!(
                "{source}#if !defined({prefix}_SW_MAJOR_VERSION) || !defined({prefix}_SW_MINOR_VERSION) || !defined({prefix}_SW_PATCH_VERSION)\n#error Missing owner software version\n#endif\n#if ({prefix}_SW_MAJOR_VERSION != {major}U) || ({prefix}_SW_MINOR_VERSION != {minor}U) || ({prefix}_SW_PATCH_VERSION != 0U)\n#error Wrong owner software version\n#endif\n"
            );
            let source = if name == "Arti.h" {
                format!(
                    "{source}#if ARTI_MODULE_ID != 5U\n#error Wrong ARTI module ID\n#endif\nint release_consumer;\n"
                )
            } else {
                format!("{source}int release_consumer;\n")
            };
            compile(
                &scratch.0,
                &scratch.0.join(path),
                &source,
                "first-include-owner-release",
                true,
                "",
            );
        }
        // Identity guards must leave every START/STOP transition live on every inclusion.
        for (path, _) in files.iter().filter(|(path, _)| path.ends_with("_MemMap.h")) {
            let name = Path::new(path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_end_matches("_MemMap.h");
            let scope = if name == "Rte" {
                "RTE_Rte".to_owned()
            } else if [
                "Can",
                "CanIf",
                "CanTp",
                "Com",
                "ComM",
                "BswM",
                "Dcm",
                "Det",
                "PduR",
                "LSduR",
                "Os",
                "Ecu_HostBusSM",
            ]
            .contains(&name)
            {
                name.to_uppercase()
            } else {
                name.to_owned()
            };
            let mut source = String::new();
            for _ in 0..2 {
                source.push_str(&format!("#define {scope}_START_SEC_CODE\n#include \"{path}\"\n#ifdef {scope}_START_SEC_CODE\n#error Unconsumed start\n#endif\n#define {scope}_STOP_SEC_CODE\n#include \"{path}\"\n#ifdef {scope}_STOP_SEC_CODE\n#error Unconsumed stop\n#endif\n"));
            }
            source.push_str("#if MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U || MEMMAP_AR_RELEASE_MINOR_VERSION != 10U || MEMMAP_AR_RELEASE_REVISION_VERSION != 0U\n#error Wrong MemMap release\n#endif\n#if MEMMAP_SW_MAJOR_VERSION != 0U || MEMMAP_SW_MINOR_VERSION != 1U || MEMMAP_SW_PATCH_VERSION != 0U\n#error Wrong MemMap software version\n#endif\nint mapped_consumer;\n");
            compile(
                &scratch.0,
                &scratch.0.join(path),
                &source,
                "repeated-memory-mapping-identity",
                true,
                "",
            );
        }
    }
}

#[test]
fn comm_source_checks_its_own_header_release_independently() {
    // Independent source/header release values; software is checked separately below.
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    let path = scratch.0.join("bsw/src/ComM.c");
    let original = std::fs::read_to_string(&path).unwrap();
    compile(&scratch.0, &path, &original, "comm-control", true, "");
    let position = original.find("/* SWS_ComM_00418:").unwrap();
    for (part, current, next) in [("MAJOR", 4, 5), ("MINOR", 10, 11), ("REVISION", 0, 1)] {
        let name = format!("COMM_AR_RELEASE_{part}_VERSION");
        for missing in [false, true] {
            let change = if missing {
                format!("#undef {name}\n")
            } else {
                format!("#undef {name}\n#define {name} {next}U\n")
            };
            let source = format!("{}{change}{}", &original[..position], &original[position..]);
            compile(
                &scratch.0,
                &path,
                &source,
                "comm-header-only-change",
                false,
                if missing {
                    "ComM.c -> ComM.h: missing internal AUTOSAR release"
                } else {
                    "ComM.c -> ComM.h: inconsistent internal AUTOSAR release"
                },
            );
            if !missing {
                let macro_name = format!("COMM_C_AR_RELEASE_{part}_VERSION");
                let line = source
                    .lines()
                    .find(|line| line.split_whitespace().nth(1) == Some(macro_name.as_str()))
                    .unwrap();
                assert_eq!(
                    line.split_whitespace().nth(2),
                    Some(format!("{current}U").as_str())
                );
                let source = source.replace(line, &format!("#define {macro_name} {next}U"));
                compile(
                    &scratch.0,
                    &path,
                    &source,
                    "comm-source-header-upgrade",
                    true,
                    "",
                );
            }
        }
    }
}

#[test]
fn excluded_consumers_do_not_acquire_external_release_obligations() {
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    // R24-11 Det_NA_00999, LSduR_NA and Comtype_NA exclude SRS_BSW_00004.
    for (consumer, dependency, prefix) in [
        ("bsw/src/Det.c", "bsw/include/Rte_Version.h", "RTE"),
        ("bsw/src/LSduR.c", "bsw/include/CanIf_Version.h", "CANIF"),
        ("bsw/include/Det.h", "bsw/include/Std_Types.h", "STD_TYPES"),
        (
            "bsw/include/LSduR.h",
            "bsw/include/Std_Types.h",
            "STD_TYPES",
        ),
        (
            "bsw/include/ComStack_Types.h",
            "bsw/include/Std_Types.h",
            "STD_TYPES",
        ),
        (
            "generated/include/ComStack_Cfg.h",
            "bsw/include/Platform_Types.h",
            "PLATFORM",
        ),
    ] {
        let path = scratch.0.join(dependency);
        let original = std::fs::read_to_string(&path).unwrap();
        let changed = original
            .lines()
            .filter(|line| {
                !["MAJOR", "MINOR"].iter().any(|part| {
                    line.split_whitespace().nth(1)
                        == Some(format!("{prefix}_AR_RELEASE_{part}_VERSION").as_str())
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_ne!(changed, original);
        std::fs::write(&path, changed).unwrap();
        let source = if consumer.ends_with(".h") {
            format!("#include \"{consumer}\"\nint excluded_consumer;\n")
        } else {
            std::fs::read_to_string(scratch.0.join(consumer)).unwrap()
        };
        compile(
            &scratch.0,
            &scratch.0.join(consumer),
            &source,
            "excluded-consumer-missing-import-release",
            true,
            "",
        );
        std::fs::write(path, original).unwrap();
    }
}

#[test]
fn comm_source_checks_each_software_component_and_accepts_synchronized_upgrades() {
    // Verifies: SWS_ComM_00418.
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    let path = scratch.0.join("bsw/src/ComM.c");
    let original = std::fs::read_to_string(&path).unwrap();
    compile(
        &scratch.0,
        &path,
        &original,
        "comm-software-control",
        true,
        "",
    );
    let position = original.find("/* SWS_ComM_00418: source software").unwrap();
    for (part, current, next) in [("MAJOR", 0, 1), ("MINOR", 1, 2), ("PATCH", 0, 1)] {
        let name = format!("COMM_SW_{part}_VERSION");
        for missing in [false, true] {
            let change = if missing {
                format!("#undef {name}\n")
            } else {
                format!("#undef {name}\n#define {name} {next}U\n")
            };
            let source = format!("{}{change}{}", &original[..position], &original[position..]);
            compile(
                &scratch.0,
                &path,
                &source,
                "comm-software-header-only",
                false,
                if missing {
                    "ComM.c -> ComM.h: missing internal software version"
                } else {
                    "ComM.c -> ComM.h: inconsistent internal software version"
                },
            );
            if !missing {
                let macro_name = format!("COMM_C_SW_{part}_VERSION");
                let line = source
                    .lines()
                    .find(|line| line.split_whitespace().nth(1) == Some(macro_name.as_str()))
                    .unwrap();
                assert_eq!(
                    line.split_whitespace().nth(2),
                    Some(format!("{current}U").as_str())
                );
                let synchronized = source.replace(line, &format!("#define {macro_name} {next}U"));
                compile(
                    &scratch.0,
                    &path,
                    &synchronized,
                    "comm-software-synchronized",
                    true,
                    "",
                );
            }
        }
    }
    // A legal major upgrade also resets the lower components (SWS_BSW_00162).
    let owner = scratch.0.join("bsw/include/Autsaro_Version.h");
    let upgraded = std::fs::read_to_string(&owner)
        .unwrap()
        .replace("AUTSARO_SW_MAJOR_VERSION 0U", "AUTSARO_SW_MAJOR_VERSION 1U")
        .replace("AUTSARO_SW_MINOR_VERSION 1U", "AUTSARO_SW_MINOR_VERSION 0U");
    std::fs::write(owner, upgraded).unwrap();
    let synchronized = original
        .lines()
        .map(|line| match line.split_whitespace().nth(1) {
            Some("COMM_C_SW_MAJOR_VERSION") => "#define COMM_C_SW_MAJOR_VERSION 1U",
            Some("COMM_C_SW_MINOR_VERSION") => "#define COMM_C_SW_MINOR_VERSION 0U",
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    compile(
        &scratch.0,
        &path,
        &synchronized,
        "comm-software-major-upgrade",
        true,
        "",
    );
}

#[test]
fn arti_software_api_observes_each_owner_component_and_each_assignment() {
    // Software portion of SWS_Arti_91005; formal vendor identity remains unresolved.
    let files =
        prepared_pipeline_files_for_target(&build(&inputs()).unwrap(), tooling::native_target());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    let implementation_path = scratch.0.join("os/src/Arti.c");
    let source_path = scratch.0.join("os/include/Arti.h");
    let source = std::fs::read_to_string(&source_path).unwrap();
    let header_path = scratch.0.join("os/include/Arti_Version.h");
    let header = std::fs::read_to_string(&header_path).unwrap();
    let target: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.0.join("target.json")).unwrap()).unwrap();
    let run = |expected: [u8; 3], accepted: bool| {
        let consumer = scratch.0.join("software-consumer.c");
        std::fs::write(
            &consumer,
            format!(
                r#"#include "Arti.h"
int main(void) {{
    Std_VersionInfoType result = {{99u, 99u, 99u, 99u, 99u}};
    void (*get_version)(Std_VersionInfoType *) = &Arti_GetVersionInfo;
    get_version(&result);
    if (result.vendorID != 0u || result.moduleID != 5u) {{ return 10; }}
    if (result.sw_major_version != {}u) {{ return 11; }}
    if (result.sw_minor_version != {}u) {{ return 12; }}
    if (result.sw_patch_version != {}u) {{ return 13; }}
    return 0;
}}
"#,
                expected[0], expected[1], expected[2]
            ),
        )
        .unwrap();
        let executable = scratch.0.join(if cfg!(windows) {
            "software-consumer.exe"
        } else {
            "software-consumer"
        });
        let mut command = tooling::c_compiler();
        command.current_dir(&scratch.0);
        for flag in target["compilerFlags"].as_array().unwrap() {
            command.arg(flag.as_str().unwrap());
        }
        for path in target["includePaths"].as_array().unwrap() {
            command.arg("-I").arg(path.as_str().unwrap());
        }
        command
            .arg(&implementation_path)
            .arg(&consumer)
            .arg("-o")
            .arg(&executable);
        let output = tooling::run_public_command(
            &mut command,
            &scratch.0,
            "arti-software-build",
            std::time::Duration::from_secs(60),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = tooling::run_public_command(
            &mut std::process::Command::new(executable),
            &scratch.0,
            "arti-software-observe",
            std::time::Duration::from_secs(30),
        );
        assert_eq!(output.status.success(), accepted, "{output:?}");
        output.status.code()
    };
    run([1, 0, 0], true);
    for (index, part, current, next) in [(0, "MAJOR", 1, 2), (1, "MINOR", 0, 1), (2, "PATCH", 0, 1)]
    {
        let name = format!("ARTI_SW_{part}_VERSION");
        let line = header
            .lines()
            .find(|line| line.split_whitespace().nth(1) == Some(name.as_str()))
            .unwrap();
        assert_eq!(
            line.split_whitespace().nth(2),
            Some(format!("{current}U").as_str())
        );
        std::fs::write(
            &header_path,
            header.replace(line, &format!("#define {name} {next}U")),
        )
        .unwrap();
        let mut expected = [1, 0, 0];
        expected[index] = next;
        run(expected, true);
        std::fs::write(&header_path, &header).unwrap();
        let assignment = format!("versioninfo->sw_{}_version = {name};", part.to_lowercase());
        assert_eq!(source.matches(&assignment).count(), 1);
        std::fs::write(&source_path, source.replace(&assignment, "")).unwrap();
        assert_eq!(run([1, 0, 0], false), Some(11 + index as i32));
        std::fs::write(&source_path, &source).unwrap();
    }
}

#[test]
fn shared_software_owner_propagates_independent_component_upgrades() {
    let files = prepared_pipeline_files(&build(&inputs()).unwrap());
    let scratch = Scratch::new();
    install(&files, &scratch.0);
    let owner = scratch.0.join("bsw/include/Autsaro_Version.h");
    let original = std::fs::read_to_string(&owner).unwrap();
    let headers = [
        ("Can_Version.h", "CAN"),
        ("CanIf_Version.h", "CANIF"),
        ("CanTp_Version.h", "CANTP"),
        ("Com_Version.h", "COM"),
        ("ComM_Version.h", "COMM"),
        ("BswM_Version.h", "BSWM"),
        ("Dcm_Version.h", "DCM"),
        ("Det_Version.h", "DET"),
        ("PduR_Version.h", "PDUR"),
        ("LSduR_Version.h", "LSDUR"),
        ("Rte_Version.h", "RTE"),
        ("MemMap_Version.h", "MEMMAP"),
        ("ComStack_Version.h", "COMSTACKTYPE"),
        ("Os_Version.h", "OS"),
        ("Std_Types.h", "STD_TYPES"),
        ("Platform_Types.h", "PLATFORM"),
    ];
    for values in [[1, 1, 0], [0, 2, 0], [0, 1, 1], [1, 0, 0]] {
        let mut changed = original.clone();
        for (part, value) in ["MAJOR", "MINOR", "PATCH"].into_iter().zip(values) {
            let name = format!("AUTSARO_SW_{part}_VERSION");
            let line = original
                .lines()
                .find(|line| line.split_whitespace().nth(1) == Some(name.as_str()))
                .unwrap();
            changed = changed.replace(line, &format!("#define {name} {value}U"));
        }
        std::fs::write(&owner, changed).unwrap();
        let mut consumer = String::new();
        for (header, prefix) in headers {
            consumer.push_str(&format!("#include \"{header}\"\n"));
            for (part, value) in ["MAJOR", "MINOR", "PATCH"].into_iter().zip(values) {
                consumer.push_str(&format!("#if {prefix}_SW_{part}_VERSION != {value}U\n#error {prefix} software {part} did not follow owner\n#endif\n"));
            }
        }
        consumer.push_str("int software_consumer;\n");
        compile(
            &scratch.0,
            &owner,
            &consumer,
            "shared-software-upgrade",
            true,
            "",
        );
    }
}
