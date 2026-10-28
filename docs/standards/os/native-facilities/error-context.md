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
