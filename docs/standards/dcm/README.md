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

ISO 14229／15765 被引用条款的完整独立原文核对保留在 #57，相关条目显式标记为差距。MISRA 授权正文与 221 项人工评估分别由 #57／#58 跟踪；静态分析的执行完整性、原始源码诊断与人工评估分别报告。版本检查由 #59、完整 BSWMD／模块交付文档由 #159、symbolic name 导入由 #160、`DCMServices` 服务端口由 #187 跟踪。
