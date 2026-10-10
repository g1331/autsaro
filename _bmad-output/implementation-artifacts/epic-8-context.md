---
epic: 8
date: '2026-10-10'
verdict: accepted
criteria: declared
headless: false
title: '多组件生成与调度契约复盘整改'
type: bugfix
created: '2026-10-10'
status: done
route: dispatch
review_loop_iteration: 0
baseline_commit: '6fac020cc916916ba4972caf5c3c145bdba9d66e'
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/CONTRIBUTING.md'
  - '{project-root}/_bmad-output/specs/spec-multi-component-scheduling/application-contract.md'
  - '{project-root}/_bmad-output/planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md'
---

# Epic 8 Context: 多组件应用与调度工程

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

把候选 R6 扩展为可运行、可再生成、可独立交接的多组件应用工程：真实 SWC、连接和任务映射共同生成应用接口、RTE／SchM 与 OS 配置，用户拥有应用算法源码。先证明真实接口与调度闭环，再完成编辑、源码保护和独立交接；历史 Epic 6 编号和 Epic 7 发行收尾保持各自范围。

## Stories

- Story 8.1: 从多组件输入生成一致的应用契约
- Story 8.2: 经真实 OS 调度运行多组件通信
- Story 8.3: 保护每个组件的用户源码再生成
- Story 8.4: 在工作区安全编辑应用组合与调度
- Story 8.5: 独立交接并复验多组件工程

## Requirements & Constraints

- 固定 R24-11、C99、单核 SC1 与既定 FreeRTOS。配置和源码准备不依赖官方档案或编译器；开发验收必须实际构建运行。
- 首批一个 ECU_EXTRACT、一个 ECU、一个平坦组合；多个不同应用类型，每种一个实例，禁止多实例支持。纯本地生产者、消费者、服务提供者和调用者都是必要形状。
- 显式非排队 uint32 S/R、P→R assembly、P 可扇出；每个使用的 R 恰有一个本地生产者或网络映射。按真实接口和应用→实现→基础类型映射验证，等宽不构成兼容。
- S/R 明确初值；首次发布前各 R 保留自身初值。本地仅支持 handleNeverReceived=false、aliveTimeout=0、handleTimeoutType=NONE、无 invalidValue，不受 CAN 停止影响。新 profile 支持最小真实 Rx I-PDU group、标准 reception deadline monitoring 和 COM→RTE 回调，保留正数网络 aliveTimeout；无 group 不启用 monitoring，私有 host timer 不能替代标准通知。网络 Read／Write 按实际 COM 状态处理，CAN 停止不能推导 COM_STOPPED；NONE 超时保留 last value，旧 profile 保持历史语义。
- 同步本地 C/S 支持 uint32 IN／OUT／INOUT 和固定 uint8[4] OUT；无 possibleErrors 时服务器返回 void，客户端 Rte_Call 返回 Std_ReturnType。唯一服务器在调用者 owner 上下文执行；拒绝空输出指针、调用环、异步、重入和跨任务关系。
- 非并发 runnable，只支持 TimingEvent／OperationInvokedEvent。周期 runnable 各一个事件，共同正整数毫秒周期、offset=0；按唯一 Task_Ecu 的合法映射位置执行。服务器仍需无 task／alarm／event／position 引用的 mapping 容器，且不进入周期表。
- 嵌套／delegation、重复类型实例、更多类型、队列、隐式／模式通信及未选变体明确拒绝。R11 仅保留信号类型、网络通道和应用映射身份接缝，不实现其运行扩展。
- 独立验收覆盖真实三组件通信／顺序／重复 epoch、CAN／DID、初值／扇出、非法关系、源码保护、异地交接与旧行为。C／模板修改先应用 MISRA 技能；实际受影响生成 profile 运行 c-check，扫描不能证明完整符合。

## Technical Decisions

