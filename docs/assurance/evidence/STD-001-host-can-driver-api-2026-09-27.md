# STD-001 主机 Can Driver 接口切片：进行中证据

2026-09-27，分支 `fix/quality-check-findings`，任务基线 `c4a01b5cd41c`。本记录只覆盖当前有界 Windows/MinGW 主机切片，不是 Can 模块或 HOST-CAN-01 的最终审查结论。

## 输入、接口与实际调用

- 输入为 `cargo run --manifest-path core/Cargo.toml --example quality_sample -- <独立临时目录>` 生成的单 Tx 信号 ECU；输出有 `files.list` 和 `files.sha256`。实际复核目录为 `<temporary-dir>/generated`。
- 本地 R24-11 Can Driver PDF 页 52–55、57、61、67、76 给出 `Can_ConfigType`、`Can_PduType`、`Can_HwHandleType`、状态类型及 `Can_Init`、`Can_SetControllerMode`、`Can_GetControllerMode`、`Can_Write` 的公开签名。当前实现为这些入口增加主机配置及类型；生成工程的 `Ecu_Init` 实际经 `Can_Init` 和 `Can_SetControllerMode` 初始化，Tx 路径经 `Can_Transmit` 主机包装层调用 `Can_Write`，随后以主机专用 `Can_HostFlush` 排空。
- 主机金向量路径 `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults` 通过。新增独立 C99 harness `standard_can_host_entry_points_reject_invalid_requests_and_send_valid_frame` 通过，覆盖未初始化/停止、无效控制器与 HTH、空 PDU、错误 ID/DLC、有效发送、接收注入与 bus-off 拒绝。

## 已运行命令与结果

- `python scripts/workflow.py verify --scope all`：通过；脚本 15/15、核心单元 3/3、端到端 50/50、UI ESLint/构建、核心与桌面增量 Clippy、桌面构建均通过。该次运行在后续 Can.c 单出口与括号整理之前；整理后分别重跑上述两个核心定向测试及 `python scripts/quality.py --base c4a01b5cd41c`，均通过。
- `python scripts/workflow.py verify --scope baseline --generated-dir <上述生成目录>`：全文件格式、Python Ruff、UI ESLint、两套全告警 Clippy 和 C API Doxygen 通过；BSW/RTE/生成 C 部分 MISRA 与逐能力规范证据分区仍失败。Can.c 整理后单独对 BSW 文件集重跑 Cppcheck，报告 327 条 MISRA 发现；其中新 Can.c 的多出口与优先级发现已消除。此扫描只覆盖部分规则，不证明 MISRA 符合。
- `python scripts/workflow.py check`：通过。

## 独立复核与修复

新的只读 Agent 对本切片检查了本地 Can Driver SWS 与实现，指出 bus-off 后标准状态查询错误、`Can_Write` 经共享 `last_host_status` 产生数据竞争、以及重复控制器状态转换未拒绝。随后核对本地 R24-11 Can Driver PDF 页 34–37 的状态转换与 bus-off 约束，修复这三项；主机包装层现在从每次调用的返回值取得错误，状态与发送入口由同一锁保护。C99 harness 增加重复转换、bus-off 查询/拒绝/恢复和主机输出失败验证。复核 Agent 尚未复审修复后的提交。

修复后重跑 `python scripts/workflow.py verify --scope all`：脚本 15/15、核心单元 3/3、端到端 50/50、UI lint/构建、增量质量、核心与桌面 Clippy、桌面构建均通过。重新生成的代表性 ECU 位于 `<temporary-dir>/generated`；以该工程重跑全量基线，格式/Python/UI/全告警 Clippy/Doxygen 均通过，BSW 部分 MISRA 334 条、RTE 9 条、生成 C 7 条及七项能力的规范义务门仍失败。该扫描会同时解析 Windows 与 POSIX 锁分支并报告部分系统 API 缺声明；没有因此删掉有效的并发保护或放宽扫描。

第二个独立只读 Agent 复核 `827a8ad3909c`，发现 bus-off 虽在标准查询中呈现 STOPPED，却无法通过标准 `Can_SetControllerMode` 恢复，以及输出回调重入 Can API 时非递归锁会死锁。进一步修复为分别记录标准控制器模式和主机 bus-off 错误状态；bus-off 后明确请求 STARTED 可恢复。锁改为 Windows `CRITICAL_SECTION` / POSIX recursive mutex，测试回调重入状态查询及两个并发发送线程各 100 次的返回值隔离。该二次修复的定向 C99 harness 已通过；最终独立复审待运行。

