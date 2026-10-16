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
| Expiry point 与计数器 | 已核对（`Os_Time.h`：`Os_ExpiryPoint.offset` 为相对表起点的 tick，`Os_ScheduleTableConfig.counter` 为唯一驱动计数器，`Os_Schedule.c` 拒绝未配置的计数器；生成器取 OsScheduleTblExpPointOffset 与唯一 OsScheduleTableCounterRef）：SWS_Os_00404、00409 |
| 自动启动顺序 | 已核对（`Os_Backend.c` 启动序列：先激活自动启动任务，再 `Os_TimeAutostart`，最后 `Os_ScheduleAutostart`）：SWS_Os_00510 |
| 所有等级可用的服务 | 已核对（`Os.h` 无条件声明 StartScheduleTableRel／Abs、StopScheduleTable、NextScheduleTable、GetScheduleTableStatus、IncrementCounter、GetCounterValue、GetElapsedValue）：SWS_Os_00521、00522、00523、00524、00528、00530、00532、00534 |
| GetElapsedValue 的多次回绕 | 已核对（规范说明：服务无法检测多于一次的相对回绕；`Os_Time.c` 按模数求差，与说明一致）：SWS_Os_00533 |
| 调度表类型 | 已核对（`Os_Types.h`：`ScheduleTableType`、`ScheduleTableStatusType` 及 STOPPED／NEXT／WAITING／RUNNING／RUNNING_AND_SYNCHRONOUS、`ScheduleTableStatusRefType`；`sc1_timing` 经 GetScheduleTableStatus 使用）：SWS_Os_00783、00784、00785 |
| ControlIdle | 已核对（`Os.h` 按条款签名声明，`Os.c` `Os_Implementation_ControlIdle` 单核下不检查 CoreID；只支持 IDLE_NO_HALT，它已经生效，因此不存在待切换的空闲模式；`idle_state` 断言返回值）：SWS_Os_00769、00802 |
| EnableInterruptSource 的挂起标志 | 已核对（取舍：与 `SWS_Os_91020` 的 ClearPending 参数冲突时按参数处理，见上文；`Os_Interrupt.c` 在 ClearPending 为 TRUE 时清除挂起标志，`interrupt_pairing` 的 `source-enable-clear` 断言清除、`source-retain` 断言保留并在使能后递送）：SWS_Os_00811 |
| 核编号 | 已核对（`Os_Types.h`：唯一核 `OS_CORE_ID_0` 为 0，`TotalNumberOfCores` 为 1；GetCoreID 返回 0，`calling_context` 断言）：SWS_Os_00825 |

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
