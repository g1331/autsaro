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

8.1 → 8.2 → 8.3 → 8.4 → 8.5；真实生成／调度关口通过后才展开依赖流程。8.2 已完成所选受控 Linux 的真实生成、通信、调度及独立审查，实施证据为提交 `c9e5e9bc6e8b286faa9d4c132de16bfbf812219d` 与[8.2 规格](spec-8-2-multi-component-runtime.md)。真实 Rx group、标准 DM、COM→RTE 回调已实现并验证；按可信计划接纳全部实际用户源与不可变快照的最小准备切片也是 8.2 运行所需依赖。它不完成 8.3 的 create-only 初始化／manifest／再生成、8.4 的完整 UI 或 8.5 的异地恢复／交接。实际生成 multi／受影响 single 的 c-check 分析完整性已核验，但原始诊断仍使 passed=false，人工规范评估仍未完成；不声明源码全面符合。Windows 运行、MCU 与未执行原生 GUI 范围不得继承 Linux 通过结论。

8.3／8.5 须共同覆盖源码初始化、manifest 接纳、打开时真实组件契约、准备快照、live→sealed 来源、输出归属、重导入及包内离线接收边界。现有输入数组不授权新槽；全部槽、文件、owner 与 producer 由同一可信计划派生，旧单槽分派精确保留。源码准备仍不得依赖 compiler／官方档案。

8.5 后续还需核对接收者说明：`core/src/generator/delivery.rs::append_native_readme` 当前沿用旧单组件契约与 `src/Application.c` 的源码归属说明，不能作为多组件源码路径指南。该故事须按真实组件槽修正说明，并以新目录恢复、重新生成及包内离线构建运行验证全部来源和归属；本次 8.1–8.4 阶段合并不宣称此交接闭包已完成。

8.4 已修复标准工程混入旧 host-can diagnostics 的问题：共享 source projection 决定实际 profile，保留 schema／definition 错误并单独定位目标生成约束；不通过前端过滤或清空 diagnostics 处理。标准字段、实例引用及连接批次沿现有检查器安全编辑，已初始化成员身份变更拒绝以保护用户源码。2026-10-10 完整 core 234 项及后续受影响 builtin72／workbench7 通过，完整开发原生 21 项已实际通过，三层独立审查结束，嵌套引用创建与 SYMBOL 源码身份回归已修复，受影响 workbench8、quality22 和 Clippy 通过；通过范围及历史失败以[8.4 规格](spec-8-4-multi-component-workbench-editing.md)为准，8.4 实施规格 done／sprint review，远端交付按真实 PR 状态核验。

复用已交付源、规则、事务和 OS 基础；Epic 7 整体状态不构成门禁，仅相关具体缺陷阻塞受影响工作。共享接口修改统一协调并保留其他任务修改。结果写回 BMad；当前分支 `feat/multi-component-handoff`。2026-10-10 PR #15 的28项CI通过并已合并，master提交660c0217e126ebd5208e052f0c2c59d9f23d9675包含8.1–8.4；8.5正在真实异地交接、审查与分析。2026-10-10 用户撤回 8.5 延期安排，本任务继续整个 Epic 8：完成 8.1–8.4 后提交、推送、创建 PR、通过必需 CI 并合并，随后继续 8.5 的独立多组件异地交接闭包和相应审查、验证、PR 合并。全部必需验收完成前 Epic 8 保持进行中，未执行的交接不记作通过。