二次修复后 `python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI 与桌面构建、增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍有四个失败分区：BSW 部分 MISRA 352 条、RTE 9 条、生成 C 7 条，以及七项规范证据门。可重入锁新增的 POSIX/Windows 系统 API 与 `abort` 也被扫描报告；标准交付所需的逐条处置未完成。

第三个独立只读 Agent 复核 `f6ce046379f7`，确认 bus-off 标准恢复与可重入发送，但发现公开枚举缺 `CAN_CS_SLEEP`（R24-11 SWS_Can_91013）以及接收注入的状态检查和 CanIf 派发之间存在并发竞态。该轮先补齐枚举、拒绝 SLEEP 转换；`Can_Inject` 在同一可重入锁内检查状态并派发，测试接收回调也重入查询控制器状态。补丁后 `python scripts/workflow.py verify --scope all` 再次通过；新代表性工程 `<temporary-dir>/generated` 的全量基线仍是相同四个失败分区和 352/9/7 条部分 MISRA 发现。

第四个独立只读 Agent 初查未定位到被 Git 忽略的 PDF；提供准确路径后，它依据本地 R24-11 Can Driver PDF 页 31、35、55、76–77 修正结论：`SWS_Can_00258`/`00290`/`00405` 要求即使硬件不支持休眠也实现逻辑 SLEEP，`SWS_Can_00275` 要求 `Can_Write` 非阻塞，忙时用 `CAN_BUSY`（`SWS_Can_00039`/`00213`/`00214`）。当前补丁实现 STOPPED→逻辑 SLEEP→STOPPED；`Can_Write` 通过 try-lock 将报文复制到单个发送槽，忙时立即返回 `CAN_BUSY`；主机包装层在公开入口入队后调用 `Can_HostFlush`，保留原有输出顺序与 I/O 错误传播。C99 harness 覆盖错误状态转换、待发槽忙、回调内再次 `Can_Write`、慢回调期间的并发 `CAN_BUSY`，并保留金向量和拒绝路径。`python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI 与桌面构建及增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍为四个失败分区，部分 MISRA 发现为 BSW 358、RTE 9、生成 C 7 条；七项规范证据门仍为 `not_run`。非阻塞改动后的独立复审尚未执行。

第五个独立只读 Agent 对非阻塞版指出同一 HTH 的输出回调重入会错误返回 `E_OK`；现用 `tx_in_flight` 将回调执行期也计入占用，重入请求返回 `CAN_BUSY` 且不覆盖在途报文。定向测试及完整 `python scripts/workflow.py verify --scope all` 已通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建与增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍失败四个分区；BSW 部分 MISRA 358 条、RTE 9 条、生成 C 7 条，七项规范证据门仍为 `not_run`。最终复审待执行。

第五个 Agent 对 `c9a179d56aa4` 只读复审，确认同一 HTH 的回调重入不再覆盖在途报文，并按 `SWS_Can_00213`/`00214` 返回 `CAN_BUSY`；未发现该限定范围内的其他缺陷。随后为处理平台锁给 BSW MISRA 扫描新增的系统 API 发现，将 Windows/POSIX 锁实现移至明确分类的主机适配源码 `Can_HostLock.c`，BSW 仅依赖小型内部锁接口；生成器按 `src/` 文件自动交付，新文件已列入 MSVC 构建说明和 C 源码分类。独立 C99 harness、生成双 ECU 金向量与分类脚本测试已通过，BSW 部分 MISRA 报告由 358 降为 329 条；最终完整增量门、全量基线和该调整的独立复核待执行。

