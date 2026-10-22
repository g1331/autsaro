# 进程与原生行为

```powershell
uv run --locked python -B -m unittest discover -s tests/python/process
cargo test --locked -p autosar-config-core --lib --features native-tests execution::
cargo test --locked -p autosar-config-core --features native-tests --lib --test native --no-fail-fast
uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py
cargo test --locked -p autosar-config-core --features official-oracles --test official --no-fail-fast
```

进程测试需 `AUTOSAR_PYTHON` 指向支持的绝对解释器路径；受管进程的探针是测试辅助脚本 `tests/python/process/probe.py`。官方对照的档案准备见[官方对照](official.md)；native 测试需对应平台的受控 GCC/binutils。OS suites 由 unittest 按 suite 生成测试，可用 `-k stack` 等筛选，只在与当前宿主一致的目标上运行；测试名对应 suite，进程失败信息包含阶段、命令和原始退出状态。

需要保留 OS 的编译产物和进程日志时，设置 `AUTOSAR_OS_OUTPUT_DIRECTORY` 为新的结果目录，unittest 按 suite 保存到该目录。已有目录不会被覆盖；未指定时仍只保留失败现场。

`native` 任务在 Windows 与 Linux 上以 `--features native-tests --lib --test native --no-fail-fast` 运行全部原生 Rust 套件（Windows 按测试名把最慢的几个模块分到第二个 runner，同一名单在另一半用 `--skip` 排除，每个测试只运行一次），工具路径取自已安装的受控 GCC、binutils、Git 及项目 uv 环境。失败的测试保留自己的 scratch 目录（输出 `scratch retained at …`），独立 ECU 校验器保留其诊断目录，失败消息附带子进程输出末尾；该任务把临时根目录作为 `native-scratch-*` artifact 在失败时上传（保留 14 天），与 OS suites 的 `os-suites-*` 相同；`native-os` 任务在两个平台运行 26 个 OS suites 和主机锁错误边界测试，Windows 用 `AUTOSAR_OS_SHARD=k/n` 把 suite 按序号分到两个 runner，每个 suite 仍单独串行运行。`native-delivery` 任务下载准备任务发布的样本，与样本分析并行运行封存工程构建模式和 CMake 构建，再把样本移到新目录：Python 3.11 运行全部四个样本，Extended 在两平台运行完整版本与样本矩阵。用 uv 管理的接收者解释器（`AUTOSAR_RECEIVER_PYTHON`）逐个直接运行包内工具完成构建与验证，不向接收者解释器安装开发工具；这只证明解释器兼容性，不代表网络隔离下的安装验收。`official` 二进制（`acceptance.yml`，见[官方对照](official.md)）与桌面场景（`desktop.yml`，见[桌面场景](desktop.md)）由不属于必需检查的 `Extended checks` 按改动路径调用（见 [CI](README.md#ci)）；PR 检查通过不代表这些检查通过。

## 检测器构建

Linux 上用 GCC 的 ThreadSanitizer、AddressSanitizer（含 LeakSanitizer）和 UndefinedBehaviorSanitizer 检查直接编译的原生测试程序，两种检测器还检查 OS suites 和封存 ECU 的 HostBatch／probe／test 闭包。`AUTOSAR_SANITIZE` 指定检测器（`thread` 或 `address,undefined`），测试辅助据此为这些程序加入 `-fsanitize=…`、`-fno-sanitize-recover=all` 与调试信息；任何报告都使程序以错误结束，测试随之失败。

```bash
export AUTOSAR_SANITIZE=thread   # 或 address,undefined
export ASAN_OPTIONS=use_sigaltstack=0:detect_stack_use_after_return=0
setarch x86_64 -R cargo test --locked -p autosar-config-core --features native-tests --test native --no-fail-fast -- runtime_contracts:: host:: integration::generated_c:: integration::os::counter_header_oracles sanitizers::sanitizer_lane_reports_deliberate_faults
setarch x86_64 -R cargo test --locked -p autosar-config-core --features native-tests --test native --no-fail-fast -- integration::protocol:: integration::sealed_projects:: integration::sealed_sanitizers::
setarch x86_64 -R uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py
```

- `setarch -R` 关闭地址随机化：Ubuntu 24.04 的 `vm.mmap_rnd_bits=32` 使 GCC 13 的 TSan 报告 `unexpected memory mapping`，这样不必修改宿主设置。
- OS 主机层自己安装备用信号栈，并用 ucontext 切换栈，所以 ASan 不接管备用信号栈（`use_sigaltstack=0`），也不把局部变量移到假栈（`detect_stack_use_after_return=0`）；后者使嵌套中断用例比较栈地址失真，代价是不检查返回后使用栈变量。
- OS suites 在两种检测器下运行，覆盖真实暂停／恢复、嵌套中断、栈上下文与故障关闭。Linux 的已暂停 actor 从现有请求状态读取终止意图，保持首次捕获的上下文并确认永久停止；运行中的 actor 仍由终止信号停止。控制器在 actor 停止后才关闭端口调度状态，关闭主机线程句柄时释放 pthread 资源并保留注册栈身份。
- `sanitizers::` 的故意越界写（`memcpy` 超出固定静态缓冲）、数据竞争和超宽移位证明检测器确实报错；未设置 `AUTOSAR_SANITIZE` 时该测试不做任何事。 自检在 `-O0`／`-O2` 下分别要求故障报告与非零退出、干净对照成功。竞争夹具先创建观察线程，再写入 volatile 非原子值，以 relaxed 原子发布安排读取而不建立 happens-before；干净对照仅将发布改为 release，与观察线程的 acquire 配对。volatile 只保留访问，不提供同步；不能用循环次数或后续绿色运行证明历史漏报原因。编译与子进程沿用受管执行入口，失败 scratch 与原始输出由 `sanitizer-scratch-*` 上传，覆盖直接原生与封存检测器两步。
- 封存 ECU 的检测器路径用于 Linux TSan 与 ASan／UBSan：测试辅助先完整执行包内生产 build（含 seal、固定工具、选择补丁、内核摘要与严格段布局检查），记录实际编译 argv，再用同一源码、include 顺序、补丁后内核、test 模式宏与链接参数额外加入检测器选项，编译到独立程序。测试辅助不得在加载原 guard 前加入包内 Python 搜索路径，以免未封存的同名标准库模块提前执行。生产 `ecu-tool`、`target.json` 和 object 检查不变；检测器程序不作为交付程序。共享 HostBatch 按检测器选项区分 image，消费者仍拿独立副本。每个新输入多编译一次，后续相同 HostBatch 复用；不缓存跨运行的通过结果。
- `integration::protocol::` 与 `integration::sealed_projects::` 在该路径运行现有协议向量和封存工程 owner 用例；`integration::sealed_sanitizers::` 验证 probe 实际完成 20 ticks、test 模式故障退出、完整编译 argv 只差检测器选项／输出路径、封存输入不变及共享副本隔离。外部 probe control 复用既有故意越界／超宽移位夹具，要求实际运行报告与非零退出，并让合法边界输入成功。
- `integration::sealed_sanitizers::` 在两种检测器下持续选择真实线程注册／中断身份回归，以及暂停／恢复进展、终止前后上下文、拒绝终止后恢复、运行 actor 停止和关闭句柄后 pthread 状态的成对回归。`HostBatch`、probe 与 test 使用相同封存源码，生产／插桩 argv 只允许检测器选项与输出路径不同。故意越界／超宽移位 control 仅在 ASan／UBSan 下运行；直接线程 TSan 测试继续运行。`generated_c` scaffold 的生产 ECU 编译前置步骤保持不插桩，其独立 C consumer 仍用 TSan。

必需 CI 的 `native-sanitizers` 任务在 Linux 上按 `thread` 与 `address,undefined` 各用一个 runner 运行上述命令；与 `native`、`native-os` 并行。

## 平台与路径

- 公共开发与交付流程不绑定本机 Windows 路径、代理会话或隐藏环境；源码、条件编译和公共命令按真实能力或平台命名。解析工具输出的相对路径或 Windows 无盘符根路径时，使用工具实际执行目录；涉及临时目录的检查覆盖跨盘情形，不沿用调用方的工作盘。
- 修改测试共用的临时目录或路径辅助时，确认 Windows 上的 native 套件通过：MinGW 编译器不接受 `\\?\` 规范化路径，需要比较工作区报告路径的测试应自行规范化（桌面场景按界面显示形式比较、预览原样回传，按文件名而非 `/` 后缀筛选文件）。
- 修改文件保存的发布原语时，同时核对回滚、清理和遗留恢复资料：在移动原文件前验证原语可用性，避免“读取确认后删除”的竞态；不能将 POSIX 的目标类型限制推定为 Windows 保证，按锁定工具链在实际平台验证文件／目录替换。硬链接遗留会共享正式文件内容，须明确恢复语义，并用真实多文件事务验证失败后的字节与基线。
- 原生 CI 使用交付目标锁定的工具链，不用滚动仓库最新版替代或放宽身份校验。测试以短期限调用 `nm` 等 binutils 时显式指定 BFD 目标（如 `--target=elf64-x86-64`）：Ubuntu 的 `/usr/lib/bfd-plugins` 带 LLVMgold，格式探测会加载 libLLVM，runner 上首次冷加载可超过数秒；外部工具超时先确认它实际加载的内容，不直接归为并行负载。

原生夹具按消费者能力放在 `core/tests/fixtures/` 的子目录：`host-batch/` 保存 HostBatch 协议与帧计时控制，`os/` 保存 OS 控制、ARTI 读取及生命周期程序，`sanitizers/` 保存共享故障注入程序，`implementation-descriptions/` 保存实现描述读取程序。OS 控制文件之间的相对 include 保持同目录；移动夹具时同步 Rust、Python 和 official 消费方。

Tauri 的 Windows MSVC 单测也需要 Common Controls v6 manifest：命令元数据收集可能把窗口运行时符号链接进 libtest，而上游默认只给应用二进制嵌入 manifest。`src-tauri/build.rs` 为应用与测试统一链接同一份 manifest，正常 `cargo test` 直接可用；遇到 `0xc0000139` 且零测试执行时先检查加载与 manifest，不能算成测试失败用例或靠跳过测试处理。

## 运行时配置结构与源文件迁移

- 给运行时配置结构（如 `Os_TargetConfigType`、`Os_ResourceConfigType`）加字段时放在末尾，并更新全部位置式初始化：`runtime/*/tests/*.c`、生成器按位置输出的 `c_layout::initializer` 与模板。这些单元以 `-Werror=missing-field-initializers` 编译，生成工程只在 `--features native-tests` 的原生套件中编译，`it` 通过不能证明。测试用 `#define` 给 hook 改名再包含另一个 harness 时，宏也会改写被包含文件里对该 hook 的取址；改由 harness 提供显式观察点。
- 在 runtime 目录之间迁移 C 源文件或头文件时，从生成样本按每个实际 main 用 `gcc -MM` 列出编译闭包，并与 `integration.json`、ownership ledger 和交付字节逐项核对。GCC 的引号 include 优先查找同目录，迁移使不同目录出现同名头文件时，其解析结果可能改变：交付工程是扁平 include，仓库内按目录拼路径的夹具、工具和 `quality` 检查则不同，要逐个复核；也要检查按目录 glob 的检查是否悄悄漏掉了迁出的文件。原生测试的生成头文件通过实际消费者包含，不用全局 `-include` 注入宿主适配编译单元，避免改变 Windows 系统类型与平台特性宏的包含顺序。
- 改动交付文件布局（如头文件移入子目录）、原生 fixture 或其共用辅助，重排运行时源码或改变 emitter 排版时，推送前在本地以 `--features native-tests` 运行受影响原生套件，并运行 OS suites、`test_host_boundaries.py` 和带改名变体（如 `OsAlt`）的原生套件。修改 OS 或 BSW 运行时 C 后，推送前也按[检测器构建](#检测器构建)运行受影响套件：插桩改变寄存器分配，`setjmp` 所在函数的 `-Wclobbered` 等告警只在该构建中出现。

## 主机时序与间歇失败

- 主机侧逐 tick 握手（如 HostBatch 等待 tick 完成与输出确认）不能依赖短超时轮询推进：FreeRTOS Windows 端口补丁移除了 `timeBeginPeriod`，Windows 上 1 ms 定时等待约 15.6 ms 才返回，Linux 宿主层则按 1 ms 轮询，问题只在 Windows 负载下表现为 watchdog 超时。对端有待处理工作时显式唤醒，并按单批墙钟预算核对。
- 宿主线程不得忙等：FreeRTOS 空闲 hook 每轮让出主机 CPU（`idle_state` 的 `host-yield` 场景），否则每个 ECU 进程占满一个 CPU，并在部分机型上拖慢跨线程交接。
- Windows 托管 runner 混用 AMD EPYC 9V45／9V74／7763 与 Intel Xeon 8370C／8573C，只在一种机型出现的时序失败（#61：9V45 上另一 vCPU 忙等时，跨线程唤醒有约 2 ms 停顿）要按 CPU 型号记录，在同一 runner 上配对比较变体；筛选机型用读取型号后立即结束的小矩阵，不用大矩阵随机抽样占用共享 runner。
- 子进程失败消息须带 stdout／stderr 的内容，不能只给随 scratch 目录删除的日志路径：共用执行入口（`ProcessOwner::run`、`ecu_tools.process.run_bounded`）与测试的 `tooling::wait_success` 已默认附带输出末尾，新的失败路径沿用它们，不另写只报路径的错误。 调整共用执行入口时，保留各调用方的相对源对象与失败解释，并覆盖同一原因涉及的其他调用点；阶段名或宿主绝对路径不能替代这层上下文。排查 CI 失败先下载 `native-scratch-*`／`os-suites-*` 现场，不靠猜测重跑。
- 新增测试后无关用例在 CI 上稳定失败时，按完成时间比较失败与基线运行中重叠的测试，不当作偶发重跑；改变线程交接时机的修复可能暴露依赖原时序的竞态，以超过 CPU 数的并发重复运行受影响用例，比较修复前后的失败率，相互依赖的修复一起合入。
- 共享主机出现编译或 supervisor 启动超时、socket 不可用、`EAGAIN` 时，保留原始输出与资源快照，停止本任务被替代的检查，并与并行任务协调单个有界验收批次。资源快照本身不证明失败原因或容量已恢复；容量发生变化后仍用原入口、期限和断言验证，启动失败、中止与部分矩阵均不计为通过，不停止其他任务的进程。
- 间歇超时先区分挂起与接近期限：Linux 上并发重复到复现，超时时用 `gdb -p` 抓全部线程栈再结束进程，失败检查不带位置时先让它记录源码行。不能未经实测就放宽期限，也不用重试或固定延时掩盖竞态。
- 长时间原生验收启动前，核对后续消费者阶段所需的全部外层工具路径，保持应用配置阶段的工具隔离；不能把缺失前置条件留到末尾才发现。含大响应时，将完整事件逐条写入既有证据文件，避免在内存保留全部历史 snapshot；保留原有 payload、场景与断言，并分别核查驱动和产品进程的内存，不能用证据写入优化掩盖产品内存问题。

开发工具的 Git 文件／属性查询与 clang-tidy 的内核补丁应用使用同一个 `ecu_tools.process.run_bounded`，期限为 30 秒；失败包含阶段／样本、命令、原始 exit 与 stdout／stderr 末尾。c-check 的工具探测／补丁为 30 秒、单 TU 编译为 60 秒、分析为 600 秒；只有 Cppcheck 的正常诊断退出码 1 被作为分析结果保留，挂起和其他异常退出仍失败。对应挂起回归可用 `uv run --locked python -B -m unittest discover -s tests/python/process -p test_feedback.py` 单独运行。

## Windows 进程

受管 supervisor 启动以期限内的认证就绪响应为准，不能将 socket 文件出现当作已开始监听。Windows 上 Job 活动计数归零与进程对象进入已退出状态之间可能有窗口，关闭后的 PID 也会很快被复用：检查进程已关闭时按创建时间排除复用的 PID，并有界等待真实退出事件，不用零超时探测 PID 存活；Windows 的父 PID 在父进程退出后不更新，按父 PID 归属进程树时要求子进程创建时间不早于父进程。

## 封存构建复用

原生集成测试的生产 HostBatch 可在同一测试进程中复用：辅助按完整工程目录字节（含 seal、应用、目标与工具）及当前工具路径精确匹配，每份输入只构建一次。缓存只保留可执行文件的字节和权限，各消费者写入自己的普通文件，运行输出与日志保持独立；测试进程结束即失效。自定义 control、preflight、Build／Verify、离线搬迁及重封存拒绝测试仍独立构建。`shared_host_batch_builds_isolate_mutating_consumers` 实际改写消费者程序与工程，检查其他副本的协议结果，并确认修改过的封存输入不能复用成功构建。
