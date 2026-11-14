# Windows 进程

## 崩溃转储

Windows 的 `native`、`native-os` 和 `native-delivery` 作业在测试前配置
[WER LocalDumps](https://learn.microsoft.com/en-us/windows/win32/wer/collecting-user-mode-dumps)：
完整转储写入 runner 临时目录的 `native-crash-dumps`，最多保留 3 份，作业失败时上传并保留
14 天。退出回调崩溃的控制程序完整转储约 10 MB，采用 `DumpType=2` 保留异常 CONTEXT、
全部线程栈和进程内存。

OS 使用 `WerSetFlags(WER_FAULT_REPORTING_NO_UI)` 保持无交互报告；
[`SEM_NOGPFAULTERRORBOX`](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-seterrormode)
会抑制 WER，故不用于这些原生入口。`WerSetFlags`／`WerGetFlags` 由现有 Kernel32 提供，
无需额外导入库；固定 MinGW 的实际退出程序已核对导入表。

自然复发后先保存同一作业的 dump 与现有失败现场，使用 Windows SDK 的 x64 `cdb`：

```text
cdb -z <file.dmp> -c ".symfix; .reload; .ecxr; .exr -1; ~*k; q"
```

按异常码、指令地址、访问地址、线程和模块偏移关联 stderr 与 `windows-job.jsonl`；
MinGW 程序同时保存对应二进制，用 `addr2line` 解析项目帧。正常 `ExitProcess`／
`TerminateProcess` 退出及受管超时终止按各自契约处理；LocalDumps 采集未处理崩溃。

## Job 成员取证

supervisor 启动须收到期限内的认证就绪响应。Windows Job 归零与进程退出可能有窗口，
检查已关闭须按创建时间排除 PID 复用并有界等待退出事件；按父 PID 归属时要求子进程创建
不早于父进程。root exit=0 不证明整树退出；诊断先保存清理前的成员身份、归属、退出与
CPU 采样，随即释放观察句柄；写入失败不得阻断清理。`windows-job.jsonl` 随 owned capture
保存阶段与完整结果；查询错误或截断标为不完整。
