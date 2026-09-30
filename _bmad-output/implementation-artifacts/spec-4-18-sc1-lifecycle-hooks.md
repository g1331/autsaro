---
title: '4.18 完整适用生命周期、Extended Status、Hook 与模拟中断'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '81492c568c9d359fd1dc2e0fb406754e383eee7b'
story_key: '4-18-补齐生命周期-extended-status-hook-和中断义务'
context: []
---

<frozen-after-approval reason="Epic4已确认产品意图；用户2026-09-30明确移除额外assurance流程">

## Intent

完成既定单核SC1真实FreeRTOS Win64后端的生命周期、标准服务／类型、两种状态模式、Hook、逻辑ISR和OSEK四一致性类能力。按BMad实现、验证、复核和记录，不另设报告、审批或默认核查矩阵。

## Boundaries & Constraints

保持参考EXTENDED、单核／NONE、固定FreeRTOS版本、唯一backend、真实原生栈及受控逻辑时间；支持显式STANDARD，不重开OS选型。汽车Task／Hook不执行阻塞宿主I/O。四一致性类最低容量、配对／嵌套、清理、调用层级和拒绝输出保持等实际行为不删减。SC2/3/4、实机及Epic2/5/6功能不扩展，不推送。C99和MISRA编码指导用于实际代码；验证和复核写入本工件。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 四一致性类合法容量配置，实际启动、激活、等待、资源和Alarm动作 | BCC1/BCC2分别至少8非挂起Task、8优先级；ECC1/ECC2至少16与16；资源分别1/8/8/8、各2内部资源、1Alarm、1模式；ECC各Task至少8事件能力。实际运行及独立观察证明，不仅检查常量 |
| BCC1/ECC1唯一优先级及单次激活；BCC2/ECC2重复优先级及多次Basic激活 | 独立轨迹符合类规则；适用抢占/非抢占、FIFO、事件和资源能力复用已有效证据并补缺；RES_SCHEDULER显式配置，不凭空创建 |
| Task入口返回，仍持有资源／禁中断／排队激活 | 在离开RUNNING前ErrorHook得到E_OS_MISSINGEND；资源及未配对屏蔽恢复，PostTaskHook和正常终止发生，下一激活或Task真实运行；不能直接整OS关闭 |
| 服务错误、ErrorHook内再次失败、参数宏配置开关 | 标准service ID／准确参数快照与状态可读取，外层快照不被覆盖且不递归；开关配置可追踪；不同层级符合标准许可表 |
| 实际抢占、WaitEvent、Terminate/Chain及显式ShutdownOS | PreTask/PostTask按真实离开／进入RUNNING顺序且Hook查询身份/状态正确；ShutdownOS不调用PostTask；Startup模式与GetActiveApplicationMode一致，成功StartOS不返回 |
| Disable/Enable、嵌套SuspendAll/ResumeAll和SuspendOS/ResumeOS交错 | 保存/恢复原状态，All屏蔽全部可屏蔽应用ISR、OS仅Cat2；无配对恢复无效果；非中断OS服务被忽略并按适用签名返回E_OS_DISABLEDINT |
| Cat2返回时泄漏屏蔽或资源、模拟嵌套ISR | 自动清理并报告E_OS_DISABLEDINT/E_OS_RESOURCE；GetISRID恢复外层；源屏蔽保留pending，ClearPending和Enable(clear)操作真实pending；非法ID/层级无副作用 |
| GetTaskID、GetISRID、isOsStarted、ControlIdle及标准类型 | 无汽车Task/ISR返回规定INVALID值；启动前isOsStarted=false、调用StartOS后true；单核ControlIdle按规范省略CoreID检查，实际支持的host idle动作及无效模式有证据；R24-11公共类型、常量、范围及签名独立编译核对 |

</frozen-after-approval>

## Code Map

- runtime/os/include/Os.h、Os_Target.h、Os_Hooks.h、Os_Types.h及src/：标准服务、类型、上下文、Hook和错误参数。
- runtime/os/src/Os_Backend.c及受控port补丁：真实切换、返回清理、ISR层级与内核状态。
- runtime/os/tests/、scripts/epic4_os.py、core/tests/end_to_end.rs：实际原生行为、容量、错误与拒绝向量。
- core/src/integration/ecu.rs、configuration.rs、schedule.rs及runtime/ecu/templates/：计划、状态模式、Counter常量、标准入口和交付生成。

## Tasks & Acceptance

- [x] 生命周期、missing-end、标准错误与准确参数、Hook上下文和真实转换。
- [x] 中断配对／嵌套、实际pending／源控制、Cat2清理、idle及启动状态。
- [x] 四类最低容量、公共类型、标准构造入口及所选生成Counter常量。
- [x] STANDARD／EXTENDED真实构建、告警、参数、拒绝和生成交付行为。
- [x] 完成实际生成中断向量段和Memory Mapping声明／链接行为。
- [x] 完成Counter OsService端口与ClientServerInterface。
- [x] 完成配置一致性要求。
- [x] 完成剩余具体产品行为、相关回归与BMad复核，同步sprint并本地提交。

