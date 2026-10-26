# Can

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。

文档：`AUTOSAR_CP_SWS_CANDriver.pdf`（`d04d35b80d8f445b66f960fd9b5be4f02f324e73cc4be239887e8c1235b172aa`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_CANDriver` 全部 368 条（含 CONSTR、NA 与 `ECUC_Can`）逐条处置：适用条目由测试以 `Verifies:` 标注，其余条目的已核对、不适用与差距见下表 | 主机虚拟控制器只有一个，所有事件都轮询；`Can.h` 只声明标准服务，使用 AUTOSAR 类型；驱动 controller ID 与 CanIf controller ID 是两个命名空间，回调只携带后者；Init 后为 STOPPED，STARTED 只能从 STOPPED 进入，STOPPED 可从任何已初始化状态进入（含 STOPPED 自环），SLEEP 是逻辑睡眠，只能从 STOPPED 或 SLEEP 进入、只能经 STOPPED 退出；模式变化由 `Can_MainFunction_Mode`（及 `Can_MainFunction_Wakeup`）通知；STOPPED 与 bus-off 取消待发报文，bus-off 后不自动恢复；`Can_Write` 拷贝 0–8 字节、不阻塞，发送槽占用时返回 `CAN_BUSY`，不解析 CAN FD 标志位；接收与确认分别由 `Can_MainFunction_Read`／`Write` 轮询，已接收帧被覆盖时报告运行时错误 `CAN_E_DATALOST` |

## 未由测试覆盖的条目

下表处置 `SWS_CANDriver` 中没有测试 `Verifies:` 的条目。“配置”类写明控制该义务的 ECUC 参数及主线不提供的取值（未写取值的参数或容器整项不提供），这些配置在生成前被拒绝，每个参数都有带 `Rejects:` 标记的测试。只能在真实控制器上成立的义务是差距，由 #170 交给 R10 目标验收。

| 分组 | 处置与条目 |
| --- | --- |
| 开发错误检测 | 不适用（配置：ECUC_Can_00064=true）：SWS_Can_00026、00089、00091、00174、00198、00199、00200、00205、00206、00209、00210、00216、00217、00219、00362、00363、00408、00505、00512、00513、00514、00517、00518、00519、91005、91006、91007、91011、91012、91016、91017、91018、91019 |
| 版本信息 | 不适用（配置：ECUC_Can_00106=true）：SWS_Can_00224、00177 |
| 中断处理 | 不适用（配置：ECUC_Can_00314=INTERRUPT、ECUC_Can_00317=INTERRUPT／MIXED、ECUC_Can_00318=INTERRUPT／MIXED、ECUC_Can_00319=INTERRUPT）：SWS_Can_00033、00049、00050、00196、00197、00204、00420、00425、00426 |
| 唤醒 | 不适用（配置：ECUC_Can_00330=true、ECUC_Can_00357、ECUC_Can_00359）：SWS_Can_00048、00112、00269、00270、00271、00294、00361、00364、00445、00446、00447 |
| 改变波特率 | 不适用（配置：ECUC_Can_00482=true）：SWS_Can_00062、00255、00256、00260、00422、00500、SWS_CAN_00491、SWS_Can_00492、00493、00494 |
| Partial Networking | 不适用（配置：ECUC_Can_00529=true、ECUC_Can_00530）：SWS_Can_00601、00602、00603、ECUC_Can_00531、00532、00533、00534、00535、00536、00537 |
| 多路发送与硬件 FIFO | 不适用（配置：ECUC_Can_00095=true、ECUC_Can_00467>1）：SWS_Can_00277、00401、00402、00403、00489、00490 |
| CAN FD | 不适用（配置：ECUC_Can_00473）：SWS_Can_00502、ECUC_Can_00475、00476、00477、00478、00479、00481、00494 |
| TriggerTransmit | 不适用（配置：ECUC_Can_00486=true）：SWS_Can_00503、00504、00506 |
| 分区映射 | 不适用（配置：ECUC_Can_00491、ECUC_Can_00492）：SWS_Can_CONSTR_00509、SWS_Can_CONSTR_00510、SWS_Can_CONSTR_00511 |
| 硬件对象负载长度 | 不适用（配置：ECUC_Can_00495）：SWS_Can_CONSTR_00512 |
| 扩展 ID | 不适用（配置：ECUC_Can_00065=EXTENDED／MIXED）：SWS_Can_00423 |
| 多个读写周期 | 不适用（配置：ECUC_Can_00437 多于一个）：SWS_Can_00441、00442 |
| L-PDU callout | 不适用（配置：ECUC_Can_00434）：SWS_Can_00443、00444 |
| 安全事件 | 不适用（配置：ECUC_Can_00496=true）：SWS_Can_91021、91022、91023、91024 |
| 全局时间 | 不适用（配置：ECUC_Can_00498=true）：SWS_CAN_91025、SWS_CAN_91026、SWS_CAN_91027、SWS_CAN_91028、SWS_CAN_91029、SWS_Can_00521–00539 |
| TTCAN | 不适用（配置：ECUC_Can_00001、ECUC_Can_00002）：ECUC_Can_00127–00142、00145–00148、00155、00157、00158、00493 |
| CAN XL | 不适用（配置：ECUC_Can_00499、ECUC_Can_00512、ECUC_Can_00524、ECUC_Can_00526）：ECUC_Can_00500–00511、00513–00523、00525、00527、00528 |
| 硬件过滤 | 不适用（配置：ECUC_Can_00468）：ECUC_Can_00469、00470 |
| 配置选择 | 已核对（配置模型：所选模块只接受 VARIANT-PRE-COMPILE，生成的唯一配置由 `Can_Init` 的 Config 传入）：SWS_Can_00021、00056、00291 |
| 共享寄存器 | 不适用（归属：Mcu）：SWS_Can_00240 |
| 轮询与主函数 | 已核对（源码：所有事件配置为 POLLING，四个轮询主函数都是实际函数而非空定义，彼此不要求执行顺序）：SWS_Can_00007、00099、00280、00110、00178、00180、00183、00185 |
| 接收缓冲 | 已核对（源码：接收帧先拷贝到影子缓冲再交给 CanIf，`Can_MainFunction_Read` 以处理标志防止自身重入，字节顺序即 AUTOSAR 定义）：SWS_Can_00012、00299、00300、00427 |
| 发送 | 已核对（源码：唯一 HTH 由 CanIf 作为参数传入；只有发送槽占用时返回 `CAN_BUSY`，被抢占的调用同样如此；驱动不处理远程帧，也不因远程帧发送）：SWS_Can_00100、00214、00237 |
| 中断禁用计数 | 已核对（源码：`Can_DisableControllerInterrupts` 嵌套计数，未禁用时 `Can_EnableControllerInterrupts` 不动作）：SWS_Can_00202、00208 |
| 模块结构与依赖 | 已核对（源码：驱动只与 CanIf、Det 和 SchM 交互，不使用其他驱动的服务；只有一种控制器类型，沿用不带厂商扩展的标准命名）：SWS_Can_00058、00077、00238、00242、00244、00284、00385、00386 |
| 配置生成 | 已核对（配置模型：工作台按所选控制器生成配置并在生成前校验；硬件相关取值由目标控制器决定）：SWS_Can_00022、00024 |
| 错误检测范围 | 已核对（源码：开发错误随 `CanDevErrorDetect` 关闭，运行时错误 `CAN_E_DATALOST` 由测试验证）：SWS_Can_00104 |
| 类型与可选接口 | 已核对（源码：`Can_GeneralTypes.h` 包含 `ComStack_Types.h` 并承载 Can、CanIf 共用的类型；`Can_ConfigType` 只含主机发送回调；可选接口所需的配置都被拒绝，驱动不调用它们）：SWS_Can_00222、00235、00413、00436 |
| 已接受的引用与枚举 | 已核对（配置模型：硬件对象引用唯一控制器且方向分别为 RECEIVE／TRANSMIT，默认波特率引用该控制器的 CanControllerBaudrateConfig，CanSupportTTCANRef 指向必须为 false 的 CanIfSupportTTCAN）：ECUC_Can_00322、00327、00435、00430 |
| 规范附录 | 已核对（条目只声明所列上游需求不适用于 Can，没有实现义务）：SWS_Can_NA |
| 控制器硬件 | 差距 #170：SWS_Can_00053、00079、00236、00239、00257、00262、00264、00265、00266、00268、00281、00372、00398、00407、00419、00515、00520、ECUC_Can_00005、00073、00074、00075、00113、00313、00382、00383、00471 |

## MemMap 的具体冲突位置

以下为 `9204ed30` 与 `mapping-sequence-final-results/standard-ecu` 对应的技术定位，不是批准表；源码仍未新增 MR12。原件版次、R20.1／R20.5／D4.10 的修订依据和通用替代排除见 [MemMap 篇](memmap.md#misra-预处理布局)。`SWS_BSW_00006`要求所选 Can 实现包含模块映射头，`SWS_MemMap_00003`要求每个实际代码／数据块完整开闭，`00011`要求消费请求。不是每次 include 都违反 R20.1：实际报告按源文件预处理前顺序核定，不能把块首请求一律算作违反。

| R20.1 的实际位置 | 被映射实体与请求 |
| --- | --- |
| `runtime/bsw/src/Can.c`：同一 `#include "Can_MemMap.h"` 的第2–24次，冻结行102、106、113、117、137、146、687、693、736、739、757、762、767、770、778、783、786、789、792、797、806、809、813 | 三组 cleared 对象分别是公共驱动／发送槽状态、仅 queued 配置的三个生命周期 token、接收／模式状态；代码块包括共用驱动函数及按 queued、Rx、Tx、bus-off 配置选定的完整私有动作定义。条件指令在块外，不能把后续 START 移到文件顶而仍映射相同实体。第一处 include 在本文件对象声明之前，不列为 R20.1 违反。 |
| `runtime/bsw/include/Can.h`：第2次，冻结行140 | 关闭标准 Can 服务原型的 CODE 请求；第一处在声明前，不列为违反。 |
| `runtime/host/include/Can_Host.h`：第1／2次，冻结行14／95 | 先有本头自身类型声明，再围住真实主机转发接口原型；不能凭 host 目录认定全文件为 BSW，也不能从 Can 源文件的首 include 情况推断这两处。这里的接口属于已选择驱动的主机接缝。 |
| `runtime/host/src/Can_Host.c`：第2次，冻结行31 | 关闭实际调用 Can_Internal* 的转发函数定义。它们是驱动实现接缝，非测试执行代码；首 include 在本文件代码前。 |
| 生成的 `generated/include/SchM_Can.h`：第2次，冻结行66 | 关闭五个 `Can_MainFunction_Read/Write/Wakeup/BusOff/Mode` 原型的 `CAN_CODE` 请求；对应驱动的周期处理接口，不是另由 `RTE_Rte_CODE` 映射的 CAN_STATE 进入／退出声明。此生成位置不冒充某个 Rust 函数的违反，也不由 runtime 文件关联检查自动覆盖。 |

