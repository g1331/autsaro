# 参与开发

**简体中文** · [English](CONTRIBUTING.md)

## 贡献政策

本项目由维护者主导开发，不接受未经邀请的外部 Pull Request，此类 PR 将直接关闭，不进行代码审查。维护者和受邀协作者按下文的 PR 流程提交改动。欢迎通过 Issue 报告缺陷或提出需求；安全漏洞请按[安全报告指南](SECURITY.zh-CN.md)提交。

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
cargo test --locked -p autosar-config-core
cargo clippy --locked -p autosar-config-core --all-targets --all-features -- -D warnings
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
uv run --locked python -m autosar_tooling quality --merge-base origin/master
```

执行一次 `uv run --locked pre-commit install --allow-missing-config`，之后每次推送前自动运行上面最后一条 `quality`，未通过时阻止推送；它不替代 CI 的同一检查。`--allow-missing-config` 让尚无该配置的旧分支（共用同一 `.git/hooks` 的工作树也在内）照常推送。

IPC 声明从 Rust 命令签名及实际序列化 DTO 生成。修改后运行 `npm run ipc:generate --prefix ui`，提交 `ui/src/ipc/` 下两份生成绑定；`npm run ipc:check --prefix ui` 重新生成并逐字比较，不修改已跟踪文件。命令使用锁定的 Cargo 生成器和项目 Prettier，先执行 `npm ci --prefix ui`。默认与 `native-webdriver` 签名分别检查；正常 TypeScript 构建还验证错误命令名、payload 和返回值假设会被拒绝。类型导出须保持 Serde 属性及原生 transport 不变。

默认 Cargo 测试不要求官方资源或受控原生工具。真实进程、官方对照和桌面验收见[测试说明](docs/development/testing/README.md)。格式检查针对修改行，不整体重排历史代码。运行时 C 已整体符合 `.clang-format`；执行一次 `git config blame.ignoreRevsFile .git-blame-ignore-revs`，`git blame` 即跳过那次全量重排。生成的 C 由 emitter 直接写成同一排版，必需检查核对生成样本。

解析、生成和运行行为的修改应有独立预期及关键拒绝路径。生成代码每个输出只渲染一次，计划不能保证的前提报告为诊断；Clippy 在这些模块拒绝 panic 和文本修补，见测试说明。修改 C 或生成 C 模板时遵守 [C 编码规范](docs/development/c-coding-standard.md)，并按仓库 MISRA 指导执行；公开 C 接口说明输入、输出和错误契约。

## 资源与提交

修改交付资源后运行 `uv run --locked python -m autosar_tooling assets check`。运行时摘要由构建计算；`assets update` 只更新独立 seal guard 清单。BSW ABI 与第三方身份单独审阅；资产测试核对固定检出换行，见[资源维护](docs/maintainers/assets.md)。

提交前检查整个 diff，包括新增文件与生成内容。提交信息采用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/)：`<type>(<scope>): <description>`，type 为 `feat`、`fix`、`docs`、`test`、`refactor`、`perf`、`build`、`ci` 或 `chore`，scope 写受影响的模块或区域（如 `dcm`），破坏性变更加 `!` 并附 `BREAKING CHANGE:` 脚注。按可独立验证的阶段提交，仓库规则与流程文档的修改单独提交。标题说明实际改动；PR 说明问题、改动、验证和未运行范围，并把 Issue 的每条验收标准及移交事项对应到证据位置（测试、样本或文档），未完成的写原因和承接的 Issue；完整证据放在 PR，Issue 只写结论与链接。合并前自审整个 diff；涉及 AUTOSAR 规范的结论按条目原文复核，不凭记忆或现有代码判断，高风险改动另做一次与 CI 并行的独立审查。必需检查失败时立即查看处理，不等其余作业结束。必需检查通过即可合并：自审后用 `gh pr merge --auto --merge` 开启自动合并，`CI required` 通过后自动合并并删除分支；Extended 检查不阻塞合并；master 最近一次已完成的 Extended 运行失败时，先判断与本 PR 是否相关：无关的确认已有 Issue 跟踪（不稳定套件开 Issue 暂停）后照常合并，由引入失败的 PR 修复或回滚；相关或无法判断的先查明再合并。界面改动附实际截图。不要提交官方下载、个人配置或临时测试产物。主机测试的结论限定为主机能力。

对使用者或集成者可见的改动在 `CHANGELOG.md` 的 `Unreleased` 下加一条：交付的 C 接口与 ABI、配置接受或拒绝、生成物、工具与界面行为；重构、测试、CI 和文档不记。每项一行，写清效果，末尾附 PR 号（如 `(#166)`），细节放 PR。条目按 `Breaking`、`Added`、`Changed`、`Fixed` 分组，破坏性或契约变化写明集成者要做的修改。发版时把 `Unreleased` 改为带日期的版本标题。CHANGELOG 只维护英文。

每个 Issue 使用独立分支，开工时用 `gh issue develop <N> --base master --name <分支>` 创建并关联到 Issue，让他人看到它正在推进。主目录可用时可直接在其中推进，独立工作树是可选方式；并行推进的 Issue 使用独立工作树隔离。

真实 GUI 验收使用隔离环境，不占用开发者正在使用的桌面。原生 GUI／IPC 场景按应用与构建输入路径纳入 `CI required`；Extended 不重复自动运行，只有明确未选择的套件可以跳过。任务与进度用 GitHub Issues 和 Milestones 跟踪；新建 Issue 或开始实现前先搜索已有 Issue（含已关闭）和进行中的 PR，已有覆盖的在原处补充，避免重复工作；顺带完成了其他 Issue 的内容时，在该 Issue 留言并注明 PR，全部完成的用 `Closes #N` 关闭；方向见 [docs/roadmap/](docs/roadmap/README.md)，设计约束见 [docs/architecture/](docs/architecture/README.md)。

每个开着的 Issue 至少有一个性质标签和一个模块标签；跨模块的 `tracking` 总览只需性质标签。性质标签为 `bug`、`known-gap`（能力、标准或验证差距）、`tech-debt`、`enhancement`、`question`、`story` 或 `tracking`（阶段总览）。模块标签可以有多个：

- `area:os`：OS 运行时、OS 配置生成与目标端口
- `area:bsw`：BSW 模块的运行时与配置
- `area:rte`：RTE／SchM 生成与组件契约
- `area:generator`：配置模型、ARXML 读写、定义目录与 C 生成主线
- `area:ui`：工作台界面、交互与文案
- `area:desktop`：Tauri 桌面后端、安装包与桌面场景测试
- `area:quality-ci`：CI、`quality`、C 静态检查与测试设施
- `area:delivery`：交付包、资产清单与离线 ECU 工具

等待实机或其他 Issue 的加 `blocked`，并在正文写明等待对象。进度由 Milestone 与关联的分支或 PR 表示，不另设状态标签；PR 的模块标签由 `Pull request labels` 工作流按改动路径自动添加（映射见 `.github/labeler.yml`）；并行推进的工作尽量不落在同一 `area`。

## 界面文案与语言

前端文案位于 `ui/src/i18n/` 的分域资源；后端文案以 `core/src/messages.json` 为唯一来源，由 Cargo 构建生成 Rust 静态模板，前端直接导入同一资源。新增产品消息使用稳定的语义键，并同时维护中英文完整句子和相同的插值参数；不要通过原句匹配或 DOM 替换翻译。

控制器和 IPC 保存消息键及参数，在显示时翻译，避免语言切换后旧提示残留。AUTOSAR 标识符、用户数据及外部工具证据保持原文。语言偏好不得参与工程指纹、生成内容或结果失效判断。修改文案后走正常 npm/Cargo 检查，并实际检查中英文的成功、失败及窄窗场景。

Windows MSI 分别构建 `zh-CN` 与 `en-US` 安装界面。英文 WiX 资源 `src-tauri/wix/locales/en-US.wxl` 使用代码页 936，容纳既有中文产品身份；不能恢复默认代码页 1252，否则英文 MSI 会因品牌与安装路径字符无法编码而链接失败。安装包语言与应用保存的语言偏好独立。

根目录面向读者的文档以英文为默认入口，中文版使用 `.zh-CN.md` 后缀。修改双语文档时同步两版的功能、命令、限制和链接；`CHANGELOG.md` 只维护英文；内部规格和子目录技术文档保持各自原有语言。

## 文档组织

一篇文档只讲一个主题。单篇超过约 20 KB（中文约 7000 字），或包含多个可独立阅读的主题时，拆成同名目录，目录下的 `README.md` 作为索引，每个子文档一行说明用途；原位置保留指向索引的链接。按版本追加的 CHANGELOG、单项决定 ADR 与只作追溯的归档不受此限；`quality` 对其余 Markdown 按 20 KiB（换行按 LF 计）检查。新增内容先放进已有的合适文档或目录，不另起平行文件。

## 许可与安全报告

原创贡献按 [Apache-2.0](LICENSE) 提供，第三方内容保留原许可；加入依赖或代码前确认有权分发。具体分发要求见[许可说明](docs/maintainers/licensing.md)。安全问题按 [SECURITY.md](SECURITY.zh-CN.md) 报告，普通问题可提交包含复现步骤和预期结果的 Issue。
