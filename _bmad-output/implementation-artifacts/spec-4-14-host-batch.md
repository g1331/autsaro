---
title: '4.14 通过 HostBatchV1 运行并闭合输出确认'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '62da62723e0427fa7aa15fd22782eb65ffecb692'
story_key: '4-14-通过-hostbatchv1-运行并闭合输出确认'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

把4.13真实ECU的输入、受控tick、汽车任务与异步输出确认连成HostBatchV1执行入口。BEGIN/RX只暂存，COMMIT接纳完整有界批，成功必须对应真实等待/排空点，不以入队或打印代替完成。

## Boundaries & Constraints

同一标准计划、Windows x64固定FreeRTOS/GCC和唯一Task_Ecu保持。文本为BEGIN <uint64 epoch>、RX <Classical CAN id> <dlc> <hex>、COMMIT；完整批非递减epoch、连续batch ID、统一uint64序号，最多256消息、跨度1000ms。非法批在推进汽车时间/状态前整体拒绝；预处理只改主机暂存。十六进制载荷恰为2*dlc字节字符，严格数字/编码/行界限，不能接纳尾随垃圾或负数。下一执行批须在上一COMMIT完整闭合后。

a→b逐个确认a+1…b−1，在b先处理全部输入再执行b tick；同epoch及epoch0只处理即时回调。每轮真实输入→驱动→CanTp→Com deadline→应用/发送→Dcm→输出确认；诊断调用不能在应用提交前偷跑。宿主阻塞I/O留在原生bridge，只有实际输出/flush成功才提交匹配ticket/PDU确认。重复/错误确认拒绝，输出256容量溢出或任何输出失败关闭ECU。每个COMMIT共用5000ms宿主单调watchdog；超时不继续汽车时间、不声称已执行批回滚。

沿用4.13原始来源、来源/预览守卫、离线构建、旧目标回归与三路复核；不引入第二时钟、runnable调度器或直接测试器写Com。4.15/16仍负责完整应用/协议验收，4.20及交接出口保持未通过；不推送、不扩展Epic2/5/6。

## I/O & Edge-Case Matrix

| 场景 | 输入／状态 | 预期 |
| --- | --- | --- |
| Ready初始或同epoch | BEGIN 0/RX/COMMIT，重复epoch | 真实输入及即时确认完成，不重跑周期 |
| 未来epoch | 0→10、10→1000，多消息 | 逐1ms完成；目标输入先于deadline，DID见同tick应用提交 |
| 接纳边界 | 256/257消息，1000/1001跨度 | 合法通过，超限整批拒绝且汽车状态/时间未改 |
| 编码/顺序/数值错误 | 负数、溢出、垃圾、DLC/hex、倒退、嵌套BEGIN或未完成新批 | 明确拒绝，保留先前完成状态，无丢最旧项 |
| 输出/确认 | 成功、失败、重复、ticket/PDU不匹配、合并事件 | 只成功输出确认；错误不伪成功，输出故障关闭 |
| 静止/超时 | 当前tick与输入/输出/确认未排空、5000ms到期 | COMMIT不能成功；超时关闭，不跳时/回滚 |

</frozen-after-approval>

## Code Map

- runtime/ecu/include与src：有界HostBatchV1暂存/解码、原生执行与输入批描述、统一序号和静止点；公开主机接口保持复制/错误契约。
- runtime/ecu/src/Ecu_Target.c、runtime/os/src/Os_Time.c：复用真实OS input/tick/WaitEvent发布边界，补齐同epoch批完成，不读汽车私有数据；旧目标条件编译分开。
- runtime/src/Dcm.c、PduR.c与同计划配置：审查同tick应用/DID及异步确认，必要即时队列由汽车唯一owner拥有，保留旧目标。
- runtime/ecu/build.ps1、core/src/integration/ecu.rs：完整打包实际bridge及明确构建/运行入口，继续真实编译预检和probe回归。
- core/tests/support/epic4_batch.rs与end_to_end.rs：正式epic4_host_batch_commit，工程外控制消费者及固定预期；256/1000/拒绝/失败/超时均执行。

## Tasks & Acceptance

- [x] 实现有界文本与内存批接纳，覆盖整批拒绝和不可变完成epoch/序号。
- [x] 连通native bridge、逐毫秒与同epoch路径、目标输入顺序和真实等待/排空发布。
- [x] 输出/flush后确认、错误确认拒绝、故障及5000ms关闭；完整离线工程入口与来源。
- [x] 正式独立向量、旧4.13与W1必要回归、适用MISRA、完整增量门、三路独立复核及提交/sprint。

## Implementation Notes

