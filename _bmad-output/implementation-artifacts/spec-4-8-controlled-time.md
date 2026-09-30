---
title: '4.8 逐毫秒受控时间与 Counter/Alarm'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'ca06d169331efd45070f614c4ed7ae6146bc1944'
story_key: '4-8-逐毫秒推进受控时间与-counter-alarm'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous per-story execution">

## Intent

每个受控逻辑毫秒只有一份已接纳请求、一次实际内核tick和一份处理完成确认。Counter/Alarm按照汽车模数和动作运行，事件合并不能合并毫秒；汽车时间不由宿主墙钟推进。

## Boundaries & Constraints

遵循INTEGRATION-CONTRACT §3–5、OSEK2.2.3 §9/§13.6和R24-11 SWS_OS §8.4.17–19（原件页180–182），保留唯一固定backend、无自主timer、NORMAL优先级及关闭门。参考SystemCounter为SOFTWARE（已有原创ECUC），max65535、ticksperbase1、mincycle1；Alarm_Work 1/1设置Ev_Work，Alarm_App 10/10设置Ev_App。epoch0和相同epoch不重复周期；一次未确认推进未结束前拒绝下一请求。

八软件Counter和两ScheduleTable完整等级能力及容量验收由4.17闭合；完整错误/Hook矩阵由4.18闭合，但本故事不得吞掉Alarm动作失败。完整BSW周期、输出确认和COMMIT静止点在4.14集成，当前Task必须在处理本tick事件/工作后明确确认。不能用标志篡改冒充实际32bit内核tick回绕，不修改依赖原件、OS选型或W3门。

## I/O & Edge-Case Matrix

| 场景 | 预期 |
| --- | --- |
| 逐一请求1–1000ms并确认 | 每毫秒kernel/Counter/Alarm/work/ticket恰一次 |
| 重复epoch、epoch0、并发未确认或跳跃/倒退 | 不额外推进；错误请求保留完整前态 |
| 10ms、Counter65535→0、内核32bit回绕 | 独立literal动作/计数与实际kernel读数一致 |
| uint64 epoch/ticket上界 | 不回绕，溢出先拒绝，无部分推进 |
| 相对/绝对/周期Alarm、取消、重复设置、错误参数 | 标准状态、原deadline和Counter保持，绝对当前值下一完整模数才到期 |
| GetCounter/GetElapsed、错误ID/值/NULL | 模数正确，Value更新为当前值；拒绝时输出保留 |
| Alarm动作失败或回调 | IncrementCounter仍E_OK；调用已配置ErrorHook，错误可定位；不静默丢失 |
| 未Ready/关闭及Category1 | 不推进，不接受不合法服务调用 |

</frozen-after-approval>

## Code Map

- `runtime/os/include/Os.h`、新增`Os_Time.h`：Counter/Alarm标准类型与服务；独立host逐毫秒请求、tick状态及owner确认接口，标准StartOS仍无私有返回值。
- `runtime/os/include/Os_Target.h`、`src/Os.c`：可选静态Time配置、Counter/Alarm引用/值/动作/autostart校验，旧harness明确NULL。保留4.7input配置及ISR30所有权。
- 新增`runtime/os/src/Os_Time.c`、`src/Os_Backend.c/h`：静态Counter/Alarm状态、实际tick回调、请求/交付/确认状态；请求先完整发布再pending一次tick，唯一汽车Task确认。沿用实际端口临界区与任务/Category2检查。
- `runtime/os/FreeRTOSConfig.h`和有限复制件补丁：开启backend tick hook，实际FreeRTOS tick驱动SystemCounter与动作；隔离测试入口只在test宏下种入实际kernel计数边界，不改CPU上下文/selector。
- 新增`runtime/os/tests/controlled_time.c`、`scripts/epic4_os.py`、`scripts/test_epic4_os.py`、`core/tests/end_to_end.rs`：正式`epic4_controlled_tick_and_alarm`入口、独立时间/Alarm预期、前态快照及门禁变异测试。
- 本 story 的验证记录与新时间证据：生产补丁身份、真实编译/执行和partial静态诊断；保留所有前序证据。

