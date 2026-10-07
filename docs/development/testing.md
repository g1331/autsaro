# 测试与检查

## 本地反馈层

| 命令 | 内容 | 额外依赖 |
| --- | --- | --- |
| uv run dev test --suite fast | Python 逻辑、fixture 完整性及工具规则回归 | quality 组，无官方档案 |
| uv run dev test --suite ui | 确认取消、保存状态、输入拒绝等前端行为 | 已安装 UI 依赖 |
| uv run dev test --suite core | Rust 基础单元与产品内置配置/保存/交付测试 | Rust、libxml2、libclang |
| uv run dev test --suite process | Python 真实进程回收/超时/取消 | process profile |
| uv run dev test --suite integration | 官方资源及兼容输入对照；包含 builtin 的官方 oracle 测试 | integration profile |
| uv run dev test --suite native | 原生 OS/ECU/C 消费者、Python/Rust 进程测试 | integration + native profiles；Windows 部分生成器回归需要 Cppcheck 2.21.0 |
| uv run dev test --suite all | 全部上述自动测试 | 完整开发及受控原生环境 |

这些套件均不代表真实桌面 GUI/IPC 或安装包验收。platform 不适用的测试显示 skipped；不得把 skipped/未运行项写成通过。

每层可用 --plan 查看实际命令，Python/Cargo 套件用 --filter 筛选测试名字。完整验收不要使用筛选来替代整层运行。

## Cargo 目标

默认 cargo test 不启动需要档案和固定 ECU 工具链的目标，但包含真实进程测试，需要 AUTOSAR_PYTHON。dev test --suite core 会显式排除进程层。

- builtin：产品内置规则、定义、工作区与交付测试，不启用官方 oracle。
- end_to_end：通过 official-oracles feature 启用，依赖合法官方档案。
- native：通过 native-tests feature 启用，包含原有受控主机与 26 项 OS suite。
- 两个 feature 同时启用时保留完整旧测试覆盖，OS suite 不重复注册。

~~~sh
uv run dev test --suite core --filter builtin
uv run dev check --scope core
uv run dev check --scope all --full --base <基准提交>
~~~

旧 python -m autosar_tooling verify --scope core|all 仍请求完整资源/原生测试；它现在复用实时日志执行器，不再隐式 npm ci。普通开发推荐 dev check，首次依赖安装用 dev setup。

## 格式与静态检查

quality 默认检查所选范围的源码卫生及相对 HEAD 的改动行。--base 指定分支基准；不会根据工作区是否干净切换到 HEAD^。缺少无关语言的工具不会阻塞当前格式检查。C 语法在 C 文件变更、runtime 范围或 --all-format 时执行。

~~~sh
uv run dev check --scope ui --base <merge-base>
uv run dev fmt --scope ui
uv run dev assets check
~~~

fmt 会格式化已变动文件的全部内容，保持该文件的现有行尾；审阅历史行变化后再提交。运行时文件的原始字节身份由 .gitattributes 和清单约束，见资产维护说明。

## 原生桌面及安装包

现有隔离验收入口继续使用：

~~~sh
uv run python -m autosar_tooling desktop --platform linux --binary <本次构建的桌面程序>
~~~

Windows/Linux 使用独立 Desktop/Xvfb 等已有隔离机制，不在用户正在操作的桌面执行自动验收。Linux 需 tauri-driver 2.1.0、WebKit driver、Xvfb/x11-utils/xdotool；Windows 使用现有私有 station/Job/CDP 路径。macOS 要求独立非 console GUI 登录会话及 native-webdriver 测试构建，尚未声明原生验收通过。

安装包附加 --installed --source-checkout <本次 owned 构建副本原路径>，binary 必须是 checkout 外的实际解包应用；--builtin-only 验证默认配置链不依赖官方档案。仅可搬离本次 owned 副本，不移动用户工作树。系统安装、升级、签名和公证需另行验证。

## CI

PR 基础 CI 运行 UI、Python 和 Linux 基础核心检查，不读取私人档案或凭据。资源集成和原生验收在维护者配置的专用 self-hosted runner 手动触发；runner 配置说明见维护指南。它们调用同一套 dev 命令，未执行的专用层不会由基础 CI 代替。
