# 2026-09-28 产品方向调整后的实施就绪检查

**分范围结果：Epic 3 的受限 Windows 主机交接为 PASS，可以按 Story 3.1→3.2→3.3 开始开发；Epic 1 仍为 `in-progress`、待正式收尾；Epic 2 仍在 backlog，其 Story 2.1 保持 CONCERNS；完整参考 ECU 的 Epic 4–6 仍为 FAIL。**Epic 编号保留历史引用，不表示前两项已经完成。Story 文件的 `ready-for-dev` 只表示其界定的范围可实施，不代表能力档案已通过，也不解除后续 Epic 的决策门。

PRD R1–R7 与 Epic 1、Epic 2 的旧主机结果和拒绝场景可追踪。Story 1.1 的固定主机切片已完成，验收和剩余标准义务记录于对应 story；完成该 story 不代表完整 Can/CanIf 标准支持。Epic 3 经重新限定为当前主机剖面的输入快照、可重建工程与离线双 ECU/诊断复验，已有多文件 ARXML 保存、独立 C99 构建、双 ECU 和诊断测试基础；源码及交接说明明确显示生成目录尚未包含 ARXML 来源。PRD R6/R7、架构交接约束、Epic 退出条件及三份 story 的正反向验收共同界定了可实施增量，不依赖 OS、SWC 或板级选择。其开发 PASS 是任务就绪判断，Epic 完成仍须由非实现者实际复验并按证据门审查。

2026-09-28 在当前 Windows 环境以 README 规定的本机 vcpkg/libclang 路径运行 `cargo test --manifest-path core/Cargo.toml`：3 个单元测试和 53 个端到端测试通过，包含移动后的单 ECU 独立构建运行、双 ECU 金向量、诊断及输入保存/拒绝测试。首次未设置当前终端的 vcpkg 路径时 `libxml` 构建脚本找不到已安装库，未进入测试；补齐仅限该测试进程的路径后重跑通过。此结果验证现有主机基线，不是 Story 3.1–3.3 新功能已实现，也不包含原生 GUI 验收。

PRD FR-1–FR-15 的完整参考 ECU 产品链由 Epic 4–6 继续承载：标准参考输入、应用/RTE/OS 主机运行、单网络完整主机行为和指定 MCU 复验。它们尚无可实施 stories。PM 已提出一组最小参考配置，并推荐 FRDM-A-S32K344 作为硬件评估候选；仍需按 R24-11 核定合法完整参考输入与产物责任、诊断/NM 义务和配置，按 OS 规范声明等级并比较可交付实现路径，再实际核对 OS/RTD/工具链版本及许可、首个板级目标和真实对端。NXP RTD 所标的规范版次差异不能静默忽略，故 Epic 4–6 保持 FAIL。当前 `Os_Advance` 既不是 FreeRTOS 接入，也不是 AUTOSAR OS 实现。

Epic 2 的双 DTC 用户结果仍明确，但准确 Dem/Dcm/NvM 条款、存储格式及两个监测源的配置闭包尚待定位；不能将未核实的标准语义当已确定实现。Story 2.1 保留 backlog，但不作为新产品路线的默认下一项；诊断支持矩阵核定后决定独立保留、并入 Epic 5 或取消未开始的增量。若经排期复核启动，`bmad-build` 应先从规范和现有配置调查这些缺口；无法在该 story 内安全收口时，再使用 `bmad-spec` 或修订规划工件。该旧任务不依赖新参考 ECU 的板级目标，也不能代替其输入/RTE/OS/NM 成果。此关注项不生成独立的纯审计任务。

本次就绪判断区分产品排序与任务状态：Story 2.1 仍在 backlog，未开始实施；Epic 3 的三个 story 可依次进入开发，sprint 中由脚本标为 `ready-for-dev`，Epic 本身保持 backlog 直到实际开工。Epic 4–6 只作为后续产品成果，不得自动派发开发。下一实施动作是按 BMad Build 开始 Story 3.1，并在完成三项 story 后核对整个 Epic 3 的交接和能力边界；规划动作则并行继续关闭 Epic 4–6 的 PRD 参考配置、规范适用、OS 与目标决策门，再拆 stories。历史产品地图只用于追溯，当前方向以 BMad 工件为准。
