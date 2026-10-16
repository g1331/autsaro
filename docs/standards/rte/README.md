# RTE 与生成契约

本篇属于[标准与规范基线](../README.md)，按[ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md)处置固定 R24-11 `SWS_RTE` 的条目。原文为 `AUTOSAR_CP_SWS_RTE.pdf`（SHA-256 `d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）与固定 SpecificationsARXML；编号目录包含 2884 条，含 draft／obsolete 状态。

主线选择单核、单 owner、源码 SWC 每类型单实例、显式非队列 uint32 S/R、同步直接 C/S、固定四字节 OUT 数组、TimingEvent 与预编译配置。契约阶段与 ECU 阶段消费同一个已验证 `CompositionContract`；普通生成不需要编译器或官方档案。原文的工具能力要求与某次配置的运行义务分开：未支持的其他机制保留在 [#212](https://github.com/g1331/autsaro/issues/212)，不把拒绝某个可选配置等同于实现该机制。

- [接口、状态与输入](contracts.md)：当前主线的接口、类型、初始化与生成阶段。
- [其他通信机制](communication.md)、[其他生成能力](generator.md)、[其他执行与配置](execution.md)：非主线能力与进入条件。
- [测量、标定与生成开关](optional-generation.md)、[Implementation Plug-In](plugins.md)：按原文条件区分已关闭配置和未提供的工具能力。
- [SchM](../schm.md)：BSW Scheduler 生命周期和独占区。
- [BSW General：Rte](../bsw-general/rte.md)：按 Rte 单独核算通用义务。

运行 `python -m autosar_tooling requirements SWS_RTE` 现算处置；结果不是符合性证书。源码分析的真实诊断与 MISRA 人工评估由 #58 保留，不能用条款核算或 CI 绿色替代。
