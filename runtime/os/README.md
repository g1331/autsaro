# 受控主机 Automotive OS runtime

内核固定为 FreeRTOS V11.3.1 commit `054e14f3397023aa83813a65aa065fc4597d481b`。Windows GCC 16.1.0 x64 使用十九个受控补丁；Ubuntu 24.04 x86_64 GCC 13.3.0 的独立 OS 测试程序使用同一内核、适用的公共补丁及 `patches/linux/` 中的受控端口补丁。内核取自 `third_party/freertos/`，原始文件、两份来源摘要和 MIT 许可保留，不在线获取其他版本。`toolchain.json` 与 `toolchain-linux.json` 分别核对编译器和目标身份，以及 objdump/nm 可执行文件的摘要（Windows 为 MSYS2 `mingw-w64-x86_64-binutils` 2.47-4）。本节所述阶段的 Linux 原生验证覆盖独立 OS 测试程序；当时完整 Linux ECU、生成工程和工作台运行仍待后续验证。

配置表的计数及模块索引使用uint32，容量仍由各OS_MAX_*限制；任务名为以NUL结尾的const uint8字节串，调用FreeRTOS时才转到原生字符接口。集成方须用当前头文件重新编译配置与消费者。

Linux 的暂停请求、确认与恢复使用同一代次：普通代次在 1..INT32_MAX-1 循环，0 表示尚未暂停，INT32_MAX 只表示不可逆停止。dispatcher 在既有中断 mutex 下串行提交普通请求；Task 创建的首次暂停先于 dispatcher 启动，独立控制线程仍可并发停止 actor。请求以 CAS 发布，避免覆盖先到的停止；确认与恢复按当前代次相等判断，信号 handler 返回前不清零旧恢复值。`NativeFacilityTests.test_suspend_generation_wrap_preserves_context_and_terminal_stop` 在测试副本中设置近上界初值，检查真实信号回绕、上下文与停止交错，并拒绝无界递增及覆盖停止的变异；这是 D4.1 的算术边界与失败预防证据。

Linux 主机调用、后端启动／关闭及中断准入将有持久副作用的操作放在独立语句或嵌套条件中；受控 POSIX 后续补丁 `linux/0002-sequence-persistent-side-effects.patch` 保留事件发布、mutex 交接与 ISR 重调度请求的原有条件和顺序。R13.5 为 Required、适用 C99（主文页113–114／PDF121–122）；volatile 读取也属于持久副作用（Glossary 页224／PDF232），TC1 Appendix J 页14／PDF20明确函数判断不依赖可能的实参或非局部对象值。因此空句柄保护、已知状态或包装调用不构成 RHS 例外，不能无条件提前执行原来受保护的系统调用或取走请求。本次改写没有增加共享状态、改变公开 ABI 或去掉 volatile；夹具保留普通、失败、容量、已结束线程和停止交错观察。固定 checker 的零条 R13.5 报告不能替代此项源码核查；采用内核及其他模块仍需分别完成处置。

R14.3（Required、C99，主文页119–120／PDF127–128）也覆盖条件运算符的第一操作数；生产配置中的恒假故障桩没有无限循环或 TC2.27–28 的 do-while 例外。backend 按既有 OS_STACK_TESTS／OS_HOST_FAILURE_TESTS 在完整 MemMap 配对外选择实现，生产分支直接执行 suspend／context／resume 和资源创建；测试分支保留一次性故障消费及原生失败检查。资源调用计数仍在每次实际尝试前递增，Linux 的非法 attributes 仍不计为尝试；lifecycle 消费者比较生产与注入构建的正常计数，并验证生产构建中的注入输入不生效。上下文观察仍先于恢复尝试，没有新增状态、配置或 ABI。

资源包装使用文件私有的 OS_RESOURCE_CREATE：测试失败时不求值原生创建表达式，其他路径只求值一次；计数独立于该选择。接收创建结果的普通函数会先执行实参，不能保留这一故障边界，因此不属于 D4.9 的函数／宏可互换条件（Advisory，主文页31–32／PDF39–40；AMD2.26、AMD3.6–8未改变本条件，未援引泛型例外）。两种宏定义都将表达式参数放在括号内，按 R20.7（Required，页159／PDF167）核查。

`SuspendThread` 的混合优先级表达式显式加括号，并只在函数末尾返回结果；依据 R12.1 主文页103–104／PDF111–112（含优先级表与表达式树）、AMD2.62 的前三行表格修订及 R15.5 主文页126／PDF134，两项在 C99 下均为 Advisory。拒绝、已暂停、成功与并发停止仍保留各自结果，未以理由记录代替这两处源码修复。固定 checker 的 R17.3 原始报告仍保留；主文页138／PDF146仅适用C90，C99的声明义务与GCC内建签名另按R1.1／R8.2／R8.4及实际编译核查。

