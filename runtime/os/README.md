# Epic 4 Windows OS 基础

本目录是独立新目标的生产基础，当前仅验证 Story 4.3 生命周期。未接入 ECU/RTE/BSW，不声明 SC1，也不放行 W3。旧 `runtime/src/Os.c` 继续用于历史主机目标，两者不能同链接。

## 固定来源与离线复建

`third_party/freertos/source-manifest.json` 固定 FreeRTOS V11.3.1 / commit `054e14f3397023aa83813a65aa065fc4597d481b`，归档 SHA-256 `dd832f699e6ebee00366d7c737e259860d9d1fd5f2c2311c90dff16e8cfbcf61`，并记录最小源码闭包的原始字节摘要和 MIT 许可。上游原件未格式化；许可与版权通知保留。构建核对全部源码，复制到临时目录并应用 `patches/0001-controlled-host-lifecycle.patch`，不修改原件、不下载依赖。摘要用于完整性复核，不是签名认证。

原生 Windows x64，MSYS2 Rev5 GCC 16.1.0 / `x86_64-w64-mingw32`，C99；使用现有 GCC/Windows SDK 头文件及系统库。其他编译器、32 位和 MCU 未验证。

```powershell
python scripts/epic4_os.py --suite lifecycle
cargo test --manifest-path core/Cargo.toml epic4_backend_lifecycle -- --nocapture
```

## 责任与资源

`Os.h` 是上层生命周期及状态入口；`Os_Target.h` 单独定义静态目标配置和受控测试 trace。汽车对象最多 16 个，优先级 1–30，模式 1/2；宿主保留栈为独立实现字段，64 KiB 对齐，64 KiB–16 MiB，不把 FreeRTOS buffer 当作执行栈。

唯一 backend 持有静态 TCB/配置 buffer，FreeRTOS 选择 Ready/Running 和上下文。隐藏 FreeRTOS bootstrap（优先级 31）执行 StartupHook、发布 Ready、启用 mode AUTOSTART 后挂起；idle 优先级 0。原生控制线程只等待关闭 mailbox，在独立 256 KiB reserve 栈执行 ShutdownHook、输出记录并 ExitProcess；不选择任务，不调用汽车 BSW。StartOS 正常不返回；正常/错误关闭均不能恢复任务。建立控制资源之前的拒绝/创建失败在启动调用者执行关闭 Hook；控制线程建立后均在独立控制栈执行。报告的 lifecycle=Closed 为终态，state 为最后初始化结果。资源回收由目标进程退出完成，当前不支持进程内重启。

基准两汽车任务加 bootstrap/idle，共 5 个新原生线程（4 个内核线程、1 个控制线程），6 个 event（4 个 yield、1 个关闭、1 个模拟中断）和 1 个模拟中断 mutex。原始调用 StartOS 的主线程承担端口模拟中断控制。静态内核对象不代表 Windows 无堆分配；原生线程/事件/mutex/系统栈由 Windows 建立。每次 Create* 计数与实际结果记录在独立证据中。任务及 Hook 不执行宿主阻塞 I/O；关闭报告只在控制线程输出。输出成功不是交付 CAN/HostBatch 的证明。

## 生产补丁边界

端口补丁保留上游原有线程上下文机制和调度选择；改为正常宿主优先级，移除墙钟 timer/首次 tick，首次只 yield。创建栈按目标 reserve，所有创建经过可审阅失败检查/计数，关闭门阻止继续 resume。未修改 `tasks.c` ready 策略；FIFO/原子结束/事件/资源补丁属于 4.4–4.7。4.8 再提供逐毫秒受控 ticket；当前无输入/tick 驱动。

`AUTOSAR_OS_FAIL_RESOURCE` 为独立 harness 的原生资源建立失败注入入口：正整数 N 表示第 N 次 CreateEvent/CreateMutex/CreateThread 返回失败，随后目标关闭。畸形、零、负值及溢出输入以 E_OS_VALUE 拒绝。正常部署不设置；测试器显式清除继承值。每个实际建立点均有独立子进程向量。

## 验收边界

证据位于 `docs/assurance/evidence/epic4/backend-lifecycle.json`，包含固定内核、生产源/补丁摘要、构建调用及 34 个实际向量。两种模式及两个同时就绪任务对照真实内核 Running/Ready/SUSPENDED，非法查询不改变输出状态；全部 12 个资源建立点失败都在 Ready 之前关闭。后续实际栈故障监测、完整 Extended Status/Hook/ISR、激活语义、Counter/ScheduleTable、ARTI、完整等级和交接仍未通过。
