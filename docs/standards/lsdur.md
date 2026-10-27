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

## MemMap 的具体责任

以下定位绑定 `58fb105a` 与四份 `direct-render-mapping-final-samples`；规则类别、修订与不可把内部宏名当SWS强制的限制见[MemMap篇](memmap/README.md#misra-预处理布局)。这是逐处技术记录，未新增批准表或MR12。自有实现／声明沿 `originalBswSources` 对应，四 profile 字节均与源码一致；映射头由MEMMAP producer保留模板后逐字输出到 `generated/include`，不是该映射中的直接原件身份。

| R20.1位置、请求和类别 | 实体与实际闭包 |
| --- | --- |
| `runtime/bsw/src/LSduR.c`，`#include "LSduR_MemMap.h"` 第2／3／4次，行16／18／154 | 第2次关闭唯一配置指针的 `VAR_CLEARED_UNSPECIFIED`；第3／4次为全部验证、配置快照和路由函数的CODE对。指针在LSDUR_STATE内仅首次发布，后续路由持有的是进程期不可变配置；不是借用调用者的临时路由副本。第一处在本文件声明之前，不列为R20.1。 |
| `runtime/bsw/include/LSduR.h`，同一指令第1／2次，行60／81 | 本头路由与配置类型之后的Init／GetConfigurationId原型使用 `LSDUR_CODE`；两处均在类型声明后。 |
| `runtime/bsw/include/LSduR_CanIf.h`／`LSduR_CanTp.h`／`LSduR_PduR.h`，各第2次，行44／20／20 | 关闭实际CanIf接收／确认／触发、CanTp发送和PduR发送原型。三个头首个请求前没有本文件C声明，不能因被包含的LSduR.h有类型就把其首处一并列为违反。 |
| `generated/include/LSduR_Cfg.h`，`#include "Rte_MemMap.h"` 第2次，前三profile行14、no-diagnostic行12 | 关闭 `LSduR_Config` 的extern，类别为 `RTE_LSduR_CONFIG_DATA_PREBUILD_UNSPECIFIED`。 |
| `generated/src/LSduR_PBcfg.c`，同一指令第2次，前三profile行55、no-diagnostic行43 | 同一配置类中的 `ls_rx`、`ls_tx` 和 `LSduR_Config` 已合成连续配对；两张表把实际CanIf句柄连接到PduR／CanTp及确认函数，初始化后不替换。 |

以上八个自有源码／头位置加两个生成配置位置，共十个物理位置，分别进入host-batch／probe，不按program重复计数。必要性连接固定 `SWS_MemMap_00003` 的声明／定义双侧映射和 `SWS_BSW_00006` 的实现包含责任；移除STOP、把全部include提前或依赖默认段均不保留该机制。SchM_LSduR的Enter／Exit属于共享RTE实现，不混入这里的LSDUR_CODE。

`LSduR_MemMap.h`四个关键词撤销位于模板行7／14／21／28，分别消费cleared与CODE的START／STOP；00011直接要求这四个请求撤销，内部active／属性撤销另按Advisory实现理由判断。D4.10对象是该头首个 `@file` 与实际多次请求：一次性guard会吞掉第二次STOP，函数内映射又违反00023。真实C/C++非法／嵌套／错配拒绝、段继承及STOP后清理控制误配风险；路由调用在独占区外、不可变配置生命周期及真实CAN/DID消费者控制实际借用风险。这些证据不替代当前Windows执行、provider或完整221项判断；后续累计复核与BSW00115逐处注释仍待完成。
