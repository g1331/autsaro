# RTE 与生成契约

本篇属于[标准与规范基线](../README.md)，按[ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md)处置固定 R24-11 `SWS_RTE` 的条目。原文为 `AUTOSAR_CP_SWS_RTE.pdf`（SHA-256 `d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）与固定 SpecificationsARXML；编号目录包含 2884 条，含 draft／obsolete 状态。

主线选择单核、单 owner、源码 SWC 每类型单实例、显式非队列 uint32 S/R、同步直接 C/S、固定四字节 OUT 数组、TimingEvent 与预编译配置。契约阶段与 ECU 阶段消费同一个已验证 `CompositionContract`；普通生成不需要编译器或官方档案。原文的工具能力要求与某次配置的运行义务分开：未支持的其他机制保留在 [#212](https://github.com/g1331/autsaro/issues/212)，不把拒绝某个可选配置等同于实现该机制。

- [接口、状态与输入](contracts.md)：当前主线的接口、类型、初始化与生成阶段。
- [其他通信机制](communication.md)、[其他生成能力](generator.md)、[其他执行与配置](execution.md)：非主线能力与进入条件。
- [测量、标定与生成开关](optional-generation.md)、[Implementation Plug-In](plugins.md)：按原文条件区分已关闭配置和未提供的工具能力。
- [SchM](../schm.md)：BSW Scheduler 生命周期和独占区。
- [BSW General：Rte](../bsw-general/rte.md)：按 Rte 单独核算通用义务。

运行 `python -m autosar_tooling requirements SWS_RTE` 现算处置；结果不是符合性证书。源码分析的真实诊断与 MISRA 人工评估由 #58 保留，不能用条款核算或 CI 绿色替代。

## 单实例句柄的实际用途

R2.8为Advisory、System（AMD4.3，印刷7／PDF14）：对象定义及其声明可删除而程序仍能编译时，属于未使用对象。当前四份实际ECU源码中，standard-ecu／user-application各两个、component-pipeline四个、no-diagnostic三个Rte_Inst_*指针对象只出现于生成头的extern声明和Rte.c定义，没有C消费者；保留其真实未使用状态。被这些指针初始化引用的私有Rte_Instance_* CDS不是同一个对象，不能仅因句柄无人读取便说CDS也无引用。

固定SWS_RTE的SWS_Rte_03711要求每个SWC实例的CDS，03712规定私有Rte_Instance_*前缀，03715要求const，03793规定单实例句柄的Rte_Inst_<cts>名字；它们支持保留公开句柄合同，但不能据此声称当前独立const指针对象是唯一合法表示。当前交付ABI保留这些对象，理由是组件头的一致句柄入口及既有集成合同；不为制造用途增加函数、宏或无效读取，也不把保留理由写成R2.8不适用、偏离批准或全部对象评估完成。其他对象及未选配置仍按实际引用、存储期和工具消费者分别判断。
