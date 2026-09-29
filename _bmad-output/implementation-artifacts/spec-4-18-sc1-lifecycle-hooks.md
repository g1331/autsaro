---
title: '4.18 完整适用生命周期、Extended Status、Hook 与模拟中断'
type: 'feature'
created: '2026-09-29'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '81492c568c9d359fd1dc2e0fb406754e383eee7b'
story_key: '4-18-补齐生命周期-extended-status-hook-和中断义务'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

使既定单核 SC1／EXTENDED 的真实 FreeRTOS Win64 后端完成全部适用错误、Hook、启动关闭、逻辑 ISR 与 OSEK 四一致性类最低容量义务。标准服务的成功、错误及清理路径由真实对象和实际内核调度证明；完整等级仍须经4.19～4.22出口。

## Boundaries & Constraints

继承4.2封存507行本故事义务：160 applicable、346 conditional_na、1 conditional；逐条建立结果，不因参考ECU未用而删除能力。缺少同步、多核、OS-Application或IOC等前提时保留封存分类并说明具体条款前提；一般公共类型定义不能借SC1选择跳过。保持SC1／EXTENDED／单核／NONE、已采用FreeRTOS版本及真实原生栈，不重开选型、不扩展Epic2/5/6、不推送、不新增汽车线程或墙钟tick。

所有实现使用C99及MISRA C:2012修订基线。Hook在真实转换边界运行，不用私有trace替代；不得在汽车owner或Hook中阻塞输出。Startup/Shutdown原有关闭控制栈、不可逆关闭与栈故障保护保留。标准调用层级按OSEK表和R24-11逐服务处理，拒绝必须无副作用，输出参数在拒绝时保持。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 四一致性类合法容量配置，实际启动、激活、等待、资源和Alarm动作 | BCC1/BCC2分别至少8非挂起Task、8优先级；ECC1/ECC2至少16与16；资源分别1/8/8/8、各2内部资源、1Alarm、1模式；ECC各Task至少8事件能力。实际运行及独立观察证明，不仅检查常量 |
| BCC1/ECC1唯一优先级及单次激活；BCC2/ECC2重复优先级及多次Basic激活 | 独立轨迹符合类规则；适用抢占/非抢占、FIFO、事件和资源能力复用已有效证据并补缺；RES_SCHEDULER显式配置，不凭空创建 |
| Task入口返回，仍持有资源／禁中断／排队激活 | 在离开RUNNING前ErrorHook得到E_OS_MISSINGEND；资源及未配对屏蔽恢复，PostTaskHook和正常终止发生，下一激活或Task真实运行；不能直接整OS关闭 |
| 服务错误、ErrorHook内再次失败、参数宏配置开关 | 标准service ID／准确参数快照与状态可读取，外层快照不被覆盖且不递归；开关配置可追踪；不同层级符合标准许可表 |
| 实际抢占、WaitEvent、Terminate/Chain及显式ShutdownOS | PreTask/PostTask按真实离开／进入RUNNING顺序且Hook查询身份/状态正确；ShutdownOS不调用PostTask；Startup模式与GetActiveApplicationMode一致，成功StartOS不返回 |
| Disable/Enable、嵌套SuspendAll/ResumeAll和SuspendOS/ResumeOS交错 | 保存/恢复原状态，All屏蔽全部可屏蔽应用ISR、OS仅Cat2；无配对恢复无效果；非中断OS服务被忽略并按适用签名返回E_OS_DISABLEDINT |
| Cat2返回时泄漏屏蔽或资源、模拟嵌套ISR | 自动清理并报告E_OS_DISABLEDINT/E_OS_RESOURCE；GetISRID恢复外层；源屏蔽保留pending，ClearPending和Enable(clear)操作真实pending；非法ID/层级无副作用 |
| GetTaskID、GetISRID、isOsStarted、ControlIdle及标准类型 | 无汽车Task/ISR返回规定INVALID值；启动前isOsStarted=false、调用StartOS后true；单核ControlIdle按规范省略CoreID检查，实际支持的host idle动作及无效模式有证据；R24-11公共类型、常量、范围及签名独立编译核对 |

</frozen-after-approval>

## Code Map & Tasks