上述共28个物理位置，均归属两个实际程序；Can.c 中新增的20处是让配置选择围住完整段而产生的真实 R20.1，不按格式变化或 checker 误报销项。四 profile 的同类文件字节相同，不能据此声称其他目标或任意配置已经运行。

`runtime/bsw/include/Can_MemMap.h` 的 R20.5 关键词位置是四个真实分支中的 `#undef CAN_START_SEC_CODE`、`#undef CAN_STOP_SEC_CODE`、`#undef CAN_START_SEC_VAR_CLEARED_UNSPECIFIED`、`#undef CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED`。这些与上表实际 CODE／VAR 请求相连；内部 OPEN 状态及属性撤销另属实现策略的 Advisory 判断，不套用00011的直接义务。

同一头文件的 D4.10 对象是其首个 `@file` 所指的重入分派主体，而不是 Can_Init 等某个函数。完整 once guard 会在第一请求后吞掉 STOP 及后续重新开启；仅保护版本头不能保护分派主体。非法／嵌套／错配检查和 STOP 后请求／属性消失的真实 C／C++ 编译观察控制误配风险；这些控制不把违反改成满足。已有累计复核核对请求机制及 Can 的状态／持区顺序，具体记录、原授权、逐处复核范围与就近注释的最终一一对应仍待完成，不能用本节代替 SWS_BSW_00115。

## 已知差距

- **Can published information**：项目尚无已分配的 AUTOSAR vendor ID，也无经核对的 R24-11 数字发布宏映射；`CanVersionInfoApi` 只接受 false，`Can.h` 不声明 `Can_GetVersionInfo`。取得正式身份后才能实现 GetVersionInfo 与外部版本交叉检查（#59，见 [runtime/README.md](../../runtime/README.md)）。
- **控制器硬件**：寄存器初始化、硬件睡眠、有界等待与超时、错误计数器和位时序只能在真实控制器上验证，由 #170 交给 R10。
