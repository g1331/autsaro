# STD-001 主机 Can Driver 接口切片：进行中证据

2026-09-27，分支 `fix/quality-check-findings`，任务基线 `caaf01e`。本记录只覆盖当前有界 Windows/MinGW 主机切片，不是 Can 模块或 HOST-CAN-01 的最终审查结论。

## 输入、接口与实际调用

- 输入为 `cargo run --manifest-path core/Cargo.toml --example quality_sample -- <独立临时目录>` 生成的单 Tx 信号 ECU；输出有 `files.list` 和 `files.sha256`。实际复核目录为 `<temporary-dir>/generated`。
- 本地 R24-11 Can Driver PDF 页 52–55、57、61、67、76 给出 `Can_ConfigType`、`Can_PduType`、`Can_HwHandleType`、状态类型及 `Can_Init`、`Can_SetControllerMode`、`Can_GetControllerMode`、`Can_Write` 的公开签名。当前实现为这些入口增加主机配置及类型；生成工程的 `Ecu_Init` 实际经 `Can_Init` 和 `Can_SetControllerMode` 初始化，Tx 路径经 `Can_Transmit` 主机包装层调用 `Can_Write`。
- 主机金向量路径 `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults` 通过。新增独立 C99 harness `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 通过，覆盖未初始化/停止、无效控制器与 HTH、空 PDU、错误 ID/DLC、有效发送、接收注入与 bus-off 拒绝。

## 已运行命令与结果

- `python scripts/workflow.py verify --scope all`：通过；脚本 15/15、核心单元 3/3、端到端 50/50、UI ESLint/构建、核心与桌面增量 Clippy、桌面构建均通过。该次运行在后续 Can.c 单出口与括号整理之前；整理后分别重跑上述两个核心定向测试及 `python scripts/quality.py --base caaf01e`，均通过。
- `python scripts/workflow.py verify --scope baseline --generated-dir <上述生成目录>`：全文件格式、Python Ruff、UI ESLint、两套全告警 Clippy 和 C API Doxygen 通过；BSW/RTE/生成 C 部分 MISRA 与逐能力规范证据分区仍失败。Can.c 整理后单独对 BSW 文件集重跑 Cppcheck，报告 327 条 MISRA 发现；其中新 Can.c 的多出口与优先级发现已消除。此扫描只覆盖部分规则，不证明 MISRA 符合。
- `python scripts/workflow.py check`：通过。

## 未闭合的标准义务

`Can_DeInit`、`Can_SetBaudrate`、中断控制、错误状态及其他适用服务/回调、线程安全的 `Can_Write`、完整 ECUC/BSWMD、MemMap 与逐规则 MISRA 处理尚未闭合。当前 `Can_ConfigType` 仍携带主机输出回调，`Can_Write` 在单线程虚拟目标同步执行；这不是第三方 CanIf/CAN ABI 或真实 MCU 证据。任务保持 `active`，HOST-CAN-01 仍为 `documented_behavior`，所有六道证据门维持原状态；后续须补标准工件、独立运行和新 Agent 复核。
