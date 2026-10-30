# ErrorHook 参数存储期

R18.6为Required/System（主文印刷150／PDF158；AMD2追加C11），AMD4.24（印刷15／PDF22）扩展到thread-local。不能把正常join、后续不再读取或phase guard直接等同于不存在长寿命地址副本。Os_LinuxCaptureContext先完成局部快照及目标自有FP指针再写目标；复制的是copied中的指针值，不是&copied。此判断只覆盖该具体绑定，不替其他信号帧成员或provider地址证明存储期。

HostBridge以既有sole native producer、同一target不可重入为前提，将watch context保留为文件内static host_watch。两字段在每次创建observer前写入，上次observer已wait/close；未新增owner状态或锁，目标接口的native_thread绑定不冒充Bridge入口锁。真实静态地址编译属性对照与正常多批次／背压执行分开取证。host/probe入口传NULL，Task启动数据及其指向的task_memory均为静态对象；不把这些实际调用外推为任意外部callback参数均符合。

Os_BackendInterruptLeave在恢复后清空退役error副本，避免继续留存已恢复的参数；父TLS状态和外层interrupt保持。Os_ErrorResult按已校验0/1开关从局部两项const source表选择empty_parameters或arguments，STD_OFF不再捕获指针字段，STD_ON保持getter语义；没有新增恒定控制表达式或宏。当前未使用槽的观察在父参数仍有效时进行，撤销清理和恢复禁用复制须被精确位置拒绝，不以失效地址解引用作反例。

上述清理不把 enabled ErrorHook 的初始长寿命复制记为 R18.6 满足。
按 [ADR 0025](../../../adr/0025-misra-layered-scope.md)，本项以固定 OSEK/VDX 2.2.3
§11.2（印刷41页）及 §13.8.4（印刷69页）为依据，不再等待 ISO 原件。
§11.2 要求服务标识 accessor；第一参数为 object identifier 时要求该参数 accessor，
其他被提供的参数按服务名／参数名命名，§13.8.4 将其使用上下文限定为 ErrorHook。
该正文没有把所有输出指针 getter 规定为不可省略的义务。

当前 `Os_ErrorResult` 将有指针成员的 arguments 复制到 TLS parameters；
`Os_ErrorContextSave/Restore` 在嵌套中断中转存并恢复同一快照。
STD_OFF 选 empty_parameters，退役 ISR 槽和退出 Hook 的 TLS 均已清空，
这些控制防止后续读取并保留父 Hook，但不改变初始地址复制的 R18.6 义务。
实际 GetEvent 等自有调用方包含自动输出对象，不能将全部输入说成静态存储来源。
保留所有 getter 的现有公开合同与当前内存安全证据，不能由“同步调用且返回前清空”
推导 R18.6 不适用，也不能仅据上述 object-identifier 义务把可选指针参数统一放入 ADR 0023。
具体原始位置、证据身份与本次处置状态归对应 PR；[既有清理闭包](https://github.com/g1331/autsaro/pull/371#issuecomment-6399598461)
继续作为未变实现的依据。

## 精确 ErrorHook 项目风险处置

[2026-10-30 项目决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6514732824)
依据当前明确工程委托，选择保留既有 getter 合同；它不是维护者直接批准 R18.6 的转述。
适用边界冻结于367d401b：两个受控 C99 x64 目标、既有失败服务／getter集合、
OS_USE_PARAMETER_ACCESS=STD_ON 的 typed TLS 与严格嵌套 ISR 快照。STD_OFF 仍选空参数。
不覆盖任意短命地址保存、新服务、异步回调、其他目标或改变后的 owner／嵌套模型。

主文§5.4（印刷11–12／PDF19–20）允许项目建立偏离程序，要求记录规则、精确条件、
相对替代方案的风险、保障证据与后果／缓解；§6.2.2（印刷13／PDF21）要求声称符合时
对 Required 违反履行正式偏离程序。R18.6 无适用于这里的同步借用例外；主文静态指针
示例仍判违反，AMD4.24也未增加清空例外。本记录及技术审阅履行项目证据要求，
不替代正式流程、适任技术授权或总体符合性证据，不宣称合法偏离、规则满足或认证。

| 规则 | 精确范围 | 实际复制与条件 | 项目选择来源 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 18.6 | `runtime/os/src/Os_Error.c` 中 `Os_ErrorResult` | 367d401b:169，parameters = *sources[OS_USE_PARAMETER_ACCESS]；仅匹配的失败服务同步Hook期间借用。 | 维护者 委托下项目决定6514732824 |
| MISRA C:2012 Rule 18.6 | `runtime/os/src/Os_Error.c` 中 `Os_ErrorContextSave` | 367d401b:56，context->parameters = parameters；仅现有Backend:446静态ISR frame保存父快照。 | 维护者 委托下项目决定6514732824 |
| MISRA C:2012 Rule 18.6 | `runtime/os/src/Os_Error.c` 中 `Os_ErrorContextRestore` | 367d401b:67，parameters = context->parameters；仅现有Backend:485恢复仍活跃父快照，随后清空退役槽。 | 维护者 委托下项目决定6514732824 |

失败服务的具名自动arguments在Os_ErrorResult返回前有效，输出对象由其调用方提供，
必须覆盖整个同步服务调用。九个指针字段为TaskID、State、Event、CounterValue.Value、
ElapsedValue.Value／ElapsedValue、AlarmBase.Info、Alarm.Tick和ScheduleStatus；getter只
返回原地址，不延长对象寿命。当前全部保存点均为同步调用，未把参数交给未来执行的工作项。
正确调用者不得在ErrorHook中结束其pointee生命周期或保存地址供服务返回后使用。

phase只允许ErrorHook读取，其他Hook／普通调用取得空参数及Unknown服务号。
error_active独立于phase，在失败查询及Cat1进入时保持，阻止递归Hook和覆写父参数；
Cat2由既有hook_depth gate阻止。Cat1在同一原生ISR dispatcher中同步嵌套，Task的TLS
则属于被暂停的原生Task线程；两平台进入／离开均成对保存phase、service、parameters及
ARTI caller，父调用栈在子返回时仍活跃。正常返回Hook清空TLS参数与服务号，Backend恢复后
立即清空退役frame，清空发生在被借用自动对象到期前，不以过期指针比较／解引用取证。
不可返回ShutdownOS另走永久关闭：Os_Closing阻止新ErrorHook，调用actor不再恢复，
控制线程以自己的TLS调用ShutdownHook后退出进程；不承诺被中断帧走正常pop／清空，
也不把遗留快照供其他actor或未来调用读取。这一路径不能作为普通服务返回的替代。

风险是错误服务绑定、递归覆写、恢复不完整或清空遗漏会暴露其他调用／过期地址；
phase gate、独立递归保护、严格LIFO恢复及两级清空共同缓解，任何一项失败须修复。
替代方案是重设计可选getter／保存合同以消除长寿命地址副本；本次维持现有公开能力，
不声称已穷尽局部改法，不用整数编码隐藏指针。服务集合、feature、目标、并发／嵌套、
状态owner或getter合同变化时重评本记录，不能按规则／文件名外推。

正常error-hooks、nested-interrupts及既有STD_OFF九字段／退役槽反例继续验证原合同。
新增定向消费者在自动输出仍有效时检查捕获、子ISR前后父参数、Hook后原始TLS清空及
非ErrorHook访问；移除保存／恢复、phase gate、递归保护或退出清空均须被具体观测拒绝。
构建失败不算行为反例成功，作者执行／独立复核／CI结果集中保留在PR工程收据。
