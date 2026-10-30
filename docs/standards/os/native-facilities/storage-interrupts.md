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
