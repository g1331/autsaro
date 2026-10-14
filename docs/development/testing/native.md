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

`native` 任务在 Windows 与 Linux 上以 `--features native-tests --lib --test native --no-fail-fast` 运行全部原生 Rust 套件（Windows 按测试名把最慢的几个模块分到第二个 runner，同一名单在另一半用 `--skip` 排除，每个测试只运行一次），工具路径取自已安装的受控 GCC、binutils、Git 及项目 uv 环境；`native-os` 任务在两个平台运行 26 个 OS suites 和主机锁错误边界测试，Windows 用 `AUTOSAR_OS_SHARD=k/n` 把 suite 按序号分到两个 runner，每个 suite 仍单独串行运行。`native-delivery` 任务下载准备任务发布的样本，与样本分析并行运行封存工程构建模式和 CMake 构建，再把 `standard-ecu` 样本移到新目录，用 uv 管理的 Python 3.11–3.14 接收者解释器（`AUTOSAR_RECEIVER_PYTHON`）逐个直接运行包内工具完成构建与验证，不向接收者解释器安装开发工具；这只证明解释器兼容性，不代表网络隔离下的安装验收。`official` 二进制由不属于必需检查的 `acceptance.yml` 运行（见[官方对照](official.md)），桌面场景由不属于必需检查的 `desktop.yml` 运行（见[桌面场景](desktop.md)）；PR 检查通过不代表这些检查通过。
