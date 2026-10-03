---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
workflowScope: Epic 4 only
workflowState: complete
requirementsConfirmed: 2026-09-28
epicStructureConfirmed: 2026-09-28
storiesConfirmed: 2026-09-28
planningValidation: passed
workflowCompleted: 2026-09-28
implementationStatus: done
planningBaseline: e5b84a6da244bc8cd6791bb3e974e39a2d73eb7e
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/architecture/epic-4/ARCHITECTURE-SPINE.md
  - _bmad-output/planning-artifacts/architecture/epic-4/INTEGRATION-CONTRACT.md
  - _bmad-output/planning-artifacts/architecture/epic-4/R24-11-CONTRACT.md
  - _bmad-output/planning-artifacts/architecture/epic-4/FREERTOS-FEASIBILITY.md
  - _bmad-output/planning-artifacts/implementation-readiness.md
updated: 2026-10-03
roadmapUpdate: R5-formalized-Epic-7
r5WorkflowScope: Epic 7 / candidate R5
r5WorkflowState: complete
r5StepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
r5InputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md
  - DESIGN.md
  - _bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/DESIGN.md
  - _bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md
r5PlanningValidation: passed
r5ImplementationStatus: not-started
r5PlanningBaseline: 2039020
r5PreviousWorkflowCompleted: 2026-10-03
r5PreviousPlanningValidation: passed-before-builtin-rule-amendment
r5ApprovalMode: delegated-planning / no-code
r5ValidationModel: software-native-builtin-rules-and-supported-module-catalog
r5BacklogState: Epic-7-and-11-stories-not-started / R11-priority-unchanged
r5PlanningAmendment: 2026-10-03-native-rules / current-working-tree
r5WorkflowCompleted: 2026-10-03
---

# Autosar Classic 产品成果与实施记录

本文件以[产品简述](product-brief.md)的完整 Classic 产品方向、[PRD](prd.md) 的用户成果和[架构](architecture.md)的实现边界为依据。Epic 按使用者能交付的工程结果划分，模块是支持范围；编号是稳定历史标识，不代表强制串行顺序。**Epic 1、3、4 保留原定范围的完成记录；Epic 4 的 22 条故事、跨故事集成、适用主机 SC1 行为和独立交接已经完成。历史 Epic 2／Story 2.1 的需求已转入候选 R7，不再独立实施，不标为 done。**既有 Epic 5、6 是尚未实施的正式成果定义；新路线以 R5–R29 标识候选，不能与正式 Epic ID 混用。

首个完整 CAN 参考工程仍须具备标准输入、应用／RTE／OS、实际诊断故障与持久状态、单网络 NM 及独立交接；指定 MCU 另验。新版路线在运行能力之前补上统一 Configurator 和可配置应用工程。主机完整配置、指定硬件、其他模块／版次和认证是不同成果，不因任一中间 Epic 完成而整体升级支持声明。

| 产品成果 Epic | 使用者完成的事 | 对应需求 | 当前实施边界 |
| --- | --- | --- | --- |
| Epic 3：可重建的受限主机工程 | 交接已保存的主机 ARXML、生成工程和离线复验入口 | R1–R7、FR-4/FR-15 的主机部分 | 固定 11 位 CAN 与有界 DoCAN；不声明完整 ECU Extract、SWC/RTE/OS |
| Epic 4：标准参考输入、应用与 OS 驱动的主机 ECU | 从核定的 ECU/BSW/SWC 输入生成工程，让示例 SWC 经生成 RTE 和已声明 OS 路径使用 CAN 与物理诊断 | FR-1–FR-6、FR-8–FR-9、FR-12、FR-14–FR-15 | 架构和跨模块契约已收口；W0–W5 共 22 条故事已确认并通过规划校验；实际输入与 OS/组合证据属于实施出口；常开 CAN 仅是中间配置 |
| Epic 5：单网络完整主机参考 ECU | 使用者请求/释放网络并验证 NM、诊断故障、持久状态和失败恢复 | FR-8–FR-12、FR-14–FR-15 | 仅声明一个经核定的 CAN/NM/诊断配置；主机存储与时间不外推到硬件 |
| Epic 6：指定 MCU 的可交接参考 ECU | 将同一受支持配置构建、上板并由真实对端复验 | FR-7–FR-14、FR-15 | 板卡、合法驱动、OS 端口与工具链未决；一个 MCU 不代表多平台支持 |

**后续规划边界：**Epic 4 的输入、接口与 OS 路线已定案，不重新选型。旧 Epic 5 的诊断／持久部分由 R7 细化，网络管理与生命周期由 R8 细化，更广存储由 R18 扩展；旧 Epic 6 对应 R10 的首个 MCU 成果。保留旧 5、6 的 backlog 和原范围作为追溯基线，在候选正式化时明确拆分／承接，不同时建立重复实现队列。旧定义中的“一 DTC 最小剖面”不取消已承接的双故障需求。

