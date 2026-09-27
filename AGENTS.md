# Repository Guidelines

## 项目结构与模块

`core/src/` 是 Rust 配置模型、ARXML 解析、代码生成和主机验证的核心；`core/tests/end_to_end.rs` 及 `core/tests/fixtures/` 存放集成测试。`src-tauri/` 提供桌面后端，`ui/src/` 是 React/TypeScript 界面，`src-tauri/icons/` 存放图标。`runtime/include/` 和 `runtime/src/` 是随生成工程交付的 C99 主机虚拟 ECU 运行时。`docs/official/` 只记录官方资料位置，下载的规范文件不入库；`scripts/` 放资料收集脚本。功能边界先读 `README.md` 和 `runtime/README.md`。

## 跨 Agent 交接

接到“状态如何”“继续开发”等请求时，先读[项目推进规则](docs/project/AGENT_OPERATING_RULES.md)，核对 Git、源码和 `_bmad-output/implementation-artifacts/sprint-status.yaml`。日常用法见[使用说明](docs/project/OWNER_GUIDE.md)。BMad sprint 文件是唯一任务进度来源；`docs/assurance/capabilities.json` 只记录能力声明及证据门，产品地图只记录方向与决策。旧 `docs/workflow/` 任务和证据作为历史输入保留；“状态如何”只读。

## 构建、测试与本地开发

在仓库根目录运行：

```powershell
npm ci --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

上述命令依次安装锁定的前端依赖、执行 TypeScript 检查并构建界面、运行核心测试、编译桌面程序。增量交付门用 `python scripts/verify.py --scope all --base <story起始提交>`；全量质量与规范证据缺口审计用 `python scripts/verify.py --scope baseline`，它允许如实报红。能力档案校验用 `python scripts/assurance.py`。首次安装检查工具见 `README.md`。界面开发用 `npm run dev --prefix ui`；桌面调试程序从 `src-tauri/target/debug/` 启动。Windows 依赖 Rust MSVC、C++ Build Tools、WebView2、GCC、vcpkg libxml2 和 libclang；具体环境变量见 `README.md`。

## 代码风格与命名

Rust 使用四空格及 `snake_case` 函数名；现有文件尚未整体通过 `rustfmt`，避免全仓格式化，但新增或修改的代码行须通过 `scripts/quality.py` 的 rustfmt 增量检查。TypeScript/TSX 沿用两空格、单引号和分号，新改行由锁定版本的 Prettier 检查，严格类型检查包含在 UI 构建中，ESLint 覆盖 UI 源码。C 代码保持 C99、四空格缩进，公开运行时接口沿用 `Can_Transmit` 等模块前缀；新改行由固定版本的 clang-format 检查。`runtime/include/` 的公开 C 接口按 Doxygen 格式说明契约，缺文档和参数说明由全量基线审计报告；内部静态函数不强制逐一注释。`.editorconfig` 约定基础空白格式。仓库尚未达到全量历史格式通过，也没有覆盖率门禁；部分 MISRA 扫描不能代替完整 MISRA 与模块 SWS 证据。

## 测试要求

对解析、生成或主机行为的改动，在 `core/tests/end_to_end.rs` 增加描述行为的 `snake_case` 测试，覆盖成功及关键拒绝路径。测试使用 `#[test]`，运行命令见上。集成测试需要本地 R24-11 XSD 和官方样例包，位置见 `README.md`；不要提交这些下载文件。Agent 的验收应在后台完成，优先使用无头命令和自动化测试，不占用用户正在使用的桌面，不弹出可见测试窗口，也不抢夺焦点或输入。确需验证 Tauri 桌面窗口与 IPC 时，使用独立桌面会话、虚拟机等不会干扰用户的隔离环境；单独启动进程或换用 release 程序不算隔离。当前环境无法后台完成的检查须如实标明未验证，不能在用户桌面上补做。

## 提交与 Pull Request

近期提交既有英文动词句（如 `Correct DCM stage evidence status`），也有 `feat:`、`docs:` 前缀；标题应简短并说明实际变更。PR 描述写明范围、关联议题、已运行的命令和结果；界面变化附截图。涉及 ARXML 或生成 C 的修改，说明输入样例、输出差异及主机运行证据。仅将已验证的主机能力表述为主机能力，勿暗示实机支持或标准符合性认证。
