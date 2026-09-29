---
title: '4.10 从标准输入建立唯一校验集成计划'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: '086b2d7e19f68a1d23e9c596eaa1bf3081350e7d'
story_key: '4-10-从标准输入建立唯一校验集成计划'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous per-story execution">

## Intent

将标准多文件输入校验为唯一只读ValidatedIntegrationPlan，供后续组件契约、ECU生成和工作台使用。同一对象用完整AUTOSAR路径与实例关联；名称和ID可变化，不能把4.1参考字面值或独立fixture审计器当产品解析器。

## Boundaries & Constraints

依赖4.1/4.2已完成；遵循INTEGRATION-CONTRACT §1–4和R24-11-CONTRACT。固定epic4-win64-sr-cs-v1单ECU/单Classical CAN/单不可重入应用实例、显式非排队uint32 S/R与同实例同步四字节ReadData；未知无关有效内容保留。标准关系必须来自ARXML，不用SDG补关系。XSD/MOD是合法取得的外部依赖，不复制到交付物。保持旧host-v1入口和行为。

W2只能证明组件契约和已存在BSW签名/依赖，不预先声称新运行工程已链接。未通过计划校验不写工程；真正的集成链接和运行由4.13起完成，W3门保持关闭。

## I/O & Edge-Case Matrix

| 输入 | 结果 |
| --- | --- |
| 4.1完整正例、同剖面名称/ID变体、多文件顺序变化 | 相同确定句柄与计划，角色/逻辑相对路径/原字节SHA、完整路径/实例/类型/通信/服务/周期/依赖齐全 |
| 4.1全部18反例、断引用/错误DEST/类型与方向长度矛盾 | 输入错误或未支持诊断，文件/对象路径/补救明确，整体拒绝 |
| 无关有效对象、不同父路径的同短名 | 原字节保留，不按短名合并，不因此污染选定目标 |
| 缺/重复生产者、C名称归一化碰撞、重复/缺失周期调度 | 定位拒绝，不形成Validated计划 |
| 未选变体、隐式/排队S/R、异步/跨ECU C/S、mode/transformer | 按剖面拒绝，不能默默降级 |
| 缺XSD/MOD或工具/来源错误 | 区分外部依赖与工具错误，不能将未运行描述为校验通过 |

</frozen-after-approval>

## Code Map

- 新增core/src/integration/：标准对象索引、跨文件引用及只读计划；按关注点分离对象图、组件/类型、通信/ECUC、服务/BSW与调度校验。
- core/src/lib.rs和Workspace：公开同一核心计划入口，沿用原始来源身份、安全文件读取及已有XSD能力。
- runtime中的显式当前BSW契约清单：对应实际头文件/实现来源、签名与阶段责任，不以测试expectations充当产品配置。
- core/tests/support及end_to_end：正式epic4_validated_integration_plan入口，直接消费原创输入、独立预期和反例；保持4.1/2封存基线不变。
- sprint及本规格：记录实施、实际验证、独立复核处置和提交。

## Tasks & Acceptance

- [x] 来源身份/XSD/MOD/完整路径对象图和定位诊断，标准引用闭合。
- [x] 组件/类型/S/R/C/S、通信/ECUC/调度与符号生产者形成只读计划。
- [x] 正例、名称/ID合法变体、全部原始反例及碰撞/生产者/重复调度验证。
- [x] 正式exact、适用回归、完整增量门及三路独立只读复核。
- [x] 保存证据、同步sprint、本地提交，继续4.11，不推送。

Given所有支持输入，When建计划，Then只读结果具有可追溯来源与确定分配，后续生成不需要重新猜测关系。

Given不闭合/不支持输入，When建计划，Then返回明确类别、文件、对象路径及补救，输入及旧输出保持。

## Design Notes

复用Rust roxmltree、SHA-256和本地R24-11 XSD校验；新计划独立于旧host-v1专用模型，不从旧Com短名推导SWC API。公共只读计划由唯一校验构造器创建，不开放反序列化或可变字段来绕过验证。原输入字节保留用于4.12往返，序列化摘要不替代原始摘要。

## Implementation Notes

4.8已提交并通过完整门；4.1/2原始输入和独立预期保持封存。当前故事不涉及OS重新选型、外部服务或推送。

已接入core/src/integration独立标准图和InputInspection结构预检：来源使用便携逻辑路径与原字节摘要，拒绝路径别名/大小写重复/Windows保留名/DTD，固定XSD/MOD身份与外部依赖拒绝；原字节保持，完整对象路径和typed DEST校验。结构预检明确不是最终ValidatedIntegrationPlan，不能据此放行生成。

组件计划已从所选Extract的根Composition与实例解析，而不是按参考短名猜测。显式uint32映射、Runnable访问/非排队com-spec/初值和deadline、同步四字节OUT服务、实际Connector及服务使用端、同ECU映射已接入。来源/顺序/断引用/错误DEST/XSD与便携路径边界测试通过；9个封存组件语义反例及应用/端口改名变体通过。原始缺客户端和mode反例同时含引用错误，补充具体语义诊断与通用引用错误并存，保留封存预期、不修改fixture。

