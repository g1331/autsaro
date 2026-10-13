# 0015. 仓库代码组织

## 背景

2026-10-11 时仓库存在以下问题：开发计划编号成为产品身份（交付身份、bin 名、`include_str!` 引用 `core/tests/fixtures/epic4/`、`core/tests/support/epic4_*`／`epic7_*` 等，共 271 行引用）；`runtime/multi`、`integration/multi_*.rs` 用“multi”区分新旧实现而非职责；`core/` 与 `src-tauri/` 各有独立 Cargo manifest；部分测试与预期做成 CLI 子命令（如 `os_suites.py`、`input_oracles`、`protocol_oracles`、`bsw_catalog`、`probe`），绕开测试框架；Windows 桌面隔离验收自建了 WindowStation、token、elevate、WFP 等约十个 Win32 模块；超大测试文件单独链接耗时。

## 决策

1. **根目录 Cargo workspace**：虚拟 manifest，`members = ["core", "src-tauri"]`，`resolver = "3"`，唯一 `Cargo.lock` 与 `target`。`core`、`src-tauri` 不搬到 `crates/`，`src-tauri` 位置与名称不变；边界稳定前不拆新 crate。
2. **产品资源**放在 `core/resources/`，产品代码不引用测试目录；`build.rs` 只写 `OUT_DIR`。
3. **`core/tests` 收成三个测试二进制**：`it/main.rs`（普通）、`native/main.rs`（`required-features = ["native-tests"]`）、`official/main.rs`（`required-features = ["official-oracles"]`），内部按能力分模块；共享辅助放 `support/`，输入与期望按能力放 `fixtures/<能力>/`。CI 按模块过滤（如 `--test native -- runtime_contracts::`）。
4. **运行时 BSW 扁平放在 `runtime/bsw/`**：`include/` 放标准公共头与 `<Mip>_MemMap.h`，`src/` 放 `<Mip>.c` 与私有头 `<Mip>_Internal.h`；`runtime/os`、`runtime/ecu`、`runtime/host` 保留，`host` 只放主机适配。`_Cfg`／`_PBcfg` 只出现在生成工程。模块数接近 30 时再考虑按功能域分层。
5. **去编号与按职责命名**：产品代码、测试、fixture、工具、bin、目录名、测试名与读者文档不出现开发计划编号，由 `quality` 检查强制；不用 multi、legacy、new 等相对词。标准模块文件、头文件和内存段名称按 SWS BSW General 与 RTE 保持不变（规范未规定目录结构，只改目录与项目私有名称）。
6. **工具只分三类**：交付工具（`ecu_tools`）；统一开发 CLI（`doctor`、`verify`、`quality`、`c-check`、`assets`）；放在 cargo test、unittest、vitest 或 node:test 中的测试，以 feature／标记区分运行条件。桌面端到端优先使用 Tauri 官方 WebDriver 与 runner 级隔离，替换自建 Win32 隔离设施。
7. **格式版本策略**：只有持久化或交付格式带版本，并登记在格式版本表中；模板 ID 等内部标识不带版本。

依据（2026-10-11 调研）：Cargo workspace 只认根目录锁文件与 target，`[profile]`／`[patch]` 只在根 manifest 生效，虚拟 manifest 须显式写 `resolver`（[Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)）；Tauri 2 允许 `src-tauri` 作为 workspace 成员（[Tauri Project Structure](https://v2.tauri.app/start/project-structure/)）；多文件集成测试写成 `tests/<name>/main.rs`，`required-features` 控制按 feature 编译的测试目标（[Cargo Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)、[Cargo Targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)）；SWS BSW General R24-11 不规定目录结构，但要求实现源包含 `<Mip>_MemMap.h`（`[SWS_BSW_00006]`）、未列出的实现文件名也须遵守 `[SWS_BSW_00103]`、文件名区分大小写且不能只靠大小写区分（`[SWS_BSW_00170]`／`[SWS_BSW_00171]`）。

## 后果

- 一次 `cargo` 构建与测试覆盖两个 crate；测试链接次数减少。
- 迁移涉及原生 fixture、交付布局与测试共用路径辅助：合并前在本地 Linux 与 Windows 运行受影响原生套件，并请维护者触发 acceptance；CI、文档与 AGENTS.md 同步改名。
- 不做：把超大测试文件拆成许多顶层 `tests/*.rs`；合并后保留子目录 `Cargo.lock`；为整齐而拆 crate；建只有一两个模块的分层空目录；让 golden 输出自动覆盖。

## 状态

已接受（2026-10-11），实施中：决策 1、5、7 及 `runtime/bsw` 已实施（#36），`runtime/host` 拆分随 #38。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（第 1 节、决定 5、7、8）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-6、AD-9、Structural Seed 与组织依据）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（B4、B5）。
