# 维护与发布准备

## 依赖更新

依赖升级使用单独 PR，说明变更动机及平台影响。Python 更新 pyproject.toml 后执行 uv lock；UI 更新 package.json 后使用推荐 npm 更新锁文件；Rust 同时核对 core 和 src-tauri 的 Cargo.lock。工具链推荐版本和受控 ECU 锁是不同契约，不能仅修改 README。

每次升级运行相关 dev check、基础行为测试及受影响的专用验收。不要将不相关的包更新混入功能修改。

## 专用 CI runner

基础 checks 工作流可在普通 GitHub hosted runner 执行。

专用 acceptance 工作流只由 workflow_dispatch 触发，在带 autosar-controlled 和所选操作系统标签的 self-hosted runner 执行。runner 使用支持 Node 24 Actions 的当前 GitHub Actions runner。维护者需准备对应宿主的 Rust/桌面开发依赖、合法资源、本地 dev.local.toml 或环境变量、固定工具链；Windows 原生生成器回归另需 Cppcheck 2.21.0。runner 不应处理不受信任的 fork PR 或在用户日常桌面执行 GUI 自动验收。

调用前检查所选 commit，确认 runner 原生平台与目标一致。原生 GUI/安装包验收另使用 desktop 工作流，并提供本次构建/解包应用的绝对路径。所有工作流在失败时保留报告和日志；runner 未配置时专用验收没有执行，不可标记为通过。

## 发行版本

版本目前为 0.1.0，需同步 core/Cargo.toml、src-tauri/Cargo.toml、ui/package.json、pyproject.toml 和 Tauri 配置，并更新相关锁文件及 CHANGELOG。版本更新必须审阅生成工程的兼容性和输入格式变化。

在已准备的平台执行 npm run tauri --prefix ui -- build，产物位于 src-tauri/target/release/bundle/。Windows MSI、Linux deb/AppImage 使用各自原生构建与隔离复验；macOS app/dmg、签名、公证、安装/升级和公开发布仍需分别验证。

## 许可与分发

项目原创代码与文档采用 [Apache-2.0](../../LICENSE)，分发时附带 [NOTICE](../../NOTICE)。生成源码工程和桌面包携带这些文件，Python 分发包在许可目录包含它们。发布前按 [许可说明](licensing.md) 核对实际打包的第三方素材、技能、依赖、内核及其原始声明。

现有研究输入见 docs/research/autosar-platform/open-source-rights.md。它记录待核事项，不是本项目取得授权的证明。官方规范档案不进入源码包或安装包。
