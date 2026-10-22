# OS：ARTI

本篇属于 [OS](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。

生成器为每个工程写出 `Os_Arti.arxml`：ArtiHardware 的核类与核实例、ArtiOs 的 OS、任务、ISR、闹钟、调度表、资源、栈与上下文的类和实例，以及引用的值表达式与类型映射。`Os_Arti.c` 把内核事件转成标准 `ARTI_TRACE` 调用，工具提供的 `Arti.h` 决定宏展开。官方套件对四个生成变体（闹钟、改名 OS、调度表、RES_SCHEDULER）检查 schema、按官方 MOD 检查每个容器的重数与引用，并用独立的表达式探针编译核对；生成描述中出现的 `ECUC_Arti_` 条目由该测试的 `Verifies:` 处置。

工具绑定的 `ARTI_MODULE_ID` 与 `Arti_GetVersionInfo` 返回 module5（BSWGeneral 附录 A.2 p95）；既有 catalog／BSWMD 已为5，独立 consumer 从哨兵值断言返回字段。软件版本初值保持1.0.0，唯一数值 owner 为 `Arti_Version.h`，API、BSWMD 与模块手册均从它取得三个分量；`release_contracts::arti_software_api_observes_each_owner_component_and_each_assignment` 独立修改每个分量观察返回值，并删除每个源赋值验证哨兵拒绝。vendor0仍是未分配占位。正式身份与其他 ARTI 义务不由此修复推定完成。

## SWS_OS 中的 ARTI 条款

应用上下文中的每个服务调用都产生 AR_CP_OS_SERVICECALLS 的进出事件，包括 GetCoreID；原生 `arti_consumer` 核对 hook 中各服务调用的事件与参数（`SWS_Os_00843` 由 `check_arti` 的 `Verifies:` 处置）。

| 对象 | 处置 |
| --- | --- |
| ARTI 描述与 hook 宏 | 已核对（描述：`arti.rs` 生成 `Os_Arti.arxml`，官方套件 `official_descriptions` 用 `arti_reader.py` 按官方 MOD 检查重数与引用；宏：`Os_Arti.c` 以 `ARTI_TRACE(<上下文>, <类>, <OS 短名>, 0u, <事件>, <参数>)` 产生 AR_CP_OS_TASK 六个转换、AR_CP_OS_CAT2ISR 起止、五个 hook 进出与应用服务进出事件，原生 `arti_consumer` 编译并核对；可选的 AR_CP_OSARTI_* 增强状态机未提供，其模板义务只约束提供时的形式；条款列出的 ORTI 容器中，ArtiOsMessageContainer 描述 OSEK 内部通信消息，§7.1.2.1 原文规定 AUTOSAR OS 无需支持内部通信，主线因而没有消息实例，ArtiOsSpinlock 随 spinlock 配置拒绝，其余容器均生成）：SWS_Os_00829、00836、00837、00840、00841、00842、00844、00849、00856、00857、00858、00859 |
| AR_CP_OS_APPLICATION | 不适用（配置：ECUC_Os_00114）：SWS_Os_00838、00839 |
| AR_CP_OS_SPINLOCK | 不适用（配置：ECUC_Os_00258）：SWS_Os_00845、00846 |

## ECUC_Arti

| 对象／条件 | 处置 |
| --- | --- |
| 可选扩展引用 | 已核对（`ArtiGenericComponent` 扩展引用均为 0..1 或 0..*；`arti.rs` 只为计数器生成通用组件，不扩展这些对象，`official_descriptions` 按官方 MOD 接受其缺省）：ECUC_Arti_00064、00065、00070、00076、00077、00078、00084、00085、00110、00116、00121、00123、00138、00143、00149、00153、00178、00183、00191、00195 |
| 主线未提供的可选观察量 | 已核对（0..1 参数：LastError、运行任务优先级、服务跟踪、闹钟动作说明、栈填充模式、调度表下一事件与核引用、OS-Application 当前值的类声明；虚拟 RES_SCHEDULER 与主机输入 ISR 没有 EcuC 对象，故不写 EcuC 引用；`official_descriptions` 按官方 MOD 接受其缺省）：ECUC_Arti_00054、00066、00067、00082、00083、00094、00095、00097、00098、00112、00142、00152、00186、00224 |
| OSEK 消息容器 | 已核对（OSEK COM 消息不属于 AUTOSAR OS 对象，ArtiOsMessageContainer 为 0..* 容器，生成器不产出）：ECUC_Arti_00126、00127、00128、00129、00130、00131、00132、00133、00134、00135 |
| 时间保护观察量 | 不适用（配置：ECUC_Os_00325）：ECUC_Arti_00213、00214 |
| ISR 时间保护观察量 | 不适用（配置：ECUC_Os_00259）：ECUC_Arti_00211、00212 |
| Spinlock 对象 | 不适用（配置：ECUC_Os_00258）：ECUC_Arti_00194、00196、00197、00198、00199、00200、00201、00202、00203、00204 |
