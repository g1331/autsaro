# Windows runner 变慢的取证与 protocol 分片余量

托管 Windows runner 偶尔整体变慢约 3 倍（#591：`native (…, protocol)` 分片的 300 s／900 s 期限用完）。
这时需要先回答一件事：慢的是机器，还是产品或夹具多做了事。Windows 作业在 ETW 停止步骤里给出三份证据，
不用再手工统计 `lifecycle.jsonl`。

## 失败时先看什么

每个启用 ETW 的 Windows 作业（`native`、`native-os`、`native-delivery`）在 `Stop Windows process lifecycle trace`
步骤的日志和 Job summary 里各打印一张表；同样的内容保存在工件 `windows-job-evidence/etw/` 下：

| 文件 | 内容 |
| --- | --- |
| `process-timing.md`／`.json` | 各类进程的耗时中位数、p95、最大值与 CPU 时间，对照基线和结论 |
| `reference-end.json`、`overhead.json` | 作业开始前与结束后各 12 次 `python -I -S -c pass` 的耗时（参照探针） |
| `host-load/summary.md`／`.json`、`samples.csv` | 作业期间每 5 s 一次的宿主 CPU、运行队列和磁盘队列计数 |

`process-timing.md` 的结论只有四种：

- `runner slow`：参照探针达到基线的 2 倍以上。探针是固定的小进程，不含产品代码，它变慢说明机器变慢。
- `reference probe normal but baselined processes are N slower`：探针正常，进程中位数的比值（有基线的各类取中位）
  达到 2 倍以上，说明这次作业的编译、补丁或夹具做的事更多或更重，转去查改动。
- `within the healthy range`：两项都低于 2 倍。
- `no baseline for this job`：没有基线可比，只读绝对数。

阈值取 2 倍的依据来自 11-19 的 Windows 通过运行：同一分片的单类进程中位数在健康运行间相差最多 1.6 倍
（`cc1.exe` 562 ms 到 880 ms），探针相差最多 1.4 倍（14.6 ms 到 19.9 ms）；#591 的失败运行探针为 64.7 ms
（4.4 倍于最快的通过运行、3.8 倍于基线），`cc1.exe` 为 1665 ms（2.3 倍于基线）。
所以结论以探针为主，进程比值作旁证。

表中的 `cpu median` 来自进程退出事件的 `CPUCycleCount` 除以追踪记录的 CPU 频率。失败运行里 `cc1.exe`
的墙钟中位数是 1.665 s、CPU 中位数是 1.491 s（通过运行 0.574 s 与 0.553 s）：进程等待调度的时间很少，
同样的编译耗掉了约 2.7 倍的 CPU 周期。这类现象指向核心本身变慢，测试之间争用 CPU 只占很小一部分。

## 基线

基线保存在 `tools/python/src/autosar_tooling/windows_timing_baseline.json`，按作业标签分开：
`native-protocol`、`native-projects`、`native-integration`、`native-rest` 各一份（由 workflow 的
`AUTOSAR_ETW_LABEL: native-${{ matrix.shard }}` 选择），记录探针中位数和各类短寿命进程
（`git`、`gcc`、`cc1`、`as`、`ld`、`conhost` 等，至少 20 个样本且 p95 低于 5 s）的墙钟与 CPU 中位数。
不同分片编译的源文件不同（`cc1.exe` 中位数从 150 ms 到 900 ms），共用一份基线会让比值失去意义。
没有标签的作业（`native-os`、`native-delivery`）只用各分片探针基线的中位数判断 runner 是否变慢。

选固定基线加同作业探针，原因是两者互补：同作业探针与产品无关，能直接判断机器；固定基线记录“健康时每类进程
有多慢”，在探针正常时用来发现产品变重。基线来自 11-19 的 master 通过运行：`native-protocol` 取 44541303411、
44550323807、44551838931、44558822646 四次，其余三个分片取前三次，逐类取各次中位数的中位数。

刷新基线（runner 镜像升级或编译内容明显变化后）：从健康运行下载同名工件，解压出 `windows-job-evidence/etw`，然后

