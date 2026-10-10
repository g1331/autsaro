---
epic: 8
date: '2026-10-10'
verdict: rejected
criteria: declared
headless: false
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
