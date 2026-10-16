# 测试

测试按实际前置条件选择，不需要恢复代理会话。

| 主题 | 内容 |
| --- | --- |
| [生成 C 的静态检查](generated-c.md) | Cppcheck／MISRA 分析、格式检查、clang 编译、封存构建模式与 CMake 构建 |
| [进程与原生行为](native.md) | 受管进程、原生 Rust 套件、OS suites、检测器构建、原生 CI 任务，以及平台路径、运行时文件迁移、主机时序与 Windows 进程的注意事项 |
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

## 依赖与 workflow 检查

```powershell
cargo deny --locked check
osv-scanner scan source --lockfile uv.lock --lockfile ui/package-lock.json
uv run --locked --group workflows actionlint
uv run --locked --group workflows zizmor .github
```

`cargo-deny` 按 `deny.toml` 检查 RustSec 漏洞与撤回版本、许可证白名单和依赖来源。未维护公告只对直接依赖失败：传递依赖中的这类 crate 来自 Tauri 的 GTK 3 与 URL pattern 依赖链，没有可迁移的版本。重复版本只告警，它们来自 Tauri 与 Windows crate 自身的版本分布。`osv-scanner` 按 OSV 数据库检查 npm 与 Python 锁文件。这两个工具需单独安装固定版本：`cargo install cargo-deny --version 0.20.2 --locked`，osv-scanner 2.7.0 从[官方发行页](https://github.com/google/osv-scanner/releases/tag/v2.7.0)下载；CI 用 `taiki-e/install-action` 按校验和安装同一版本。actionlint 与 zizmor 的版本固定在 uv 的 `workflows` 依赖组中。PATH 中有 shellcheck 时 actionlint 还会检查 `run` 脚本：GitHub 托管 runner 自带 shellcheck，本地缺少它就会漏掉 CI 报告的脚本问题。设置 `GH_TOKEN` 后，zizmor 还会在线核对固定的提交确实属于所引用的 action。

加入新许可证前先确认它与 Apache-2.0 兼容。确实无法修复的发现在出现处写明依据后再忽略（如 `# zizmor: ignore[github-env]`）。第三方 Actions 按提交 SHA 引用，行尾注释写对应版本。Dependabot（`.github/dependabot.yml`）每月（北京时间 03:00）为 Actions、Cargo、npm、uv 各提一个分组更新 PR，只提议发布满 7 天的 minor 与 patch 版本；大版本（以及 Dependabot 视作 minor 的 `sha2` 0.11）由人工升级，见 #153。Tauri 的 Rust crate 与 `@tauri-apps` npm 包须处于同一 minor 版本（否则 `tauri build` 报错），两侧的 minor 升级也由人工在同一个 PR 中完成，patch 更新照常提议。它不会自行 rebase，合并前评论 `@dependabot rebase`，避免每次 master 前进都让所有更新 PR 重跑完整 CI。clang-format 与 clang-tidy 在同一分组中；`quality` 要求 `uv.lock` 中两者版本相同，只升级其中一个的 PR 会失败。`verify --scope all` 与必需 CI 的 `quality` 任务都运行这四项检查。

修改 workflow 后用 YAML 解析器加载一遍，再运行上面的 actionlint 与 zizmor：以冒号结尾的 Cargo 过滤参数（如 `c_analysis::`）会使普通标量失效，须加引号；`${{ }}` 中的输入与外部数据经 `env` 传入脚本，不直接展开在 `run` 中。被 `workflow_call` 调用的工作流不设自己的 `concurrency`：其中的 `github.workflow` 取调用方名称，会与调用方落入同一组而互相取消或卡住。

## CI

CI 分两层：

- **PR 层**：`Development checks`（`checks.yml`）中的 `CI required` 是分支规则唯一要求的检查，阻塞合并的任务清单写在该任务的 `needs` 中，增删检查只改工作流。原生、OS、检测器、交付与生成 C 分析在 Windows 和 Linux 上完整运行；Python 只跑 Linux 3.12／3.14 与 Windows 3.14，UI 只跑 Linux 与 Windows。PR 只改根目录指南、`docs/`、代理说明或配置（含 `.codex/` hooks）或 PR 模板时，构建与测试任务整体跳过，`quality` 照常运行。除这些文档外只改 `ui/` 的 PR（如前端依赖更新）只运行 `ui`、`python`、`desktop` 与 `quality`，跳过 core、原生、OS、检测器、交付、生成 C 分析与进程任务：这些任务的输入都不在 `ui/` 下，`tests/python/test_ci_selection.py` 检查它们的源码不引用 `ui/`，并核对跳过的任务与 `CI required` 接受跳过的名单一致。分支无须先与主干同步即可合并，合并后主干推送会再运行一次。
- **扩展层**：`Extended checks`（`extended.yml`）不属于必需检查，不设定时任务，按改动路径选择：其余 Python 版本与 macOS 上的 Python／UI（改动 Python 或 UI 源码时）、[桌面场景](desktop.md)和[官方对照](official.md)。master 每次推送都运行它；PR 只在改动界面、桌面后端或场景时运行桌面场景，改动官方对照相关路径时运行官方对照。PR 加 `full-ci` 标签后运行扩展层全部检查；手动运行也是全部。汇总任务 `Extended checks result` 给出结果，未被选中的检查记为跳过。

开新 PR 或合并前先看 master 最近一次 `Extended checks` 的结果：失败与本 PR 无关时，确认已有 Issue 跟踪（不稳定的套件明确暂停并开 Issue 跟踪根因）后照常合并，由引入失败的 PR 修复或回滚；相关或无法判断的先查明。master 推送的 `Development checks` 共用一个并发组，组内只保留一个等待中的运行：重跑较旧提交的失败作业会取消最新提交待运行的检查，确认偶发失败时重跑 master 最新提交的运行。

公开仓库不注册自托管 runner，workflow 只用 GitHub 托管 runner。托管 runner 由并行开发共享，公开仓库的并发上限为 20 个作业。推送前确认会触发哪些运行，只保留需要的；中间提交和探测推送的检查在后续推送前取消，实验矩阵保持小规模（同时约四个作业），临时探测分支和一次性 workflow 用完即删。CI 因耗时超限时先依据实际阶段耗时消除重复构建、复用依赖缓存并并行独立样本，不能只延长总超时；完整检查范围与失败条件保持，优化效果以实际运行核对。按作业拆分或分片前，核对首批并发作业数：超出上限的部分排队，会抵消拆分收益；分片名单从同一列表筛选与排除，保证每个测试恰好运行一次。

## 本地复核与变异确认

- 本地复核 Clippy 时按 CI 与 `verify` 的每条命令及其 feature 组合分别运行；`--workspace --all-features` 会打开 `native-webdriver` 等 feature，使 `cfg(not(feature = ..))` 分支不受检查，不能代替默认 feature 的检查。
- 按 feature 汇总多个测试二进制的结果时加 `--no-fail-fast`，否则 Cargo 在第一个失败的二进制后停止，后续套件既未运行也不报失败。
- 变异确认改动一方运行时源码时，无需更新资产摘要；每次构建从实际字节计算身份。以测试结果行和被测断言判断变异，不把编译失败当作杀死。第三方 source manifest 仍固定内核／端口身份，不能为变异自动更新。同一工作区先后构建两个变体时，旧二进制仍绑定旧来源身份；`runtime_contracts::` 检查当前资源目录会报告 `identity_differs`，变体应在各自检出中构建和运行。原生套件缺少 `AUTOSAR_CC` 等工具变量时以 `executable_environment` 失败，这不算杀死。
- OS suites 从 `autosar_tooling.config.ROOT`（可编辑安装所在检出）取运行时源码，在副本中变异时须让 `PYTHONPATH` 指向副本的 `tools/python/src`，否则测的仍是原检出；用 `-k NativeSuiteTests.test_<suite>` 选套件并确认实际运行了测试。
- 用短名选择 ARXML 容器做变异时同时匹配 `DEFINITION-REF`：`RxValue` 等短名在 CanIf、Com 等模块重复，选错对象会使拒绝测试因输入未变而失效。
- 生成器的模板编辑和测试变异按文本锚定运行时或生成 C 时，锚点不能依赖空白与换行：标识符用 `Edit::Token` 按完整 token 替换，`Edit::Every` 只要命中一处就不报错，会静默漏掉换行后的其余位置。

## 离线 ECU 包

离线 ECU 包可由 Python 3.11 或更新版本运行自带 `tools/ecu-tool.py`，不需要 uv、Rust、Node 或源 checkout。构建和验证仍检查封存内容与目标工具链。支持版本的 Windows/Linux 实际构建由受控验收验证，未运行的平台或版本不宣称通过。
