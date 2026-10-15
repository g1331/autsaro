# OS：ECUC 配置与生成

本篇属于 [OS](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。

内置 metadata 保留官方定义的类型、默认值、范围和重数，R24-11 OsOS 的三个必填参数 OsLockTrustedFunctionCall、OsStackMonitoring、OsUseArti 与其余参数同样必填；官方重数为 0..1 的 OsScalabilityClass 由集成器要求显式给出。集成器另核对消费语义：未列入主线的容器与参数在内置目录中没有定义，按 `DEFINITION_UNKNOWN`／`REFERENCE_UNRESOLVED` 在源位置拒绝；已定义但取值超出所选剖面的参数按 `TARGET_PARAMETER_UNSUPPORTED` 在所属容器拒绝。每个拒绝都有注入该配置的测试（`os_configuration.rs`、`os.rs`），测试同时断言集成计划与普通校验报告同一文件和对象。

## ECUC_Os

| 对象／条件 | 处置 |
| --- | --- |
| Os 模块与 OsOS | 已核对（`builtin.rs` `operating_system()` 定义与 `configuration.rs` 的 `os_policies`：OsOS 唯一，OsStatus、OsScalabilityClass、OsStackMonitoring、OsUseArti、OsUseGetServiceId、OsUseParameterAccess、OsUseResScheduler、OsLockTrustedFunctionCall 逐一必填；OsStatus 生成 `Os_Cfg.h` 的 `OS_STATUS_EXTENDED`；OsUseGetServiceId／OsUseParameterAccess 固定为 true；OsStackMonitoring／OsUseArti 固定为 true，与运行时始终启用的栈检测和 ARTI hook 一致；OsLockTrustedFunctionCall 两值均接受，没有 trusted function 时不影响生成；缺项与错值由 `scheduler_configuration_rejections`、`os_policy_values_outside_the_profile_are_rejected` 拒绝）：ECUC_Os_00044、00046、00047、00048、00307、00396、00406、00410 |
| OsScalabilityClass | 已核对（`configuration.rs` 固定策略只接受 SC1；`os_policy_values_outside_the_profile_are_rejected` 拒绝 SC2–SC4）：ECUC_Os_00259 |
| OsUseResScheduler | 已核对（`configuration.rs` 要求显式布尔值；`ecu.rs` 为 true 时生成虚拟 RES_SCHEDULER，并忽略同名配置资源；原生 `scheduler_resource_runs` 运行四种取值）：ECUC_Os_00049 |
| OsHooks | 已核对（`configuration.rs` 固定策略：Error／Pre／Post／Startup／Shutdown hook 为 true，生成 `Os_Hooks.h` 回调；ProtectionHook 只接受 false）：ECUC_Os_00035、00036、00037、00038、00039、00040、00214 |
| OsAppMode | 已核对（`schedule.rs`：唯一 OsAppMode，任务、闹钟和调度表的自动启动都引用它；其余模式以 `OS_OBJECT_UNUSED` 拒绝；`arti.rs` 把它写入 ARTI OS 实例）：ECUC_Os_00022 |
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

Task、Counter、Alarm、ScheduleTable、资源、ISR、OsTimeConstant、其余闹钟动作与调度表任务激活的参数属于 SC1 能力：当前生成器只接受参考 ECU 的形态（唯一任务、单激活、全抢占、一个 1 ms 软件计数器、RELATIVE 自动启动、SetEvent 动作），其余取值和对象在源位置拒绝。它们不按配置写成不适用，由 [#190](https://github.com/g1331/autsaro/issues/190) 实现或记为差距。

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
