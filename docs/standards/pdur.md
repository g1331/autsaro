# PduR

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。

文档：`AUTOSAR_CP_SWS_PDURouter.pdf`（`3f97e95f6b36c876daeddc1bd90db7d0b24ebd0aa652078349d323ad6664ad6f`）。draft 条目（`SWS_PduR_00911–00917`、`91002`、`SWS_PduR_CONSTR_00920／00931`、`ECUC_PduR_00368`）与 valid 条目同等处置，不宣称为成熟条款（[README §1](README.md#1-固定基线)）。

## 所选配置

主线只接受 `VARIANT-PRE-COMPILE` 的 PduR 配置。`PduRGeneral` 的四个开关均为 `false`，`PduRRoutingPaths.PduRConfigurationId` 必填并生成到 `PduR_Config` 的标识。每个 EcuC Pdu 恰有一个 `PduRSrcPdu`、一个 `PduRDestPdu` 和一条路由，源与目的引用同一个 Pdu，并要求转发发送确认：

- 上层为 Com（IF）与 Dcm（TP），下层为 LSduR（转往 CanIf）与 CanTp；
- 路由一对一、同一分区，不带 metadata；
- 没有网关、缓冲、FIFO、默认值、路由组、Dem 事件、CDD 或 `PduRBswModules`；
- 不做开发错误检测，不提供版本信息 API，不支持 zero cost operation。

Com 的发送路由下层为 LSduR，句柄是 `PduRDestPduHandleId`；Dcm 的发送路由下层为 CanTp，句柄是 CanTp 的 N-SDU。两类句柄分属不同命名空间，`PduR_TxRouteType.lower_module` 区分它们，IF 回调只匹配 LSduR 路由。

PduR 只在自己的 `PDUR_STATE` 区内读取已发布的配置指针，离开该区后再查表和调用其他模块（[ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md)）。`PduR_Init` 只在 `PDUR_UNINIT` 时接受有效配置，状态机没有 `PDUR_ONLINE` 到 `PDUR_ONLINE` 的转换。

## 条款处置

由测试验证的条目见 `core/tests/fixtures/bsw-contracts/routing_contract.c`、`routing_parallel.c` 与 `core/tests/it/integration/single_component.rs` 中的 `Verifies:`。下表处置其余条目。“配置”类写明控制该义务的 ECUC 参数及主线不提供的取值（未写取值的参数整项不提供），这些配置在生成前被拒绝，每个参数都有带 `Rejects:` 标记的测试。网关指目的 Pdu 不同于源 Pdu 的路由，一对多与多对一指同一 Pdu 有多条路由或多个目的。

| 分组 | 处置与条目 |
| --- | --- |
| 头文件与依赖 | 已核对（源码：`PduR_Com.h`、`PduR_Dcm.h`、`PduR_CanIf.h`、`PduR_CanTp.h` 按调用方分开声明）：SWS_PduR_00216；已核对（源码：`PduR.c` 包含 `Det.h`）：SWS_PduR_00802；已核对（源码：`PduR.h` 经 `ComStack_Types.h` 取得导入类型）：SWS_PduR_00333；已核对（源码：`Det_ReportRuntimeError` 经 `Det.h` 可用，所选配置没有需要报告的运行时错误）：SWS_PduR_91001；已核对（源码：所用的下层与上层接口按标准签名调用，取消、释放缓冲等可选接口不在所选配置中）：SWS_PduR_00424 |
| 状态与配置类型 | 已核对（源码：单一静态配置指针即唯一状态机实例）：SWS_PduR_00644；已核对（源码：`PduR_StateType` 的两个取值，配置指针为空即 `PDUR_UNINIT`）：SWS_PduR_00324、SWS_PduR_00742 |
| 接口命名与生成 | 已核对（源码：CanIf 只提供 IF、CanTp 只提供 TP，接口名不需要 `If`／`Tp` 中缀）：SWS_PduR_00800；已核对（生成样本：网关与 FIFO 在所选配置中不存在，代码中没有对应实现；TP 与 IF 接口由主线固定交付的 CanTp 与 CanIf 链路调用，生成器只输出实际配置的路由行）：SWS_PduR_00764 |
| 同一 Pdu 的路由 | 已核对（配置模型：源与目的引用同一个 Pdu，MetaDataType 必然相同）：SWS_PduR_00828；已核对（配置模型：同一 Pdu 的 `KeepLocalBuffer` 等设置必然相同）：SWS_PduR_CONSTR_00931；已核对（配置模型：主线只接受 Com、Dcm、CanIf 与 CanTp 的 Pdu，没有 Dds 拥有的 Pdu）：SWS_PduR_CONSTR_00920 |
| 多分区 | 已核对（配置模型：主线只有一个 `EcucPartition`，Pdu 不带分区引用，所有路由都是分区内路由）：SWS_PduR_00836、00837、00838、00839、00840、00841、00843、00844、00845、00846、00847、00848、00849、00850、00851、00852、00853、00854、00855、00856、00857、00858、00859、00860、00881、00882、00883、SWS_PduR_CONSTR_00842、SWS_PduR_CONSTR_00861 |
| 规范标记的不适用 SRS | 已核对（规范原文：规范自身列出的不适用 SRS，没有实现义务）：SWS_PduR_NA_00777 |
| 开发错误 | 不适用（配置：ECUC_PduR_00302=true）：SWS_PduR_00100、00119、00221、00648、00649 |
| 错误报告的实例号 | 不适用（配置：ECUC_PduR_00302=true、ECUC_PduR_00291、ECUC_PduR_00356）：SWS_PduR_00824 |
| 运行时错误 | 不适用（配置：ECUC_PduR_00291、ECUC_PduR_00356）：SWS_PDUR_00816 |
| 生产错误 | 不适用（配置：ECUC_PduR_00365、ECUC_PduR_00336）：SWS_PduR_00921 |
| 版本信息 | 不适用（配置：ECUC_PduR_00316=true）：SWS_PduR_00338 |
| zero cost operation | 不适用（配置：ECUC_PduR_00317=true）：SWS_PduR_00287、00619 |
| post-build | 不适用（配置：ECUC_PduR_00293 声明 VARIANT-POST-BUILD）：SWS_PduR_00295、00296 |
| 缓冲与排队策略 | 不适用（配置：ECUC_PduR_00336、ECUC_PduR_00356、ECUC_PduR_00367）：SWS_PduR_00797、00798、00785、00787、00307、00645、SWS_PduR_CONSTR_00871、SWS_PduR_CONSTR_00872、SWS_PduR_CONSTR_00873 |
| 默认值 | 不适用（配置：ECUC_PduR_00299）：SWS_PduR_00308 |
| 一对多、多对一与组播 | 不适用（配置：ECUC_PduR_00354）：SWS_PduR_00164、00901、00902、00903、00904、00905、00911、00912、00913、00914、00915、00916、00917、00218、00633、00589、00871、00872、00631、00632、00812、00765 |
| 网关 | 不适用（配置：ECUC_PduR_00291）：SWS_PduR_00638、00825、00826、00829、00436、00437、00303、00306、00746、00784、00793、00665、00667、00786、00662、00640、00819、00666、00809、00783、00794、00830、00831、00832、00833、00835、00696、00637、00740、00818、00551、00697、00705、00813、00814、00815、00708、00317、00811、00808、00707、00823、00789、00790、00791、00792、00788、00807、00256、00255、00669、00670、00806、00799、00687、00689、00803、00804 |
| 路由组 | 不适用（配置：ECUC_PduR_00308）：SWS_PduR_00892、00894、00895、00897、00898、00717、00715、00805、00810、00663、00709、00899、00896 |
| 取消发送、取消接收与释放接收缓冲 | 不适用（配置：ECUC_PduR_00295）：SWS_PduR_00710、00721、00723、00722、00724、00700、00701、00726、00736、00727、00732、00769、00767、91002 |
| CDD 接口 | 不适用（配置：ECUC_PduR_00295）：SWS_PduR_00504 |
| 头文件版本号 | 差距 #59：SWS_PduR_00762 |

### ECUC 参数

| 分组 | 处置与条目 |
| --- | --- |
| 模块与路由表 | 已核对（配置模型与生成样本：模块只接受 `VARIANT-PRE-COMPILE`；`PduR_PBcfg.c` 的路由行取自这些容器与参数，源与目的句柄分别是 `PduRSourcePduHandleId` 与 `PduRDestPduHandleId`，引用缺失、重复或不唯一时在生成前拒绝）：ECUC_PduR_00293、00310、00248、00353、00354、00249、00322、00291、00288、00311、00318 |
| 发送确认 | 已核对（配置模型：主线的每条路由都转发发送确认，`PduRSrcPduUpTxConf` 或 `PduRTransmissionConfirmation` 为 `false` 时以 `PDU_ROUTE_CONFIRMATION` 拒绝）：ECUC_PduR_00351、00339 |
| `PduRBswModules` | 不适用（配置：ECUC_PduR_00295）：ECUC_PduR_00295、00340、00297、00298、00360、00362、00307、00368、00332、00358、00359、00364、00361、00363、00357、00312、00313、00314、00338、00294 |
| Dem 事件 | 不适用（配置：ECUC_PduR_00365）：ECUC_PduR_00365、00366 |
| 路由组 | 不适用（配置：ECUC_PduR_00308）：ECUC_PduR_00308、00329、00309、00352 |
| 路由容量 | 不适用（配置：ECUC_PduR_00350、ECUC_PduR_00348）：ECUC_PduR_00350、00348 |
| 缓冲、排队与 TP 阈值 | 不适用（配置：ECUC_PduR_00336、ECUC_PduR_00356、ECUC_PduR_00367、ECUC_PduR_00320、ECUC_PduR_00355）：ECUC_PduR_00336、00324、00304、00356、00367、00320、00355 |
| 数据提供方式 | 不适用（配置：ECUC_PduR_00289）：ECUC_PduR_00289 |
| 默认值 | 不适用（配置：ECUC_PduR_00299）：ECUC_PduR_00299、00300、00290、00292 |

## 已知差距

- 头文件不含软件与规范版本号（#59）。
- 生成 C 的 MISRA 诊断裁决与人工评估未完成（#58）。
