---
title: '4.18 连接显式OS配置与RES_SCHEDULER生成行为'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '3b853473134c322bdd9fcb867dc7a63957f82690'
context: []
---

<frozen-after-approval reason="用户授权完整Epic4与已验证增量连续实施">

## Intent

将参考生成工程的OS配置与实际后端保持一致，补齐OsUseResScheduler的显式输入、检查与真正资源配置；杜绝缺失或错放的受支持OS策略被忽略后生成另一种行为。

## Boundaries & Constraints

固定单核SC1、同一校验计划、既有RES_SCHEDULER优先级天花板和资源语义；不重开OS选型。参考输入显式false保持现有无资源配置，true／1生成一个本核虚拟RES_SCHEDULER，允许全部本核Task使用，天花板为最高Task优先级、无ISR访问；false／0不凭空生成。R24-11 ECUC_Os_00049的必选参数不得静默丢失。受支持OsOS状态／类／错误参数开关和OsHooks配置须唯一、位置正确并满足本剖面已约定值。其他配置支持边界保持明确拒绝，不将未知资源定义当作实现。产品说明与配置进入已有生成工程；验证和复核只按BMad记录，不增加报告体系、远端操作或其他Epic功能。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 明确false／0，生成并构建运行 | 无虚拟资源，实际目标配置与原参考行为一致 |
| 明确true／1，生成并搬移构建运行 | 真正资源配置id=RES_SCHEDULER、最高本核Task天花板、全部Task访问且无ISR；实际Task可获取／释放，错误快照和未占用行为正确 |
| 缺失／重复／非法布尔／错放OS字段，重复或错放OsOS／OsHooks | 同一build_plan拒绝，不能获得可生成计划；保留原输入、旧工程 |
| 配置资源、句柄／访问或天花板被断开 | 独立配置／真实消费者或原生配置检查拒绝，不以源码关键词代替行为 |

</frozen-after-approval>

## Code Map

- core/src/integration/configuration.rs：OsOS／OsHooks及已支持字段的必选值／位置／唯一性检查。
- core/src/integration/ecu.rs、runtime/ecu/templates/Ecu_Config.c.in：同一配置记录生成实际scheduler资源及计数／指针。
- core/tests/fixtures/epic4/positive/ecuc.arxml：参考显式false，不改变既定参考调度。
- core/tests/support/epic4_os_configuration.rs、end_to_end.rs：四布尔表示、结构／拒绝及实际搬移消费者；复用临时目录与后台看门狗。
- runtime/os/src/Os.c、Os_Backend.c及既有资源／容量测试：复用实际优先级天花板行为和无配置拒绝，不引入第二套状态。

## Tasks & Acceptance

- [x] 验证受支持OS配置的唯一性、位置、必选参数及布尔域。
- [x] 从同一计划生成真正RES_SCHEDULER配置，并保留false参考行为。
- [x] 正式注册真实生成／搬移／Task消费及关键拒绝测试，完成相关回归。
- [x] 完成BMad三路复核与本地提交，继续父story完整复核。

## Implementation Notes

已核对本地R24-11 OS p105／278～282／393的00049与00850～00854；当前参考没有OsResource，原生后端已检查RES_SCHEDULER天花板、全部Task访问和无ISR规则，且有真实获取／释放、未配置拒绝和容量运行测试。主代理负责实现。

## Spec Change Log

## Review Triage Log

| 来源／发现 | 判定／处置 | 依据与修正 |
| --- | --- | --- |
| blind | 无发现 | 已核对配置归属、生成映射与实际资源约束。 |
| verification-gap：0／1只有生成预检，没有消费者断言 | medium／patch | 原测试确实只运行文本布尔；若1错误映为关闭，编译仍可通过。四表示现在均搬移、构建并运行同一真实Task消费者，以独立预期传入enabled状态，数值1的同名资源忽略亦被实际消费。 |
| edge初次partial：同后缀不同定义路径被计划接受；最终代理撤回 | medium／patch | 主代理用合法本地VendorOs模块／布尔定义实测：原计划接受，但生成报ECU_SOURCE_CLOSURE“validated OS scheduler resource configuration is missing”。不是无定义悬空引用。复用已有必选OS策略列表检查定义身份与位置，拒绝供应商同名字段替换标准字段。三个正式案例覆盖scheduler／service-id／ErrorHook，保留合法未使用供应商定义的输入；替换标准字段则在build_plan阶段得到OS_CONFIGURATION。 |

补丁最终正式生成配置测试186.80s通过，包含四种表示、两个文本和两个数值搬移消费者、原生错误配置，以及三个供应商定义保留／拒绝对照。Clippy与增量格式／C99再次通过，15 Python通过。修正仅影响这些配置定义／新增测试；未受影响的既有105集成与最终ScheduleTable183.43s结果继续有效。没有待修正或递延项。

## Verification

运行新增生成配置测试、既有资源／四一致性类最低容量及生成工程回归；缺失、重复、错放、非法值不得产生可生成计划。实际生成资源通过交付构建与真实Task消费；错误配置副本须被实际校验／消费者拒绝。按影响范围运行核心集成、Python、Clippy及增量格式／C99，BMad规格记录结果；测试只用系统临时目录。

验证结果：首轮新增生成配置测试134.55s通过；最终核心全量运行3单元通过、106集成中的105通过，唯一失败为独立ScheduleTable输入遗漏新必选OsUseResScheduler（OS_CONFIGURATION），原生43计时向量单独通过。补齐该产品夹具显式false，并把旧“not-repeating”变异从全局true替换缩至OsScheduleTableRepeating，保留其他策略；受影响正式epic4_sc1_timing_capacity最终183.43s通过，涵盖生成、独立消费者、编辑保存和两个偏移的实际HostBatch。复用未受夹具改动影响的105个有效结果，没有降低断言或规则。最终Clippy correctness／suspicious、增量格式／C99与15 Python通过。四种布尔表示实际编译闭包验证，false／true搬移后由真实生成Task在stage9测试入口各获取／释放两次；true的三种错误资源副本被原生配置拒绝，1下同名显式定义被忽略，false下未支持显式资源拒绝。缺失、重复、非法值、错放合法定义及SC2均无法建立可生成计划。原参考字段和时序预期保持；夹具只更新必要显式参数及其原始字节身份，所有测试中间文件为系统临时目录。