三个 `__atomic_compare_exchange_n` 调用的 `weak` 参数均使用 `stdbool.h` 的 `false`，与 [GCC 原子内建契约](https://gcc.gnu.org/onlinedocs/gcc/_005f_005fatomic-Builtins.html) 的 Boolean 形参一致。R10.3（Required，主文页86–87／PDF94–95，Glossary 的 assignment 含实参传递）不允许把 essentially signed 的整数0作为Boolean实参；TC1的常量例外、TC2.20的case修正和AMD3.27的浮点例外均不提供该转换的例外。这里保留strong CAS语义，修复的是实参的essential type，非新增同步机制。

在[PR #371](https://github.com/g1331/autsaro/pull/371)核验的 `1b4c11ff` 八份双平台四样本中，R5.5 的47个原始位置对应46个名字：tasks.c的29处、queue.c的17处及受控task.h的extern一处。两处是pxCurrentTCB，其余为FreeRTOS API函数；相关tasks／queue／task.h／mpu_wrappers.h／FreeRTOS.h／FreeRTOSConfig.h字节在八样本间相同，所选端口和编译参数不覆盖cores／MPU默认值。configNUMBER_OF_CORES=1排除了pxCurrentTCB的SMP宏定义；portUSING_MPU_WRAPPERS=0排除了45个API的MPU宏别名。Linux标准ECU的实际GCC `-E -dD` 确认全部名字保留为标识符，且从未进入生效的宏定义历史（含后续undef的历史，非仅检查最终宏表）。据R5.5主文页53／PDF61、AMD2.39及Glossary（页223／PDF231）对条件编译排除代码的定义，这47处报告的同名宏条件不成立；不据此判定其他R5.5位置通过。

固定2.21.0 addon的最小对照仍对排除分支中的同名函数式宏报告R5.5；生效的同名宏、定义后undef而标识符继续存在两个真实违反对照也均报告。misra_5_5从directives收集名字时没有区分上述排除定义；因此保留这些原始报告与adopted／patched归属，在人工裁决中说明误报，不改addon、suppression或源码名字。

八份同配置工件中的 `arrayIndexOutOfBounds`（error，tasks.c:3554）均指向 `prvCreateIdleTasks` 的默认 `configIDLE_TASK_NAME="IDLE"`：源对象为5个char，目标 `cIdleName` 为16个char，单核保留1个终止位置，循环上界为15。实际只读取索引0..4；复制索引4的NUL后立即break，再在同一索引写目标终止符，不会读取报告中的索引14。R18.1（Required、C99，主文页143／PDF151）约束实际指针算术和数组索引；AMD2.75–76（页16／PDF23）及TC2.33–34（页11／PDF17）没有把此字符串判断改成无条件扫描到上界，也无需使用其对象表示注释作为例外。

Linux真实scheduler的ASan／UBSan对照正常到达Ready、stderr为空；仅在测试副本移除上述break，UBSan在同一读取行精确报告索引5越过char[5]并以1退出。这支持此报告的边界误报裁决，不是全部R18.1通过。Windows判断依据相同内核、宏配置和源码条件，未作Windows运行声明；保留原始error报告、adopted归属及扫描的passed=false，执行收据见PR #371。

Linux `Os_HostCreateActor` 的 `mmap` 使用 `MAP_PRIVATE | MAP_ANONYMOUS`、fd=-1、offset=0。[Linux mmap 契约](https://www.man7.org/linux/man-pages/man2/mmap.2.html)明确匿名映射忽略 fd，并建议使用 -1；去掉匿名标志后才需要有效文件描述符。固定 Cppcheck 2.21.0 的 posix.cfg 对第五参数无条件限定非负数，不能表达该 flags 条件，因此这里的 `invalidFunctionArg`（error）不构成已证实的无效参数调用。最小真实调用对照中，匿名映射可读写并正常 munmap，去掉匿名标志则以 EBADF 拒绝；固定模型对两者都报告同一诊断。保留原始 error、源码归属与 passed=false，不修改固定模型、调用参数或 suppression；此裁决不解决动态分配、映射失败处理或全部平台调用的人工义务。

同一构造函数在保护设置或 pthread 创建／属性设置失败后，必须检查映射回滚的 `munmap` 结果，再清空 mapping 和释放 reserved。D4.7（Required、C99，主文页30／PDF38；AMD2只增加C11适用性）要求有意义地检查错误信息，其“已证明不能报错”例外不能仅由地址／长度有效推出；Linux 契约还列有 `munmap` 的 ENOMEM。回滚失败时沿现有 Linux 原生致命失败约定 `ExitProcess(1)`，不把仍存活的映射遗失后作为普通 NULL 创建拒绝继续；正常成功及成功回滚的结果不变。实际 errno 夹具分别注入两处失败，检查精确位置回执、退出值和无正常收尾输出，并以恢复各处旧检查的编译副本证伪。此整改只覆盖这两个回滚点，不构成全部D4.7／D4.12处置或符合性结论。

Os_TargetConfigType 静态配置汽车 Task、资源、内部资源、Counter、Alarm、ScheduleTable、Hook 和逻辑中断。backend 适配 FreeRTOS，由内核决定 Running 状态、抢占和上下文切换；汽车层维护激活请求 FIFO、事件与资源约束，任务选择交给内核。

StartOS 成功后不返回。Task 正常终止或调用 ChainTask 时，通过原生入口移交；意外返回时报告 MISSINGEND，恢复遗留资源和屏蔽状态，再完成本次激活。PreTask／PostTask 随 Running 状态转换调用，Shutdown 不额外产生 PostTask。ErrorHook 提供标准服务标识与参数快照，Hook 内错误不会递归调用或覆盖外层快照。支持参考 EXTENDED 配置和显式 STANDARD 配置；两种模式均保留防御检查、非零结果报告、标准激活及 Alarm 告警。

Disable／Enable、嵌套All／OS配对按调用者保存和恢复状态。Cat1与Cat2有不同屏蔽语义；源操作改变实际pending／disabled位。嵌套ISR使用同一真实S线程的原生栈，返回恢复外层身份和资源；泄漏屏蔽／资源在Cat2退出清理。Os_IsrConfigType.entries可选32槽void身体表，用OS_ISR_ENTRY(name)绑定；私有0／1、Cat1、无优先级及已配置input mailbox槽不得指定身体，配置身体不能被raw handler替换。

TASK、ISR和ALARMCALLBACK用于标准入口声明／定义；TASK与ISR分别用OS_TASK_ENTRY／OS_ISR_ENTRY绑定实际配置。CounterType为uint32，汽车TickType为uint64，FreeRTOS的TickType_t为uint32。CounterType与TimeInMicrosecondsType由Rte_Type.h唯一定义，Rte_Os_Type.h包含该共享头。所选生成Counter提供对应常量及单位转换宏；转换参数只求值一次，合法范围0..UINT32_MAX，整数秒向下截断。

时间由controlled_logical_ms逐毫秒请求／确认推进；软件Counter与内核tick分别保持模数，宿主看门狗不推进汽车时间。实际Waiting及输入／tick／输出确认决定提交完成。GetISRID读取真实ISR帧；GetCoreID在任何上下文及StartOS之前返回OS_CORE_ID_0；单核ControlIdle支持IDLE_NO_HALT且省略CoreID检查。isOsStarted 按 R24-11 的 DRAFT 定义记录是否进入过 StartOS，Ready 状态另行判断。

生成输入须在唯一OsOS中明确声明OsUseResScheduler及受支持状态／类／错误参数开关，OsHooks直接归属该OsOS。参考值false保持无虚拟scheduler资源；true生成全部本核Task可访问、无ISR访问、天花板为最高Task优先级的RES_SCHEDULER。启用时同名显式OsResource按SWS_Os_00850被忽略，虚拟实例取代其配置；生成剖面不接受其他未映射的显式资源。原生Os_TargetConfig仍支持已验证的Task／ISR资源及四一致性类容量：`use_res_scheduler`为0时名为RES_SCHEDULER的配置资源与普通资源相同（SWS_Os_00854）；`linked`资源按`link`解析到同一根资源，须与根资源天花板和ISR访问一致，不得成环（SWS_Os_00855）。生成的ErrorHook访问宏`OS_USE_GET_SERVICE_ID`／`OS_USE_PARAMETER_ACCESS`按OsUseGetServiceId／OsUseParameterAccess取值，生成的ECU以`StartOS(OSDEFAULTAPPMODE)`启动。

Windows物理栈由实际线程、保护页和保证区验证。Linux OS 使用每个 Task／唯一 S dispatcher／双控制线程／宿主 actor 的独立 mmap+pthreads 栈，低端 PROT_NONE guard 与预留 sigaltstack；pthread 启动栈只负责一次进入该 mmap 执行栈，ASan 使用实际 fiber 生命周期；Task 栈和 FreeRTOS 的 512-word 元数据缓冲分开。普通停车使用 SIGRTMIN 的代际确认与 futex，仅在 port critical／宿主锁持有期间屏蔽；独立 SIGRTMIN+1 无锁故障停止。S 独占 ISR、受控 tick 与唯一内核任务选择器；host producer 只发布边沿，不能执行 ISR。仅注册地址保护页内的 SIGSEGV/SIGBUS 才归类栈故障，损坏 actor 停在 altstack，健康 C/D 停止其他 actor 并报告；未知故障或双控制失败非零 fail-closed，不在信号 handler 中调用 Hook／stdio。两目标均不推定 MCU 电气层或硬实时性。

## 构建与测试

在各自目标的原生 Windows 或 Ubuntu24.04 x86_64 环境、根目录已执行 `uv sync --locked --group quality` 后，运行相应独立 C99 消费者：

```sh
uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py
uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py -k stack
```

目标取当前执行宿主；`-k` 可选择 `lifecycle`、`stack`、`nested_interrupts`、`time` 等共 27 个 suite。编译器和原生进程由 owner 管理并设置执行期限，失败时保留独立工作区、stage/vector、stdout/stderr 和 PID 状态。Linux 单个 harness 最多 10 秒，自动 tick 和 Windows timer 不能驱动汽车时间；每次接受的 1ms epoch 只推动一次实际 FreeRTOS tick。原生 stack suite 还运行独立 ARTI C 消费者和 ELF64 符号／ucontext 断言，Windows 保留 PE/TLS 与原生栈检查。

真实 dispatcher 使用32槽Os_InterruptVectorTable；Windows生成工程交付该源码及第14个受控补丁，Linux OS harness 从固定 POSIX 原件应用受控补丁。两目标都链接到256字节可重定位、可写的.os_vec段，分别检查PE或ELF的实际符号与段绑定。

模块函数声明／定义、静态状态、常量与生成的 OS 配置通过成对段标记及重复包含 `Os_MemMap.h` 映射；配置常量使用 `CONFIG_DATA_PREBUILD`，固定实现常量使用 `CONST`；段内只含所属类别的对象，平台与测试条件位于配对之外。`OS_MEMMAP` 仅在配对期间有效；OSEK 入口宏保留 `OS_CODE`。代码使用只读可执行 `.os_code`，原生向量表保留可写、八字节对齐的 `.os_vec`；CLEARED、INIT 与 actor TLS 分别保持原有 C 启动值和线程归属。常量指针在 ELF 中使用重定位后只读区域，在 PE 中使用 `.rdata`。错误、嵌套、冲突和未支持标记均拒绝；完整配置仍由同一生成入口提供，不引入新生命周期状态。

生成工程的generated/include/Rte_Os.h与generated/src/Rte_OsService.c提供同步Counter服务。Rte_Call_OsService_GetCounterValue与Rte_Call_OsService_GetElapsedValue绑定配置的Counter句柄，直接调用原生OS接口；输出为tick及tick差（SWS_Os_00560），不按TimeInMicrosecondsType名称额外换算。服务器OsService_{Counter}_GetCounterValue／GetElapsedValue接受CounterType端口定义参数。generated/descriptions/Os_Service.arxml描述OsService提供端口、服务接口、共享类型、错误及Runnable；受支持的单Counter配置句柄为0。调用方拥有输出存储，拒绝调用保留原生错误、ErrorHook参数快照与输出契约。

## ARTI observation

The generated ECU includes `generated/descriptions/Os_Arti.arxml`, R24-11 ECUC descriptions of its actual OS, logical core, Task, input Cat2 ISR, native stacks/context, Counter, configured Alarm/ScheduleTable and optional scheduler resource. Expressions use global data and constants; Task observations are refreshed from the native kernel under its mutex, while timer/resource pointers refer to original storage. These values never select runnable work. The delivered build retains debug type information for those expressions.

`Os_Arti.h` includes the tool-provided `Arti.h` first. The binding calls standard `ARTI_TRACE` with literal context, class, configured OS SHORT-NAME and event tokens. It covers basic Task transitions, paired Cat2 events, the five configured OS Hooks, and application service entry/return. Internal timer/table/mailbox actions are excluded from application service events. Successful Terminate/Chain, StartOS and ShutdownOS do not return, so no Return event is recorded.

This host tool binding retains at most 4096 `Arti_Events` in memory; it allocates no actor storage and performs no blocking I/O. Read records after actor quiescence, respecting each slot's `published` flag. `Arti_EventsDropped` reports omitted events and saturates at INT32_MAX on both hosts. Getter records include the status to distinguish an invalid result from a valid UINT32_MAX value. Pointer returns carry the standard uint32 token plus the full x64 address in tool side data. Logical actor and Hook scopes preserve the captured data across nesting. Linux ARTI exposes `linux-x86_64-ucontext`, guard/base/high/altstack addresses, separate kernel metadata buffers and a valid flag set only after a captured park. Windows TEB, commit-growth and guarantee observations are specific to the Windows target. `Arti_Init` runs before actors start; `Arti_GetVersionInfo` reports version 1.0.0 with the standard module ID 5 (BSWGeneral Appendix A.2) and unassigned vendor ID 0. A null version pointer raises ARTI_E_PARAM_POINTER in the debugger-visible development-error cell. Records remain in memory; the binding writes no report files.
