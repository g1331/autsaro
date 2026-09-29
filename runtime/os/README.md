# Epic 4 Windows OS 基础

4.18的Task Hook增量接入真实内核转换：第八受控补丁在既有FIFO选择器确定候选后、赋值`pxCurrentTCB`前观察旧/新对象，不增设选择器或修改ready列表。抢占的PostTaskHook在旧Task仍为RUNNING时调用，PreTaskHook在新Task已被内核选中且内部资源取得后调用。WaitEvent、TerminateTask和ChainTask在改写状态/激活队列前发Post，随后内核转换不重复发Post；同一TCB的排队激活/自身Chain仍有新激活的Pre。无切换yield、已满足的WaitEvent不发额外Hook，ShutdownOS不发Post。`python scripts/epic4_os.py --suite task-hooks`用八个独立进程验证字面顺序、实际身份/RUNNING状态与内部资源，正式入口为`epic4_real_task_hook_transitions`。missing-end、中断控制和完整SC1调用表仍开放。

4.18的错误报告增量在25个现有标准StatusType服务边界调用可选ErrorHook，提供有类型的service ID与参数快照。`Os_TargetConfig.hooks`为NULL时不调用标准Hook；配置`Os_HookConfig.error`后，Task、Cat2及Startup内的服务错误在原执行上下文报告。ErrorHook内再次失败不递归、不覆盖外层快照，输出参数在服务拒绝时保持原值。`Os_Cfg.h`默认启用`OS_USE_GET_SERVICE_ID`与`OS_USE_PARAMETER_ACCESS`，可在编译时分别设置0/1；对应标准访问宏在禁用时不定义。固定生产ECUC的两项开关为true，生成交付包含`Os_Types.h`、`Os_Cfg.h`、`Os_Hooks.h`、`Os_Error.c`和`Ecu_OsHooks.c`。`python scripts/epic4_os.py --suite error-hooks`运行四种开关各配置/未配置八个独立进程，正式入口为`epic4_standard_error_hook_parameters`。PreTask/PostTask回调的真实任务转换另有独立增量验证，missing-end及完整中断控制仍待实现，4.18与完整SC1出口保持开放。

4.18的容量增量使用真实BCC1／BCC2／ECC1／ECC2配置运行最低任务、优先级、资源、内部资源、Alarm、模式和事件能力；`python scripts/epic4_os.py --suite capacity`执行18个独立轨迹及拒绝向量，正式入口为`epic4_sc1_class_capacity`。BCC配置只包含Basic Task。静态`Os_TimeConfig.wake_event=0`与`owner=INVALID_TASK`选择标准软件Counter/Alarm使用，不创建私有受控tick确认通道；此配置的硬件Counter拒绝。非零wake_event仍要求AUTOSTART Extended owner，继续使用既有实际内核受控tick/完成确认，不改变参考ECU的时间协议。完整错误、Hook、ISR和SC1出口仍未完成。

4.17在同一后端增加有界Counter/ScheduleTable计时：最多八Counter、八表、每表32个ExpiryPoint及每点16个动作。表提供标准Rel／Abs启动、Stop、Next和Status接口，支持单次／重复、当前绝对值完整回绕、NEXT替换/分离及同点先激活后设事件。同步策略限已确认NONE；其他策略在建立线程前拒绝。软件Counter增量Alarm采用最多八帧的迭代工作栈，配置增量环拒绝，不使用递归或额外调度线程。宿主定时器Counter由实际受控内核tick ISR驱动；无MCU计时器声明。

`python scripts/epic4_os.py --suite sc1-timing`运行独立容量、状态、拒绝和宿主Counter向量；`epic4_sc1_timing_capacity`同时运行真实RTE表配置、外部消费者及周期编辑后的CAN/DID字节。完整SC1等级及所有编码/ARTI/交接义务仍以后续出口为准，不能仅据这些向量升级声明。

