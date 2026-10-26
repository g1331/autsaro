# CanIf

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_CANInterface.pdf`（`84bf624c9e497d765653c8df4d62a6e3835efb64f76c742d727b177a64c10f53`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_CANInterface` 全部 493 条（374 条 `SWS_CANIF`，含 CONSTR 与 NA；119 条 `ECUC_CanIf`）逐条处置：适用条目由测试以 `Verifies:` 标注，其余条目的已核对、不适用与差距见下表；`SWS_Comtype_00007`（句柄连续） | Init 取 `CanIf_ConfigType*`，无效配置保持未初始化，未初始化时服务返回 `E_NOT_OK`、回调不处理；`CanIf.h` 只含标准声明；Transmit 服务 ID 0x49（RxIndication 0x14），不同 PduId 可重入、同 PduId 不可重入；Init 后 PDU mode 为 OFFLINE，STOPPED／BusOff→TX_OFFLINE，SLEEP→OFFLINE，SetPduMode 须在 STARTED；只有 OFFLINE 的 Tx 报告 `CANIF_E_STOPPED`，长度拒绝报告 62／90，短 DLC 报告 61；Rx 按 controller、HRH 与 CAN ID（`STANDARD_NO_FD_CAN`）路由；每个被接受的发送至多一次上层确认，PDU 模式抑制的确认结束请求；Rx／Tx 句柄按方向从 0 连续；controller ID 从 0 起；BusOff／模式通知按 dispatch 送往 host CDD |

## 未由测试覆盖的条目

下表处置 `SWS_CANInterface` 中没有测试 `Verifies:` 的条目。“配置”类写明控制该义务的 ECUC 参数及主线不提供的取值（未写取值的参数整项不提供），这些配置在生成前被拒绝，每个参数都有带 `Rejects:` 标记的测试。同一通道混用标准与扩展 ID 是差距，由 #117 承接。