4.13已提交62da627。HostBatch暂存/严格解码、生产文本入口、完整批复制邮箱、逐tick执行、真实Waiting发布及物理sink后确认已接线；打包预检覆盖probe和HostBatch两种入口。codec与原生消费者已通过256/257、同epoch、未来必须真实tick、错误/重复确认、诊断接收占用及恢复向量。独立向量曾实际读到同tick旧DID，现由汽车owner有界延迟诊断队列在应用提交后的Dcm阶段派发；PduR保持单物理连接，旧目标按宏隔离。

1000ms向量最初在约28tick关闭；日志显示128字节诊断轨迹恰好填满，实际原因是Os_TargetTrace容量满时主动关闭，并非watchdog吞吐不足。完整构建补丁0001早已移除周期定时器，原始vendor中的定时器不能代表执行源码。诊断前缀现在保留原字节并在关闭报告中明确trace_dropped，容量满不改变汽车执行；真正输出256队列溢出仍关闭。重新执行已实际完成1000tick和100次真实CAN输出、正常关闭、报告4082个省略marker；5000ms固定COMMIT watchdog与真实完成确认未变。

当前三项正式HostBatch测试已通过（54.65s），包含生产文本非法后缀、实际sink失败/阻塞、序号极限及TestMode启动owner经真实Can_TransmitPdu填满256并触发257关闭。精确N_Cr=200ms处输入先于timeout的向量通过；发现并修正目标DoCAN短FC/NRC需固定DLC=8，诊断输入非8字节在推进状态前整批拒绝。旧主机目标条件宏保持原路径。W1受控时间40个原生向量已通过并刷新实际来源证据，增量格式/C99检查通过。Windows保留目录名nul已改为embedded_nul，协议拒绝预期未修改。完整增量门正在执行，新增明确全局序号、执行后协议拒绝与恢复、序号Execute预检向量会由该次门执行。尚待受影响旧4.13回归、静态分析与完整增量门、三路独立复核，不得提前标done。4.20完整SC1/MISRA及交接出口仍未通过。

## Spec Change Log

## Review Triage Log

- blind：无可确认的可达缺陷；只读复核完整diff、周边队列/完成逻辑与实际内核补丁，未运行测试。
- edge：无可确认边界缺陷；未用空发现替代行为测试或完整出口。
- gap／medium／patch：生产stdout只在进程退出后读取，可能漏掉生产sink未及时flush。补入保持stdin打开的真实生产pipe消费者：进程仍存活时读取Ready、真实OUT及COMMIT_OK，再发送同epoch批并读取回执；错误/超时先关闭专属子进程并回收reader。现有sink源码每条输出执行fflush成功后才由桥接确认。新增向量仍须正式复验。
- root／medium／patch：打包BSW原始来源身份受core.autocrlf影响。按实际33个include_bytes生产者分别固定原有LF/CRLF检出规则，不改源字节；从当前Git索引以core.autocrlf=false检出到专属新目录后，33项逐字节比较均通过，声明摘要和实际生成行为保持。生产live-pipe及全部最终向量已通过，前述gap完成。

## Verification

正式epic4_host_batch_commit与epic4_ecu_integration_generation；独立生成/迁移/原生执行和固定输入/轨迹预期。python scripts/verify.py --scope all --base 62da62723e0427fa7aa15fd22782eb65ffecb692。MISRA C:2012修订基线适用人工/真实生成部分Cppcheck，完整符合声明仍由4.20出口决定。

完整增量门退出0：29 Python、3核心单元、80核心集成（285.02s）、UI lint/build、双clippy及桌面build通过。复核后live-pipe生产输出向量恰选1项通过（55.98s）。部分扫描修正后最终3项HostBatch正式向量通过（52.08s），最终4.13生成/双入口链接/改名/保护回归恰选1项通过（188.07s），旧CanTp异步确认回归及最终core clippy/增量格式通过；W1 40原生向量通过。生产CLI再次预览/安装/编译最终工程，四组实际生产轨迹以及最终来源/生成树/二进制身份逐项核对通过，见host-batch.json。

Cppcheck2.21 C99/win64当前HostBatch的26个主机产品C单元使用新私有分析目录扫描，退出1、诊断保留在host-batch-static-analysis.json；四个内核C单元未在这次产品补充扫描覆盖，完整采用代码义务仍待核查。排除ecu_probe.c仅因完整构建入口实际互斥选择main。工具Windows/CRT配置缺失与实际严格GCC原型验证、主机stdio的未批准Required偏离、Advisory及待逐条核查分别保留。此记录不是MISRA/SC1符合性通过，完整221项与合法原文/批准出口仍由4.20承担，未更改质量门、能力声明或Epic完成状态。三路只读复核已完成，唯一验证缺口补测通过；没有将已确认的4.14行为缺陷递延。所有验证无头，不干扰用户桌面，不推送。
