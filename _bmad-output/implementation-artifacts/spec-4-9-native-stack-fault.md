---
title: '4.9 Windows 真实执行栈故障关闭'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: 'cfdb2cd648d97a7ddf1dab553f97f91ef54a5d5b'
story_key: '4-9-捕获-windows-真实执行栈故障并关闭目标'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

基于已验证 4.3 backend，提前闭合 Windows x64/GCC 16.1.0 真实栈故障门。注册任务、隐藏内核任务、模拟 ISR/StartupHook 和关闭控制线程的实际栈；独立子进程触发真实消耗/边界故障并捕获关闭。来源依赖固定，禁止以 fake buffer/flag 算通过；不重开选型，不放行 W3 或声明 SC1。

## Boundaries & Constraints

使用 GetCurrentThreadStackLimits、VirtualQuery、GetThreadContext 和 SetThreadStackGuarantee；记录实际 reserve/commit/guard/SP、配置 reserve 及宿主对齐值。FreeRTOS buffer 单独区分。异常 shim 使用 Windows VEH 与预分配记录，在保证栈内只记录/关门/发关闭提示/永久停驻，不调用 BSW、恢复应用、分配或等待普通锁。独立控制栈执行 ShutdownOS(E_OS_STACKFAULT)/ShutdownHook 与关闭；调度控制线程故障也必须有独立关闭执行者。上层 API 不扩散原生 API，端口仅增加注册/观测/停止门，不更换上下文调度机制。所有故障关闭后哨兵不可执行。测试后台无窗口；正常运行及 4.3 失败路径回归。

## I/O & Edge-Case Matrix

| 场景 | 实际输入 | 独立预期 | 错误处理 |
| --- | --- | --- | --- |
| 正常 | 实际任务/ISR/控制线程注册与 OS 边界/切换 | reserve/commit/guard/SP 档案闭合，任务栈与配置 buffer 不同；不误报 | 正常 E_OK 关闭 |
| 任务溢出 | 任务入口递归消耗原生栈 | 真实 STATUS_STACK_OVERFLOW、fault thread/SP、E_OS_STACKFAULT、ShutdownHook、无 X 哨兵 | 独立控制栈关闭，非零13 |
| 边界破坏 | 原生栈保留区边界实际非法访问 | 真实异常记录指向注册栈边界、无继续调度 | 同上 |
| ISR 溢出 | 实际模拟 ISR 线程递归，持端口 mutex | 同上，关闭不等待该 mutex | 同上 |
| StartupHook 溢出 | Ready 前实际初始化栈消耗 | Failed，真实记录与 Hook，无 Ready/应用入口 | 同上 |
| 控制栈溢出 | ShutdownHook 内实际控制线程栈消耗 | 第二预建立控制栈处理关闭，无故障栈恢复 | 同上 |
| 栈建立错误 | 无合法保证栈/注册失败 | Ready 前 E_OS_STATE 关闭，普通失败不冒充实际栈故障 | 非零关闭 |

</frozen-after-approval>

## Code Map

- `runtime/os/src/Os_Backend.c`：4.3 静态对象、bootstrap、独立控制线程和资源包装；扩展注册/关闭原子发布与控制执行者。
- `runtime/os/patches/0001-controlled-host-lifecycle.patch`：固定端口有限适配；新增第二补丁用于线程 trampoline、切换上下文观测。
- `runtime/os/src/Os.c`、`Os_Backend.h`：标准 OS 边界进入栈检查，内部 hook 类型与故障档案接口。
- `scripts/epic4_os.py`、`core/tests/end_to_end.rs`：保留 4.3 suite，注册独立新 suite，超时/崩溃/无捕获全部失败。

## Tasks & Acceptance