| 分组 | 处置与条目 |
| --- | --- |
| 开发错误检测 | 不适用（配置：ECUC_CanIf_00614=true）：SWS_CANIF_91006、00311、00774、00313、00656、00898、00899、00319、00320、00341、00860、00346、00657、00907、00908、00909、00910、00410、00416、00417、00419、00429、00700 |
| 发送缓冲 | 不适用（配置：ECUC_CanIf_00618=true、ECUC_CanIf_00834>0）：SWS_CANIF_00063、00849、00381、00895、00881、00835、00836、00068、00837、00386、00668、00070、00183、00387、00033、00485 |
| 动态 L-PDU 与 metadata | 不适用（配置：ECUC_CanIf_00593=DYNAMIC、ECUC_CanIf_00610=true、ECUC_CanIf_00824）：SWS_CANIF_00844、00188、00673、00855、00856、00854、00857、00847、00848、00189、00944、00352、00353、00355、00357、00851、ECUC_CanIf_00824 |
| CAN 收发器 | 不适用（配置：ECUC_CanIf_00273、ECUC_CanIf_00686、ECUC_CanIf_00790、ECUC_CanIf_00792）：SWS_CANIF_00655、00712、00287、00358、00538、00648、00362、00288、00363、00364、00650、00367、00289、00368、00537、00649、00371、00290、00372、00535、00536、00373、00760、00766、00769、00771、00761、00765、00770、00813、00762、00757、00805、00806、00808、00763、00759、00809、00810、00812、00764、00706、00708、00710、00788、00794、00795、00796、00797、00798、00814、00800、00801、00802、00803、00804、00693、00694、00695、00696、00697、00698、ECUC_CanIf_00273、00587、00654、00606、00605、00685、00686、00789、00790、00791、00792 |
| 唤醒 | 不适用（配置：ECUC_CanIf_00843=true、ECUC_CanIf_00637=true、ECUC_CanIf_00611=true、ECUC_CanIf_00741、ECUC_CanIf_00549）：SWS_CANIF_00395、00720、00678、00286、00179、00756、00219、00398、00178、00404、00408、00532、00659、00456、00563、00564、ECUC_CanIf_00531、00549、00741 |
| Partial Networking | 不适用（配置：ECUC_CanIf_00772=true、ECUC_CanIf_00866=true、ECUC_CanIf_00865=true、ECUC_CanIf_00773、ECUC_CanIf_00820、ECUC_CanIf_00868）：SWS_CANIF_00747、00748、00863、00749、00750、00751、00896、00752、00878、00815、00753、00816、00817、00754、91016、00963、00964、00965、00966、00821、00823、00824、00825、00826、00827、91017、00958、00959、00960、00961、00962、ECUC_CanIf_00773、00819、00820、00867、00868 |
| CAN XL | 不适用（配置：ECUC_CanIf_00855、ECUC_CanIf_00860）：SWS_CANIF_00937、00938、00939、00957、00940、00941、00942、00943、91015、00945、00946、00947、00948、00949、00950、00951、00952、00953、00954、00955、00956、SWS_CANIF_CONSTR_00001–00014、ECUC_CanIf_00855、00860、00856、00857、00858、00859 |
| 安全事件 | 不适用（配置：ECUC_CanIf_00848=true、ECUC_CanIf_00849）：SWS_CANIF_91010、00913、00915、92000、00916、92001、00917、92002、00918、92003、91008、00919、91009、00920、00921、ECUC_CanIf_00849、00850、00851、00852、00853 |
| Bus Mirroring | 不适用（配置：ECUC_CanIf_00847=true）：SWS_CANIF_00903、00904、00905、00906、91005、00911、00912 |
| 时间同步与时间戳 | 不适用（配置：ECUC_CanIf_00854=true）：SWS_CANIF_91014、00922、00923、00924、00925、91011、00926、00927、00928、91012、00929、00930、00931、00932、91013、00933、00934、00935、00936、00967 |
| 读取接收数据与通知状态 | 不适用（配置：ECUC_CanIf_00607=true、ECUC_CanIf_00608=true、ECUC_CanIf_00609=true、ECUC_CanIf_00600=true、ECUC_CanIf_00595=true、ECUC_CanIf_00589=true）：SWS_CANIF_00198、00199、00472、00473、00201、00194、00324、00325、00326、00329、00330、00202、00393、00331、00335、00230、00394、00336、00340、00297、00391、00392 |
| 版本信息 | 不适用（配置：ECUC_CanIf_00613=true）：SWS_CANIF_00158 |
| 改变波特率 | 不适用（配置：ECUC_CanIf_00838）：SWS_CANIF_00867、00868、00869、00871、ECUC_CanIf_00838 |
| Tx 确认轮询 | 不适用（配置：ECUC_CanIf_00733=true）：SWS_CANIF_00734、00736、00738、00740 |
| TriggerTransmit | 不适用（配置：ECUC_CanIf_00844=true、ECUC_CanIf_00840）：SWS_CANIF_00883、00885、00882、ECUC_CanIf_00840 |
| TX_OFFLINE_ACTIVE | 不适用（配置：ECUC_CanIf_00837=true）：SWS_CANIF_00072 |
| Tx 截断 | 不适用（配置：ECUC_CanIf_00845=true）：SWS_CANIF_00894 |
| 关闭接收长度检查 | 不适用（配置：ECUC_CanIf_00617=false、ECUC_CanIf_00846=false）：SWS_CANIF_00830 |
| link-time 配置 | 不适用（配置：ECUC_CanIf_00244 声明 VARIANT-LINK-TIME）：SWS_CANIF_00378 |
| 多个 HRH／HTH | 不适用（配置：ECUC_CanIf_00259、ECUC_CanIf_00258 各多于一个）：SWS_CANIF_00664、00667 |
| 范围接收与 ID 掩码 | 不适用（配置：ECUC_CanIf_00628、ECUC_CanIf_00743、ECUC_CanIf_00822、ECUC_CanIf_00823）：SWS_CANIF_00645、00646、00852、ECUC_CanIf_00628、00825、00826、00629、00644、00630、00743、00744、00745、00822、00823 |
| post-build loadable 容量与 CDD 头文件 | 不适用（配置：ECUC_CanIf_00828、ECUC_CanIf_00829、ECUC_CanIf_00830、ECUC_CanIf_00671）：ECUC_CanIf_00828、00829、00830、00671 |
| CanSM | 不适用（配置：ECUC_CanIf_00547=CAN_SM、ECUC_CanIf_00684=CAN_SM）：SWS_CANIF_00559、00691 |
| 可选下层接口表 | 不适用（配置：ECUC_CanIf_00843=true、ECUC_CanIf_00838、ECUC_CanIf_00772=true、ECUC_CanIf_00273、ECUC_CanIf_00855、ECUC_CanIf_00614=true、ECUC_CanIf_00844=true、ECUC_CanIf_00847=true、ECUC_CanIf_00848=true、ECUC_CanIf_00547=CAN_SM）：SWS_CANIF_00294 |
| 配置集名 | 已核对（主线只生成一个配置集：`CanIf_PBcfg.c` 中的 `CanIf_Config` 传给 `CanIf_Init`，集名只作标识）：ECUC_CanIf_00623 |
| 规范附录 | 已核对（条目只声明所列上游需求不适用于 CanIf，没有实现义务）：SWS_CANIF_NA_00999 |
| 同一通道混用标准与扩展 ID | 差距 #117：SWS_CANIF_00281 |

