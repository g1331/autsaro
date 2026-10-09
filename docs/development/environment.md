# 开发环境

Node/npm 的支持范围见 [ui/package.json](../../ui/package.json)，开发 Python 的范围见 [pyproject.toml](../../pyproject.toml)，离线工具要求见[运行时说明](../../runtime/README.md)。无需安装某个指定补丁版本。Rust 的编译器和组件由 [rust-toolchain.toml](../../rust-toolchain.toml) 管理。

## 安装项目依赖

```powershell
npm ci --prefix ui
uv sync --locked
```

`.python-version` 提供默认解释器选择。使用本地另一受支持的 Python 时，可通过 `uv sync --locked --python <解释器路径>` 指定它。只开发界面不必执行 Python 安装。锁文件固定包依赖，`uv.lock` 不要求指定的 uv 程序版本。

## 平台依赖

Windows 核心和桌面开发需要 Rust MSVC、Visual Studio C++ Build Tools、Windows SDK、WebView2、libclang 和 vcpkg 的 libxml2。设置 `VCPKG_ROOT`、`VCPKGRS_TRIPLET=x64-windows-static-md`、`LIBCLANG_PATH`；vcpkg 安装 `libxml2[iconv,zlib]`。

Ubuntu 核心开发安装 `build-essential libxml2-dev libclang-dev clang pkg-config`；桌面另安装 `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev patchelf`。macOS 使用 Xcode Command Line Tools、libxml2 和 LLVM。

原生 ECU 检查仍核对 `runtime/os/toolchain.json` 或 `toolchain-linux.json` 中的 GCC/binutils 身份。它们不是 UI 或默认核心测试的要求。官方资源仅供对应资源测试，位置与身份统一定义在 `core/resources/official.json`，下载不入库。

## 本地配置与排错

普通 npm/Cargo 命令直接读取环境变量，不加载项目 Python 配置。专用验证可复制根目录 `dev.local.example.toml` 为未跟踪的 `dev.local.toml`，环境变量优先。工具路径在原生或离线入口使用绝对路径，以避免工作目录和 PATH 改变执行对象。

按需要诊断某一部分：

```powershell
uv run --locked python -m autosar_tooling doctor --component ui
uv run --locked python -m autosar_tooling doctor --component desktop
uv run --locked python -m autosar_tooling doctor --component resources
uv run --locked python -m autosar_tooling doctor --role native --target windows-x64-controlled-v1
```

诊断不会安装依赖、修改主机或成为每次开发的必经步骤。Linux/WSL 的 checkout 和 Cargo target 宜放在 Linux 文件系统。GUI 自动验收必须使用独立会话或现有隔离设施。
