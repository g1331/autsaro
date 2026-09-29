---
title: '4.11 由 SWC 描述生成组件接口与同步服务类型'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: '38d163dc6b64eca25a60b568c62158bbc218ee9b'
story_key: '4-11-由-swc-描述生成组件接口与同步服务类型'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

从4.10唯一ValidatedIntegrationPlan生成可供独立应用编译的组件契约。SWC端口/数据元素决定RTE符号，Runnable/OperationInvokedEvent决定应用声明；同步四字节ReadData双方使用相同类型和调用绑定。

## Boundaries & Constraints

遵循INTEGRATION-CONTRACT §1–2。固定C99、显式非排队uint32 S/R、同步Std_ReturnType四字节OUT，保持旧host-v1生成器和运行时。Read参数仅uint32指针、Write仅uint32值，不出现valid/Invalidate/OpStatus/NRC或异步PENDING。产品只消费私有已验证计划，不能重新解析或读测试expectations补关系。

头文件集合包含Std_Types.h、Rte_Type.h、Rte.h及应用/服务使用端组件头；每个外部函数只有一个所属声明头，带参数名/void、防重包含和契约说明。桩是测试私有消费者，不交付。W2结果仅契约，不声称W3 ECU已链接。复用现有生成器的来源摘要、只读预览、旧输出完整性检查、暂存安装/备份机制；遇拒绝不修改旧目录。不会加入费用、远端操作或扩大其他Epic。

## I/O & Edge-Case Matrix

| 输入／状态 | 预期 |
| --- | --- |
| 正例及合法组件/端口/类型改名，重复生成 | 标准Read/Write/Call及Runnable随计划变化，头文件集合/原字节确定，独立应用严格C99编译 |
| 同步ReadData四字节OUT | 类型容量4、双方签名相容；测试桩正常E_OK与基础设施失败可被调用者区分 |
| 类型/方向/异步/跨ECU/未声明操作/碰撞错误 | build_plan定位拒绝，不产生可用于生成的计划，旧输出不变 |
| 错误参数数目/valid/Invalidate/OpStatus/NRC/错误函数指针 | 独立消费者编译失败，断言失败诊断来自接口而非工具缺失 |
| 旧生成文件被编辑、用户文件、失效预览 | 现有安全门拒绝；保留原字节和用户文件 |

</frozen-after-approval>

## Code Map

- core/src/integration/contracts.rs：新增只读ComponentContractFiles及ValidatedIntegrationPlan契约生成/预览/确认写出入口，原生C模板唯一来源。
- core/src/integration/mod.rs：导出契约集合；plan.rs已有25个符号/阶段所有者供生成选择与校验。
- core/src/generator.rs：仅将seal_files、preview_prepared、generate_prepared改为crate可见，复用成熟安全安装逻辑，不改旧host行为。
- core/tests/support/epic4_contract.rs及end_to_end.rs：正式入口、实际GCC编译运行、错误接口拒绝及输出安全测试。复用4.10完整输入和实际runtime清单。
- sprint及本规格：记录验证/三路独立复核/提交。

## Tasks & Acceptance

- [x] contracts.rs：从同一计划生成头文件及只读集合，保存计划摘要和来源证明。
- [x] generator.rs/mod.rs：接入成熟安全预览/写出，不开放未验证构造。
- [x] 测试正例、改名、确定性、标准状态类型、错误调用编译拒绝、18反例与旧输出保持。
- [x] exact正式入口、完整增量门及三路独立只读复核全部适用出口通过。
- [x] 同步sprint、本地提交，继续4.12；W3保持关闭。

Given合法SWC计划，When生成并用独立应用消费者编译，Then端口和Runnable声明与类型一致，不依赖未来ECU运行故事。

Given非法输入或输出冲突，When请求计划/生成，Then定位拒绝且现有输出字节保持。

## Implementation Notes

4.10最终门通过：29 Python、3 Rust单元、74集成、UI lint/build、双clippy和桌面构建；提交38d163dc6b64eca25a60b568c62158bbc218ee9b。已读取MISRA技能及相关全部指导；本故事生成声明/常量而不实现RTE行为，使用带名原型、有界四元素类型及单一声明所有者。标准状态数值已直接核对本机R24-11 SWS_RTE p688：COM_STOPPED128、NEVER_RECEIVED133、MAX_AGE_EXCEEDED64；不复制官方原件。无授权MISRA全文，不能作完整符合声明，4.20保持独立出口。

## Spec Change Log

## Review Triage Log

## Verification

正式epic4_component_contract_generation exact恰选1项；严格GCC C99/Wall/Wextra/Werror/pedantic独立编译和运行。最终python scripts/verify.py --scope all --base 38d163dc6b64eca25a60b568c62158bbc218ee9b。全程无头，无可见桌面进程。

正式exact入口最终恰选1项通过，75项测试中的其余74项未在该命令运行。正例和合法改名消费者均用严格GCC C99编译、链接并运行退出0；6个错误接口编译确实拒绝；18封存反例不改变旧输出；重复生成/输入顺序确定，用户改写与失效预览拒绝。增加头文件大小写/guard/宏/类型/函数/C99关键字/保留命名碰撞边界5组，定位CONTRACT_NAME_COLLISION且旧输出保持。增量格式/C99门通过。当前三路复核及最终完整门待完成，W3不放行。

三路独立只读复核结束：blind一项medium成立，plan.rs的S/R声明所有者仍写story-4.11:Rte.h，生成实际原型位于应用组件头且provenance未覆盖。将计划中的两项声明所有者修正为component contract header，生成与元数据使用同一映射；新增检查6个4.11符号都有provenance项、所属文件含原型且全部头文件仅一处声明。edge无发现，gap无验证缺口。未修改冻结意图，不降低标准接口要求。完整增量门正在运行，故事保持in-review。

最终完整增量门退出0：29 Python、3 Rust单元、75核心集成测试全部通过，UI lint/build、核心及桌面clippy、桌面构建通过。最终6符号声明所有权一致性断言已在75项门中运行通过。已核对暂存9文件范围和复核后diff，旧host安全安装行为未改；没有交付桩。4.11完成，4.12未完成、W3仍关闭。