- runtime/os/include/Os.h、Os_Target.h、Os_Hooks.h及Os_Error.c／内部错误声明：补标准服务、类型、错误码与参数宏；显式可选Hook和两个错误宏配置开关，默认未配置时保留既有消费者行为。已有初始化器和生成配置同步更新，不擅自实现SC2/3/4行为。
- runtime/os/src/Os.c、Os_Time.c、Os_Schedule.c及内部声明：统一服务级错误身份/参数报告和调用层级/禁中断保护；ErrorHook递归抑制及准确输出保持。软件Counter/Alarm配置与私有受控tick owner要求分开验证，纯Basic容量配置不依赖Extended owner。
- runtime/os/src/Os_Backend.c、FreeRTOSConfig.h：接入真实内核SWITCHED_OUT/IN及当前汽车身份；missing-end在原生Task trampoline中清理资源/屏蔽后正常结束；Shutdown路径不补PostTaskHook；保持真实栈关闭机制和StartOS不可返回。
- runtime/os/patches/：仅在实际移植边界需要时增加受控补丁，保留来源/顺序/摘要。模拟pending位派发前统一判断全局、OS类、源及资源门槛；实际源清除与嵌套上下文恢复，不以后台线程模拟新调度。
- runtime/os/tests/sc1_errors_hooks.c、scripts/epic4_os.py：独立字面Hook/错误/参数/ISR/清理/容量轨迹，新进程隔离配置；保留全部原生既有向量并更新被新正确行为改变的missing-end独立预期，记录依据。
- core/tests/end_to_end.rs、生成模板及offline引用：加入epic4_sc1_errors_hooks_and_isr正式入口，生成交付闭包包含新实现/参数配置，实际参考ECU/HostBatch编译链接和行为回归。
- docs/assurance/evidence/epic4/story-4-18.json、spec/sprint和既有说明：507行逐条结果与具体配置/实现/测试可双向追踪；拒绝或未跑如实记录，完整SC1、ARTI、221项和交接未通过不关闭。

## Tasks & Acceptance

- [ ] 核对实际R24-11/OSEK原文、公共类型及完整调用表，冻结独立错误/Hook/中断/四类容量预期。
- [ ] 补标准服务和错误/参数快照、Hook上下文及真实任务转换；实现missing-end正常终止与清理。
- [ ] 补全中断屏蔽配对/嵌套、源pending/ID及Cat2退出清理；实际idle动作与启动状态验证。
- [ ] 四类成功容量与关键拒绝向量、所有适用服务和类型独立验证；507行逐条处置并保留封存适用性。
- [ ] 既有OS/栈/时间/参考ECU/HostBatch回归、最终源码/二进制身份、静态/MISRA核查、完整增量门与三路独立复核，本地提交及sprint同步。

## Implementation Notes

ErrorHook独立增量已接入25个标准StatusType服务边界、有类型参数快照、两个访问宏开关及可选回调配置；真实Task/Cat2/Startup三类执行上下文覆盖。Startup无汽车Task的嵌套错误报告返回INVALID_TASK；ErrorHook内再次失败绕过活跃报告函数，不递归、不覆盖外层快照，回调结束恢复之前Hook限制。Alarm动作错误在标准回调已配置时仅报告一次。三头文件分离避免循环include，生成交付闭包包含新头文件、Os_Error.c和Ecu_OsHooks.c。Pre/Post函数已绑定但真实转换调用仍待实现，绝不按本增量关闭4.18。八个独立新进程覆盖四种开关与配置/未配置，已配置每次31条独立错误记录；部分Cppcheck诊断保留，完整221项仍开放。

下一实施增量的接口选择：ErrorHook参数快照使用逐服务的有类型结构/联合成员，保留TickType与各类输出指针原类型，不通过uintptr_t或void指针强转取回参数。一个错误报告边界负责回调前后上下文与递归抑制；ErrorHook内再次失败只返回状态，不覆盖外层快照。标准ErrorHook与现有Time.error_hook私有兼容通路须明确单次报告责任，原40向量作为回归依据。新可选Hook配置和两个宏开关按真实生产配置接入，旧未配置消费者不引入未定义回调符号。

下一项已定位实际边界：configOS_REQUEST_FIFO的taskOS_SELECT_READY直接选择ready-list头并赋予pxCurrentTCB，Hook必须在候选确定、赋值前后接入且过滤相同Task；WaitEvent/Terminate/Chain要在waiting/activation状态改写前发Post，避免报告已WAITING/SUSPENDED的任务。当前参考ECUC明确OsErrorHook/Pre/Post及两个访问开关为true，生成配置尚无完整实现；后续须同时补runtime与生成闭包，不能只补测试回调。

