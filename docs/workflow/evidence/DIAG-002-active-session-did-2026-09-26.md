# DIAG-002：活动会话 DID 主机证据（2026-09-26）

## 组合、输入和独立预期

基线 `b3ee5c9`，分支 `feature/DIAG-002-active-session-did`。CP/FO R24-11、无变体、Windows 主机虚拟 ECU、MSVC Rust 1.98.1、MSYS2 MinGW GCC 16.1.0。依据 [R24-11 Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) §7.4.2.2、`SWS_Dcm_00085`（PDF 页 112）：DSP 内部管理 `0xF186` 的读访问。本主机档案将其限定为已实现的默认会话 `01` 与扩展会话 `03`，不借此声明完整 Dcm 标准接口。

`core/tests/end_to_end.rs::active_session_did_reports_session_transitions_and_rejects_invalid_reads` 创建一个 Tx 帧、一个 32-bit 信号、`0x700/0x708` 物理诊断连接，配置 DID `0x1234` 与 S3=5000 ms。测试先确认不能把保留 `0xF186` 配为普通 DID；然后保存、重开、XSD/语义验证、生成 C99 工程并编译。生成后原 ARXML 字节未变化。独立预期按 SID `22` 的正响应 `62`、DID 高低字节 `F1 86` 和活动会话编号构成，测试不从生成器读取预期响应。

| 注入请求或状态变化 | 独立预期主机 CAN 响应 |
| --- | --- |
| 初始 `03 22 F1 86` | `X 1800 5 0462F18601` |
| 未配置 `03 22 F1 87` | `X 1800 4 037F2231` |
| 缺少 DID 低字节 | `X 1800 4 037F2213` |
| `02 10 03` 后读 `0xF186` | `X 1800 5 0462F18603` |
| `02 10 01` 后读 `0xF186` | `X 1800 5 0462F18601` |
| 再入扩展会话，推进虚拟时钟到 5001 ms 后读 | `X 1800 5 0462F18601` |

直接运行生成 ECU 的逐行协议断言全部通过；`host::run_diagnostic` 也分别验证默认、扩展、主动 `10 01` 切回及 S3 回退，且注入未知 DID 和错误长度请求、检查 NRC `0x31`/`0x13`，最终报告通过。测试器依据普通配置 DID 在 `0xF187/0xF188` 中选择未配置编号；测试另将普通 DID 改为 `0xF187`，重新保存、生成、编译并运行测试器，避免合法 DID 与拒绝探针碰撞。未配置诊断连接时没有 0x22 服务入口；既有纯信号工程行为未改变。

## 命令与结果

- 设置本机 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>` 后，定向运行 `cargo test --manifest-path core/Cargo.toml active_session_did_reports_session_transitions_and_rejects_invalid_reads -- --nocapture`：1/1 通过。
- `python scripts/workflow.py verify --scope all`：`git diff --check`、工作流测试 8/8、`npm ci`、UI TypeScript/Vite 构建、核心单元 2/2、端到端 48/48、Tauri 构建均通过。Cargo 链接输出既有 `LNK4098` 默认库冲突警告，未导致门禁失败。
- `python scripts/workflow.py check`：`Workflow state is valid.`
- 首次独立审查指出工作台测试器缺少主动 `10 01` 切回与两项拒绝场景，虽然端到端用例已经覆盖。补齐 `core/src/host.rs` 后，两条定向集成测试各 1/1 通过；再次执行上述完整 `verify --scope all`，工作流 8/8、核心单元 2/2、端到端 48/48、UI 与 Tauri 构建均通过。此修复仅扩展测试器请求及断言，没有改变生成 ECU 的运行分支。
- 第二次独立复核发现固定的未知 DID `0xF187` 会与合法配置 DID 碰撞。测试器改为按配置在 `0xF187/0xF188` 中选择，端到端增加 `0xF187` 配置回归；定向用例 1/1 通过。最终再次执行完整 `verify --scope all`：工作流 8/8、核心单元 2/2、端到端 48/48、UI 与 Tauri 构建均通过。

原生 Tauri 桌面窗口/IPC 本轮未在隔离桌面实际操作；UI 文字与调用入口由 TypeScript/Tauri 构建和共用后端诊断测试器覆盖。第三方 Dcm/Can、完整 ECUC/BSW 静态义务、目标 MCU、电气层和实机均未验证。六道能力证据门与支持等级保持原状态，不据本卡升级。

## 独立复核

首次只读审查由 Agent `diag002_review` 执行，指出测试器未覆盖任务卡所列的主动切回及错误请求；审查者重跑新用例和原有诊断连接用例，均 1/1 通过。第二次复核指出未知 DID 探针与合法配置碰撞，审查者另重跑安全写入用例 1/1 通过。两项意见均已修复。最终只读复核通过，审查者再次独立运行新定向用例 1/1 通过，`workflow.py check`、`git diff --check` 通过；审查者未重跑完整 `verify --scope all`，该门禁由开发侧在最终修改后执行。审查者确认未发现阻断本地集成的问题，未升级主机之外的支持声明。