本目录是独立新目标的生产基础，Story 4.3 生命周期及 Story 4.9 真实栈路径已完成独立复核；Story 4.4 提供请求 FIFO 与实际内核状态。未接入 ECU/RTE/BSW，不声明 SC1，也不放行 W3。旧 `runtime/src/Os.c` 继续用于历史主机目标，两者不能同链接。

## 固定来源与离线复建

`third_party/freertos/source-manifest.json` 固定 FreeRTOS V11.3.1 / commit `054e14f3397023aa83813a65aa065fc4597d481b`，归档 SHA-256 `dd832f699e6ebee00366d7c737e259860d9d1fd5f2c2311c90dff16e8cfbcf61`，并记录最小源码闭包的原始字节摘要和 MIT 许可。上游原件未格式化；许可与版权通知保留。构建核对全部源码，复制到临时目录并应用 `patches/0001-controlled-host-lifecycle.patch`，不修改原件、不下载依赖。摘要用于完整性复核，不是签名认证。

原生 Windows x64，MSYS2 Rev5 GCC 16.1.0 / `x86_64-w64-mingw32`，C99；使用现有 GCC/Windows SDK 头文件及系统库。其他编译器、32 位和 MCU 未验证。

```powershell
python scripts/epic4_os.py --suite lifecycle
cargo test --manifest-path core/Cargo.toml epic4_backend_lifecycle -- --nocapture
python scripts/epic4_os.py --suite stack
```

## 责任与资源

`Os.h` 是上层生命周期及状态入口；`Os_Target.h` 单独定义静态目标配置和受控测试 trace。汽车对象最多 16 个，优先级 1–30，模式 1/2；宿主保留栈为独立实现字段，64 KiB 对齐，64 KiB–16 MiB，不把 FreeRTOS buffer 当作执行栈。

唯一 backend 持有静态 TCB/配置 buffer，FreeRTOS 选择 Ready/Running 和上下文。隐藏 FreeRTOS bootstrap（优先级 31）执行 StartupHook、发布 Ready、启用 mode AUTOSTART 后挂起；idle 优先级 0，其内部 Hook 只检查实际栈。两个原生控制线程等待关闭 mailbox，互为单栈故障的备用，在独立 256 KiB reserve 栈执行 ShutdownHook、输出记录并 ExitProcess；不选择任务，不调用汽车 BSW。StartOS 正常不返回；正常/错误关闭均不能恢复任务。建立健康控制资源之前的拒绝/创建失败在启动调用者执行关闭 Hook；健康控制线程建立后在独立控制栈执行。报告的 lifecycle=Closed 为终态，state 为最后初始化结果。资源回收由目标进程退出完成，当前不支持进程内重启。

4.3 历史基准为 5 个新线程、6 个 event、1 个 mutex。4.9 的两汽车任务基准为 6 个新线程（4 个内核、2 个控制）、17 个 event（4 个 yield、8 个线程注册/gate、2 个关闭、2 个控制注册、1 个模拟中断）、1 个模拟中断 mutex；另复制一个主线程 handle。原始调用 StartOS 的主线程承担端口模拟 ISR，实际主栈容量由链接器设置、单独观测，不继承汽车任务 reserve 字段。静态内核对象不代表 Windows 无堆分配；原生线程/事件/mutex/系统栈及复制句柄由 Windows 建立。全部 24 个 Create* 点逐一失败注入；控制与任务注册完成后才能 Ready。任务及 Hook 不执行宿主阻塞 I/O；报告只在独立控制线程输出。输出成功不是 CAN/HostBatch 交付证明。

## 生产补丁边界

端口补丁保留上游原有线程上下文机制和优先级调度选择；改为正常宿主优先级，移除墙钟 timer/首次 tick，首次只 yield。创建栈按目标 reserve，所有创建经过可审阅失败检查/计数，关闭门阻止继续 resume。第三补丁在复制件中增加有限 ready 排序政策，见下方 4.4 契约。完整 Finish/Chain、资源及事件属于 4.5–4.7。4.8 再提供逐毫秒受控 ticket；当前无输入/tick 驱动。

