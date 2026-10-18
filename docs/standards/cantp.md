# CanTp

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。

文档：`AUTOSAR_CP_SWS_CANTransportLayer.pdf`（`7334d633b02c443aacbe0ca25e20c319e977104eeb4bf8e67ac0358cfea34a22`）。

## 所选配置

主线只接受 `VARIANT-PRE-COMPILE` 的 CanTp 配置，且只在配置诊断传输时生成。`CanTpGeneral` 的五个 API／报告开关（ChangeParameter、DevErrorDetect、安全事件、ReadParameter、VersionInfo）均为 `false`，`CanTpPaddingByte` 为 0；`CanTpMainFunctionPeriod` 为 1 ms。

- 唯一的 `CanTpChannel` 是一条全双工的 Classical CAN 连接：一个 `CanTpRxNSdu`（诊断请求）与一个 `CanTpTxNSdu`（诊断响应），两者均为标准寻址、物理 TaType、收发都填充。
- 接收侧 `CanTpBs`、`CanTpSTmin`、`CanTpRxWftMax` 均为 0，发送侧 `CanTpTc` 为 `false`；N-SDU 的最大长度取自 Dcm 的 64 字节缓冲。
- 每个方向的数据帧与 FC 共用一个 N-PDU：请求的 `CanTpRxNPdu` 同时是响应的 `CanTpRxFcNPdu`，响应的 `CanTpTxNPdu` 同时是请求的 `CanTpTxFcNPdu`，对应的句柄号相等。N-SDU 与 N-PDU 的句柄号显式配置，须能放入 `PduIdType`。
- 六个计时参数（N_Ar、N_Br、N_Cr、N_As、N_Bs、N_Cs）是至少 2 ms 的整数毫秒，生成时按主函数周期换算为 tick：计时器可能在启动后的下一次主函数就到期，一个周期不足以完成留给主函数的动作。

CanTp 只在自己的 `CANTP_STATE` 区内读写连接状态、计时器与接收帧队列，离开该区后再调用 PduR、LSduR 与 Det（[ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md)）。收发各有一台状态机，共用 Tx N-PDU 的占用记录；收到的帧先排入区内的队列（深度 4），同一时刻只有一个上下文按顺序取出工作并调用其他模块，其余上下文登记后立即返回。连接结束、取消、超时、开始接收与 `CanTp_Init` 都会推进该方向的代次，被它们超越的调用结果在回到区内时按当前真实状态丢弃。最终通知在调用上层前已使连接回到空闲，因此回调中可以立即请求下一个 N-SDU。`CanTp_TxConfirmation` 只结算确认，同一 N-PDU 的下一帧留给下一次主函数发送：主机 Can 驱动在确认回调期间对同一发送槽返回 `CAN_BUSY`。

SWS_CanTp_00229 把超时与协议错误称为开发错误，表 SWS_CanTp_00352 则把它们定义为运行时错误；实现按表格经 `Det_ReportRuntimeError` 报告。

## 条款处置

由测试验证的条目见 `core/tests/fixtures/bsw-contracts/cantp_contract.c`、`cantp_parallel.c` 与 `diagnostic_contract.c` 中的 `Verifies:`；拒绝测试见 `core/tests/it/integration/single_component.rs`。下表处置其余条目。“配置”类写明控制该义务的 ECUC 参数及主线不提供的取值（未写取值的参数整项不提供），这些配置在生成前被拒绝，每个参数都有带 `Rejects:` 标记的测试。

