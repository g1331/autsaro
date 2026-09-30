---
title: '4.18 生成并运行Counter OsService端口与C/S接口'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'bd915f9791ac2bd91798f2b7947ac0a0fc75119c'
context: []
---

<frozen-after-approval reason="用户已授权完整Epic4及按独立增量持续实施">

## Intent

生成工程已有Counter原生API和RTE共享类型，但尚未交付SWS_Os_91027／00560要求的OsService提供端口与ClientServerInterface。为同一校验计划的Counter生成标准服务描述、真实C服务实现和绑定客户端入口，保持原生Counter状态由OS唯一拥有。

## Boundaries & Constraints

固定单核SC1、Win64、参考单软件Counter及其真实配置，不改变调度、时间源或OS路线。接口OsService_{Counter}为服务接口；GetCounterValue为OUT，GetElapsedValue为INOUT Value／OUT ElapsedValue，TimeInMicrosecondsType为uint64，CounterType为uint32。按R24-11 p228参数定义返回原始tick及tick差，不额外换算微秒。提供端口OsService以CounterType端口定义参数绑定真实Counter句柄；对应C服务器转发现有API，客户端绑定所选句柄。C99、共享类型、错误／输出保持和调用层级继续成立。函数与文件名称冲突用现有完整编译链接闭包拒绝。生成描述是产品工件，验证／复核只用BMad，不生成assurance报告。不扩展其他Epic，不推送，不据此提前关闭4.18。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 校验计划有配置OsSecondsPerTick；生成并搬移构建 | 提供OsService端口、OsService_{Counter}服务接口、32／64位类型及两个操作；描述通过R24-11 XSD且引用闭合，源码／头文件／绑定完整交付 |
| 实际OS Task消费所生成客户端，推进软件Counter及回绕 | GetCounterValue返回实际值，GetElapsedValue更新前值并返回正确模数差；不同Counter短名的绑定仍一致 |
| 直接服务器无效ID／越界前值／空指针／被屏蔽调用 | 原生状态、参数快照和输出保持契约完整传播，没有第二份Counter状态或伪E_OK |
| 生成服务签名、端口参数或源码被断开／符号冲突 | 独立结构／类型／行为检查或真实构建拒绝，不安装错误输出，不损坏旧工程 |

</frozen-after-approval>

## Code Map

- core/src/integration/os_service.rs：从ScheduleContract和同一Counter句柄生成自含服务ARXML、Rte_Os.h和Rte_OsService.c；名称取合法短名，复用共享Rte_Os_Type.h。
- core/src/integration/mod.rs、ecu.rs：接入确定性产品生成，进入现有文件完整性清单、C99双入口预检和安全安装；保留其他源和接口。
- runtime/os/src/Os.c、Os_Time.c、include/Rte_Os_Type.h：复用实际GetCounterValue／GetElapsedValue及共享ABI，不复制计数状态，不改变实现。
- runtime/os/tests/counter_service.c：工程外实际Task消费者和独立tick／回绕／拒绝字面预期，使用完整原生backend；不以Mock服务代替。
- scripts/epic4_os.py：在既有临时原生构建中显式接入由核心测试提供的实际生成源／头文件，保留固定编译器和原件身份校验。
- core/tests/support/epic4_os_service.rs、end_to_end.rs：正式注册生成／XSD／引用／签名及实际原生消费测试；复用既有Scratch、build_plan和独立构建方式。

## Tasks & Acceptance

- [x] 生成真实服务类型、提供端口、参数绑定、操作与服务器描述，XSD／引用闭合。
- [x] 实现共享ABI的真实C服务及客户端绑定，接入完整生成／编译闭包。
- [x] 验证实际Task下tick／回绕／错误／输出保持与改名绑定，补签名／断开拒绝。
- [x] 完成BMad验证与三路复核，本地提交并继续父story。

## Implementation Notes

已核对本地R24-11 OS p227～229及实际00053 XSD的PortAPIOption／PortDefinedArgumentValue、服务接口和Runnable结构。CodeGraph已定位现有生成、类型和双入口链接预检消费者；由主代理实现，代理只读复核。

## Spec Change Log

## Review Triage Log

| 来源／发现 | 判定／处置 | 依据 |
| --- | --- | --- |
| blind：服务ARXML未列出ILLEGAL_ADDRESS／DISABLEDINT／CALLEVEL，因而与原生返回不一致 | false／reject | R24-11 OS p228的SWS_Os_00560明确规定GetCounterValue操作为E_OK／E_OS_ID，GetElapsedValue为E_OK／E_OS_ID／E_OS_VALUE，接口全局错误为0／1／3／7／8；当前描述逐项一致。防御检查保持原生状态属于C接口契约，不应擅自改写标准服务定义；实际生成Rte_Os.h包含Os.h，调用方可识别这些原生常量。 |
| edge：原生防御错误未在操作中声明，生成客户端无法关联返回状态 | false／reject | 同一标准定义不要求把非法指针、屏蔽或非法层级的防御状态扩入操作ApplicationError；Rte_Os.h→Os.h→Os_Types.h实际公开E_OS_CALLEVEL=2、E_OS_DISABLEDINT=9及E_OS_ILLEGAL_ADDRESS=10。客户端可直接识别，真实Task测试亦按这些符号检验结果及ErrorHook快照。 |
| verification-gap | 无发现 | 独立层已按指令核对测试与调用路径。 |

没有本增量待修正或递延事项；父4.18配置一致性继续开放。

## Verification

正式新增服务测试须实际生成并编译C服务器和客户端，在真实OS Task执行独立预期；原生非法调用状态和输出哨兵保留。检查生成ARXML的独立类型、方向、错误和PortDefinedArgumentValue，并用合法外部XSD验证；生成源码的签名／绑定故障副本须实际拒绝。运行相关生成／Counter类型／时间／原入口回归、Python、增量格式/C99和核心Clippy。所有中间文件在临时目录，结果进入本规格。

最终代码验证：正式epic4_generated_counter_service134.88s通过；SystemCounter／AuxClock均使用实际生成的所有OS头文件和服务C，真实Task验证tick、回绕、七错误／参数快照及输出保持，四种实际编译／运行故障副本被拒绝。两种服务符号／头文件冲突由完整生成预检拒绝。服务ARXML通过外部R24-11 XSD，独立检查类型／方向／错误／端口参数／符号和全部内部引用。搬移生成工程237.84s、Counter时间83.75s、公共Counter类型9.91s通过。15 Python、增量格式／C99、项目既有Clippy correctness／suspicious命令通过；未宣称全MISRA符合。额外尝试把所有Clippy warning升级时触发既存collapsible_if／result_large_err诊断，未改写既有项目检查配置。所有中间文件为系统临时目录，未产生受版本管理的报告。
