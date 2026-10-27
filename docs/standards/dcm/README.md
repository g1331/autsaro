# Dcm

本篇属于[标准与规范基线](../README.md)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_DiagnosticCommunicationManager` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR／ISO 符合或认证。

原文为 `AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf`，SHA-256：`cf5aeee78fda6e5a25f982f04cb146ce4a75586de7ba41913d46b0cea1cc2407`；编号全集取自固定 SpecificationsARXML，而非 PDF 编号搜索。BSW 通用义务另按 [Dcm 模块](../bsw-general/dcm.md) 核算。

## 已选配置与行为

主线提供一个物理 UDS-on-CAN 连接、默认／扩展会话、DiagnosticSessionControl（0x10）、ReadDataByIdentifier（0x22）与 TesterPresent（0x3E）。服务 0x10 的会话变更在正响应成功确认后发布；抑制正响应时在本地完成。F186 返回当前会话；应用 DID 可省略，存在时通过同步 C/S ReadData 读取四个 opaque 字节。P2 约束等待 FullCom 的时间，P2* 随会话响应广告，S3 控制空闲扩展会话恢复；这里没有异步应用处理或 response-pending 服务。

Rx／Tx 使用 Dcm 自有缓冲区，传输拒绝与错误确认均结束当前请求且不重发。超长、格式错误、未知 DID 与应用／RTE 失败保留各自的错误语义。主线对未选服务返回 NRC 0x11，配置侧拒绝启用它们。#33／#124 已将写 DID、routine、DTC、SecurityAccess 与 Dem／NvM 交接 R7；这里不恢复旧链路。

OBD mirror 为 false，且参考 ECU 的 DemOBDSupport 为 DEM_OBD_NO_OBD_SUPPORT。F4xx／F8xx 中每隔 0x20 的 availability DID 由 Dcm 返回已配置 DID 的位图，不调用应用；这些 DID 和 F186 不得被应用覆盖。普通 F4xx／F8xx 使用 UDS ReadData。F6xx 同样按 `SWS_Dcm_01197` 使用 UDS 接口；`SWS_Dcm_00482` 对 OBD $06 的要求须结合 no-OBD 配置与 `SWS_Dcm_91002` 中 Dem OBD 接口的可用条件阅读。生成前在 DemGeneral 拒绝三种 OBD-relevant ECU 声明；该选择不能依赖模板默认值。此结论只适用于 no-OBD、mirror=false。

## 处置与证据

- [传输、会话与服务](behaviour.md)：接收／发送、通信模式门控、请求身份、S3、DSD／DSP、未选服务。
- [公开接口与数据类型](interfaces.md)：标准签名、消息上下文、同步 ReadData 的返回契约、数组、RTE 模式及可选接口。
- [ECUC 配置与选择边界](configuration.md)：允许的来源、重数、时限、引用及生成前拒绝。

独立 C 消费者通过标准公开头编译／链接并验证服务与失败恢复；线程握手在 Dcm 独占区退出后暂停，验证应用非重入、发送身份与模式发布。实际生成的 SchM 另验证非 owner 上下文中的会话重置。配置测试同时要求集成计划与普通校验在源对象位置拒绝，不用另一个同码诊断代替。

## 剩余差距

### MemMap 的逐处责任

以下绑定 `8a26e3b0` 和四份 `complete-declarations-final-samples`，不是批准表。[MemMap篇](../memmap/README.md#misra-预处理布局)给出R20.1／R20.5／D4.10的原文和修订，以及00003完整声明／定义配对、00011消费关键词、00023禁止函数体内包含的必要性与替代边界。

| 位置及请求 | 实体、class与实际闭包 |
| --- | --- |
| `runtime/bsw/src/Dcm.c` 的 `#include "Dcm_MemMap.h"` 第2／3／4次，行140／148／768 | 关闭配置指针、Rx／Tx缓冲与游标、会话／期限／请求代次及执行／发布状态的cleared段，随后完整包围helper和服务定义的 `DCM_CODE`。DCM_STATE保护数据，应用ReadData及模式发布在区外执行；Init保留仍在途的executor与publisher，避免回调重入或旧模式覆盖新模式。 |
| `runtime/bsw/include/Dcm.h`，同一指令第1／2次，行85／211 | 类型之后的公开服务及TP回调原型，`DCM_CODE`。 |
| `runtime/bsw/include/Dcm_ComM.h`，同一指令第2次，行45 | 关闭NoCom／SilentCom／FullCom三个通知原型，`DCM_CODE`；ComM配置保存其真实地址。第一处在本文件声明前，不因被包含头提供类型而计入R20.1。 |
| `generated/include/SchM_Dcm.h`，同一指令第1／2次，行48／61 | 前有共享独占区声明，故两处均为R20.1；包围Dcm_MainFunction的 `DCM_CODE` 原型。该头Rte_MemMap行39关闭共享Enter/Exit，69／89及97／118包围会话／认证SchM接口的 `RTE_DcmService` 请求与 `RTE_CODE` 属性，分别沿RTE责任判断，不能混入Dcm实现CODE。 |
| `generated/src/Ecu_RuntimeConfig.c` 紧随六个 `RTE_Dcm` 请求的 `#include "Rte_MemMap.h"` | 标准／用户行324／327、329／346、736／742；组件326／329、331／348、750／756。分别包围Ecu_DcmAuthenticationMode私有原型、dcm_config及该callback定义，class依次为CODE、CONFIG_DATA_PREBUILD_UNSPECIFIED、CODE。Bootstrap将配置交给Dcm_Init，认证发布由配置地址到SchM开关；定义与声明的配对不能互相替代。无诊断profile没有这六处配置／callback。 |

四profile实际都交付并编译Dcm.c及上述头，故自身八个R20.1位置不因关闭诊断配置消失；三个诊断profile各另有六个配置／callback位置，共十四个。上述计数不含RTE服务侧段，也不按两个程序重复计数。生成位置来自原配置／callback emitter，没有对生成C作后处理。

`Dcm_MemMap.h`模板第7／14／21／28行撤销模块START／STOP关键词；18／19／32／33行的内部active／属性撤销另属实现清理，不借00011批量授权。MEMMAP producer交付的可重入主体连接D4.10与这些真实请求；一次性guard吞掉STOP，顶部include、默认段或函数体内include不能保留同一00003／00023合同。头部段继承、请求拒绝、版本边及真实诊断／取消／模式交错消费者提供风险控制，不补齐ISO原件或provider证明。最终逐处复核与BSW00115对应仍须完成，本节不新增Approved／MR12，不改变原始false／refusal／not_assessed。

ISO 14229／15765 的所选物理服务、计时、消息和NRC核对见 [behaviour.md](behaviour.md#跨标准原文与适用闭包)，原件收据由 #57 记录。MISRA固定七份正文已取得，221项人工评估仍由 #58 跟踪；静态分析的执行完整性、原始源码诊断与人工评估分别报告。版本检查由 #59、`DCMServices` 服务端口由 #187 跟踪。