- ARXML 原字节是权威；私有不可变计划核定完整身份、关系、符号、调度和源码槽。消费者不重读 XML 或猜关系；失败保留输入、会话和旧输出。
- 新 profile 为 `singlecore-multi-swc-v1`；组件头暴露各自标准 API，映射到唯一内部实现符号。拒绝 C 标识符、保留名、大小写不敏感文件名和 include guard 碰撞；旧 profile／槽／交接仍按原形状分派。
- 本地存储和网络 transport 分离；RTE local 与 COM 周期状态由唯一 owner 访问。真实异步 CAN confirmation／mode callback 共享的 CanIf、ComM、BswM 状态必须由同一递归 CAN 资源的 SchM exclusive area 保护，声明、wrapper 与来源由真实 BSWMD 共同派生，不能以 owner 假设省略保护。既有 OS backend 裁定 tick，顺序为 BSW 输入／COM 周期 DM、全部应用、发送、诊断；同 epoch 不重复执行，无第二调度器。
- 关闭选定配置直接消费的 Com 生命周期、状态、信号接口、标准回调和 Dcm／PduR 服务接缝差距；标准 TriggerTransmit 是 pull-copy，主机主动发送及时间另用明确适配接口。Rx group 成员与启停、deadline 配置和 COM→RTE 接收／超时通知由同一可信计划共同生成，按实际 COM 状态更新 RTE freshness；验证启用、禁用、超时与恢复，不能依赖旧 host Dem 策略或第二计时器。其他未选 group 组合仍明确拒绝，不扩大为通用 group 支持。同步修正所有生产者、调用者和交付资源，摘要更新不能授权 ABI 变化。
- 可信计划派生全部源码槽：`componentPath` 是 full type path，槽名为 `singlecore-multi-swc-v1:<full instance path>`，live `application/<checked type C name>.c` 对应 sealed `src/<same name>.c`。初始化 create-only，与 manifest 原子接纳；每份 live 源逐字节快照，任何外部变化使旧确认失效。
- v2 数组可承载多槽，但输出归属必须由真实 profile／计划重建，未知槽／路径、身份不符和 sealed 篡改拒绝。异地导入在新 live 工程恢复来源；sealed 输出永不转成可写输入。

详细边界与独立预期继续以[规格](../specs/spec-multi-component-scheduling/SPEC.md)、[应用契约](../specs/spec-multi-component-scheduling/application-contract.md)、[验收向量](../specs/spec-multi-component-scheduling/acceptance.md)、[官方契约与差距](../specs/spec-multi-component-scheduling/compliance-references.md)及[最终架构](../planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md)为依据。

## UX & Interaction Patterns

复用工程树、检查器、引用选择、问题焦点、批次／保存预览和源码入口；身份由后台提供。配置、live、sealed、构建结果区分归属；生成变更使执行结果失效。遵循 DESIGN／EXPERIENCE，实际改动验证真实数据、长名称、窄窗口、键盘、主题及中英反馈。未支持生成不成为全局保存门。

## Cross-Story Dependencies

8.1 → 8.2 → 8.3 → 8.4 → 8.5 已依序完成，先通过真实生成／调度关口再展开源码、编辑和交接。8.2 已实现真实 Rx group、标准 deadline monitoring 与 COM→RTE 回调，并在实际 OS owner 验证通信、周期与拒绝；具体标准接缝及历史分析见[8.2 规格](spec-8-2-multi-component-runtime.md)。旧 profile 保持原行为，Epic 7 整体状态不构成门禁，R11 未实施。

8.3 的初始化／manifest／再生成与 8.5 的异地恢复共同闭合全部可信槽、live→sealed、owner／producer 和输入来源。接收者 README 已按真实组件槽显示全部源码、实例、headers 与 runnable；旧 host／single 说明逐字节保持。8.5 实际搬移包并移除原工作区后，正常 Rust 导入与重新生成保持全 payload，包内 Python -I -S 构建并实际运行生产和 canonical owner；成功与六类拒绝由非实现者独立重跑，3／3 通过，三层审查无待处置发现。证据见[8.3](spec-8-3-multi-component-source-protection.md)与[8.5](spec-8-5-independent-multi-component-handoff.md)。

8.4 修复标准工程混入旧 host-can diagnostics 的具体问题，标准字段、实例引用与连接按既有事务安全编辑；已初始化成员身份变更拒绝以保护用户源码。完整开发原生工作台 21／21 及嵌套引用创建、SYMBOL 身份保护回归已通过；实际范围与历史失败见[8.4 规格](spec-8-4-multi-component-workbench-editing.md)。

