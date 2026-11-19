# 覆盖率报告

覆盖率用于定位未执行路径，不设百分比门槛，也不能单凭相同行覆盖率删除测试。先准备[平台依赖](../environment.md)，从仓库根目录运行：

```powershell
cargo install cargo-llvm-cov --version 0.9.1 --locked
rustup component add llvm-tools-preview
npm ci --prefix ui
npm run build --prefix ui
cargo llvm-cov clean --workspace
cargo llvm-cov --locked --workspace --no-fail-fast --no-report
cargo llvm-cov report --html --output-dir build/coverage/rust
cargo llvm-cov report --json --output-path build/coverage/rust/coverage.json
npm run test:coverage --prefix ui
uv sync --locked --group coverage
uv run --locked --group coverage coverage run -m unittest discover -s tests/python
uv run --locked --group coverage coverage html
uv run --locked --group coverage coverage json
```

HTML 入口分别是 `build/coverage/rust/html/index.html`、`build/coverage/ui/index.html`、`build/coverage/python/html/index.html`；同目录的 JSON 用于按文件查看未执行行，UI 另有 `lcov.info`。输出位于已忽略的 `build/`，不提交报告。Rust 每次独立测量先显式清理旧测量数据（`--no-report` 会保留数据用于后续报告，不能依赖隐式清理）；Python 与 Vitest 默认覆盖上次结果，避免把不同版本的执行混在一起。

Rust 测量 core 与 src-tauri 默认测试的行、区域和函数，不宣称分支覆盖；稳定工具链不测量 doctest；不包含 `native-tests`、`official-oracles`、原生 GUI／IPC 或生成 C 运行时。需要单独调查原生路径时，可在同一 Cargo 命令追加 `--features native-tests`，先准备[原生工具](native.md)，将结果与默认报告分开保存。

V8 插桩入口将并发 worker 限为 2，避免多个 jsdom 工作台测试与插桩转换同时争用资源；测试集合、断言和期限与普通测试相同。V8 测量 `ui/src` 的 TS／TSX，显式包含未导入文件；Python 测量 `tools/python/src` 的包，启用分支覆盖并包含未导入模块，普通 discover 不递归运行独立的 native／process 套件。测试辅助、第三方依赖及构建生成文件不作为产品分母；内联 Rust 单测仍受 LLVM 的源码映射影响。

## 运行时与生成 C

在 Linux 上准备[原生测试锁定的 GCC 13 与 binutils](native.md)，从仓库根目录运行：

```bash
uv run --locked --group coverage python -m autosar_tooling.c_coverage
```

打开 `build/coverage/c/index.html` 查看行、分支覆盖率和标注后的源码。相邻的 `coverage.json` 保留逐行计数和分支结果。重复测量时用 `--output` 指定一个新目录。测量使用临时构建目录和独立计数目录，不读取以前的计数。

测量运行 Cargo 的运行时契约、主机、两项 HostBatch 协议测试及生成 RTE 消费者，并运行 Python 的 OS suites。封存工程先通过原有生产构建，再用同一组输入编译单独的插桩 HostBatch。插桩的 OS 与 HostBatch 在成功调用 `_exit` 前保存计数，错误退出或外部强制终止时可能丢失计数。

报告按 BSW 模块、OS 核心、主机层及 ECU 支撑代码分组，交付中与仓库字节一致的运行时副本归入对应分组。分母来自实际插桩的自有 C 和含可执行代码的头文件，排除 FreeRTOS、测试夹具和应用源码。生成文件按路径与内容摘要区分。gcovr 先按每个可执行程序读取分支，源码和分支布局一致时才合并计数。一个源文件出现多个布局时，每种布局各有源代码页，并各自计入汇总。

GCC 13 偶尔输出负计数。gcovr 会逐处告警并继续生成报告，在日志和工件中的 `gcov-warnings.txt` 保留源码位置与原始数值。分诊拒绝或错误分支前，先核对对应位置是否有计数告警。

独立的 `Coverage reports`（`test-coverage.yml`）在 master 推送及手动运行时生成四份 Linux 工件，保留 14 天。C 作业仅在 master 运行。该工作流不在 PR 上自动运行，也不接入 `CI required`。测试失败仍使报告作业失败，可渲染的数据会随工件上传。查看失败运行时，先核对运行结果、提交和测试范围，再结合源码解释 JSON 中的未执行路径。

