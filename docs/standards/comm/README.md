# ComM

本篇属于[标准与规范基线](../README.md)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_COMManager` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR 符合或认证。

原文为 `AUTOSAR_CP_SWS_COMManager.pdf`，SHA-256：`564560db2899fa0f8695e5fad607f3cc477a4c84abfccfe968e916117cf2ef34`；编号全集取自固定 SpecificationsARXML，而非 PDF 编号搜索。BSW 通用义务另按 [ComM 模块](../bsw-general/comm.md) 核算。

## 已选配置与行为

主线只有一个 ComM 通道：`ComMBusType` 为 `COMM_BUS_TYPE_CDD`，`ComMCDDBusPrefix` 为主机总线状态管理器 `Ecu_HostBusSM`，`ComMNmVariant` 为 NONE；1 至 32 个用户全部映射到该通道。PNC、唤醒抑制、模式限制、同步唤醒、NvM、Det 与版本 API 均不选，对应开关只接受 false，其余值在源对象位置拒绝。

通道按 SWS Figure 7.7 的 NM NONE 状态机运行：用户 FULL 请求、Dcm 活动诊断或 EcuM 唤醒使 NO_PENDING_REQUEST 进入 REQUEST_PENDING，只有该子状态评估 CommunicationAllowed；进入 FULL 后最小 Full 时长到期且没有需求时进入 READY_SLEEP，NM NONE 不从 READY_SLEEP 回到 NO，也不进入 SILENT。进入 NO（含 `ComM_Init`）与 FULL 时向总线状态管理器请求对应模式，被拒绝的请求由主函数重发。总线状态管理器的模式指示不改变通道状态，只把实际模式传播给 BswM 与引用该通道的 Dcm 连接；初始化后的默认 NO 不通知。

ComM 按 [ADR 0018](../../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md) 使用自己的 `COMM_STATE`：总线请求、模式查询与模式发布都在区外，由一个调用者按顺序发出，并发或重入的指示在当前发布之后只发布最新值。`ComM_DeInit` 只在 NO_COM_NO_PENDING_REQUEST 且总线实际为 NO 时生效。

## 处置与证据

- [通道状态机与行为](behaviour.md)：子状态、用户与诊断需求、最小 Full 时长、总线请求与模式传播。
- [公开接口与类型](interfaces.md)：标准签名、初始化前行为、回调、类型与返回值扩展、未选接口。
- [ECUC 配置与选择边界](configuration.md)：接纳的容器与参数、重数、时基及生成前拒绝。

独立 C 消费者通过标准公开头编译／链接并验证状态机、回调与拒绝；真实线程握手在 ComM 退出 `COMM_STATE` 后暂停，验证总线请求与模式发布按序且不在区内调用其他模块。配置测试要求集成计划与普通校验在源对象位置拒绝。

## MISRA 偏离

### MemMap 的逐处责任

以下绑定 `8a26e3b0` 和四份 `complete-declarations-final-samples`，不是批准表；规则修订、00003／00011／00023及替代判断沿[MemMap篇](../memmap.md#misra-预处理布局)。R20.1按预处理前的本文件内容判断，不把包含头内的声明当作本文件前置声明。

| 位置及请求 | 实体、class与实际闭包 |
| --- | --- |
| `runtime/bsw/src/ComM.c` 的 `#include "ComM_MemMap.h"` 第2／3／4次，行76／84／488 | 关闭配置指针、用户请求、通道状态、期限、总线握手和模式发布状态的 cleared 段，随后完整 `COMM_CODE` 对包围校验、状态推进、发布 helper 和服务定义。COMM_STATE保护数据；BusSM请求和通知在区外执行，Init保留仍在途的调用及发布者。 |
| `runtime/bsw/include/ComM.h`，同一指令第1／2次，行73／195 | 类型之后的公开服务原型；声明与定义均使用 `COMM_CODE`。 |
| `ComM_BswM.h`／`ComM_Dcm.h`／`ComM_EcuM.h`，同一指令第2次，行21／29／20 | 分别关闭 CommunicationAllowed、Active／InactiveDiagnostic、WakeUpIndication 原型；均为 `COMM_CODE`。第一处在各自声明前。关闭EcuM消费者不消除已提供接口的声明映射责任。 |
| `runtime/bsw/src/ComM_Internal.h`，同一指令第2次，行13 | 关闭 ComM_RunChannel 的 `COMM_CODE` 私有适配声明；生成周期包装调用它推进真实通道状态。 |
| `generated/include/SchM_ComM.h` 的 `#include "Rte_MemMap.h"` 第3／4次，行33／45 | 配置周期接口的 `RTE_ComM_CODE` 原型；行30关闭共享 `RTE_Rte_CODE` Enter/Exit，另归RTE。 |
| `generated/src/Ecu_RuntimeConfig.c` 紧随四个 `RTE_ComM` 请求的同一指令 | 标准／用户行357／373、389／392；组件359／375、391／394；无诊断298／314、330／333。前对包围已合并的 users／comm_config，后对包围周期包装定义。配置保存CDD BusSM与Dcm通知地址，Bootstrap交给ComM_Init；包装器调用ComM_RunChannel。 |

