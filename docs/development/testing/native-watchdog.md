# Linux HostBatch watchdog 取证

用于区分批次期限被调度等待耗尽与运行时等待／握手停滞。先保留失败的完整输入、独立协议向量、ELF、编译命令及 stdout／stderr；协议集成场景目录中的 `stdin` 是实际送入进程的字节。关闭后的栈和状态已经受清理影响，判断首次原因须取得关闭前现场。

## 取得关闭前现场

仅在诊断构建链接 `core/tests/fixtures/host-batch/linux_watchdog_capture.c`，加上三个链接选项：

```text
-Wl,--wrap=Os_BackendShutdown
-Wl,--wrap=Os_BackendRequestShutdown
-Wl,--wrap=Ecu_TargetAbortNative
```

观察器在 reason=7 的外部调用入口、真实关闭函数开始前，以当前线程的 `raise(SIGSTOP)` 暂停线程组；`Ecu_TargetAbortNative(E_OK)` 的实际失败映射也覆盖。它在失败后仅向自己的父进程及其调试器授予 ptrace 权限，保持系统 Yama 设置。使用进程定向的 `kill(getpid(), SIGSTOP)` 会留下调用方先委派关闭的窗口，须用负对照核对暂停位置。

GNU ld 的 `--wrap` 只重定向外部引用。同一编译单元内部的调用可能绕过观察器；因此每次都检查 `Os_Closing=0` 和真实调用栈。捕获失败、字段缺失、已关闭现场均保留原始错误，另行取证。

需要 Linux、GDB、setarch、GCC 及带调试信息的原始构建。取证入口在开始前检查工具；新的输出目录须为空。保留的 `compile-commands.json` 中 `instrumented` 数组来自原生测试构建，源文件路径须仍存在，原来的 TSan 参数保持原样。

```sh
uv run --locked python tests/python/native/watchdog_capture.py \
  --build-receipt /absolute/retained/event-build/compile-commands.json \
  --input /absolute/retained/cases/s3-expired/stdin \
  --output /tmp/host-watchdog-capture
```

也可用 `--binary /absolute/diagnostic/ecu_probe` 重放已经链接观察器的 ELF。入口直接持有子进程 PID，使用独立的进程所有权根管理 GDB 和构建；GDB 从该父进程派生，以满足受限 ptrace 权限。仅回收自己的子进程，不按进程名结束进程。

输出包括 ELF／输入摘要、构建命令、原始流、批次墙钟时间线、约 50 ms 一次的只读 `/proc` 观察，以及暂停后的全部线程栈。冻结快照记录 gate 的逻辑 owner／generation 与原生 mutex、actor 的 requested／acknowledged／resumed、线程信号掩码和 `schedstat`。采样间隔用于观察，不参与被测程序同步。只有冻结的快照具备同时性；每条区间观察同时保存读取开始与结束时间，不能用非原子读取的开始时间推断 watchdog 提前到期。

成功取得关闭前快照后，入口结束诊断进程并返回 7；`result.json` 的 `captured_abort` 表示取证完成，原生程序的正常退出码另行记录。无有效快照的退出 7 会报错。健康运行仍须由原来的协议测试核对全部向量；这个入口仅负责重放与取证。

## 正反对照与负载判别

先用可控的 CPU 竞争制造期限耗尽，验证观察器确实停在委派前，且 `Os_Closing` 仍为 0；再跑同一 ELF、输入、CPU affinity 和并发数的无额外压力对照。`test_watchdog_capture_stops_before_delegation` 编译真实观察器，核对三个入口及零错误码映射；删除暂停或 ptrace 设置后都失去原有的关闭前观察，正常关闭入口继续委派。

记录每组实际进程数、亲和性、nice 值、CPU 压力 PID、失败次数／总次数及各批次时间。未改动的原始 ELF 也应在相同压力下对照，排除观察器引入失败。调度证据看线程的实际运行时间与可运行队列等待增量，勿仅据主机 load average、总耗时或重跑通过判断。`sched_schedstats` 开关与实际计数均保留，按采集到的计数解释。

握手故障需要找到持续未满足的请求及其阻塞闭包；短暂 gate 占用与正常 dispatch 的快照应结合前后 generation 和 ack 推进判断。采样没有观测到停滞时，结论限于采样分辨率；记录最大采样空隙，勿把零个采样跨度写成零等待。

共享机的计时 TSan 验收使用 `RUST_TEST_THREADS=1`、`CARGO_BUILD_JOBS=2`，本任务不施加额外压力。其他任务的负载通过调度证据识别，不要求单个任务保证整台主机独占。协议重放从子进程启动开始自动采样每个线程的 `schedstat`，原始时间序列存入场景目录；失败时同时输出首末计数及运行／运行队列等待增量，包括写入 stdin 期间的失败。采集错误明确输出；短寿命线程或不足两个样本的线程须结合原始计数解释。需要冻结线程、gate 和握手时再用上述取证入口。

共享主机出现编译或 supervisor 启动超时、socket 不可用、`EAGAIN` 时，保留原始输出与资源快照，停止本任务被替代的检查，与并行任务协调有界批次。资源快照自身不证明原因或容量恢复；资源条件改变后仍用原入口、期限、向量及断言验收。

加压主对照已归因为调度竞争；串行失败仍待查。串行失败中，只要运行时间、排队、阻塞与实际进展还解释不了预算消耗，就保留为待查现场；非零排队时间或仍有 CPU 推进均不足以排除机制问题。

取证需同时保留：

- 每批和关键交接点的单调时间戳，以及完成的 tick 数；
- 逐线程运行、运行队列等待增量和观察窗口，窗口扣除两者的其余时间须结合线程状态解释；
- `status` 中的 `voluntary_ctxt_switches`／`nonvoluntary_ctxt_switches`（或 `sched`），以及线程状态和 `wchan`。

自动采集使用同一 CLOCK_MONOTONIC 时基，原始线程指标存入 `schedstat.jsonl`，输出事件存入 `batches.jsonl`；失败输出包含每批回执的观测时间、已提交批次的确切 tick 数、最后输出 epoch 和未完成批次的 tick 进度下界。输出在一次采样内可能合并，时间戳是观察时间，精度受采样间隙限制；OUT 可在当前 tick 完成前发布，因此下界只计其 epoch 之前的 tick。内部交接点的精确时间／完成计数仍需主机诊断入口或专用测试观察器；勿将父进程的观察时间当成内部执行时间。

`run_failure_records_scene_and_diagnostics` 经过真实协议 `run()` 启动最小失败消费者，检查 stdin／stdout／stderr、两个 JSONL 场景文件及失败消息；删除 `run()` 的 Scheduling 接入调用后该测试必须失败。

发现尚未解释的串行或 CI 失败时，附同次完整输入、ELF、编译命令、逐批进展与线程现场继续调查，不将后续成功重放写成该次失败的解释。CI 配置、watchdog、断言和完整向量保持不变，不加重试或固定延时。
