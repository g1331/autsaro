# 多组件剖面的已核定义务

本篇属于[标准与规范基线](README.md)。

当前主线（多组件剖面，按 [ADR 0010](../adr/0010-single-generation-mainline.md) 将同时承接单 SWC 工程）的首批支持配置如下。范围外的配置在生成前明确拒绝，不静默降级；无关有效内容原样保留。

| 维度 | 支持范围与依据 | 拒绝 |
| --- | --- | --- |
| 组合与实例 | 一个 `ECU_EXTRACT`、一个 ECU、一个平坦 CompositionSwComponentType；多个不同 ApplicationSwComponentType，每种一个实例，`supportsMultipleInstantiation=false`（RTE §4.1.4.2–4.1.4.4、`SWS_Rte_02008／02009`） | 同类型重复实例、嵌套与 delegation |
| 类型 | 显式非排队 uint32 S/R；同步 C/S 的 uint32 IN／OUT／INOUT 与固定 uint8[4] OUT；应用 primitive→implementation→base type 映射逐项核对（`constr_1068／1069`） | 等宽但不同接口／类型、缺映射；结构体、动态数组或转换 |
| 本地 S/R | 每个已使用 R 端点恰有一个本地 P 或一个网络映射，P 可扇出；每个端点要求明确 ComSpec 初值，首次发布前各 R 用自己的初值，冲突时 R 优先（`SWS_Rte_04501／04502`）；`handleNeverReceived=false`、`aliveTimeout=0`、`handleTimeoutType=NONE`、无 invalidValue | 缺初值；本地 `handleNeverReceived=true`（待按 `SWS_Rte_07381–07383` 实现） |
| 网络 S/R | `IMMEDIATE` 信号处理（COM `00300`）；最小真实 Rx I-PDU group＋标准 reception DM＋COM→RTE 通知（`SWS_Com_00772`、`SWS_Rte_08061／08062／08103／08104`）；未分组 Tx PDU 按 `SWS_Com_00840` 隐式启动；`COM_STOPPED` 只由实际 COM 状态映射（`SWS_Rte_06830／07822`），CAN 停止不等于 COM 不可用；`ComSignalInitValue` 为 0，RTE 接收初值独立取自 R ComSpec | `DEFERRED`（COM `00301`）；显式 `ComIPduHandleId`；未选 group 变体、COM TP、动态信号 |
| 同步 C/S | 本地同步调用，非重入服务器；服务器在调用者 owner 上下文执行；无 possibleErrors 时 server entry 返回 void（`SWS_Rte_08913`），客户端 `Rte_Call` 返回 `Std_ReturnType`；参数 policy 仅缺省或 USE-ARGUMENT-TYPE（`SWS_Rte_07027`、`CONSTR1286`） | USE-VOID；调用环、并发、异步、跨任务；空 OUT／INOUT 指针（不修改输出） |
| 调度 | 每个周期 runnable 恰有一个 TimingEvent，全部共用正整数毫秒周期、offset=0，映射到唯一 Extended `Task_Ecu` 的合法 OsEvent 与 Alarm／ExpiryPoint；周期 mapping 显式 `isMappedToTask=true` 并带 task ref；OperationInvokedEvent 的 mapping 为 false 且无 task 引用（`CONSTR08936`／`08938`） | 同 runnable 多 TimingEvent、不同周期或非零 offset、重复位置、task 不属 owner、周期与 alarm 不符 |
| 分区 | 显式唯一 EcucPartition ID0，关联唯一 CoreId0，包含全部 APPLICATION／SERVICE root-context 实例；不启用 OsApplication | 多分区 |
| 诊断路由 | 完整 physical Dcm→PduR→CanTp 路由，可无应用 DID（仍提供内部 F186、0x10／0x3E，未知 DID 返回 NRC 0x31，Dcm `00085`）；两条诊断路由均不选为 network-only | 半条路由或单个端点 |
| CAN | 11 位 Classical CAN（Rx `STANDARD_NO_FD_CAN`）、单控制器单通道、static normal addressing、单 physical DoCAN 连接、`CanIfPublicTxBuffering=false`；controller 与 HOH 编号从 0 连续（`ECUC_Can_00316`、`SWS_CANIF_00653`、`ECUC_Can_00326`）；CanIf Rx／Tx 句柄按方向从 0 连续、各不超过 32（`SWS_Comtype_00007`）；BusOff／模式通知 dispatch 为 `CDD` 的 `Ecu_HostBusSM` | 29 位（同一通道混用标准与扩展 ID 是 CanIf 差距，见 #117）、CAN FD、多通道、metadata、动态 ID；非连续 controller／HOH 编号或 CanIf 句柄；dispatch 选 `CAN_SM` 或其他名称 |
| 名字 | 组件类型与 EcucPartition 短名不得占用已交付 runtime 的内存 scope（Rte、Ecu、Can、CanIf、CanTp、Com、Dcm、PduR、LSduR、ComM、BswM、Ecu_HostBusSM）；C 标识符、文件名、include guard 碰撞拒绝 | `CONTRACT_NAME_COLLISION` 等生成前诊断 |

ECUC metadata 的数值范围通过不等于配置语义合法；非等值 ID（如 controller 7／9）的模块向量只检验命名空间机制，不是合法 source profile 的验收证据。
