# 参与开发

人和 Agent 使用同一套开发命令、测试和审查规则。普通修复可以直接从 Issue 或明确的问题开始，不需要 Codex、BMad story 或工程工作簿。

## 首次准备

安装 Git、[uv](https://docs.astral.sh/uv/getting-started/installation/) 和 [.node-version](.node-version) 声明的 Node。npm 支持 11.x，推荐版本见 [ui/package.json](ui/package.json) 的 packageManager。Python 由 uv 按 .python-version 准备。

在仓库根目录执行：

~~~sh
uv sync --locked
uv run dev setup --profile ui
uv run dev doctor --profile ui
uv run dev start ui
~~~

浏览器开发可检查界面和前端逻辑，完整文件操作及 IPC 需要桌面应用。桌面开发另准备 [平台依赖](docs/development/environment.md)，再执行：

~~~sh
uv run dev doctor --profile desktop
uv run dev start desktop
~~~

开发者主动启动应用可以使用自己的桌面；自动化 GUI/IPC 验收应使用隔离桌面，避免干扰正在工作的用户。

## 修改与检查

从最新基线创建分支，保持改动围绕一个问题。以下命令不重装依赖：

~~~sh
uv run dev check --scope ui
uv run dev check --scope tooling
uv run dev check --scope core
~~~

UI 检查包括 ESLint、行为测试和 TypeScript/Vite 构建。tooling 检查包括 Ruff 和不需要官方档案的 Python 测试。core 检查需要 Rust/libxml2/libclang，运行基础 Rust 与内置配置测试、Clippy 和资产检查，不需要官方档案或固定 ECU 编译器。

格式检查默认与 HEAD 比较待提交改动；检查整个分支时显式传入 --base，通常为与主分支的 merge-base。setup/start/check/test 可用 --plan 查看执行计划，--json 提供机器可读结果，执行日志同时保存在输出指出的目录。doctor 也支持 --json。

~~~sh
uv run dev check --scope ui --base <基准提交>
uv run dev test --suite core --filter builtin
uv run dev fmt --scope ui
uv run dev assets check
~~~

fmt 格式化已变动文件的完整内容，执行后审阅 diff；不要为了局部修改整体重排历史文件。运行时或交付工具的源码变化需要显式更新派生清单，见 [资产维护](docs/maintainers/assets.md)。

测试范围及完整验收见 [测试指南](docs/development/testing.md)。缺少某层依赖时可以贡献其他层，PR 中明确未运行的检查；不把跳过当作通过。

## 编码与测试

- Rust 使用四空格和 snake_case；TypeScript 使用两空格、单引号和分号；以仓库格式配置为准。
- C 保持 C99 和公开模块接口，公开运行时头文件说明契约。修改 C 或生成 C 模板时遵循适用的 MISRA C:2012 指导，验证真实输出；不将静态工具结果称为完整 MISRA 或 AUTOSAR 认证。
- 为改变的行为补成功和关键拒绝路径的测试。纯内置配置测试放在 builtin 目标，官方对照测试使用 official-oracles，编译/运行消费者放在 native 目标。
- 不提交官方 PDF/XSD/MOD/样例档案、私人路径、本地配置和构建结果。保持用户输入、应用代码及已有产物的所有权边界。

## 提交与 PR

提交标题简短说明改动，可用 feat:、fix:、docs: 等前缀。PR 描述问题、改动范围、验证命令和结果；界面改动附截图；生成代码变化说明输入和输出影响。普通修复不要求创建规格或复核报告。

与已有 BMad story 关联时，在 PR 中链接相应工件，并按实际结果更新该工件；独立贡献以 Issue/PR 和 Git 为记录，不建立另一份 sprint 台账。Agent 的额外约束见 [AGENTS.md](AGENTS.md)。

## 许可与安全报告

项目原创代码与文档采用 [Apache-2.0](LICENSE)。有意提交并纳入项目的原创贡献，除明确另有约定外，按该许可证第 5 条以同一许可提供。第三方组件保留原许可证，不要加入自己无权贡献的内容；适用范围与分发要求见 [许可说明](docs/maintainers/licensing.md)。

安全问题的报告方式见 [SECURITY.md](SECURITY.md)。可复现的普通问题使用仓库 Issue；需求讨论说明用户操作、预期结果和实际限制。
