---
title: '4.17 Counter/ScheduleTable 完整适用计时与容量'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '10c250426b2cf827c9f6d5df9b8b65374358c21a'
story_key: '4-17-完成-counter-scheduletable-能力及-sc1-容量'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

在现有真实 FreeRTOS 后端与标准 OS 接口上补齐已确认 SC1 计时能力，独立证明八个软件 Counter、至少两个 ScheduleTable 及适用标准服务。参考 ECU 只使用 Alarm 不能代替等级容量和状态机证据；完整 SC1 等级出口仍由4.20/4.22复核。

## Boundaries & Constraints

继承4.2已封存单核 SC1／EXTENDED、同步策略 NONE、global-time=false 的适用性，不增加 SC2/4 同步服务或改写 conditional_na。Os.h 提供标准类型、状态及 StartScheduleTableRel／Abs、StopScheduleTable、NextScheduleTable、GetScheduleTableStatus；共享现有唯一时间状态、Counter增量、临界区和汽车上下文，禁止将 FreeRTOS timer 改名、另设汽车线程或在owner阻塞打印。既有Counter/Alarm接口保持契约；Alarm增加软件Counter增量动作并防止无界循环配置。

八个Counter各有独立模数与值；两个以上合法表可并发、单次或重复运行。ExpiryPoint按唯一递增offset组织，同刻对同Task先Activate再SetEvent。InitialOffset、邻接间距、FinalDelay按底层Counter约束：初始允许0；单次FinalDelay允许0，重复至少mincycle。相对启动offset>0且offset+InitialOffset<=max；绝对启动等待Counter下次等于Start，当前值相等须完整回绕，不能立即执行。链接在前表FinalDelay后启动后表并加后表初始offset；合法替换旧NEXT回STOPPED，非法调用保留前态。自动启动顺序为Task、Alarm、ScheduleTable。

配置验证、Rust生成、实际运行时和RTE TimingEvent映射使用一致的Alarm／ExpiryPoint约束。现有单Task参考剖面、输入顺序和HostBatch不扩展成任意ECU编辑器；新增独立容量配置按相同标准契约构建运行。不改变原始FreeRTOS来源，不重开OS选择，不扩展Epic2/5/6，不推送。C99/MISRA修订基线、真实栈、无头后台及有界资源限制保持。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 封存two-unsynchronized-schedule-tables，C0/C1逐次独立推进 | A在C0=3、事件在5、表0在7停止；B在C1=2、表1在6停止；其他六Counter仍0 |
| 八Counter不同模数/软件增量/回绕/Elapsed、Alarm增量动作 | 每对象值和差分独立，实际动作无丢失，非法值和递归配置拒绝 |
| Rel／Abs与初始0/非0、当前绝对值相等/回绕 | 首个动作发生在规范规定tick，状态在服务返回前为RUNNING |
| 同点Activate+SetEvent、重复及单次FinalDelay0 | 先激活后设事件，终点/重启精确，不重入已经结束的点 |
| Next链接/替换/停止源或NEXT对象、重启 | STOPPED／NEXT／RUNNING和实际轨迹一致，旧NEXT释放，停止后无残留动作 |
| 重复启动、错误id/Counter、越界offset/Start、NULL/禁止上下文 | 标准ID/VALUE/STATE/NOFUNC/ILLEGAL_ADDRESS/CALLEVEL，合法对象及输出哨兵保持 |
| 容量/ExpiryPoint超限、重复offset、非法动作和非NONE策略 | 生成/Prepare在建立线程前拒绝，不静默截断或提升等级 |
| 相同TimingEvent改用合法Alarm或ExpiryPoint及错误周期/offset | 共同规则接受/拒绝，实际调度顺序保持，不能仅检查数组长度 |

</frozen-after-approval>

## Code Map

- runtime/os/include/Os.h、Os_Time.h及src/Os_Time.c、Os_Backend.c：公开标准类型/接口；有界ScheduleTable配置/状态，统一Counter增量与ExpiryPoint动作，启动顺序、合法链接及受控关闭。
- runtime/os/tests/sc1_timing.c、scripts/epic4_os.py、core/tests/end_to_end.rs：正式epic4_sc1_timing_capacity，独立八Counter/双表真实内核配置、封存轨迹和手写边界/错误向量；控制线程输出复制观测。
- core/src/integration/schedule.rs、ecu.rs及相关配置模板：一致的Counter/表配置验证和实际生成；TimingEvent使用Alarm／ExpiryPoint的接受/拒绝边界、公共符号闭包；更新受影响初始化器。
- core/tests/fixtures/epic4_oracles/os.json、docs/assurance/epic4/obligations.json、reviewed-baseline.json：原封存规则/预期保持；4.17全部123条适用/条件义务逐条关联实际结果，未评估不写通过。
- docs/assurance/evidence/epic4/、spec/sprint、runtime/os/README.md：实际源/补丁/配置/二进制身份、状态轨迹及MISRA核查；完整等级/交接仍保持开放。

