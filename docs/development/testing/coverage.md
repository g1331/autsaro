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

独立的 `Coverage reports`（`test-coverage.yml`）在 master 推送及手动运行时生成三份 Linux 工件，保留 14 天；它不在 PR 上自动运行，不接入 `CI required`，不影响必需检查。测试失败仍使报告作业失败，后续渲染与上传保留已收集的数据；失败运行的报告可能不完整，查看时先核对运行结果、提交和测试范围。下载工件后打开各 HTML 入口；以 JSON 中的未执行错误／拒绝路径结合源码分派给对应模块 Issue，不将 Linux 默认测试覆盖率外推到其他平台、原生套件或完整符合性。

