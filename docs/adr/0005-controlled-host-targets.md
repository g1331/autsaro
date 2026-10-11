# 0005. 受控主机目标 Windows／Linux 与证据边界

## 背景

生成的 ECU 需要在开发者主机上可重复地构建、运行和验证，而主机线程调度和墙钟不可控。若宿主线程直接修改 BSW、或 RTOS tick 与虚拟时间双重推进，行为不可复现；若主机结果被当作 MCU 证据，会夸大支持范围。

## 决策

1. 受支持的执行目标是 `windows-x64-controlled-v1`（MSYS2 GCC 16.1.0 Rev5，PE）与 `linux-x64-controlled-v1`（Ubuntu GCC 13.3.0＋binutils 2.42，ELF），工具链身份记录在 `runtime/os/toolchain*.json`，错误宿主或工具身份明确失败。
2. 时间模式为 `controlled_logical_ms`：uint64 逻辑 epoch，控制面逐个请求 1 ms tick，并在内核计数、Counter 动作、周期工作和输出确认到达静止点后推进下一 tick；标准 Counter 与 32 位内核 tick 各自保持模数。墙钟 timer 不生成汽车 tick，宿主 watchdog 只控制测试进程存活。
3. 主机输入以有界批次（HostBatch）进入，bridge 赋予全 ECU 单调序号；同刻先注入输入再推进 tick。阻塞 I/O 在宿主桥中，不进入被调度任务。
4. 通信开放后由唯一 owner task 串行执行应用与 BSW；宿主线程、模拟 ISR 和内核隐藏任务只提交已复制记录或设置事件。
5. 证据按层独立：主机接口／逻辑行为、实现能力、MCU 实时性／ISR／内存段分别取证；Windows 与 Linux 结果互不替代；未执行记为未执行。
6. MCU 目标（首个候选 FRDM-A-S32K344）在取得合法板卡、驱动与工具链并通过实际对端之前不宣称支持。

## 后果

- 同一输入可在两个主机目标上得到可复现的行为证据，适合 CI 与离线接收者复验。
- 主机证据不提供硬实时、真实中断或段放置保证；这些义务由目标阶段关闭。
- 平台原语按真实平台验证；POSIX 的保证不能推定为 Windows 保证。

## 状态

已接受（2026-09／2026-10，Linux 目标后续加入）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/INTEGRATION-CONTRACT.md`（§3–§5）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/ARCHITECTURE-SPINE.md`（AD-6、AD-7）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture.md`（工程链与责任边界）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/BASELINE.md`（profile、target 与实际来源）。
