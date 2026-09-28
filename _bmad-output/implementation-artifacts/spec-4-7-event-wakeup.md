---
title: '4.7 事件所有权与无丢失唤醒'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '09faac506545809271a026d5d90d89bed03fc550'
story_key: '4-7-保持事件所有权与无丢失唤醒'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous per-story execution">

## Intent

输入记录先完整发布，再通知唯一汽车任务；事件合并不丢失记录或完成 ticket。Extended Task 的事件属于该实例，Wait 不消费，Clear 只作用于调用任务；实际等待、唤醒和 ready 顺序仍由固定 FreeRTOS backend 驱动。

## Boundaries & Constraints

遵循 OSEK2.2.3 §7、§13.5（原件页27–28、60–62）及 INTEGRATION-CONTRACT §3–5。复用4.6已验证等待事务，不使用 notification pending 状态代替存储事件。bridge/ISR只复制记录或设置事件，不调用 BSW/SWC。验证模拟 Category1/2 核心调用层级，不宣称真实 MCU 中断、完整 Hook/错误义务或4.18完成。不得降低W3门、扩展其他Epic或修改依赖原件。

## I/O & Edge-Case Matrix

| 场景 | 预期 |
| --- | --- |
| 事件先置位、连续Wait、Wait后置位 | 不消费事件，等待/唤醒及高优先级恢复位置正确 |
| 两个Extended Task使用相同bit | Clear/Wait仅作用于自身；Get读取指定对象，不混用owner |
| Basic、非法ID、suspended对象、NULL输出、ISR上下文 | 适用标准错误、输出及完整前态保留 |
| Get之前、Get/Clear之间、Clear之后、判空/Wait之间发布 | 每条记录/ticket恰消费一次，未处理记录不会永久睡眠 |
| 多条记录合并事件、队列满、ticket耗尽 | 有界明确拒绝，无覆盖/丢旧记录，无半发布和伪确认 |
| Category1/2调用event/resource | Category1拒绝；Category2允许Set/Get和有访问权资源，拒绝Wait/Clear等Task-only服务 |

</frozen-after-approval>

## Code Map

- `runtime/os/src/Os_Backend.c`、`Os.c`：保留stored mask、current_task_index、Wait事务、实际kernel ready stamps；收紧Category1身份检查。新实例仍由既有根入口清空事件。
- `runtime/os/include/Os_Target.h`、`src/Os_Backend.h`：目标显式Category1 bitmask；可选输入接收Task/event配置。公开有界复制输入API与Task消费API，固定单bridge生产者/单配置Task消费者，不暴露BSW对象。
- `runtime/os/src/Os_Mailbox.c`、`include/Os_Mailbox.h`：静态256槽SPSC记录，32字节不透明载荷，单调uint64 ticket；Windows Interlocked发布/读取count，槽内容先完成后发布。bridge通过专用Category2模拟IRQ通知Task；事件只是提示。生产者身份、容量、关闭和溢出拒绝明确。
- `runtime/os/tests/event_wakeup.c`：原生进程、真实host生产线程和ISR，控制交错；独立literal记录/ticket/trace及实际TCB检查。复用已有physical suspend observer而不调用第二selector。
- `scripts/epic4_os.py`、`scripts/test_epic4_os.py`、`core/tests/end_to_end.rs`：注册正式`epic4_event_wakeup_races`，严格向量及证据变异检查。
- 前序OS harness配置机械补齐新增字段，文档和`docs/assurance/evidence/epic4/`保存当前证据，不覆盖历史证据。

## Tasks & Acceptance

- [x] event服务所有权/新实例和Category1/2上下文矩阵，前态保留。
- [x] mailbox完整发布、唯一ticket、有界拒绝、消费及实际ISR通知接线。
- [x] Get/Clear/Wait交错、合并、多次重复与实例重启独立正反向验证。
- [x] 正式exact入口、前序回归、增量质量门及三路只读复核。
- [x] 保存证据、同步sprint、独立增量本地提交后继续4.8。

Given 连续Wait且尚未Clear，When 同一事件已置位，Then 两次都立即返回且stored bit不变。

Given 记录/ticket先发布，When 通知与Get/Clear/Wait交错，Then 所有已接纳记录按ticket顺序恰处理一次；队列非空的重新检查不进入等待，判空后发布由仍存储的事件唤醒。

