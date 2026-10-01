# 受控主机 Automotive OS runtime

固定 FreeRTOS V11.3.1 commit `054e14f3397023aa83813a65aa065fc4597d481b`。Windows GCC16.1.0 x64 使用原有十四个受控补丁；Ubuntu24.04 x86_64 GCC13.3.0 独立 OS 消费者使用同一内核、其中适用的公共补丁和 `patches/linux/0001-controlled-posix-port.patch`。`third_party/freertos/` 的原件、两份来源摘要与 MIT 许可保持不变，不从网络获取浮动内核。`toolchain.json` 与 `toolchain-linux.json` 分别核对本机编译器及目标身份；Linux 锁还核对 objdump/nm 可执行文件。此阶段仅 Linux **OS 独立 harness** 已原生验证，完整 Linux ECU、生成工程和工作台运行仍待后续阶段。

Os_TargetConfig静态配置汽车Task、资源、内部资源、Counter、Alarm、ScheduleTable、Hook和逻辑中断。唯一backend适配真实内核；FreeRTOS决定实际Running、抢占和上下文切换，汽车层维护完整激活请求FIFO、事件和资源契约，不建立另一套Task选择器。

StartOS成功不返回。Task正常终止／ChainTask通过实际原生入口移交；Task意外返回会报告MISSINGEND、恢复遗留资源和屏蔽，再完成激活。PreTask／PostTask对应真实Running转换；Shutdown不伪造PostTask。ErrorHook提供标准服务身份和准确参数快照，Hook内错误不递归、不覆盖外层快照。参考EXTENDED不变，显式STANDARD配置也受支持；两模式保留防御检查和非零结果报告，标准激活及Alarm告警保留。

Disable／Enable、嵌套All／OS配对按调用者保存和恢复状态。Cat1与Cat2有不同屏蔽语义；源操作改变实际pending／disabled位。嵌套ISR使用同一真实S线程的原生栈，返回恢复外层身份和资源；泄漏屏蔽／资源在Cat2退出清理。Os_IsrConfig.entries可选32槽void身体表，用OS_ISR_ENTRY(name)绑定；私有0／1、Cat1、无优先级及已配置input mailbox槽不得指定身体，配置身体不能被raw handler替换。

TASK、ISR和ALARMCALLBACK用于标准入口声明／定义；TASK与ISR分别用OS_TASK_ENTRY／OS_ISR_ENTRY绑定实际配置。CounterType和FreeRTOS TickType均为uint32；Rte_Os_Type.h的TimeInMicrosecondsType为uint64。所选生成Counter提供对应常量及单位转换宏；转换参数只求值一次，合法范围0..UINT32_MAX，整数秒向下截断。

时间由controlled_logical_ms逐毫秒请求／确认推进；软件Counter与内核tick分别保持模数，宿主看门狗不推进汽车时间。实际Waiting及输入／tick／输出确认决定提交完成。GetISRID读取真实ISR帧；单核ControlIdle支持IDLE_NO_HALT且省略CoreID检查。isOsStarted按R24-11的DRAFT定义表示进入过StartOS，而非Ready。

生成输入须在唯一OsOS中明确声明OsUseResScheduler及受支持状态／类／错误参数开关，OsHooks直接归属该OsOS。参考值false保持无虚拟scheduler资源；true生成全部本核Task可访问、无ISR访问、天花板为最高Task优先级的RES_SCHEDULER。启用时同名显式OsResource按SWS_Os_00850被忽略，虚拟实例取代其配置；生成剖面不接受其他未映射的显式资源。原生Os_TargetConfig仍支持已验证的Task／ISR资源及四一致性类容量。

Windows物理栈由实际线程、保护页和保证区验证。Linux OS 使用每个 Task／唯一 S dispatcher／双控制线程／宿主 actor 的独立 mmap+pthreads 栈，低端 PROT_NONE guard 与预留 sigaltstack；Task 栈和 FreeRTOS 的 512-word 元数据缓冲分开。普通停车使用 SIGRTMIN 的代际确认与 futex，仅在 port critical／宿主锁持有期间屏蔽；独立 SIGRTMIN+1 无锁故障停止。S 独占 ISR、受控 tick 与唯一内核任务选择器；host producer 只发布边沿，不能执行 ISR。仅注册地址保护页内的 SIGSEGV/SIGBUS 才归类栈故障，损坏 actor 停在 altstack，健康 C/D 停止其他 actor 并报告；未知故障或双控制失败非零 fail-closed，不在信号 handler 中调用 Hook／stdio。两目标均不推定 MCU 电气层或硬实时性。

