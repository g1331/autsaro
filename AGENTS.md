# Agent 开发指南

人和 Agent 共用 [CONTRIBUTING.md](CONTRIBUTING.md) 中的开发、测试和 PR 规则。先读该指南、README 和相关模块说明，再检查 Git 工作区。不要覆盖用户修改，不要为当前任务重建已有工程。

## 导航

- core/src：Rust 配置、解析、生成与主机验证；core/tests：builtin、官方对照及 native 测试目标。
- ui/src：React/TypeScript；src-tauri：桌面后端。
- runtime：交付的 C99 运行时；scripts/autosar_tooling：开发工具；scripts/ecu_tools：随工程交付的工具。
- docs/development：环境与测试；docs/maintainers：资产及发行维护；docs/official：本地参考档案入口。

## 开发与验证

使用 uv run dev --help 查找共同入口，按改动范围执行 dev check；需要官方或原生证据时选择明确测试层。--plan 可预览，--json 供自动处理。检查结果说明运行范围及未验项目，不以构建代替 GUI/发行验收。

自动验收优先无头运行。真实 GUI/IPC 使用已有隔离入口，不弹出窗口干扰用户、不抢夺焦点或输入；开发者主动要求交互调试时可以启动应用。无法隔离时报告未验证，不在用户桌面补测。

修改 C 源码、头文件或生成 C 的 Rust 模板之前，读取并使用 [misra-c2012](.agents/skills/misra-c2012/SKILL.md)。共同 C 编码和行为测试要求见贡献指南；技能调用本身不构成符合性证据。纯 Rust/TypeScript、文档及只读任务不触发该技能。

修改交付资产后使用 dev assets check；显式更新身份前审阅源码与清单差异。不要修改固定上游哈希来吸收本地变更，不因局部修改全仓格式化。

## 任务协作

普通 Issue、明确修复和贡献无需先创建 BMad story，也无需生成 Excel 或额外报告。用户委托 BMad/story/epic 时使用相应仓库技能，核对规划、Git、源码和实际结果，更新已有工件；不建立重复状态台账。状态查询只读。

BMad 保留详细规划和历史实施状态，使用说明见 [Owner Guide](docs/project/OWNER_GUIDE.md)。可选 automotive skills 按具体规格或审阅任务使用，模板结果不替代测试，不成为普通贡献的前置条件。

完成时说明改动、实际运行的检查及未验范围。未经用户授权不推送、发布或发送外部消息。
