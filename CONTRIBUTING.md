# 参与开发

项目使用 Rust、React/TypeScript、Tauri 和 Python。依赖版本由各包配置和锁文件维护；开发使用 Node 24、npm 11、Python 3.12 或更新版本，Rust 由 rustup 读取 `rust-toolchain.toml`。

## 开始开发

在仓库根目录运行：

```powershell
npm ci --prefix ui
npm run dev --prefix ui
```

桌面调试使用 `npm run tauri --prefix ui -- dev`。先按[环境说明](docs/development/environment.md)准备平台依赖。开发 Python 工具运行 `uv sync --locked`；仅开发界面无需 Python、官方档案或 ECU 编译器。

## 检查改动

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked --manifest-path core/Cargo.toml
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
uv run --locked python -m autosar_tooling quality --base <基准提交>
```

默认 Cargo 测试不要求官方资源或受控原生工具。真实进程、官方对照和桌面验收见[测试说明](docs/development/testing.md)。格式检查针对修改行，不整体重排历史代码。

解析、生成和运行行为的修改应有独立预期及关键拒绝路径。修改 C 或生成 C 模板时按仓库 MISRA 指导执行；公开 C 接口说明输入、输出和错误契约。

## 资源与提交

修改交付资源后运行 `uv run --locked python -m autosar_tooling assets check`。确认自有源码差异后可显式执行 `assets update`；BSW ABI 和第三方身份不能随来源摘要自动接受，见[资源维护](docs/maintainers/assets.md)。

提交前检查整个 diff，包括新增文件与生成内容。标题说明实际改动；PR 说明问题、改动、验证和未运行范围。界面改动附实际截图。不要提交官方下载、个人配置或临时测试产物。主机测试的结论限定为主机能力。

真实 GUI 验收使用隔离环境，不占用开发者正在使用的桌面。BMad 用于项目规格与状态，不是启动或修改工程的前置条件。

## 许可与安全报告

原创贡献按 [Apache-2.0](LICENSE) 提供，第三方内容保留原许可；加入依赖或代码前确认有权分发。具体分发要求见[许可说明](docs/maintainers/licensing.md)。安全问题按 [SECURITY.md](SECURITY.md) 报告，普通问题可提交包含复现步骤和预期结果的 Issue。
