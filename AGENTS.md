# Repository Guidelines

## 项目结构与模块

`core/src/` 是 Rust 配置模型、ARXML 解析、代码生成和主机验证的核心；`core/tests/end_to_end.rs` 及 `core/tests/fixtures/` 存放集成测试。`src-tauri/` 提供桌面后端，`ui/src/` 是 React/TypeScript 界面，`src-tauri/icons/` 存放图标。`runtime/include/` 和 `runtime/src/` 是随生成工程交付的 C99 主机虚拟 ECU 运行时。`docs/official/` 只记录官方资料位置，下载的规范文件不入库；`scripts/` 放资料收集脚本。功能边界先读 `README.md` 和 `runtime/README.md`。

## 跨 Agent 交接

接到“状态如何”“继续开发”等请求时，先读 [`docs/workflow/AGENT_WORKFLOW.md`](docs/workflow/AGENT_WORKFLOW.md) 并核对 Git 与源码。日常使用方式见 [`docs/workflow/OWNER_GUIDE.md`](docs/workflow/OWNER_GUIDE.md)。执行状态和工作流反馈由仓库文件保存，不以当前 Agent 的聊天记录为准；“状态如何”只读，不修改仓库。产品地图记录决策，任务和证据记录实际进展，两者不能混作完成证明。

## 构建、测试与本地开发

在仓库根目录运行：

```powershell
npm ci --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

上述命令依次安装锁定的前端依赖、执行 TypeScript 检查并构建界面、运行核心测试、编译桌面程序。界面开发用 `npm run dev --prefix ui`；桌面调试程序从 `src-tauri/target/debug/` 启动。Windows 依赖 Rust MSVC、C++ Build Tools、WebView2、GCC、vcpkg libxml2 和 libclang；具体环境变量见 `README.md`。

## 代码风格与命名

Rust 使用四空格及 `snake_case` 函数名；现有文件尚未整体通过 `rustfmt`，修改时遵循邻近格式，避免全仓格式化。TypeScript/TSX 沿用现有两空格、单引号和分号，类型检查包含在 UI 构建中。C 代码保持 C99、四空格缩进，公开运行时接口沿用 `Can_Transmit` 等模块前缀。仓库没有统一的 ESLint、Prettier 或覆盖率配置。

## 测试要求

对解析、生成或主机行为的改动，在 `core/tests/end_to_end.rs` 增加描述行为的 `snake_case` 测试，覆盖成功及关键拒绝路径。测试使用 `#[test]`，运行命令见上。集成测试需要本地 R24-11 XSD 和官方样例包，位置见 `README.md`；不要提交这些下载文件。Agent 的验收应在后台完成，优先使用无头命令和自动化测试，不占用用户正在使用的桌面，不弹出可见测试窗口，也不抢夺焦点或输入。确需验证 Tauri 桌面窗口与 IPC 时，使用独立桌面会话、虚拟机等不会干扰用户的隔离环境；单独启动进程或换用 release 程序不算隔离。当前环境无法后台完成的检查须如实标明未验证，不能在用户桌面上补做。

## 提交与 Pull Request

近期提交既有英文动词句（如 `Correct DCM stage evidence status`），也有 `feat:`、`docs:` 前缀；标题应简短并说明实际变更。PR 描述写明范围、关联议题、已运行的命令和结果；界面变化附截图。涉及 ARXML 或生成 C 的修改，说明输入样例、输出差异及主机运行证据。仅将已验证的主机能力表述为主机能力，勿暗示实机支持或标准符合性认证。
