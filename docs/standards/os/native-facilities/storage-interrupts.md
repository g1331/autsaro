# OS：动态设施与同步嵌套 ISR

## 动态存储及回收责任

D4.12为Required、适用C99（主文34–35／PDF42–43），明确覆盖第三方动态分配；AMD2仅扩展C11。`Os_HostCreateActor`的匿名mmap执行栈及guard是真实动态存储，静态handles／TCB／FreeRTOS缓冲／altstack不改变其来源。pthread仅获得bootstrap栈大小而非调用方栈；对应glibc的allocate_stack在缓存不足时mmap，并经_dl_allocate_tls／allocate_dtv分配TLS。Win32创建线程也保留／提交native栈；精确Windows DLL／CRT身份及配套实现证据仍需实际目标材料，不由符号或头声明补写。

D4.13按获取、初始化、使用、释放的实际顺序判断。Linux joinable observer在未结束或未成功join时close返回失败，调用方仍持有所有权；join／detach的0→2→1状态只在真实操作成功后发布完成，失败恢复0，并发wait不把进行中当成完成。HostBatch保留signal→wait→close observer→close event，文件内static host_watch具有进程期存储；sole producer且同一target不可重入，上次observer完成wait／close后才重写其字段；创建或执行失败进入既有进程停止，不返回后继续使用失效上下文。H注册失败及请求ID不可得均原生停止，不能隐藏失败后继续StartOS。

有限H actor在注册前保存线程原备用信号栈，在callback完成后、pthread退休前恢复；查询或恢复失败原生停止，observer不替换备用栈。回调全程保留项目guard／altstack；原生适配器以typed syscall同时支持查询和设置，不通过关闭检测器回避栈所有权。D4.13约束这项临时借用后的归还，实际provider异步语义仍须单独取证。

注册H／Task／dispatcher／controller按当前进程期拥有。detach不终止actor，terminal park不使pthread返回，关闭观察句柄不能写成所有native栈资源均已归还。对应glibc的join／detach回收进入__nptl_deallocate_stack；默认bootstrap栈缓存可保留映射／TLS，缓存再用不同于重新分配，也不同于直接归还系统。项目执行栈的munmap、provider缓存和静态槽位分别核算。同进程重启／返回宿主或不同provider需重新完整证明，不借本项解除#200。

动态设施按下列已接受类别记录处置；固定上界、只在启动分配或主机角色不是原文例外。

## 嵌套中断与递归的实际支持范围

R17.2为Required（主文137／PDF145，AMD2.27增加C11），禁止直接或间接递归；有界深度不构成例外。四个默认生成工程的input／tick优先级均为1，严格更高优先级的nested边在这些工程中不可达；该事实不能代表运行时全部已支持配置。

运行时的较高优先级嵌套ISR确有同步调用环：Linux的run_interrupt→已注册handler→发中断或退出critical→Os_PortDispatchNested→run_interrupt；Windows的prvRunOsInterrupt具有相同闭包。既有nested-interrupts夹具以20→6→4号ISR、优先级3→9→15检查父／子／孙的进入与返回，另覆盖相等／较低优先级延迟、mask、资源ceiling及最大向量。它是实际支持能力，不能为了规则判断把它缩成默认四profile。

OSEK OS2.2.3 §6（25–26页）及§14.2.3.1／Figure14-1（73–74页）描述中断类别、嵌套与调度风险，没有规定必须使用同一C函数递归。当前端口的严格递增优先级、yield排除、32槽frame容量、原生栈／guard及父上下文恢复是风险控制，不能据其把真实R17.2违反判为满足，也不证明任意用户handler的最坏栈用量。