`AUTOSAR_OS_FAIL_RESOURCE` 为独立 harness 的原生资源建立失败注入入口：正整数 N 表示第 N 次 CreateEvent/CreateMutex/CreateThread 返回失败，随后目标关闭。畸形、零、负值及溢出输入以 E_OS_VALUE 拒绝。正常部署不设置；测试器显式清除继承值。每个实际建立点均有独立子进程向量。

## 验收边界

4.3 历史证据 `docs/assurance/evidence/epic4/backend-lifecycle.json` 保留原 commit 的 34 向量与摘要；4.9 修改后的生命周期回归单独保存于 `backend-lifecycle-stack-regression.json`，46 向量，包含两种模式、多 ready 优先级和全部新资源建立失败点。受控 tick、完整 Finish/Chain、Extended Status/Hook/ISR、Counter/ScheduleTable、ARTI、等级及交接仍待完成。

## Windows 真实栈（4.9）

第二端口补丁只增加原生线程注册 handshake、切换 GetThreadContext/真实 SP 检查、模拟 ISR 入口/出口检查以及 Suspend/Resume 失败检查；上游上下文/就绪调度机制保持。每个内核线程先初始化真实栈保证、枚举 VirtualQuery reserve/commit/guard、记录 GetCurrentThreadStackLimits，再在 gate 等待，被同步挂起后才作为端口线程交付。观察记录同时列出 FreeRTOS buffer 地址；它是静态 ThreadState 容器，不是实际执行栈。

VEH 只处理实际 STATUS_STACK_OVERFLOW，或访问本线程注册时未提交的真实栈保留区所产生的 ACCESS_VIOLATION。保证栈请求 16 KiB，记录实际返回容量。切换时对出站和将恢复线程均检查实际 Context.Rsp/VirtualQuery 区域；M 来源记录真实线程上下文边界发现，exception=0，不能冒充 Windows 异常。E 来源保留原始 Windows exception code。故障路径仅写预分配记录、原子关门、发布 Win32 关闭事件并永久停驻，不等待故障线程持有的端口普通 mutex，不分配、展开或恢复应用。控制线程先停止 ISR 所在线程并用 GetThreadContext 确认同步挂起，再停止任务；由健康控制栈调用 ShutdownOS(E_OS_STACKFAULT)/ShutdownHook 并关闭进程。主控制栈故障交备用，备用故障交主控制；每线程故障槽独立发布，首次完整故障以 CAS 固定，后续故障不覆盖；健康控制选择使用实际故障执行者与健康标记，不依赖被覆盖的角色。若两控制栈均在 Hook 内损坏，保证栈上的最终 shim 仅输出预分配的最小诊断提示（stdout写失败/短写转stderr）并 ExitProcess(E_OS_STACKFAULT)，不重复 Hook、等待普通锁、调用 CRT 或恢复应用。此时两个配置 Hook 已尝试并失败，报告 controllers_exhausted=1；不能宣称 Hook 已完成。

| 实际线程角色 | 注册/正常观测 | 实际故障子进程 |
| --- | --- | --- |
| T 汽车任务 | 目标 reserve、实际 SP/区域、buffer 区分、OS 边界 | 递归溢出、未提交栈区写入、真实线程 Context.Rsp 破坏后恢复前拒绝 |
| B bootstrap/StartupHook | Ready 前注册与切换 | StartupHook 递归溢出，无 Ready |
| I idle | 静态内核线程、内部边界检查 | 实际 idle Hook 递归溢出 |
| S 模拟 ISR/主线程 | 主栈档案、ISR 入口/出口 | 持模拟中断 mutex 时真实溢出 |
| C 主关闭控制线程 | Ready 前注册、独立控制栈 | ShutdownHook 真溢出，D 接管 |
| D 备用控制线程 | Ready 前注册、独立控制栈 | 测试 APC 真溢出，C 接管 |