长期范围的唯一目录是[产品简述 R5–R29](product-brief.md#长期候选路线)。2026-10-03 用户授权将 R5 收口为正式 Epic 7：统一 Configurator，与 R6 共用对象／引用及应用源码所有权接缝，不在此扩展多组件调度；本次再按授权改为软件自有内置规则及全部受支持 BSW 定义的规划，普通用户不提供官方 XSD/MOD。历史 frontmatter 中的 `planningValidation: passed`、`workflowScope: Epic 4 only` 仍只描述已完成 Epic 4；R5 原放行不作为本次修订的新通过证据，当前等待独立最终复核。软件尚未实现本次内置校验、当前外部档案依赖未被此文档消除；Epic 7 与 11 个稳定 Story ID 仍未实施，R11 优先级及旧 Epic 5／6 与诊断承接不变，后续新成果继续使用未占用编号。

## Epic 1: 主机 CAN 标准接口与可运行信号链

用户价值：生成的虚拟 ECU 经明确的 CanIf→Can 接口收发 11 位 Classical CAN，遇到无效输入或状态时给出真实拒绝。对应 PRD R2、R3、R4；限定 R24-11、一个虚拟控制器、一个 Tx 句柄、DLC 1–8、无变体。阶段出口以双 ECU 信号闭环、无效请求拒绝、独立构建和未满足义务的准确标注判断。

收尾判断：Story 1.1 已完成，当前无头核心测试的双 ECU 金向量、无效请求拒绝和生成工程移动后独立构建运行通过；story 的独立复核与隔离桌面记录已存在，未满足的标准、MISRA 和用户交接证据仍明确列为缺口。故本 Epic 仅按**固定主机切片**结束，`HOST-CAN-01` 仍为 `documented_behavior`，不宣称 Can/CanIf 模块完整支持或整个主机产品阶段退出。

### Story 1.1: Complete STD-001 host Can Driver slice

承接迁移前的 STD-001 任务，既有实现与验证见 [BMad story](../implementation-artifacts/1-1-complete-std-001-host-can-driver-slice.md)，不重复实现已提交代码。核对 R24-11 `AUTOSAR_CP_SWS_CANDriver.pdf` 的 `SWS_Can_00223`、`SWS_Can_00230`、`SWS_Can_91014`、`SWS_Can_00233`，及 `AUTOSAR_CP_SWS_BSWGeneral.pdf` 的 `SWS_BSW_00006`、`SWS_BSW_00115`；ECUC/MOD 依赖及其他相关接口与行为按该 BMad story 完成。

验收：生成工程的 CanIf→Can 发送确实进入标准入口并保留可观察报文；正向双 ECU 结果与独立预期一致；空配置、错误句柄/ID/DLC、空 PDU 或数据、非法状态及影响生成的变体被拒绝；运行增量质量门、记录基线红项和未验证范围；新的独立复核结论写入 story 记录。未完成门禁不得升级 `HOST-CAN-01` 声明。

## 已合并的诊断故障记忆增量（历史编号 2）

**处理决定：需求合并，不再独立实施；未实现、未验收。**Epic 2 与 Story 2.1 的编号、原始范围和验收保留如下，原 backlog 状态仅用于历史说明，不再作为 sprint 开发项。接收范围为[候选 R7](product-brief.md#r7)，当前承接义务见下文[诊断承接契约](#diagnostic-carryover)及 PRD DIAG-1–DIAG-4。不得另做一次旧模型双 DTC 扩展，再重复接入标准工程。

原始定位：从现有单 DTC 主机档案扩展为两个可区分的故障状态，诊断请求能分别读出并清除，重启后的主机存储结果可重现。对应 PRD R2、R3、R4；仅沿既有 11 位物理 DoCAN 和主机文件 NvM 实现，第三方 Dem/NvM 与真实 Flash 不在原 Epic 声明内。已有主机行为证据不因规划合并而升级。

### 原始双 DTC 故事（历史编号 2.1）

原始标题：Story 2.1 — Independent host state and diagnostic reporting for two DTCs。

在既有单 DTC 配置上扩展到最多两个不同 DTC；分别绑定既有支持的 Rx 监测源，保留每个事件独立的状态、`0x19/0x01` 数量和 `0x19/0x02` 列表行为。先从本地 R24-11 Dem、Dcm、NvM SWS 和 ECUC MOD 定位该子集的适用条款、对象与拒绝条件；若标准语义与当前主机设计冲突，修正此 story 后再编码，不猜测或伪装符合标准。

验收：双 DTC 的配置往返与生成工程闭包成立；独立触发其中一个故障时只改变对应状态，两个故障分别触发时数量与列表正确；重复 DTC、第三个 DTC、无效监测引用及不支持的诊断子功能明确拒绝；运行主机正反向与跨进程检查，记录仍欠缺的规范、界面和硬件证据。先前提出的 `AUDIT-003` 仅作为这项实现所需的证据线索，不单独抢占下一开发项。

<a id="diagnostic-carryover"></a>
### R7 诊断承接契约

本节是已确认的需求接收位置，尚不是已通过就绪检查的正式实施 Epic。历史 Story 2.1 的有效行为全部保留；原“最多两个、第三个拒绝”的限定被以下容量规则明确替代，不静默删除或重新限制产品范围。

| 需求来源 | 接收义务与最小验收 |
| --- | --- |
| 2.1 配置与监测关系 → DIAG-1 | 至少两个不同故障可独立配置、往返与生成，监测源绑定合法；同一标准工程表达事件／DTC／存储关系 |
| 2.1 独立状态 → DIAG-1 | 分别触发、恢复、清除及同时触发有独立预期；一个故障不会错误改变另一个 |
| 2.1 数量／列表 → DIAG-2 | 保留 `0x19/0x01`、`0x19/0x02` 的行为义务，数量、列表与状态掩码符合实际事件；所选清除语义按规范明确 |
| Epic 2 重启持久状态 → DIAG-3 | 跨进程保存／恢复与清除逐故障成立，损坏、写入失败和配置不匹配不伪装成空故障或成功 |
| 2.1 拒绝场景 → DIAG-4 | 重复 DTC、非法监测引用、不支持子功能和超出声明容量明确拒绝；至少两个是隔离场景，不是未经设计的永久上限 |
| 2.1 规范与证据 | 编码前核对 R24-11 Dem／Dcm／NvM SWS／ECUC，保留正反向与跨进程预期；主机不继承真实 Flash 或 MCU 能力 |

R7 正式化时必须分配具体 Story、引用历史 2.1 与 DIAG-1–DIAG-4，并固定支持配置、容量、独立请求和预期。不因本次合并宣称功能完成，也不创建占位 Story 文件。旧 Epic 2／Story 2.1 的 sprint 键经明确迁移后移出，已完成 Epic／Story 与 action items 保持不变。

## Epic 3: 交接可重建且可离线复验的受限主机工程

当前状态：`done`，实现位于提交 `817063e`，三个 story 文件及 sprint 状态一致。以下保留原定成果、范围与验收契约；本轮只核对记录和当前源码，不重跑其运行验收，也不升级七项BMad 实现记录的 `documented_behavior`。

用户成果：一位 ECU 集成工程师以当前受支持的 R24-11 多文件主机 ARXML 配置生成 C99 工程，连同准确的输入快照与交接清单交给另一位工程师；接收者用明确列出的同版工作台与合法 XSD 在新目录中重新导入和生成，再用独立预期复验一组双 ECU CAN 与有界诊断行为。现有 CAN/诊断运行代码及 `files.list`、`files.sha256`、`build.ps1` 作为基础复用。

支持边界：固定 11 位 Classical CAN、单虚拟控制器、当前有界物理 DoCAN、Windows/PowerShell/GCC；输入只承诺当前工作台已支持的主机配置，不把它称为完整 ECU Extract、BSW Delivered Bundle 或 SWC 描述。不得额外打包密钥、运行状态和未保存编辑；生成元数据不记录原机器绝对路径，源 ARXML 原文由交付者审阅。此 Epic 满足 PRD R1–R7 与 FR-4/FR-15 的主机部分，不满足完整参考 ECU 的 FR-1、FR-5、FR-6 或实机声明。

关键依赖：现有多文件 ARXML 导入/保存与生成闭包、主机 GCC 工程、双 ECU 和诊断独立测试器；R24-11 方法学及 ECUC/MOD 用来界定本剖面与完整 ECU 配置的差别。无需等待 AUTOSAR OS、真实 SWC、MCU 或供应商驱动选择。

退出条件：非实现者用交付的合法参考输入、工程文件及明确列出的同版工作台、合法 XSD 和主机工具，在新目录完成重新导入、生成和构建；按固定独立预期复验双 ECU CAN、适用诊断以及拒绝/恢复路径；缺文件、源文件外部改动、未保存编辑、编译失败或实际报文不符均报告失败且不损坏旧包。交付状态明确限于主机；完整能力声明仍经BMad story 验证另审。下列三个 story 共同构成本 Epic 的可开发范围，不能只完成其中一项就把 Epic 标为 done。

### Story 3.1: Package saved host ARXML inputs with generated ECU

使用者通过独立于普通生成操作的“导出可重建主机交付包”，从已保存且通过受限主机校验的 Workspace 导出工程时，交付目录包含所有影响本次生成的 ARXML 原文、可重建的逻辑文件映射、R24-11/工具/目标信息，并将其列入现有文件清单和完整性记录。输入有未保存修改、外部改动、缺失文件、路径冲突或影响生成的断开引用时拒绝“可重建交付”，旧输出保持不变；不额外复制密钥或运行状态，路径映射不写源机器绝对路径。现有仅为构建而生成的 C99 工程继续按原有声明运行，不冒称包含源输入。详见同名 story 文件中的正反例与交付边界。

### Story 3.2: Reopen delivered inputs and reproduce host C99 project

接收者使用明确列出的同版工作台及合法 R24-11 XSD，按交接说明在新目录导入 Story 3.1 的输入快照，经同一受支持剖面校验后再生成等价 C99 源工程；比较与路径无关的文件集合及内容，对声明的工具版本和外部构建依赖作出可检查说明。遗漏、篡改或不支持的源输入阻断重新生成，旧工程和原输入不被静默修复或覆盖。详见同名 story 文件。

### Story 3.3: Verify delivered host behavior without the workbench

接收者仅依交付说明、Windows/PowerShell/GCC 和随包离线测试入口，构建一组固定的双 ECU 参考工程并对照独立 CAN/诊断预期执行正向与拒绝/恢复向量。缺工具、校验失败、超时或实际报文不符必须得到失败结果；对任意非参考配置保持“未验证”，不能因参考用例通过自动升级其支持声明。详见同名 story 文件。

## Epic 4: 标准参考输入与示例应用经 RTE 和已声明 OS 运行主机 ECU

### 需求提取：W0–W5 故事规划输入（已确认）

本小节记录 `bmad-create-epics-and-stories` 的需求提取结果，依据已提交架构基线 `e5b84a6`。需求、Epic 结构及 22 条故事均已确认，本范围规划校验通过，故事规划流程于 2026-09-28 完成。保留现有 Epic 1–6、实施状态及历史验收记录，不重新初始化整份文档。工作范围固定为 Epic 4；本轮仅完成故事规划，不授权功能开发、生产依赖集成、支持声明升级或远端操作。Q1/Q2 沿用最终架构，不重新选型。

#### Functional Requirements

PRD 共定义 R1–R7 七项既有主机需求及 FR-1–FR-15 十五项参考 ECU 需求。以下保留原编号并标出本次边界；不能把 Epic 4 的分阶段覆盖写成完整参考 ECU 已满足。

| 需求 | 提取的成果 | Epic 4 的适用范围 |
| --- | --- | --- |
| R1 | 多文件 ARXML 形成可检查状态，安全保存、重开并保留有效内容 | 继承并扩展到新目标输入角色 |
| R2 | 确定性生成独立 C99 工程，主机构建并运行 CAN/诊断 | 新目标独立取证，不继承旧轮询目标的运行结论 |
| R3 | 版本、变体、引用及运行输入错误可定位拒绝 | 新剖面正反例及失败关闭 |
| R4 | 输入、独立预期、实际正反结果与适用规范可追溯 | 各工作包产物及最终story 验收 |
| R5 | 区分运行、交付、证据进展和未验证维度 | 规划及未来交付状态准确呈现 |
| R6 | 已保存输入、逻辑身份、来源摘要随可重建包交接，新目录可再生成 | 复用 Epic 3 安全规则；新角色/格式显式版本化 |
| R7 | 随包离线入口以独立预期复验行为与失败路径 | 扩展新目标；旧参考回归另保留 |
| FR-1 | 显示并接受声明支持的 Extract、ECUC、BSW/SWC 描述、映射及来源 | 原创、已展开的单 ECU Extract，不实现通用系统提取器 |
| FR-2 | 受支持参数可检查/修改并预览影响；保留有效无关内容；不安全保存拒绝 | 新角色参与现有安全编辑、保存和重开链 |
| FR-3 | 模块及目标配置、类型、变体和跨模块引用闭合；错误可定位 | 新剖面的 RTE/OS/CAN/物理诊断；NM 留 Epic 5 |
| FR-4 | 源/生成文件、配置、来源、外部依赖及构建清单可独立交接 | Windows x64/GCC 新目标完整闭包 |
| FR-5 | 真实 SWC 描述驱动接口与 ECU 集成生成；应用发送、接收、服务可运行 | 显式未排队 uint32 S/R＋同一实例同步 ReadData C/S |
| FR-6 | OS 等级的适用配置、接口与运行义务逐项验证 | 单核 SC1/Extended Status；W1 核心门与 W5 完整等级出口分开 |
| FR-7 | 指定 MCU 与合法驱动工程构建及真实运行 | 本轮不拆，Epic 6 |
| FR-8 | 单 CAN 应用信号打包/解包、状态、超时与拒绝可复验 | 11 位 Classical CAN，固定参考字节向量 |
| FR-9 | 声明物理 DoCAN/UDS 服务、会话及拒绝可复验 | 新应用 DID 和已声明物理诊断回归；完整 DTC 剖面留 Epic 5 |
| FR-10 | 监测故障与真实目标存储的保存、重启恢复、清除及失败行为 | 本轮不拆，Epic 5/6 |
| FR-11 | ComM/Nm/CanNm/CanSM 请求、释放、NM、睡眠/唤醒、故障闭环 | 本轮不拆，Epic 5/6；常开 CAN 是 Epic 4 中间配置 |
| FR-12 | 另一执行者重建主机配置并复验应用、通信、诊断及失败恢复 | Epic 4 覆盖新目标应用链；NM/持久化部分留 Epic 5 |
| FR-13 | 指定板卡与真实对端独立复验同一声明范围 | 本轮不拆，Epic 6 |
| FR-14 | 支持矩阵绑定版次、配置/变体、OS、目标及工具链；按实际行为显示 | 本轮仅规划；实施中保留有界集成与完整 SC1 声明差别 |
| FR-15 | 输入、生成、构建和行为失败可定位，保留旧输入/产物 | 静态拒绝不写半套输出；运行目标故障停机，不伪称回滚已执行批 |

#### NonFunctional Requirements

PRD 没有独立编号的 NFR 清单。以下 E4-NFR 编号仅是本次提取的追踪标识，来源为 PRD 的交付约束、AD-1–AD-9、集成契约，不补造 AUTOSAR 要求编号。

- E4-NFR-1：相同输入、固定工具/内核/补丁及目标生成相同文件集合与内容；新目录复验不依赖源机器绝对路径或仓库隐含文件。
- E4-NFR-2：多文件边界和原始字节 SHA-256 可追溯；未修改文件不重写，无关有效内容保留；外部变更、未保存编辑及引用风险拒绝交接。
- E4-NFR-3：C99 及项目增量质量门适用；按产物角色核对 API/类型/配置、BSWMD、条件适用的 MemMap/MISRA 及偏离，不以零警告代替标准证据。
- E4-NFR-4：静态生成汽车 OS 对象与容量，FreeRTOS 对象优先静态创建；宿主线程/端口资源分配另记录，不声明整个进程无动态分配。
- E4-NFR-5：有界输入/输出与确定的逻辑时间；每批至多 256 消息、跨度至多 1000 ms、输出队列 256 条、COMMIT watchdog 5000 ms。watchdog 只控制进程存活，不改变汽车计时。
- E4-NFR-6：失败与 not_run 分开；参考配置通过不外推任意配置、MCU、硬实时、ASIL、完整 MISRA或官方一致性。
- E4-NFR-7：源码、内核补丁、目标端口、工具链及许可来源固定，离线构建不拉取浮动依赖；官方 XSD/MOD/PDF/showcase 不默认随包分发。
- E4-NFR-8：测试使用后台、无头或独立故障注入进程；原生 Tauri/IPC 验收必须使用隔离桌面，无法隔离时记未验证。

#### Additional Requirements

架构 AD-1–AD-9 均适用。仓库为已有项目，没有新建工程 starter template 要求。

- E4-AR-1（AD-1/2）：剖面 `epic4-win64-sr-cs-v1`；ARXML 为唯一配置权威。完整路径＋实例身份、角色、相对文件身份、原始摘要及定义来源保留；不以短名、工具 SDG 或猜测补齐标准关系。
- E4-AR-2（AD-1/3）：先组件契约、后 ECU 集成；生成器只消费只读 `ValidatedIntegrationPlan`。类型、端口、访问、服务、调度与外部符号生产者/消费者闭合；符号归一化碰撞、缺实现、重复调度整体拒绝。
- E4-AR-3（AD-3）：`EchoApplication` 单实例不可重入；显式未排队 uint32 S/R；Rx/Tx 初值 0，应用/Tx 周期 10 ms，Rx deadline 30 ms、ComFirstTimeout=0。读取状态为 NEVER_RECEIVED/E_OK/MAX_AGE_EXCEEDED；超时保留接收值、应用采用 0；写失败不提交新快照。
- E4-AR-4（AD-4）：DID `0x1234`、UINT8_N 四字节大端、默认/扩展会话可读；`USE_DATA_SYNCH_CLIENT_SERVER`，同一应用实例的提供端口、ReadData 操作及 OperationInvokedEvent 绑定。同步服务没有 OpStatus/NRC 输出，正常服务器填数据并 E_OK；RTE 基础设施错误单独处理，不伪造正响应。
- E4-AR-5（AD-5/8）：沿用 FreeRTOS V11.3.1/commit `054e14f3397023aa83813a65aa065fc4597d481b`、Windows x64/GCC 16.1.0，单核 SC1/Extended Status。FreeRTOS 独占实际 ready/Running/上下文调度；自有层维护汽车语义，经唯一 backend 转换；研究补丁不是生产实现。
- E4-AR-6（AD-8）：全部激活请求 FIFO、同级抢占/ceiling 恢复位置、Finish/Chain 合法不返回与错误原子回滚、非抢占/内部资源、事件所有权/竞态及资源 LIFO/ceiling/调用层级有独立向量；不能以计数、notification 或 mutex 默认语义替代。
- E4-AR-7（AD-6）：`Task_Ecu` Extended/FULL/priority=1/maxActivation=1，Ev_Work/Ev_App/Ev_IO；Counter 1 ms，Alarm_Work 1/1 ms、Alarm_App 10/10 ms。StartupHook 按契约初始化，Ready 前输入/tick 闭锁；失败发布 Failed 并经合法 ShutdownOS 关闭，不发明 StartOS 私有返回值。
- E4-AR-8（AD-6）：HostBatchV1、uint64 epoch/序号、32 bit FreeRTOS tick、SystemCounter 模数 65536；逐毫秒请求/完成确认，禁用墙钟自主 tick。同 epoch 输入先于 deadline，应用先于 Dcm；事件是唤醒提示，记录/ticket 是完成依据。逐条确认不能因事件合并而丢失。
- E4-AR-9（AD-6）：Task_Ecu 唯一拥有应用/BSW 可变状态；bridge/模拟 ISR 只发布复制记录或事件。按契约固定输入/确认→驱动→CanTp→Com 接收 deadline→应用→Com 发送→Dcm→确认回调；不得遗漏实际启用的周期实体或同 tick 重复运行。
- E4-AR-10（AD-6）：宿主阻塞 I/O 不进入 FreeRTOS 任务；输出成功才产生匹配 PDU/ticket 确认。批在状态改变前拒绝协议/容量/倒退/序号错误；运行输出失败、溢出或 watchdog 到期关闭 ECU。
- E4-AR-11（AD-5/8）：原生线程真实栈注册、边界/上下文观测和异常捕获；独立进程实际栈消耗/破坏证明 E_OS_STACKFAULT、ShutdownHook 与无继续执行。预分配故障记录/独立控制栈，不以假 buffer/flag、MCU 栈证据替代主机门。
- E4-AR-12（AD-5/7）：W5 完成全部适用 SC1/OSEK 一致性类、至少八个软件 Counter/两个 ScheduleTable、生命周期、ISR/错误/Hook/ARTI 的实际产品能力；参考少量对象不缩减等级能力。对应 BMad story 记录实际验证范围与结果。
- E4-AR-13（AD-7）：新目标与 host-v1 独立标识、链接与证据；旧包读取和旧目标行为保持可复验。交付格式扩展显式版本化；新目录重导入、生成、独立构建与运行闭合，已有目录不覆盖。
- E4-AR-14（AD-9）：W0 从最终输入契约开始；W1 从已确认 backend/目标契约开始；W2 依赖 W0并可与 W1 并行；W3 必须等待 W1 核心语义/时间/真实栈及 W2 闭包通过；W4 验证组合行为；W5 完整等级与独立交接决定 Epic 出口。实际停止条件触发时止损，普通未实现项不重开 OS 选型。
- E4-AR-15（集成契约 §6）：独立对端 RX `0x320/4: 78 56 34 12`，epoch 10 后 TX `0x321/4: 78 56 34 12`；应用 DID 的 UDS 为 `62 12 34 12 34 56 78`，CAN SF 为 `07 62 12 34 12 34 56 78`，DoCAN 请求/响应 ID 为 `0x700/0x708`、DLC=8。接收在 epoch 0 时 epoch 30 应用替代为 0；epoch 30 的新帧先接收后判超时；初始 DID 四字节为 00。预期不由被测生成器或打包实现推导。

#### UX Design Requirements

规划目录中未发现独立 UX DESIGN/EXPERIENCE 契约。本次提取已有 PRD、架构中的实际用户流程，不扩展为界面重设计。

- UX-DR1：配置入口显示新剖面必需/可选/外部输入及来源，能定位文件、完整对象路径、引用、失败原因和补救动作；不能由 UI 私有映射补全核心计划。
- UX-DR2：使用者可检查受支持参数、预览保存/生成差异并重开输入；未支持的有效无关内容保留，影响生成的未支持关系明确阻断。
- UX-DR3：区分已保存、已校验、已生成、已构建、主机行为已验证、等级义务证据已审与实机已验证；有界 BSW-on-RTOS 不能显示为完整 SC1。
- UX-DR4：沿用独立导出操作、安全预览、旧目录保护及可恢复错误；缺依赖/权利或失败时不显示完整交付成功。原生界面/IPC 只在隔离桌面取证。

### Epic List：本次结构（已确认）

本次仅更新 **一个既有 Epic 4**，不新增 Epic、不改变 Epic 编号，也不将 W0–W5 拆成六个按技术层划分的 Epic。完整仓库继续保留 Epic 1–6；文首成果表及 Epic 1/2/3/5/6 的历史范围有效。

**Epic 4：标准参考输入与示例应用经 RTE 和已声明 OS 运行主机 ECU。**使用者导入/建立受支持的原创单 ECU Extract、SWC/类型、实际 BSW 描述与 ECUC 配置，经安全检查和确定性生成得到 Windows 工程；接收者在新目录重建，用独立 CAN/诊断对端验证应用发送、接收与同步应用 DID，并取得该目标全部适用 SC1 义务及交接证据。应用链与独立交接构成一个用户成果，因此输入、生成、OS/backend、目标集成与验证工作在本 Epic 内有序拆分。

**覆盖：**FR-1–FR-6、FR-8、FR-9 的声明物理诊断部分、FR-12 的应用主机验证部分、FR-14–FR-15；继承 R1–R7 对安全输入、来源完整性、独立交接和准确声明的约束。

**与其他 Epic 的关系：**复用已完成 Epic 3 的输入快照、完整性清单及交接规则；不依赖暂缓 Epic 2 的双 DTC 扩展，也不等待未来 Epic 5/6 的 NM、持久化或 MCU 才能交付本范围。Epic 5 在本成果上扩展完整单网络行为；Epic 6 再形成指定 MCU 的独立目标证据。Epic 4 的成果可独立使用，但仍不是整条完整 Classic 参考 ECU 产品链的完成。

#### FR Coverage Map

| PRD 需求 | Epic 4 本次覆盖 | 保留的其他 Epic 责任 |
| --- | --- | --- |
| FR-1 | 声明支持的 Extract/SWC/BSW/ECUC 输入角色、来源与闭包 | Epic 5/6 后续配置/目标的扩展输入 |
| FR-2 | 新剖面的参数检查/编辑、保存预览、往返保留与拒绝 | Epic 5/6 各自新增配置的同等安全义务 |
| FR-3 | 新剖面的类型、通信、服务、OS/目标计划闭包与定位诊断 | Epic 5 NM/存储闭包，Epic 6 MCU 目标闭包 |
| FR-4 | Windows 新目标的完整输入、源码/生成物、依赖与独立构建交接 | Epic 3 为既有 host-v1 基础；Epic 6 真实目标交接 |
| FR-5 | 模型驱动组件契约与 ECU 集成；S/R＋同步 C/S 应用服务 | Epic 6 同一应用接口的板级复验 |
| FR-6 | 固定路线的 Windows 单核 SC1 目标；完整适用等级义务为本 Epic 出口 | Epic 6 MCU 端口与目标义务另验，不继承主机结论 |
| FR-7 | 不覆盖 MCU 驱动实现 | Epic 6 |
| FR-8 | 应用 CAN 的独立正向、边界、超时与拒绝向量 | Epic 5 网络模式下行为，Epic 6 实机行为 |
| FR-9 | 应用 DID、会话/服务拒绝及声明物理 DoCAN 回归 | Epic 5 完整 DTC 诊断剖面，Epic 6 真实对端复验 |
| FR-10 | 不新增故障/持久化能力 | Epic 5 主机存储，Epic 6 真实存储；Epic 2 增量继续暂缓 |
| FR-11 | 常开 CAN 为中间配置，不覆盖完整 NM | Epic 5 主机 NM，Epic 6 板级 NM |
| FR-12 | 独立重建与应用/CAN/DID/失败路径主机验证 | Epic 3 旧剖面；Epic 5 NM/持久化主机验证 |
| FR-13 | 不覆盖实机验证 | Epic 6 |
| FR-14 | 新组合的声明、来源、范围与story 验收；实现和验证记录按 BMad | Epic 3/5/6 分别按各自范围留证 |
| FR-15 | 新目标输入/生成拒绝、运行故障关闭、旧产物保护 | 所有其他 Epic 保留各自失败关闭义务 |

#### W0–W5 的组织与依赖

以下是已提交集成契约的工作包归属，具体故事见后文“W0–W5 故事索引与实施规则”。每个包内按可独立验收的结果拆分；依赖只能指向已存在的基础或先交付的结果，不能形成对未来 story 的循环依赖。

| 工作包 | 使用者价值与结果责任 | 进入依赖与放行边界 |
| --- | --- | --- |
| W0 输入与证据基线 | 有合法原创输入、拒绝样例、标准适用性及来源清单，能确定工具承诺什么 | 最终架构及固定来源；不要求新解析器、OS 或完整 SC1 已实现 |
| W1 OS/backend 基础 | 新目标能按声明汽车语义执行、推进受控时间，并在真实栈故障时安全关闭 | 最终 backend/目标契约及固定内核/编译器/许可；核心语义、时间、真实栈结果共同构成 W3 前置门 |
| W2 模型及契约生成 | 输入形成唯一可检查计划及组件接口，错误关系在生成前被定位拒绝 | W0 支持剖面与正反输入通过；可与 W1 并行，不依赖后续 ECU 集成或应用运行 |
| W3 ECU/目标集成生成 | 从同一计划获得 RTE/SchM/OS/BSW 配置、单执行者任务与可离线构建 Windows 工程 | W1 关键门及 W2 闭包通过；未通过不能称为可交付参考 ECU |
| W4 应用/通信/诊断闭环 | 独立对端复验同一实例的 S/R、快照、ReadData、字节预期和失败恢复 | W3 可构建工程；新目标行为与旧目标回归分别留证 |
| W5 等级与交接出口 | 接收者独立重建复验；完整 SC1、来源权利、支持边界有逐项证据 | 对已实现服务及集成结果验收；完整等级门不倒置为 W0/W1 开始条件 |

各包跨越的核心实现共同属于同一输入→计划→生成→运行链；W2/W3 共享模型与生成器，W1/W3 共享 OS/backend 配置边界，W3/W4/W5 共享构建交接入口，因此维持单一 Epic 更利于类型、符号与时间契约一致。具体文件拆分留给后续规格，当前不制造新的持久化配置或并行调度器。

#### Epic 4 退出条件

1. 原创参考输入通过合法 R24-11 XSD 与该剖面语义检查，错误引用/类型/变体和来源冲突被定位拒绝；安全保存/重开保留输入边界及无关有效内容。
2. 同一 `ValidatedIntegrationPlan` 生成组件契约、RTE/SchM、BSW/OS 配置、任务入口与构建清单；无缺失/重复生产者、类型签名冲突或重复周期调用，独立 C99 编译及链接通过。
3. 固定 Windows 目标在 OS 边界遵守唯一 Task_Ecu、逐毫秒请求/确认、同刻输入/deadline/应用/Dcm 顺序及真实执行栈故障关闭；不得以旧 Os_Advance 或轮询选择 runnable 代替。
4. 独立 CAN/UDS 字节向量、初值、超时边界、输出/确认错误及失败关闭通过；现有旧目标单独回归，不能由旧证据替代新组合验收。
5. 全部适用 SC1/OSEK 能力、最小容量、调用层级、计时、错误/Hook/ARTI 及主机栈义务逐项有证据；未跑或失败时保持有界集成描述，不将 Epic 4 标 done。
6. 另一执行者在新目录按固定依赖重导入、再生成、构建和离线复验；来源/许可、适用工件质量和BMad story 验证接受独立复核，原生 UI/IPC 以隔离桌面验证或如实记未验证。

本结构及以下 22 条故事已确认，第四步规划校验通过，故事规划流程于 2026-09-28 完成；仅写入规划文件，不创建 ready-for-dev 工件。sprint、BMad 实现记录及功能源码保持原状态。

架构收口进度（2026-09-28）：[spine](architecture/epic-4/ARCHITECTURE-SPINE.md) 已记录维护者确认的 Q1 主路线：单核 SC1 目标＋FreeRTOS 固定内核＋自有 AUTOSAR OS 语义＋有限单核策略扩展，保持原生 Windows；Trampoline/ATK2 等仅作参考和备选。Q2 已确认显式 S/R＋同步 C/S 应用 DID。原创输入及组件契约/集成生成按 [规范矩阵](architecture/epic-4/R24-11-CONTRACT.md) 核定；具体输入/接口/时间已由 [集成契约](architecture/epic-4/INTEGRATION-CONTRACT.md) 固定，按 W0–W5 拆验收明确的 stories；实现证据在对应工作包交付，不循环前置。选型已结束，不因普通待验证项重新扩展候选；只在实际停止条件或明确范围变更时重新比较。固定生产依赖已引入；完整 SC1 尚未完成，Epic 4 保持 in-progress，当前进度由 sprint 文件记录。

用户成果：集成工程师从按 R24-11 方法学核定的系统/ECU、BSW/ECUC 与示例 SWC 输入建立参考 ECU，将真实描述的 SWC 映射到目标并生成必要 RTE/BSW/OS 配置与接口；另一位工程师在 Windows 主机目标独立构建并运行，使应用值通过 CAN 信号链发送、接收，且能从独立诊断测试器读取声明的应用 DID。现有有界 DoCAN 行为必须按选定配置复验，不以 Epic 3 的主机专属输入、固定 `Rte.c` 桥接或 `Os_Advance` 虚拟轮询充当成果。

支持边界：只声明经核定的一组 ECU Extract/BSW/ECUC/SWC 输入、一组 SWC 接口、CAN 与物理 DoCAN/UDS 子集，以及一个按 R24-11 OS 规范界定的可扩展性等级。FreeRTOS 原生任务调度可用于前期实验，但不是 OS 义务的通过证据；常开 CAN 可用于本 Epic 的中间运行，完整单网络管理留给 Epic 5。对应 FR-1–FR-6、FR-8–FR-9、FR-12、FR-14–FR-15。

关键依赖：Epic 3 的可重建输入/产物闭包；已审的 SWC/RTE 描述和 ECU 映射；OS 等级、适用 SWS/ECUC 义务、可交付实现与主机端口选择；独立 CAN/诊断对端。退出条件：从合法输入生成并独立构建主机工程；对端可复验应用发送、接收和诊断读取，错误接口/映射及不支持服务被拒绝；声明范围内的 OS 配置、接口与运行义务有逐项主机证据，未验证的实时性与硬件中断明确留空。OS 主路线已确认；架构契约已固定；W3 集成必须等待 W1 核心语义/时间/真实栈与 W2 闭包通过，W0/W1 不因尚未实现而被阻断。

### W0–W5 故事索引与实施规则

以下 22 条 story 均为 **规划已确认／实施 backlog／功能验收 not_run**。文中“产物”和“执行入口”是未来实现要求，当前不存在的新 fixture、测试、OS/backend、生成接口和目标工程不因写入规划而变成已实现。实际开发进入前仍需形成对应开发规格、冻结 story 起始 commit 及核实外部资料；本轮不创建 implementation-artifacts 中的开发任务文件、不派发 build、不更新 sprint。

| 工作包 | Story | 交付结果 | 前置结果 |
| --- | --- | --- | --- |
| W0 | 4.1 | 原创标准输入及结构/语义拒绝目录 | 最终架构、合法 R24-11 资料 |
| W0 | 4.2 | 标准义务、固定依赖来源及独立预期目录 | 4.1 |
| W1 | 4.3 | 固定内核、唯一 backend、静态对象与生命周期基础 | 最终契约、固定内核/工具/许可；不依赖 W0 完成 |
| W1 | 4.4 | 全激活请求 FIFO 与内核状态一致 | 4.3 |
| W1 | 4.5 | 原子 Terminate/Chain 与入口重新开始 | 4.4 |
| W1 | 4.6 | 非抢占/内部资源/资源 ceiling 与位置恢复 | 4.5 |
| W1 | 4.7 | 事件、合法模拟 ISR 边界及无丢失唤醒 | 4.6 |
| W1 | 4.8 | 逐毫秒 tick、Counter/Alarm 核心与确认 | 4.7 |
| W1 | 4.9 | Windows 真实线程栈故障捕获与关闭 | 4.3；可提前验证，W1 放行仍要求 4.3–4.9 全部通过 |
| W2 | 4.10 | ARXML 模型、唯一校验计划与闭包诊断 | 4.1、4.2；与 W1 并行 |
| W2 | 4.11 | SWC 组件契约头与同步 C/S 类型 | 4.10 |
| W2 | 4.12 | 安全检查/编辑/保存、预览与重开 | 4.11 |
| W3 | 4.13 | RTE/SchM/BSW/OS 集成生成与独立链接 | 4.3–4.9、4.10–4.12 全部通过 |
| W3 | 4.14 | HostBatchV1、输出确认及可运行 Windows 工程 | 4.13 |
| W4 | 4.15 | EchoApplication 的 S/R、快照与应用 DID | 4.14 |
| W4 | 4.16 | 独立边界/失败恢复、声明诊断及旧目标回归 | 4.15、4.2 的独立预期 |
| W5 | 4.17 | Counter/ScheduleTable 完整适用能力与容量 | 4.8、4.2；可先于 W4 在独立 OS 配置验收 |
| W5 | 4.18 | Extended Status、Hook/生命周期与完整模拟 ISR 义务 | 4.3–4.9、4.2；可先于 W4 独立验收 |
| W5 | 4.19 | R24-11 ARTI 描述/Hook 与实际状态对应 | 4.17、4.18 |
| W5 | 4.20 | 适用 C 产物、BSWMD、MemMap/MISRA 及模块证据 | 4.16–4.19 |
| W5 | 4.21 | 新格式交接、离线复验与准确工作台状态 | 4.20 |
| W5 | 4.22 | 独立全等级与交接复核、Epic 出口判定 | 4.21及全部前置结果 |

**W1→W3 放行门：**4.3–4.9 的实际配置、源码/补丁摘要、独立预期与正反结果均通过，且元数据与内核状态一致、受控 tick 无损、真实栈故障不可继续执行。4.9 可以从 4.3 提前开展，避免将关键端口风险拖到集成之后。W2 可以独立编译组件契约和验证计划，不链接未通过的 W1。W3 不要求 4.17–4.22 已完成；它们是后续等级/交接出口。

**开发与验证：**按安装的 BMad 完成规格、实施、验证、复核及 sprint 同步。各 story 使用覆盖其预期行为的实际测试与适用构建检查，结果记录在该工件。

BMad 规格的 Verification 记录本次选用的命令和实际结果；`scripts/verify.py` 可组合工程检查，不另设通用审批或报告流程。

### Story 4.1: 建立原创参考输入与可定位的拒绝样例

As a ECU 集成工程师，
I want 获得符合最终契约的原创单 ECU 输入及已标明错误的反例，
So that 后续解析与生成可以对照同一输入基线验收。

**工作包／依赖：**W0；最终架构与合法 R24-11 XSD/MOD，无需新 OS/解析器已实现。**覆盖：**FR-1/FR-3，E4-AR-1、E4-NFR-2/7。**产物：**产品原创多文件 ARXML、角色/路径/原始摘要清单、正反例目录及人工独立核对的语义预期。BSW 描述绑定现有真实实现；尚未实现的 OS/RTE 入口列为未来生产者要求，不冒称当前符号已存在。**执行入口：**`epic4_reference_input_baseline`。

**Acceptance Criteria:**

- **Given** 合法 R24-11 XSD/MOD 和集成契约 §1/2/6，**When** 独立校验原创参考输入，**Then** 正例包含 ECU_EXTRACT/ReferenceEcu、单 EchoApplication、完整类型/端口/事件、通信和 ECUC 映射及定义来源，XSD 通过，**And** 参数与固定剖面一致，语义预期独立记录，不用当前旧主机导入成功替代。
- **Given** 正例副本，**When** 分别注入目标不唯一、断链/DEST 错、方向/长度/类型冲突、TimingEvent/Alarm 不一致、多实例/重入/未选变体、缺 BSW/服务使用端及排队/隐式/mode/transformer，**Then** 每个反例有文件/对象路径、错误类别和预期拒绝，**And** 分开列明 XSD 结构失败与结构有效但语义应拒绝的样例。
- **Given** 多文件引用及无关有效内容，**When** 建立样例来源和完整性清单，**Then** 保留文件边界与原始 SHA-256，不含官方原件/密钥/状态文件，**And** 缺 XSD 标 not_run/阻断，不能伪造验证通过。

### Story 4.2: 固定标准义务、依赖来源及独立验收预期

As a 参考 ECU 验收工程师，
I want 固定实际接口、依赖权利和独立预期，
So that 实现者和复核者不会把局部实验当成等级能力或交付证据。

**工作包／依赖：**W0；4.1。**覆盖：**FR-4/FR-6/FR-14/FR-15，E4-AR-5/12/15、E4-NFR-6/7。**产物：**固定源码、许可、补丁与工具链身份，独立 CAN/UDS/OS 轨迹及负例测试夹具。**执行入口：**`epic4_independent_oracle_contracts`。

**Acceptance Criteria:**

- **Given** 核验过的 R24-11 官方资料、OSEK 接口和固定依赖，**When** 确定产品接口与独立预期，**Then** 保留参考配置及 SC1 的 BCC1/BCC2/ECC1/ECC2、至少八软件 Counter/二 ScheduleTable、栈/ISR/错误/Hook/ARTI 产品范围，**And** 必要输入、版次、依赖身份和许可按其工程用途保存。
- **Given** 固定 FreeRTOS commit 与 GCC 16.1.0，**When** 核对归档、关键源码、许可和研究补丁，**Then** 固定真实摘要及来源/通知，研究补丁与未来生产补丁分列，**And** 不重新比较 OS，不在构建阶段取浮动源码；无法确认权利的交付项明确阻断打包。
- **Given** 集成契约 §6 与 OS 规范，**When** 建立独立预期，**Then** 固定 CAN/UDS 字节、初值/epoch 30 边界、FIFO 两种请求次序、非法调用/错误状态/栈关闭轨迹，**And** 每个向量标预期来源、输入和判定条件，生成器与被测调度实现不能重写预期；未固定的诊断计时/响应先依据已声明配置补齐并核对，不能凭空选择。

### Story 4.3: 建立固定内核的唯一 backend 与静态生命周期基础

As a 主机 ECU 集成工程师，
I want 使用固定内核和清晰隔离的汽车 OS backend，
So that 后续语义测试具有可复建的实际运行基础。

**工作包／依赖：**W1；最终契约、固定内核/GCC/许可，可独立于 W0 开始。**覆盖：**FR-6/FR-4/FR-15，AD-8，E4-NFR-4/7/8。**产物：**固定内核与可审阅生产补丁基线、标准 OS 接口/唯一 backend、静态对象配置、无 BSW 的 C99 生命周期 harness、来源与 ABI 档案。**执行入口：**`epic4_backend_lifecycle`。

**Acceptance Criteria:**

- **Given** 固定 Windows x64/GCC 来源组合，**When** 离线构建并启动最小静态 OS 配置，**Then** FreeRTOS 是唯一实际 ready/Running/上下文调度所有者，标准上层不直接调用 FreeRTOS/Windows API，**And** 生产补丁限于核定的策略/原子转换/端口边界，无第二个 runnable 选择器。
- **Given** AUTOSTART/启动模式与标准 StartOS/ShutdownOS，**When** 正常启动、关闭或注入资源建立失败，**Then** 记录 Initializing/Ready/Failed 和标准调用状态，失败关闭且拒绝后续输入/tick，**And** StartOS 没有新增私有返回值，隐藏任务、原生线程/事件及实际分配记录独立可查。
- **Given** 缺依赖/摘要不符、非法对象 ID 或对象容量越界，**When** 准备/执行配置，**Then** 缺来源/配置在启动前拒绝、非法服务按标准状态拒绝，**And** 无 BSW harness 可独立验证，不等待后续 RTE/应用故事；不据此声明 SC1。

### Story 4.4: 保留全部激活请求 FIFO 与实际内核状态

As a OS 验收工程师，
I want 所有激活请求按规范顺序执行并可对照内核状态，
So that 同优先级任务不会因重复激活或抢占恢复而乱序。

**工作包／依赖：**W1；4.3。**覆盖：**FR-6，E4-AR-6、AD-8。**产物：**Activate/GetState 转换、完整请求记录/容量、有限 ready 政策补丁和独立调度轨迹。**执行入口：**`epic4_activation_fifo`。

**Acceptance Criteria:**

- **Given** 同级 Basic A/B 和高优先级 H，**When** 分别提交 A1→A2→B1 与 A1→B1→A2，并在 A1 中被 H 抢占，**Then** 依请求顺序得到不同且符合规范的执行序列，恢复当前实例位置，**And** 每个请求恰执行一次；不以计数＋终止时无条件尾插替代 FIFO。
- **Given** Basic 多次激活及 Extended 单激活配置，**When** 达到/超过合法激活上限或传非法 ID/调用上下文，**Then** 合法请求完整保存、超限/非法请求返回适用标准状态，**And** 拒绝前后请求队列及运行实例状态不变。
- **Given** 独立 observer 在转换边界读取状态，**When** 激活、抢占或恢复，**Then** 汽车元数据与 backend 实际 Ready/Running 一致，**And** 不存在可见的半转换、额外 runnable 调度或由宿主线程轮询制造的顺序。

### Story 4.5: 原子终止、链式移交与新入口执行

As a OS 应用开发者，
I want 合法 TerminateTask/ChainTask 不返回旧帧，错误则保留原状态，
So that 每次任务实例从正确入口执行且失败不会丢失激活。

**工作包／依赖：**W1；4.4。**覆盖：**FR-6/FR-15，E4-AR-6。**产物：**Finish/Chain 临界转换、C99 入口重新开始策略、错误回滚和独立向量。**执行入口：**`epic4_finish_chain_atomicity`。

**Acceptance Criteria:**

- **Given** 终止、自链和向高/同/低优先级任务链的合法配置，**When** 调用服务，**Then** 不执行调用后的旧帧哨兵，后续实例局部变量重新初始化，完整请求顺序和目标事件初始化符合规范，**And** 所有入口/栈重新进入机制封装在 backend，应用不依赖 setjmp 或内核私有 API。
- **Given** 目标无效/激活满、源持资源或非法调用层级，**When** Chain/Terminate 拒绝，**Then** 返回对应标准状态并保留源实例及目标激活、事件、资源、ready 位置，**And** 不先结束源再尝试可能失败的目标请求。
- **Given** observer 与额外激活在转换边界交错，**When** 原子 Finish/Chain 完成，**Then** 只观察完整前态或后态，所有请求无丢失/重复，**And** missing-end 的最低失败处理可观察；完整 Hook/清理能力归 4.18，不作为本故事启动前提。

### Story 4.6: 实现非抢占、内部资源及资源上限恢复

As a OS 配置工程师，
I want 显式调度点与资源 ceiling/LIFO 遵守汽车 OS 契约，
So that 任务在共享资源与非抢占配置下保持正确执行顺序。

**工作包／依赖：**W1；4.5。**覆盖：**FR-6，E4-AR-6。**产物：**SetEffectivePriority、Resource/内部资源及 Schedule 路径、独立嵌套资源/混合抢占配置。**执行入口：**`epic4_resource_and_preemption`。

**Acceptance Criteria:**

- **Given** FULL/NON 任务和内部资源，**When** 高优先级激活及 Schedule/等待/终止边界，**Then** 抢占与内部 ceiling 释放/重新获取逐项匹配规范预期，**And** 不用全局调度锁代替逐任务属性。
- **Given** A/B 同级、H 高级及嵌套/同 ceiling 资源，**When** 获取和按 LIFO 释放资源，**Then** 有效优先级与当前实例 ready 位置正确恢复，独立研究反例不再复现，**And** 不将 FreeRTOS mutex 优先级继承当 ceiling 证明。
- **Given** 逆序释放、非所有者释放、持资源等待/结束及禁止上下文，**When** 调用服务，**Then** 对应标准错误和前态保留可复验，**And** task/模拟 Category2 ISR 的核心资源边界分别记录，完整中断义务在 4.18 补齐。

### Story 4.7: 保持事件所有权与无丢失唤醒

As a 主机 ECU 集成工程师，
I want 输入与事件的发布/等待边界不会丢失记录或唤醒，
So that 即使事件合并，应用和 BSW 仍能处理全部已接纳工作。

**工作包／依赖：**W1；4.6。**覆盖：**FR-6/FR-15，E4-AR-6/7/8。**产物：**Wait/Wake 与独立 event mask、最小 mailbox/ticket 发布协议、模拟 ISR 核心调用层级向量。**执行入口：**`epic4_event_wakeup_races`。

**Acceptance Criteria:**

- **Given** Extended 任务、自有事件 mask 及 Ev_Work/Ev_App/Ev_IO，**When** 先置位再连续 Wait、或等待后置位，**Then** Wait 不消费事件、只有合法所有者 Clear，按新实例规则初始化，**And** Basic/错误上下文/非法对象请求按适用标准状态拒绝。
- **Given** 完整记录先发布再置事件，**When** 控制注入 GetEvent/ClearEvent 之前、之间、之后及判空→Wait 的交错，**Then** 每条记录和 ticket 恰消费一次，任务不会带未处理记录睡眠，**And** 多次 SetEvent 合并不合并消息数或确认数。
- **Given** 模拟 Category1/2 ISR 与任务服务路径，**When** 调用允许/禁止的 event/resource 服务，**Then** 调用层级与原子 ready 转换符合对应矩阵，**And** bridge/ISR 不访问应用/BSW 状态；不声称真实 MCU 中断验证。

### Story 4.8: 逐毫秒推进受控时间与 Counter/Alarm

As a 独立主机测试者，
I want 每个逻辑毫秒有独立请求和完成确认，
So that 周期、deadline 和同刻事件不会被宿主 tick 合并或墙钟改变。

**工作包／依赖：**W1；4.7。**覆盖：**FR-6/FR-12，E4-AR-7/8。**产物：**AdvanceOneTick、Counter/Alarm 核心服务、controlled_logical_ms 端口、独立时间 harness 与剩余等级义务表。**执行入口：**`epic4_controlled_tick_and_alarm`。

**Acceptance Criteria:**

- **Given** 关闭自主 timer、NORMAL 主机优先级和唯一 tick ticket，**When** 顺序请求 1–1000 ms 并等待每次静止点，**Then** 内核 tick、Counter、动作和完成记录没有丢失/重复，**And** 拒绝并发未确认推进，不把多个 tick OR 入 pending bitmask。
- **Given** SystemCounter max=65535、Alarm_Work 1/1 和 Alarm_App 10/10，**When** 跨 10 ms、65535→0 及 32 bit 内核 tick 回绕的可执行边界配置，**Then** 周期/偏移、Counter/GetElapsedValue 和模数换算正确，**And** uint64 epoch 不回绕、溢出拒绝；边界配置不能只篡改预期标志来冒充端口行为。
- **Given** epoch 0、相同 epoch 或未 Ready/已 Failed，**When** 请求时间/Alarm 服务，**Then** 无额外周期或启动前推进、合法 Alarm/错误参数/重复设置按标准状态判定，**And** 宿主 watchdog 不作为汽车时间；八 Counter/二 ScheduleTable 全能力在 4.17 交付。

### Story 4.9: 捕获 Windows 真实执行栈故障并关闭目标

As a 主机目标维护者，
I want 真实原生线程栈故障可被记录并不可恢复地关闭，
So that Windows 目标的栈义务有实际证据并能提前止损。

**工作包／依赖：**W1；4.3，可在其他 W1 服务之前验证。**覆盖：**FR-6/FR-15，E4-AR-11，SWS_Os_00067/00068/00396。**产物：**线程实际栈档案、边界/SP 观测、异常捕获 shim、预分配故障记录/独立控制栈及故障子进程证据。**执行入口：**`epic4_native_stack_fault_shutdown`。

**Acceptance Criteria:**

- **Given** 按目标配置创建的原生线程，**When** 注册/切换/OS 边界观测，**Then** 记录实际 reserve/commit/guard、SP/区域及宿主对齐容量，**And** 与 FreeRTOS buffer、标准 ECUC 栈参数区分；正常运行无错误关闭。
- **Given** 独立进程中的真实栈消耗或边界破坏，**When** 触发实际异常/栈故障，**Then** 捕获真实记录、禁止继续调度，以 E_OS_STACKFAULT 进入 ShutdownOS/配置 ShutdownHook，**And** 不执行故障后应用哨兵、不等待损坏线程普通锁、不在损坏栈调用 BSW/分配/恢复；超时、崩溃或未捕获均判失败。
- **Given** 同目标模拟 Category2 ISR 实际使用的线程/控制栈及错误初始化路径，**When** 验证其适用故障边界，**Then** 明确每种真实执行栈监测/处理覆盖，**And** flag/fake buffer 注入不能计通过；机制被证实无法满足时阻断 W3并按停止条件报告，不删栈义务或改 OS 选型。

### Story 4.10: 从标准输入建立唯一校验集成计划

As a ECU 集成工程师，
I want 工具从受支持输入建立可检查且闭合的唯一计划，
So that 生成器不会各自猜测对象、类型、服务和目标。

**工作包／依赖：**W2；4.1、4.2，与 W1 并行。**覆盖：**FR-1/FR-3/FR-15，E4-AR-1/2、UX-DR1。**产物：**按需扩展模型/解析、ValidatedIntegrationPlan、对象/符号生产者清单及定位诊断。**执行入口：**`epic4_validated_integration_plan`。

**Acceptance Criteria:**

- **Given** W0 正例及满足同剖面的名称/ID 合法变体，**When** 解析并校验，**Then** 完整路径＋实例关联、角色/摘要、通信/类型/服务/任务/周期/符号/依赖形成只读计划，**And** 句柄与名称确定分配，不强迫用户复制参考 ID，不用短名合并不同对象。
- **Given** 4.1 的每个语义反例及符号归一化碰撞、缺/重复生产者、调度不唯一，**When** 建立计划，**Then** 明确文件/路径、输入错误/未支持/工具错误及补救说明，**And** 整体拒绝、不写工程、不以 SDG 补关系；未知无关有效内容仍保留。
- **Given** BSW 描述/ECUC 定义、实现和类型签名清单，**When** 验证服务使用端/提供端与依赖，**Then** 每个当前可生成契约符号有生产者或显式集成阶段责任，**And** W2 组件契约可独立校验，不能将尚未实现的 W1 runtime 冒称已链接；W3 才做全运行符号闭包。

### Story 4.11: 由 SWC 描述生成组件接口与同步服务类型

As a 应用 SWC 开发者，
I want 在 ECU 集成前获得由描述生成的确定组件契约，
So that 应用可按类型正确的标准 RTE API 编译。

**工作包／依赖：**W2；4.10。**覆盖：**FR-5，E4-AR-2/3/4。**产物：**Rte.h/Rte_Type.h、组件契约头、Runnable 声明、数据类型/服务 API 和编译验证桩；桩仅用于契约测试，不进入交付运行。**执行入口：**`epic4_component_contract_generation`。

**Acceptance Criteria:**

- **Given** 合法 SWC 端口/访问/类型/事件，**When** 契约生成并编译独立应用片段，**Then** Read 为 `Std_ReturnType Rte_Read_RxValue_Value(uint32* data)`、Write 为 `Std_ReturnType Rte_Write_TxValue_Value(uint32 data)`，类型与 Runnable 绑定一致，**And** 无私有 valid 参数、未声明 Invalidate 或从 Com 名字硬编码的接口。
- **Given** 同实例 ReadData 提供端、OperationInvokedEvent 及 Dcm 服务使用端，**When** 生成 C/S 契约，**Then** 固定 UINT8_N 的 uint8[4] 类型、同步 Std_ReturnType 接口和 Rte_Call 绑定一致，无 OpStatus/NRC 输出，**And** 正常服务器 E_OK、基础设施错误和不支持操作的边界有类型/拒绝测试。
- **Given** 缺类型映射、错误 OUT 参数/方向、异步/PENDING/跨 ECU 操作或命名碰撞，**When** 生成，**Then** 定位拒绝、旧输出不变，**And** 相同输入重复生成头文件集合与字节一致；只用组件描述和 W2 计划完成，不依赖未来运行故事。

### Story 4.12: 接入标准输入的安全编辑、保存与重开

As a 工作台使用者，
I want 查看标准输入和阻断位置并安全修改受支持参数，
So that 重开后仍能得到同一配置且原始有效内容不丢失。

**工作包／依赖：**W2；4.11。**覆盖：**FR-1/FR-2/FR-3/FR-15，UX-DR1/2，E4-NFR-2。**产物：**核心安全编辑/保存/预览与桌面连接、角色/来源/诊断呈现、往返测试；不设计全新布局。**执行入口：**`epic4_safe_input_roundtrip`；UI build 与无头交互检查，原生 IPC 另需隔离桌面。

**Acceptance Criteria:**

- **Given** 多文件参考输入与受支持 CAN/周期参数，**When** 检查角色、编辑并预览/保存/重开，**Then** 显示受影响文件与同一核心计划，受支持参数可追溯往返，**And** 未修改文件字节不变、有效无关对象/属性/引用保留，UI 不补隐含映射。
- **Given** 外部修改、保存中断、断链/类型错误、影响生成的变体或保留项反向引用风险，**When** 保存/计划检查，**Then** 拒绝并定位原因/补救，原输入或可恢复备份完整，**And** 不静默迁移、强制转换或宣称新配置已生成。
- **Given** 缺必要输入/合法外部 XSD，**When** 使用配置入口，**Then** 区分必需/可选/外部提供及不可生成状态，**And** 核心自动测试和 UI 构建通过；原生 Tauri/IPC 无隔离环境时记 not_run，不占用交互桌面。

### Story 4.13: 从唯一计划生成可链接 ECU 集成工程

As a ECU 集成工程师，
I want 同一计划产生匹配 backend 的 RTE/SchM/BSW/OS 工程，
So that 无需手补符号即可独立构建并检查任务/启动顺序。

**工作包／依赖：**W3；4.3–4.9、4.10–4.12 全部关键门通过。**覆盖：**FR-4/FR-5/FR-6/FR-15，E4-AR-2/7/9/13。**产物：**RTE 实现、SchM/BSW 配置与描述、OS 元数据/FreeRTOSConfig、任务入口、目标构建与完整清单。**执行入口：**`epic4_ecu_integration_generation`。

**Acceptance Criteria:**

- **Given** W1 通过记录和 W2 闭合计划，**When** 重复生成并移到新目录编译/链接，**Then** 标准 RTE/服务/BSW/OS 符号签名及生成/复用源码闭合、文件集合与路径无关内容一致，**And** 不混入 host-v1 Os/Rte 入口、无空壳或手补符号；使用 W1 独立控制 harness 验证启动，不等待 4.14 的文本 bridge。
- **Given** AUTOSTART Task_Ecu/两个 Alarm 与 BSW 周期描述，**When** 验证生成任务/StartupHook 及运行轨迹，**Then** 启动依赖、Ready 闭锁、唯一状态所有者和固定周期顺序符合契约，**And** StartupHook 不调用禁止的激活/事件/Alarm 服务；初始化每个阶段注入失败均发布 Failed 并关闭。
- **Given** 缺/重复周期入口、服务生产者不符、需要跨任务共享却无单执行者证明、时间来源冲突，**When** 集成生成，**Then** 整体拒绝且旧工程字节不变，**And** SchM 只在已证明单所有者处消锁，跨域 mailbox/OS 仍用各自原子协议。

### Story 4.14: 通过 HostBatchV1 运行并闭合输出确认

As a 独立主机测试者，
I want 按有界批驱动生成 ECU 并收到真实完成状态，
So that 提交成功意味着输入、周期、输出与确认均已闭合。

**工作包／依赖：**W3；4.13。**覆盖：**FR-6/FR-12/FR-15，E4-AR-8/9/10、E4-NFR-5。**产物：**受控 Windows bridge/BEGIN-RX-COMMIT、输出/确认 mailbox、ticket/序号、进程故障控制及可运行工程。**执行入口：**`epic4_host_batch_commit`。

**Acceptance Criteria:**

- **Given** Ready ECU 与有效批，**When** 提交 epoch 0、相同 epoch、跳到未来 epoch 及多消息批，**Then** 先逐毫秒推进中间 tick、目标 epoch 输入先于 tick，相同 epoch 不重跑周期，**And** COMMIT 只在任务重新等待、ticket 完成、队列清空且无未确认 PDU 时成功。
- **Given** 256/257 消息、1000/1001 ms 跨度、倒退/溢出/错误编码/未完成前新批，**When** 接纳批，**Then** 合法边界通过，非法批在汽车状态改变前整体拒绝，**And** 不丢最旧消息、不跳历史周期、epoch/序号来源全 ECU 一致。
- **Given** 宿主输出成功/失败、重复或不匹配确认、输出队列溢出、COMMIT 超过 5000 ms，**When** 执行，**Then** 仅实际输出成功才产生匹配 ticket/PDU 的确认，事件合并不丢确认；输出失败/溢出/watchdog 关闭目标，**And** I/O 在调度任务之外、不继续虚拟时间、不声称回滚已运行批，watchdog 不推进协议时间。

### Story 4.15: 运行同一应用实例的 S/R、快照和同步 DID

As a ECU 应用集成工程师，
I want 示例应用经生成 RTE 收发值并通过同步 DID 读取同一快照，
So that 通信与诊断确实来自应用而不是测试器直接改 Com。

**工作包／依赖：**W4；4.14。**覆盖：**FR-5/FR-8/FR-9，E4-AR-3/4/15。**产物：**原创 EchoApplication C99 实现、同实例服务器、RTE/Com/Dcm 状态转换及独立 CAN/UDS 轨迹。**执行入口：**`epic4_application_sr_cs_loop`。

**Acceptance Criteria:**

- **Given** Ready 后初始应用与尚未接收的 Rx，**When** 读取或在 10 ms 周期执行，**Then** Read 返回值 0/RTE_E_NEVER_RECEIVED，应用采用 0；初始 DID 为四字节 00，**And** 应用只经声明 RTE API 写 Tx、成功才提交值/epoch 快照，写失败保留原快照并记录标准状态。
- **Given** epoch 0 收到 `0x320/4: 78 56 34 12`，**When** epoch 10 应用执行及 0x22/0x1234 读取，**Then** Tx=`0x321/4: 78 56 34 12`、UDS=`62 12 34 12 34 56 78`、SF=`07 62 12 34 12 34 56 78`，**And** 默认/扩展会话均可读；服务器在同 Task_Ecu 同步执行，Dcm 不重复换端序或读私有 Com 值。
- **Given** 接收后到 epoch 30 未有新帧，或恰在 epoch 30 收到新有效帧并读 DID，**When** 运行该 tick，**Then** 前者 Read 保留原值但返回 MAX_AGE_EXCEEDED、应用采用/提交 0；后者先接收后 deadline，应用采用新值，**And** 同 tick Dcm 看到本次应用提交，不出现撕裂或第二状态所有者。

### Story 4.16: 独立验证边界、诊断恢复与旧目标回归

As a 独立 ECU 验收工程师，
I want 用外部预期覆盖新组合的失败路径并保留旧主机行为，
So that 一个成功回显不会掩盖时序、协议或兼容性缺陷。

**工作包／依赖：**W4；4.15、4.2。**覆盖：**FR-8/FR-9/FR-12/FR-15，E4-NFR-6、E4-AR-13/15。**产物：**独立 CAN/诊断测试器扩展、固定正反/恢复向量、old/new 目标分离验证及所用诊断参数/服务范围。**执行入口：**`epic4_independent_behavior_and_legacy_regression`。

**Acceptance Criteria:**

- **Given** 独立预期和不同接收值/多帧输入时序，**When** 运行新目标 CAN、deadline、同 tick 应用/DID 和输出确认场景，**Then** 字节/状态/提交 epoch 精确匹配，并拒绝错误 DLC/方向/状态，**And** 预期不通过调用同一生成器或同一打包算法得到。
- **Given** 声明物理连接的 0x10/0x3E/0x22、CanTp SF/FF/CF/FC、固定已核计时及非法请求，**When** 注入长度/会话/未知服务、序号和超时错误后再发合法请求，**Then** 对应 NRC、传输失败/恢复与 S3 行为逐条匹配独立目录，**And** 未选 0x2E/0x31/0x27/DTC/NvM 不自动加入新剖面；旧目标已具备行为按旧配置回归，不移除或升级其声明。
- **Given** 写入基础设施失败、输出错误/watchdog、批拒绝及进程停机，**When** 观察后续状态/重新启动，**Then** 不伪造正响应或成功 COMMIT、拒绝后旧状态或故障后的关闭边界准确，**And** 旧 host-v1 构建/离线向量单独通过或明确报告失败，不借旧结果替代新目标。

### Story 4.17: 完成 Counter/ScheduleTable 能力及 SC1 容量

As a OS 等级验收工程师，
I want 验证参考 ECU 未使用的适用计时能力与最低容量，
So that 少量 Alarm 场景不会被误当作完整 SC1 计时证明。

**工作包／依赖：**W5；4.8、4.2，可在 W4 之前独立推进。**覆盖：**FR-6，E4-AR-12。**产物：**剩余标准 Counter/Alarm/ScheduleTable 服务、状态机/配置生成支持、至少八 Counter/二 ScheduleTable 的独立能力配置。**执行入口：**`epic4_sc1_timing_capacity`。

**Acceptance Criteria:**

- **Given** 八个独立软件 Counter 与两个合法 ScheduleTable，**When** 编译/运行其容量配置，**Then** 真实创建/运行对象及所有适用接口通过，独立推进/回绕不串扰，**And** 不能仅检查数组长度或参考单 Counter。
- **Given** 相对/绝对启动、停止、状态、切换/链接、ExpiryPoint 及同刻动作，**When** 执行对应配置支持的路径，**Then** 次序、偏移、Counter 模数及状态转换匹配独立轨迹，**And** 显式审查适用同步策略，不以 FreeRTOS timer 改名代替。
- **Given** 零偏移/边界、非法对象/重复启动/非法状态/容量超限，**When** 配置或调用，**Then** 按标准约束拒绝并保持合法对象前态，**And** 组件 TimingEvent 的 Alarm/ExpiryPoint 配置接受及拒绝规则一致，上述行为有结果或明确未通过。

### Story 4.18: 补齐生命周期、Extended Status、Hook 和中断义务

As a OS 等级验收工程师，
I want 剩余错误、Hook、启动关闭和模拟中断义务有可执行实现与证据，
So that 非正常路径也遵守 SC1 的接口与运行契约。

**工作包／依赖：**W5；4.3–4.9、4.2。**覆盖：**FR-6/FR-15，E4-AR-12。**产物：**完整适用调用层级/错误/Hook/中断配对与生命周期路径、BCC1/BCC2/ECC1/ECC2 能力和最小容量向量。**执行入口：**`epic4_standard_calling_context`、`epic4_standard_error_hook_parameters`、`epic4_standard_interrupt_pairing`、`epic4_real_task_hook_transitions`、`epic4_sc1_class_capacity`及父规格中的其余正式生命周期／配置入口。

**Acceptance Criteria:**

- **Given** 完整产品能力/容量与标准调用表，**When** 执行四一致性类、抢占/非抢占、激活/事件/资源及错误路径，**Then** 各接口、最低任务/资源/事件/内部资源容量与元数据/内核状态均匹配，**And** 所有遗漏项由本故事补实现和独立结果，不能因参考配置未用而 N/A。
- **Given** 启动模式、Startup/Shutdown/Error/PreTask/PostTask 等适用 Hook 与 missing-end，**When** 注入正常/非法层级、嵌套限制、未结束返回及错误关闭，**Then** Hook 顺序、错误参数、清理/关闭行为符合条款，**And** 不把 StartOS 当私有可返回初始化函数，重复启动策略明确；未配置 ProtectionHook 的栈路径仍经 4.9 实际关闭。
- **Given** 模拟 Category1/2 ISR、禁中断/挂起恢复配对与资源屏蔽，**When** 运行嵌套、错误配对和合法/非法服务，**Then** 标准状态及原子边界有独立证据，**And** 主机只声明逻辑模拟，不用 Windows mutex 推定硬件 ISR/实时性；任何主机适用未跑项阻断等级出口。

### Story 4.19: 生成并验证 R24-11 ARTI 描述与 Hook

As a OS 维护与验收工程师，
I want 生成的 ARTI 工件与实际对象和状态对应，
So that R24-11 可观察性义务有真实实现而非仅有私有日志。

**工作包／依赖：**W5；4.17、4.18。**覆盖：**FR-6/FR-14，SWS_Os_00858/00829/00836/00837 与对应适用条款。**产物：**ARTI/ORTI 描述、适用 Hook、对象/状态绑定及独立一致性向量。**执行入口：**`epic4_arti_description_and_hooks`。

**Acceptance Criteria:**

- **Given** 已生成的真实 OS 对象及 R24-11 定义，**When** 生成/检查描述和适用 Hook，**Then** 对象身份、容器、接口、符号及版次与计划/实现一致，**And** 不用旧日志宏替代标准工件，不输出不存在对象。
- **Given** 独立任务、事件、资源、计时及错误轨迹，**When** 记录实际 Hook/状态，**Then** 描述指向的状态与 backend 实际状态一致，**And** Hook 不改变调度顺序、漏/重复通知或伪造 Running。
- **Given** 断开的描述引用/签名、缺适用 Hook 或无法采集的条款，**When** 校验和证据审查，**Then** 明确拒绝或未通过、不提升 SC1 声明，**And** 每项 N/A 有具体配置/条款理由。

### Story 4.20: 关闭适用 C 工件、模块描述和静态质量证据

As a 工程交付验收工程师，
I want 生成及复用产物逐项满足其适用接口、描述与质量义务，
So that 可构建工程不会被误称为已经通过全部规范门。

**工作包／依赖：**W5；4.16–4.19。**覆盖：**FR-4/FR-14，E4-NFR-3/6。**产物：**必要的 C 接口、模块文档、BSWMD/RTE 描述与内存段修正。**执行入口：**`epic4_generated_artifact_obligations`；验证与复核结果记录在对应 BMad story。

**Acceptance Criteria:**

- **Given** 全生成/复用文件及各文件的实际模块角色，**When** 核对 BSW API/类型/命名/配置、模块文档、BSWMD 和 RTE 生成产物描述，**Then** 条款→文件/符号对应且 C99 编译链接通过，**And** 条件适用配置源按真实参数产生，不增空源或无依据 Compiler.h；所需修正与复测留证。
- **Given** 适用 MemMap、MISRA C:2012 和声明主机目标，**When** 执行静态分析及段/链接检查，**Then** 在BMad story记录实际检查结果、问题位置与技术理由，**And** 主机专属代码、应用、BSW、生成 RTE 责任分别判断，不以 warning-free 或历史扫描替代。
- **Given** 本次C实现与生成工件，**When** 按BMad验证和复核，**Then** 实际接口、构建与预期行为成立，**And** 发现的问题修正并复验，无法验证的必要行为如实记录。

### Story 4.21: 交付新目标可重建包与准确工作台状态

As a ECU 工程接收者，
I want 获得完整新目标包和明确的验证状态，
So that 可以在新目录重建并离线复验而无需仓库隐含文件。

**工作包／依赖：**W5；4.20。**覆盖：**FR-4/FR-12/FR-14/FR-15，R6/R7、UX-DR3/4、E4-AR-13。**产物：**显式版本化交接格式/来源清单、应用/BSW/内核/补丁/目标闭包、build/verify 入口与独立测试器、工作台预览及阶段状态连接。**执行入口：**`epic4_independent_handoff`（4.22合并维护同一严格消费者）；原生 UI/IPC 另在隔离桌面留证。

**Acceptance Criteria:**

- **Given** 已保存、校验且来源/权利闭合的新目标工程，**When** 明确导出可重建包并在新目录重导入/再生成/构建/离线复验，**Then** 文件身份与原字节摘要完整、生成源码稳定、CAN/DID/失败向量匹配，**And** 所有依赖实际随许可交付或明确合法取得方式，构建无浮动下载/源机器绝对路径。
- **Given** 脏输入/外部变更/缺文件/篡改/修改旧输出/依赖权利不明，**When** 导出或重建，**Then** 定位失败并保留原输入/旧包，不额外打包密钥/运行状态/官方原件，**And** 新格式显式版本化，host-v1 包读取及旧离线入口回归通过。
- **Given** 生成成功但构建/行为/等级证据缺失或失败，**When** 工作台及交接清单呈现结果，**Then** 分开显示保存/校验/生成/构建/主机行为/等级审查/实机状态，有界集成不显示完整 SC1，**And** 无法隔离验证的原生流程记 not_run，不能用 UI 构建或核心测试代替用户流程证据。

### Story 4.22: 独立复核完整等级与工程交接出口

As a 产品维护者，
I want 非实现者复核全部适用等级证据和独立交接，
So that Epic 4 的完成结论由可复现结果支持。

**工作包／依赖：**W5；4.21及所有前置结果。**覆盖：**FR-6/FR-12/FR-14/FR-15，E4-NFR-6/8、AD-7/9。**产物：**BMad独立复核、可重建交接复验和准确产品边界；只有获授权的状态更新才执行。**执行入口：**`epic4_independent_handoff`，并由非实现者在新目录重复交接复验和对应 story 的独立行为测试。

**Acceptance Criteria:**

- **Given** 所有前置 story 的规格、实现与验证结果，**When** 非实现者核对并重跑，**Then** 每项条款、配置、实现和结果可双向追踪，FIFO/事件/资源/计时/栈/错误/Hook/ARTI 与最低容量全部有真实证据，**And** 未覆盖项不能靠评审文字关闭，须返回责任故事补实现/复验；研究观测不计生产通过。
- **Given** 新目录与声明的合法固定依赖，**When** 重导入、生成、独立编译链接、CAN/UDS/故障及隔离用户流程复验，**Then** 输入/产物完整性与独立预期一致，实际命令/退出码/故障记录可复现，**And** 必要产品功能未完成或其实际验证失败时不判 Epic done。
- **Given** 主机等级和交接复核结果，**When** 给出出口与声明，**Then** 按全部story的功能与BMad验证／复核结果确定Epic完成，否则准确保留有界成果和阻断项，**And** 不继承 MCU/硬实时/ASIL/量产/官方一致性或公开发布许可，不擅自改 NM/存储及后续 Epic 范围。

### 故事覆盖核对（规划）

| 提取项 | 承担故事 |
| --- | --- |
| FR-1 | 4.1、4.10、4.12 |
| FR-2 | 4.12、4.21 |
| FR-3 | 4.1、4.10–4.13 |
| FR-4 | 4.2、4.3、4.13、4.20、4.21 |
| FR-5 | 4.11、4.13、4.15 |
| FR-6 | 4.2、4.3–4.9、4.13、4.14、4.17–4.19、4.22 |
| FR-8 | 4.15、4.16 |
| FR-9（本范围） | 4.15、4.16 |
| FR-12（本范围） | 4.8、4.14、4.16、4.21、4.22 |
| FR-14 | 4.2、4.19–4.22 |
| FR-15 | 4.1–4.3、4.5、4.7、4.9、4.10、4.12–4.14、4.16、4.18、4.21、4.22 |
| E4-NFR-1/2/3/4 | 4.11/4.13/4.21；4.1/4.12/4.21；4.20；4.3/4.13 |
| E4-NFR-5/6/7/8 | 4.14；4.2/4.16/4.20/4.22；4.1–4.3/4.21；4.9/4.12/4.21/4.22 |
| E4-AR-1/2/3/4/5 | 4.1/4.10；4.10/4.11/4.13；4.11/4.15；4.11/4.15；4.2/4.3 |
| E4-AR-6/7/8/9/10 | 4.4–4.7；4.3/4.8/4.13；4.7/4.8/4.14；4.13/4.14；4.14 |
| E4-AR-11/12/13/14/15 | 4.9；4.2/4.17–4.19/4.22；4.13/4.16/4.21；故事依赖表及 W1→W3 门；4.2/4.15/4.16 |
| UX-DR1/2/3/4 | 4.10/4.12；4.12；4.21/4.22；4.21 |

这张表证明 Epic 4 规划责任已分配，不代表其当时的实施覆盖。FR-7/FR-10/FR-11/FR-13 和 FR-9/FR-12 的后续范围按本文件的旧 Epic 5／6 与候选承接关系细化。Epic 4 的所有故事依赖指向已有基础或较早编号，4.9、W2 与 W5 独立 OS 能力的提前执行仅按显式依赖允许，不改变 W3 的关键门。

## Epic 5: 单网络管理与故障持久化闭合主机参考 ECU

用户成果：在 Epic 4 的同一主机工程中，通信使用者可请求和释放单 CAN 网络，独立对端能观察 NM PDU、模式转换、睡眠/唤醒及异常恢复；一个受监测故障可经声明的诊断子集读取、清除，并按主机存储策略在重启后复验。此时才可把主机结果称为“声明配置下的完整主机参考 ECU”，仍不声称指定 MCU 已通过。

原定支持边界：ComM、Nm、CanNm、CanSM 及下层 CanIf/Can 的调用与回调形成真实闭环；只覆盖明确列出的单网络变体、诊断服务、一个 DTC 与主机文件存储。部分网络、网关协调、其他总线 NM、真实 Flash 及未声明的 UDS/DTC 变体留待后续决策。对应 FR-8–FR-12、FR-14–FR-15。新版路线由 R7 接收诊断和历史 Epic 2 的双故障义务，R8 接收 NM／生命周期；此处保留原退出范围，不将一个 DTC 改写成通用诊断容量，也不与 R7 重复实施。

关键依赖：Epic 4 的应用、RTE、OS 与 CAN/诊断运行链；按 R24-11 SWS/ECUC 核定的 NM/诊断/存储配置与独立报文预期。退出条件：非实现者在同一交接工程中复现网络请求与释放、NM 报文、睡眠/唤醒、bus-off 或唤醒失败的恢复/拒绝、DTC 触发/读取/清除及跨进程状态；配置往返、独立构建、错误输入和story 验收通过。不得以直接切换 Can 控制器状态或常开 CAN 代替网络管理验收。

## Epic 6: 指定 MCU 上交付完整参考 ECU

用户成果：ECU 集成工程师沿同一声明的配置及应用接口，生成含目标依赖的工程；另一位工程师用指定板卡、合法工具链和真实对端完成独立构建、上板及复验。硬件行为和资源证据单独形成目标档案，不从 Windows 主机结果继承。

支持边界：只声明一个已核定 MCU/板卡、OS 实现与端口、MCAL/驱动、工具链及参考 CAN/诊断/NM 配置。首个评估候选 FRDM-A-S32K344 尚不是已选目标；供应商 RTD 与项目 R24-11 的版次及接口差异须显式核查，驱动许可和再分发权单独记录。其他 MCU、功能安全、完整 MISRA、第三方互操作及官方一致性不随一次上板自动成立。对应 FR-7–FR-14、FR-15。

关键依赖：Epic 5 的完整主机参考成果；板卡、OS/驱动/工具链的合法来源和固定版本；真实 CAN、诊断、存储、唤醒与故障注入对端。退出条件：非实现者按交接清单在指定目标复现应用 CAN 信号、物理诊断、故障保存/重启/清除、网络请求/释放与睡眠/唤醒，以及适用的中断、时序、资源和异常恢复；目标证据与主机证据分开审查。购买或绑定板级目标前先关闭目标档案决策门。


**历史调查（2026-09-28，非当前状态）：**固定 V11.3.1 实际 commit `054e14f3397023aa83813a65aa065fc4597d481b` 和 GCC 16.1.0，在隔离 Windows 后台进程复现策略反例并验证有限策略扩展与 C99 重启；详见 [调查与采用条件](architecture/epic-4/FREERTOS-FEASIBILITY.md)。当时激活顺序、真实栈、错误／ISR／计时／Hook 和完整证据尚未闭合，故事未就绪。此后 4.1–4.22 已完成生产实现、BMad 验证与独立复核；旧调查缺口不作为当前未完成事项，也不为 R5–R29 提供实施就绪判断。

## R5 实施输入与要求提取

来源为 PRD 的“R5 选定范围与完成契约”及本次软件自有内置校验授权；本轮只修订 Epic 7，不编码、不推送。全局 PRD 的 draft 由远期未选定范围造成，R5 需求范围已选定，本次修订的就绪／放行结论由独立最终复核确定。既有 Epic 1–6、完成状态、旧验收和 R7 承接需求原样保留。

### Functional Requirements — selected R5

- CFG-1.1：无需官方 XSD/MOD、编译器或联网即可进入工作台，选择并安全检查真实多文件输入及原文；内置规则执行结果、支持覆盖和未执行范围如实区分。
- CFG-1.2：原创空 CAN、信号与标准 ECU 模板经文件预览建立完整工程，不覆盖目录、不含官方档案或个人路径。
- CFG-1.3：工程成员与剖面可重启／搬移恢复，原 ARXML 和两类交接入口仍按真实来源、规则身份及完整性打开；旧 v1 严格兼容不改写历史要求。
- CFG-2.1：真实模块／容器／参数／引用统一身份、文件归属、名称／路径／定义／值搜索与焦点。
- CFG-2.2：软件内置目录覆盖全部产品受支持 BSW／配置剖面的类型、范围、基数、默认来源与引用语义，支持真实多模块通用编辑；应用不部分修改，不从 VALUE 猜类型。
- CFG-2.3：真实合法引用候选与跳转，定义允许的实例创建／改名／移除；入站引用在同批解决，未知影响拒绝。
- CFG-2.4：可预览多对象变化及关系影响，以输入／RuleSetIdentity／定义身份全部应用或全部拒绝，允许安全修复无关旧问题。
- CFG-2.5：原字节安全保存及真实问题接力，外部改源／失效确认／恢复备份拒绝；source-safety/schema/definition/target-generation 分域报告，schema 是明确支持范围的原生结构检查，不冒称完整官方 XSD 通过。
- CFG-3.1：版次／内置规则覆盖／模块定义／产品目标／执行工具分层能力；支持范围内无需用户注册官方资源，产品自有规则不可被用户设置替换；可选第三方定义显式按版次／摘要独立接受，不能覆盖内置目录，也不授予生成能力。
- CFG-3.2：外观／规则与模块定义／执行工具类别隔离设置；内置版本与覆盖只读，可选扩展导入／移除显式确认、相关结果失效，持久浅深主题不重置工程草稿；编译工具环境覆盖保留，XSD/MOD 环境不替代内置权威。
- CFG-4.1：有限剖面验证后纯源码预览／确认，生成与预检／构建／行为分别报告，未知生成语义拒绝。
- CFG-4.2：配置／用户应用／产品／生成／目标／构建拥有权明确，重复生成及升级不覆盖用户代码或不明旧产物。

### Non-Functional Requirements — selected R5

CFG-NFR-1：磁盘及原字节安全往返与关键拒绝。CFG-NFR-2：唯一状态、身份和受管取消／晚结果隔离，规则身份贯穿快照与确认。CFG-NFR-3：暂定桌面设计、统一导航、草稿、键盘／错误／焦点与图标。CFG-NFR-4：Windows/Linux 隔离真实 IPC，无官方档案／checkout／开发工具的离线配置及独立源码／行为交接，macOS 如实界定。CFG-NFR-5：输入安全边界、非 GUI 线程重任务、快照索引复用及有界日志；开发时用合法固定版官方 oracle 对比，不将其变成用户安装依赖。

### Additional Architecture Requirements — selected R5

AD-1–AD-11 位于 [Epic 7 spine](architecture/epic-7/ARCHITECTURE-SPINE.md)，约束原字节权威、安全源投影、唯一不透明身份、内置结构规则与定义／生成能力分离、原子 ChangeSet、PreparedSave、成员 manifest 与原创模板、单 reducer／Session、生成拥有权、规则库存及可选定义缓存与隔离分发。`BuiltinRuleSet` 是产品作者编写的校验代码与不可变自有模块元数据，禁止把官方 XSD/MOD 的原始／加密 XML 或机械转换嵌入代码充当授权替代品；官方资源仅是合法持有的开发参考／oracle，不虚构法律许可。确定性打包库存绑定实现来源及内置元数据，并在产品构建／启动核验；`RuleSetIdentity={release,rulesVersion,sha256}` 的摘要标识产品规则库存／实现来源而非官方压缩包。ProjectProjection、当前能力及输入 fingerprint 纳入 ruleSetIdentity；definitionFingerprint 仍是不透明的内置定义及已明确接受扩展目录身份，禁止前端业务 ID 或 JS 数值往返。不是新的绿色项目，不引入 starter、云后端、第二调度器或配置库。

### UX Design Requirements — selected R5

- UX7-1：根 DESIGN 的明暗 tokens、系统／等宽字体、13px 正文与角色色、紧凑树表检查器；1080×720 主链可操作，窄屏只阅读回退。
- UX7-2：统一菜单／操作条／工具轨、一棵对象与文件树、复用文档标签、属性／引用检查器、底部问题／生成／构建／主机验证／日志和状态栏，不另设孤立工作区。
- UX7-3：稳定对象选择／筛选、真实只读原文／路径复制、字段 label／单位／范围／错误，真实引用与问题焦点接力。
- UX7-4：对象、批次、标签、工程切换、重开／交接与退出共用草稿／dirty 保护；应用失败不继续，非破坏面板和主题保留输入。
- UX7-5：保存与生成的逐文件真实差异／拥有权预览、不同确认动词、失效身份及冲突；预览不写入、不启动编译。
- UX7-6：外观／规则与模块定义／执行工具类别独立草稿与保存，内置规则版本／覆盖只读，可选第三方定义用原生文件选择显式导入／移除及版次／摘要反馈；官方 XSD/MOD 不属于普通用户设置动作，旧外部资源设置仅为兼容／开发用途且不替代内置规则；执行工具环境覆盖语义保留。透明珊瑚 A、明暗灯位、固定通用桌面图形与正式 favicon／窗口／安装包同步。
- UX7-7：公共命令与 Ctrl/Cmd+K/S/O、Alt+1/4/9、Escape，树键盘／可访问名称、选择非颜色提示、label／aria-invalid／describedby、焦点圈定及归还、温和状态播报与 reduced motion。
- UX7-8：当前 CAN、诊断／DTC、标准输入与各交付／构建／运行真实动作全部迁入新壳；原型 setter／路径／计时器不能替代 IPC，能力与取消／失效理由就近显示。

### R5 成果分组

**Epic 7：可独立使用的统一 ECU 工程配置工作台。**集成工程师从真实 ARXML 或原创模板建立工程，无需下载／注册官方 XSD/MOD，即可凭软件自有规则与全部受支持 BSW 定义导航、编辑、批量修复、安全保存和验证，再独立交接受支持的源码；设置与配置不被缺编译器锁住。可选第三方定义是隔离扩展，其缺失／无效仅影响消费者，不覆盖内置定义或授予生成能力。覆盖全部 CFG-1–CFG-4、CFG-NFR-1–CFG-NFR-5 和 UX7-1–UX7-8。复用已完成 Epic 1／3／4，不依赖尚未实施的 R6／R7、旧 Epic 5／6 或 MCU。

一个用户成果 Epic 而非分别拆“数据库／API／UI”：源索引、事务、壳层与拥有权反复触及同一工作台，按可实际使用的纵向故事顺序推进。分阶段交付不取消完整 R5 退出。

## Epic 7: 可独立使用的统一 ECU 工程配置工作台

来源：候选 R5；覆盖 CFG-1–CFG-4、CFG-NFR-1–CFG-NFR-5、UX7-1–UX7-8。每个故事贯通实际核心、IPC 和所需界面，不拆出等待未来 UI／后端才能使用的空层。11 个 Story ID 和顺序依赖不变，仅指向之前故事；既有配置／生成／主机行为始终保留。四个校验域及目标 allowlist 不放宽：schema 只表示明确支持范围内的原生结构检查，未知／不支持覆盖必须明确，不声称完整标准通过；未知内容原字节保留、不安全语义只读，其消费语义不支持阻断有关生成而非所有浏览／安全修复。原型不是验收工具。

### Story 7.1: 以内置规则安全打开并检查真实工程

As a ECU 集成工程师,
I want 无需官方 XSD/MOD 和编译器即可用软件内置规则打开真实输入、检查结构并查看源文件与对象,
So that 我可以离线确认工程与真实问题，而不是被环境总门阻挡。

覆盖 CFG-1.1、CFG-2.1、CFG-3.1、CFG-NFR-1/2/5，UX7-3/8；AD-1/2/3/6/8/11。依赖：已完成的工程基础。

**Acceptance Criteria:**

- **Given** 未设置官方 XSD/MOD／工具、软件内置规则库存有效且选择原创 CAN、七文件标准 ECU 或合法未知保留内容，**When** 原生打开、选择文件／对象并检查，**Then** 同一 Workspace 返回真实源结构、epoch/source/object 身份、ruleSetIdentity 与原文，执行支持范围内的结构／顺序／基数／类型检查，界面区分实际结果、未执行与不支持覆盖及每动作原因，**And** 不造帧、关系、通过状态，不要求编译器／官方档案／联网，不改磁盘，不声称完整官方 XSD 校验通过。
- **Given** 相同显示名跨文件、无法识别剖面、坏引用或无效版次，**When** 浏览或定位诊断，**Then** 使用实际身份／归属区分对象，未知语义只读或明确阻断有关目标，**And** 不能由 UI 强选 CAN 绕过来源检查。
- **Given** 非良构、实体／DTD、50 MiB 以上单输入、危险／链接逃逸路径或工程外任意原文请求，**When** 打开／获取原文，**Then** 拒绝并保留原工程，**And** 无外部资源的合法源检查不放宽已有输入安全限制。
- **Given** 草稿、dirty 或正在执行的旧工程，**When** 新导入或原两类交接入口触发切换，**Then** 先处理保护并遵守旧 fingerprint／ProcessOwner，**And** 被取消或晚到的结果不替换当前输入。
- **Given** 受限大小内的多文件解析／源索引或主动取消场景，**When** 后台工作尚未结束时切换工具窗口、复制已知路径及请求取消，**Then** 原生 GUI 仍可操作，解析／全文扫描不占 GUI 主线程，取消按实际归属完成或显示未确认，**And** 长错误摘要有界、不假丢弃诊断，完整 owned 详情可按受管身份获取和复制。
- **Given** 确定性打包的内置规则实现／元数据及其 RuleSetIdentity，**When** 产品构建／启动核验发现库存缺失、损坏或不匹配，**Then** 将其报告为软件安装／工具错误并拒绝依赖该库存的校验、编辑确认及生成，不降级为“通过”，**And** 安全源查看仍遵守源安全边界，不要求用户下载／注册 XSD/MOD，不从设置、环境变量或拷贝的包元数据替换可执行规则。
- **Given** 开发者合法持有的固定 R24-11 官方 XSD 参考和独立真实正例／聚焦反例，**When** 对产品原生结构校验作开发 oracle 对比，**Then** 覆盖结构、顺序、基数、类型及已知不支持案例，差异按明确支持边界处理并保留原有效正反向测试，**And** 不压低原验收、不抑制错误、不以未运行冒充通过，oracle 不进入用户运行／安装依赖或内置 XML 副本。

### Story 7.2: 使用内置定义检查参数约束与真实引用

As a BSW 配置工程师,
I want 使用软件内置的受支持 BSW 定义查看参数约束、默认来源和引用，并按需接受独立第三方扩展,
So that 我知道哪些配置可以修改以及缺失的真实关系，不必注册官方模块档案。

覆盖 CFG-2.1/2.2/2.3、CFG-3.1、CFG-NFR-2/5，UX7-3；AD-3/4/10。依赖：7.1。

**Acceptance Criteria:**

- **Given** 软件内置 R24-11 定义目录及真实多模块嵌套配置，**When** 打开对象检查器，**Then** 无须官方 MOD 注册即可从不可变自有定义返回类型、范围、单位、基数、枚举、默认来源和可写原因；目录覆盖全部产品受支持 BSW／配置剖面而非少数字段，**And** 不联网跟随 schemaLocation、不从官方 XML 自动转换副本取代产品元数据，不把定义可读当成生成支持。
- **Given** integer/float/boolean/enumeration/string/function-name/reference 与未落盘默认值，**When** 展示描述符，**Then** wire 使用 kind/lexeme 而不是 JS number，默认与 explicit 不混合，**And** unknown expression／variation／instance-reference 明确只读，不猜类型或补值。
- **Given** 跨文件目标、错误 DEST、缺目标、重复路径与重复显示名，**When** 查看引用、入站关系、合法候选或跳转，**Then** 用同一后台索引及真实身份显示目标／问题，**And** 不能列不存在或错类型对象；定义可读不等于目标可生成。
- **Given** 已有规则／定义快照及相关草稿，**When** 显式接受／移除可选扩展或注册失败，**Then** 按版次／摘要隔离核对，同名覆盖内置、冲突、错版次／摘要明确拒绝；成功变化推进相关 definitionFingerprint 与能力／输入 fingerprint、使依赖结果失效且保留草稿供核对，失败保持旧目录，**And** 扩展缺失／无效仅影响其消费者；非 GUI 线程读取、按规则／定义摘要复用索引，用户不能修改或替换内置规则。
- **Given** 已明确接纳的第三方 catalog 及完整原字节成员，**When** 安装本机缓存、复用相同身份或关闭尚未建立 manifest 的 ARXML 会话，**Then** 以 owned staging 完整核验后发布到 app_config_dir 的摘要键不可变缓存，现存成员损坏拒绝；直接 ARXML 会话不从机器已安装集合隐式恢复接纳，**And** 只缓存定义原字节、不执行代码、不复制参数或通过状态，界面说明保存为工程可保留接纳选择；跨重启的工程接纳集合由 7.6 持久化，本故事不反向依赖它。
- **Given** 开发者合法持有的固定 R24-11 官方 MOD／XSD 参考、独立真实多模块配置及聚焦反例，**When** 对内置参数／定义与引用校验作开发 oracle 对比，**Then** 核对值域、默认来源、参数／引用类型、DEST／目标、基数及已有跨模块目标约束，并呈现已知不支持语义，**And** 保留合法旧测试与真实拒绝，不用仅有表单字段、缩小覆盖、抑制错误或“未运行”替代真实校验；官方参考不随用户软件以原始／加密／机械转换 XML 方式分发。

### Story 7.3: 按定义应用参数与引用并安全保存

As a BSW 配置工程师,
I want 修改定义允许的参数和普通引用后应用、预览保存并重开,
So that 实际 ARXML 对应我的配置，拒绝时输入不会被破坏。

覆盖 CFG-2.2、CFG-2.3、CFG-2.5、CFG-NFR-1/2，UX7-3/5；AD-4/5/6。依赖：7.2。

**Acceptance Criteria:**

- **Given** 可写描述符，**When** 编辑已支持类型及普通引用，经 prepareChange／applyChange 应用，**Then** 使用内部 op 标签、FieldRef、absent/explicit 和当前 capabilities.fingerprint／后台 definitionFingerprint，fingerprint 绑定当前 RuleSetIdentity，核对原值与定义约束后一次补丁发布，**And** 下一请求用返回 current token 而非 Reply 请求 echo；大整数保持准确，非有限 float／非法函数名拒绝，未改 lexeme 不重写。
- **Given** 省略值、默认值、显式清空或错误引用，**When** 应用，**Then** 区分各操作并只按明示动作 materialize，拒绝保留原工程与字段草稿，**And** 未支持定义没有伪可写入口。
- **Given** 应用后的配置与未改源文件，**When** 无官方档案／编译器的原生预览、确认保存及重新打开，**Then** 磁盘只改变对应条目、引用和必要结构，未改文件原字节一致，**And** 保存返回真实 dirty／分域验证状态，执行支持范围内内置检查；未执行或不支持范围不标通过、不让依赖验证的生成可用，不冒称完整官方 XSD 校验通过。
- **Given** 外部改源、旧预览、写入失败／未恢复备份或无关旧问题，**When** 保存或修复安全字段，**Then** 外部／写入风险拒绝并保留旧文件；按 source-safety/schema/definition/target-generation 分域核对，未变旧 witness 保留，新或恶化违规拒绝，**And** schema 域实际原生结构错误仍阻断保存，未知生成语义不误作所有安全编辑／保存门。

### Story 7.4: 预览并原子应用多对象参数与引用修改

As a 集成工程师,
I want 搜索并选中多个对象，预览批次的实际影响后一次应用,
So that 一组相关修正不会成功一半或留下断开的引用。

覆盖 CFG-2.1、CFG-2.4、CFG-NFR-1/2，UX7-3/4/5；AD-3/5/8。依赖：7.3。

**Acceptance Criteria:**

- **Given** 名称／路径／定义／值筛选的真实对象集，**When** prepareChange 跨文件批次，**Then** 后台返回逐项旧／新值、实际展开入站关系、绑定 RuleSetIdentity 的 input／definition token 与 changeRevision，预览不改内存／磁盘，**And** 筛选隐藏对象不丢选择／草稿；改草稿即使 Workspace 未变也使确认失效。
- **Given** 有效预览及全部合法修改，**When** 明确应用，**Then** 一次发布全部变化并推进一次 revision，返回同一选择与逐项结果，**And** 部分无变化项不产生伪修改；未知生成语义仍准确阻断有关目标。
- **Given** 一项越界／不匹配定义、重复或冲突目标、错 DEST、外部变化或失效预览，**When** 应用，**Then** 整批拒绝、源集合及旧 revision 不变，错误指向实际 change／field，**And** 不以逐项 invoke 拼出批次原子性。
- **Given** 已建立源／内置规则／定义快照，**When** 连续更换搜索词、清空筛选并反复展开／收起树或执行校验／全文扫描，**Then** 当前快照索引复用、非 GUI 线程重任务期间仍可操作／取消，**And** 不为筛选或展开重复读源／加载规则库存／解压可选定义档案；输入、规则或定义身份变化才重建相关索引。

### Story 7.5: 安全创建改名和移除配置实例

As a BSW 配置工程师,
I want 按定义维护模块与容器实例，并在同批处理引用,
So that 工程结构不被固定样例锁死，也不会产生悬空关系。

覆盖 CFG-2.3/2.4/2.5、CFG-NFR-1/2，UX7-3/4/5；AD-3/4/5/6。依赖：7.4。

**Acceptance Criteria:**

- **Given** 定义许可的模块／嵌套容器及源归属，**When** 在同批 create-instance、new FieldRef 必需值／子实例和指向 created ObjectRef 的引用预览后确认，**Then** 对 prospective graph 校核父子／choice／基数／名称并原子发布，返回 createdIds／createdFields 映射，可保存重开，**And** 不能先发布不完整父对象，默认值不隐式落盘、未知条件拒绝。
- **Given** 已被跨文件引用的对象，**When** 改名并确认实际入站影响，**Then** objectId 保持、显示路径和全部可核定引用一致更新，**And** 不改包含旧名字的无关文本／未知有效内容。
- **Given** 仍有入站引用或重名／不明引用影响，**When** 移除或改名，**Then** 未在同批解决的影响使整个变化拒绝；合法批次同时重定向或移除入站关系，**And** 没有自动级联删除未知内容或旧 ID 复用。
- **Given** 结构改动将导致已声明剖面规则错误，**When** 应用／保存／生成检查，**Then** 拒绝新约束违规或明确呈现该目标不支持，不替换生成器能力声明，**And** 重开与删除对象后旧草稿不能落到新实例。

### Story 7.6: 从原创模板建立可搬移的工程

As a ECU 集成工程师,
I want 新建工程、实例化模板并保存成员信息后重开或搬移,
So that 不依赖个人路径和参考样例，也无需每次手动找齐文件。

覆盖 CFG-1.2/1.3、CFG-3.1、CFG-4.2、CFG-NFR-1/4，UX7-4/5/8；AD-6/7/9。依赖：7.5。

**Acceptance Criteria:**

- **Given** can-empty-v1、can-signals-v1、standard-ecu-v1 和用户选择的新空目录，**When** 无官方档案／编译器时检查文件预览并确认合法工程名，**Then** 从产品原创输入创建完整源集合及 v1 成员 manifest，source roles／引用一致、支持范围内内置结构／定义检查可用，**And** 不含官方档案、密钥、成功日志或个人绝对路径；未执行范围明确，库存损坏报告软件错误而非缺用户资源。
- **Given** 成员 manifest、源文件及实际明确接纳的扩展集合，**When** 保存后重启或整体搬移打开，**Then** 按实际内容重识别剖面，恢复成员、归属及 acceptedExtensionDefinitions 的精确身份，从本机核验缓存重建 catalog／definitionFingerprint，**And** builtin-only 集合为空且无需扩展；缺失／损坏只限制实际消费者，保留源文件并等待明确接纳匹配原字节，不能默选其他版本或机器全局定义；hint、旧通过状态、坏版本、越界／链接、重复路径／角色均不获信任。
- **Given** 原来直接打开的不同目录 ARXML 文件集合和会话接纳选择，**When** 明确保存为新工程，**Then** 先预览复制映射到新空目录并列出扩展接纳身份，不改原文件；名字冲突明确拒绝／请求用户命名，**And** 成员、接纳集合与源保存用同一 PreparedSave 事务，未保存的接纳变化为 dirty，失败保留旧集合且不留下误认完成的部分工程。
- **Given** 两个不同 CAN 工程及七文件标准工程模板，**When** 无官方档案离线修改合法配置、重开、搬移并生成纯源码，**Then** 真实模型、引用、内置校验及已有生成剖面成立，**And** 不是固定结果复制或修改生成器；本故事不要求尚未实现的 v2 封存交接，无官方档案的新交接由 7.10／7.11 验收；原两类 v1 交接入口的兼容验证按各自原资源／完整性要求另验，不伪造历史摘要或默改格式。

### Story 7.7: 在统一工程壳层中使用全部真实操作

As a 集成工程师,
I want 在同一树表检查器与文档工具窗口内完成配置及交付,
So that 切页不会变成另一套工程或丢失操作上下文。

覆盖 CFG-2.1、CFG-3.1、CFG-NFR-2/3，UX7-1/2/3/5/8；AD-8/11。依赖：7.6。

**Acceptance Criteria:**

- **Given** 任一实际工程或无工程起始状态，**When** 使用菜单／操作条／工具轨／对象或文件树／中心文档／检查器／底部窗口，**Then** 落实 DESIGN 的 tokens 与层级并共用一个 reducer、选择及真实后台能力，**And** 1480×920 与1080×720主链可用，窄屏收面板、表格局部滚动，不造未来模块目录。
- **Given** CAN 创建／编辑、原诊断与 DTC、有序 DID、标准输入、两种交接、保存／源码预览、预检／构建／各主机行为与取消，**When** 从任一适用表面执行，**Then** 调同一真实命令与保护／可用性，显示实际文件内容、日志与输入身份，**And** 不用原型数据、成功计时器、不同剖面编辑旁路或第二 store。
- **Given** 同一对象从树／表／引用或源文件选择，**When** 打开或切换文档，**Then** 标签复用、检查器属性／引用一致、只读说明可复制，非破坏工具窗口保留草稿，**And** 生成、构建、运行各用真实结果，不升级目标或标准声明。

### Story 7.8: 保护草稿并完成键盘与问题焦点接力

As a 长时配置工程师,
I want 导航保护未应用输入并用键盘定位问题和完成操作,
So that 修复错误、切工程和退出不会丢失工作。

覆盖 CFG-2.1/2.5、CFG-NFR-2/3，UX7-3/4/7；AD-3/8/11。依赖：7.7。

**Acceptance Criteria:**

- **Given** 对象／批次／诊断／标准参数草稿或 dirty 工程，**When** 切对象、关标签或定位，**Then** 用应用并继续／放弃草稿／留在原处；新建、重开、交接、切工程及退出另用返回并保存／明确放弃／取消替换，必须完成全部应用及保存预览确认才继续，**And** 任一失败不导航、不静默写盘，标签关闭不是关闭工程。
- **Given** 真实含字段、只含对象、只含源文件或不能定位的问题，**When** 主动定位，**Then** 草稿先保护，再树／文档／行／检查器接力，焦点落到可核定字段或标题；无法解析时保留可复制详情，**And** 不造链接、清空旧问题或把修复回显当成验证通过。
- **Given** 键盘、屏幕阅读器、reduced motion 与字段输入焦点，**When** 使用 tree、tab、对话框和 Ctrl/Cmd+K/S/O、Alt+1/4/9、Escape，**Then** label／aria-invalid／describedby、非颜色选中、命名／焦点圈定归还和温和状态播报成立，**And** 快捷键不吞文本，面板调整有键盘／折叠入口。

### Story 7.9: 保存分层设置并迁入暂定身份

As a 工作台使用者,
I want 分别保存外观、规则与模块定义和执行工具，并看到一致的明暗身份,
So that 配置不受工具缺失阻断，也不因设置变化丢草稿。

覆盖 CFG-3.1/3.2、CFG-NFR-2/3/4，UX7-6/7；AD-2/10/11。依赖：7.8。

**Acceptance Criteria:**

- **Given** 原生外观／规则与模块定义／执行工具设置及工具环境覆盖，**When** 查看内置只读版本／覆盖、显式导入／移除第三方定义、浏览工具路径、只提交当前类别、取消或重开，**Then** 外观／工具沿现有配置目录原子保存、类别草稿不串写、执行工具覆盖优先且不写回；扩展操作复用 7.2 接纳／缓存及 7.6 工程接纳集合机制，不另建机器全局选中策略，已有 manifest 的接纳变化为 dirty 并经工程预览保存持久化，**And** 失败保持旧配置与接纳集合，已有设置兼容新外观默认；官方 XSD/MOD 仅为兼容／开发路径、非普通设置或必填项，其环境变量不替换内置权威。
- **Given** 规则升级、已接受扩展身份、工具或目标成功改变，**When** 回到工程，**Then** 实际依赖结果与预检失效、旧确认拒绝，输入／草稿保留且有具体原因，**And** 缺编译器不阻断源码准备，内置库存缺失／损坏准确报告软件错误，可选扩展缺失仅影响消费者，不伪造验证。
- **Given** light/dark/system、系统明暗变化和未应用字段，**When** 保存／取消外观或收到主题事件，**Then** 依据有效主题同步灯位而不重建数据，透明背景与车身一致，**And** 正式启动／导航／favicon／窗口及安装包全部迁入暂定身份、旧图标引用移除；固定 ICO/ICNS 不宣称 OS 自动适配。

### Story 7.10: 独立预览源码并保护用户代码再生成

As a ECU 工程交付者,
I want 预览明确归属的源码并安全再生成和交接,
So that 用户算法和旧有效工程不会被生成器或升级覆盖。

覆盖 CFG-4.1/4.2、CFG-NFR-1/2/4，UX7-5/8；AD-7/9。依赖：7.9。

**Acceptance Criteria:**

- **Given** 已保存且通过所需内置结构／定义／目标剖面检查的 CAN 或标准 ECU 工程、无官方档案及编译器，**When** 预览并确认独立源码或原交接种类，**Then** 展示实际 new/changed/unchanged/removed、拥有权及目标／依赖，安装预览字节，**And** 不启动编译；未解释消费参数／模块、变体、必要扩展缺失或规则库存错误准确拒绝，不将 schema 域结果冒称完整标准通过。
- **Given** 声明 applicationInputs 的 live 用户源码和新输出 workbench-ownership.json v1，**When** 初始化或重复生成，**Then** live 代码仅在明确初始化且不存在时创建，后续从当前字节做 immutable snapshot，绝不写回 live 树，**And** snapshot／ledger 进入原严格 seal 闭包；旧 snapshot 被改拒绝，应用变化使旧确认／build/run 失效，不能变更 owner 旁路保护。
- **Given** 用户改过生成文件、未知／旧版清单、目的地变化、失效身份、升级或删除本工具旧产物，**When** 预览／确认，**Then** 冲突列出并拒绝未核定覆盖／迁移；合法新增／移除经显式预览及 owned staging 事务，失败保留旧有效输出，**And** 不自动清目录。
- **Given** R5 autosar-workbench-handoff-v2、原 v1 两类包及已声明应用槽，**When** 重开／再生成／构建／离线复验，**Then** v2 resourceIdentities 记录 RuleSetIdentity 与必需的已接受扩展定义，导入绑定兼容的同一规则身份、核对真实本地库存及完整 source／rerender／seal 闭包；规则不匹配明确拒绝，或显式在新空目录迁移，预览、staging、保存输入、ledger、重导入和离线工具共同按新协议核对，**And** 不盲信复制元数据为可执行规则、不把旧 xsd/mod 摘要转换为原生规则摘要；旧 v1 精确分派、原资源身份及严格完整性验证原样保留，兼容路径与默认 R5 内置流程分开；live 输入与封存副本分开，不带官方包／个人路径，不靠放宽 closure 或手改生成器交付。

### Story 7.11: 独立复核发行配置链与完整工程交接

As a 非实现者的接收工程师,
I want 在干净隔离环境操作发行工作台并复验生成工程,
So that R5 的完成来自真实工程而不是截图或模拟成功。

覆盖全部 CFG、CFG-NFR、UX7 的组合出口；AD-1–AD-11。依赖：7.10。

**Acceptance Criteria:**

- **Given** 本次 Windows MSI 和 Linux deb／解包 AppImage、原 checkout 不存在、无 Node/Rust/uv/编译器，且无官方 XSD/MOD 档案、路径设置或环境文件、网络不可用，**When** 通过现有隔离 desktop 入口建立／导入支持范围内工程、内置定义编辑、批量修复、保存重开、原生验证及生成源码，并执行预期拒绝，**Then** 完整正常 R5 配置链由打包内置规则与全部受支持模块定义通过真实 IPC／原生文件操作完成，而非借官方资源或开发机依赖，**And** 不切换用户输入桌面，受管进程清理回执真实；macOS 无宿主的检查单独标未验证，原 v1 兼容所需资源另验、不偷换默认流程。
- **Given** 两个原创不同 CAN 工程、七文件标准变体、真实多模块内置定义／结构输入及合法官方模型保留项，**When** 按 PRD 四个退出场景独立运行正例和关键拒绝，**Then** 通用类型／结构／批次／引用编辑、原字节、实际引用／值／作用域、批次全拒绝、用户代码和旧输出保持均有证据，缺失／损坏内置库存报软件错误；可选第三方定义导入／冲突／错版次／摘要及缺失影响单独验证，**And** 不重复同一模拟路径、不将少数内置表单字段当成完整目录、不从原生 schema 域通过推断生成支持或完整标准符合。
- **Given** 封存 CAN／标准工程和锁定独立工具／消费者，**When** 在新位置实际构建并复验 CAN／DID／N_Cr／非法输入恢复及旧目标回归，**Then** 结果对应本次输入与目标，**And** 没有新增 OS／BSW／MCU／认证声明；本机不适用执行明确未运行。
- **Given** 本轮所有故事、开发者固定 R24-11 合法 oracle 对比、当前类型／源码检查与实际 UI 证据，**When** Epic 出口复核，**Then** 关闭全部必要发现、更新故事／sprint及使用说明，并核对 7.1/7.2 开发对比与无外部档案离线安装跨故事证据，**And** 不把 build/test 或各 story 单独通过替代跨故事退出，不删有效旧测试、不压低验收、不用“未运行”或伪通过替代必要证据，不留下伪入口、模拟实现或未接线控件；本次规划修订不提前声明这些检查已通过。
- **Given** 大型受限输入、人为延迟解析／校验／全文扫描、重复筛选展开及超长失败日志，**When** 独立原生复验，**Then** GUI／取消入口仍可操作，记录快照内读源／规则加载／可选定义解压次数确认索引复用，摘要／日志呈现有界且能获取复制完整 owned 日志，**And** 不以快速 mock 或仅引用 AD 代替此性能／资源验收。

### R5 Requirements Coverage Map

| Requirement | Primary stories |
| --- | --- |
| CFG-1.1 | 7.1, 7.11 |
| CFG-1.2, CFG-1.3 | 7.6, 7.11 |
| CFG-2.1 | 7.1, 7.2, 7.4, 7.7, 7.8 |
| CFG-2.2（内置完整支持目录／开发 oracle） | 7.2, 7.3, 7.11 |
| CFG-2.3 | 7.2, 7.3, 7.5, 7.11 |
| CFG-2.4 | 7.4, 7.5, 7.11 |
| CFG-2.5（分域原生校验） | 7.3, 7.5, 7.8, 7.11 |
| CFG-3.1（RuleSetIdentity／内置覆盖／隔离扩展） | 7.1, 7.2, 7.6, 7.9–7.11 |
| CFG-3.2 | 7.9, 7.11 |
| CFG-4.1, CFG-4.2 | 7.6, 7.10, 7.11 |
| CFG-NFR-1, CFG-NFR-2 | 7.1–7.6, 7.8–7.11 |
| CFG-NFR-3 | 7.7–7.9, 7.11 |
| CFG-NFR-4（无官方档案离线安装／严格新旧交接） | 7.1, 7.2, 7.6, 7.9–7.11 |
| CFG-NFR-5 | 7.1, 7.2, 7.4, 7.11 |
| UX7-1, UX7-2 | 7.7 |
| UX7-3 | 7.1–7.5, 7.7, 7.8 |
| UX7-4 | 7.4–7.8 |
| UX7-5 | 7.3–7.7, 7.10 |
| UX7-6（规则与模块定义只读内置／显式扩展设置） | 7.9, 7.11 |
| UX7-7 | 7.8, 7.9 |
| UX7-8 | 7.1, 7.6, 7.7, 7.10, 7.11 |

