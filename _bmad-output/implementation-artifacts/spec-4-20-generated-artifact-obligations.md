---
title: '4.20 交付实际模块与RTE实现描述'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '912a9508555149f48ed93fb46e561f6368cb9419'
story_key: '4-20-关闭适用-c-工件-模块描述和静态质量证据'
context: []
---

<frozen-after-approval reason="按持续完成Epic4授权推进，额外assurance流程已移除">

## Intent

让生成的SC1主机工程同时交付与实际C源码、入口及生成RTE一致的R24-11实现描述和必要模块使用说明。已有输入BSWMD与声明检查继续有效；生成描述补齐交付代码和RTE生命周期、文件依赖及适用内存段，接收者能够定位真实接口及实现。

## Boundaries & Constraints

单一ValidatedIntegrationPlan、真实FreeRTOS/Win64和C99保持；原输入及已有输出保护。描述只使用真实符号、相对交付路径及实际模块角色；固定原始依赖和许可证保留。RTE/application/主机辅助与BSW责任明确，未实现标准接口不虚构，未分配数据不造MemorySection。应用和公共ABI不变。MISRA C:2012编码指导适用，局部静态检查不推导完整符合。流程、状态、复核和验证结论仅用BMad现有工件；不生成assurance报告、矩阵或审批，不推送。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 已校验计划／生成并搬移工程 | R24-11描述XSD有效；实际模块、行为、代码文件、RTE生成文件和公开入口闭合，C99编译链接及CAN/DID原行为保持 |
| 实际生成RTE和OS／检查链接段 | 描述与真实段、函数和所有者一致；适用MemMap配对，未配置的硬件地址和数据段不伪造 |
| 引用断开、源码依赖缺失、描述API不存在或MemMap不配对／独立检查 | 明确拒绝，成功断言识别实际破损而非重复生成器数据；测试中间文件仅临时目录 |

</frozen-after-approval>

## Code Map

- `core/src/integration/ecu.rs`：完成源码后、封闭清单和link_check前加入产品描述；原始BSW来源和contract.json可复用。
- `core/src/integration/catalog.rs`／`plan.rs`：25个已校验符号契约及原始文件身份；`component.rs`提供真实RTE和组件名称。
- `core/src/integration/artifacts.rs`（新增）：标准BSW/RTE实现描述及模块工程说明。
- `runtime/ecu/templates/Rte.c.in`及生成`Rte_MemMap.h`：真实代码段与生命周期；RTE无独立静态数据，应用数据由Application.c拥有。
- `core/tests/support/epic4_artifacts.rs`／正式入口：复用Scratch、XSD、搬移构建、独立consumer和objdump。
- 本地R24-11 BSWModuleDescriptionTemplate第133、143–155页及RTE SWS第103–105页：代码／生成依赖、实际生命周期与描述约束。

## Tasks & Acceptance

- [x] `artifacts.rs`／`ecu.rs`／`mod.rs`：生成标准模块、行为、实现、代码及RTE依赖描述；加入已校验闭包和模块说明。
- [x] `Rte.c.in`及生成内存映射头：准确映射实际RTE代码，不分配额外数据或改变ABI。
- [x] `epic4_artifacts.rs`／独立fixture／`end_to_end.rs`：验证XSD、引用、实际文件／符号、搬移行为与关键拒绝，复用已有OS段检查。
- [x] 当前BMad工件：记录实际静态检查、适用性与复核，同步sprint并提交。

Given原始标准输入和固定主机工具，when独立消费描述并构建搬移工程，then源码、接口、RTE和内存段可定位且行为正确，拒绝缺失或虚构依赖，验证结果进入本story。

## Implementation Notes

工程内的13模块实现描述指向实际交付文件，六个选定BSW模块的17入口及12个RTE入口使用真实签名。RTE生命周期采用实际Ecu_TargetInitializeRte，不虚构Start／Stop。工程对象以文件名和明确domain映射表示相对路径；C对象大小由独立编译检查，ARTI版本与实际1.0.0 API一致。生成RTE及Counter服务使用重复include的Rte_MemMap.h并进入.rte_code，build.ps1按描述核对全部入口；既有OS Task／Hook和向量检查保留。模板按Git属性保持LF，源码／许可和原始BSW身份仍由既有交付闭包保护。

采用MISRA C:2012修订链编码指导检查接口所有权、原型一致、初始化、参数求值、预处理和资源边界。GCC section属性是固定Win64目标的编译器扩展，封装在MemMap头中；不据此声明完整MISRA符合性或偏离已获批准。静态分析只覆盖本次两个实际RTE翻译单元，其他主机／BSW源码沿用既有行为和C99检查。

## Spec Change Log

## Review Triage Log

| 来源／发现 | 判定／处理 | 依据 |
| --- | --- | --- |
| 盲审：RTE generatedArtifact未包含其他BSW及宿主适配头文件，消费者可能缺编译依赖 | false | R24-11 SWS_Rte_05192（第105页）明确排除其他BSW／应用文件；此清单表达RTE生成文件，CodeDescriptor表达模块自有代码。外部头文件仍在封闭工程内并由build.ps1实际include路径提供，独立搬移构建通过；没有仅抽取RTE自有文件即可编译的产品契约。 |
| 验证缺口：29条目计数和RTE集合不能识别BSW入口被兼容已导出函数替换 | medium／patch | 按六个BSW模块固定独立17入口集合；增加Can_MainFunction_Wakeup被现有void(void) Can_DeInit替代的拒绝，保留函数指针、sizeof和真实链接检查。 |

边界与验收审查未提出已证实代码问题；四层BMad复核完成。一个验证补丁已修复复验，一个发现已否定，无延后项。

## Verification

正式入口`epic4_generated_artifact_obligations`；实际C99构建、独立描述读取与故障拒绝、段/符号检查、原生CAN/DID消费者、增量格式、核心Clippy及按影响复用回归。Cppcheck只检查实际选择范围，不建立完整MISRA矩阵或报告。

最终工件入口44.00s通过：13模块、29真实入口、XSD／引用／sizeof及真实签名、RTE文件闭包、搬移后20tick和CAN/DID、链接段、Cppcheck 2.21.0以及断开引用、缺源码、兼容错误BSW替换、虚构API与两种MemMap错误均检查并拒绝。生成预检及独立编译使用同一实际C99源码；测试中间文件仅临时目录。

相关回归通过：ECU生成280.22s，Counter服务158.24s，ARTI161.95s，安全编辑／保存／重开72.09s。Counter临时目标已补齐Rte_MemMap.h，原成功及变异断言保留。最终核心Clippy correctness／suspicious及增量源码卫生／格式／Python／C99检查通过；15项Python测试通过。普通测试未产生受版本管理报告变更，结论记录在本BMad工件。