```sh
uv run --locked python -m autosar_tooling.windows_process_timing baseline native-protocol \
  "<运行号与日期>" <etw 目录 1> <etw 目录 2> ...
```

也可以对任一工件目录单独查看汇总：`... windows_process_timing summarize <etw 目录> native-protocol`。

## 宿主负载

作业开始时 `windows_etw.start` 启动一个只读计数器的 `typeperf`，作业结束时 `stop` 读取 `samples.csv` 并结束它；
未被停止时它在 480 个样本（40 分钟）后自行退出。它不创建数据收集器集、不改注册表、不改宿主全局设置。
采样项：`% Processor Time`、`% Privileged Time`、`% Interrupt Time`、`% DPC Time`、`Processor Queue Length`、
`Context Switches/sec`、`PhysicalDisk` 的 `% Disk Time`、`Avg. Disk Queue Length`、`Avg. Disk sec/Transfer`、
`Disk Transfers/sec`，以及 `Available MBytes`。采样器启动或汇总失败只在日志里报告，不影响作业。

计数器反映客户机看到的状态，判断是否被其他租户占用时和另外两份证据合读：

- 作业自己的负载低（`cpu_busy_pct` 中位数不高、`run_queue_length` 小）而参照探针很慢，或 CPU 周期比基线多出同等倍数，
  说明核心被其他负载拖慢。
- `run_queue_length` 长期高于 vCPU 数（4）而 `cpu_busy_pct` 接近 100，说明作业自己的并行度超过 CPU。
- `disk_queue_length` 持续大于 2 且 `disk_latency_ms` 升高，说明磁盘排队，慢在文件读写（补丁、对象文件、杀毒扫描）。

## 编译对象复用

native 的 protocol、projects、integration 及 Linux 全量作业为本次运行分配临时对象目录。
本地可将 `AUTOSAR_TEST_OBJECTS` 设为本次测试专用空目录的绝对路径，使用同一固定 SDK。
行为夹具保留原 guard、工具检查、内核补丁与摘要校验。GCC 在实际工程目录编译并通过 `-MMD` 记录每个对象读取的源码与非系统头文件。
复用时核对编译器路径和摘要、编译参数、源码和 include 搜索目录下的文件名、补丁后内核文件名，以及该对象所有依赖的实际字节。新增候选头会使对象失效。读取含上级目录引用或文件存在性探查的源码或头文件时，还核对完整封存文件名清单。
对象和依赖记录在编译成功后一起发布；每个用例仍在自己的目录编译入口或外部 control，链接并运行程序。
COM 用例还逐次编译涉及变异的应用、COM、ISR、诊断输出和排空路径源码。交付接收、封存拒绝及检测器有独立的构建入口。

## protocol 分片的期限余量（#591）

分片在 4 vCPU 上运行 14 个用例（Cargo 默认 4 个并行线程）。下表数据来自 ETW 的进程树，通过运行是 44558822646，
失败运行是 44539397861；一次完整构建编译 54 个 C 文件（54 个 `cc1.exe`），CPU 约 31 s。

| 期限 | 位置 | 通过运行实测 | 余量 | 失败运行 |
| --- | --- | --- | --- | --- |
| 外层 900 s | `protocol.rs` `com-refusal-consumers` | 307 s | 2.9 倍 | 900 s 时完成 12 个构建（共 14 个） |
| 内层 300 s | `normal_verify.py` `public-verify` | 80 s（2 个构建） | 3.7 倍 | 301.6 s，完成 96／108 个编译 |
| 外层 360 s | `protocol.rs` `normal_verify_…` | 同上 | 4.5 倍 | 内层先到期 |
| 构建 180 s | `protocol.rs` 各用例的 `build` | 39–53 s（1 个构建，运行中争用） | 3.4–4.6 倍 | 未到期 |

