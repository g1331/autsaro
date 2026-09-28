# Epic 4 Windows OS 基础

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