调度计划已解析标准Rte/Os定义与显式容器值，检查Task/Counter、唯一event/position、Alarm start/cycle/SetEvent和应用周期一致，形成只读顺序。参考六实体literal序列通过，原period-alarm-conflict和重复任务位置拒绝通过。随后补充Rte实例与BSW实现行为归属检查，仍需在最终组合上复验。当前epic4_选定11入口回归通过（发生在新增Schedule归属检查之前）；增量格式检查通过。通信/ECUC剩余关系、BSW签名/生产者清单、最终唯一Validated计划、正式4.10完整入口/独立复核/全门尚未完成，sprint保持in-progress，W3不放行。

Schedule实例/BSW实现归属补充后的exact正例及period/重复position拒绝已通过。S/R计划沿端口实例、SystemSignal、I-SIGNAL、PDU映射、Frame/trigger、Com标准template引用、canonical ECUC PDU和CanIf配置建立完整路径链；独立检查位序/长度/方向/初值/deadline/周期。方向、长度、transformer原反例和CAN ID1100/1101合法变体通过，参考800/801/4byte/30ms/10ms literal通过。

新增runtime/contracts/bsw-v1.json和可复验materializer，清单来自实际17个当前头文件/源码生产者，产品代码不读取fixture expectations。Rust加载的是随本工具编译的契约身份，并校验实际源文件摘要；ARXML返回/参数顺序/类型/方向、模块归属和唯一C实现与其比较。正例17签名、缺Com_AdvanceTime原反例和ABI类型错配拒绝通过。实际GCC严格C99编译/链接17个函数指针及当前host runtime并运行退出0，证据current-bsw-contract.json的全部来源摘要一致；明确是当前host BSW链接证明，不是新Epic4 ECU链接或标准签名认证。

Dcm/CanTp计划已连接同步四字节OUT端口/DID、default/extended sessions、P2/P2*、64byte canonical SDU/缓冲、物理normal-addressed padded传输及双向NPDU/CanIf/Extract CAN ID。正例0x1234/0x700/0x708、50/5000/200ms及64byte literal通过；随后补齐padding/取消/PENDING/timer-adjust固定剖面拒绝，尚需最终复验。Rx与Tx配置句柄属于不同方向域，不把合法相同数值当成同一对象；跨方向身份由完整路径和方向区分。剩余：通道路由及其他ECUC支持边界、唯一最终计划/句柄/符号阶段闭包、正式全部18反例、生成消费者接入、完整门/独立复核/提交。当前不标4.10 done。

## Spec Change Log

## Review Triage Log

正式exact入口最终恰选1项通过完整正例、Workspace同构计划、顺序稳定、18个封存反例和17签名实际链接探针。额外边界exact恰选1项通过声明符号竞争、C数组类型与原生类型冲突、负数transport ID、未知影响目标参数、合法名称/CAN ID变体、不同路径同短名及原字节保持、标准省略OsEventMask的确定AUTO分配。新增physicalController/HOH/bitrate、PDU/Signal trigger关系、Task/Alarm autostart mode和C/S com-spec守卫已接入并在正式入口复验。Integer ID校验曾误把boolean OsUseGetServiceId当ID，已基于MOD实际定义种类修正，相关正例与额外边界均通过。增量格式/C99通过，完整增量门及三路独立复核尚待完成；不标故事done。

## Verification

独立三路复核均已结束，主代理核实并处置全部三项发现：

- blind / medium：ASCII-only 大小写身份比较漏掉 Windows Unicode 别名。改为保守 Unicode 大写折叠，补充 Ä/ä 与 Σ/ς 拒绝；exact 来源图入口通过。
- edge / medium：报告示例称使用 selected_cluster，但当前代码实际全局选择 cluster/channel。发现的影响成立：无关有效 CAN 网络会污染所选目标。沿所选 ECU 的 COMMUNICATION-CONNECTOR-REF 定位唯一通道，并复用到物理速率检查；添加无关 250 kbit 网络，验证接受、完整路径保留及原字节不变。额外边界 exact 通过。
- gap / medium：Workspace 仅覆盖未修改来源。添加临时七文件副本，在打开后进行合法外部注释修改，调用 integration_plan 必须 SOURCE_CHANGED，并确认磁盘外部字节未被覆盖；纳入正式入口和最终完整门。

复核为只读 default/fork-none 三代理；blind 的独立 Cargo 未配置 vcpkg，因此该代理没有运行成功证据；gap 已按目标环境运行 epic4_ 16 项通过。最终结果以主代理最终代码的完整增量门为准。

正式epic4_validated_integration_plan exact入口，全部4.1反例与同剖面变体；最终python scripts/verify.py --scope all --base 086b2d7e19f68a1d23e9c596eaa1bf3081350e7d。XSD/MOD缺失必须报未验证，不跳过成功。所有验收无头，原生IPC另按4.12隔离环境要求处理。

最终完整增量门退出0：29项Python、3项Rust单元、74项集成测试全部通过，UI lint/build、核心与桌面clippy、桌面构建通过。最终变更20文件包含原始输入只读计划、当前17个BSW签名实际链接证据、Workspace入口及测试；复核后修改已在最终门覆盖。4.10完成，Epic4仍in-progress，W3须等待4.11/4.12。
