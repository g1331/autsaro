# 测试

测试按实际前置条件选择，不需要恢复代理会话。

| 主题 | 内容 |
| --- | --- |
| [生成 C 的静态检查](generated-c.md) | Cppcheck／MISRA 分析、格式检查、封存构建模式与 CMake 构建 |
| [进程与原生行为](native.md) | 受管进程、原生 Rust 套件、OS suites 与原生 CI 任务 |
| [官方对照](official.md) | 官方档案下载与对照、条款处置核算 |
| [桌面场景](desktop.md) | Tauri WebDriver 驱动的真实应用与 IPC 场景 |

## 基础检查

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked -p autosar-config-core
cargo test --locked -p autosar-config-desktop
cargo clippy --locked -p autosar-config-core --all-targets --all-features -- -D warnings
cargo clippy --locked -p autosar-config-desktop --all-targets -- -D warnings
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
```

仓库根目录是 Cargo workspace（成员 `core` 与 `src-tauri`），共用根目录的 `Cargo.lock` 与 `target/`；`cargo test --locked --workspace` 一次运行两个 crate 的默认测试，需先准备桌面平台依赖。前端使用 Vitest；`npm run test:watch --prefix ui` 用于开发。Python 普通测试位于 `tests/python`，进程和平台测试独立选择。默认 Rust 测试不需要外部 Python 或官方档案。

core 的集成测试分为三个二进制，按能力模块组织，共用 `core/tests/support/`：

| 二进制 | 运行方式 | 前置条件 |
| --- | --- | --- |
| `it` | `cargo test -p autosar-config-core --test it` | 无 |
| `native` | `cargo test -p autosar-config-core --features native-tests --test native` | 目标 GCC／binutils、Git 与 `AUTOSAR_PYTHON` |
| `official` | `cargo test -p autosar-config-core --features official-oracles --test official` | 官方档案（见[官方对照](official.md)）与 `AUTOSAR_PYTHON` |

测试名带模块路径，可按名称筛选，例如 `cargo test -p autosar-config-core --test it -- integration::diagnostics::`、`cargo test -p autosar-config-core --features native-tests --test native -- integration::os::`。协议与 OS 独立对照向量（`core/tests/fixtures/oracles/`）由 `it` 的 `protocol_oracles::` 测试绑定到参考 ECUC 配置，设置或固定字面值漂移时失败。

Tauri 后端单测覆盖 Session、设置与请求归属，需先准备[桌面平台依赖](../environment.md)并运行 `npm run build --prefix ui`。本地 `verify --scope desktop`／`all` 及 PR 的 desktop job 均执行锁定 Cargo 单测、build 与 Clippy；PR 只在 Linux 运行 Clippy。这些检查不启动原生窗口，不能替代 GUI／IPC 行为验收。

Clippy 以默认规则集加 `-D warnings` 检查两个 crate，本地 `verify` 与 PR 使用相同参数。生成与集成模块（`core/src/generator.rs`、`core/src/integration/mod.rs`、`core/src/prepared.rs` 及其子模块）在模块属性中另外拒绝 `unwrap`、`expect`、`panic!` 以及 `core/clippy.toml` 列出的文本修补与文件嵌入：计划无法保证的前提返回 `PlanDiagnostic`，每个输出只渲染一次，受信模板经 `outputs::render` 编辑。确有理由的例外使用带 `reason` 的局部 `#[allow]`。启用门禁时其余模块遗留的告警以 `#[expect(.., reason = "clippy baseline #37: ...")]` 标记，`git grep -n "clippy baseline"` 列出剩余基线；修好告警后该 `#[expect]` 本身会报 `unfulfilled_lint_expectations`，须一并删除标记。CI 只在 Linux 运行 Clippy，Windows 专属的 `cfg` 代码不在门禁内。

## CI

`Development checks` 中的 `CI required` 是分支规则唯一要求的检查，阻塞合并的任务清单写在该任务的 `needs` 中，增删检查只改工作流。PR 只改根目录指南、`docs/`、代理说明或 PR 模板时，构建与测试任务整体跳过，`quality` 照常运行；其余 PR、推送到主干及手动运行都执行完整检查。分支无须先与主干同步即可合并，合并后主干推送会再完整运行一次。

## 离线 ECU 包

离线 ECU 包可由 Python 3.11 或更新版本运行自带 `tools/ecu-tool.py`，不需要 uv、Rust、Node 或源 checkout。构建和验证仍检查封存内容与目标工具链。支持版本的 Windows/Linux 实际构建由受控验收验证，未运行的平台或版本不宣称通过。