## Implementation Notes

实际原生服务和调用表、参数／输出保持、屏蔽清理及同线程嵌套已有测试覆盖；GetISRID在真实ISR帧恢复外层，Hook不改变实际调度。CounterType统一uint32，TickType及TimeInMicrosecondsType为uint64；转换契约为合法Counter值域0..UINT32_MAX。生成器保持唯一验证计划和原输入拒绝规则。

最新增量065dbfa将TASK／ISR／ALARMCALLBACK接入实际配置和调度；11原生向量、8实际实现变异、三配置33常量断言及21头文件变异通过，完整100集成通过。d8ae0c2使STANDARD真实输入可生成、移到含空格新目录构建运行；两模式完整告警、18个Standard容量向量及三个删告警变异通过，完整102集成通过。两次均经BMad三路复核。参考配置仍为EXTENDED。

00336实际可重定位向量、00815实际CODE映射及00560／91027真实Counter服务均已由独立增量实施、验证和BMad复核；后续完成配置一致性并复核完整父story。保留BSD/MIT及固定依赖边界。

## Review Triage Log

- 配置一致性增量：OsOS／OsHooks唯一性、归属、已支持必选字段和标准定义身份已校验；同一计划生成虚拟RES_SCHEDULER，四种布尔表示均经搬移后真实生成Task消费，数值1同名定义替换、三种原生错误配置和供应商字段拒绝通过。BMad三路复核的数值覆盖及定义后缀问题已修正，最终186.80s通过，无递延。105既有集成与修正后的ScheduleTable183.43s、Clippy／格式／C99／15 Python均有效；详见spec-4-18-os-configuration.md。

- Counter服务增量：实际生成标准OsService端口／类型／操作／参数绑定及C服务器和客户端，两个Counter名称在真实Task下通过tick／回绕／七拒绝结果／参数和输出保持；四种编译／行为故障及两种符号／头文件冲突被拒绝，XSD／引用闭合与搬移工程通过。BMad三路复核已完成，两个“补更多ApplicationError”建议由SWS_Os_00560原件及实际公开原生常量驳回，无递延；详情见spec-4-18-counter-service.md。配置一致性继续实施，父story保持in-progress。

- Memory Mapping增量：标准Task／ISR／AlarmCallback及五种Hook通过CODE标记和.os_code实际段验证；10入口、6编译／链接拒绝与真实生成／搬移构建通过。三路BMad复核的OS_CODE覆盖问题已修正；人为私有初始化状态覆盖按low拒绝。详情见spec-4-18-memory-mapping.md，Counter服务仍开放。

- 可重定位向量增量：真实port表已绑定32槽可写.os_vec，生成／搬移工程独立链接检查通过；两种编译故障副本被拒绝。入口11、嵌套26、退出清理10、生命周期46及真实生成测试通过；三路BMad复核无发现。增量规格见spec-4-18-interrupt-vector.md。Memory Mapping和Counter服务仍待实现，父story保持in-progress。

- 公共兼容增量修正复核：同三路blind／edge／verification-gap均已完成最终差异复核且无剩余发现。七个真实头文件变异和六包含顺序已通过，完整符号基准包含local/static及重数；新增正式看门狗在最终源上2.70s通过，实际后代退出。最终完整99门仍运行，未据此提前提交或关闭故事。

- 公共兼容增量首轮blind无发现；verification-gap“仅三名符号黑名单未排除其他输出”为medium／patch：实际static volatile CompatibilityExtra used变异在严格C99编译／运行成功且输出相同，旧名单确实漏检。改为逐包含顺序对同源去除兼容声明的基准对象与实际对象全部已定义符号的名称／类别／重数作比较（包括static），不依赖预先知道多余符号名；新增第七真实变异由这项独立检查拒绝。
- 公共兼容增量edge“新生成消费者GCC／执行无看门狗”为medium／patch：新Command.output确实无期限，可能阻断开发验收。新路径显式编译60s／执行5s，文件采集防止继承管道阻塞，超时终止自有进程树，清理器亦限3s；CREATE_NO_WINDOW保持无桌面干扰。正式epic4_public_consumer_watchdog用实际挂起父／子进程验证诊断保留和后代退出。未把历史其他无期限构建重写为本增量已解决。两项修正须定向复核；完整门与故事出口仍开放。

