---
title: 'Epic 4 A1 交接命令的受管进程关闭'
type: 'bugfix'
created: '2026-10-01'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'bc348ad79bb8b9768db6b98d9343bc8530a27e2f'
context: []
---

<frozen-after-approval reason="用户已明确委托完成A1／A2，沿用持续实施授权；A1先行">

## Intent

修复主机交接watchdog在清理工具无法启动时留下父子进程的问题。工作台与独立verify.ps1必须在执行前托管本次进程树，超时或失败后关闭自己的进程，不依赖taskkill.exe成功。

## Boundaries & Constraints

保留180s构建、270s工作台验证、60s离线行为及5000ms COMMIT时限、原输出断言和固定Windows主机范围。后台隐藏运行，临时文件保持原私有路径保护；仅关闭本次创建的进程。固定FreeRTOS、汽车时间与接口不变，不生成assurance工件，不推送。A2在独立Build增量随后实施。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 正常合法命令／完成 | 保留退出状态和stdout／stderr，回收本次私有目录 |
| 真实父子进程停滞／watchdog到期，taskkill无法启动 | 命令失败，两个进程均退出；没有被保留的后代或假成功 |
| Job建立或分配失败／准备执行 | 尚未执行用户命令的子进程关闭，准确报错，不降级为无托管执行 |
| 离线包／构建超时 | 返回watchdog失败，整个本次构建树退出；原CAN／DID验证断言保留 |

</frozen-after-approval>

## Code Map

- `core/src/integration/handoff.rs`：受控执行及现有Windows父子进程单测；产物脚本由include_bytes生成。
- `core/src/integration/windows_job.rs`、`mod.rs`：新增私有Win32 Job生命周期封装，复用现有隐藏／挂起／先分配再恢复的隔离脚本机制。
- `runtime/ecu/process-tree.cs`、`verify.ps1`：离线原生启动辅助层，随包交付，不要求Python；构建输出仍按原语义收集。
- `core/tests/support/epic4_handoff.rs`：复用现有生成包和缩短测试副本时限的停滞工具回归，增加清理工具不可启动情形。
- `scripts/epic4_desktop.py`：仅参考Job机制，不改变隔离或用户桌面。

## Tasks & Acceptance

- [x] `windows_job.rs`／`mod.rs`／`handoff.rs`：建立执行前Job托管、关闭及有界等待；保留错误和输出语义。
- [x] `process-tree.cs`／`verify.ps1`／`handoff.rs`：接入离线受管构建，生成辅助源进入同一产品闭包。
- [x] `handoff.rs`／`epic4_handoff.rs`：扩展真实父子进程与工具失效回归，保留成功、失败和目录回收断言。
- [x] 本spec：记录实际验证与BMad审查，完成后本地提交。

Given当前固定主机依赖，when正式新目录交接重新生成、搬移、构建、运行并注入清理工具故障，then原成功／拒绝／恢复向量通过且失败进程树关闭。Given宿主限制不允许Job分配，when启动命令，then失败闭合而非让命令无监督继续。

## Implementation Notes

采用非继承、kill-on-close的私有Windows Job；父进程挂起创建，分配后才恢复，禁止无监督降级。Rust沿用标准Command的参数、环境和输出文件处理，以Toolhelp／创建时间取得主线程；离线C#层直接持有CreateProcess返回的主线程，捕获私有文件并核对Job active计数归零。无新Cargo依赖。实现由主代理完成：仓库规则将非机械实现保留给主代理，调查与复核使用只读default子代理。

依据[Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)的继承／kill-on-close机制；[Rust ChildExt](https://doc.rust-lang.org/std/os/windows/process/trait.ChildExt.html)主线程句柄仍为nightly API，因此不升级工具链。

现有全部7项单元测试已实际通过，包括新加的无效taskkill负例及真实query-only Job导致的分配拒绝；拒绝的PowerShell命令未执行，原目录回收断言保留。最终结果见下方Verification。

## Spec Change Log

## Review Triage Log

| 来源／发现 | 判定／处置 | 核实与处理 |
| --- | --- | --- |
| blind：CreateProcess继承调用者的全部可继承句柄 | medium／patch | Win32的true确会额外继承调用者的标准捕获句柄；改为STARTUPINFOEX三项句柄白名单，列表与本次文件句柄均由finally回收。 |
| blind：只验证一层子进程 | low／patch | 原新增向量仅一层属实；扩展无效工具负例至父／子／孙三级，逐个PID检查退出。原2s父子watchdog测试不变。 |
| gap：离线C#分配失败缺回归 | medium／patch | Rust失败测试不覆盖C#实现属实；在临时生成副本向Assign传无效Job，真实Win32拒绝后断言命令标记未写、记录的挂起进程退出。 |
| edge：全可继承句柄传递 | medium／patch | 同blind句柄根因；同一三项白名单修正，保留独立来源记录。 |
| edge初次partial：Job查询失败／5s未退出仍返回错误 | false／reject | StopTree明确报Observe错误或shutdown could not be confirmed，不宣称已关闭；finally仍终止并关闭kill-on-close Job。未展示固定工具会触发该内核失败或绕过Job，不能把准确失败报告当假关闭。 |
| edge完整复核：同一未确认关闭意见 | false／reject | carried：同位置同主张；完整复核确认此前partial只需补完范围。沿用上述反证，不重复修补。 |


## Verification

运行现有Windows受管执行单测、`epic4_independent_handoff -- --exact`、核心Clippy及源码增量检查；构建最终桌面程序供A2隔离验收。原始输出和测试产物只放系统临时目录，结论进入本spec。无产品目标或不可逆操作的未决选择。

A1正式新目录交接退出0，1项通过／109项精确过滤，210.10s；实际生成／搬移／再生成、正常与额外输出拒绝、离线停滞构建和无效清理工具环境均运行。离线父子PID检查均已退出。源码增量质量检查退出0。首次额外加-D warnings的Clippy被历史告警和本次一处err().expect()阻断；本次告警已直接修正，按仓库verify.py已有Clippy命令验证，不修改历史规则或无关源码。

审查三层已完整完成；blind仅返回两条可定位意见，没有按数量补造。新增三级负例最初只捕获父PID，源于fixture未显式继承stdout；改为明确输出继承和新向量5s建立期后7项单测全部通过（5.48s），原2s单测和生产时限不变。审查补丁后的正式交接已通过。

最终审查补丁后正式交接退出0：1项通过／109项精确过滤，222.44s（临时autosar-a1-handoff-final.log）；新增离线无效Job拒绝实际执行，命令未写标记且父PID退出。最终源码增量检查及仓库固定Clippy规则均退出0，无递延审查项；旧全量的不变OS／应用断言复用4.22有效结果。

最终桌面程序cargo build退出0（31.58s），供A2使用本次真实后端。没有修改汽车C代码或规范输入，所有已运行验证对应最终语义。
