# BswM

本篇属于[标准与规范基线](../README.md)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_BSWModeManager` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR 符合或认证。

原文为 `AUTOSAR_CP_SWS_BSWModeManager.pdf`，SHA-256：`a6ce38abd020c3aef91bc90ebc35243b888d05b899c36eba33a2305bfbe08ab3`；编号全集取自固定 SpecificationsARXML，而非 PDF 编号搜索。BSW 通用义务另按 [BswM 模块](../bsw-general/bswm.md) 核算。

## 已选配置与行为

主线只启用 ComM 输入：一个 `BSWM_IMMEDIATE` 的 `BswMModeRequestPort`，来源为引用 ComM 通道的 `BswMComMIndication`，初值为 `COMM_NO_COMMUNICATION`。三条规则分别以 `BSWM_EQUALS` 比较 NO、SILENT、FULL，各自的 true 动作列表以 `BSWM_CONDITION` 执行一个 `BswMUserCallout`，调用文本固定为 `Ecu_HostBusSM_ApplyMode(<通道>u, COMM_<模式>_COMMUNICATION)`。其他模块输入、DEFERRED 处理、事件与定时器、TRIGGER、false 列表、嵌套列表、其他动作、分区与 Det 都在生成前拒绝。

`BswM_ComM_CurrentMode` 在调用者上下文仲裁，只评估使用该输入的规则。BswM 按 [ADR 0018](../../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md) 使用自己的 `BSWM_STATE`，动作列表在区外运行：动作期间到达的输入不递归执行，而是在当前动作之后各仲裁一次，仲裁前取最新指示值（`SWS_BswM_00069`、`00281`）。`BswM_Deinit` 之后的输入不再处理，正在运行的动作照常结束；`BswM_MainFunction` 没有 DEFERRED 输入可处理。

## 处置与证据

- [仲裁与动作](arbitration.md)：IMMEDIATE 仲裁、推迟输入、规则与动作列表、未选的仲裁特性。
- [公开接口与类型](interfaces.md)：初始化、反初始化、ComM 输入、主函数、类型与未选模块接口。
- [ECUC 配置与选择边界](configuration.md)：接纳的容器与参数、模块开关及生成前拒绝。

独立 C 消费者通过标准公开头验证规则与动作；真实线程握手在 BswM 退出 `BSWM_STATE` 后暂停，验证推迟输入的次序、反初始化与重启。配置测试要求集成计划与普通校验在源对象位置拒绝。

## MISRA 偏离

| 规则 | 范围 | 依据与风险控制 | 批准 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7（advisory） | `runtime/bsw/src/BswM.c` 中 `BswM_MainFunction` 的定义 | `SWS_BswM_00053` 规定该主函数以外部链接提供；所选配置全部输入为 IMMEDIATE，函数体为空，生成的调度不调用它，Cppcheck 因此报告只在一个翻译单元中引用。偏离只影响符号可见性，`mode_contract.c` 断言调用它不重复任何动作；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-15（#45） |
| MISRA C:2012 Rule 2.3（advisory） | `runtime/bsw/include/BswM.h` 中的 `BswM_ModeType` 与 `BswM_UserType` | `SWS_BswM_00214`、`00216` 规定这两个类型经 `BswM.h` 提供；所选配置不提供使用它们的通用请求接口，因此没有声明引用它们。它们只是公开类型，尺寸由 `mode_contract.c` 断言，不影响行为；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-15（#45） |

## 剩余差距

版本检查由 #59、MISRA 人工评估由 #58 跟踪。