## Tasks & Acceptance

- [x] 静态Counter/Alarm配置及标准服务、autostart、动作错误处理。
- [x] 唯一逐毫秒请求/实际kernel tick/owner完成确认，拒绝未确认推进。
- [x] 实际回绕、独立1–1000预期、epoch边界和Alarm正反向向量。
- [x] 正式exact入口、前序回归、完整增量门、partial静态分析和三路只读复核。
- [x] 保存证据、同步sprint并本地提交，继续4.10；W3仍需4.10–12。

Given参考Alarm和顺序已确认ticket，When请求1000个毫秒，Then每次独立校验kernel tick、Counter、Ev_Work/Ev_App动作与完成记录，没有宿主自主推进。

Given模数/内核回绕或错误Alarm请求，When实际执行边界，Then对应原件与4.2独立预期，拒绝不改deadline/输出/时间档案。

Given某Alarm激活目标已满，When IncrementCounter触发动作，Then服务E_OK、已配置ErrorHook收到E_OS_LIMIT；不能以关闭或忽略替代规定行为。

## Design Notes

Counter与uint64 epoch、FreeRTOS32bit tick分别归属，不能用一个整数替代三者。公共software IncrementCounter遵守标准，参考生成循环仅由受控tick推进SystemCounter；独立服务向量可在其他配置验证软件增量。相对剩余量/模数换算使用可表示完整周期的内部宽类型；对外Tick类型与允许最大值必须一致，不让“绝对当前值”溢出成立即到期。

host请求可在独立原生控制线程执行，不能调用BSW/SWC；tick ISR只推进内核/Counter/Alarm及事件，Task处理实际工作后确认。未确认ticket关闭下一入口，确认状态发布在结果完整后；宿主等待仅是watchdog/测试握手，不是汽车时间。同刻动作顺序固定为静态配置顺序，可复验。

## Implementation Notes

4.7基线已提交、三路复核及完整增量门通过。Counter标准原件已重新读取；GetElapsed无法识别经过多轮模数的差值，按标准保留该限制，不伪称绝对时间差。无未决产品选择、外部副作用或新增依赖。

已接入静态Counter/Alarm标准服务及单次真实tick请求门。汽车TickType为uint64（配置Counter最大UINT32_MAX），独立于FreeRTOS32bit tick；内部/公开剩余量可表示完整Counter模数。参考SystemCounter保持SOFTWARE。tick ISR只推进Counter/Alarm及wake_event（参考可绑定Ev_IO），应用/BSW仍由Task唯一所有。完成标记只在owner真实进入内核suspended-list、事件清空且mailbox无已发布/在途记录后发布；原生完成event消除每tick的宿主Sleep轮询。公开WaitTick只用宿主watchdog，不产生汽车tick。

首轮1000向量因测试每轮Sleep(1)累计宿主等待，在约512次工作触发8秒watchdog；保留失败事实，不提高watchdog或削弱断言。改用原生完成事件后4个实际子进程通过：1000 tick/1000 Work/100 App；Counter65535→0与真实kernel4294967295→0独立回绕；最后uint64 epoch及溢出拒绝；未确认请求/完成读拒绝。初始和重复epoch不推进。26个前序event向量在新增Time接线后通过。正式时间入口已注册，当前只证明这4个向量；完整Alarm正反向、错误/回调、静态质量和独立复核仍待实施，不标done。

后续扩展至38向量：4.2的相对跨模数和绝对周期取消条件、绝对当前值完整周期、UINT32_MAX Counter的4294967296剩余量与elapsed、周期回调、动作E_OS_LIMIT调用配置ErrorHook且保留已有激活、14个标准拒绝、Category1/2实际服务调用、21个静态配置拒绝、真实Windows完成句柄重置/通知失败和IRQ1替换拒绝。完成event改为手动重置；通知成功后才发布结果，提前唤醒仍能重读发布而不丢信号。关闭永久位与PUBLISHING阶段保留，失败记录time_signal_failed且没有完成ticket。每个正常owner还验证7个private API上下文/NULL/未交付拒绝及输出保留。当前native38通过；最终正式exact、全量增量门、partial扫描最终身份及独立复核仍待收齐。