平台锁分离后完整 `python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建及增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线先发现新增 Python 测试文件格式未对齐；按锁定 Ruff 版本修正后复跑，格式/Python/UI/全告警 Clippy/Doxygen 均通过，仍失败 BSW 329、RTE 9、生成 C 7 条部分 MISRA 与七项规范证据门。该次调整尚待独立复核。

第六个独立只读 Agent 对平台锁分离及检查器源码分类未发现限定范围内的回归；确认生成器会收集并编译新主机锁源码，同时指出 HOST 分类仍不在部分 MISRA 扫描中，不能由 BSW 数字下降推断完整 MISRA 进展。其后对 `CanIf.c`、`LSduR.c` 与主机虚拟调度器 `Os.c` 的多出口和表达式分组作语义保持整理，保留 CAN/诊断优先、DLC 拒绝、周期调度及错误短路顺序。独立双 ECU 金向量及完整 `python scripts/workflow.py verify --scope all` 通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建与增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线中格式/Python/UI/全告警 Clippy/Doxygen 均通过；仍失败 BSW 314、RTE 9、生成 C 7 条部分 MISRA 及七项规范证据门。

随后针对 `Com.c`、`Dem.c`、`PduR.c` 的同类发现作单出口、明确运算分组、检查 `memcpy` 返回值的显式丢弃，以及跨翻译单元内部静态名称唯一化；调用顺序、DTC 落盘条件与传输缓冲的拒绝分支保持。开发前核了本地 R24-11 COM PDF 页 106、109、127 与 Dem PDF 页 261、279、300；现有主机 API 与标准签名、异步和并发要求仍不等价，本次没有升级支持声明。DTC 生命周期、DoCAN 错误恢复定向测试通过，完整 `python scripts/workflow.py verify --scope all` 再次通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建与增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍失败四个分区：BSW 部分 MISRA 251 条、RTE 9 条、生成 C 7 条，以及七项 `spec_obligations=not_run`。

## 当前切片的规范逐项核对

CanIf 接收入口先核本地 R24-11 CanIf SWS 页 125–126、Can Driver SWS 页 53–54 和 Communication Stack Types SWS 页 15–17，再将 `CanIf_RxIndication` 改为标准 `Can_HwType`/`PduInfoType` 签名；原主机 ID/DLC/时间/返回值入口明确改名 `CanIf_HostRxIndication`，实际主机注入经该包装进入标准回调。固定生成配置的 HRH、控制器均为 0；数据长度类型按当前 256 字节诊断 SDU 选 `uint16`。独立只读复核发现嵌套标准回调可能继承外层主机时间，已改为进入路由前消费时间覆盖；另一个“嵌套结果覆盖外层”疑点经赋值顺序核对并以独立 C99 重入 harness 验证，复核 Agent 再审确认两项均闭合。双 ECU 金向量、配置诊断多帧定向测试与完整 `python scripts/workflow.py verify --scope all` 通过（脚本 15/15、核心 3/3 + 51/51、UI/桌面构建和增量 Clippy）。代表性工程 `<temporary-dir>/generated` 的全量基线在格式、Python、UI、全告警 Clippy、Doxygen 通过后仍失败四个分区：BSW 部分 MISRA 25 条、RTE 9 条、生成 C 7 条及七项规范证据门。CanIf 的标准 Tx 确认、控制器模式/bus-off 通知、DET 及完整 PduR/LSduR 上层确认链尚未闭合；本段只证明接收回调形态和主机路由。

在该提交头部另以相同 Cppcheck/MISRA addon 对 `runtime/src` 全部 C 源码运行只读扫描：303 条部分 MISRA 发现，其中 HOST 分类源文件位置 294 条。现有基线没有 HOST MISRA 分区，故基线中的 BSW 25 条、RTE 9 条和生成 C 7 条不能相加当作完整运行时总数，也不能把扫描范围扩展后的数字误称为新引入缺陷。

依本地 R24-11 Can Driver SWS PDF 页 32、37、55、59–70，在固定单控制器、唯一波特率配置 ID 0、无硬件唤醒/错误计数器的目标下，新增 `Can_DeInit`、`Can_SetBaudrate`、`Can_GetControllerErrorState`、Rx/Tx 错误计数器查询、嵌套中断禁用/启用及 `Can_CheckWakeup` 的标准签名与有界主机行为。独立只读 Agent 对照规范和 ECUC 配置复核，未发现本切片的确定签名或状态转换缺陷；指出 error-passive 状态不能由当前主机模拟产生，中断计数未连接物理中断路径。独立 C99 harness 覆盖单一波特率接受/拒绝、错误状态/bus-off/恢复、计数器不可用、反初始化与重初始化；完整 `python scripts/workflow.py verify --scope all` 通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建和增量 Clippy）。代表性工程 `<temporary-dir>/generated` 的全量基线在补齐新枚举值 Doxygen 文档后，格式、Python、UI、全告警 Clippy、Doxygen 通过；仍失败 BSW 部分 MISRA 26 条、RTE 9 条、生成 C 7 条及七项规范证据门。新增公开入口在 BSW-only 扫描中也触发 `8.7`，但其外部 ABI 不能改为 `static`。本地 CanIf SWS 页 123–129、133–134、155–156 表明当前主机 `CanIf_RxIndication` 签名、Tx/模式/bus-off 回调及上层确认链尚不符合标准接口；不能据此升级完整 Can/CanIf 交付声明。

随后将主机 NvM 文件打开、读写、`fflush` 与 `fsync`/`_commit` 抽入 `NvM_HostStorage.c`，BSW `NvM.c` 保留双槽 CRC/配置指纹、序号选择与持久化成功后才提交状态；新主机源按现有分类显式登记并随生成工程自动交付。`CanTp.c` 连续帧循环和 `CanIf.c`、`Os.c` 静态标识符也做了语义保持整理。开发前查本地 R24-11 NvM SWS 页 37–39、44–45、84、99–101、139 与 CanTp SWS 页 32、34、66–67；这些整理未实现 NvM 标准异步队列或新的标准 CanTp API。脚本测试 15/15、诊断定向 6/6、两项跨重启 DTC 定向测试及完整 `python scripts/workflow.py verify --scope all`（核心 3/3 + 50/50、UI/桌面构建和增量 Clippy）通过。独立只读 Agent 核对文件新建/损坏、双槽恢复、持久化时序和 CF/FC 逻辑，未发现确定行为回归，但没有运行故障注入。代表性工程 `<temporary-dir>/generated` 的全量基线在修正 Python 格式后仍失败四个分区：BSW 部分 MISRA 18 条、RTE 9 条、生成 C 7 条，以及七项规范证据门；其他格式、Lint、全告警 Clippy、Doxygen 均通过。额外只读 Cppcheck 对 `runtime/src` 全部 C 源码运行同一部分 MISRA 扫描报 295 条，其中大量属于明确分类的 HOST 源码；18 条 BSW 数字不得解释为整个交付物只剩 18 条。

Dcm 质量整改按已查本地 R24-11 Dcm SWS 的请求、会话和确认顺序，将当前主机服务分派的九个服务处理分成单出口函数，保留原有 NRC 优先级、读/写回调顺序、`pending_session` 发送失败复位及 Dem 状态更新调用。`cargo test --manifest-path core/Cargo.toml diagnostic_ -- --nocapture` 通过（单元 1/1、端到端 6/6）；完整 `python scripts/workflow.py verify --scope all` 通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建和增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线中格式、Python、UI、全告警 Clippy 和 Doxygen 通过；仍失败 BSW 部分 MISRA 77 条、RTE 9 条、生成 C 7 条及七项规范证据门。随后将 Dcm 文件内重复的静态标识符改为模块专属名称，再次单独扫描 BSW 文件集降为 76 条；这一步尚未重跑完整基线。此主机 Dcm API 仍不等价于规范要求的 Dcm/PduR 异步接口与完整 DSL/DSD/DSP 能力，不能升级标准支持声明。

独立只读 Agent 对 Dcm 改动逐服务核对本地 Dcm SWS 7.4.1.2–7.4.1.3、7.4.2.2、7.4.2.6、7.4.2.8、7.4.2.13、7.4.2.15、7.4.2.16 与 7.4.2.24，未发现与 HEAD 相比的确定行为回归；指出错误优先级组合、回调失败副作用和发送失败后会话状态缺少专门单元测试。本轮完整端到端通过，但未据此宣称这些独立路径均已验证。

后续 CanTp 扫描整改前阅读本地 R24-11 `AUTOSAR_CP_SWS_CANTransportLayer.pdf` 页 31–33、66–67，核对首帧、连续帧、流控、N_Bs/N_Cr 超时的状态与错误路径。仅整理表达式、退出路径和局部变量；保留 `AbortRx`、`FinishTx`、FC 发送及确认的调用顺序，不升级现有主机 CanTp 公开接口为标准接口。`cargo test --manifest-path core/Cargo.toml diagnostic_ -- --nocapture` 通过（单元 1/1、端到端 6/6）；完整 `python scripts/workflow.py verify --scope all` 在配置本机 vcpkg/libclang 路径后通过（脚本 15/15、核心 3/3 + 50/50、UI/桌面构建和增量 Clippy）。新代表性工程 `<temporary-dir>/generated` 的全量基线仍失败四个分区：BSW 部分 MISRA 173 条、RTE 9 条、生成 C 7 条及七项规范证据门。另查本地 R24-11 Dcm SWS 页 35–36、51–56、111–112 与 NvM SWS 页 37–39、44、84、99–101、139；现有同步主机诊断与文件写入不符合标准模块的请求占用、异步作业/接口要求，后续整改必须以这些义务为输入。

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

## R24-11 接收轮询入口与重入修复

在修改前核对本地 Can Driver SWS 页 43–44、82 的 `SWS_Can_00396`、`SWS_Can_00012`、`SWS_Can_00226`、`SWS_Can_00108`，以及生成配置 `CanRxProcessing=POLLING`。新增随生成工程交付的 `SchM_Can.h` 和 `Can_MainFunction_Read`。主机 `Can_Inject` 把接收帧复制入单槽缓冲，再触发一次轮询；轮询入口复制帧后通过 `CanIf_HostRxIndication` 调用标准 `CanIf_RxIndication`。主机入口仍同步完成注入，以维持既有逐行协议的错误返回；该主机调度方式不是独立周期调度或硬件中断证据。

独立只读复核指出，初稿在接收回调重入时可能再次执行 `Can_MainFunction_Read`，违反 `SWS_Can_00012`。修复后用 `rx_processing` 防止主函数自重入，并以 `ECU_ERR_CAN_BUSY` 拒绝回调内再次注入；C99 harness 验证这两种嵌套调用均不产生第二次回调且外层结果正确。该 Agent 复审确认自重入问题已关闭。首轮关于共享 `rx_result` 会覆盖外层返回值的判断，在修复后的赋值顺序与拒绝路径下不成立。

修复后 `python scripts/workflow.py verify --scope all` 通过：脚本 15/15、核心单元 3/3、端到端 51/51、UI lint/构建、增量 Clippy、桌面构建。新生成工程 `<temporary-dir>/generated` 的 `files.list` 含 `SchM_Can.h` 和 `Can.c`。以此工程运行全量基线，格式、Python、UI、全告警 Clippy 与 Doxygen 通过；BSW 部分 MISRA 25、RTE 9、生成 C 7 条，七项规范义务门仍 `not_run`，因此基线未通过。下一标准链按 Can Driver SWS 页 40、81 的 `swPduHandle`、Tx 轮询与 `CanIf_TxConfirmation` 要求处理；当前主机 `Can_Transmit` 仍把句柄固定为 0，不能当作已完成的标准确认路径。

## R24-11 主机 Tx 句柄与同步确认链

实施前核对本地 Can Driver SWS 页 40、81 的 `SWS_Can_00276`、`SWS_Can_00016`、`SWS_Can_00225`、`SWS_Can_00031`；CanIf SWS 页 50、123、156 的 `SWS_CANIF_00383`、`SWS_CANIF_00007`；Com SWS 页 129 的 `SWS_Com_00124`；CanTp SWS 页 26–27、68 的 `SWS_CanTp_00075`、`SWS_CanTp_00355`、`SWS_CanTp_00215`。生成 ECUC 已给每个信号 Tx PDU 分配帧索引，诊断响应 Tx PDU 分配 `frame_count`，均适合当前 `PduIdType=UINT8` 容量。

`Can_Write` 现保留传入的 `swPduHandle`；主机输出回调成功后，由 `Can_MainFunction_Write` 调用 `CanIf_TxConfirmation(handle)`，再按配置路由到 LSduR/PduR、Com 或 CanTp。Com 周期 Tx 与 CanTp 分段发送在该同步主机剖面内以对应确认作为成功条件；输出失败不产生成功确认，CanTp 同步 `E_NOT_OK` 确认会终止当前发送。控制器 STOPPED 取消未输出帧而不产生成功确认，依据 Can Driver `SWS_Can_00282`。独立 C99 harness 覆盖非零句柄保留、确认自重入/回调内发送拒绝、输出失败无成功确认、停止取消、CanIf Tx/Rx 句柄分流与 CanTp 失败确认后重新启动。生成 ECU 金向量及诊断多帧测试通过。

独立只读复核指出异步失败确认可能需要区分 Tx 与 Rx 会话所有者。当前主机 sink 在发送调用内同步给出结果，且只有一条物理诊断连接；已撤回把成功确认后的同一帧再次报失败的无效测试。完整异步确认、N_As 等待与并发会话不在本切片的支持声明中，仍是标准交付缺口；没有把规范义务门标为通过。最终 `python scripts/workflow.py verify --scope all` 通过（脚本 15/15、核心 3/3、端到端 52/52、UI/桌面构建与增量 Clippy）。最新代表性工程 `<temporary-dir>/generated` 的全量基线仍失败：BSW 部分 MISRA 26、RTE 11、生成 C 7 条，以及七项 `spec_obligations=not_run`。其中 BSW 新增的 8.7 与 RTE 新增的 2.3/2.4 属于公开接口跨翻译单元但局部扫描未见使用者的报告，不能据此宣称全运行时或完整 MISRA 审核完成。

进一步按 MISRA C:2012 Rule 8.7 清查本轮新增公开符号：旧 `Can_Transmit` 包装函数已经没有产品调用方，故删除并将独立测试改用保留 PDU 句柄的 `Can_TransmitPdu`。`python scripts/quality.py --base c4a01b5cd41c`、Can Driver C99 harness 和生成 ECU 金向量通过。以重新生成的 `<temporary-dir>/generated` 复跑全量基线，BSW 部分 MISRA 回到 25 条，RTE 11、生成 C 7 条，七项规范义务门仍未运行；其他基线分区通过。RTE 增加的 2.3/2.4 是 `Com.h` 为标准 `Com_TxConfirmation` 签名引入 ComStack 类型后，在单独扫描 RTE 翻译单元时看到的未使用类型；未通过删除标准公开签名或屏蔽规则来消音。完整 MISRA 规则集、逐条偏差审批和全运行时扫描仍未完成。

## R24-11 CanTp N_As 与异步确认闭环

实施前直接核对本地 R24-11 `AUTOSAR_CP_SWS_CANTransportLayer.pdf`：PDF 页 26–27 的 `SWS_CanTp_00075`、`00076`、`00355` 要求 N_As 超时中止对应会话，并在迟到确认前继续占用 CAN N-PDU；页 68 的 `SWS_CanTp_00215` 定义带 PDU ID 和结果的确认；页 114 的 `ECUC_CanTp_00263` 单独定义 `CanTpNas`。因此配置读取、编辑、ARXML 往返、主机清单和生成 C 均保留独立 N_As，不再把它静默复制为 N_Bs。旧八字段主机清单仍可读，缺失 `nas=` 时沿用旧 N_Bs 值；新生成清单明确写入 `nas=`。导入 ARXML 若缺少必需的 `CanTpNas`，以 `DIAG_UNSUPPORTED` 拒绝生成。

CanTp 现在按配置的 Tx PDU 句柄记录确认，SF/FF/CF/FC 在成功确认后推进状态；未确认时按 N_As 计时，超时中止拥有该帧的会话，但保留帧占用直至迟到确认。C99 独立用例覆盖错误句柄、异步成功和失败、单帧超时、迟到确认后恢复、FF/CF 延迟确认、N_Bs 从 FF 确认时起算，以及 RX FC 与并发 TX 的所有权。独立只读审查发现初稿的 FC 超时只看 `tx.active`，可能误终止另一发送会话；按帧类型修正后，并发故障向量证明只终止对应 RX。现有正常主机 sink 仍在发送调用内同步确认，故这些新增异步路径的证据来自独立 C99 harness，而非外部硬件。

将 CanIf、Com、Dcm、NvM、Os 中 13 条变量作用域提示按原控制流收窄；对应 BSW 部分扫描不再有非 MISRA 的 Cppcheck 提示。`python scripts/workflow.py verify --scope all` 通过：脚本 17/17、核心单元 3/3、端到端 53/53、UI lint/构建、桌面构建与增量 Clippy；`python scripts/quality.py --base c4a01b5cd41c` 和 `python scripts/workflow.py check` 通过。以重新生成的 `<temporary-dir>/generated` 运行全量基线，格式、Python、UI、两套全告警 Clippy 与 Doxygen 通过；仍有 BSW 25、RTE 11、生成 C 7 条部分 MISRA 报告及七项 `spec_obligations=not_run`，基线未通过。所列 MISRA 报告主要来自局部扫描未见跨翻译单元的公开入口和宏使用；对整个 `runtime/src` 的同类扫描还包含主机适配源码的多类报告，不能用局部计数推断整体符合性。ISR 并发安全、完整 CanTp SWS、MemMap/BSWMD、完整 MISRA 逐规则处理及七项规范证据门仍待闭合，能力级别与 STD-001 状态不变。

链接器原有的 `LNK4098` 与 vcpkg `x64-windows-static` 的静态 CRT 设置有关。安装同版 libxml2、libiconv、zlib 的 `x64-windows-static-md` triplet 后，分别清理核心与桌面 Cargo 缓存中的 `libxml` 构建产物，再以新 triplet 执行 `cargo test --manifest-path core/Cargo.toml --no-run`、`cargo build --manifest-path src-tauri/Cargo.toml`：两者均通过且没有 `LNK4098`。README 已改为此配置并记录切换缓存的步骤；旧证据中的旧 triplet 和原警告保留为当时的历史事实。