## MemMap 的具体责任

以下定位绑定 `58fb105a` 源码与四份 `direct-render-mapping-final-samples`，不是批准表。固定 MISRA 主文 R20.1（Advisory，印刷155／PDF163）、R20.5（Advisory，158／166）及 D4.10（Required，32–33／40–41）的修订与通用替代判断见 [MemMap 篇](memmap.md#misra-预处理布局)。位置按预处理前物理源码判断，不将所属两个程序或四 profile 重复计数。

| 位置、请求与所属 class | 实体及必要性 |
| --- | --- |
| `runtime/bsw/src/CanIf.c`，同一 `#include "CanIf_MemMap.h"` 第2／3／4次，行57／65／531 | 第2次关闭配置指针、控制器／PDU模式、四个Tx ledger及三代次计数的 cleared 段；随后完整 CODE 对包围五个私有 helper 和十四个公开服务／回调定义。第一处在这些对象之前，不列为 R20.1。配置与ledger均由 CANIF_STATE 保护，不能把开始／关闭请求搬到文件顶而仍映射相同实体。 |
| `runtime/bsw/include/CanIf.h`，同一指令第1／2次，行81／235 | 已有配置与路由类型之后，十四个公开函数原型使用 `CANIF_CODE`；两处都在本头声明之后。00003要求声明与定义分别完整映射，定义已有段不能替代原型侧请求。 |
| `generated/include/CanIf_Cfg.h`，`#include "Rte_MemMap.h"` 第2次，四 profile 均为行15 | 关闭 `CanIf_Config` 的 extern 配置声明，class 为 `RTE_CanIf_CONFIG_DATA_PREBUILD_UNSPECIFIED`。第一处在本文件对象声明前；不把所包含其他头的类型当作此处前置声明。 |
| `generated/src/CanIf_PBcfg.c`，同一指令第2次，前三 profile 行34、no-diagnostic 行32 | 关闭同类 `can_rx`、`can_tx` 及 `CanIf_Config` 三个连续对象。相邻配对已经合并，不能再沿用旧工程中逐对象重复开闭的五个报告位置。Ecu bootstrap通过 `CanIf_Init(&CanIf_Config)`保存配置，路由数组和实际 BusSM通知地址在该对象整个使用期内有效。 |

上述是七个 CanIf 代码／数据／配置位置的技术判断，未把其他模块文件名中带 CanIf 的报告算入本模块。`SchM_CanIf.h:30`关闭 Enter/Exit 的 `RTE_Rte_CODE` 原型；其真实定义与其他模块的 Enter/Exit 共处 `SchM_CanIf_Host.c`，该文件的 Rte 映射属于共享 SchM 实现，沿 RTE 责任核对，不能冒充 CanIf 实现 CODE。配置声明／定义及 SchM 原型均来自实际生成来源，未假称 runtime 文件路径是其交付原件身份。

`CanIf_MemMap.h`由 `outputs::template_of` 预留给 MEMMAP producer，再经 `emit_verbatim` 输出至 `generated/include`；四份实际输出均与保留模板字节相同，但 `originalBswSources` 不把该 emitter 输出列为直接交付原件。R20.5的模块关键词为模板第7／14／21／28行四个 START／STOP撤销，直接连接 `SWS_MemMap_00011`；第18／19／32／33行的活动状态和属性撤销属于实现控制，不能把内部宏名说成SWS强制。

D4.10对应这个映射头的首个 `@file` 与上述四类实际请求，不对应某个服务函数。一次性guard会吞掉后续STOP及下一段；移到函数体违反00023，靠默认段又不满足00003。现有真实头部段继承、C/C++请求拒绝、STOP后清理、外部版本边及封存ECU协议消费者分别提供风险控制证据，不由符号存在或绿色CI推出整体符合。新增位置尚须连到后续累计复核及最终BSW00115注释；此节未新增Approved／MR12，旧raw状态、严格拒绝和221项人工责任保持。