正式时间exact入口已恰选1项通过38向量；前序activation20、finish22、resources25、events26、stack23、lifecycle46全部实际回归通过。主代理阅读完整Time实现、接口/补丁、关键diff和harness，补齐标准时间服务入口/返回的关闭守卫，并保留WaitTick超时后重读的实际错误状态。最新版native38再次通过，增量格式/C99及7项OS验证脚本测试通过。完整增量门和三路复核仍待收齐，sprint保持in-progress。剩余等级义务表已进入runtime/os/README，未把4.17/18/20/22义务当作当前通过。

## Spec Change Log

## Review Triage Log

三路default只读独立复核全部返回complete后逐项核验：

- blind：认为vTaskSuspend只在唤醒后返回，完成发布永远错过。处置为误报。Os_BackendWait持外层taskENTER_CRITICAL；实际Windows port.c的vPortGenerateSimulatedInterrupt仅在ulCriticalNesting为0才等待yield event，内层vTaskSuspend移入真实suspended-list后返回到外层。Os_TimeOnWaiting在外层退出前发布；vPortExitCritical最后降至0才执行pending yield。复制件补丁未改变该嵌套机制，实际1000个完成记录与监测TCB suspended状态共同验证该路径，不按通用端口语义改写。
- edge：预留后、TIME_PENDING前关闭，仍返回E_OK却没有tick。处置为修复。将pending CAS设为最终接纳点，CAS丢给永久关闭位时返回E_OS_STATE且ticket保持；OS_TIME_TESTS只在该边界注入真实Os_TimeClose，原生close-before-pending向量证明kernel0/Work0/App0、拒绝1、关闭reason7，无完成记录。
- edge：完成event分配失败后继续启动。处置为误报。Os_TimeInit调用Os_PortEvent；该实际包装在NULL时执行Os_BackendShutdown，后者RequestShutdown并Sleep(INFINITE)/ExitProcess，不返回启动调用者。新增Time启用且资源故障1原生向量，验证trace=IZ、threads/events/mutexes=0、resource_calls=1、reason7、没有tick或工作，补强原有lifecycle故障测试的Time配置覆盖。
- gap：GetElapsedValue第二个输出NULL未覆盖。处置为补测。采用合法Counter/previous=0与ElapsedValue=NULL，断言E_OS_ILLEGAL_ADDRESS及previous保持；原Value=NULL向量也断言elapsed保持。标准拒绝独立预期从14增加为15，没有削弱原断言。

修复后实际时间套件40向量通过。新增边界是测试宏下的观测/注入入口，生产构建没有该入口；原生事件分配故障使用现有资源故障注入，无新增平台或依赖。

## Verification

`cargo test --manifest-path core/Cargo.toml epic4_controlled_tick_and_alarm -- --exact --nocapture`恰选1项；独立native时间与Alarm向量、前序activation20/finish22/resources25/events26/stack23/lifecycle46；`python scripts/verify.py --scope all --base ca06d169331efd45070f614c4ed7ae6146bc1944`。实际Time/端口宏参数partial Cppcheck/MISRA及源码身份检查，完整4.20规范/偏离仍保持未闭合。全部无头，未通过全部适用出口前不将故事或Epic标done。

最终修复后exact恰选1项通过；完整增量门退出0，29 Python测试、源码卫生/新改行格式/C99、UI lint/build、3 Rust单元和67集成（包括全部前序OS套件）、core/desktop clippy和desktop build全部通过。40 native向量共享证据及partial静态扫描的全部product_sources匹配最终源码；158条静态处置无unresolved，扫描原始退出1及不完整Windows模型、未批准偏离保持如实记录。三路复核已全部complete且发现均已核实处置。沿用C盘构建缓存、dev/test增量关闭，未删除缓存、未启动可见桌面、未推送。4.8核心出口成立；W3、完整SC1、4.20及4.22尚未闭合，Epic仍in-progress。
