# Windows 进程

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
`job.wprp` 是该次实际 profile，`collector.jsonl` 还保存 runner 的 provider 定义。
采集或解码失败使步骤失败，原始文件仍上传；先核对 `summary.xml` 的丢事件信息。

清理前仍有成员时，两种实现各调用一次 `NtQuerySystemInformation(SystemProcessInformation)`，
按 PID＋创建时间匹配，将父 PID、镜像和线程状态写入成员的 `system_snapshot`。
`state=5` 的 `wait_reason` 是采样时的等待原因；其他状态保存空值。
该查询不打开成员句柄，缓冲区固定为 16 MiB，查询失败保存 `system_snapshot_error`。
原 `image_error` 保留；系统快照提供的镜像与进程句柄查询各自标明来源。

读取下一次失败时：

1. 在对应 capture 的 `before_stop` 找到活动成员的 PID＋`created_100ns`、wait 与 membership，
   再读 `system_snapshot` 的镜像、父 PID、线程状态和等待原因。
2. 在 `members-etw.jsonl` 按同一身份读取创建／退出事件及命令行；PID 复用后创建时间不同的
   记录分开关联。原始事件字段完整保留，字段随 provider 版本变化时可重查 XML／ETL。
3. 结合 `stopped` 与退出事件判断成员进入退出、线程已结束及进程对象 signaled 的先后关系。
   查询错误、缺失事件和顺序采样的窗口都保留在报告中；归属和完成判定仍由原 Job 逻辑执行。

Windows `rest` 作业实际运行 Python 三层 orphan 反例：令镜像 API 返回拒绝访问，要求
清理前报告仍有子进程身份、镜像、父 PID 和线程等待原因；删除系统快照采集的变异须被
同一断言拒绝。作业结束再断言 ETW 有该 PID＋创建时间的镜像、父 PID、命令行和退出事件。
Rust 原有三层反例同步断言相同成员字段。普通成功路径验证不落完整报告；
`etw/overhead.json` 和 `job-evidence/overhead.json` 保存同 runner 的配对耗时原始样本。