合计十五个物理 R20.1 位置。映射模板第7／14／21／28行撤销模块关键词，连接00011；18／19／32／33行撤销内部active／属性，另属实现清理。映射头经MEMMAP producer交付，D4.10连接其可重入主体与上述请求；普通声明头仍各自有guard。移到文件顶、一次性guard或默认段不能保持同一完整段与原型继承；函数体内映射违反00023。实际头部段继承、请求拒绝、版本边和模式握手消费者控制误配及调用风险，不替代最终逐处复核、BSW00115注释或MISRA允许条件；本节不新增Approved／MR12。

| 规则 | 范围 | 依据与风险控制 | 批准 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7（advisory） | `runtime/bsw/src/ComM.c` 中 `ComM_DeInit`、`ComM_GetStatus`、`ComM_GetInhibitionStatus`、`ComM_SetECUGroupClassification`、`ComM_GetMaxComMode`、`ComM_GetRequestedComMode`、`ComM_GetCurrentComMode`、`ComM_GetCurrentPNCComMode` 与 `ComM_EcuM_WakeUpIndication` 的定义 | `SWS_ComM_00147`、`00242`、`00619`、`00552`、`00085`、`00079`、`00083`、`91002` 与 `00275` 规定这些 API 经 `ComM.h` 以外部链接提供；所选程序没有 SW-C 端口（#195）和 EcuM，没有模块调用它们，Cppcheck 因此报告只在一个翻译单元中引用。偏离只影响符号可见性：这些函数只在 `COMM_STATE` 区内读写本模块数据，行为由 `mode_contract.c` 验证；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-15（#45） |
| MISRA C:2012 Rule 2.5（advisory） | `runtime/bsw/include/ComM.h` 中的 `COMM_E_MODE_LIMITATION`、`COMM_E_MULTIPLE_PNC_ASSIGNED`、`COMM_E_LEARNING_ACTIVE` 与 `COMM_SILENT_COM`；`runtime/bsw/include/Rte_ComM_Type.h` 中的 `COMM_FULL_COMMUNICATION_WITH_WAKEUP_REQUEST` | `SWS_ComM_91027`、`00674` 与 `00672` 规定这些公开常量；所选配置没有模式抑制、PNC，NM NONE 不进入 SILENT 子状态，也不请求带唤醒的 Full，因此没有代码引用它们。它们只是公开常量，取值由 `mode_contract.c` 断言，不影响行为；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-15（#45） |

## 剩余差距

SW-C 服务端口 `UR_`／`UM_`、`ComM_UserRequest`／`ComM_CurrentMode` 接口与 RTE 模式通知由 #195 跟踪。版本检查由 #59、MISRA 人工评估由 #58 跟踪。