`native-stack.json` 保存 23 向量：正常、保留区读/写、六角色真实溢出与实际 Context 破坏、并行故障、连续控制故障及其stdout失败兜底、三类上下文操作失败、无关异常分类、6 类真实栈保证 API 拒绝。超时、未捕获的应处理故障、错误关闭码、故障后 X 哨兵均失败。所有角色的保证初始化失败在 Ready 前 E_OS_STATE 关闭，不能冒充 E_OS_STACKFAULT 证据。`AUTOSAR_OS_BAD_GUARANTEE=<角色>` 仅用于 verifier 请求超出实际 reserve 的容量，观察 SetThreadStackGuarantee 的真实失败；verifier 清除继承注入值。`OS_STACK_TESTS` 只为独立 harness 提供 backup handle/idle 消耗测试入口，生产构建不含这些入口。上下文操作失败向量仅在测试构建强制一次失败返回、保留有效句柄，以验证停止/关闭分支，明确不是实际栈故障或新的能力声明。编译后二进制未链接 emutls helper；异常路径 TLS 不依赖首次动态分配。能力档案不因这组证据升级，W3 仍要求其他 W1/W2 全部前置通过。


## 完整激活请求 FIFO（4.4）

静态任务声明种类及激活上限：Basic 为 1–32，Extended 固定为 1；每个成功接纳的请求占用独立静态环槽，当前实例也计入上限。ActivateTask 在同一事务内核验容量、保存请求、更新实际内核队列，并在合法边界请求已有内核调度。拒绝不消费序号、不改请求或运行状态。GetTaskState 读取实际 TCB 状态；NULL 输出按 SWS_Os_00566 返回 E_OS_ILLEGAL_ADDRESS。

第三补丁按最早存活请求排序实际 FreeRTOS ready 列表，同优先级始终选头部；抢占后恢复同一原生帧。序号是可原子压缩的存活顺序标记，不是永久唯一 ID；达到 uint64 上界时仅压缩元数据并更新内核键，保留完整槽和相对顺序。该政策要求固定单核 generic selector 且关闭时间片，不支持其他 selector。最小 TerminateTask 使用私有、同线程的 setjmp/longjmp 根帧重新进入下一实例，成功不会返回旧应用帧；完整 Chain、资源拒绝和事件唤醒语义按后续 stories 闭合。

Windows 模拟 ISR 已持有端口递归 interrupt-event mutex；服务临界区可在该固定端口嵌套取得，退出借 xInsideInterrupt 避免任务级等待。ISR 激活使用 xTaskResumeFromISR，返回布尔值只表示需要切换，由 backend 在 ISR 出口请求原选择器。没有 MCU 中断安全性声明。请求接纳与关闭通过同一 Interlocked 状态字线性化：关闭赢得原子门后不写新请求；接纳先赢得门的请求属于已接纳前缀，关闭后保留其记录，但不得执行或正常返回应用。关闭位永久保留，关闭控制不等待在途事务或普通 mutex。服务仍在入口、事务内及返回边界核验。测试注入的 100 ms 正常关闭延迟仅存在于 OS_ACTIVATION_TESTS，生产和真实栈故障路径均没有该延迟。

```powershell
python scripts/epic4_os.py --suite activation
cargo test --manifest-path core/Cargo.toml epic4_activation_fifo -- --exact --nocapture
```

20 个实际子进程向量覆盖 AAB/ABA/AABB/ABAB、抢占帧保留、Basic/Extended 拒绝、32 请求容量与 48 次环回绕、序号压缩、同级 autostart、ISR 成功/拒绝以及关闭竞争。独立 observer 只读已缓存 TCB 的真实状态/排序键并核对完整请求档案，不轮询选择 runnable。未来 Wait/Wake 的 ready 位置必须由 4.7 单独证明。

新共享证据明确标记为路径归一化表示，原始结构化记录位于忽略的 .scratch/epic4，并关联其 SHA-256；保留诊断所需的真实栈地址和线程 ID。activation-static-analysis.json 保留实际扫描命令、源码身份和完整诊断。Cppcheck Windows 模型和 MISRA 修订覆盖不完整，扫描仍报红；新增 backend 根帧及主机 I/O 的偏离尚未批准，4.20 完整质量出口仍须闭合。


