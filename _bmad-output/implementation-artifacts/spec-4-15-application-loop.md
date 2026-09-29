---
title: '4.15 运行同实例 S/R、提交快照和同步 DID'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'adbc96d39bc2be2132bd504b30a7dde2f3399181'
story_key: '4-15-运行同一应用实例的-s-r-快照和同步-did'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

让EchoApplication经生成RTE运行S/R并提交同实例的值/epoch快照，Dcm经声明的同步C/S服务器读取该快照；真实控制消费者验证初始、有效、过期、恰好deadline接收和写失败的状态，不直接改Com或复制应用状态所有者。

## Boundaries & Constraints

复用4.14真实HostBatch、固定Windows FreeRTOS、单Task_Ecu与严格来源/生成/链接入口。不改标准输入范围、API映射、CAN/诊断参数或HostBatch容量/watchdog；不重开OS选型，不扩大Epic2/5/6。原始计划保持Rx0x320/Tx0x321、uint32小端、10ms周期、30msRx期限、DID0x1234四字节大端，默认/扩展会话可读。

Read初始值0并返回RTE_E_NEVER_RECEIVED；有效接收返回实际值/E_OK；过期保留最后接收值并返回RTE_E_MAX_AGE_EXCEEDED。应用对未接收/过期/读失败采用配置初值0，只经声明的Rte_Write写Tx；成功才原子提交value/epoch，写失败保留之前已提交的value/epoch并记录实际标准状态。应用/服务器/RTE缓存均由同一Task拥有，不能引入原生线程写汽车状态或第二快照。

输入先于该epoch deadline和应用周期，再处理Dcm；同epoch不重复周期。DID同步读取已提交值，只换一次字节序，不直接读取私有Com数据。故障验证用真实owner调用将实际Can controller停止，使实际RTE写拒绝，再恢复同一controller；测试点仅在明确TestMode启用，不替换BSW/RTE/应用实现或伪造失败返回。当前产品只声明Windows主机行为；完整协议验收仍由4.16、完整SC1/MISRA/交接由4.20以后独立出口决定。不推送。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| Ready初始；无Rx，在0读DID或10执行 | Read=0/NEVER_RECEIVED，初始DID四字节0，成功周期提交0/epoch10 |
| 0收到78563412，在10应用和读DID | Read=0x12345678/E_OK；Tx78563412；UDS62123412345678、SF0762123412345678；value/epoch10一致 |
| 默认/扩展会话分别读同实例DID | 经真实0x10会话服务和同步服务器，快照及端序相同 |
| Rx0后到30无新帧，在30执行和读DID | Read保留0x12345678/MAX_AGE_EXCEEDED，应用采用/提交0/epoch30，同tick DID为0 |
| 在30收到新有效值并读DID | 新输入先于deadline，Read新值/E_OK，Tx和同tick DID见该次提交；重复epoch不再提交 |
| 实际停止controller令RTE写拒绝，再恢复 | 记录COM_SERVICE_NOT_AVAILABLE；旧value/epoch保留，服务器仍见旧提交；后续合法写恢复提交 |
| 原生消费者/空指针越权读取应用状态 | 明确拒绝且输出参数保持，无撕裂或原生汽车状态写入 |

</frozen-after-approval>

## Code Map

- runtime/ecu/templates/Rte.c.in：Com_ReceiveSignal当前对valid=0覆盖原值，破坏过期保留值；Com本身保留signal_values，仅清valid。保留实际Com读值，状态仍经Ecu_TargetReceiveStatus映射，初值由真实Com_Init提供。
- runtime/ecu/templates/Application.c.in：当前仅value快照，写失败直接Shutdown；改为同owner提交值/epoch与实际读写状态，成功提交、失败保留。服务器仅编码提交值。
- runtime/ecu/include/Ecu_Target.h与src/Ecu_Target.c、core/src/integration/ecu.rs：明确owner只读应用状态检查和TestMode周期前后检查点，复用现有真实阶段回调。实际传输、Dcm顺序及生成接口来自同一计划，不添加RTE C/S操作或第二周期调度。
- core/tests/fixtures/application_loop.c、core/tests/support/epic4_application.rs、core/tests/end_to_end.rs：外部原生消费者通过完整生成工程运行epic4_application_sr_cs_loop；固定literal状态、epoch和CAN/UDS字节，真实controller故障/恢复，不用BSW桩。
- docs/assurance/evidence/epic4/、本spec和sprint：保存最终生成/源码身份、实际独立轨迹、三路复核与限定能力；不提前升级完整出口。