## 构建与测试

在各自目标的原生 Windows 或 Ubuntu24.04 x86_64 环境、根目录已执行 `uv sync --locked --group quality` 后，运行相应独立 C99 消费者：

```sh
uv run --locked python -m autosar_tooling os --target windows-x64-controlled-v1 --suite all
uv run --locked python -m autosar_tooling os --target linux-x64-controlled-v1 --suite all
```

目标必须与当前执行宿主一致；可把 `all` 换成 `lifecycle`、`stack`、`nested-interrupts`、`time` 等共 26 个 suite。正式 core 集成测试也分别注册同一 26 个消费者，Rust 执行前设置绝对路径 `AUTOSAR_PYTHON`，所有编译器／原生进程经过有界 owner，失败保留独立工作区、stage/vector、stdout/stderr 和 PID 状态。Linux 单个 harness 最多 10 秒，自动 tick 和 Windows timer 不能驱动汽车时间；每次接受的 1ms epoch 只推动一次实际 FreeRTOS tick。原生 stack suite 还运行独立 ARTI C 消费者和 ELF64 符号／ucontext 断言，Windows PE/TLS 与现有真实栈合同不降级。验收证据与阶段状态在本轮 BMad spec，而非额外共享报告。

真实 dispatcher 使用32槽Os_InterruptVectorTable；Windows生成工程交付该源码及第14个受控补丁，Linux OS harness 从固定 POSIX 原件应用受控补丁。两目标都链接到256字节可重定位、可写的.os_vec段，分别检查PE或ELF的实际符号与段绑定。

标准入口通过OS_START_SEC_CODE／OS_STOP_SEC_CODE与重复包含Os_MemMap.h声明，固定GCC目标的OS_CODE属性将Task、ISR、AlarmCallback及Hook放入只读可执行.os_code段；嵌套、错配、冲突和未支持标记明确拒绝。先包含Os.h，再开始映射，结束后再次包含Os_MemMap.h。参考配置没有SwAddrMethod位置引用，使用默认CODE。

生成工程的include/Rte_Os.h与src/Rte_OsService.c提供同步Counter服务。Rte_Call_OsService_GetCounterValue与Rte_Call_OsService_GetElapsedValue绑定配置的Counter句柄，直接调用原生OS接口；输出为tick及tick差（SWS_Os_00560），不按TimeInMicrosecondsType名称额外换算。服务器OsService_{Counter}_GetCounterValue／GetElapsedValue接受CounterType端口定义参数。os/Os_Service.arxml描述OsService提供端口、服务接口、共享类型、错误及Runnable；受支持的单Counter配置句柄为0。调用方拥有输出存储，拒绝调用保留原生错误、ErrorHook参数快照与输出契约。

## ARTI observation

The generated ECU includes `os/Os_Arti.arxml`, R24-11 ECUC descriptions of its actual OS, logical core, Task, input Cat2 ISR, native stacks/context, Counter, configured Alarm/ScheduleTable and optional scheduler resource. Expressions use global data and constants; Task observations are refreshed from the native kernel under its mutex, while timer/resource pointers refer to original storage. These values never select runnable work. The delivered build retains debug type information for those expressions.

`Os_Arti.h` includes the tool-provided `Arti.h` first. The binding calls standard `ARTI_TRACE` with literal context, class, configured OS SHORT-NAME and event tokens. It covers basic Task transitions, paired Cat2 events, the five configured OS Hooks, and application service entry/return. Internal timer/table/mailbox actions are excluded from application service events. Successful Terminate/Chain, StartOS and ShutdownOS have no fabricated Return event.

This host tool binding retains at most 4096 `Arti_Events` in memory; it allocates no actor storage and performs no blocking I/O. Read records after actor quiescence, respecting each slot's `published` flag. `Arti_EventsDropped` reports omitted events and saturates at INT32_MAX on both hosts. Getter records retain the actual status, so an invalid result cannot be confused with a genuine UINT32_MAX value. Pointer returns carry the standard uint32 token plus the full x64 address in tool side data. Logical actor and Hook scopes preserve this captured data across nesting. Linux ARTI exposes `linux-x86_64-ucontext`, actual guard/base/high/altstack, distinct kernel metadata buffers and a valid flag only after a captured park; it does not claim Windows TEB/commit-growth/guarantee equivalence. `Arti_Init` runs before actors start; `Arti_GetVersionInfo` reports this tool binding's 1.0.0 version with unassigned vendor/module IDs 0. A null version pointer raises ARTI_E_PARAM_POINTER in the debugger-visible development-error cell. The binding does not write report files.
