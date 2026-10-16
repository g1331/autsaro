# OS

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_OS` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR／OSEK 符合或认证。

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3（`os223.pdf`）。编号全集取自固定 SpecificationsARXML，`ECUC_Os_` 与 `ECUC_Arti_` 条目也在其中。

## 已选配置

单核 SC1，固定一个后端（FreeRTOS 内核加本项目的 OS 适配层）。ECU 只有一个所属任务、一个软件计数器（1 ms／tick）、一个名为 OSDEFAULTAPPMODE 的应用模式；周期触发来自 RELATIVE 自动启动的闹钟或重复、不同步的调度表，动作只有对所属任务的 SetEvent；资源只有可选的虚拟 RES_SCHEDULER。Error／Pre／Post／Startup／Shutdown hook、栈检测与 ARTI 始终启用。

OS-Application、IOC、多核、spinlock、时间／内存保护、ProtectionHook、外设区域与调度表同步不属于 SC1，在生成前于源对象位置拒绝，相应条款按配置不适用。运行时还实现了更一般的 SC1 能力（多任务与多计数器、ActivateTask／callback／IncrementCounter 闹钟动作、绝对起点、单次与多点调度表、配置资源与 LINKED 资源、ISR），并有原生测试；生成器只接受上述参考形态，其余取值同样在源位置拒绝，依赖它们的条款按配置不适用，每个参数都有 `Rejects:` 测试。放开生成器由 [#198](https://github.com/g1331/autsaro/issues/198) 跟踪。生成的 OS 只包含调度实际使用的对象，未使用的 OsAlarm、OsEvent、OsAppMode 或调度表同样拒绝，不会静默丢弃。

## 处置与证据

- [SC1 内核](kernel.md)：原生套件之外经审阅确认的内核条目、规范之间的取舍与依赖生成器所拒配置的条目。
- [ECUC 配置与生成](configuration.md)：全部 `ECUC_Os_` 条目、生成器接受的形态与拒绝方式、OS 生成章节。
- [SC1 以外的能力](extensions.md)：多核、OS-Application、保护、IOC、外设、调度表同步等由配置拒绝证明不适用的条目。
- [ARTI](arti.md)：ARTI 描述文件、hook 宏与 `ECUC_Arti_` 条目。

## 核算

`requirements SWS_OS` 只剩中断向量表段（`SWS_Os_00336`）与 OS 声明的 MemMap 包装（`SWS_Os_00815`）未处置，它们随 SchM／MemMap 责任链在 [#191](https://github.com/g1331/autsaro/issues/191) 处置；SchM 与 BSW 通用义务（`SWS_BSWGeneral --module Os`）同样由 #191 处置。ShutdownHook 返回后的死循环（`SWS_Os_00425`）由目标 OS 端口承担，记为差距 [#200](https://github.com/g1331/autsaro/issues/200)。SC1 的条款按能力本身处置，覆盖等级要求的最小容量（`SWS_Os_00241`），不缩减为参考配置实际使用的对象。

## 已知差距

- 引用的 OSEK 版本 ISO 17356-3 全文未取得，严格 ISO 条款由 [#57](https://github.com/g1331/autsaro/issues/57) 跟踪；固定 FreeRTOS 内核（[ADR 0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)）的上游代码诊断标记为 `adopted` 但不排除（[ADR 0008](../../adr/0008-misra-baseline-and-no-deviation.md)），由 [#58](https://github.com/g1331/autsaro/issues/58) 跟踪。
- Windows 实际交接运行只在 CI 的 Windows 套件中执行。
