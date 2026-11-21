# Agent 开发指南

本文件是所有代理与贡献者共用的入口目录，规则不依赖特定代理工具或会话记忆。开工前先读本文件、[贡献指南](CONTRIBUTING.zh-CN.md)（[English](CONTRIBUTING.md)）、[README](README.zh-CN.md) 与相关模块说明，并核对目标版本、分支及工作区，保留用户修改。

## 仓库结构

- `core/`：Rust 配置、解析、生成及验证；`src-tauri/`：桌面后端；`ui/`：React 界面。
- `runtime/`：交付的 C99 运行时；`tools/python/src/`：开发工具与离线 ECU 工具；`tests/`：Python 与桌面测试。
- `scripts/`：官方资料收集；`docs/development/` 与 `docs/maintainers/`：开发与发行说明；`docs/standards/`：标准基线与模块义务。

## 常用入口命令

```sh
uv run --locked python -m autosar_tooling verify --scope changed --base origin/master --list
uv run --locked python -m autosar_tooling quality --scope all --merge-base origin/master
uv run --locked python -m autosar_tooling requirements
uv run --locked python -m autosar_tooling assets check
```

常规前后端与 Rust 检查命令见 [CONTRIBUTING.zh-CN.md](CONTRIBUTING.zh-CN.md#检查改动)，环境诊断见 `uv run --locked python -m autosar_tooling doctor --component <ui|desktop|resources>`。

## 按任务先读

| 开始这类工作前 | 先读 |
| --- | --- |
| 修改 C、头文件、生成 C 的模板或 emitter | [C 编码规范](docs/development/c-coding-standard.md)、[misra-c2012](.agents/skills/misra-c2012/SKILL.md)、[生成 C 的静态检查](docs/development/testing/generated-c.md)、[Emitter 规则](docs/architecture/mainline.md#emitter-规则) |
| 实现或核对 BSW／RTE／OS 规范，写条款处置 | [标准基线](docs/standards/README.md)（§5 处置标记与核算）及对应模块篇 |
| 修改独占区、跨模块回调或并发 | [运行时架构 · 独占区协议](docs/architecture/runtime.md#独占区协议) |
| 原生、OS、进程测试；迁移 runtime 文件或改运行时配置结构；排查时序与间歇失败 | [进程与原生行为](docs/development/testing/native.md)、[Windows 进程](docs/development/testing/windows-processes.md) |
| CI／workflow、日常反馈、模糊测试、源码验收 | [测试说明](docs/development/testing/README.md)、[最小有效反馈](docs/development/testing/feedback.md)、[源码验收](docs/development/testing/source-acceptance.md) |
| 进入调查、行为证伪、变异确认与独立审查 | [工程验证、调查与独立审查](docs/development/testing/verification-and-review.md) |
| 桌面场景与界面交互验收 | [桌面场景](docs/development/testing/desktop.md) |
| 官方档案、官方对照或登记了字节摘要的夹具 | [官方对照](docs/development/testing/official.md) |
| 交付资产与资产摘要 | [资产清单维护](docs/maintainers/assets.md) |
| 工作台界面与软件帮助 | [DESIGN.md](DESIGN.md) 及其[交互规格](docs/design/ux-baseline/EXPERIENCE/README.md) |
| Issue、分支、提交、PR、文档规范与版本发布 | [贡献指南](CONTRIBUTING.zh-CN.md)、[发行说明](docs/maintainers/releasing.md)、[规划与协作](docs/project/OWNER_GUIDE.md) |

## 核心原则

- **先查同类成熟项目的通行做法**：做任何流程、工具或设计决策，先查同类成熟项目的通行做法，再选其中高效且可靠的方案；偏离通行做法要写明具体理由。
- **Issue 粒度与父子拆分**：一个 Issue 应能由一个 PR 关闭。预计需要多个 PR 的工作，开工前拆成 GitHub 子 Issue，每个 PR 写 `Closes <子 Issue>`；`Refs` 只用于父 Issue。范围在创建时写成可数清单（如本批多少条规则、多少个场景）。父 Issue 正文顶部放一张状态表，原地更新；评论只记录决定、阻塞和范围变化，发现新工作就新建子 Issue，不悄悄扩大已有 Issue。分支用 `gh issue develop <子 Issue>` 关联到子 Issue，合并时直接关闭该子 Issue。
- **使用正常开发入口**：采用 `npm`、`Cargo`、`uv` 与正常测试、Issue、PR 入口；不新增代理专用编排、报告、状态系统或独立台账。
- **以原文与实证为依据**：AUTOSAR／MISRA 结论依据固定版次原文（含渲染图）；验证预期从标准独立定义；行为改动从正常入口实测成功、失败与关键状态变化；不通过放宽期限、弱化断言、重试或扩大排除让检查通过，只报告实际运行的平台与范围。
- **维护目录与规则**：约定变更或高代价教训主动写入仓库，不留在会话记忆。本文件只留跨任务目录与核心原则并控制在 8 KB 以内；流程与专题细节放 [CONTRIBUTING](CONTRIBUTING.zh-CN.md) 或专题文档（单篇不超过 20 KiB），新增或改名专题文档时同步“按任务先读”表，工具或功能失效时清理或改写旧条目。