## Tasks & Acceptance

- [x] 核对R24-11功能页38–46、服务页170–179、容量页129–130及对应配置定义；冻结独立轨迹和全部适用义务映射。
- [x] 完成标准服务、Counter增量Alarm动作、有界NONE表状态机、动作次序、自动启动、链接与错误前态。
- [x] 更新配置生成／TimingEvent映射及受影响初始化器；线程创建前拒绝非法/超容量配置。
- [x] 八Counter/双表封存轨迹、单次/重复/绝对/回绕/同点/链接/停止/重启和全部拒绝向量真实通过。
- [x] 受影响受控时间、完整ECU/HostBatch、最终源身份及MISRA检查、完整增量门、三路独立复核、本地提交和sprint同步。

## Implementation Notes

当前Os_Time.c仅有八Counter/十六Alarm固定存储及已运行单Counter轨迹，缺ScheduleTable类型和服务、Alarm软件Counter增量动作；schedule.rs要求单个Counter和Alarm映射。已读取本地官方R24-11功能/服务/容量原件及封存NONE适用性，已实现有界NONE表、软件Counter增量Alarm的非递归工作栈和实际受控内核tick驱动的宿主定时器Counter。43原生向量及真实双表生成/外部消费者/周期编辑与CAN/DID字节向量通过。已逐项核对123条义务，保持封存适用性及同步条件；完整门和最终复核已通过，完整Epic出口仍开放；4.16已独立提交10c250426b2cf827c9f6d5df9b8b65374358c21a，以此完整提交为实施基线。

## Spec Change Log

## Review Triage Log

- medium / patch：verification-gap 指出周期编辑只覆盖offset0，无法检出非零offset的start错误。补充合法offset2/start8原配置，编辑为period20/start18；两个offset均执行真实生成HostBatch并严格断言CAN/DID在20与40触发、30不触发。补测135.36s通过；独立补测源码复核未发现具体缺陷。
- low / patch：逐条证据复核指出SWS_Os_00286缺少已实际运行的counter-chain直接引用。补充该行及相关Counter动作引用；不改变封存分类。
- blind与edge两路没有可定位缺陷；补充义务复核确认123行范围和NONE同步条件前提边界。复核不替代实际执行或完整SC1出口。

## Verification

最终43原生计时向量与40受控时间回归均通过；正式epic4_sc1_timing_capacity补测135.36s通过。实际CLI最终源生成、生产HostBatch构建及7条独立CAN/DID/receipt记录对比通过。此前完整门83测试/29Python/UI/桌面及Clippy通过；补测后的完整门exit0：83个集成测试全部通过（458.00s）、29个Python测试、UI构建/lint、桌面构建与两组Clippy通过。最终27个生产翻译单元Cppcheck部分扫描exit1，诊断原样保留；新Schedule模块只有Advisory15.5/8.7，形参修改Advisory17.8已消除。完整221项与Required批准不作通过声明。

补测首次遗漏原配置30ms接收超时，于40预期旧RX字节而失败（私有timing417-offset-final.log保留）；读取Com配置和实际过期路径后在30重新输入相同RX，保持触发时刻/字节断言，未改产品超时或降低门。

正式epic4_sc1_timing_capacity及受影响4.8／生成／HostBatch/协议回归；python scripts/verify.py --scope all --base 10c250426b2cf827c9f6d5df9b8b65374358c21a，同主机完整测试按Owner Guide设置两个测试线程，固定watchdog及断言保持。实际C99构建与部分MISRA不替代221项原文核对、Required批准、完整SC1或交接出口。

配置生成保持所选单Task／单软件SystemCounter参考剖面；周期RTE组通过一个重复ExpiryPoint表达，初次deadline及duration匹配TimingEvent周期。多Counter/多点/单次/链接等完整适用能力使用独立原生静态OS配置实际运行，不能借参考工程替代容量证明。硬件Counter证据限定实际内核受控tick ISR驱动的Win64宿主定时器，不声明MCU。