Given 错误对象或调用层级，When 请求服务，Then 返回适用标准状态并保留对象、输出、资源及原实例。

## Design Notes

事件身份为owner Task加mask；公开标准服务不发明“mask越界”错误，OSEK服务表未定义该错误。Clear没有目标参数。新目标配置只允许一个bridge输入生产者和一个Task消费者，生产者不使用汽车临界区或访问汽车状态；复制队列使用目标原子发布协议。256槽和32字节载荷是当前最小运输容器，不解析CAN/BSW，也不实现HostBatch执行协议（4.14）。完整输入确认和tick完成仍由后续集成故事消费ticket闭合。

## Implementation Notes

依赖4.6已提交并通过完整增量门。无未决产品选择或不可逆操作；新增目标接口、静态队列和配置字段属于本故事足迹，用户已授权逐story自主形成规格及实施。

已实现Category1核心层级和不受Category2资源ceiling屏蔽的政策，既有实际Waiting/ready转换保持。静态256槽单生产者/单消费者队列和ISR30接线通过22原生向量；包括Get/Clear交错、实际阻塞后唤醒、522条环绕、最后uint64 ticket和队列满拒绝、接口所有权与配置拒绝。第一版阻塞测试的低优先级监测任务过早关闭进程，修正监测条件为接收Task已结束；最终严格检查records=1/ticket=1，不能以早关闭的exit0作为通过。新增正式Rust exact入口恰选1项并通过；6个OS验证脚本测试和增量格式/C99已通过。扫描发现的局部未全初始化、赋值条件和可缩小静态变量作用域已修正；完整规则/分析模型及偏离仍待4.20，不预先声明符合。

## Spec Change Log

## Review Triage Log

- medium / patch — edge指出StartupHook或后续端口代码可替换已保留IRQ30，导致队列接纳但失去通知。检查复制端口vPortSetInterruptHandler确实无所有权检查；该触发在当前StartupHook接线可复现。补充私有端口注册guard和实际替换拒绝向量，保留既有selector/上下文和原件。
- false — gap的Other finding称input-inactive任务3的autostart_modes=1。实际初始化为`{3u, "N", inactive, 2u, 0u, OS_EXTENDED_TASK, 1u, ...}`：第5字段autostart_modes=0，第7字段activation_limit=1。场景将input_task设为3，因此Os_TargetPrepare的非零autostart检查返回E_OS_VALUE，23原生向量与独立blind实际运行均通过；该具体前提不成立。
- blind完整复核无发现并实际运行23原生向量；edge与gap均已complete。没有保留未分级的发现。

最终实现扩展至23原生向量，加入所选启动mode不激活接收Task的线程创建前拒绝。增量格式/C99通过；当前等待三路只读复核和最终完整增量门，不将阶段测试或partial MISRA视为完整出口。

IRQ30修复经原edge复核者聚焦确认，原缺口已关闭且无修复引入的缺陷。最终26原生向量包含启动/运行替换拒绝及Category1在Category2资源ceiling期间的实际交付；复核发现均已处置。

## Verification

`cargo test --manifest-path core/Cargo.toml epic4_event_wakeup_races -- --exact --nocapture`恰选1项；native event向量及前序activation20/finish22/resources25/stack23/lifecycle46回归；`python scripts/verify.py --scope all --base 09faac506545809271a026d5d90d89bed03fc550`。实际新C与宏配置补做partial Cppcheck/MISRA，诊断逐项处置但不宣称完整符合。保持无头，不启动用户桌面窗口；完整SC1/4.20质量/4.22交接未完成前Epic不done。

最终正式exact入口恰选1项、26向量全部通过；完整增量门退出0：28 Python测试、源码卫生/新改行格式/C99、UI lint/build、3 Rust单元和66集成测试（包括全部前序OS向量）、核心及桌面clippy、桌面build均通过。Cargo沿用C盘临时target，dev/test增量关闭；未删除缓存或启动可见桌面。两份当前证据的全部product_sources与最终工作区哈希一致。partial扫描73条处置无unresolved行；Windows SDK分析模型、adopted-source和未批准host-report偏离仍如实保留，完整4.20质量出口未闭合。当前故事已验证可提交，完整Epic/W3/SC1门仍按后续依赖保持。
