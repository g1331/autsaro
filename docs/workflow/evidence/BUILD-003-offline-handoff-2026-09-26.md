# BUILD-003：生成工程离线构建交接证据（2026-09-26）

## 组合与独立预期

- 基线 `0f7f474`，分支 `feature/BUILD-003-offline-handoff`。CP/FO R24-11；Windows 主机虚拟 ECU、MinGW GCC 16.1.0；无配置变体。`HOST-CAN-01` 的无诊断信号子集是本轮独立构建与运行组合。`core/tests/end_to_end.rs` 的 `create_pair` 重建输入：Alpha Tx `Command` ID `0x321`、DLC 2、周期 10 ms，8-bit LSB0 `SendCount` 从第 3 位开始、初值 5。
- 独立预期：初值 5 左移 3 位为 `0x28`，第二字节为 0；输入 `T 10` 应输出 `X 801 2 2800`。工程移到带空格的路径后，脚本仅引用自身目录即可编译；若二进制已有用户字节，脚本须拒绝覆盖。
- 任务依据是 07 号验收议题的用户离线交付质量门；未新增 AUTOSAR ECUC 或 BSW 行为义务。现有主机信号和诊断规范义务仍须按能力档案分别审查。

## 实施与可复现结果

- `core/src/generator.rs` 把 `runtime/generated-README.md` 与 `runtime/generated-build.ps1` 纳入生成文件，按配置填入无诊断、DTC、0x27 或两者同时启用的启动命令。两文件进入预览、`files.list` 和 `files.sha256`。UI 生成结果指向目录内说明。
- `generated_handoff_builds_and_runs_after_moving_without_the_workbench` 生成 Alpha 后把工程移动到临时目录 `Delivered ECU with spaces`，从另一工作目录执行 README 中的 Windows PowerShell 命令，将占位符替换为实际交付路径。GCC C99 编译链接退出 0；新二进制输入 `T 10` 后得到 `X 801 2 2800`。将二进制改为 `owner binary` 后再执行脚本，非零退出且字节不变。测试临时目录自动清理，以测试名重建。
- `regeneration_preserves_user_edits_to_generated_files` 将说明、脚本逐一改为用户字节，再生成均拒绝并保留字节。既有生成预览测试逐项比较预览与确认后的文件，覆盖新增文件。DTC、0x27、DTC＋0x27 既有端到端用例分别核对 README 启动参数，但不替代无诊断离线运行证据。
- 工具链：Rust/Cargo 1.98.1 MSVC，Node 24.19.0，npm 11.17.0，Python 3.12.9，MSYS2 GCC 16.1.0。Cargo 前设置并核对 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`。
- 聚焦 `cargo test --manifest-path core/Cargo.toml --test end_to_end generated_handoff_builds_and_runs_after_moving_without_the_workbench -- --nocapture`：1/1 通过。`python scripts/workflow.py verify --scope all`：退出码 0，工作流测试 8/8、`npm ci`、UI TypeScript/Vite 构建、核心单元 2/2、端到端 44/44、Tauri debug 构建通过。`python scripts/workflow.py check`：状态有效。Rust 链接仍有既有 `LNK4098` 默认库冲突警告；未据此声称静态质量通过。

## 拒绝与未验证

- 已有 `ecu_host.exe` 不被脚本覆盖；手改说明或脚本后再生成被完整性规则拒绝。旧版生成目录缺少新增文件时，清单版本检查会拒绝原地覆盖，须选新空目录。`files.sha256` 只供工作台再生成时检测改动，不提供签名认证。
- 本轮未打开或操纵维护者桌面，也未在隔离桌面运行原生 Tauri UI；实际系统目录选择器到交付流程尚未验证。只在本机 Windows/GCC 验证离线构建，未在另一台机器、第三方协议栈、真实 MCU 或板卡执行。
- `HOST-CAN-01` 的六门状态和 `documented_behavior` 声明不升级。仍需外部跨引用多文件 ARXML 的用户全链、另一执行者离线交接，以及逐模块规范工件与静态义务核对。

## 独立审查

Agent `build003_review` 在独立会话读取任务、源码和证据，并复跑移动交付 1/1、再生成保护 1/1、安全访问 2/2、DTC 1/1 及工作流状态检查。审查发现 P2 问题：初版 README 的 `-File` 路径未加引号，含空格路径按说明执行会失败；初版测试通过 `Command::arg` 直接传路径，漏测文档命令。提交 `129b7b6` 将两种 PowerShell 示例的路径加引号，并改为从生成 README 读取示例、替换实际路径后经 PowerShell 执行。修复后完整 `verify --scope all` 再次退出 0，端到端 44/44、核心单元 2/2、UI 与 Tauri 构建通过。审查者独立复跑聚焦用例 1/1、`workflow.py check`，另用 `pwsh` 核对加引号后含空格路径作为单个参数传入；2026-09-26 复审确认 P2 已解决，本任务范围内无其他实质问题。审查者未独立重跑全量门禁或原生桌面流程。