- 00809源重复调用增量：blind与edge独立复核均完成且无发现。verification-gap唯一发现“unconfigured未执行重复源服务”为false：source_repetition.c最终else实际调用repeated_enable(TRUE)，断言返回E_OS_NOFUNC且源仍启用；该场景Hook指针NULL，随后真实pending递送一次且errors0。原复核者沿实际分支复核后撤回，并确认无其他缺口。不把Python空错误记录列表误读为原生服务返回E_OK。

- 00367非Status服务增量三路独立只读复核完成：blind和verification-gap无发现；edge初次仅返回未完成的空输入读入结果，没有计为通过，要求同一复核者分块读取实际558492字节diff后重新分析，最终返回无发现。当前十原生向量、五个逐函数旧分支编译变异和9／28／5回归通过，330项来源摘要匹配；完整门仍须最终终态确认，不能据此关闭故事。

- 时间生成增量三路只读复核：blind和verification-gap无发现。edge提出“TickType大于UINT32_MAX会导致纳秒溢出”：false（当前生成Counter的合法值域），实际Os_Types.h的TickType为uint64_t，不能以C类型宽度驳回。原生Os_TimeValidate也明确拒绝maximum>UINT32_MAX，生成器的ScheduleContract.counter_maximum来自parse::<u32>()，当前Counter配置最大65535；已校验分辨率为1ms，生成转换宏明确支持0..UINT32_MAX，比全部合法配置Counter值域更宽，最大支持参数的纳秒乘积4294967295000000小于UINT64_MAX。更大的TickType虽可在C中表达，但超出这一生成Counter／转换契约，不能声称已支持任意64位数的纳秒转换。保留真实UINT32_MAX边界断言及上述显式契约，不把uint64类型误记为uint32。本故事尚有产品功能未完成，独立交接仍开放。

- Counter／RTE公共类型增量blind、edge、verification-gap三路独立只读复核均无发现。范围为唯一uint32 Counter定义、uint64时间类型、六包含顺序、17原生拒绝向量、三个实际旧宽度变异及生成离线消费者；不以该增量关闭对应产品功能或完整故事。

- 嵌套ISR增量三路独立复核：blind与edge无发现；verification-gap的旧isr_cleanup ErrorHook内要求Cat2已可递送为medium/patch。OSEK §11.1明确Hook不能被Cat2打断，新Hook gate导致旧断言与实际契约冲突。改为Hook内仍屏蔽Cat2，保留随后真实ISR6 probe、helper和原Task获取/释放全部资源的断言；不删除资源重获和LIFO清理验证。完整门首轮已观察到该旧测试失败，日志保留；修正后须定向复验并完成最终门。

- 第十一补丁／44时间向量／repeat-start修正的追加blind、edge、verification-gap三路复核均无新增发现。最终行为门92集成测试全部通过（567.52秒），Python29、UI lint/build、桌面build与两Clippy通过。随后按既有LF属性归一OS文本字节，原始补丁和上游不变；七套原生回归、三调用错误变体、两旧ISR时序变体及12单元部分扫描均从规范化来源刷新，Python／增量质量复验通过；生成ECU包亦在该来源重编复验。4.18仍有产品工作，保持in-progress。

- 调用上下文/idle增量blind提出“StartOS失败或被忽略后isOsStarted不应为TRUE”：false。已核对R24-11 p205的DRAFT91034，以是否调用过StartOS为返回依据，非Ready或成功；当前入口原子记录调用，屏蔽查询另遵守00093，无两者混淆。edge无发现。verification-gap提出“屏蔽时isOsStarted也应TRUE”：false，p74 SWS_Os_00093要求Task/ISR/Hook自身屏蔽时忽略任何非中断OS服务，DRAFT查询未声明例外；当前返回无效FALSE且ErrorHook报告9，恢复后TRUE，原状态未被改写。两条建议均不修改实现或弱化断言。

- 专项tick探查指出native生产者读取kernel可能有普通Task服务上下文问题：固定portmacro.h将32位tick标为原子，xTaskGetTickCount路径不进入普通critical；去掉该读取与递送前printf的复制实验仍有失败。失败后采集定位到票据已完成而Wait返回仍WAITING的原断言。缓存yield条件的复制端口不能修复；实际dispatcher在释放mutex后才到下一轮清除共享xInsideInterrupt，Task可能先取得mutex并错误跳过critical出口等待。第十一补丁在mutex内完成ISR阶段后才释放；六次对应复制实验全部通过，新增两种消费Task自身屏蔽的确定性握手与原有40/两monitor屏蔽合计44向量通过。复制恢复旧标志时序时两种握手均被原断言拒绝；复制native观察者失败后等待健康controller报告，避免生产者先退出99遮蔽Task断言，产品成功断言未弱化。原失败日志保留，完整嵌套及SC1出口仍开放。

