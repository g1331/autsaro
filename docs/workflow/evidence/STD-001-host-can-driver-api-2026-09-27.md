# STD-001 主机 Can Driver 接口切片：进行中证据

2026-09-27，分支 `fix/quality-check-findings`，任务基线 `caaf01e`。本记录只覆盖当前有界 Windows/MinGW 主机切片，不是 Can 模块或 HOST-CAN-01 的最终审查结论。

## 输入、接口与实际调用

- 输入为 `cargo run --manifest-path core/Cargo.toml --example quality_sample -- <独立临时目录>` 生成的单 Tx 信号 ECU；输出有 `files.list` 和 `files.sha256`。实际复核目录为 `<temporary-dir>/generated`。
- 本地 R24-11 Can Driver PDF 页 52–55、57、61、67、76 给出 `Can_ConfigType`、`Can_PduType`、`Can_HwHandleType`、状态类型及 `Can_Init`、`Can_SetControllerMode`、`Can_GetControllerMode`、`Can_Write` 的公开签名。当前实现为这些入口增加主机配置及类型；生成工程的 `Ecu_Init` 实际经 `Can_Init` 和 `Can_SetControllerMode` 初始化，Tx 路径经 `Can_Transmit` 主机包装层调用 `Can_Write`，随后以主机专用 `Can_HostFlush` 排空。
- 主机金向量路径 `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults` 通过。新增独立 C99 harness `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 通过，覆盖未初始化/停止、无效控制器与 HTH、空 PDU、错误 ID/DLC、有效发送、接收注入与 bus-off 拒绝。

## 已运行命令与结果

- `python scripts/workflow.py verify --scope all`：通过；脚本 15/15、核心单元 3/3、端到端 50/50、UI ESLint/构建、核心与桌面增量 Clippy、桌面构建均通过。该次运行在后续 Can.c 单出口与括号整理之前；整理后分别重跑上述两个核心定向测试及 `python scripts/quality.py --base caaf01e`，均通过。
- `python scripts/workflow.py verify --scope baseline --generated-dir <上述生成目录>`：全文件格式、Python Ruff、UI ESLint、两套全告警 Clippy 和 C API Doxygen 通过；BSW/RTE/生成 C 部分 MISRA 与逐能力规范证据分区仍失败。Can.c 整理后单独对 BSW 文件集重跑 Cppcheck，报告 327 条 MISRA 发现；其中新 Can.c 的多出口与优先级发现已消除。此扫描只覆盖部分规则，不证明 MISRA 符合。
- `python scripts/workflow.py check`：通过。

## 独立复核与修复

新的只读 Agent 对本切片检查了本地 Can Driver SWS 与实现，指出 bus-off 后标准状态查询错误、`Can_Write` 经共享 `last_host_status` 产生数据竞争、以及重复控制器状态转换未拒绝。随后核对本地 R24-11 Can Driver PDF 页 34–37 的状态转换与 bus-off 约束，修复这三项；主机包装层现在从每次调用的返回值取得错误，状态与发送入口由同一锁保护。C99 harness 增加重复转换、bus-off 查询/拒绝/恢复和主机输出失败验证。复核 Agent 尚未复审修复后的提交。

修复后重跑 `python scripts/workflow.py verify --scope all`：脚本 15/15、核心单元 3/3、端到端 50/50、UI lint/构建、增量质量、核心与桌面 Clippy、桌面构建均通过。重新生成的代表性 ECU 位于 `<temporary-dir>/generated`；以该工程重跑全量基线，格式/Python/UI/全告警 Clippy/Doxygen 均通过，BSW 部分 MISRA 334 条、RTE 9 条、生成 C 7 条及七项能力的规范义务门仍失败。该扫描会同时解析 Windows 与 POSIX 锁分支并报告部分系统 API 缺声明；没有因此删掉有效的并发保护或放宽扫描。

第二个独立只读 Agent 复核 `833e48a`，发现 bus-off 虽在标准查询中呈现 STOPPED，却无法通过标准 `Can_SetControllerMode` 恢复，以及输出回调重入 Can API 时非递归锁会死锁。进一步修复为分别记录标准控制器模式和主机 bus-off 错误状态；bus-off 后明确请求 STARTED 可恢复。锁改为 Windows `CRITICAL_SECTION` / POSIX recursive mutex，测试回调重入状态查询及两个并发发送线程各 100 次的返回值隔离。该二次修复的定向 C99 harness 已通过；最终独立复审待运行。

二次修复后 `python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI 与桌面构建、增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍有四个失败分区：BSW 部分 MISRA 352 条、RTE 9 条、生成 C 7 条，以及七项规范证据门。可重入锁新增的 POSIX/Windows 系统 API 与 `abort` 也被扫描报告；标准交付所需的逐条处置未完成。

