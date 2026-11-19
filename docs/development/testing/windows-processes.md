# Windows 进程

## Bash 脚本测试

Python 测试执行 Actions 的 Bash 脚本时，Windows 显式选择
`Path(os.environ['ProgramFiles']) / 'Git/bin/bash.exe'`；裸 `bash` 会启动系统 WSL 入口。
沿用 `test_core_build_dependencies.py` 的 `--noprofile --norc -eo pipefail` 调用，
传给 Bash 的路径使用 `as_posix()` 并正确引用，用含空格的临时目录检查路径传递。
失败断言附 shell 路径、返回码和 stdout／stderr，便于区分启动失败与脚本行为失败。

## 崩溃转储

Windows 的 `native`、`native-os` 和 `native-delivery` 作业在测试前配置
[WER LocalDumps](https://learn.microsoft.com/en-us/windows/win32/wer/collecting-user-mode-dumps)：
完整转储写入 runner 临时目录的 `native-crash-dumps`，最多保留 3 份，作业失败或取消时上传，
上传步骤限时 2 分钟并保留工件 14 天。退出回调崩溃的控制程序完整转储约 10 MB，采用 `DumpType=2` 保留异常 CONTEXT、
全部线程栈和进程内存。

交付运行时保持原 `SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX` 策略。
stack 测试夹具在 OS 初始化后的 `StartupHook` 中清除抑制位，并通过
`WerSetFlags(WER_FAULT_REPORTING_NO_UI)` 显式选择无交互采集；独立 restart 测试入口自行设置。
[`SEM_NOGPFAULTERRORBOX`](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-seterrormode)
会抑制 WER，因此由这些测试夹具选择性清除。`WerSetFlags`／`WerGetFlags` 由现有 Kernel32 提供，
无需额外导入库；固定 MinGW 的实际退出程序已核对导入表。

故意崩溃的探针在进程内保持 `SEM_NOGPFAULTERRORBOX`：`port-restart.exe` 的 `unhandled`、`foreign`
模式与 stack 夹具的 `unrelated` 场景以预期的未处理异常结束，若进入 WER 会写约 10 MB 完整转储，
转储耗时还会超过测试的 10 秒期限（#543）。LocalDumps 的全局配置保持不变，新增故意崩溃的
探针沿用同一做法。

自然复发后先保存同一作业的 dump 与现有失败现场，使用 Windows SDK 的 x64 `cdb`：

```text
cdb -z <file.dmp> -c ".symfix; .reload; .ecxr; .exr -1; ~*k; q"
```

按异常码、指令地址、访问地址、线程和模块偏移关联 stderr 与 `windows-job.jsonl`；
MinGW 程序同时保存对应二进制，用 `addr2line` 解析项目帧。正常 `ExitProcess`／
`TerminateProcess` 退出及受管超时终止按各自契约处理；LocalDumps 采集未处理崩溃。

生产构建原样运行时，可另用预先附加的 ProcDump（`-ma -e -x`）采集；该路线本次未接入。

## Job 成员取证

supervisor 启动须收到期限内的认证就绪响应。Windows Job 归零与进程退出可能有窗口，
检查已关闭须按创建时间排除 PID 复用并有界等待退出事件；按父 PID 归属时要求子进程创建
不早于父进程。root exit=0 不证明整树退出；诊断先保存清理前的成员身份、归属、退出与
CPU 采样，随即释放观察句柄；写入失败不得阻断清理。`windows-job.jsonl` 随 owned capture
保存阶段与完整结果；查询错误或截断标为不完整。

Rust 外层与交付 Python 内层（含 Git patch check/apply、编译）使用 `windows-job-v2`
记录：`assigned`、`root_exited`、`before_stop`、`stopped`、`result`。正常成功只暂存记录；
非零退出、超时、成员回收或 OrphanedMembers 才写完整 `windows-job.jsonl`。
必需 Windows native、native-os、native-delivery 作业通过
`AUTOSAR_JOB_EVIDENCE_DIRECTORY` 另存失败记录，避免测试成功后删除主动反例现场。
对应现有工件以 `always()` 上传，包含 `windows-job-evidence/job-evidence` 和 `etw`。

### 进程身份与退出时间线

CI 在作业外使用系统自带 WPR 的最小 profile：`ProcessThread` 保存内核进程命令行，
`Microsoft-Windows-Kernel-Process` 的 `0x30` 保存进程／线程生命周期与创建／退出时间。
采集不启用 CPU 栈采样或上下文切换，不修改注册表或持久设置。作业结束在 `always()`
步骤停止会话，以 tracerpt 解码；保留 `process.etl`、`events.xml`、`summary.xml`、
`collector.jsonl`（命令、退出码、耗时与输出）、`lifecycle.jsonl` 和 `members-etw.jsonl`。
`job.wprp` 同时定义 File／Memory 两种模式，并用唯一 ID 引用 collector 与 provider；
内核态 `JobProcessProvider` 必须设置 `NonPagedMemory="true"`，否则 WPR 的 provider
启用可能返回 `0xc5580612`；调整 profile 时用单个 Windows 作业实际核对 start／stop 和解码，
XML 引用检查只验证结构。该文件是本次实际 profile，`collector.jsonl` 还保存 runner 的
provider 定义。
采集启动、停止或解码失败时，记录原因到 `collector-state.json`，原始文件照常上传，
测试继续执行；该次 ETW 取证标为不完整。`trace-health.json` 记录实际解码数量、进程
生命周期事件数及丢事件／缓冲区计数；只有系统事件或发现丢失时同样标为不完整。
先核对采集状态及 `summary.xml` 的丢事件信息。

清理前仍有成员时，两种实现各调用一次 `NtQuerySystemInformation(SystemProcessInformation)`，
按 PID＋创建时间匹配，将父 PID、镜像和线程状态写入成员的 `system_snapshot`。
`state=5` 的 `wait_reason` 是采样时的等待原因；其他状态保存空值。
该查询不打开成员句柄，缓冲区固定为 16 MiB，查询失败保存 `system_snapshot_error`。
原 `image_error` 保留；系统快照提供的镜像与进程句柄查询各自标明来源。

读取下一次失败时：

1. 在对应 capture 的 `before_stop` 找到活动成员的 PID＋`created_100ns`、wait 与 membership，
   再读 `system_snapshot` 的镜像、父 PID、线程状态和等待原因。
2. 在 `members-etw.jsonl` 按同一身份读取创建／退出事件及命令行；PID 复用后创建时间不同的
   记录分开关联。命令行仅从明确的进程创建事件关联，区间以该进程创建事件头的时间开始，
   到同 PID 下一次创建或该进程退出事件头的时间为止，rundown 与其他 provider 的事件保持
   未关联。区间只用事件头时间：payload 的 `CreateTime`／`ExitTime` 是内核系统时间，与跟踪
   时钟的偏移每次运行不同（#577 中一次运行为 −3.6 到 +3.4 ms，另一次为 +0.05 到
   +2.8 ms），混用会让早于 `CreateTime` 的命令行事件落到区间外。原始字段完整保留，
   字段随 provider 版本变化时可重查 XML／ETL。
3. 结合 `stopped` 与退出事件判断成员进入退出、线程已结束及进程对象 signaled 的先后关系。
   查询错误、缺失事件和顺序采样的窗口都保留在报告中；归属和完成判定仍由原 Job 逻辑执行。

Windows `rest` 作业实际运行 Python 三层 orphan 反例：令镜像 API 返回拒绝访问，要求
清理前报告仍有子进程身份、镜像、父 PID 和线程等待原因；删除系统快照采集的变异须被
同一断言拒绝。作业结束再断言 ETW 有该 PID＋创建时间的镜像、父 PID、命令行和退出事件；
`rest` 作业设置 `AUTOSAR_ETW_REQUIRE_CONTROLS=true`，对照记录缺失、损坏或其中任一项不成立时
停止步骤以非零退出；采集启动、停止、解码失败和丢事件同样让该步骤失败，使缺证据的运行不会被计为干净运行（ADR 0031）。其他作业仍只标为不完整。
Rust 原有三层反例同步断言相同成员字段。普通成功路径验证不落完整报告；
`etw/overhead.json` 和 `job-evidence/overhead.json` 保存同 runner 的配对耗时原始样本。

### Console host 收尾窗口

`CREATE_NO_WINDOW` 创建的命令会得到一个隐藏 console，由根进程的后代 `conhost.exe` 承载，该
进程也属于命令的 Job。最后一个客户端退出后 console host 立即退出（实测约 3 ms），但进程对象的
收尾与 Job 活动计数的下降可能滞后：一次 CI 工件（`windows-job-evidence`，ETW 共 1619 个 console
host）中 p50 为 2.6 ms，p99 为 6.5 ms，滞后超过 100 ms 的只有同一段 ProcessStop 事件间隔内的
四个样本（398–404 ms），其中两个导致 Job 在 200 ms 记账宽限届满时仍有成员，被判为
`orphaned_members`（#412、#414）。

200 ms 宽限是根进程退出后的轮询窗口，判定依据是“观察到 Job 为空”：观察时刻受调度影响，可能
晚于宽限和命令期限，这条原有路径不做绝对期限检查，本次修复保持不变。
`tests/python/process/test_windows_console_host.py` 与 Rust 的
`the_accounting_grace_is_a_fixed_window_after_the_root_exit` 固定了这一现状。下面新增的 host
额外等待则受命令绝对期限与 5 s 上限约束。

宽限届满后，Rust `ProcessTree::hosts_finished` 与 Python
`WindowsJob._hosts_finished` 只在以下条件同时成立时继续等待：

- 剩余成员全部是此前记录的 `GetSystemDirectoryW` 所返回目录下的 `conhost.exe`；读不到系统目录时
  不识别任何 host。身份按 PID＋创建时间记录，镜像在收尾阶段会因拒绝访问读不出，所以根退出时先
  记录一次，宽限内每 10 ms 再记录。
- 这些 host 的 `GetExitCodeProcess` 已不是 `STILL_ACTIVE`。

等待上限取 5 s，与 `stop` 在 `TerminateJobObject` 后等待成员离开的既有预算一致，并受命令的绝对
期限约束。每次观察后先对照期限与上限，晚于它们才第一次看到 Job 清空也判失败。其他成员、仍在
运行的 host、不可读的成员列表或超过上限，仍按 `orphaned_members` 处理。Python 在分类过程中抛出
异常时同样先关闭 Job 再向上传播。

Python 的 `tests/python/process/test_windows_console_host.py` 用模拟 Job 与时钟在各平台覆盖等待
规则；Windows 作业另验证真实 console host 被识别。真实泄漏的后代由既有 parent-first 反例保持失败。