## 原子 Finish/Chain 与入口（4.5）

TerminateTask 和 ChainTask 共用先验证后提交的完成事务。持有外部资源、目标无效/已满或调用层级不允许时，源/目标请求、事件、资源所有权、有效优先级及 ready 位置全部保留。自链先完成当前实例，再按新的请求位置接续；即使处于激活上限也不额外占槽。Basic 已排队的实例保持 FIFO，Extended 新实例（含自链）清空存储事件。所有成功路径进入同原生线程私有根帧，局部变量重新初始化，不返回旧应用调用。

静态资源配置最多8个，声明 id、ceiling 和按 TaskID 位映射的 task_access。当前外部资源核心以真实所有权/LIFO及内核有效优先级支持持资源拒绝，绝不阻塞等待；4.6 再闭合 NON/内部资源、完整恢复位置和ISR资源。SetEvent/GetEvent 当前支持活动 Extended 任务的存储与查询，4.7 再完成 Clear/Wait 与无丢失唤醒；当前没有可达 Waiting 任务。直接返回入口仍最低失败关闭 E_OS_STATE，完整 missing-end/Hook 契约由4.18负责。

标准 OS SetEvent 与 Win32 SetEvent 同名。公共服务宏映射到 Os_SetEvent；私有 Os_Windows.h 暂时移除该宏导入 Windows 声明后恢复，原生事件信号通过独立 Os_HostSetEvent 翻译单元调用真正 Win32 API。汽车应用仅包含公开 OS 头文件并调用标准服务；使用 Windows API 的原生桥必须经 Os_Windows.h 适配边界导入。第四复制件补丁只替换端口的 Win32 信号调用，不修改选择器、上下文或上游原件。

```powershell
python scripts/epic4_os.py --suite finish
cargo test --manifest-path core/Cargo.toml epic4_finish_chain_atomicity -- --exact --nocapture
```

22 个原生场景覆盖满容量自链、pending、优先级链/同级请求、拒绝完整快照、实际资源ceiling、事件重置、Chain前/后夹点的实际ISR与额外激活、missing-end最低关闭和6种资源配置拒绝。ISR在普通临界区退出后执行，只观察完整状态；完成旧A实例的 order3 被移除，Chain目标B order4 先于ISR重新激活A的 order5。Observer读实际TCB状态/排序键，资源和事件快照均来自实际核心。静态分析仍是部分覆盖，后续完整质量出口和偏离审批要求保持。

静态外部资源 ceiling 按 OSEK §8.5 校验：不低于所有访问任务的最高优先级，并低于不访问该资源、且优先级高于最高访问者的任务。资源测试声明 A/H 为访问者，验证过低或过高 ceiling 在创建线程前拒绝。


## 混合抢占与资源（4.6）

每个 Task 声明 FULL/NON 和可选内部资源，静态配置支持两组内部资源。唯一内核先按真实就绪队列选出 Running，backend 再为该已选 Task 取得内部 ceiling；激活不会预先抬高未运行任务。NON 使用最高汽车任务优先级的隐式内部资源；抢占保留内部持有，Schedule、真正等待和成功结束释放，恢复运行时重新取得。外部资源所有权/LIFO独立记录，持外部资源拒绝这些释放边界。

WaitEvent 使用一个完整临界事务发布谓词、释放内部资源并进入真实内核 suspended-list；存活激活和等待标记将该物理停驻明确映射为汽车 WAITING。SetEvent满足谓词后更新独立 ready位置并恢复同一原生实例；激活FIFO记录仍保留，普通唤醒不是新激活。已设置事件的Wait不释放内部资源。完整事件所有权、输入发布/等待竞态及唤醒顺序仍由4.7闭合，不以notification默认行为作等价证明。