- [x] `runtime/os/src/Os_Stack.c`、`Os_Stack.h`：预分配线程/故障记录，实际区域枚举/SP、保证栈、VEH 分类，普通异常不误称 stackfault。
- [x] `runtime/os/src/Os_Backend.c`、`Os.c`、`Os_Backend.h`、`patches/0002-native-stack.patch`：所有实际汽车/ISR/控制线程注册、调度观测、标准入口检查、无普通锁关闭与双控制栈备用。
- [x] `runtime/os/tests/native_stack.c`：实际任务/ISR/启动/控制溢出和边界故障，哨兵及 ShutdownHook 控制栈地址验证。
- [x] `scripts/epic4_os.py`、`scripts/test_epic4_os.py`、`core/tests/end_to_end.rs`：独立预期、运行/退出/异常记录/线程角色与源摘要，保留普通失败区别。
- [x] `docs/assurance/evidence/epic4/native-stack.json`、`runtime/os/README.md`：实际证据、监测覆盖表及源/补丁绑定，同步 sprint。

**Acceptance Criteria:**
- Given 按目标创建的原生线程，When 注册、切换和服务边界，Then 观测实际区域/SP且与内核 buffer 区分；正常运行不误报。
- Given 实际栈故障，When VEH 接收真实记录，Then 故障栈仅发布预分配记录/停止，独立控制调用标准关闭及 Hook，无普通锁等待/继续调度/故障后哨兵；超时或原生崩溃失败。
- Given 模拟 ISR/初始化/关闭控制栈，When 分别真实消耗，Then 每种角色均有实际故障/关闭证据；全部通过才可把4.9标done，W3其他前置仍闭锁。

## Implementation Notes

- 最终23栈/46生命周期通过；并行与双控制故障额外60子进程重复通过。第二轮实质意见已直接修正并复验；完整质量门核心3/60、UI、桌面均已通过，最终C shim再做定向复验。

- 23 个栈向量通过：六类实际线程递归/初始化/控制故障、真实保留区写入、实际 SetThreadContext.Rsp 破坏、无关异常及六角色保证容量 API 拒绝。真实异常 E 与实际 Context 边界发现 M 分开记录，后者 exception=0。
- 全部新线程在调度前完成注册；主/备用控制 handshake Ready 前完成。恢复前与切换后 GetThreadContext/VirtualQuery 对照，OS/ISR/idle 边界检查。FreeRTOS buffer 地址单列，实际 SP 不在其中。
- 生命周期回归扩展46向量（包括全部24个Create*失败点）；普通初始化失败保持E_OS_STATE。完整质量门的核心3/60、UI与桌面通过，最新定向测试持续补验。
- 编译后二进制 objdump 强制无 emutls helper，PE 原生 TLS；错误与故障记录全预分配。独立测试入口只在 OS_STACK_TESTS 生效，生产未编译测试 callback。

4.3 已提交并通过独立复核/增量质量；起始 commit 固定。按整项授权连续执行，主代理实现，子代理只读复核。

## Spec Change Log

- 复核指出全局故障可能覆盖、关闭 Hook 连续溢出及原生上下文失败分支未验证。收敛非冻结实现决策：每线程预分配槽/独立发布、CAS 保留首次完整记录、按健康执行者选择接管、双控制耗尽的有限最终关闭；再实施该故障子系统并补真实并行/双故障与普通 API 失败向量。KEEP：六角色注册/实际 SP 与区域、PE TLS、受控端口、真实异常/Context 区分、标准接口和现有正常/拒绝向量。未改变冻结用户目标、OS 路线或出口门。

## Review Triage Log

第二轮复核：blind 无新增可达问题；edge 的最终诊断输出失败通过真实关闭stdout→stderr兜底向量修正；gap 的并行断言通过仅测试构建的关闭前屏障等待两次实际异常，严格要求T/D两记录、对应线程及原始代码。gap另称F导致ISRAFD不一致为false：当前生产注入只追加一次F，预期和观测均为ISRAFD，完整suite实际通过。

