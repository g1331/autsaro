# 维护与发布准备

## 依赖更新

依赖升级使用单独 PR，说明变更动机及平台影响。Python 更新 pyproject.toml 后执行 uv lock；UI 更新 package.json 后使用 npm 更新锁文件；Rust 同时核对 core 和 src-tauri 的 Cargo.lock。开发工具范围和受控 ECU 锁是不同契约，不能仅修改 README。

每次升级运行相关构建、lint 和测试、基础行为测试及受影响的专用验收。不要将不相关的包更新混入功能修改。

## 专用 CI runner

基础 checks 工作流可在普通 GitHub hosted runner 执行。

专用 acceptance 工作流只由 workflow_dispatch 触发，在带 autosar-controlled 和所选操作系统标签的 self-hosted runner 执行。runner 使用支持 Node 24 Actions 的当前 GitHub Actions runner。维护者需准备对应宿主的 Rust/桌面开发依赖、合法资源、受控工具链，以及指向这些资源和可执行文件的环境变量（Cargo/Tauri 直接读取）；Windows 原生生成器回归另需 Cppcheck 2.21.0。runner 不应处理不受信任的 fork PR 或在用户日常桌面执行 GUI 自动验收。

`AUTOSAR_CC`、`AUTOSAR_OBJDUMP` 和 `AUTOSAR_GIT` 必须为绝对路径；资源路径通过 `AUTOSAR_XSD_ARCHIVE`、`AUTOSAR_MOD_ARCHIVE`、`AUTOSAR_SAMPLE_ARCHIVE` 指定。工作流设置开发解释器；offline 工作流另外选择接收者解释器，直接运行生成包，不向该解释器安装开发工具。

调用前检查所选 commit，确认 runner 原生平台与目标一致。原生 GUI/安装包验收另使用 desktop 工作流，并提供本次构建/解包应用的绝对路径。acceptance 上传测试日志，desktop 上传指定结果目录，bundles 上传按提交 SHA 命名的安装包；runner 未配置时专用验收没有执行，不可标记为通过。

## 发行版本

版本目前为 0.1.0，需同步 core/Cargo.toml、src-tauri/Cargo.toml、ui/package.json、pyproject.toml 和 Tauri 配置，并更新相关锁文件及 CHANGELOG。版本更新必须审阅生成工程的兼容性和输入格式变化。

bundles 工作流接受明确的 revision，在原生 runner 用 Tauri CLI 构建并按提交 SHA 保存安装包，不执行公开发布。在已准备的平台执行 `npm run tauri --prefix ui -- build`，产物位于 src-tauri/target/release/bundle/。Windows MSI、Linux deb/AppImage 使用各自原生构建与隔离复验；macOS app/dmg、签名、公证、安装/升级和公开发布仍需分别验证。

## 许可与分发

项目原创代码与文档采用 [Apache-2.0](../../LICENSE)，分发时附带 [NOTICE](../../NOTICE)。生成源码工程和桌面包携带这些文件，Python 分发包在许可目录包含它们。发布前按 [许可说明](licensing.md) 核对实际打包的第三方素材、技能、依赖、内核及其原始声明。

权利研究及待核事项见 docs/research/autosar-platform/open-source-rights.md。分发前需核对具体内容的授权与条款，官方规范档案不随源码包或安装包分发。
