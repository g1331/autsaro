# 维护与发布准备

## 依赖更新

依赖升级使用单独 PR，说明变更动机及平台影响。Python 更新 pyproject.toml 后执行 uv lock；UI 更新 package.json 后使用 npm 更新锁文件；Rust 更新根目录 workspace 唯一的 Cargo.lock。开发工具范围和受控 ECU 锁是不同契约，不能仅修改 README。

每次升级运行相关构建、lint 和测试、基础行为测试及受影响的专用验收。不要将不相关的包更新混入功能修改。

## CI 工作流

所有工作流都在 GitHub 托管 runner 上运行，仓库不使用自托管 runner。基础 checks 工作流包含离线接收兼容性：`native-delivery` 在 Linux 与 Windows 上用 Python 3.11 运行全部四个移动后的生成包，3.12–3.14 运行 `standard-ecu`；Extended 在两平台运行完整版本与样本矩阵（见[测试说明](../development/testing/native.md)）。网络隔离下的安装与离线验收目前没有自动入口。

官方对照工作流 acceptance 运行时从 autosar.org 下载官方档案并按 `core/resources/official.json` 的哈希校验，不属于必需检查；触发方式与失败的查看方法见[测试说明](../development/testing/official.md)。维护者需关注定时运行的失败通知。站点更换 TLS 证书链时，下载入口固定的中间证书指纹会失效：审阅新证书链后更新指纹，不能关闭证书校验。官方档案不能提交进仓库、作为 artifact 上传或输出到日志。

原生 GUI 场景由 desktop 工作流自行构建验证版并运行，上传场景结果目录；它不覆盖安装包的安装与离线验收。bundles 工作流见下节。

## 发行版本

版本目前为 0.1.0，需同步 core/Cargo.toml、src-tauri/Cargo.toml、ui/package.json、pyproject.toml 和 Tauri 配置，并更新相关锁文件及 CHANGELOG。版本更新必须审阅生成工程的兼容性和输入格式变化。

bundles 工作流由 workflow_dispatch 触发并接受明确的 revision，在 `ubuntu-24.04` 与 `windows-latest` 上用 Tauri CLI 构建，安装包上传为 `bundles-<Linux|Windows>-<提交 SHA>` artifact，不执行公开发布。本地在已准备的平台执行 `npm run tauri --prefix ui -- build`，产物位于 target/release/bundle/。当前产物为 Windows MSI（en-US、zh-CN）和 Linux deb、AppImage；它们的安装、升级、签名和公开发布仍需分别验证；macOS 原生构建不在支持范围内。

## 许可与分发

项目原创代码与文档采用 [Apache-2.0](../../LICENSE)，分发时附带 [NOTICE](../../NOTICE)。生成源码工程和桌面包携带这些文件，Python 分发包在许可目录包含它们。发布前按 [许可说明](licensing.md) 核对实际打包的第三方素材、技能、依赖、内核及其原始声明。

权利研究及待核事项见 docs/research/autosar-platform/open-source-rights.md。分发前需核对具体内容的授权与条款，官方规范档案不随源码包或安装包分发。