外部资源共8槽，预定义 RES_SCHEDULER 使用保留的第8槽，未显式配置时自动可用；它仅阻止其他Task抢占，IRQ仍能交付。显式配置保留槽必须覆盖全部Task、使用最高汽车Task优先级且不得分配ISR访问。普通资源声明task_access与isr_access，当前固定Category2模拟组虚拟优先级31；共享资源以实际内核有效优先级与端口待处理IRQ屏蔽维护ceiling。ISR另有真实所有权/LIFO，离开handler前必须释放，不借用被中断Task的资源栈。完整类别/嵌套/中断义务仍归4.18。

第五复制件补丁只在现有端口边界增加资源屏蔽、真实ISR身份和完整ISR期间的同步Suspend/GetThreadContext/Resume。pending在handler前消费，屏蔽时保持，回调重投的边不被尾部清除。使用既有上下文机制和唯一kernel selector；没有新调度器或Windows实时优先级。外部IRQ注入场景在ISR观察区间睡眠50ms并核对应用原子进度未增加，直接证明应用线程整个区间停驻，避免同宿主普通优先级下仅在切换时挂起造成并行执行。

```powershell
python scripts/epic4_os.py --suite resources
cargo test --manifest-path core/Cargo.toml epic4_resource_and_preemption -- --exact --nocapture
```

当前25个原生场景覆盖FULL/NON、两组内部ceiling的真实升降、Schedule/阻塞/已设置事件、嵌套和同ceiling恢复、拒绝快照、Task/ISR共享屏蔽、外部IRQ物理停驻、RES_SCHEDULER，以及8种静态配置拒绝。独立4.2的NON调度和ceiling释放条件以其原始优先级单独运行。原件与历史证据保留，新的 partial静态扫描覆盖实际复制件端口；诊断与完整质量出口继续如实记录。

## 事件与输入发布（4.7）

事件身份为 Task 与 mask，Wait 不消费存储位，Clear 只清除调用 Extended Task 的位；新实例（含自链）清零。Category1 由目标 category1_isrs 明确指定，拒绝汽车事件/资源核心服务；Category2 可 Set/Get 和使用声明的资源，但不能 Wait/Clear。Category1 不受 Category2 资源 ceiling 屏蔽。完整 Hook、中断暂停和嵌套仍须4.18验证。

可选 input_task/input_event 将唯一输入消费者绑定到 autostart Extended Task；input_event=0关闭该入口。启用时专用模拟IRQ30由 backend保留，不能同时声明Category1或替换其handler。`Os_Mailbox.h` 提供32字节复制记录、单调uint64 ticket和256槽输入队列。一个持续存活的原生bridge线程是唯一生产者，配置Task是唯一消费者；其他生产者/Task拒绝。生产者先完整复制槽，再通过Windows Interlocked发布计数，最后pending模拟ISR，ISR只设置事件。队列满和ticket耗尽在写槽之前拒绝，输出ticket保持；消费者按ticket顺序复制后释放槽。汽车Task和ISR不能从bridge接口发布，bridge不能直接调用汽车事件/消费接口。

消费循环使用GetEvent→Clear已观察位→排空完整记录→再次检查队列/事件→Wait。事件合并不合并记录或确认数；Get/Clear之间的通知即使被Clear，记录仍在队列并被排空。判空后发布的记录由随后存储的事件唤醒实际等待Task。队列与事件分别承担数据及唤醒责任，不能只凭事件位计数。ticket只代表记录接纳，业务完成、输出成功和HostBatch静止点仍由4.8/4.14集成闭合。

```powershell
python scripts/epic4_os.py --suite events
cargo test --manifest-path core/Cargo.toml epic4_event_wakeup_races -- --exact --nocapture
```

26个独立原生向量覆盖连续Wait、所有权、新实例、标准错误、Category1/2调用矩阵、Category1在共享资源ceiling期间仍能交付、五个发布/等待交错、256槽容量、522条环绕、最后uint64 ticket和后续拒绝、bridge/消费者接口拒绝及5个配置拒绝。另验证接收Task未在所选mode中autostart时，在创建线程之前拒绝启动。第六复制件补丁维护IRQ30的handler所有权，启动期或运行期替换均以E_OS_ACCESS关闭，未接纳输入、未继续应用；原件不改。控制生产者通过实际Windows线程/IRQ接线；观察者读取真实TCB状态，只有测试握手使用宿主等待，不替代内核选择。MISRA仍为partial扫描，完整质量和偏离批准不能由这些行为测试替代。

