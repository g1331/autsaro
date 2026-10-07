# 开发环境

## 按任务准备

| 任务 / doctor profile | 必需依赖 | 不需要 |
| --- | --- | --- |
| ui | Node 24、npm 11、锁定 UI 依赖、Git | Rust、官方档案、ECU 工具链 |
| tooling | 项目 Python 3.12、quality 组、Git | 桌面库、官方档案 |
| core | tooling、固定 Rust、libxml2、libclang | 官方档案、固定 ECU 编译器 |
| desktop | core、ui、平台 Tauri 依赖 | 官方档案、固定 ECU 编译器 |
| process | tooling、可运行子进程的宿主；Linux 的真实 /proc 线程子进程接口 | 桌面库、官方档案、ECU 编译器 |
| integration | core、合法 XSD/MOD/样例包 | GUI 会话 |
| native | 固定 CPython、显式 GCC/binutils/Git 路径及目标身份 | 独立 OS 消费者不需要官方档案 |
| all | 上述全部开发和受控工具 | GUI/安装包验收仍另行准备 |

native profile 检查独立 ECU 工具身份；完整 native 测试还消费兼容输入，所以要同时通过 integration profile。默认 doctor 只检查所选 profile，不推断其他能力。

~~~sh
uv sync --locked
uv run dev doctor --profile ui
uv run dev doctor --profile core
uv run dev doctor --profile integration --json
~~~

版本以 rust-toolchain.toml、.node-version、.python-version、ui/package.json、pyproject.toml 和各锁文件为准。正常开发允许 Python 3.12 和 npm 11 的其他补丁版本，并提示推荐版本；Node 需要满足 package.json 的最低版本。--strict 要求推荐补丁版本。格式工具保持固定版本；原生 ECU 工具仍按 runtime/os 的目标锁核对版本、目标与原始字节。

uv 默认安装 quality 组，后续 uv run 不会移除它。可选工程工作簿依赖用 uv sync --locked --group automotive，不是普通开发的前置条件。setup 只安装项目依赖，不更改操作系统、下载官方档案或更新锁文件。

## 本地配置

按需要把 dev.local.example.toml 复制为 dev.local.toml，填写相关项；该文件不入库。已有环境变量优先于本地配置，档案相对路径从仓库根目录解析。开发入口自动准备 AUTOSAR_PYTHON、RUST_TEST_THREADS 和 UTF-8 输出，只影响当前进程及子进程。

直接使用 Cargo/npm 时仍可自行设置环境变量；经 dev 启动时无需手工设置 Python 路径。故障排查首先看 doctor 的阻塞项，再看阶段日志；日志目录包含 summary.json、stdout 和 stderr。

## 平台依赖

### Ubuntu 24.04

核心依赖：

~~~sh
sudo apt-get update
sudo apt-get install build-essential libxml2-dev libclang-dev clang pkg-config
~~~

桌面另需：

~~~sh
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev patchelf
~~~

WSL 的 checkout、Cargo target 和 uv 缓存放在 Linux 文件系统。固定 Ubuntu GCC/binutils 身份见 runtime/os/toolchain-linux.json，普通发行版的 GCC 不一定能通过受控原生验收。

进程所有权验收需要可观察真实 /proc/<pid>/task/<tid>/children 的 Linux 宿主。隐藏该接口的容器或合成 /proc 环境不能证明逃逸子进程回收；process/native doctor 会将它列为阻塞项。UI、Python 逻辑及基础核心检查仍可独立执行。

### Windows

使用 Rust MSVC、Visual Studio C++ Build Tools 和 WebView2。vcpkg 安装 libxml2[iconv,zlib]:x64-windows-static-md，配置 VCPKG_ROOT、VCPKGRS_TRIPLET=x64-windows-static-md，以及包含 libclang.dll 的 LIBCLANG_PATH。在新终端检查结果；不要提交个人安装路径。

固定 ECU 工具链见 runtime/os/toolchain.json，使用 AUTOSAR_CC、AUTOSAR_OBJDUMP、AUTOSAR_GIT 指定绝对工具路径。

### macOS

准备 Xcode Command Line Tools、pkg-config、libxml2 和 LLVM/libclang。UI 和 Python 检查可以运行；核心/桌面源码开发路径提供入口，但原生桌面、IPC、安装包尚未完成验证。没有 macOS ECU 运行目标。跨平台 CI 通过不代表完整原生发行通过。

## 官方参考输入

官方档案不随仓库分发，由使用者合法取得；只用于对应的开发对照和兼容测试。

| 变量 | 默认文件 |
| --- | --- |
| AUTOSAR_XSD_ARCHIVE | docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip |
| AUTOSAR_MOD_ARCHIVE | docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip |
| AUTOSAR_SAMPLE_ARCHIVE | docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip |

XSD 和 MOD 固定摘要来自核心及 fixture 的权威身份；样例包检查有效 ZIP 和测试所需路径，不声明已经核对官方摘要。doctor、核心测试和输入审计使用同样的路径覆盖规则。资料收集脚本支持 PATH 中的 curl，但不替代合法取得资料的前置条件。