## Tasks & Acceptance

- [x] 修正RTE过期保留值和标准返回状态，保持未接收、停止、空指针拒绝语义。
- [x] 实现同owner value/epoch成功提交与写失败保留/标准状态，DID只读同一提交；TestMode使用真实停止/恢复路径。
- [x] 正式执行初始/有效/过期/deadline/同epoch/default+extended/越权/实际写失败恢复的独立向量。
- [x] 当前HostBatch与4.13必要回归、适用C/MISRA分析、完整增量门、三路独立复核、提交与sprint同步。

## Implementation Notes

4.14已完成并提交adbc96d。RTE保留真实Com值、应用value/epoch成功提交/失败保留、owner状态检查和TestMode真实周期检查点已实施。正式epic4_application_sr_cs_loop恰选1项通过（27.65s），五个独立进程分别验证初始、过期、deadline、真实controller停止导致的标准读写128及恢复、扩展会话；固定状态133/64/128、提交epoch及CAN/UDS字节均匹配。原生/空指针状态检查拒绝且输出保持，同epoch不重跑周期。

为验证回归向量实际识别原错误，临时恢复原RTE“valid=0时覆盖初值”的代码，仅运行正式入口：stale进程明确失败（27.33s），随后finally恢复修复后原始字节；没有保留错误代码或放宽预期。首次严格链接预检发现新增Std_ReturnType声明缺少显式Std_Types.h；消费者发现Can_MainFunction_Wakeup属于SchM_Can.h。均修正实际包含关系，拒绝结果未被隐藏为通过。增量格式/C99门通过。当前完整增量门、受影响回归、静态分析、三路复核及最终交付身份仍待完成；4.15保持in-progress。真实5000ms与256/257输出边界不降低；完整授权MISRA原文/Required偏离批准仍未成立，不作完整符合声明。

## Spec Change Log

## Review Triage Log

- blind：无可行动缺陷；只读核对实际输入→deadline→应用→Dcm顺序、过期值保留、成功提交和真实controller故障/恢复。
- edge：无可确认边界发现；未运行构建或替代行为门。
- gap：无验证缺口；独立消费者覆盖状态、提交epoch、CAN/UDS固定字节与真实写失败恢复。所有完整出口继续依赖实际门结果。
- root：验收回调的日志改为有界不可变观测记录，由native bridge读取/输出，保持汽车owner不阻塞宿主I/O。修改仅限独立测试消费者，最终exact五组向量通过（29.80s）。

## Verification

正式epic4_application_sr_cs_loop，受影响HostBatch/生成/旧目标检查；python scripts/verify.py --scope all --base adbc96d39bc2be2132bd504b30a7dde2f3399181。MISRA C:2012修订基线人工核查与实际生成部分扫描，真实报告工具覆盖及未通过项，完整221项和SC1出口保持独立。

最终完整增量门退出0：29 Python、3核心单元、81核心集成（217.32s）、UI lint/build、双clippy/桌面build通过，包含当前HostBatch和4.13回归。测试日志移动到native后最终正式exact恰选1项通过（29.80s），增量格式/C99及assurance再次通过。生产CLI实际预览/安装同一计划与双入口严格链接；工程外TestMode消费者最终重新编译，五个实际进程状态、epoch、CAN/DID、故障/恢复及原生越权均匹配固定预期，来源/生成树/实际二进制身份记录application-loop.json。三路只读审查无可行动发现及验证缺口，未递延已确认的本story缺陷。

Cppcheck2.21当前生成的26个主机产品C单元补充扫描退出1，原始诊断保留application-loop-static-analysis.json；新增Application/RTE报告为Advisory早返回/外部API使用位置，未删除公共owner检查来消除诊断。工具平台配置、既存主机I/O Required偏离及完整采用代码/221项核查仍未完成；四个内核C单元与工程外消费者未在本次产品扫描覆盖。不作完整MISRA、SC1或交接通过声明，Epic4继续in-progress。全部验证无头，不推送。
