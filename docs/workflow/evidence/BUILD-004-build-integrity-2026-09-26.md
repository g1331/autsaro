# BUILD-004：工作台构建输入完整性证据（2026-09-26）

## 组合、预期与实现

- 基线 `dad392b`，分支 `feature/BUILD-004-build-integrity`。CP/FO R24-11、Windows 主机虚拟 ECU、MinGW GCC 16.1.0，无配置变体；`HOST-CAN-01` 的 11 位 Classical CAN 信号工程。输入由 `core/tests/end_to_end.rs` 的 `create_pair` 重建：Alpha 的 `Command` 帧为 CAN ID `0x321`、DLC 2、10 ms 周期，`SendCount` 为第 3 位起的 8-bit LSB0，初值 5。独立预期 `T 10` 得 `X 801 2 2800`。
- 07 号验收议题要求把生成工件与实际构建输入对应起来，这是项目质量门。本卡没有新增 AUTOSAR 配置或运行语义，因此没有新引入的 SWS/MOD 条款；既有 BSW、RTE 与目标行为的规范义务仍由能力档案逐项审查。
- `generator::build` 在调用 GCC 前检查清单格式和路径、列出文件及摘要、目录内额外文件，并在 GCC 成功后、安装正式二进制前再检查一次。变化时保留临时编译产物路径供诊断，不产生正式 `ecu_host.exe`。构建本身仍保留已有二进制拒绝策略。

## 可复现执行与实际结果

- 本机工具链：Cargo/Rust 1.98.1 MSVC，MSYS2 GCC 16.1.0，Node 24.19.0，npm 11.17.0，Python 3.12.9。Cargo 前设置 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`，路径已核对存在。
- `cargo test --manifest-path core/Cargo.toml --test end_to_end build_rejects_changed_generated_inputs_before_compiling -- --nocapture`：1/1 通过。每次由相同 ARXML 配置生成新目录，分别在构建前改动 `Ecu_Config.c`、`files.list`，增加 `src/Owner.c`，删除 `include/Can.h`；四种情况均返回“拒绝构建”，未创建正式二进制，原目标文件状态保持不变。
- `python scripts/workflow.py verify --scope all`：退出码 0；工作流测试 8/8、`npm ci`、UI TypeScript/Vite 构建、核心单元 2/2、端到端 45/45、Tauri debug 构建通过。既有 `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults` 与 `generated_handoff_builds_and_runs_after_moving_without_the_workbench` 等用例验证正常生成工程仍由 GCC 编译链接并运行；后者仍观察到 `T 10` 对应 `X 801 2 2800`。`python scripts/workflow.py check`：状态有效。
- Rust 链接仍报告既有 `LNK4098` 默认库冲突警告；本轮未据此声称静态质量通过。

## 边界与未验证

- 构建前后两次核对缩小外部误改窗口，但未提供编译器运行期间逐文件不可变快照，也不能阻止有人同时改写文件和摘要记录；`files.sha256` 不是签名或恶意篡改防护。
- 本轮未启动或操纵维护者桌面，未在隔离桌面运行原生 Tauri UI；界面“构建”按钮到后端拒绝提示的实际窗口流程未验证。离线 `build.ps1` 是独立交付路径，不运行本次工作台完整性检查；第三方协议栈、真实 MCU/板卡及适用规范义务仍未验证。
- `HOST-CAN-01` 维持 `documented_behavior`，六道证据门均不因本卡升级。

## 独立审查

待新 Agent 会话审查源码、任务范围、失败路径和证据后记录结论。
