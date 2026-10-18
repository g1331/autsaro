# COM

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。

文档：`AUTOSAR_CP_SWS_COM.pdf`（`8addd33ad1e625b4e9d01afc938131d424f7161f18075f44d2a5fcfa925c5190`）。draft 条目（`SWS_Com_00900／00903`、`91016–91021`、`ECUC_Com_10031`）与 valid 条目同等处置，不宣称为成熟条款（[README §1](README.md#1-固定基线)）。

## 所选配置

主线只接受 `VARIANT-PRE-COMPILE` 的 Com 配置。`ComGeneral` 的三个开关（安全事件、信号组数组 API、版本信息 API）均为 `false`，`ComSupportedIPduGroups` 为 1；其余 `ComGeneral` 参数不在所选目录中。

- 每个 I-PDU 是 `NORMAL`、`IMMEDIATE` 的 4 字节 PDU，恰含一个从第 0 位开始、32 位、小端、初值 0、`PENDING` 的 `UINT32` 信号，没有未用区域、更新位、callout、过滤器或 metadata；句柄号由生成器分配，显式的 `ComHandleId`／`ComIPduHandleId` 在生成前拒绝。
- 全部 Rx I-PDU 属于唯一的 I-PDU group（句柄 0），只按 I-PDU 做接收截止监控：首次超时为 0、超时动作为 `NONE`，`ComTimeout` 等于接收端口的存活超时且是 Rx 主函数周期的整数倍。
- Tx I-PDU 不属于任何 group，由 `Com_Init` 启动（SWS_Com_00840），`ComMinimumDelayTime` 为 0，只有 `ComTxModeTrue` 下周期等于 Tx 主函数周期的 `PERIODIC` 模式；没有 `ComTxModeFalse`，其缺省模式 NONE 只经 `Com_SwitchIpduTxMode(…, FALSE)` 选中。发送信号不做截止监控。
- 唯一用户是 RTE：`ComUserModuleCnf` 位于 `Rte/RteComUser` 下，回调只有 `COM_RX_ACK`（`Rte_COMCbk`）与 `COM_RX_TOUT`（`Rte_COMCbkRxTOut`），按 `ComUserCbkHandleId` 区分接收信号。
- 各有一个 `ComMainFunctionRx` 与 `ComMainFunctionTx`，生成为 `Com_MainFunctionRx_<名>`／`Com_MainFunctionTx_<名>`（声明在 `SchM_Com.h`），每个 I-PDU 由其 `ComIPduMainFunctionRef` 引用的主函数处理。

Com 只在自己的 `COM_STATE` 区内读写信号值、计时器与待送通知，离开该区后再调用 PduR 与用户回调（[ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md)）。接收与超时通知在区内登记，同一时刻只有一个上下文按登记顺序送达：每次接收各通知一次，重复的超时合并，最后登记的事件最后送达。

## 条款处置

由测试验证的条目见 `core/tests/fixtures/bsw-contracts/com_contract.c`、`com_chain_contract.c` 与 `rte_com_contract.c` 中的 `Verifies:`；拒绝测试见 `core/tests/it/integration/single_component.rs`。下表处置其余条目。“配置”类写明控制该义务的 ECUC 参数及主线不提供的取值（未写取值的参数整项不提供），这些配置在生成前被拒绝，每个参数都有带 `Rejects:` 标记的测试。

| 分组 | 处置与条目 |
| --- | --- |
| 总括与 ISO 17356 | 已核对（规范原文：总括条款，义务由本规范各条目展开并在本篇逐条处置）：SWS_Com_00010、00011、00012；已核对（源码：没有实现 ISO 17356-4 的可选特性）：SWS_Com_00013；已核对（配置模型：参数以 ECUC ARXML 提供）：SWS_Com_00006；已核对（规范原文：规范自身列出的不适用 SRS，没有实现义务）：SWS_Com_NA_00999 |
| 数据类型与端序 | 不适用（配置：ECUC_Com_00127=BOOLEAN／UINT8／UINT16／UINT64／SINT8–SINT64／FLOAT32／FLOAT64／UINT8_N／UINT8_DYN、ECUC_Com_00157=BIG_ENDIAN／OPAQUE）：SWS_Com_00675、00007、00810、00472、00674、00008、00723、00579、00580；已核对（源码：`Com_RxIndication` 先拆装并更新信号值，再登记通知）：SWS_Com_00352 |
| 信号组 | 不适用（配置：ECUC_Com_00345）：SWS_Com_00829、00741、00769、00742、00743、00770、00682、00683、00718、00575、00513、00876、00484、00050、00051、00821、00200、00635、00637、00201、00638、00639、00461、00557、00645、00860 |
| 信号组数组访问 | 不适用（配置：ECUC_Com_10002=true）：SWS_Com_00841、00842、00843、00848、00844、00845、00850、00846、00847、00849、00851、00852、00853、00854、00855、00856、00857 |
| 过滤与 TMS 计算 | 不适用（配置：ECUC_Com_00339）：SWS_Com_00694、00695、00602、00325、00380、00439、00764、00866、00273、00836、00132、00603、00604、00302、00303、00231、00793、00676、00678、00679、00763、00813、00245、00229、00495、00535；已核对（源码：没有过滤器，所有 I-PDU 的 TMS 恒为 true，只有 `Com_SwitchIpduTxMode` 显式改变发送模式）：SWS_Com_00677、00605 |
| 信号网关 | 已核对（源码：没有信号网关代码）：SWS_Com_00370；不适用（配置：ECUC_Com_00544）：SWS_Com_00377、00872、00873、00357、00360、00361、00383、00735、00833、00362、00701、00702、00703、00704、00705、00706、00359、00466、00539、00365、00384、00598；不适用（配置：ECUC_Com_10013）：SWS_Com_00400、00666、00890 |
| 初始化 | 已核对（源码：`Com_Init` 以信号初值装入每个 I-PDU；所选 I-PDU 由一个 32 位信号占满，没有未用区域）：SWS_Com_00217、00015；已核对（配置模型：同一 I-PDU 只有一个信号，位于第 0 位且占满 4 字节）：SWS_Com_00102、00785；已核对（配置模型：初值只接受字符串 `0`）：SWS_Com_00790 |
| 传输属性与发送模式 | 不适用（配置：ECUC_Com_00560=TRIGGERED／TRIGGERED_ON_CHANGE 等）：SWS_Com_00330、00767、00734、00768、00762、00625；不适用（配置：ECUC_Com_00137=DIRECT／MIXED）：SWS_Com_00582、00467；不适用（配置：ECUC_Com_00281）：SWS_Com_00279、00305、00494；已核对（配置模型：只组合 `PENDING` 信号与 `PERIODIC` 模式）：SWS_Com_00238；已核对（配置模型：每个 `ComTxIPdu` 都有 `ComTxModeTrue`，TMS 恒为 true，不会选到未配置的模式）：SWS_Com_00465、00497 |
| 显式模式切换 | 不适用（配置：ECUC_Com_00180）：SWS_Com_00881 |
| 发送失败重试 | 不适用（配置：ECUC_Com_00780）：SWS_Com_00773、00774、00775、00776 |
| 无效值 | 不适用（配置：ECUC_Com_00391、ECUC_Com_00314）：SWS_Com_00099、00680、00681、00717、00642、00859 |
| I-PDU group | 已核对（配置模型：每个 Rx I-PDU 恰属于唯一的 group，group 只含 Rx I-PDU）：SWS_Com_00771、00871、00534；已核对（源码：`Com_IpduGroupStart` 重新开始接收截止监控，与 `Initialize` 无关；首次超时为 0，接收到之前不监控）：SWS_Com_00787、00733 |
| 不属于 group 的 Tx I-PDU | 已核对（配置模型：Tx I-PDU 不属于任何 group，初始化后一直处于启动状态，没有停止、未写入信号或按内容选模式的情形）：SWS_Com_00223、00228、00334、00777、00800、00884 |
| 停止与截止监控 | 不适用（配置：ECUC_Com_10034=COM_TX_ERR）：SWS_Com_00479 |
| 接收截止监控 | 已核对（配置模型：每个 I-PDU 只有一个信号，没有更新位）：SWS_Com_00291；已核对（源码：计时只由接收与主函数推进，不读取信号值）：SWS_Com_00738；不适用（配置：ECUC_Com_00412=REPLACE／SUBSTITUTE）：SWS_Com_00470、00875、00744 |
| 发送截止监控 | 不适用（配置：ECUC_Com_00263、ECUC_Com_00183）：SWS_Com_00481、00445、00878、00879、00880、00696、00835、00697、00304、00308、00739、00392、00115；不适用（配置：ECUC_Com_00709）：SWS_Com_00708 |
| 最小发送间隔 | 已核对（配置模型：`ComMinimumDelayTime` 只接受 0，不做 MDT 监控）：SWS_Com_00471；不适用（配置：ECUC_Com_00181）：SWS_Com_00698、00828、00469、00812、00388、00475；不适用（配置：ECUC_Com_00788）：SWS_Com_00789 |
| 解包 | 不适用（配置：ECUC_Com_00119=DEFERRED）：SWS_Com_00301；不适用（配置：ECUC_Com_00158=0）：SWS_Com_00870 |
| 更新位 | 不适用（配置：ECUC_Com_00257、ECUC_Com_00576）：SWS_Com_00117、00794、00292、00055、00059、00061、00801、00062、00577、00578、00324、00802、00067、00310 |
| 大型与 TP I-PDU | 不适用（配置：ECUC_Com_00761=TP）：SWS_Com_00662、00863、00838、00882、00760、00713、00714、00867、91015、00650、00725、00691、00654、00655、00657、00721、00818、00874、00692、00782、00693、00783 |
| 动态长度与 uint8[n] | 不适用（配置：ECUC_Com_00127=UINT8_DYN／UINT8_N）：SWS_Com_00753、00757、00832、00758、00627、00628、00629、00630、00690、00711、00712、00724、00754、00755、00756、00105、00443、00553 |
| metadata | 不适用（配置：ECUC_Com_10004）：SWS_Com_00887、00888、91005、91006、91007、91008、91009、91010、91011、91012、00858、00862、00868、00891 |
| 开发错误 | 不适用（配置：ECUC_Com_00141）：SWS_Com_91014、00442、00670 |
| 安全事件 | 不适用（配置：ECUC_Com_10042=true）：SWS_Com_00900、00901、00902、00903 |
| 多核与主函数 | 已核对（生成样本：每个 I-PDU 由其 `ComIPduMainFunctionRef` 引用的主函数处理，引用不符时以 `COM_RX_GROUP` 拒绝）：SWS_Com_00885；已核对（生成样本：每个 `ComMainFunctionRx`／`Tx` 容器生成 `Com_MainFunctionRx_<名>`／`Com_MainFunctionTx_<名>`，声明在 `SchM_Com.h`）：SWS_Com_00398、00399；已核对（源码：未初始化时 `Ecu_ComMainFunctionTx` 直接返回）：SWS_Com_00665；不适用（配置：ECUC_Com_10020）：SWS_Com_00886、91013 |
| 类型与通用 API 约定 | 已核对（源码：`Com_ConfigType` 的内容由实现定义）：SWS_Com_00825；已核对（规范原文：不要求实现检查重入）：SWS_Com_00321 |
| 版本信息 | 不适用（配置：ECUC_Com_00438=true）：SWS_Com_00426 |
| 用户回调 | 不适用（配置：ECUC_Com_10034=COM_TX_ACK／COM_TX_ERR／COM_TX_TOUT／COM_INV）：SWS_Com_91016、91017、91018、91021 |
| I-PDU callout | 不适用（配置：ECUC_Com_00387）：SWS_Com_00221、00492、00700、00816、00346、00381、00719、00720、00839；不适用（配置：ECUC_Com_00765）：SWS_Com_00766、00395 |
| post-build | 不适用（配置：ECUC_Com_00540 声明 VARIANT-POST-BUILD）：SWS_Com_00373 |
| 发送信号的归属 | 已核对（配置模型：唯一的 Com 用户是 RTE）：SWS_Com_00892 |
| 名字规则 | 已核对（所选 ComConfig 下 signal／IPdu／group 共享对象路径，Graph 重名拒绝与 SHORT-NAME 标量语法；回调仅接纳固定 Rte_COMCbk／Rte_COMCbkRxTOut，不与 Com_ 前缀相撞；symbolic_com_names_share_the_standard_short_name_namespace）：SWS_Com_00401、00402、00732 |

### ECUC 参数

| 分组 | 处置与条目 |
| --- | --- |
| 模块与通用参数 | 已核对（配置模型：模块只接受 `VARIANT-PRE-COMPILE`，三个开关只接受 `false`，`ComSupportedIPduGroups` 只接受 1）：ECUC_Com_00540、00541、10042、10002、00438、00710 |
| I-PDU、信号与 group | 已核对（配置模型与生成样本：`Ecu_RuntimeConfig.c` 的 Com 行取自这些容器与参数，取值超出所选配置时在生成前拒绝）：ECUC_Com_00337、00340、00493、00119、00761、00206、10012、00518、00711、00341、00184、00185、00344、00183、00412、00263、00259、00158、00157、00170、00127、00560、00002 |
| 句柄号 | 已核对（配置模型：省略时由生成器分配，显式值在生成前拒绝）：ECUC_Com_00165、00175 |
| 主函数 | 已核对（配置模型与生成样本：时基须等于 OS 中该主函数的调度周期，分区引用指向唯一的 `EcucPartition`）：ECUC_Com_10011、10018、10017、10014、10010、10019 |
| 发送参数 | 已核对（配置模型：`ComTxIPdu` 必有，最小发送间隔只接受 0，只有 `ComTxModeTrue` 下的 `PERIODIC` 模式，`ComTxModeFalse` 只接受缺省）：ECUC_Com_00496、00181、00017、00351、00137、00178、00455、00454 |
| Com 用户 | 已核对（配置模型：`ComUserModuleCnf` 只接受 `Rte/RteComUser` 下的定义位置，即该目的 URI 的唯一目的；头文件为 `Rte_Com.h`，回调类型与名字固定，每个接收信号恰有一个 `ComUserSignal`）：ECUC_Com_10039、10040、10030、10032、10028、10033、10035、10025、10038、10036、10034 |
| 未选的通用参数 | 不适用（配置：ECUC_Com_10000、ECUC_Com_00141、ECUC_Com_00788、ECUC_Com_10004、ECUC_Com_00780、ECUC_Com_10001、ECUC_Com_00783、ECUC_Com_00782）：ECUC_Com_10000、00141、00788、10004、00780、10001、00783、00782 |
| 安全事件 | 不适用（配置：ECUC_Com_10041）：ECUC_Com_10041、10043 |
| 信号网关与路由主函数 | 不适用（配置：ECUC_Com_00544、ECUC_Com_10013）：ECUC_Com_00544、00545、00548、00546、00549、00232、00257、00550、00551、00547、10013、10016、10015、10021 |
| 过滤器 | 不适用（配置：ECUC_Com_00339）：ECUC_Com_00339、00146、00235、00317、00318、00313、00312、00147 |
| 信号组 | 不适用（配置：ECUC_Com_00345）：ECUC_Com_00345、10003、00001、00520、00519 |
| I-PDU 的未选参数 | 不适用（配置：ECUC_Com_00387、ECUC_Com_00709、ECUC_Com_00765、ECUC_Com_00576、ECUC_Com_10022）：ECUC_Com_00387、00709、00765、00576、10022、10023、10024 |
| 信号的未选参数 | 不适用（配置：ECUC_Com_00314、ECUC_Com_00811、ECUC_Com_00391、ECUC_Com_00437、ECUC_Com_10006）：ECUC_Com_00314、00811、00391、00437、10006 |
| 发送模式的未选参数 | 不适用（配置：ECUC_Com_00281、ECUC_Com_00282、ECUC_Com_00180）：ECUC_Com_00281、00282、00180 |
| 发送前通知 | 不适用（配置：ECUC_Com_10020）：ECUC_Com_10020 |
| Com 中的用户模块与信号组用户 | 不适用（配置：ECUC_Com_10031、ECUC_Com_10027）：ECUC_Com_10031、10029、10027、10026 |

## MISRA 偏离

| 规则 | 范围 | 依据与风险控制 | 批准 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7（advisory） | `runtime/bsw/src/Com.c` 中 `Com_InvalidateSignal` 与 `Com_SwitchIpduTxMode` 的定义 | `SWS_Com_00203／00784` 规定这两个 API 经 `Com.h` 以外部链接提供；所选程序没有模块调用它们，Cppcheck 因此报告只在一个翻译单元中引用。偏离只影响符号可见性：两个函数只在 `COM_STATE` 区内读写本模块数据，不调用其他模块，行为由 `com_chain_contract.c` 验证；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-14（#42） |
| MISRA C:2012 Rule 2.5（advisory） | `runtime/bsw/include/Com.h` 中的 `COM_BUSY` | `SWS_Com_00865` 规定该返回码；只有大型或 TP I-PDU 的缓冲被锁定时才返回，所选配置没有这类 I-PDU，因此没有代码引用它。它只是公开常量，已列入应用符号的保留名，不影响行为；原始诊断保留，不加 suppression | 维护者 Umaru，2026-10-14（#42） |

## 已知差距

- 头文件不含软件与规范版本号（#59）。
- Com_Cfg.h 导出 ComSignal、ComIPdu 与 ComIPduGroup 符号；Tx 配置使用 PduR 提供的源句柄，Rx 回调映射使用 COM 提供的 PDU 句柄。
- 生成 C 的 MISRA 诊断裁决与人工评估未完成（#58）。
