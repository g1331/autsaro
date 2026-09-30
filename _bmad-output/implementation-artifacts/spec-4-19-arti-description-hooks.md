---
title: '4.19 R24-11 ARTI 描述与真实运行时 Hook'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '0554395b0684e99ed53b5b0f59845963cbc3177f'
story_key: '4-19-生成并验证-r24-11-arti-描述与-hook'
context: []
---

<frozen-after-approval reason="已委托持续完成Epic4；产品条件保持，额外assurance流程已由用户移除">

## Intent

生成R24-11标准ARTI描述、工具绑定及适用OS事件，让交付工程的任务、ISR、Alarm、资源、ScheduleTable、实际原生栈和上下文可观察。描述中的对象、表达式及Hook必须对应同一校验计划和真实backend，独立消费者能够核对身份、事件顺序和实际状态。

## Boundaries & Constraints

保持固定单核SC1、真实FreeRTOS/Win64、唯一调度backend、C99和MISRA编码指导。ARTI只观察，不参与选择或建立第二状态权威；调试表达式使用全局数据/常量，不含函数调用或局部变量。ARTI_TRACE的context/class/实例/event使用规范字面token，core为0；服务事件只覆盖应用调用，内部Alarm/表动作不得冒充应用。跟踪有界、无阻塞I/O，不在Task/Hook分配或写报告。按实际配置输出对象，SC1未配置的应用隔离、Spinlock、时间保护不输出虚构实例。BMad为唯一流程、状态和验证记录；不生成额外报告或矩阵，不推送，不重选OS或扩展其他Epic。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 已校验输入／生成后搬移工程 | Arti模块配置使用R24-11正式定义；OS短名、对象ID、类型、引用和符号与真实生成配置一致，XSD及引用闭合，独立构建通过 |
| 原生任务激活、切换、等待、事件释放、终止及抢占 | 六个基本Task事件与真实转换对应；表达式可读取真实观测值，不伪造Running、不漏或重复状态通知，不改变已有轨迹 |
| Cat2嵌套、标准Hook及应用服务成功／失败 | Cat2 Start/Stop配对、Hook Start/Return及服务Start/Return参数符合规范；成功不返回的服务不产生虚假Return；内部动作不会额外产生应用服务事件 |
| Alarm／ScheduleTable计时、资源获取／释放、栈和模式 | 描述能定位实际对象、状态和有效性；独立预期与实际backend观察一致 |
| 断开描述引用、错误签名／对象ID、删除适用Hook | XSD、引用／消费者编译或行为检查明确拒绝，原输入和旧输出受到保护 |

</frozen-after-approval>

## Code Map

- `core/src/integration/ecu.rs`：同一计划生成配置和交付闭包；`configuration.rs`提供正式定义与对象路径。
- `core/src/integration/arti.rs`（新增）：标准ECUC ARTI对象、表达式、类型映射和Hook描述，按实际OS短名生成工具绑定。
- `runtime/os/src/Os_Backend.c`：真实选择／切换、队列、等待及Cat2帧；`Os_Time.c`、`Os_Schedule.c`为实际计时状态与内部动作。
- `runtime/os/src/Os_Error.c`：Hook包装及错误快照；公开服务位于`Os.c`、`Os_Time.c`、`Os_Schedule.c`、`Os_Interrupt.c`。
- `runtime/os/src/Os_Stack.c`：真实Windows栈；`core/src/integration/offline.rs`及`runtime/ecu/build.ps1`、`scripts/epic4_os.py`：交付与原生构建闭包。
- 官方本地OS SWS §7.17／§10.4及ARTI SWS配置章节、已固定MOD/XSD：正式定义和语义依据，原件不提交。

## Tasks & Acceptance

- [x] `core/src/integration/arti.rs`／`ecu.rs`：生成标准描述及工具绑定，复用实际对象和固定依赖。
- [x] `runtime/os/include/Arti.h`／`Os_Arti.h`和相关src：实现有界标准跟踪、实际观测数据和初始化／版本接口。
- [x] backend／错误／服务／时间／中断源码：接入实际转换点，准确分开应用服务和内部动作。
- [x] `offline.rs`／构建脚本：源码和头文件随工程交付，搬移后闭包不依赖仓库。
- [x] `core/tests/support/epic4_arti.rs`／`runtime/os/tests/arti.c`及正式入口：独立描述、真实行为、故障变异和回归；BMad复核、同步sprint、本地提交。

Given同一输入和目标，when生成、搬移、构建并独立消费描述／Hook，then标准描述与真实状态和预期轨迹一致，关键拒绝可识别破损实现，既有调度、计时及交付行为保持。

## Implementation Notes

## Review Triage Log

| 来源／发现 | 判定 | 依据与处理 |
| --- | --- | --- |
| 盲审：任务暂停后保存CONTEXT不刷新，可能读取旧上下文 | false | 实际交付补丁0005在挂起当前线程后执行Os_PortGetThreadContext及Os_StackObserve，再处理ISR和选择任务；后者更新ARTI表达式指向的saved_contexts和有效标志。补丁0002同样保留此路径，原生上下文检查通过；无需修改。 |

BMad盲审、边界、验证缺口及验收审查均已完成；除上述已否定发现外无其他已证实问题，无延后项。

## Verification

正式入口`epic4_arti_description_and_hooks`；按变更选择原生成功／拒绝、生成／搬移回归、Python、核心Clippy与增量格式／C99检查。结果写入本工件，测试中间文件仅临时目录，不改受版本管理报告。

实现记录：实际生成Arti模块／逻辑核／Task／主机输入ISR／原生栈与保存CONTEXT、Counter、Alarm／ScheduleTable和可选RES_SCHEDULER；指向原始状态存储或内核观察值。基本Task／Cat2／五Hook和应用服务事件已接入，内部动作与嵌套逻辑上下文区分。标准uint32参数旁保留Win64完整指针和真实服务状态；固定4096槽、发布标志与饱和丢弃计数不参与调度。R24-11 ARTI附录C已核对，所有表达式使用外部链接符号及其允许运算。

最终验证：`epic4_arti_description_and_hooks`定向158.30s通过，包含三个生成／搬移配置、R24-11 XSD和独立定义／引用／表达式消费者、35原生向量、三种描述／绑定拒绝及两个实际编译实现变异拒绝。关机后的测试断言使用独立进程退出，实际主机输入ISR槽30由独立预期检查。

`python -B -X utf8 scripts/quality.py --base 0554395b0684e99ed53b5b0f59845963cbc3177f`通过源码卫生、增量格式、Python与主机C99检查。15项Python测试通过。`python -X utf8 scripts/verify.py --scope core`最终通过3单元和107集成测试（1199.55s，无忽略／失败），随后核心Clippy correctness／suspicious检查通过。普通测试未写受版本管理报告，验证结果保存在本BMad工件。

这些检查覆盖实际SC1主机描述、生成、运行及拒绝行为，不声称完整MISRA或官方符合性认证。
