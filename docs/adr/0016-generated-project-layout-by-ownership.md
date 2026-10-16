# 0016. 生成工程按所有权布局并提供 CMake 描述

## 背景

当前封存工程把静态 BSW、生成配置、用户代码快照和适配层混放，构建只能经包内 `ecu-tool` 完成。商用与开源工具普遍把静态代码、可整体重写的生成目录与用户代码分开存放，并提供标准构建入口（如 Vector DaVinci Configurator Classic 的 `GenData`／`Templates`、EB tresos 的 `output/generated`、NXP S32DS 的 `generate/{include,src}`、openAUTOSAR classic-platform 的板级 `config/`、autoas/as 的 `GEN/`）。R24-11 Memory Mapping 规定 `{Mip}_MemMap.h` 与 `{componentTypeName}_MemMap.h` 由 Memory Mapping 按输入配置提供（`[SWS_MemMap_00032]`／`[SWS_MemMap_00029]`）。

## 决策

1. 交付包按所有权分目录，标准文件名不变：
   - `bsw/{include,src}/`：静态 BSW，只读，按构建时计算的资源身份校验；
   - `os/`、`kernel/`：静态 OS 适配与固定第三方内核；
   - `generated/{include,src}/`：全部配置生成物（`*_Cfg.h`、`*_PBcfg.c`、`Rte*`、`SchM_*`、`*_MemMap.h`、Ecu 配置），每次生成整体重写；
   - `generated/templates/`：SWC 骨架，不参与编译；
   - `app/<SWC>/`：用户代码快照（用户在 live 工程中修改，见 ADR 0007）；
   - `integration/`：主机／目标适配；
   - `tools/`、`inputs/`、`meta/`：离线复验工具、原始输入、台账与完整性清单。
2. `CMakeLists.txt` 与 `target.json` 从同一清单生成：前者是标准构建描述，后者是 `ecu-tool` 构建清单；二者构建同一组源文件。
3. 主机目标的 MemMap 头统一由生成方产出并归入 `generated/`；接入 MCU 目标时由集成层通过显式配置声明覆盖，不手改生成文件。

## 后果

- 重新生成后只有 `generated/`、`app/`、`meta/`、配置快照 `inputs/` 与根目录的 `README.md`、`target.json`、`CMakeLists.txt` 变化，审阅范围清楚；因此包内 `tools/ecu-tool.py` 不嵌入随配置变化的期望值，改由 `meta/seal-policy.json` 提供。
- 改变交付位置须同时核对交付路径、target sources／includes、headers、ownership／reopen 与封存工具，不只更新仓库路径；这是交付格式变化，按 ADR 0011 审阅并写入 CHANGELOG。
- CMake 与 `ecu-tool` 两个构建入口须在两个平台都通过，作为一致性证据。

## 状态

已接受（2026-10-11），已实施（2026-10-13，[#38](https://github.com/g1331/autsaro/issues/38)）。SWC MemMap 头（`[SWS_MemMap_00029]`）由 [#51](https://github.com/g1331/autsaro/issues/51) 承接。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（Structural Seed：生成工程布局）、`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（执行顺序第 7 步）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（B7）。

---