| 分组 | 处置与条目 |
| --- | --- |
| 总括与不适用 SRS | 已核对（规范原文：总括条款，ISO 15765-2 的推荐由本规范各条目展开并在本篇逐条处置）：SWS_CanTp_00033；已核对（规范原文：规范自身列出的不适用 SRS，没有实现义务）：SWS_CanTp_NA_00327 |
| 类型与状态 | 已核对（源码：内部类型均按 `CanTp_<名>Type` 命名，只定义在 `CanTp.c` 中）：SWS_CanTp_00002、00296；已核对（源码：`CanTp_ConfigType` 由 `CanTp.h` 提供，内容由实现定义）：SWS_CanTp_00340；已核对（源码：配置指针是清零的静态变量，`CanTp_Init` 才置为非空，各 API 以它判断是否已初始化）：SWS_CanTp_00161 |
| 中断上下文 | 已核对（源码：`CanTp_RxIndication`／`CanTp_TxConfirmation` 只在区内登记或结算后返回，另一上下文正在调用其他模块时不等待；区由 SchM 按目标实现）：SWS_CanTp_00235、00236 |
| 配置来源与一致性 | 已核对（配置模型：参数取自 ECUC ARXML，生成前由内置规则检查一致性）：SWS_CanTp_00146、00147；已核对（配置模型与生成样本：`CanTpMainFunctionPeriod` 只接受 1 ms，生成为 `main_period_ms`，计时按它换算）：SWS_CanTp_00300 |
| 连接通道 | 已核对（配置模型与生成样本：唯一的 `CanTpChannel` 只含一个 Rx 与一个 Tx N-SDU，通道是 `CanTp.c` 的内部状态，不对外提供，数量由生成器从配置得出）：SWS_CanTp_00121、00190、00285、00286、00289；已核对（配置模型：每个 N-SDU 只属于一个通道且没有 MetaData，同一方向只有一个 N-SDU，不共享通道资源）：SWS_CanTp_00287、00288；不适用（配置：ECUC_CanTp_00137、ECUC_CanTp_00138 各多于一个）：SWS_CanTp_00120、00122 |
| N-SDU 与 N-PDU | 已核对（配置模型：每个 N-SDU 只引用一个发送 N-PDU，数据帧与 FC 共用同一 N-PDU，生成器核对两侧引用与句柄号一致）：SWS_CanTp_00035、00328；已核对（规范原文：FC 的作用说明，FS 的具体义务由 `cantp_contract.c` 验证）：SWS_CanTp_00282 |
| 扩展与混合寻址 | 不适用（配置：ECUC_CanTp_00281、ECUC_CanTp_00262 非 CANTP_STANDARD，ECUC_CanTp_00139、ECUC_CanTp_00284）：SWS_CanTp_00094、00095、00281、00283、00284 |
| 功能寻址 | 不适用（配置：ECUC_CanTp_00250=CANTP_FUNCTIONAL、ECUC_CanTp_00270=CANTP_FUNCTIONAL）：SWS_CanTp_00092、00093 |
| 块大小 | 不适用（配置：ECUC_CanTp_00276 大于 0）：SWS_CanTp_00278、00325 |
| 发送取消 | 不适用（配置：ECUC_CanTp_00282=true）：SWS_CanTp_00243、00255、00256 |
| 参数读写 | 不适用（配置：ECUC_CanTp_00299=true）：SWS_CanTp_00091、00302、00303、00304、00338；不适用（配置：ECUC_CanTp_00300=true）：SWS_CanTp_00323 |
| 开发错误 | 不适用（配置：ECUC_CanTp_00239=true）：SWS_CanTp_00031、00254、00260、00293、00305、00319、00321、00322、00324、00356–00360 |
| 可选接口 | 不适用（配置：ECUC_CanTp_00239=true、ECUC_CanTp_00318=true）：SWS_CanTp_00217 |
| 扩展生产错误 | 不适用（配置：ECUC_CanTp_00307）：SWS_CanTp_00291、00361–00370 |
| 安全事件 | 不适用（配置：ECUC_CanTp_00318=true）：SWS_CanTp_00371–00383 |
| 通用连接（MetaData） | 不适用（配置：ECUC_CanTp_00303）：SWS_CanTp_00330–00337 |
| post-build | 不适用（配置：ECUC_CanTp_00306 声明 VARIANT-POST-BUILD）：SWS_CanTp_00169 |
| 版本信息 | 不适用（配置：ECUC_CanTp_00283=true）：SWS_CanTp_00210；差距 #59：SWS_CanTp_00267 |

### ECUC 参数

| 分组 | 处置与条目 |
| --- | --- |
| 模块与通用参数 | 已核对（配置模型：模块只接受 `VARIANT-PRE-COMPILE`，五个开关只接受 `false`，填充字节只接受 0，主函数周期只接受 1 ms）：ECUC_CanTp_00306、00278、00290、00240、00299、00239、00318、00300、00283、00298 |
| 通道与 N-SDU | 已核对（配置模型与生成样本：唯一通道下各有一个 Rx 与 Tx N-SDU，引用 PduR 一侧的 N-SDU，句柄号写入生成的 `CanTp_ConfigType`；寻址、TaType、填充、BS、STmin、WFTmax 与 Tc 只接受所选取值）：ECUC_CanTp_00288、00137、00138、00241、00261、00301、00268、00281、00262、00250、00270、00249、00269、00276、00252、00251、00282 |
| 计时 | 已核对（配置模型与生成样本：至少 2 ms 的整数毫秒，按主函数周期换算为 tick）：ECUC_CanTp_00277、00245、00279、00263、00264、00265 |
| N-PDU | 已核对（配置模型与生成样本：数据帧与 FC 共用每个方向的 N-PDU，引用与句柄号须一致）：ECUC_CanTp_00256、00257、00258、00274、00275、00286、00259、00260、00287、00271、00272、00273 |
| 未选的通用参数 | 不适用（配置：ECUC_CanTp_00302、ECUC_CanTp_00305、ECUC_CanTp_00303、ECUC_CanTp_00304）：ECUC_CanTp_00302、00305、00303、00304 |
| 生产错误引用 | 不适用（配置：ECUC_CanTp_00307）：ECUC_CanTp_00307–00317 |
| 安全事件引用 | 不适用（配置：ECUC_CanTp_00319）：ECUC_CanTp_00319–00329 |
| 寻址信息 | 不适用（配置：ECUC_CanTp_00139、ECUC_CanTp_00253、ECUC_CanTp_00284）：ECUC_CanTp_00139、00255、00253、00254、00284、00285 |

## MISRA 偏离

| 规则 | 范围 | 依据与风险控制 | 批准 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7（advisory） | `runtime/bsw/src/CanTp.c` 中 `CanTp_CancelTransmit` 与 `CanTp_CancelReceive` 的定义 | `SWS_CanTp_00246／00257` 规定这两个 API 经 `CanTp.h` 以外部链接提供；所选程序没有模块调用它们，Cppcheck 因此报告只在一个翻译单元中引用。偏离只影响符号可见性：两个函数只在 `CANTP_STATE` 区内读写本模块数据，在区外报告 Det 与通知 PduR，行为由 `cantp_contract.c` 验证；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-15（#43） |

## 已知差距

- 头文件不含软件与规范版本号（#59）。
- CanTp_Cfg.h 分别导出数据与 flow-control 的 N-SDU／N-PDU 容器符号；当前相同值约束保留，跨模块句柄由提供方配置头导入。
- 生成 C 的 MISRA 诊断裁决与人工评估未完成（#58）。
