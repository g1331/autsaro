# OS

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_OS` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR／OSEK 符合或认证。

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3（`os223.pdf`）。编号全集取自固定 SpecificationsARXML，`ECUC_Os_` 与 `ECUC_Arti_` 条目也在其中。

## 已选配置

单核 SC1，固定一个后端（FreeRTOS 内核加本项目的 OS 适配层）。ECU 只有一个所属任务、一个软件计数器（1 ms／tick）、一个应用模式；周期触发来自 RELATIVE 自动启动的闹钟或重复、不同步的调度表，动作只有对所属任务的 SetEvent；资源只有可选的虚拟 RES_SCHEDULER。Error／Pre／Post／Startup／Shutdown hook、栈检测与 ARTI 始终启用。

OS-Application、IOC、多核、spinlock、时间／内存保护、ProtectionHook、外设区域与调度表同步不属于 SC1，在生成前于源对象位置拒绝，相应条款按配置不适用。ISR 配置、显式资源、ActivateTask／callback／IncrementCounter 闹钟动作、绝对起点和超出参考形态的 Task／Counter 取值属于 SC1 能力，当前生成器同样在源位置拒绝，但这是待 [#190](https://github.com/g1331/autsaro/issues/190) 实现或记为差距的缺口，不算不适用。生成的 OS 只包含调度实际使用的对象，未使用的 OsAlarm、OsEvent、OsAppMode 或调度表同样拒绝，不会静默丢弃。

## 处置与证据

- [ECUC 配置与生成](configuration.md)：OsOS、hook 与 SC1 以外能力的 `ECUC_Os_` 条目、OS 生成章节与固定策略。
- [SC1 以外的能力](extensions.md)：多核、OS-Application、保护、IOC、外设、调度表同步等由配置拒绝证明不适用的条目。
- [ARTI](arti.md)：ARTI 描述文件、hook 宏与 `ECUC_Arti_` 条目。

## SC1 内核义务（整改中）

以下条款族由 [#190](https://github.com/g1331/autsaro/issues/190) 逐条处置，SchM 与 BSW 通用义务由 [#191](https://github.com/g1331/autsaro/issues/191) 处置；在此之前它们仍是已定位条款，不计入处置。目标等级须覆盖 SC1 能力与最小容量，不缩减为参考配置实际使用的对象。

| 义务族 | 条款 | 要点 |
| --- | --- | --- |
| 一致性类与容量 | OSEK §3.2（p13–14）；`SWS_Os_00241`、Table 7.4（p129–131） | BCC1／BCC2／ECC1／ECC2；SC1 至少两个调度表、八个软件 Counter |
| 激活顺序与抢占恢复 | OSEK §4.3／4.5（p19–20） | 全部激活请求按优先级内请求顺序记录，不只记数量 |
| 终止／链式／错误返回 | OSEK §4.7、§13.2.3.2–3（p23／51–52）；R24-11 任务缺失结束条款（p74–75） | 合法调用不返回原应用帧；错误调用返回并保留源实例与目标状态 |
| 非抢占与内部资源 | OSEK §4.6、§8.7 | 逐任务属性、内部 ceiling 的释放／重新取得 |
| 事件 | OSEK §7、§13.5（p26–28／60–62） | 自有 mask；检查与进入 WAITING、Set 与 ready 转换同一原子域 |
| 资源 | OSEK §8；R24-11 §7.9.21（p105） | 固定 ceiling／LIFO；不能以 mutex 继承代替 |
| Counter／Alarm／ScheduleTable | R24-11 §7.3、§7.5–7.7、§8／10；`SWS_Os_00560` | 回绕、零偏移、同 tick 动作顺序、相对／绝对起点 |
| 栈 | §7.5；`SWS_Os_00067／00068／00396` | Task／Cat2 ISR 栈故障检测；无 ProtectionHook 时调用 ShutdownOS |
| 生命周期、ISR、错误、Hook | §7.1／8／10；OSEK §6 | StartOS／ShutdownOS、Extended Status、中断禁用配对、调用层级 |
| RTE 调用的 OS 服务 | `SWS_Os_00560`、`00794`、`00786` | GetCounterValue／GetElapsedValue 的同步／重入属性 |

## 已知差距

- 引用的 OSEK 版本 ISO 17356-3 全文未取得，严格 ISO 条款由 [#57](https://github.com/g1331/autsaro/issues/57) 跟踪；固定 FreeRTOS 内核（[ADR 0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)）的上游代码诊断标记为 `adopted` 但不排除（[ADR 0008](../../adr/0008-misra-baseline-and-no-deviation.md)），由 [#58](https://github.com/g1331/autsaro/issues/58) 跟踪。
- Windows 实际交接运行只在 CI 的 Windows 套件中执行。
