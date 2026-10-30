# 受控库与ARTI观察出口

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

R8.7为Advisory/System；保留具名native控制／观察出口和ARTI视图的原链接合同。其他真实C消费者不反写host-batch／probe的single-TU raw；Os_ArtiCurrentIsr的工具表达式消费单独承认，不当作第二C TU。Ecu_HostBatchParse／Execute的此项报告只限probe。owner、发布同步、边界校验及既有stack／ARTI／HostBatch消费者保持。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/bsw/src/Can.c` 中 `Can_InternalRxIndication` | shape=`CAN_CODE EcuStatus Can_InternalRxIndication ( uint32 id , uint8 dlc , const uint8 data [ 8 ] , uint64 now_ms ) {`; 范围 `All/Both/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/ecu/src/Ecu_HostBatch.c` 中 `Ecu_HostBatchParse` | shape=`StatusType Ecu_HostBatchParse ( Ecu_HostBatch * batch , const char * line , size_t length , Ecu_BatchCommand * command ) {`; 范围 `All/Both/probe`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/ecu/src/Ecu_HostBridge.c` 中 `Ecu_HostBatchExecute` | shape=`StatusType Ecu_HostBatchExecute ( Ecu_HostBatch * batch , Ecu_HostSink sink , void * context ) {`; 范围 `All/Both/probe`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/ecu/src/Ecu_Target.c` 中 `Ecu_TargetPostFrame` | shape=`StatusType Ecu_TargetPostFrame ( uint64_t at , uint32_t id , uint8_t dlc , const uint8_t data [ 8 ] , uint64_t * ticket ) {`; 范围 `All/Both/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os.c` 中 `Os_TargetInspectActivation` | shape=`OS_MEMMAP StatusType Os_TargetInspectActivation ( TaskType id , Os_ActivationInfoType * info ) {`; 范围 `All/Both/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Backend.c` 中 `Os_ArtiCurrentIsr` | shape=`OS_MEMMAP const uint32 * const Os_ArtiCurrentIsr = & current_interrupt ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Backend.c` 中 `Os_ArtiAppMode` | shape=`OS_MEMMAP const AppModeType * const Os_ArtiAppMode = & startup_mode ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Backend.c` 中 `Os_ArtiResourceOwners` | shape=`OS_MEMMAP const uint32 * const Os_ArtiResourceOwners = resource_owner ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Backend.c` 中 `Os_ArtiTaskContexts` | shape=`OS_MEMMAP const void * Os_ArtiTaskContexts [ OS_MAX_TASKS ] ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Error.c` 中 `Os_ErrorParametersCurrent` | shape=`OS_MEMMAP const Os_ErrorParametersType * Os_ErrorParametersCurrent ( void ) {`; 范围 `All/Both/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Error.c` 中 `Os_ErrorServiceId` | shape=`OS_MEMMAP OSServiceIdType Os_ErrorServiceId ( void ) {`; 范围 `All/Both/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Schedule.c` 中 `Os_ArtiScheduleTables` | shape=`OS_MEMMAP const Os_ArtiScheduleStateType * const Os_ArtiScheduleTables = tables ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Time.c` 中 `Os_ArtiCounters` | shape=`OS_MEMMAP const TickType * const Os_ArtiCounters = values ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Time.c` 中 `Os_ArtiAlarms` | shape=`OS_MEMMAP const Os_ArtiAlarmStateType * const Os_ArtiAlarms = alarm_states ;`; 范围 `All/Both/Both`；保留既有ARTI观察出口与原owner／同步；工具读取不冒充样本的第二C TU引用。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/host/linux/Os_HostLinux.c` 中 `GetExitCodeThread` | shape=`OS_MEMMAP BOOL GetExitCodeThread ( HANDLE thread , DWORD * code ) {`; 范围 `All/Linux/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/host/linux/Os_StackLinux.c` 中 `Os_StackFaultCount` | shape=`OS_MEMMAP LONG Os_StackFaultCount ( void ) {`; 范围 `All/Linux/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_Stack.c` 中 `Os_StackFaultCount` | shape=`OS_MEMMAP LONG Os_StackFaultCount ( void ) {`; 范围 `All/Windows/Both`；保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
