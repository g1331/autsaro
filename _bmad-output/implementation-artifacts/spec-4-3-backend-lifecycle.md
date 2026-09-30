---
title: '4.3 固定内核 backend 与静态生命周期'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'a6429d4017547b1ada108688133aaaff07e85466'
story_key: '4-3-建立固定内核的唯一-backend-与静态生命周期基础'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

建立固定 FreeRTOS 的生产基础，为 W1 汽车 OS 语义和提前进行的 4.9 提供真实线程与内核调度环境。交付独立 C99 生命周期 harness、唯一 backend、静态对象、来源/补丁/ABI 档案；不声称 SC1 已通过。用户已授权逐 story 规格、实现、验证及本地提交，无逐条确认。

## Boundaries & Constraints

固定 V11.3.1 / `054e14f3397023aa83813a65aa065fc4597d481b`、Windows x64/GCC 16.1.0。保留原始 MIT 源码/摘要，构建只应用可审阅的端口补丁。上层仅标准 OS API；FreeRTOS 保持唯一 ready/Running/上下文所有者。汽车对象静态配置，原生资源分配明确记录。正常优先级后台运行，禁止 GUI/抢焦点。旧 runtime 不改，不推送，不扩展 Epic 2/5/6。任务入口及 Hook 不执行宿主阻塞 I/O；关闭记录由独立控制线程完成。正常启动的 StartOS 不返回；失败关闭并拒绝后续服务/输入/tick。完整 Hook/错误/激活/时间/栈能力分别留给后续故事，不能据此放行 W3。

## I/O & Edge-Case Matrix

| 场景 | 输入 | 独立预期 | 失败处理 |
| --- | --- | --- | --- |
| 正常 | 两任务、mode 1、选择性 autostart | Initializing→StartupHook→Ready→唯一 autostart 任务 Running→ShutdownHook；StartOS 后代码不执行 | 进程成功关闭 |
| 另一模式 | mode 2 | 只启动 mode 2 的任务，未启动任务为 SUSPENDED | 同上 |
| 配置拒绝 | 空任务/越容量/零或超限优先级/重复 ID/空入口/非法模式 | 任务开始前 Failed，E_OS_ID 或 E_OS_VALUE；无 Ready | 非零关闭 |
| 非法服务 | Ready 中查询非法任务或空输出 | E_OS_ID/E_OS_VALUE；有效任务状态不变 | 不终止当前合法任务 |
| 建立失败 | 每个实际 CreateEvent/CreateMutex/CreateThread 点注入失败 | Failed、E_OS_STATE、无任务入口或 Ready；控制路径/兜底关闭 | 非零关闭，无正常 StartOS 返回 |
| 依赖错误 | 缺源码/摘要或编译器 ABI 不符 | 构建前明确拒绝 | 不执行旧二进制 |

</frozen-after-approval>

## Code Map

- `runtime/src/Os.c`：旧 Os_Advance 轮询；原目标保持。
- `core/src/generator.rs`：旧生成/构建；新集成到 4.13 才接入。
- `_bmad-output/planning-artifacts/architecture/epic-4/feasibility/source-manifest.json`：固定身份及已核查 MIT。
- 上游 `portable/MSVC-MingW/port.c`：原线程/事件和切换；配置 buffer 只存 ThreadState，实际栈独立。原墙钟 timer/实时优先级必须在新目标禁用。

## Tasks & Acceptance

**Execution:**
- [x] `third_party/freertos/`：保留最小离线源码闭包、MIT、逐文件摘要及来源身份；原件不格式化。
- [x] `runtime/os/include/Os.h`、`Os_Target.h`：标准生命周期/状态接口与独立目标配置；目标容量和 host reserve 分开。
- [x] `runtime/os/src/Os.c`、`Os_Backend.c`：静态对象、bootstrap、mode/autostart、真实状态映射、独立控制关闭，拒绝重复启动/非法对象。
- [x] `runtime/os/FreeRTOSConfig.h`、`runtime/os/patches/`：静态内核、无 timer daemon/mutex、有限端口适配及正常宿主优先级，不引入调度选择器。
- [x] `core/tests/end_to_end.rs`、`scripts/epic4_os.py`、`runtime/os/tests/`：注册 epic4_backend_lifecycle，固定独立正反预期，离线构建/实际执行/资源失败与依赖拒绝。
- [x] `runtime/os/README.md`、本 story 的验证记录：真实来源/ABI、补丁职责、隐藏任务/原生资源及验收结果，同步 sprint。

**Acceptance Criteria:**
- Given 固定源码/工具，When 离线编译执行两种启动模式，Then trace 符合上述预期且内核报告唯一 Running，标准上层不依赖 Windows/FreeRTOS API。
- Given 启动/对象/资源建立错误，When 执行对应向量，Then 无 Ready/任务继续执行，服务返回标准状态、关闭无 StartOS 私有返回值。
- Given 此基础通过，When 查看能力/sprint，Then 只有 4.3 可 done，真实栈/完整 SC1 与 W3 门仍未放行。

## Implementation Notes