[既有提案](https://github.com/g1331/autsaro/pull/371#issuecomment-6395097006)已由 ADR 0025 接受方案 A1，类别范围见下文。改为延迟处理会改变既有抢占合同；仅换helper名称不能消除调用环。独立ISR actor／原生上下文切换是可研究的替代架构，需要重新证明身份、容量、mask、资源、故障、Task返回及检测器合同，不能直接声称不可能或已经等价。具体取舍必须连同这些差异评估，不由默认profile或已有绿色CI代替。


## 主机动态设施类别偏离

规则为 D4.12、R21.3／R22.1（Required）。授权为 [ADR 0025](../../../adr/0025-misra-layered-scope.md)及其项目决定，依据只引用 [D4.12 方案1](https://github.com/g1331/autsaro/pull/371#issuecomment-6393803857)和 [TLS 补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6401003340)。

范围：`runtime/os/src/host/{linux,windows}/` 的 actor／线程创建与原生栈设施、`runtime/os/patches/linux/0001-controlled-posix-port.patch` 及 Windows 受控 port／线程补丁的实际创建路径，由 `runtime/ecu/src/Ecu_HostBridge.c` 的 HostBatch observer 生命周期消费。限 linux-x64-controlled-v1（C99／glibc-pthread x86-64）及 windows-x64-controlled-v1（C99／Win32）、现有四 profile 的 HostBatch／probe 与同实现 native 验证。D4.12 覆盖执行栈／guard 映射、bootstrap、线程描述符／TLS 和平台线程栈。R21.3／R22.1 仅覆盖 Linux 已核 provider 的 pthread_create→allocate_stack→_dl_allocate_tls→allocate_dtv/calloc，以及 NPTL 缓存和进程期 actor 的 DTV 释放边界，不扩至 Windows 未核实现或自有业务分配。

理由：当前原生抢占、物理栈／ARTI、guard 和同步监督合同依赖这些宿主设施；静态汽车缓冲或仅启动分配不能消除 provider 分配，直接改后端需要重新证明上述合同。保留真实违反，不把 join／进程退出说成已显式 free 全部 TLS。

风险与控制：内存／句柄不足、缓存及分配耗时、初始化和回收失败；保持现有 96 句柄槽、21 注册栈及配置上界，检查创建／回滚结果，保持所有权、guard、每批 observer signal→wait→close 和既定监督路径。D4.13 初始化／join／close 错误仍正常修复；既有 native 生命周期、失败回滚和 HostBatch 检查承担行为证据。provider 内部与任意阻塞输出不承诺默认完成期限。

重评：provider／TLS 缓存、ABI、flags、handler、容量、分配入口、actor 生命周期、同进程重启／退出或 restart 合同变化。项目补丁按自有代码评估，未核 Windows provider 内部不写成已核；Windows 以必需 CI 的实际编译／运行为集成证据。

## 同步嵌套 ISR 类别偏离

规则 R17.2（Required），依 [ADR 0025](../../../adr/0025-misra-layered-scope.md)接受[方案 A1](https://github.com/g1331/autsaro/pull/371#issuecomment-6395097006)。范围仅上述两个受控 C99 端口现有 SC1 同步嵌套：Linux `0001-controlled-posix-port.patch` 的 run_interrupt→handler→post_interrupt／退出 critical→Os_PortDispatchNested→run_interrupt，以及共同 `0012-nested-interrupt-dispatch.patch` 的 Windows prvRunOsInterrupt／nested dispatcher 环；四 profile 和实际支持的更高优先级配置均在范围内。

理由：子 ISR 必须在父 handler 继续前完成；改为父返回后排队会改变已有抢占语义。OSEK 并不强制 C 递归，故属于平台类别取舍。风险为栈耗尽、父身份或 mask／ErrorHook 状态损坏。控制是优先级在 1..31 严格递增、排除 yield、32 槽 frame、同一 S actor／原生栈与 guard、父上下文／ErrorHook 快照恢复及资源 ceiling。`nested_interrupts.c` 的 ISR20→6→4（优先级3→9→15）检查同步顺序；默认样本相同优先级不能代替此证据。任意应用 handler 的最坏栈用量仍须按其配置核定。

provider／ABI、优先级范围、frame 容量、handler、mask、资源或调度合同变化即重评；不覆盖任意递归、用户回调环或新增平台。

## 人工评估的配置与证据范围

#385 的人工记录只覆盖固定 checker 不覆盖的 66 项；有 checker 的 155 项仍包含部分能力限制。
正常 `c_check.coverage` 的 `rules[].assessment=not_assessed` 是原始目录状态；独立的人工记录汇总不改写它。
`satisfied`、`not_applicable`、`retained`、`not_assessed` 分别表示限定范围已核对、条件不成立、保留真实违反及理由、仍有具体缺口。
`retained` 不计作满足；Required 的正式偏离义务与项目风险选择分别保留。尚未核实的记录不计入已分类数。
规则的 TU／System 属性取原文；指令没有 Analysis 属性（主文§6.5–6.6），表中“指令”表示须结合项目设计与实际跨 TU 闭包评估。

配置为 `linux-x64-controlled-v1`／`windows-x64-controlled-v1`、C99、四 profile
standard-ecu／user-application／component-pipeline／no-diagnostic、各自 host-batch／probe。
自有平台无关源码、主机适配和项目有序补丁分别核查；原始 SDK 与固定 FreeRTOS 原件沿
[ADR0025](../../../adr/0025-misra-layered-scope.md)及[采用责任](../adoption.md)处理，以下结论不逐条认证供应方内部。
自有调用、宏用途和系统交互不因采用来源免除。人员培训／资质记录按 #385 的明确排除不在此范围。

源码冻结为已合并 #397 的 `e8c6bf7696bbb71721443c2234f894277e80ce42`（master `6c59ba235548e848852b0ae3940d160e1d87972f`），
以及独立核对的 #393 `1306e32a7352f545006aab7d942c19b47046400b`；后者不是合并后的组合验证。
两组各八份实际封存源码／summary逐记录核对，补丁后内核与输入原件分开；原始 exit1、passed=false、error=null、refusal和全部源身份保留。
实际编译／行为与变异证据分别见[#397最终收据](https://github.com/g1331/autsaro/pull/397#issuecomment-6520362578)、
[#393八份交付](https://github.com/g1331/autsaro/pull/393#issuecomment-6518916990)、
[#381精确收据](https://github.com/g1331/autsaro/pull/389#issuecomment-6471535155)和
[#383采用／补丁收据](https://github.com/g1331/autsaro/pull/395#issuecomment-6519130040)。
新 head、target、profile、程序、producer或实际构造改变时重评；记录不自动评估未来生成物或任意外部应用。

原文为 March2013 主文＋AMD1–4／TC1–2，七份收到字节均已通过 SHA256SUMS；身份入口见
[规范来源](../../../../.agents/skills/misra-c2012/references/standard-baseline.md#原文定位与身份)。
下表主文页为印刷页，PDF页加8；AMD1／AMD3加8，AMD2／AMD4加7，TC1／TC2加6。
AMD2.14／27的语言扩展不改变 C99 义务；每项另列实质修订。表格是既有主题的人工结论，不是新的诊断基线或偏离审批表。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| D1.1 | Required | 指令／C99 | satisfied | 记录并理解影响输出的实现定义行为（主文21–23，AMD2.15–18、101–103）。[实现依赖](../implementation.md#实现依赖编译与需求追溯)给出整数、ABI、volatile、字符及库边界；[Windows所选ld身份](../implementation.md#windows-外部名字与所选工具)已连接固定包及完整字符串处理材料。库内部沿ADR0025，不把GCC版本或静态断言单独视为全部证明。 |
| D2.1 | Required | 指令／C99 | satisfied | 全部实际源文件无编译错误（主文23；AMD2.19）。上述双平台四profile的真实TU编译、封存build及host-batch/probe消费者成功；此结论取编译日志及实际源集合，不取外层分析exit0或绿色摘要。 |
| D3.1 | Required | 指令／C99 | satisfied | 代码追溯至书面需求（主文23）。[实现与需求连接](../implementation.md#实现依赖编译与需求追溯)、[运行时架构](../../../architecture/runtime.md)、各模块标准篇将BSW/OS服务、配置producer、RTE通信、HostBatch监督、guard/restart及失败响应连接至所选SWS或项目合同；seal只证明身份，供应方内部沿ADR0025。未处置AUTOSAR条款不因此变成已实现。 |
| D4.1 | Required | 指令／C99 | satisfied | 以设计、值域和动态检查减少运行失败（主文24–25；AMD2.20），并非保证不会失败。实际容量/索引、空指针、配置关系、分配回滚、guard及停止路径见[原生设施](README.md)与[输入先于tick修复](https://github.com/g1331/autsaro/pull/397)的成功/交错反例；任意阻塞输出没有新增完成期限。 |
| D4.2 | Advisory | 指令／C99 | satisfied | 文档说明汇编理由及C接口（主文26）。[平台扩展](../platform.md#c99-平台扩展的使用边界)和[restart架构](../../../architecture/runtime.md#激活重启e4-host-restart)连接RSP读取、Linux typed syscall、CallOnStack及两端restart的寄存器/clobber、对齐、CFA/unwind和实际消费者；不是“主机所以不适用”。 |
| D4.3 | Required | 指令／C99 | satisfied | 汇编隔离在汇编函数/C函数/宏内（主文26；AMD2.21–22）。Os_LinuxNative的native_call/Os_LinuxAltStack和独立Os_LinuxCallOnStack汇编函数、OS_HOST_STACK_POINTER宏及Os_PortRestart平台函数为实际边界；文件作用域定义汇编函数不等于业务语句散布汇编。 |
| D4.7 | Required | 指令／C99 | not_assessed | 有错误信息的返回须有意义地检查，只有证明实际实参不可能失败才适用例外（主文30）。原生失败/回滚及[errno实参依据](../platform.md#errno-的错误来源与诊断裁决)有效；Com.c的Com_TriggerIPDUSend和Com_MainFunctionTx仍显式丢弃可能拒绝的transmit返回。无重试配置不自行取消本指令，须核清必需合同并整改或形成有原文的精确处置；不能由(void)或其它错误处理销项。 |
| D4.10 | Required | 指令／C99 | retained | 防止头内容重复纳入（主文32–33；TC2.2只说明保护形式非穷举）。普通头有唯一guard；重复包含MemMap依[既有D4.10精确记录](../../memmap/README.md)保留START/include→STOP/include契约，复用完整配对/缺STOP/段属性消费者。项目冲突处置不等于正式Required偏离完成。 |
| D4.11 | Required | 指令／C99 | satisfied | 进入受限定义域库函数前验证参数（主文33–34；TC1更正J.3.12，AMD3.7–9补浮点域）。[字符串/复制容量](../implementation.md#字符串终止与复制容量)、[native上下文与实际provider](README.md)连接有界memcpy、终止串、格式、句柄、signal集合和上下文容量；实际接口不使用math/ctype域。供应方内部不逐条复评，外部应用调用前提仍有效。 |
| D4.12 | Required | 指令／C99 | retained | 动态分配包括第三方设施（主文34–35），固定上界/只启动分配不是例外。仅[既有主机动态设施类别](#主机动态设施类别偏离)的mmap、pthread/Win32栈及Linux TLS路径保留；静态汽车配置、采用原件与自有补丁分开。保留真实违反、分配失败控制和正式Required义务。 |
| D4.13 | Advisory | 指令／C99 | satisfied | 资源操作顺序正确（主文35–36）。本篇实际observer signal→wait→close、失败恢复所有权、altstack借用/恢复及[HostBatch生命周期](README.md)为证据；detach/永久park不是线程已结束或全部TLS已释放，后者仍由D4.12/R22.1处理。 |
| D4.14 | Required | 指令／C99 | satisfied | 外部值先验证有效性（AMD1§1.1，1–2）。Ecu_HostBatch的number/command检查溢出、token、ID/DLC/长度、epoch/容量及顺序，Ecu_HostBridge验证发布容量，公开BSW/OS接口先验证配置/ID/指针；[协议真实消费者及交错反例](https://github.com/g1331/autsaro/pull/397)覆盖拒绝和状态保护，不把内部可信配置等同任意外部输入。 |
| D4.15 | Required | 指令／C99 | not_applicable | 可能产生NaN/Inf的浮点运算须检测，可按传播语义延后但不可流入未设计处理的代码（AMD3.10，5–6）。所选自有/补丁源码仅有float32/64和port类型定义、long-double对齐存储/sizeof，无浮点算术或math入口；[具体构造依据](../../types.md#整数常量初始化与浮点构造)。 |
| D5.1 | Required | 指令／C11 | not_applicable | AMD4.2（2–4）限制线程间数据竞争，仅适用C11；当前C99使用GNU原子/pthread/Win32，非C11原子或threads.h。实际并发风险仍按[平台扩展](../platform.md#c99-平台扩展的使用边界)、D4.1与独占区协议核查，不称没有并发。 |
| D5.2 | Required | 指令／C11 | not_applicable | AMD4.2（4–5）禁止线程死锁，仅适用C11；所选源码没有C11线程/锁对象。POSIX/Win32的实际锁序、回调边界与等待仍受[独占区协议](../../../architecture/runtime.md#独占区协议)和D4.1约束。 |
| D5.3 | Required | 指令／C11 | not_applicable | AMD4.2（5–6）只允许明确启动阶段创建线程，仅适用C11；无C11线程入口。HostBatch每批创建observer，不能写成只在启动创建；其真实生命周期沿本篇D4.12/13控制。 |
<!-- misra-manual-end -->
