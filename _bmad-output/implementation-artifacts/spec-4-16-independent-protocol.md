---
title: '4.16 独立协议边界、故障恢复与旧目标回归'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: '8431b5bf6834db903692d9b9081cf7b64a2c2d8f'
story_key: '4-16-独立验证边界-诊断恢复与旧目标回归'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

以4.2封存独立协议目录及补充非法/恢复向量验收真实新组合，准确报告错误、时序及会话状态；新目标只开放已选服务，旧host-v1按旧配置单独运行，不用旧行为代替新目标证据。

## Boundaries & Constraints

复用4.15同实例应用/RTE、4.14完整HostBatch、单Task与真实输出确认。core/tests/fixtures/epic4_oracles/protocol.json保持不变，不从实现/生成器计算预期。物理11bit正常寻址、DLC8/零填充、64字节Rx/Tx缓冲、最多2个DID；服务限0x10/0x3E/0x22，应用DID0x1234及标准会话DIDF186。未选0x2E/0x31/0x27/DTC/NvM不能自动加入新剖面；旧目标服务保留。

固定P2=50、P2*=5000、S3=5000、N_As/N_Ar/N_Bs/N_Br/N_Cr/N_Cs=200ms，BS/STmin/WFTmax=0。输入先于同epoch超时及应用/Dcm；deadline输入有效，晚到不能复活已终止连接。正常传输失败终止对应连接、准确报告实际状态并允许后续合法请求；输出失败、256输出溢出或5000ms宿主watchdog仍关闭ECU，不继续时钟、不假回滚。接纳拒绝前缀不执行，执行后失败保留已执行前缀并准确报告。

失败记录用有界复制/发布边界交给原生桥接，阻塞I/O只在原生线程；统一uint64序号和执行前容量/溢出检查保持。旧目标及完整规范证据分开；不重开OS，不扩展Epic2/5/6、不推送，完整SC1/MISRA/交接仍为后续出口。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 目录11组初始/SR/deadline/DID/会话/多帧 | epoch及CAN/UDS精确匹配，先前周期帧单独核对 |
| 长度、未知DID/服务/会话/未选服务错误后合法SF | NRC13/31/11/12匹配，快照未改且合法请求恢复 |
| FF/CF/FC、错SN、非法FC/STmin、WFTmax0 | 正确分段/确认；错误连接终止、实际错误报告、合法SF恢复 |
| N_Bs/N_Cr在210输入或空210后迟到211 | 边界先输入成功；迟到不复活，失败后合法SF恢复 |
| 扩展会话0后空1000…5000或5000先输入F186 | 前者回默认，后者保持扩展，F186目录字节匹配 |
| DLC/方向/容量/序号批拒绝、实际写失败 | 拒绝前不改汽车状态；写失败保留提交/标准状态，不假正响应 |
| 真实输出失败/阻塞/停机及新进程、旧host-v1 | 故障关闭无成功回执；新进程初始正确；旧目标独立通过 |

</frozen-after-approval>

## Code Map

- core/tests/fixtures/epic4_oracles/protocol.json、本 story 的验证记录，禁止修改目录迎合代码。
- core/tests/support/epic4_protocol.rs、end_to_end.rs、工程外消费者：正式epic4_independent_behavior_and_legacy_regression，真实生产/native执行，目录与手写失败/恢复字节，旧目标单独验证。
- runtime/src/Dcm.c：当前通用dispatch仍走旧服务且未限制2DID；增加目标专属SID及数量守卫，旧路径保持。
- runtime/src/CanTp.c、runtime/ecu/src/Ecu_Target.c：目标目前把timeout当全ECU故障；已核传输失败终止连接并发布真实状态，配置/时间/输出故障仍关闭；WFTmax0拒绝FC WAIT。
- runtime/ecu/include/Ecu_Target.h、Ecu_HostBatch.h及src/Ecu_HostBridge.c、ecu_host_batch.c：有界失败记录、实际epoch/状态、统一序号预检及错误回执，不以打印代替完成。
- runtime/contracts/bsw-v1.json、本 story 的验证记录，不升级完整能力出口。

## Tasks & Acceptance

- [x] 执行全部封存协议及补充正反/恢复向量，独立预期与真实周期帧保留。
- [x] 已选服务/2DID/WFT0守卫、可恢复传输失败及准确epoch/状态/序号，硬输出故障仍关闭。
- [x] S3及N_Bs/N_Cr边界/晚到恢复、基础设施失败/重启/拒绝、旧host-v1离线配置向量单独通过。
- [x] 最终身份、适用MISRA核查/扫描、完整增量门、三路独立复核与本地提交/sprint同步。

## Implementation Notes

4.15已提交8431b5b，81集成门及五组应用向量通过，真实controller故障和owner提交检查可复用。4.2目录固定2DID/WFT0及6组时序预期。现有Dcm仍dispatch旧服务、FC WAIT重新启动N_Bs、目标对CanTp timeout直接fail，属于需要正式证明/修正的范围内缺口，不改预期或当作外部阻断。已以真实生产HostBatch运行11组封存协议、6组时序及SID/FC/错SN恢复；五组独立旧host-v1配置通过。原始三DID实现被独立目录拒绝，修正后通过。最终35个新进程向量与5组旧配置通过，源码/生成物/二进制身份封存；26个产品翻译单元扫描exit1如实保留。完整门两个测试线程通过全部82集成、3单元、29Python及UI/core/desktop检查。默认并行负载曾触发S3实际5000ms宿主watchdog，失败日志保留，不改watchdog或断言；运行环境已进入Owner Guide。完整MISRA原文/Required批准及SC1出口未成立，全程无头。

## Spec Change Log

## Review Triage Log

- false — blind复核提出“S3先expire后dispatch导致边界输入丢失”。实际Dcm_RxIndication在复制请求后先更新last_request_ms，owner先消费输入才调用Dcm_AdvanceTime；s3-input-at-boundary真实返回0462F18603000000，空到期场景返回0462F18601000000。复核只追到deferred dispatch，遗漏接收刷新。
- false — edge复核假设“同epoch多次advance_transport产生两个timeout，3*span身份预算不足”。CanTp_AdvanceTime会在同次调用终止所有已到期Rx/Tx；已终止连接不再次超时，frame_aborted防重复报告，同epoch确认/新传输只会启动200ms的正计时。所选单物理连接每个tick最多一次timeout发布，3*span覆盖tick、Com输出及timeout；诊断输入和旧transport tail另有预算。N_Bs/N_Cr晚到测试均准确记录transport_count=1；实际错误前缀保持epoch210，后续恢复通过。
- verification-gap独立复核无可定位缺口。三路均只读；主代理核对实际源码与已有原生故障/序号门，不以复核意见代替行为证据。


## Verification

正式epic4_independent_behavior_and_legacy_regression、受影响HostBatch/应用/生成与旧目标；python scripts/verify.py --scope all --base 8431b5bf6834db903692d9b9081cf7b64a2c2d8f。严格真实生成C99构建、部分MISRA及未通过范围如实保存，不替代完整出口。

最终完整门：test result: ok. 82 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 579.28s；exit0。最终单项1通过／81 filtered／85.14s。证据位于本 story 的验证记录；实际CLI preview/install、交付工程HostBatch构建通过。三路复核疑点逐条核实，MISRA扫描exit1及完整出口未完成如实保留。