- 当前完整门91通过/1失败为旧repeat-start轨迹ISID；源码确认Startup内重复StartOS应按R24 Table7.1/00088忽略，而非重入原生初始化并增加第二个I。生命周期场景在重复调用返回后核对实际Mode1/DRAFT已调用状态并加R，独立预期ISRD；保留显式Startup随后Shutdown7及资源失败拒绝，46原生向量通过。历史生命周期工件不覆写。

- Cat2资源出口增量blind/edge/verification-gap三路独立复核均无发现。十个真实过程、28屏蔽/26资源回归通过；actual旧backend副本在single/maximum/mixed/unconfigured-mixed四个新独立断言场景均因原断言关闭被拒绝，证明本次测试识别原问题；12翻译单元部分静态诊断保留，不升级完整MISRA声明。

- 中断增量blind/edge/verification-gap三路完成；Cat1屏蔽服务建议按OSEK原文驳回，源上下文及Cat2覆盖缺口已修正。追加两条独立pending/拒绝副作用覆盖缺口已修正，最终定向复核均无剩余发现。五个真实编译变异（Cat2两种清除no-op及三个拒绝操作仍产生副作用）全部被新原生断言拒绝；实际28向量、公共类型三顺序、八ErrorHook配置与最终交付摘要匹配。完整故事仍开放。

- 返回Task/资源增量blind／edge／verification-gap三路独立复核均无发现。七个返回Task、22Finish/Chain和26资源真实向量均通过；Python首轮因仍读取历史finish-chain/resource-preemption记录不符合新标准行为而报红，已改用本次真实回归工件，同时增加旧整OS关闭和隐式scheduler的拒绝变异验证，历史记录不覆写。完整故事中断恢复和SC1仍开放。

- Task Hook增量blind／edge／verification-gap三路独立复核均无发现。首轮完整门86通过/1失败，实际Pre/Post增加诊断标记后，20tick总139标记超出127前缀，trace_dropped12；旧生成测试按未截断片段计数失配。改为独立字面139序列的精确保留前缀和丢弃数量，保持completed20、两个实际CAN输出、全部初始化失败断言；额外只读verification-gap复核无发现。失败原始日志与Windows链接占用日志保留，源实现未因此改动。

- ErrorHook增量edge与verification-gap独立复核均无发现。blind提出“GetTaskID标准service ID应0x04”：false，实际R24-11 OS p167的0x04属于CheckTaskMemoryAccess，p147将GetTaskID列为OSEK服务，OSEK2.2.3 §13.8.3只规定各OSServiceId_xx唯一，没有该数值；当前0x80～0x90与AUTOSAR规定的Counter/Schedule ID分开且独立字面预期检查。保留现有值，不将其他模块/实现常量当作本规范义务。

- 容量独立增量三路blind／edge／verification-gap均未发现可定位缺陷或验证缺口；完整故事的Hook/错误/ISR工作仍开放。正式容量/真实生成两项测试163.73s通过；18容量、40受控时间、43计时向量封存，26最低容量义务直接映射到四个主场景，部分静态exit1原样保留。容量增量完整门exit0：85个集成测试（660.24s）、29Python、UI/桌面构建和两组Clippy通过；本故事Hook/错误/ISR仍未完成，sprint保持in-progress。

## Verification

已运行的正式入口覆盖真实栈／生命周期、激活FIFO、终止／链式移交、资源、事件、受控Counter／ScheduleTable、调用上下文、Hook、ISR清理／嵌套、公共接口、生成ECU和HostBatch。最新两增量完整检查分别100／102核心集成、29Python、UI lint/build、桌面build及两组Clippy通过；对应代码提交065dbfa／d8ae0c2。

本次清理不删减这些行为。后续按具体任务运行覆盖剩余功能的测试和BMad复核，结果更新本节；不因前序通过而把未实现功能标done。

父story最终复核：生命周期、调用层级、两种状态模式、Hook／标准参数、四一致性类容量、ISR与中断配对、标准类型／入口／Counter时间、实际向量段、CODE映射、Counter服务和配置一致性均已由连续开发增量实施并记录BMad复核。最新有效验证覆盖全部106集成（105全量结果＋受影响ScheduleTable修正183.43s＋最终配置186.80s），3单元、15 Python、核心Clippy和增量格式／C99；无新增未验证代码。4.19 ARTI、4.20适用工件及4.21／4.22交接属于后续故事，不提前关闭Epic。按原始baseline对最终父story整体差异进行独立BMad复核；用户移除额外assurance流程的指令优先，删除的旧报告不再构成完成条件。

最终BMad复核：原始baseline至最终代码的blind、edge-case、verification-gap三路均完成且无发现；code-review完整规格模式的Acceptance Auditor亦完成且无发现。四路均只读，实际构建和行为验证沿用上述最终有效结果，无递延项。build同步review后，code-review按无未解决发现的完成条件将本story及sprint设为done；Epic仍in-progress，继续4.19。