## 受控时间与Counter/Alarm（4.8）

可选Time配置提供独立Counter/Alarm对象及唯一完成owner。汽车TickType为uint64，Counter最大UINT32_MAX；FreeRTOS实际tick仍为32bit，逻辑epoch为uint64。SystemCounter按原创ECUC保持SOFTWARE，参考max65535/mincycle1/ticksperbase1。标准IncrementCounter可独立驱动软件Counter，硬件Counter拒绝该服务；GetElapsedValue按单个模数返回差值，不能识别经过多轮模数的间隔。

原生bridge以Os_TargetAdvanceOneTick请求恰好下一个epoch；epoch0和最后已确认epoch是无推进操作，跳跃/倒退或未确认下一请求拒绝。第七复制件补丁在原有tick ISR前取得唯一pending请求门；实际xTaskIncrementTick触发backend tick hook，推进SystemCounter、按静态配置顺序执行Alarm，并设置wake_event（参考可用Ev_IO）。IRQ1不能被替换。Alarm_Work设置Ev_Work，Alarm_App设置Ev_App，wake_event仅让owner处理ticket，不代表额外BSW周期。

owner通过Os_TargetCurrentTick取得已交付ticket，处理事件/工作后调用Os_TargetCompleteTick标记，再清空已观察事件并进入Wait。只有真实内核suspended-list成员、无待处理事件、输入mailbox无已发布或在途记录时，才发布完整Os_TickCompletion。原生bridge用Os_TargetWaitTick等待手动完成event；通知先成功、结果后发布，event保持到同一producer预留下个ticket时重置，避免通知与发布交错丢失。watchdog只控制宿主存活，不产生汽车时间。完成event的实际重置/通知失败会关闭目标并报告time_signal_failed=1，不返回完成记录。

Alarm支持相对/绝对/周期/取消、ActivateTask/SetEvent/Callback核心。绝对start等于当前Counter值时等待完整模数；宽TickType可表示最大Counter的4294967296剩余量。动作错误调用已配置error_hook，IncrementCounter自身仍E_OK。完整ErrorHook身份、调用矩阵、递归抑制及服务参数访问由4.18闭合。

```powershell
python scripts/epic4_os.py --suite time
cargo test --manifest-path core/Cargo.toml epic4_controlled_tick_and_alarm -- --exact --nocapture
```

40个原生向量覆盖1000次独立完成（1000 Work/100 App）、Counter与实际内核独立回绕、uint64上界、未确认推进、4.2相对/绝对Alarm预期、完整模数/最大剩余量、动作失败和回调、标准错误及Category1/2、21种配置拒绝、真实完成句柄失败和IRQ1替换拒绝。另验证预留至pending间关闭时拒绝且ticket保持、Time启用时完成event分配失败在任何线程创建前关闭。初始实际kernel计数和关闭边界仅在OS_TIME_TESTS入口注入，随后真正运行原内核跨界；生产配置没有该入口。该范围是时间核心行为证明，完整SC1、规范质量与集成交接仍须以下出口闭合。

| 尚未闭合的等级/集成义务 | 出口 |
| --- | --- |
| 八software Counter容量、两个ScheduleTable及Alarm IncrementCounter动作/交互 | 4.17独立容量与语义向量 |
| 完整生命周期、Extended Status、Hook/中断上下文和嵌套 | 4.18完整义务矩阵 |
| ARTI描述/Hook、完整C质量和偏离批准 | 4.19/4.20 |
| BSW/SWC周期、输出成功/确认和HostBatch COMMIT静止点 | 4.14–16跨story集成 |
| 可重建包、完整SC1等级声明及独立交接 | 4.21/4.22 |