| 层/发现 | 判定与证据 | 处理 |
| --- | --- | --- |
| blind-1 全局故障覆盖 | high：原全局字段可被第二控制故障覆盖，角色选择可能指向损坏控制线程。 | 每线程故障槽独立发布，CAS 固定首次完整记录，健康选择按实际故障执行者；并行真实故障向量及30次重复验证 |
| blind-2 未检查访问种类 | false：契约认定访问注册栈未提交区域为边界违规；非法读/写均不能合法恢复，不依赖C表达式用途。 | 增加真实boundary-read，原生AV/关闭正确；不扩大到无关地址 |
| blind-3 连续Hook溢出悬停 | high：原主控制被停后，备用再溢出会通知已损坏主控制。 | 立即撤销故障控制健康标记；全部耗尽时有限最终诊断/Exit，真实double-control及30次重复通过 |
| blind-4 Hook重复调用 | medium：正常E_OK Hook被栈故障中断后确实再以E_OS_STACKFAULT在备用调用。第二次也失败时原来无有限出口。 | 明确正常关闭被打断→故障关闭的两次尝试；最终耗尽路径不再调用Hook。记录Hook失败，绝不声称完成；与blind-3同根处理 |
| blind-5 停线程API失败跳Hook | false：停线程使用本模块持续持有的有效句柄，线程入口不退出，当前无已证明可达失效路径。 | 控制停止保留不可恢复兜底；新增三个调度API失败分支独立验证，正常有效控制句柄完成关闭 |
| blind-6 通知event失败 | false：事件创建后不关闭/替换，所有内部句柄固定，不存在输入使其失效的路径；未证明所称坏结果。 | 不以任意宿主API故障假设扩展设计 |
| blind-7 主线程路径观测不全 | false：ISR入口/出口、出站/恢复前上下文及VEH均存在；任意路径真实溢出仍走VEH，不要求在每条C语句轮询SP。 | 持端口mutex真实ISR溢出与恢复前实际Context破坏已覆盖 |
| blind-8 注册失败不能区分 | false：SetThreadStackGuarantee拒绝、VirtualQuery失败均是初始化失败E_OS_STATE，不是未经异常证明的E_OS_STACKFAULT；符合错误初始化矩阵。 | 各角色实际容量API拒绝通过，Fail不冒充捕获 |
| blind-9 注册失败占槽 | false：每线程只注册一次，失败即关闭进程，StartOS禁止重启；没有重复注册耗尽的运行路径。 | 21槽精确容纳16汽车+bootstrap/idle+ISR+双控制，失败未完成记录不输出 |
| blind-10 并行/读缺验证 | medium：确实缺少专门的并行与读向量。 | 实际APC备用与任务并行消耗，逐记录角色/线程/原异常码对照；实际保留区读AV验证 |
| edge 层 | 无发现。 | 独立边界复核完成 |
| gap 上下文操作失败 | medium：Suspend/GetContext/Resume的新错误分支原来只有成功/坏Rsp结果，无失败返回覆盖。 | 三个测试构建的单次普通失败向量验证ISRAFD、E_OS_STATE、闭锁、无应用继续；不计真实栈故障 |


## Design Notes

关闭控制线程互为故障备用执行者；每线程预分配故障槽与首次完整记录分开。两控制Hook均故障时必须有限最终Exit，不重入Hook，最小预分配诊断提示不使用CRT/普通锁/分配。故障记录与停止门独立于端口 mutex。真实故障线程不可恢复，保持驻留至进程退出。正常线程 trampoline 初始化保证栈并注册后才进入内核任务；主调用线程注册为模拟 ISR，保护真正执行回调的栈。

## Verification

- `python scripts/epic4_os.py --suite stack --evidence docs/assurance/evidence/epic4/native-stack.json`
- `cargo test --manifest-path core/Cargo.toml epic4_ -- --nocapture`
- `python scripts/verify.py --scope all --base cfdb2cd648d97a7ddf1dab553f97f91ef54a5d5b`

- 最终验证：23栈向量/46生命周期回归与两个注册的Rust集成入口通过；质量/assurance通过。全量增量门核心3/60、UI lint/build、桌面build/clippy通过。真实栈早期关口PASS；其他W1/W2及完整SC1/交接仍未通过，Epic保持in-progress。