2026-10-10：五条故事与 Epic 8 均 done。[PR #15](https://github.com/g1331/autsaro/pull/15) 与 [PR #16](https://github.com/g1331/autsaro/pull/16) 各 28 项远端检查通过并已合并，master 提交 e3b14a3c5e39386cc07ae06280d58b4f83f00943 包含完整实现；最终 BMad 收口分支为 `docs/complete-multi-component-epic`。用户撤回 8.5 延期后的整个 Epic 授权范围已完成，原分阶段安排保留在规划历史。

完成结论限首批配置、受控 Linux 实际运行、原生开发工作台与跨平台开发／分析 CI。实际重新生成 multi／受影响 single 的 c-check 分析完整，源码 passed=false、人工规范 assessment 未完成；Windows 交接运行、真实安装、网络 namespace、MCU 与认证未验证，不继承上述通过结论。完整验收结果见[现有验收规格](../specs/spec-multi-component-scheduling/acceptance.md#2026-10-10-首批配置验收结果)。

## Epic 8 复盘（2026-10-10）

本节按用户要求复用现有 Epic 规格，代替单独的 retrospective 文档；不创建额外报告或台账。复盘范围为五条故事及跨故事接缝，保留原验收与失败记录。

### 证据范围

- 完成检查：安装技能的 `sprint_status.py detect-epic --epic 8` 返回 `story_count=5`、`pending_stories=[]`、复盘状态 `optional`。返回的 `done_stories` 包含其他 Epic 的已完成项，不用其长度代替本 Epic 的五条故事数。
- 意图与独立判据：[架构](../planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md)、[AC-1–AC-12](../specs/spec-multi-component-scheduling/acceptance.md)及本目录五份 `spec-8-*` 实施规格。Story 8.1 的基准为 `74128a4`；历史区间 `74128a4..4e4e86a` 包含首条实现及最终收口，另有其他任务合入，不能把区间所有变更计为 Epic 8 自有工作。
- 前次复盘：[Epic 7](epic-7-retro-2026-10-09.md)。sprint 中 Epic 4／7 六项既有行动均已 done；本次不更改这些状态。
- 历史运行：8.4 的完整 21／21 原生开发验收及失败记录；8.5 的实际异地搬移、Rust／Python 接收、独立构建和运行；multi／single 实际 C 分析。临时原始工件是否仍可读取将在本次核对，不将规格摘要等同原始日志。

### 跨故事发现与处置

**F1 — 服务参数名遮蔽运行时调用，合法输入生成不可编译工程。** `core/src/integration/multi.rs:1689–1700` 的 argument 检查覆盖 C 保留名、局部集合和重复项，却未复用 runnable 已使用的 `runtime_namespace`；`multi_rte.rs:208` 将参数短名直接写入函数形参，并在函数体调用 `Ecu_TargetIsOwner()`。将真实 fixture 的 `types.arxml` 中 `/Types/ScalarService/Transform/Input` 短名改为 `Ecu_TargetIsOwner`，其类型、方向和连接保持不变，正常 Workspace 初始化、saved plan、prepare、generate 均接纳；包内 `tools/ecu-tool.py build --mode host-batch` 实际报 `src/Rte.c:134:9: error: called object ‘Ecu_TargetIsOwner’ is not a function or function pointer`。违反架构 AD-6 的符号碰撞拒绝及 AC-1 的合法生成契约。处置：**fix now，proposed**；不能只给示例改名，需要闭合生成函数实际使用的名称空间，并在正常校验／生成入口覆盖拒绝和无副作用。

**F2 — 不同周期共享 OS event 被接纳，实际周期变成触发集合的并集。** `core/src/integration/schedule.rs:465–480、600–621` 验证每条 mapping 的 task、event、counter 和 alarm 对应，但缺少反向的 event→时序组一致性约束；`multi_ecu.rs:551–565` 仅按 event bit 分派全部 entity。仅将 `ecuc.arxml` 从 `Alarm_App` 开始的全部 `/Configuration/Os/Ev_App` VALUE-REF 改为 `Ev_Work`，保留 Task 的三个不同事件声明、1ms／10ms 周期及对应 alarm，其关系通过 saved plan 和共同生成。正常包内生产构建成功；真实 HostBatch 从 epoch0 接收 `RX 800 4 78563412` 后推进至10，输出 id801 在 epoch1–10每毫秒各一帧，而不是原定10ms首次发送。计划仍明确记录应用／Tx为10ms、work为1ms。违反架构 AD-5、AC-3／AC-7 的周期及非法调度拒绝契约。处置：**fix now，proposed**；应在可信计划边界拒绝不可区分的 event／触发时序组合，不在模板中暗加第二调度器。整改须覆盖合法共组、不同周期或相位冲突、源序变换及原 10ms 行为。

**F3 — 原生验收重跑的原因分布有证据，不能只归因于“测试多”。** [8.4 Verification](spec-8-4-multi-component-workbench-editing.md#verification) 第102、108、112、114、116–121行保留：最终完整21项耗时1,422,838ms；前19项完成后才发现工具缺失、日志目录非私有；大投影历史回复常驻导致 Node OOM；重复 saved-source seal 和 snapshot 读取；另有真实 dialog、operation idle、supervisor ready 竞态与合法 split-package 回归。公共API同一延迟配置实测 saved-plan reads20→10、初始化预览reads30→10；不是靠放宽期限通过。处置：已修复实例 **accept as-is**，下一轮按阶段完成工具预检、受管失败／取消与大数据聚焦验证，再执行完整套件；保留晚期来源校验、全部payload和拒绝断言。现有AGENTS已记录工具预检、流式证据和异步同步规则，不重复登记为未完成整改。最终 native memory 原件明确统计 application／WebKit／server subtree、排除外层Node；不把它当作驱动内存或完整长期内存评估。

**F4 — 独立交接的完整成功与拒绝闭包值得保留为下一轮入口。** [8.5](spec-8-5-independent-multi-component-handoff.md#implementation-notes)、`core/tests/multi_component_contracts.rs:109、262、4466` 实际搬移包并删除原作者目录，严格Rust恢复／重新生成全payload相同，接收者Python `-I -S` 构建生产与工程外canonical probe；源码／owner／producer／路径／身份六类拒绝保护旧接收文件和外部sentinel。不能把同目录重复生成、哈希重封或只编译代替该行为。处置：**accept as-is**；R11所选类型与通道的新增向量要通过同一消费者链，避免末尾才补交接。

**F5 — 官方契约的依赖闭包改变了真实实现范围，C分析边界保持准确。** [官方核定记录](../specs/spec-multi-component-scheduling/compliance-references.md) 保留固定PDF身份、SWS_Com_00772／00840和SWS_Rte_08061／08062／08103／08104等依据；本次首批Rx group／DM／真实COM→RTE回调不是旧host计时器改名，typed消费者、真实owner六相位和旧profile回归有独立预期。直接依赖核定还涉及Com生命周期／status／TriggerTransmit、TP handle翻译、mode／DET／MemMap／BSWMD，不能只对新增函数做签名检查。处置：**accept as-is**；R11先按选定信号／通道／transport查适用义务和身份域，不自动引入全部CAN／诊断／NM变体。C分析原件multi52／single42 TU完整、error=null但passed=false；原2607／1814诊断及人工assessment未完成均保留，这不是新的假绿发现，也不能被本次功能回顾关闭。

**聚合结构核验。** 安装的 `git_evidence.py` 对历史区间测得42 commits／8 merges，first-parent测量6 merges，不能将未测合并当零变化，也不能将merge_files与非merge files相加。提交主题不用Story编号，脚本自动归属不足，按规格baseline与实际内容核定：8.1 `b4b3a3b`；8.2中间整改至`c9e5e9b`；8.3 `3da3607`；8.4 `31f2f96`与跨平台测试修正`c3c8c32`；8.5 `a461673`；收口`67ff5b3／4e4e86a`。其他任务的保存整改和技能移除不归本Epic。高增量模块实际检查了当前职责：`multi.rs`1786行核定模型，`multi_ecu.rs`719行共同生成，`artifacts.rs`1268行元数据；不是只凭体积判定缺陷。源码按model／COM／BSW／mode／RTE／ECU拆分，旧ABI由source_assets profile分派，有意保留旧运行时不是应立即删除的重复实现。未运行全仓依赖图或克隆检测，未据此声称不存在其他循环或重复。

按 `bmad-review` 独立执行 adversarial、edge-case-hunter、verification-gap 三镜头，前两者各返回F1／F2候选，主代理均以公开入口实际复现后接纳；verification-gap在其已读独立生产向量、deadline、源码和交接范围返回空列表。该空列表不覆盖随后增加的两个反例，也不表示全仓无缺陷。未进行可选party讨论，未以数量要求补造发现。

### 本次行为复核

本次从正常 `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts handoff -- --nocapture` 入口执行，显式提供绝对路径gcc／objdump／git／Python：3 passed、0 failed／ignored、67 filtered，24.65s。它实际运行异地接收、重新生成、包内生产CAN／DID、canonical owner及六类拒绝，不只是单元mock；过滤项是其他故事，不声称全量重跑。日志为临时 `/tmp/autosar-epic8-retro-handoff.log`。

两个新反例借用现有fixture／Scratch／公共Workspace／包内CLI，在临时Cargo集成测试中执行：2 passed、0 failed／ignored、70 filtered，10.88s，日志 `/tmp/autosar-epic8-retro-reproduction-final.log`；**passed表示缺陷复现断言成立，不表示产品修复**。首轮schedule断言错误地假定OUT字段相邻，实际输出已证实每毫秒发送；改为按独立字段核验并断言10帧后重跑。首轮失败日志保留 `/tmp/autosar-epic8-retro-reproduction.log`，不隐去该测试错误。临时测试源码取证后移出仓库；两个修复及正式稳定回归均尚未实施。

历史原件本次可读：8.4 `native-builtin-results.json` 的21个checks均passed、installed=false、configurationStatus=passed、error=null；IPC JSONL实际1,095,211,129字节，本次只核对文件存在／大小及结果摘要，未重新逐行解析或重跑完整native。8.5两份c-check summary实际读取：multi52 TU／两个程序各51，single42／各41，均exit1、passed=false、error=null；诊断数量／adopted692与691符合原规格。本次没有重新扫描、完成221项人工评估或查看全部诊断路径。

远端只读核对PR #15／#16／#17均MERGED，分别merge `660c021／e3b14a3／1a89f84`，fetch确认共享基准；CI28项通过沿用原交付证据，本次未重新下载全Actions日志。过程分析使用实施规格中的逐次失败、metrics和本次可读原件，不依赖记忆中的旧进度；未完整重放全部会话日志，因此不推断未留证的时间比例。

### 前次行动跟进

Epic7复盘的四项与Epic4两项在sprint均已done，无open／in-progress前次行动要转移。Epic7整改复评有`6fba29f／9fd6f41／97bb803`及双平台保存回归来源；本次历史中确实包含该共享基线，8.3复用其capture／verify／no-clobber原语。不重跑旧整改、不改变状态、不调用`--set-action-status`。前次完成不是当前F1／F2的通过证据。

### 下一轮改进与验收判定

**verdict: rejected；criteria: declared；本次机器判定，无人工覆盖。** 五条故事pending为空，PR均已合并，但F1／F2在现有支持配置下违反既定生成／调度契约，不能降为accepted-with-open-items。拒绝的是当前质量复评，不改写已交付、原通过／失败及未验证事实，不自动回退Epic／Story的done。复盘键done只表示回顾完成；机器验收判定读取本文件frontmatter。

| 行动ID | Owner | proposed整改与独立退出条件 |
| --- | --- | --- |
| epic-8-retro-item-1-runtime-argument-shadowing | 集成模型／RTE生成维护者 | F1：闭合参数与生成函数实际依赖的名字冲突；正常校验和prepare拒绝上述合法改名向量且输入／旧输出保持；其他合法参数仍能编译运行，原typed／C/S向量保持。 |
| epic-8-retro-item-2-shared-event-timing | 集成调度／OS生成维护者 | F2：可信计划核定event对应触发周期与相位；拒绝10ms应用／Tx与1ms work共事件的完整有效引用向量，覆盖合法共享与源序反转；生产首帧epoch10、重复epoch无重放、deadline和旧profile回归保持。 |

以上两项写入现有sprint为open，等待后续开发委托执行；复盘不自动实施、不修改生成C／模板、不启动R11。整改如涉及C或模板，继续先读MISRA技能并对实际受影响profile运行c-check，保留源码诊断和人工项。修复并独立验证前，本复盘不支持R11消费者实施就绪；其范围调查仍可推进，只有实际消费受影响生成／调度链的工作受阻，不将Epic7整体状态或完整MISRA认证作为泛化门禁。

下一轮具体采用：①完整native前先通过原失败／取消和工具预检的聚焦场景，按metrics消除同一不可变调用的重复读取，最后再跑完整套件；②符号与调度合法性同时检查局部对象和跨组件／BSW共享资源；③先冻结选定配置的官方适用义务、真实回调与错误语义，再生成独立成功／拒绝向量；④所选新信号／通道从真实作者源码一路走到删除作者目录后的接收者构建运行，C分析分别写工具、TU完整性、源码结论与人工评估。已有规则不另开“流程建设”故事。

已将两类复现共同揭示的具体检查规则补入AGENTS的工程推进约定：参数名要对照生成函数体实际依赖，同event的周期／相位要作集合核定。这是已确认经验的记录，未冒充产品缺陷已修复。安装技能的sprint更新返回ok=true、optional→done、action_items_added=2、action_items_updated=0、verdict=rejected；随后sprint验证valid=true／problems=[]。本次修改的Markdown共29个本地文件链接可解析，diff空白检查通过，临时Cargo探查文件已移出仓库；仅五份既有文档／状态文件有改动。

### 未决与范围边界

没有需要猜测的产品决策来确认F1／F2；待用户选择是否委托整改，未收到人工覆盖rejected的决定。Windows实际独立交接、安装／namespace、MCU／硬实时、认证、全部C诊断整改及221项人工规范评估保持未完成；这次复盘没有替它们补证。本地修改只包含既有BMad结果／sprint与已确认经验规则，没有提交、push或新PR。

## 授权整改（2026-10-10）

用户已委托完整处理F1／F2至PR合并。上述复盘记录保留为整改前事实；本节是本次Build规格，复用既有文件，不另建报告。统一目标是让首批多组件工程在生成前拒绝不能正确编译／调度的配置。已确认原五份修改均为本次复盘，随功能修复分支保留；基于master安全快进至`1a89f849ee089924469475517b972497a22950cc`，复盘阶段提交`6fac020cc916916ba4972caf5c3c145bdba9d66e`。不存在未定产品选择；PR／合并已获本次授权，代码与整合由主代理完成，子代理只读调查／审查。

<frozen-after-approval reason="human-owned intent — complete remediation explicitly authorized">

### Intent

关闭参数遮蔽与共享event错误周期，在正常配置校验、可信计划与生成前一致拒绝，保留合法C/S参数、同节奏不同trigger及旧profile行为。

### Boundaries & Constraints

固定R24-11／C99，沿同一模型与生成链修复；不重命名用户参数、不改公开ABI、不在模板新增计时器。命名检查核对实际producer、runtime函数与selected宏；只登记真实发出的符号，组件API alias保持本地作用域。已有alarm首次到期=period，table start+offset=period；event组比较有效节奏，不要求相同trigger身份或相同原始offset。不放宽已有phase／owner／mapping拒绝、源码guard或交接字节核对，不实施R11／Epic7发行工作。

### I/O & Edge-Case Matrix

| 场景 | 输入 | 预期 |
| --- | --- | --- |
| 遮蔽拒绝 | Ecu_TargetIsOwner、跨组件server／实际client实现、NULL_PTR／真实MemMap宏参数 | 正常校验与计划拒绝并定位来源；无悬空引用掩盖，输入和旧输出保持 |
| 合法参数 | 普通value／status／data等名字 | 全槽初始化、sealed构建、真实OUT／INOUT行为保持 |
| 错误节奏 | 10ms应用／Tx与1ms work完整有效引用共event；源序反转 | 生成前拒绝，不能先被破坏task refs的反例冒充覆盖 |
| 同节奏 | 同trigger或不同alarm／table，同period且相同有效首次到期 | 接纳；offset0/start10与offset2/start8不能误拒 |
| 非法phase／旧行为 | table首次到期9ms但period10；旧single、真实生产／deadline、源码与异地交接 | 原phase拒绝保留；成功、拒绝及字节保护回归保持 |

</frozen-after-approval>

### Code Map

- `core/src/integration/multi.rs::check_names`：两遍登记实际全局producer，再检查组件alias与参数；保留producer的runtime_namespace规则，参数按实际符号／宏拒绝，补实际NULL_PTR／MemMap活动宏及头文件包含链。
- `core/src/integration/schedule.rs::inspect_events`：实体收集后核定(task,event)的周期一致性；沿现有首周期／phase不变量拒绝，消息来源在`core/src/messages.json`。
- `core/tests/multi_component_contracts.rs`：复用inputs、rejects_in_both、Workspace、scaffold及production／handoff消费者。旧table回归复用`core/tests/support/epic4_timing.rs`，不改single更窄producer约束。
- R24-11 OS00403／00404／00278与RTE09024–26／09069–71说明真实event／trigger及首次到期；同event节奏约束属于本profile的bit分派能力边界，不虚构标准要求相同trigger。

### Tasks & Acceptance

- [x] `multi.rs`及既有multi测试：完整名字域拒绝、合法参数实际编译和OUT／INOUT行为。
- [x] `schedule.rs`、`messages.json`及既有multi／single测试：错误共event拒绝、等价alarm／table接纳、phase／源序和源码保护。
- [x] 真实生产／deadline／异地handoff回归、正常完整core／quality；生成受影响profile并执行实际c-check，保留源码诊断与人工项。
- [ ] 独立三层审查、两项行动复评done、提交／PR／必要CI／合并与master核对，历史rejected和失败保留。

Given合法名字与同节奏配置，when正常Workspace生成、包内构建运行，then既定CAN／DID／C/S及周期成立。Given上述非法有效引用配置，when普通校验／计划／生成准备，then准确拒绝、源与旧输出不变。Given独立接收者，when搬移恢复再生成和运行，then原源码归属、严格payload与成功／拒绝闭包保持。

### Implementation Notes

调查已确认修复可限Rust可信计划校验，生成C模板与标准签名保持。名称按两遍登记全部组件producer后核定，组件API alias保持局部；活动宏在producer登记前收齐，避免组件排列影响结果。先添加真实反例并确认基准失败，再修改模型；相位冲突按原table检查独立拒绝。前述完整推进授权覆盖本节范围，不重复请求阶段批准。

### Review Triage Log

三层审查同步派发、收齐后处置；verification-gap返回无遗漏。每项候选单独核查，未按数量补造产品缺陷。

| 来源／候选 | 判定／处置 | 依据 |
| --- | --- | --- |
| Blind 1：COM／ECU头文件宏遗漏 | medium／patch | Rte.c实际先包含Com.h、Ecu_Target.h；空宏／数值宏会改写形参。补包含链实际名称，新增有效引用拒绝向量。 |
| Blind 2：MemMap活动标记遗漏 | medium／patch | memory_map在包装函数代码段定义RTE_MEMMAP_ACTIVE及scope_CODE_ACTIVE；新增反例修复前被接纳，修复后两入口拒绝。 |
| Blind 3：局部参数整族前缀误拒绝 | medium／patch | runtime_namespace是producer名字域规则，局部Ecu_sample无实际producer；改为实际名称集合，并保留Ecu_sample／Os_sample／xTask_sample正常校验与真实编译。 |
| Blind 4：旧输出断言未接操作 | low／patch | 原sentinel目录未传入生成入口，不能证明输出保护；删除无效断言，增加真实sealed包→源变化→旧准备generate(同目录)拒绝及完整payload比较。 |
| Blind 5：周期反例可能命中旧分支 | false／拒绝 | 修复前同一有效引用反例已在新测试被接纳，修复后仅新增组检查拒绝；仍强化到shared_event_period_mismatch的消息身份，避免未来测试含混。 |
| Blind 6：未覆盖Alarm与Table混用 | false／拒绝 | Table样本只转换Process／Observe，Ingress仍使用Alarm_App；同Ev_App已经包含两种触发，且进入真实production向量。 |
| Edge 1：活动宏遮蔽仍可接纳 | medium／patch | 与Blind 2同根因，补全活动宏，完整名字回归覆盖。 |
| Edge 2：完整名字域声明与宏遗漏冲突 | medium／patch | 对实际生效宏的核查成立，与前项同根因；在实现与测试闭合，不靠缩小已确认范围规避。 |

无intent_gap／bad_spec／defer项，没有另建deferred-work或报告；修补后刷新diff并重新执行受影响验证。

### Verification

正常Cargo native-tests覆盖multi及single/table；实际导出六种c_analysis样本并对multi／standard-ecu运行固定Cppcheck c-check；完整默认Cargo、现有quality／assets／diff检查。PR按正常checks工作流，源码passed=false与分析完整性继续分别记录，不将CI通过解释为MISRA认证。


### 2026-10-10 整改复评与验证结果

当前机器判定 **accepted／criteria: declared**，适用于本Epic既定开发、受控Linux生成／通信／调度／交接范围；历史rejected与反例保持。两项行动由既有retrospective脚本精确按id更新done，未添加新行动；sprint validate返回valid=true、problems=[]。修复关闭R11实际消费者链上的这两个具体缺陷，不代表R11实现或其全部进入条件已完成。

| 验证 | 当前证据与边界 |
| --- | --- |
| 修复前反例 | 名称与共享event新测试分别失败；活动宏增补在修复前也失败，错误均为旧计划接纳。原生成编译失败和每毫秒生产证据保留于上文。 |
| 多组件实际工程 | native-tests首轮73／73、宏修补后74／74通过；原生产4向量及新增等价alarm／混合alarm+table 2向量实际编译运行，CAN首帧epoch10、重复epoch无重放、DID、deadline、源码保护及异地handoff保持。 |
| 合法参数与旧输出 | 普通value/data/status和无冲突Ecu_sample/Os_sample/xTask_sample脚手架实际编译运行，OUT清零／INOUT保持；新增旧准备结果generate(同sealed目录)拒绝1／1（含两种非法配置），完整payload及源字节不变。 |
| 旧profile／完整默认回归 | 默认Cargo177／177、0 ignored；multi runtime9／9；旧OS sc1-timing1／1通过。Linux目标不执行Windows-only旧generated_tables oracle，等价multi table真实生产已单独覆盖。 |
| R24-11官方契约 | 本轮复核本地官方RTE、OS PDF，SHA分别d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769、6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44，与既有compliance-references一致；按OS00403/00404/00278及RTE09024–26/09069–71核定首次到期。组节奏约束来自当前event-bit分派能力，不写成标准禁止不同trigger。 |
| 实际生成C分析 | 六样本正常生成及异地恢复；multi 52 unique TU（两program各51）2607诊断／692 adopted；standard-ecu 42 unique TU（各41）1814／691。固定Cppcheck两program均exit1、error=null、passed=false，221项assessment均not_assessed。最终模型再次生成六样本，multi 172份及single121份C/H与已分析工件逐字节一致；本次没有C或模板改动。 |
| 工具与审查 | 三层审查全部完成，macro／局部名字与无效输出断言已处理；无遗留整改／defer。clippy correctness+suspicious、quality、assets check（0 changes）、diff检查通过。首次quality缺rustfmt PATH、旧OS首次使用系统Python缺开发包；修正每命令环境后原检查通过，没有修改宿主配置或产品规则。 |

原生输出保护测试开发时曾错误假定非法调度必须拒绝整个Workspace reopen；实际工作台允许打开可修复配置，只有可信计划／生成准备拒绝。已保持该正常编辑行为，并检查允许打开时saved plan／prepare拒绝，以及任何旧准备结果不能覆盖交付包。失败探查日志未用通过结果替换。

交付分支为`fix/multi-component-contract-validation`；本地整改和复评已就绪，接着创建PR、等待必要CI并合并。CI、远端合并及最终master核对以PR实际结果补证；Windows实际独立交接、安装包／MCU／硬实时、完整AUTOSAR认证、C源码诊断关闭和221项人工评估保持未验证／未完成。