第三个独立只读 Agent 复核 `c4ca40f`，确认 bus-off 标准恢复与可重入发送，但发现公开枚举缺 `CAN_CS_SLEEP`（R24-11 SWS_Can_91013）以及接收注入的状态检查和 CanIf 派发之间存在并发竞态。该轮先补齐枚举、拒绝 SLEEP 转换；`Can_Inject` 在同一可重入锁内检查状态并派发，测试接收回调也重入查询控制器状态。补丁后 `python scripts/workflow.py verify --scope all` 再次通过；新代表性工程 `<temporary-dir>/generated` 的全量基线仍是相同四个失败分区和 352/9/7 条部分 MISRA 发现。

第四个独立只读 Agent 初查未定位到被 Git 忽略的 PDF；提供准确路径后，它依据本地 R24-11 Can Driver PDF 页 31、35、55、76–77 修正结论：`SWS_Can_00258`/`00290`/`00405` 要求即使硬件不支持休眠也实现逻辑 SLEEP，`SWS_Can_00275` 要求 `Can_Write` 非阻塞，忙时用 `CAN_BUSY`（`SWS_Can_00039`/`00213`/`00214`）。当前补丁实现 STOPPED→逻辑 SLEEP→STOPPED；`Can_Write` 通过 try-lock 将报文复制到单个发送槽，忙时立即返回 `CAN_BUSY`；主机包装层在公开入口入队后调用 `Can_HostFlush`，保留原有输出顺序与 I/O 错误传播。C99 harness 覆盖错误状态转换、待发槽忙、回调内再次 `Can_Write`、慢回调期间的并发 `CAN_BUSY`，并保留金向量和拒绝路径。`python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI 与桌面构建及增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍为四个失败分区，部分 MISRA 发现为 BSW 358、RTE 9、生成 C 7 条；七项规范证据门仍为 `not_run`。非阻塞改动后的独立复审尚未执行。

第五个独立只读 Agent 对非阻塞版指出同一 HTH 的输出回调重入会错误返回 `E_OK`；现用 `tx_in_flight` 将回调执行期也计入占用，重入请求返回 `CAN_BUSY` 且不覆盖在途报文。定向测试及完整 `python scripts/workflow.py verify --scope all` 已通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建与增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍失败四个分区；BSW 部分 MISRA 358 条、RTE 9 条、生成 C 7 条，七项规范证据门仍为 `not_run`。最终复审待执行。

## 当前切片的规范逐项核对

下列核对使用本地 `docs/official/R24-11/CP/Communication/AUTOSAR_CP_SWS_CANDriver.pdf` 原文，列出本切片已声明的行为与尚未闭合的依赖；它不是完整 Can SWS 覆盖表。

| 条款 | 当前证据与缺口 |
| --- | --- |
| `SWS_Can_00223`、`00259` | `Can_Init` 使用标准签名并将控制器置 STOPPED；重复初始化的 DET 路径、配置工件仍未实现。 |
| `SWS_Can_00230`、`00409`、`00258`、`00290`、`00405` | 标准入口拒绝非法转换，允许 STOPPED→STARTED、STOPPED→逻辑 SLEEP→STOPPED；异步模式通知、STARTED 重新初始化、控制器中断与 DET 仍待实现。 |
| `SWS_Can_00020`、`00272` | 主机 bus-off 停止发送并报告 STOPPED，显式 STARTED 可恢复；CanIf 的 bus-off 通知仍待实现。 |
| `SWS_Can_91014`、`91015` | 标准模式查询返回当前 STARTED、STOPPED 或逻辑 SLEEP；DET 错误上报仍待实现。 |
| `SWS_Can_00039`、`00212`–`00214`、`00233`、`00275` | 一个 HTH 的 `Can_Write` 复制报文并立即返回；占用或抢占时返回 `CAN_BUSY`，慢主机回调不阻塞其他 `Can_Write`；Tx confirmation、独立排空调度与完整硬件语义仍待实现。 |
| `SWS_Can_00218`、`00219`、`00505` | 当前非 FD/非 TriggerTransmit 剖面拒绝 DLC>8、空 PDU/SDU；标准 DET 诊断仍待实现。 |

## 未闭合的标准义务

`Can_DeInit`、`Can_SetBaudrate`、中断控制、错误状态及其他适用服务/回调、完整 ECUC/BSWMD、MemMap 与逐规则 MISRA 处理尚未闭合。当前 `Can_ConfigType` 仍携带主机输出回调，公开 `Can_Write` 只接受到单槽主机缓存、需由主机专用入口排空；这不是第三方 CanIf/CAN ABI 或真实 MCU 证据。任务保持 `active`，HOST-CAN-01 仍为 `documented_behavior`，所有六道证据门维持原状态；后续须补标准工件、独立运行和新 Agent 复核。
