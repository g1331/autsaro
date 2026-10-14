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

`native` 任务在 Windows 与 Linux 上以 `--features native-tests --lib --test native --no-fail-fast` 运行全部原生 Rust 套件（Windows 按测试名把最慢的几个模块分到第二个 runner，同一名单在另一半用 `--skip` 排除，每个测试只运行一次），工具路径取自已安装的受控 GCC、binutils、Git 及项目 uv 环境；`native-os` 任务在两个平台运行 26 个 OS suites 和主机锁错误边界测试，Windows 用 `AUTOSAR_OS_SHARD=k/n` 把 suite 按序号分到两个 runner，每个 suite 仍单独串行运行。`native-delivery` 任务下载准备任务发布的样本，与样本分析并行运行封存工程构建模式和 CMake 构建，再把 `standard-ecu` 样本移到新目录，用 uv 管理的 Python 3.11–3.14 接收者解释器（`AUTOSAR_RECEIVER_PYTHON`）逐个直接运行包内工具完成构建与验证，不向接收者解释器安装开发工具；这只证明解释器兼容性，不代表网络隔离下的安装验收。`official` 二进制（`acceptance.yml`，见[官方对照](official.md)）与桌面场景（`desktop.yml`，见[桌面场景](desktop.md)）由不属于必需检查的 `Extended checks` 按改动路径调用（见 [CI](README.md#ci)）；PR 检查通过不代表这些检查通过。

## 检测器构建

Linux 上用 GCC 的 ThreadSanitizer、AddressSanitizer（含 LeakSanitizer）和 UndefinedBehaviorSanitizer 检查直接编译的原生测试程序，ASan／UBSan 另外检查 OS suites。`AUTOSAR_SANITIZE` 指定检测器（`thread` 或 `address,undefined`），测试辅助据此为这些程序加入 `-fsanitize=…`、`-fno-sanitize-recover=all` 与调试信息；任何报告都使程序以错误结束，测试随之失败。

```bash
export AUTOSAR_SANITIZE=thread   # 或 address,undefined
export ASAN_OPTIONS=use_sigaltstack=0:detect_stack_use_after_return=0
setarch x86_64 -R cargo test --locked -p autosar-config-core --features native-tests --test native --no-fail-fast -- runtime_contracts:: host:: integration::generated_c:: integration::os::counter_header_oracles sanitizers::
# 仅 address,undefined
setarch x86_64 -R uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py
```

- `setarch -R` 关闭地址随机化：Ubuntu 24.04 的 `vm.mmap_rnd_bits=32` 使 GCC 13 的 TSan 报告 `unexpected memory mapping`，这样不必修改宿主设置。
- OS 主机层自己安装备用信号栈，并用 ucontext 切换栈，所以 ASan 不接管备用信号栈（`use_sigaltstack=0`），也不把局部变量移到假栈（`detect_stack_use_after_return=0`）；后者使嵌套中断用例比较栈地址失真，代价是不检查返回后使用栈变量。
- OS suites 不在 TSan 下运行：TSan 把异步信号处理推迟到它自己的拦截点执行，FreeRTOS POSIX 端口用 `SIGRTMIN` 暂停 actor 并取得被中断的上下文，这一机制在 TSan 下超时（`E_OS_STATE`）。TSan 覆盖 BSW 独占区与主机锁的真实线程用例（`runtime_contracts::`）；它在 OS suites 中报告过信号处理函数改写 `errno` 的问题，已修复。
- `sanitizers::` 的故意越界写（`memcpy` 超出固定静态缓冲）、数据竞争和超宽移位证明检测器确实报错；未设置 `AUTOSAR_SANITIZE` 时该测试不做任何事。
- 经 `ecu-tool` 封存构建的 ECU（`integration::protocol`、`sealed_projects` 等）不插桩：交付工具只接受 `target.json` 锁定的编译选项，并核对段布局。只列出直接编译程序的模块，避免重复运行未插桩的构建；HostBatch 集成路径的检测器覆盖见 [#140](https://github.com/g1331/autsaro/issues/140)。

必需 CI 的 `native-sanitizers` 任务在 Linux 上按 `thread` 与 `address,undefined` 各用一个 runner 运行上述命令；与 `native`、`native-os` 并行。
