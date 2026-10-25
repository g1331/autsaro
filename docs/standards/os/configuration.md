# OS：ECUC 配置与生成

本篇属于 [OS](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。

内置 metadata 保留官方定义的类型、默认值、范围和重数，R24-11 OsOS 的三个必填参数 OsLockTrustedFunctionCall、OsStackMonitoring、OsUseArti 与其余参数同样必填；官方重数为 0..1 的 OsScalabilityClass 由集成器要求显式给出。集成器另核对消费语义：未列入主线的容器与参数在内置目录中没有定义，按 `DEFINITION_UNKNOWN`／`REFERENCE_UNRESOLVED` 在源位置拒绝；已定义但取值超出所选剖面的参数按 `TARGET_PARAMETER_UNSUPPORTED` 在所属容器拒绝。每个拒绝都有注入该配置的测试（`os_configuration.rs`、`os.rs`），测试同时断言集成计划与普通校验报告同一文件和对象。

## ECUC_Os

| 对象／条件 | 处置 |
| --- | --- |
| Os 模块与 OsOS | 已核对（`builtin.rs` `operating_system()` 定义与 `configuration.rs` 的 `os_policies`：OsOS 唯一，OsStatus、OsScalabilityClass、OsStackMonitoring、OsUseArti、OsUseGetServiceId、OsUseParameterAccess、OsUseResScheduler、OsLockTrustedFunctionCall 逐一必填；OsStatus 生成 `Os_Cfg.h` 的 `OS_STATUS_EXTENDED`；OsUseGetServiceId／OsUseParameterAccess 两值均接受，分别生成 `OS_USE_GET_SERVICE_ID`／`OS_USE_PARAMETER_ACCESS`，由 `error_hook_access_switches_follow_the_configuration` 核对；OsStackMonitoring／OsUseArti 固定为 true，与运行时始终启用的栈检测和 ARTI hook 一致；OsLockTrustedFunctionCall 两值均接受，没有 trusted function 时不影响生成；缺项与错值由 `scheduler_configuration_rejections`、`os_policy_values_outside_the_profile_are_rejected` 拒绝）：ECUC_Os_00044、00046、00047、00048、00307、00396、00406、00410 |
| OsScalabilityClass | 已核对（`configuration.rs` 固定策略只接受 SC1；`os_policy_values_outside_the_profile_are_rejected` 拒绝 SC2–SC4）：ECUC_Os_00259 |
| OsUseResScheduler | 已核对（`configuration.rs` 要求显式布尔值；`ecu.rs` 为 true 时生成虚拟 RES_SCHEDULER，并忽略同名配置资源；原生 `scheduler_resource_runs` 运行四种取值）：ECUC_Os_00049 |
| OsHooks | 已核对（`configuration.rs` 固定策略：Error／Pre／Post／Startup／Shutdown hook 为 true，生成 `Os_Hooks.h` 回调；ProtectionHook 只接受 false）：ECUC_Os_00035、00036、00037、00038、00039、00040、00214 |
| OsAppMode | 已核对（`schedule.rs`：唯一 OsAppMode 必须名为 OSDEFAULTAPPMODE，否则以 `OS_DEFAULT_APP_MODE_REQUIRED` 在该模式拒绝，由 `default_application_mode_is_required` 核对；任务、闹钟和调度表的自动启动都引用它，生成的 ECU 调用 `StartOS(OSDEFAULTAPPMODE)`；其余模式以 `OS_OBJECT_UNUSED` 拒绝；`arti.rs` 把它写入 ARTI OS 实例）：ECUC_Os_00022 |
| OsEvent | 已核对（`configuration.rs` 为所属任务的事件分配单比特 mask，省略 OsEventMask 时自动分配；未被所属任务引用的事件以 `OS_OBJECT_UNUSED` 拒绝）：ECUC_Os_00033、00034 |
| 调度表同步 | 已核对（`configuration.rs` 固定策略：OsScheduleTableSync 只接受 OsScheduleTblSyncStrategy=NONE，IMPLICIT／EXPLICIT 由 `os_policy_values_outside_the_profile_are_rejected` 在所属容器拒绝；同步属于 SC2／SC4，见 [SC1 以外的能力](extensions.md)）：ECUC_Os_00063、00065 |
| OS-Application 及其访问授权 | 不适用（配置：ECUC_Os_00114）：ECUC_Os_00004、00020、00021、00031、00054、00077、00114、00115、00116、00124、00125、00213、00221、00230、00231、00234、00254、00392、00394、00395、00402、00408 |
| 多核 | 不适用（配置：ECUC_Os_01019）：ECUC_Os_01019 |
| 外设区域 | 不适用（配置：ECUC_Os_00397）：ECUC_Os_00397、00398、00399、00400、00401 |
| Spinlock | 不适用（配置：ECUC_Os_00258）：ECUC_Os_00258、01021、01022、01038 |
| 任务周期（只供 RTE 等计算时基的信息，OS 不消费） | 不适用（配置：ECUC_Os_00404）：ECUC_Os_00404 |
| 任务时间保护 | 不适用（配置：ECUC_Os_00325）：ECUC_Os_00082、00083、00084、00085、00086、00185、00325、00391 |
| 可调 expiry point | 不适用（配置：ECUC_Os_00068）：ECUC_Os_00068、00069、00070 |
| 显式同步精度 | 不适用（配置：ECUC_Os_00064）：ECUC_Os_00064 |
| IOC | 不适用（配置：ECUC_Os_01000）：ECUC_Os_00405、00407、01000、01001、01003、01005、01010、01012、01014、01015、01016、01017、01023、01024、01035、01036 |

## Task、Counter、Alarm 与 ScheduleTable

生成器只接受参考 ECU 的形态：一个 Extended、全抢占、激活上限 1、引用三个事件和 OSDEFAULTAPPMODE 的所属任务；一个 SOFTWARE、TicksPerBase 与 MinCycle 为 1、1 ms／tick 的计数器；RELATIVE 自动启动、周期等于映射事件周期的 SetEvent 闹钟，或单点、重复、不同步的调度表。其余容器在源位置以 `DEFINITION_UNKNOWN`／`REFERENCE_UNRESOLVED`／`OS_RESOURCE_PROFILE` 拒绝，其余取值以下表所列代码在所属容器（闹钟周期在映射的 RTE 事件）拒绝；`sc1_shapes_outside_the_reference_ecu_are_rejected`、`os_policy_values_outside_the_profile_are_rejected` 与 `unselected_os_features_have_no_definition` 逐项注入并断言代码与对象。依赖被拒形态的条款按配置不适用。运行时本身支持更一般的 SC1 配置并有原生测试；让生成器接受它们由 [#198](https://github.com/g1331/autsaro/issues/198) 跟踪。

| 对象／条件 | 处置 |
| --- | --- |
| OsTask | 已核对（`schedule.rs` `inspect_events`：唯一 OsTask，OsTaskActivation 为 1、OsTaskSchedule 为 FULL、OsTaskPriority 在 1–30、恰好三个 OsTaskEventRef、唯一 OsTaskAutostart 引用唯一应用模式，其余以 `TASK_PROFILE`／`SCHEDULE_NOT_UNIQUE` 拒绝；`Ecu_Config.c.in` 写入优先级、事件掩码与自动启动模式）：ECUC_Os_00073、00074、00075、00076、00078、00080、00081 |
| OsCounter | 已核对（`schedule.rs`：唯一 OsCounter，SOFTWARE、OsCounterTicksPerBase 与 OsCounterMinCycle 为 1、OsSecondsPerTick 为 0.001，其余以 `COUNTER_PROFILE`／`SCHEDULE_NOT_UNIQUE` 拒绝；OsCounterMaxAllowedValue 生成 `OSMAXALLOWEDVALUE_<Counter>` 与运行时模数，`counter_service_description` 用 15 核对）：ECUC_Os_00026、00027、00028、00029、00030、00255 |
| OsAlarm 及其自动启动与 SetEvent 动作 | 已核对（`schedule.rs`：每个闹钟绑定一个周期 RTE 事件，OsAlarmCounterRef 指向唯一计数器，唯一 OsAlarmAutostart 为 RELATIVE、OsAlarmAlarmTime 与 OsAlarmCycleTime 等于事件周期、OsAlarmAppModeRef 与所属任务模式一致，唯一动作 OsAlarmSetEvent 设置所属任务的事件；缺少自动启动以 `PERIOD_ALARM_CONFLICT` 在闹钟拒绝，周期不符在映射事件拒绝；`ecu.rs` 生成 `OS_ALARM_EVENT` 闹钟）：ECUC_Os_00003、00005、00006、00009、00010、00011、00012、00013、00016、00017、00018 |
| OsScheduleTable 及其自动启动与 SetEvent 动作 | 已核对（`schedule.rs`：调度表绑定周期 RTE 组，OsScheduleTableCounterRef 指向唯一计数器，Duration 等于周期，唯一 expiry point 带唯一 OsScheduleTableEventSetting、OsScheduleTableRepeating 为 true，RELATIVE 自动启动的 StartValue 与 offset 之和等于周期；表自身形态以 `PERIOD_SCHEDULE_TABLE_CONFLICT` 在表拒绝，多个事件动作以 `PERIOD_ALARM_CONFLICT` 在 expiry point 拒绝；`ecu.rs` 生成重复、不同步的 `Os_ScheduleTableConfigType`）：ECUC_Os_00053、00056、00057、00058、00059、00060、00061、00062、00141、00143、00144、00145、00335 |
| ActivateTask 闹钟动作 | 不适用（配置：ECUC_Os_00007）：ECUC_Os_00007、00008 |
| Alarm callback | 不适用（配置：ECUC_Os_00014）：ECUC_Os_00014、00087、00409 |
| IncrementCounter 闹钟动作 | 不适用（配置：ECUC_Os_00302）：ECUC_Os_00015、00302 |
| 调度表任务激活 | 不适用（配置：ECUC_Os_00066）：ECUC_Os_00066、00067 |
| 时间常量 | 不适用（配置：ECUC_Os_00386）：ECUC_Os_00002、00386 |
| 硬件计数器驱动 | 不适用（配置：ECUC_Os_00371）：ECUC_Os_00032、00371 |
| 配置资源（虚拟 RES_SCHEDULER 之外） | 不适用（配置：ECUC_Os_00252）：ECUC_Os_00050、00051、00052、00252 |
| 任务资源引用 | 不适用（配置：ECUC_Os_00079）：ECUC_Os_00079 |
| ISR 及其时间保护 | 不适用（配置：ECUC_Os_00041）：ECUC_Os_00041、00042、00043、00222、00223、00229、00326、00387、00388、00389、00390、00403 |

## 生成

生成器的一致性检查即定义校验与集成计划：任一错误都使生成入口得不到计划，因而不产出 OS。`SWS_Os_00850`、`SWS_Os_00851`（虚拟 RES_SCHEDULER）、`SWS_Os_00370`（`meta/os-generation-timing.json` 中的内部定时器报告）与 `SWS_Os_00393`（`OS_TICKS2<Unit>_<Counter>` 宏）由测试的 `Verifies:` 处置。

| 对象／条件 | 处置 |
| --- | --- |
| 读入与一致性检查 | 已核对（读入：`build_plan_native` 读取所选 ARXML 输入；一致性检查：定义校验与集成计划在生成前给出带对象位置的诊断，生成入口 `prepare_ecu_project_with_applications` 只接受已通过的 `ValidatedIntegrationPlan`；与等级不符的属性报错而非警告，比条款要求更严）：SWS_Os_00172、00173、00179、00320 |
| 服务保护与等级组合 | 不适用（配置：ECUC_Os_00259）：SWS_Os_00050、00328、00461 |
| 多核或 SC3／SC4 的 OS-Application 归属 | 不适用（配置：ECUC_Os_00114、ECUC_Os_00259）：SWS_Os_00177、00311、00343、00344、00361 |
| 时间保护组合 | 不适用（配置：ECUC_Os_00325）：SWS_Os_00045、00562 |
| 隐式同步时长 | 不适用（配置：ECUC_Os_00065）：SWS_Os_00440 |
| 规范附录 | 已核对（规范原文：附录列出不适用于本规范的上游需求，没有实现义务）：SWS_Os_NA_00767 |

中断向量表段（`SWS_Os_00336`）与 OS 声明的 MemMap 包装（`SWS_Os_00815`）随 SchM／MemMap 责任链在 [#191](https://github.com/g1331/autsaro/issues/191) 处置。

## C99 数组与回调类型

R17.6为Mandatory（主文印刷140／PDF148），R18.7为Required（印刷151／PDF159）；AMD2追加C11。当前四份callback-parameter工程与两目标补丁后内核中，没有static数组形参，也没有柔性数组成员。未写边界的声明已分别核对：fatal文本及生成配置表由字符串／初始化列表确定大小，都是完整对象；内核prvSearchForNameWithinSingleList的pcNameToQuery[]是函数参数，不是结构成员。生成Task表中的.entry等designator初始化结构元素，不是给外层数组指定下标；不能仅凭其中出现点号，就按R9.5的数组designator构造裁决。

R18.8为Required，AMD4.25–29将其明确到VLA；新增R18.10为Mandatory（AMD4.31，印刷17／PDF24），约束指向可变修改数组类型的指针。当前声明边界来自整数字面量、固定宏／枚举或非VLA类型的sizeof／对齐表达式：任务缓冲区为512个StackType_t，OS上限、协议容量及本机65536字节备用栈由固定定义决定，CAN_HOST_AREA_COUNT和OS_HOOK_ALARM是枚举常量。编译器实际宏收据确认当前容量；timers、registry、TLS及多核专属声明仍按各自关闭条件判断。没有发现运行值决定的数组声明或指向数组的可变修改指针；局部char条件数组的sizeof／offsetof布局约束是整数常量表达式，不是运行时VLA。此结论只覆盖已核声明、配置和生成来源，不覆盖SDK类型。

R17.13为Required（AMD3.38，印刷20–21／PDF28–29），适用当前C99。已核回调typedef均形成函数指针类型；配置中的const限定指针对象或返回对象，不限定函数类型本身。TaskFunction_t的声明宏同样形成函数指针；私有布局断言的typedef是字符数组。没有以指针对象的const或函数返回类型的const误报函数类型限定，也不从这项判断推导任意外部回调、SDK或其他类型规则已经满足。

R17.3为Mandatory，但主文仅适用C90（印刷138／PDF146）；AMD2的语言表没有为它追加C99／C11。当前真实编译命令为-std=c99，因此原始R17.3报告不能作为本配置的Mandatory违反或偏离记录。C99调用前声明的语言约束仍由R1.1及实际原型／包含链承担；严格C编译通过不替代R8.2命名、R8.4可见声明或跨目标ABI核查，既有62条标准ECU原始报告仍保留。