容量增量已实现：标准软件Counter/Alarm可独立于私有Extended-owner确认通道使用，wake_event0/ownerINVALID显式选择；该模式没有宿主硬件tick来源，硬件Counter配置拒绝，参考非零wake_event路径保持。GetTaskID读实际kernel选中Task，GetActiveApplicationMode读实际StartOS模式；完整Hook/错误/调用表仍待后续实现。四类最低容量、四类含非抢占Task、两类重复优先级/三次Basic排队激活、模式2及七类配置越界拒绝共18向量通过，实际资源/内部优先级/Alarm二次Task入口、ECC每Task八事件置位清除状态均有独立预期。

首个类型增量：已补公共类型及范围/指针/常量，独立C99消费者覆盖header-only、Windows-before、Windows-after三种真实包含顺序及16项AccessType真值表。原生生命周期46向量通过；初次原生编译暴露Windows COM ApplicationType重名，现由既有Os_Windows.h边界隔离。类型/源码证据在public-type-contracts-4-18.json；本故事其他服务/Hook/ISR/容量仍未完成，不因类型编译通过升级等级。

需求/授权无待确认缺口；无外部不可逆操作。新增公共服务与Hook配置影响OS声明、生成配置和全部本地静态初始化器，按所有消费者同步更新。纯标准公共类型先独立编译核对，条件类型不代表对应SC2/多核行为已实现。

2026-09-29三路只读探查已返回。Task trampoline目前返回即E_OS_STATE关闭；只有Startup/Shutdown Hook，缺标准错误参数宏和Pre/Post；模拟ISR仅单current_interrupt和资源门槛，缺配对/源控制。现有数组上限足够表达四类容量，但没有完整成功容量向量。Os_TimeValidate强制Extended owner，须为纯软件Counter/Alarm实际使用消除不适用的私有tick条件。已读取本地R24-11 OS p74–76、131、194、201–202、205及OSEK最低容量/调用表；ControlIdle单核明确不查CoreID，idle模式硬件相关，isOsStarted在R24-11标为DRAFT，不能冒充已定稿条款。

## Spec Change Log

## Review Triage Log

- ErrorHook增量edge与verification-gap独立复核均无发现。blind提出“GetTaskID标准service ID应0x04”：false，实际R24-11 OS p167的0x04属于CheckTaskMemoryAccess，p147将GetTaskID列为OSEK服务，OSEK2.2.3 §13.8.3只规定各OSServiceId_xx唯一，没有该数值；当前0x80～0x90与AUTOSAR规定的Counter/Schedule ID分开且独立字面预期检查。保留现有值，不将其他模块/实现常量当作本规范义务。

- 容量独立增量三路blind／edge／verification-gap均未发现可定位缺陷或验证缺口；完整故事的Hook/错误/ISR工作仍开放。正式容量/真实生成两项测试163.73s通过；18容量、40受控时间、43计时向量封存，26最低容量义务直接映射到四个主场景，部分静态exit1原样保留。容量增量完整门exit0：85个集成测试（660.24s）、29Python、UI/桌面构建和两组Clippy通过；本故事Hook/错误/ISR仍未完成，sprint保持in-progress。

## Verification

ErrorHook增量完整门exit0：86个核心集成测试600.13s（包含正式epic4_standard_error_hook_parameters、真实参考ECU/HostBatch/栈及所有既有回归）、29Python、UI/桌面构建和两组Clippy通过。最终仅内部声明参数名校正；八配置原生编译/运行、增量quality及桌面构建已针对复验，随后文档身份刷新。部分Cppcheck九个翻译单元exit1，诊断和人工范围核查记录于standard-error-static-analysis-4-18.json。四条相关义务均按部分支持记录，不把未来服务/missing-end/ISR清理或整体SC1标为通过。

正式epic4_sc1_errors_hooks_and_isr、四类最低容量成功运行、错误/Hook/参数/层级/ISR独立向量；受影响既有OS/时间/实际栈/参考ECU/HostBatch回归。完整门python scripts/verify.py --scope all --base <4.17完整本地提交>，测试线程按Owner Guide固定2。实际C99编译、部分Cppcheck与人工重点核查不替代完整221项原文/Required批准或SC1最终出口。全部验收无头后台，原生UI/IPC只能在隔离桌面验证。
