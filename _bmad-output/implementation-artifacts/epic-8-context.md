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
