# LSduR

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。

文档：`AUTOSAR_CP_SWS_LSduRouter.pdf`（`2cd776bc9d54dd8d7f12fa300f24bbf4b790560f9fe7cc9702d20c76b24fdd3a`，R24-11 DRAFT）。除 `CP_SWS_LSduR_91015／91016／91017` 外全部条目为 draft；按固定版本原文与 valid 条目同等处置，不宣称为成熟条款（[README §1](README.md#1-固定基线)）。

## 所选配置

主线不接受 LSduR 的 ECUC 模块（`ECUC_LSduR_00001`，生成前以 `MODULE_UNSUPPORTED` 拒绝）。LSduR 的路由由同一计划从 PduR、CanIf 与 CanTp 的配置推导，生成到 `LSduR_PBcfg.c`：

- 上层为 PduR 与 CanTp，下层为 CanIf；
- 路由一对一、同一分区，不带 metadata；
- 不做开发错误检测，不提供版本信息 API；
- 不提供 ReleaseRxBuffer 与 CancelTransmit，也不支持 zero cost operation；
- 只有预编译配置，没有 post-build 更新。

LSduR 只在自己的 `LSDUR_STATE` 区内读取已发布的配置指针，离开该区后再查表和调用 CanIf 或上层（[ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md)）。`LSduR_Init` 只在 `LSDUR_UNINIT` 时接受有效配置，状态机没有 `LSDUR_ONLINE` 到 `LSDUR_ONLINE` 的转换。

## 条款处置

由测试验证的条目见 `core/tests/fixtures/bsw-contracts/routing_contract.c`、`routing_parallel.c` 与 `com_chain_contract.c` 中的 `Verifies:`。下表处置其余条目。“配置”类的依据是 LSduR 模块整体不被接受，`Rejects: ECUC_LSduR_00001` 的测试证明这一点。

| 分组 | 处置与条目 |
| --- | --- |
| 头文件与依赖 | 已核对（源码：`LSduR_CanIf.h`、`LSduR_PduR.h`、`LSduR_CanTp.h` 按调用方分开声明）：CP_SWS_LSduR_00001；已核对（源码：`LSduR.c` 包含 `Det.h`）：CP_SWS_LSduR_00002；已核对（源码：`LSduR.h` 经 `ComStack_Types.h` 取得导入类型）：CP_SWS_LSduR_91017；已核对（源码：`Det_ReportRuntimeError` 经 `Det.h` 可用，所选配置没有需要报告的运行时错误）：CP_SWS_LSduR_91015；已核对（源码：`CanIf_Transmit` 直接调用，上层回调来自生成的路由表）：CP_SWS_LSduR_91016 |
| 状态与配置类型 | 已核对（源码：单一静态配置指针即唯一状态机实例）：CP_SWS_LSduR_00026；已核对（源码：`LSduR_StateType` 的两个取值）：CP_SWS_LSduR_91005；已核对（生成样本：生成器把 `LSduR_Config` 写入 `LSduR_PBcfg.c`）：CP_SWS_LSduR_00035 |
| 分区约束 | 已核对（配置模型：所选配置只有一个分区，所有路由都是分区内路由）：SWS_LSduR_CONSTR_00001 |
| 规范标记的不适用 SRS | 已核对（规范原文：规范自身列出的不适用 SRS，没有实现义务）：SWS_LSduR_NA |
| 开发错误与运行时错误 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00004、00032、00034、91001、91002 |
| 一对多、多对一与组播 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00010、00040、00041、00042、00043 |
| ReleaseRxBuffer 与 CancelTransmit | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00013、00014、91012、91020 |
| metadata | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00009 |
| zero cost operation | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00024、00025 |
| CDD 接口 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00033 |
| 多分区 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00044、00045、00046、00047、00048、00049、00050、00051、00052、00053、00054、00055、00056 |
| post-build 与配置标识 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_00037、00038、00039 |
| 版本信息 | 不适用（配置：ECUC_LSduR_00001）：CP_SWS_LSduR_91007 |
| ECUC 定义 | 不适用（配置：ECUC_LSduR_00001）：ECUC_LSduR_00001–00024 |
| 头文件版本号 | 差距 #59：CP_SWS_LSduR_00003 |

## 已知差距

- 头文件不含软件与规范版本号（#59）。
- 生成 C 的 MISRA 诊断裁决与人工评估未完成（#58）。