调整 14 个构建的排队顺序后，通过运行 [44585924169](https://github.com/g1331/autsaro/actions/runs/44585924169)
仍有 1512 个 `cc1.exe`。用该作业的 ETW 工件重算 `process-timing.md`，`cc1.exe` 合计 1162.5 CPU 秒。
比较时读同一分片表中 `count` 和 `cpu total s` 两列，CPU 总量按每个退出事件的周期数求和。

| 通过运行 | `com-refusal-consumers`（900 s） | 余量 | `normal_verify` 内层（300 s） | 余量 |
| --- | ---: | ---: | ---: | ---: |
| 44585924169，共享对象前 | 462 s | 1.95 倍 | 约 166 s | 约 1.81 倍 |
| 44594837691，共享对象前 | 449.65 s | 2.00 倍 | 145.15 s | 2.07 倍 |
| 44621218308，共享对象后 | 119.38 s | 7.54 倍 | 17.28 s | 17.36 倍 |

后两行以 ETW 起止事件配对受期限约束的 Python 进程：外层取 `consumers.py`，内层取 `normal_verify.py` 启动的 `ecu-tool.py verify`。余量按期限除以未取整的墙钟计算。

在最初的 #591 对照中，整个分片通过时 CPU 总量约 911 core·s，占 4 vCPU 307 s 的 74%；失败时约 3006 core·s，占 901 s 的 83%。

结论与处理：

1. 降低并行度不会改善。`cc1.exe` 的墙钟与 CPU 中位数之比在通过运行是 1.04、在失败运行是 1.12，
   用例之间争用 CPU 的损耗不超过 12%；两个长用例各自是串行的构建链，线程数减到 3 个只会让其余 13 个用例排得更长。
2. 最长的 `com-refusal-consumers` 在分片的最后 95 s 单独运行（最多 2 个构建同时进行），其余 vCPU 空闲。
   它的 14 个构建原先分成 5、5、4 个的三组，由 2 个工作线程执行，最长链是 9 个构建。改为逐个构建排队后，2 个工作线程各做 7 个，最长链缩短到
   7 个构建（约 22%），并发度与期限都保持不变。
3. `normal_verify` 的内层 300 s 是余量最小的一项：通过时 80 s，CPU 约 63.5 s，需要整机慢约 3.7 倍才会到期；
   失败运行的 CPU 慢了约 3.1 倍，再加上排队就超时。不放宽这个期限。根本的缓解是减少重复编译：这个分片在 307 s 内
   完成了 28 次完整构建（1512 个 `cc1.exe`，每次 54 个），后续对象复用的实测结果见下节。
4. 其他期限（180 s 的构建、外层 360 s）在 3 倍慢的运行里没有到期，当前余量保持。

下一次复发时，按“失败时先看什么”读结论；结论为 `runner slow` 时，这些期限在更慢的机器上给不出更大的余量，
要做的是减少工作量，期限与重试保持原样。

## Windows native 分片的编译工作量（#595）

[改前运行 44594837691](https://github.com/g1331/autsaro/actions/runs/44594837691) 与 [PR 运行 44621218308](https://github.com/g1331/autsaro/actions/runs/44621218308) 的三个 Windows native 分片均通过。下表取自各分片的 `process-timing.md`。改前用相同汇总代码回放原始 ETW 工件，CPU 秒取配对退出事件的周期数除以追踪频率。

| 分片 | `cc1.exe` 次数（前 → 后） | CPU 秒（前 → 后） | CPU 变化 | 启动探针中位数，ms（开始／结束，前 → 后） |
| --- | ---: | ---: | ---: | ---: |
| protocol | 1512 → 560 | 1168.1 → 290.2 | −75.2% | 22.2／22.8 → 15.4／17.4 |
| projects | 2722 → 2712 | 1447.1 → 1626.8 | +12.4% | 25.2／20.3 → 20.1／19.7 |
| integration | 1580 → 1162 | 822.4 → 748.2 | −9.0% | 16.0／15.6 → 19.9／18.5 |

调用次数直接反映编译工作量。protocol 改后 runner 的固定启动探针更快，实测余量同时受复用和机器速度影响。integration 改后探针更慢，仍少编译 418 次，且 CPU 总量下降。projects 只少编译 10 次，CPU 总量和作业墙钟均上升；两次运行不足以确定上升的原因。
