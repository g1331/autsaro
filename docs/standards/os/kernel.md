# OS：SC1 内核

本篇属于 [OS](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。

SC1 的任务、事件、资源、计数器、闹钟、调度表、中断、hook、错误与生命周期条款主要由原生套件处置：`tests/python/native/os_suites.py` 中各 `check_*` 函数上方的 `Verifies:` 列出它独立断言的条目，`core/tests/it/integration/os.rs` 处置生成的服务描述与配置开关。本篇记录不以单个测试证明、经审阅确认的条目，以及依赖生成器所拒配置的条目。

## 规范之间的取舍

- `SWS_Os_00811` 要求 EnableInterruptSource 同时清除挂起标志，后定义的 API `SWS_Os_91020` 增加了参数 ClearPending 来决定是否清除。实现按 ClearPending 处理：TRUE 时清除，FALSE 时保留挂起中断并在使能后递送（`interrupt_pairing` 的 `source-enable-clear`／`source-retain`）。
- 关中断时调用 GetISRID 按 `SWS_Os_00093` 忽略服务并返回 INVALID_ISR；`SWS_Os_00263` 只约束可调用状态下 Cat2 ISR 及其 hook 中的返回值。isOsStarted（`SWS_Os_91034`）同理在关中断时返回 FALSE。
- `SWS_Os_00560` 把 E_OK（0）列为 OsService 的可能结果；在生成的服务描述中它是操作成功，不作为应用错误，应用错误只有 E_OS_ACCESS、E_OS_ID、E_OS_STATE、E_OS_VALUE（`counter_service_description`）。
- Cat1 ISR 中调用服务的结果未定义（`SWS_Os_00088` 只约束不在 Cat1 ISR 内的错误上下文），因此没有 Cat1 调用者的测试预期。

## 审阅确认的条目

| 对象／条件 | 处置 |
| --- | --- |
| OSEK OS API 兼容 | 已核对（`Os.h`：OSEK OS 2.2.3 §13 的任务、中断、资源、事件、闹钟、执行控制与 hook 服务签名，`DeclareTask`／`DeclareResource`／`DeclareEvent`／`DeclareAlarm`、`TASK`／`ISR`／`ALARMCALLBACK` 宏，`OSDEFAULTAPPMODE`、`OSErrorGetServiceId` 与 `OSMAXALLOWEDVALUE`／`OSTICKSPERBASE`／`OSMINCYCLE`／`OSTICKDURATION`；`public_compatibility` 以独立 C99 消费者编译这些声明与 StatusType 编码）：SWS_Os_00001 |
| 高等级特性的接口 | 已核对（`Os_Types.h`／`Os.h`：SC1 同时提供的高等级类型与中断源服务按 R24-11 签名声明，`public_types` 与 `public_compatibility` 按独立约束编译；不支持的高等级服务不声明）：SWS_Os_00240 |
| Alarm callback 只用于 SC1 | 已核对（配置：OsAlarmCallback 由 `unselected_os_features_have_no_definition` 拒绝，SC2–SC4 由 `os_policy_values_outside_the_profile_are_rejected` 拒绝，生成的工程不含 callback）：SWS_Os_00242 |
| OS 自用定时器 | 已核对（主机目标没有 GPT 驱动；固定 FreeRTOS 端口补丁 `runtime/os/patches/0007-controlled-tick.patch` 与 `runtime/os/patches/linux/0001-controlled-posix-port.patch` 由 OS 安装受控 tick 中断，`Os_Time.c` `Os_TimeTick` 驱动系统计数器）：SWS_Os_00374 |
| Expiry point 与计数器 | 已核对（`Os_Time.h`：`Os_ExpiryPointType.offset` 为相对表起点的 tick，`Os_ScheduleTableConfigType.counter` 为唯一驱动计数器，`Os_Schedule.c` 拒绝未配置的计数器；生成器取 OsScheduleTblExpPointOffset 与唯一 OsScheduleTableCounterRef）：SWS_Os_00404、00409 |
| 自动启动顺序 | 已核对（`Os_Backend.c` 启动序列：先激活自动启动任务，再 `Os_TimeAutostart`，最后 `Os_ScheduleAutostart`）：SWS_Os_00510 |
| 所有等级可用的服务 | 已核对（`Os.h` 无条件声明 StartScheduleTableRel／Abs、StopScheduleTable、NextScheduleTable、GetScheduleTableStatus、IncrementCounter、GetCounterValue、GetElapsedValue）：SWS_Os_00521、00522、00523、00524、00528、00530、00532、00534 |
| GetElapsedValue 的多次回绕 | 已核对（规范说明：服务无法检测多于一次的相对回绕；`Os_Time.c` 按模数求差，与说明一致）：SWS_Os_00533 |
| 调度表类型 | 已核对（`Os_Types.h`：`ScheduleTableType`、`ScheduleTableStatusType` 及 STOPPED／NEXT／WAITING／RUNNING／RUNNING_AND_SYNCHRONOUS、`ScheduleTableStatusRefType`；`sc1_timing` 经 GetScheduleTableStatus 使用）：SWS_Os_00783、00784、00785 |
| ControlIdle | 已核对（`Os.h` 按条款签名声明，`Os.c` `Os_Implementation_ControlIdle` 单核下不检查 CoreID；只支持 IDLE_NO_HALT，它已经生效，因此不存在待切换的空闲模式；`idle_state` 断言返回值）：SWS_Os_00769、00802 |
| EnableInterruptSource 的挂起标志 | 已核对（取舍：与 `SWS_Os_91020` 的 ClearPending 参数冲突时按参数处理，见上文；`Os_Interrupt.c` 在 ClearPending 为 TRUE 时清除挂起标志，`interrupt_pairing` 的 `source-enable-clear` 断言清除、`source-retain` 断言保留并在使能后递送）：SWS_Os_00811 |
| 核编号 | 已核对（`Os_Types.h`：唯一核 `OS_CORE_ID_0` 为 0，`TotalNumberOfCores` 为 1；GetCoreID 返回 0，`calling_context` 断言）：SWS_Os_00825 |

## 主机事件的错误信息

Linux `Os_HostLinux.c` 的 `CreateEventA` 为每个事件持有非阻塞 eventfd；`Os_HostSetEvent`、`ResetEvent` 与 `WaitForSingleObject` 由原生端口、tick 确认和关闭路径消费。事件计数只在完整八字节读取后视为已消费；手动复位等待只观察可读性。负 `poll` 或 `POLLERR`／`POLLHUP`／`POLLNVAL` 返回 `WAIT_FAILED`，不能成为正常超时或无限等待。读／写／poll 的 `EINTR` 保留未完成操作并重新调用；读到 `EAGAIN` 表示当前无可消费事件，零读／短读及其他读错误保留失败。

此处按固定 MISRA 2013 主文 D4.7（印刷页30／PDF38）的错误信息义务整改，不是 AUTOSAR 必需冲突或偏离。R17.7（页141–142／PDF149–150）允许显式丢弃普通返回值，但不能替代错误分类。AMD1 R22.8–22.10（页13–16／PDF21–24）也覆盖实际通过 errno 返回错误的附加函数：每次调用前在同函数清零，只有负返回值才按本次 errno 分类；返回值本身可确定是否发生错误时使用 R22.9 的例外。AMD2 的语言扩充及 TC2.38 的 R22.9 标题勘误不改变本项目 C99 的该结论。固定 checker 的 R22.10 原始报告不因此删除或改写。

同一D4.7义务也适用于等待时钟：单对象及wait-any后续读取、多对象初始读取、wait-all逐项读取失败均返回WAIT_FAILED；普通wait-all超时保留WAIT_TIMEOUT。`test_native_handle_errors_and_resource_reuse`用真实事件验证I/O、时钟失败和正常对照；不改变wait-all在现有单消费者启动标记上的顺序获取方式。

`CloseHandle` 的资源状态按 D4.13（2013主文页35／PDF43，AMD2扩充C11）另行核查：mutex只有在真实销毁成功后清除kind并归还固定槽，失败时仍持有原资源；否则正常反复创建／关闭会耗尽96个槽。事件的Linux close即使返回EINTR也已释放描述符，保留失败返回但立即撤销本句柄的fd所有权，避免误关随后复用该数字的资源；依据为 [Linux close契约](https://man7.org/linux/man-pages/man2/close.2.html#NOTES)，不外推其他POSIX目标的EINTR行为。实际消费者执行128轮mutex获取／释放／关闭，并在真正关闭事件后复用同一个fd再注入EINTR，断言失败返回及新描述符仍有效。事件close成功后注入EINTR也验证槽归还；mutex销毁失败经链接器注入，保持API前置条件。

`reserve_handle`以CAS领取槽位，正常关闭以原子操作归还；六个构造失败回滚（actor映射／保护／pthread创建、eventfd、mutex初始化／初始获取）也改用`InterlockedExchange`，避免普通写入与其他领取者竞争。[GCC 13.3 §6.55](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc.pdf)的原子扩展要求无数据竞争，按R1.3（Required，主文页38–39／PDF46–47）评估实际C99实现。先清理再归还，不增加锁或状态，保持注册栈身份和失败mutex所有权。`test_concurrent_event_construction_and_rollback`链接HostServices和Linux设施，以失败／干净对照检查竞争和复用；两个sanitizer lane运行同一回归。

## 原子返回值与条件类型

`Arti.c`、`Os.c`、`Os_Backend.c`、`Os_Interrupt.c`、`Os_Mailbox.c` 中只用于更新状态的32处 Interlocked 调用显式转为 `void`；这些接口返回先前值或更新后的计数，不返回错误信息。Linux `InterlockedCompareExchange` 同样丢弃 builtin 的成功标志并返回交换前值，保持 Windows 的返回契约。此处采用 R17.7（2013主文页141–142／PDF149–150）的允许形式，不把有错误信息的事件／锁调用一并丢弃。`control` 对 `Os_StackRegister`、`Os_StackHasFault` 的整数结果显式与零比较，满足 R14.4（页121／PDF129）的 essentially Boolean 条件要求；原有成功／故障分支不变。AMD2只扩充这两条的C11适用范围，其他修订没有为这些源码点增加条件例外。

`Os_HostInstallSignals`、`try_wait_mutex`、`Os_StackPrepare` 的六处集合初始化调用也显式转为void：目标均为本函数可写的自动 `sigset_t`，sigaddset前已初始化，加入的是所选glibc提供的有效SIGRTMIN；按 [sigsetops契约](https://man7.org/linux/man-pages/man3/sigemptyset.3.html) 不存在无效signal参数，适用D4.7的参数保证例外。其他sigaction、pthread_sigmask和锁操作仍检查真实失败。`Os_PortRestart` 对 `_Unwind_RaiseException` 采用void转换，因为任何返回都代表本次restart未成功，紧接着无条件以E_OS_STATE停止；成功展开不返回。这里没有丢弃可恢复错误后继续运行的路径。

## 移位宽度与时间单位

Linux受控port补丁中，`Os_PortInterruptSourceEnabled`、`Os_PortInterruptSourceControl`、`next_interrupt`、`run_interrupt`、`Os_PortDispatchNested`、`dispatch`（两处）与 `post_interrupt` 共八处中断位图移位的左操作数显式使用 `uint32_t`。R12.2（2013主文页105–106／PDF113–114，含原页示例）约束 essential type 的宽度；在实际GCC模型中 `UINT32_C(1)` 展开成 `1U`，其 essential type 按附录D.3／D.6的UTLR为 unsigned char，不能仅因标准类型是32位就允许移到第31位。TC1附录D.3勘误和AMD2.97不取消此限制。移位量仍由既有0..31边界及调用闭包限制；声明目标、内核选择器和中断状态协议未改变。该补丁属于项目修改代码，显式更新其固定摘要，不把它重新标为未修改的采用代码。原生 `nested-interrupts` 的 `maximum` 向量实际递送至31号中断。

Linux `Sleep` 的两个时间字段先把原始 DWORD 转成实际64位 `time_t`／`long`，再做除法和余数，避免将无符号复合表达式转为不同essential类别。依据R10.8（主文页91–92／PDF99–100）、TC1的模数说明勘误及AMD3.35仅针对浮点／复数的例外；这些例外不适用于此处整数运算。有限DWORD最大值在两个有符号目标类型内可表示，余数0..999乘以1000000后小于1000000000；`INFINITE` 仍在转换前走原停车路径。

## 内核与主机端口的状态类型

采用的 `tasks.c` 以 `TCB_t * volatile` 定义 `pxCurrentTCB`，原两个主机端口却以 `void * volatile` 声明，并通过 `size_t *` 取首字段、转回线程状态指针。整改补丁 `0015-typed-port-state.patch` 在共同的 `task.h` 声明真实 `TaskHandle_t volatile`；`TaskHandle_t` 已是同一 `tskTaskControlBlock` 的指针。内核内部的 `pxTaskOsPortStack` 直接取具名成员，端口只借用该任务的栈位置，不再解读TCB布局或把指针经整数往返。依据R8.4（2013主文页67–68／PDF75–76）；TC1增加的main例外不适用于此对象，AMD2的C11扩充也不改变C99判定。

两个端口在创建任务时用真实自动线程状态对象构造句柄，以unsigned char表示复制到原有StackType_t数组保留区，返回该数组内的StackType_t指针；读取时复制回真实同类型对象。保存区按元素数向上取整，不新增分配或任务选择状态。读取逐字节保留TCB成员原有volatile限定；Windows删除路径写回清空的线程句柄也保留volatile，不经memcpy去掉限定。初始化时的memcpy两实参均指向unsigned char，只复制本状态对象的sizeof字节。依据R11.3（主文页96–97／PDF104–105，TC2.21交叉引用R18.1，AMD4.7–4.9将标题改为conversion且字符例外限定非atomic对象）、R11.8及AMD1 R21.15（页6／PDF14）。这些对象不是_Atomic；字符访问和表示复制有明确的原文允许条件，不能推广为不同结构指针互转的许可。

任务创建、首次启动、调度器挂起／恢复、中断交接、事件等待、Windows非当前任务删除和自删除都消费同一状态表示；Windows自删除仍先写回NULL再关闭句柄。保护仍由原调度门、任务停止和初始化时序提供；volatile不是新增同步机制。源清单保留全部固定上游原始摘要，仅增加项目受控补丁的身份，交付目标登记和原生构建入口同时选择该补丁。旧封存工程仍按自身身份识别，新的生成工程包含新内核／端口字节，不能拿旧工件当作本改动的分析结果。

`StaticTask_t` 和 `StaticQueue_t` 的旧dummy结构虽与实际对象尺寸对齐，仍不是同一C对象类型；把其地址转成TCB／Queue后访问字段不因尺寸相同而满足R11.3。相同补丁把真实任务、队列定义移入各自内核头，静态缓冲类型直接引用原始tag，不复制第二份字段布局。内核源码、静态分配调用者、idle任务内存回调和队列创建／静态缓冲查询因而使用相同类型；结构字段和调度算法保留。独立C99消费者检查这两种静态对象类型、任务栈身份与队列FIFO／满空／peek／reset；旧dummy类型必须编译失败。

链表哨兵选用上游已有的完整ListItem_t表示（`configUSE_MINI_LIST_ITEM=0`），消除MiniListItem_t与ListItem_t之间的不同结构访问；这改变的是内核存储表示，没有减少任务、中断、队列或配置支持。每个List_t在两个64位目标上增加两个指针字段，静态缓冲随真实类型统一计算，必须重新生成并重编译直接消费者。采用源码仍保留其原始摘要，项目承担新增补丁和此表示选择的责任；上游的coverity说明注释不作为本项目偏离许可。

同一补丁把Windows端口的七处UINT32_C(1)位图移位也改为显式uint32_t左操作数，理由与Linux的R12.2处置相同；原有四处1UL左操作数保持，它的essential type按原文页106示例为unsigned long，在所选32位long的Windows目标上允许0..31的移位。内核创建任务时直接断言端口返回位置处于同一栈数组的有效下界和原栈顶之间，避免将指针差的复合表达式转为无符号整数，修复R10.8；两个指针仍位于调用者的同一栈数组。`Os_Backend.c` 的backup controller参数使用其真实静态句柄对象的地址作为非NULL身份，替代整数1转void指针；control仍只检查是否为NULL，不解引用该借用地址，依据R11.6（主文页99／PDF107，只有零整数常量的例外；TC2.22将整数转void指针的说明更正为implementation-defined，AMD2扩充C11不改变本项）。

Windows采用端口在处理器数不满足其既有进入条件时保留pdFAIL及调度启动失败路径，删除其中非必需printf和stdio.h包含；外层仍走正常的OS失败报告，不以C标准I/O作为内核低层报错设施。此处按R21.6（2013主文页169／PDF177，AMD2.79覆盖标准I/O标识符／宏和wide I/O）修复独立的采用代码，不追认上游例外，也不重复#60已整改的一方I/O。新表示复制显式包含string.h，ptrdiff_t显式包含stddef.h。

人工核查将相同移位根因继续追到`Os.c`的资源任务组／任务mask与`Os_BackendResource`：实际支持16个Task，验证只把ID限制为0..15，因此1u的unsigned-char essential宽度仍不足；四处改为uint32左操作数。`Os_InterruptAllows`和`source_control`的两个UINT32_C(1)同样改为uint32，仍由0..31边界保护。保留全部Task／ISR容量，`capacity`与`nested-interrupts`继续消费真实配置。内核优先级、任务名长度和notification数组数量采用unsigned常量，队列unsigned长度／大小及scheduler挂起计数按同类别比较，处理R10.4；这些固定数值和原有拒绝边界没有改变。公共类型与boolean宏的处置见[标准类型](../types.md#c99-essential-type-与实际常量)。

人工核查的R22.8还覆盖`Os_HostLinux.c`的时钟、sysconf、映射／保护／回收、eventfd、等待中的nanosleep、Sleep和信号安装，以及`Os_StackPrepare`的两个sigaction。每次调用前在同一函数清零；短路中的连续调用拆开，SIGRTMIN的动态求值在清零前完成。R22.9原文例外允许由返回值判定是否出错，不增加重复errno判错；失败、EINTR重试和资源回收仍走原结果路径。依据AMD1 R22.8–22.10（页13–16／PDF21–24）及TC2.38，另按所选Linux的[时钟](https://man7.org/linux/man-pages/man2/clock_gettime.2.html#RETURN_VALUE)、[sysconf](https://man7.org/linux/man-pages/man3/sysconf.3.html#RETURN_VALUE)、[映射](https://man7.org/linux/man-pages/man2/mmap.2.html#RETURN_VALUE)和[nanosleep](https://man7.org/linux/man-pages/man2/nanosleep.2.html#RETURN_VALUE)契约分类。直接内核边界`Os_LinuxAltStack`／futex返回负错误值，pthread调用返回错误号，不因此伪装成errno接口。

`NativeFacilityTests.test_native_errno_across_initialization_wait_and_cleanup`以真实设施成功后遗留非零errno检查九类入口，覆盖EINTR、保护／创建失败、可回收actor、等待与mutex。signal.h／time.h规则及原始分析／人工状态另行处置。

R10.1（主文页82–84／PDF90–92、TC1逻辑指针补充及AMD3.20表）要求逻辑操作数为essential Boolean；R14.4（页121–122／PDF129–130）也覆盖单次执行宏的循环条件。`0016`将采用内核断言的指针／计数／原生BOOL参数显式比较，tasks／queue／list宏用`false`；自有configASSERT用`FALSE`，OS原生整数返回值显式比较，私有故障谓词采用boolean。参数仍只求值一次，原来的失败／非空／非零含义与全部配置容量保持。实际activation、nested-interrupts、resources、error-hooks及静态对象消费者验证未改变调度／队列行为；固定addon对R10.1逻辑操作数和R14.4的for条件覆盖不足，不能据其无报告判定满足。

R13.3（Advisory，主文页111／PDF119）处置到trace、线程表、mutex mask及两端口故障文本：存储与索引自增分成两个full expression，保留原顺序和容量。R15.7（Required，页129／PDF137、TC1函数调用澄清、TC2.29交叉引用）在ARTI出／入边、故障controller标记和source_control的五处else-if链明确无动作含义。未增加新状态或容错分支。

采用头`include/deprecated_definitions.h`的15个旧端口文件名通过0017改用`/`分隔符，未删除分支或文件。R20.2（Required，主文页156／PDF164）禁止header-name中的反斜杠，不以所选端口未进入这些分支判为豁免；上游原件不变，补丁与摘要固定交付内容。

## 目标端口承担的差距

| 义务 | 处置 |
| --- | --- |
| ShutdownHook 返回后关中断并进入死循环 | 差距 #200：SWS_Os_00425 |

主机受控目标在 ShutdownHook 之前已关闭输入与 tick 中断，返回后结束进程而不是死循环，以便测试与工具观察停机结果；真实 MCU 的 OS 端口须提供该停机行为。

## 依赖生成器所拒配置的条目

运行时支持这些能力并有原生测试，但生成器在源对象位置拒绝对应配置（[#198](https://github.com/g1331/autsaro/issues/198) 跟踪放开），所列参数都有 `Rejects:` 测试。

| 能力 | 处置 |
| --- | --- |
| IncrementCounter 闹钟动作及其循环检查 | 不适用（配置：ECUC_Os_00302）：SWS_Os_00301、00303 |
| 调度表任务激活 | 不适用（配置：ECUC_Os_00066）：SWS_Os_00402 |
| 单次调度表的配置 | 不适用（配置：ECUC_Os_00144=false）：SWS_Os_00413 |
| 绝对闹钟的自动启动 | 不适用（配置：ECUC_Os_00011=ABSOLUTE）：SWS_Os_00476 |