- 固定归档摘要及 27 个原始源码文件已核对，原件保留字节并由 gitattributes 禁止换行转换；生产端口补丁与产品源码摘要随实际证据记录。
- 原生生命周期扩展到 34 向量、依赖拒绝 2 测试通过；核心 3 单元/59 集成、UI build/lint、增量源码检查 通过，桌面 build/clippy 也已通过；修正后定向生命周期与质量复验通过。
- Windows 资源实际 5 个新线程、6 个 event、1 个 mutex；全部 12 个创建点逐一失败注入通过。
- 构建会话按 README 使用现有 vcpkg static-md/libclang；没有安装新依赖或全局修改环境。

规格由规划、当前源码及工具核查形成；按本轮整项授权继续，不增加逐条审批。实现由主代理承担，独立复核只读。

## Spec Change Log

- 独立复核的直接修正没有改变冻结目标：增加既有边界的向量、严格故障注入解析、明确终态与资源前 fallback，保持唯一内核/静态对象/无 BSW 和真实栈后续门。

## Review Triage Log

| 层/发现 | 判定与证据 | 处理 |
| --- | --- | --- |
| blind-1 关闭 SetEvent 失败 | false：关闭 event 是本 backend 创建、无任何关闭/替换路径的有效句柄；未证明在契约输入下可达失败。极端宿主故障的立即退出仍不可恢复，不把假设当功能缺陷。 | 不增加未证明分支 |
| blind-2 控制 wait 失败 | false：有效 event/INFINITE 固定 wait，无 invalid handle 路径；审查未给出可达触发。 | 保留兜底不可恢复退出 |
| blind-3 初始拒绝 Hook 栈 | low：实际 bad-mode 在控制资源前拒绝，matrix 已明确兜底；公开注释此前未解释例外。 | 直接明确前资源阶段的 caller fallback，并保留独立控制栈运行路径 |
| blind-4 优先级失败继续 | medium：端口的返回失败分支确实只打印再继续。 | 补丁直接 E_OS_STATE 关闭；固定已应用补丁重编译复验 |
| blind-5 ResumeThread 失败未检查 | false：使用本端口持有的成功创建句柄，当前不允许删除任务/外部改句柄；没有已证明可达失败或因此继续运行的向量。 | 不以任意系统 API 失败假设扩展本 story |
| blind-6 SuspendThread 失败未检查 | false：同上，当前固定内核线程无删除/关闭路径；尚未证明真实坏结果。4.9 会验证故障关闭时所有线程的终止门。 | 后续栈契约按原计划验证，不升级4.3能力 |
| blind-7 单 runnable 验证不足 | medium：原来仅单任务 autostart，缺直接多 ready 状态判据。 | priority 向量同时 AUTOSTART，内核选高优先级 B 并观测 A 为 READY |
| blind-8 配置边界缺测试 | medium：实现正确但 null/auto mask/stack 边界未覆盖。 | 补 null config/array、mask、上下界及保留原非对齐向量 |
| blind-9 关闭状态不清 | low：state 原意是最后初始化结果，与 closed flags 一起表示，但字段不够明确。 | 新增 lifecycle=Closed 并解释 state，ShutdownHook 中直接验证 Ready 门已关闭 |
| blind-10 绝对命令无法重放 | false：证据是实际本机执行调用记录；重建入口是 repo-relative scripts/epic4_os.py，不要求已删除的临时命令可直接重放。 | 保留真实调用路径与明确的离线重建入口 |
| edge-1 畸形故障注入参数 | medium：strtoul 原来忽略 end/overflow，invalid 静默等于零。 | 严格解析并实际验证 invalid/-1/overflow/0 四子进程 E_OS_VALUE |
| edge-2 优先级失败 | medium：与 blind-4 相同真实分支。 | 同一直接修正，独立登记不丢发现 |
| gap-1 启动前 GetTaskState | medium：原测试只在 Ready 调用，未覆盖初始化门。 | 每个正常准备后、StartOS 前检查 E_OS_STATE 且输出值不改；实际执行通过 |


## Design Notes

隐藏 bootstrap 任务只发布初始化/Ready 并启用 AUTOSTART，随后挂起。原生控制线程只拥有生命周期/故障 mailbox，不选择 runnable，不执行汽车工作。ShutdownHook 与记录由独立控制栈执行，ExitProcess 完成目标资源回收；StartOS 不允许返回正常应用路径。4.9 再补实际栈注册、异常 shim 及不可恢复关闭验证。

## Verification

- `cargo test --manifest-path core/Cargo.toml epic4_backend_lifecycle -- --nocapture`：所有独立正反向量通过。
- `python scripts/verify.py --scope all --base a6429d4017547b1ada108688133aaaff07e85466`：增量质量、核心、UI、桌面构建及源码检查 通过；不做交互桌面验收。

- 最终验收：34 个原生向量、2 个依赖拒绝测试、核心 3/59、UI lint/build、桌面 build/clippy、增量 源码检查 全通过。复核直接修正后定向测试通过；未做用户桌面 GUI 验收，本 story 不要求 GUI。
