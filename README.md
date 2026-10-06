<div align="center">
  <img src="ui/public/logo-app.png" width="112" alt="Autsaro 汽车图标">
  <h1>Autsaro</h1>
  <p>面向 AUTOSAR Classic 的配置与代码生成工作台</p>
</div>

[功能一览](#功能一览) · [能力范围](#当前支持) · [使用流程](#使用顺序) · [本地开发](#本地开发环境) · [验证](#代码质量检查) · [文档](#文档导航)

Autsaro 支持从内置模板创建 ECU 工程，或导入多份 ARXML；在同一工作区编辑配置、检查引用、预览保存差异，并生成独立 C99 源码工程。当前采用 AUTOSAR Classic R24-11。

日常配置与源码准备不依赖官方 XSD/MOD、编译器或联网；本机预检、构建与主机行为验证需要另外准备声明的工具链。

## 功能一览

<h3><img src="docs/images/icon-configuration.svg" width="22" alt=""> 配置编辑与引用检查</h3>

工程树、帧与信号表、属性检查器共用同一工作区。选择对象后，可检查参数与引用关系，编辑 CAN ID、信号布局和发送周期。

<a href="docs/images/workbench-configuration.png"><img src="docs/images/workbench-configuration.png" width="980" alt="CAN 配置工作区：左侧工程树、中间帧与信号表、右侧属性和引用检查器"></a>

<table>
  <tr>
    <td width="50%" valign="top">
      <h3><img src="docs/images/icon-files.svg" width="22" alt=""> 多文件工程</h3>
      <p>在同一工程中组织应用、BSW、ECUC 与系统描述，查看各文件的来源角色。</p>
      <a href="docs/images/workbench-files.png"><img src="docs/images/workbench-files.png" width="480" alt="标准输入视图：七份 ARXML 文件及其来源角色"></a>
    </td>
    <td width="50%" valign="top">
      <h3><img src="docs/images/icon-validation.svg" width="22" alt=""> 校验与问题定位</h3>
      <p>结构、定义与生成约束分别报告；从问题详情定位到对应配置对象。</p>
      <a href="docs/images/workbench-validation.png"><img src="docs/images/workbench-validation.png" width="480" alt="问题工具窗口：分域校验结果、基数错误与定位到真实来源操作"></a>
    </td>
  </tr>
</table>

## 当前支持

| 范围 | 支持内容 |
| --- | --- |
| 工程与 ARXML | 多文件导入、跨文件引用、内置模板、项目成员管理与安全保存 |
| 配置编辑 | 对象树、字段与引用检查、批量修改、实例结构编辑 |
| CAN 信号 | 11 位 Classical CAN，DLC 1 至 8；每 ECU 最多 32 帧、64 信号；1 至 32 位无符号 LSB0 小端信号 |
| 主机诊断 | 有限 DoCAN、会话与 DID 服务；部分目标可配置写入、例程、DTC 与主机文件持久化 |
| 源码交付 | BSW、OS、RTE／应用接口、配置、目标依赖与离线构建工具 |

诊断服务与容量随生成目标不同，详见[运行时支持范围](runtime/README.md)。未知配置按覆盖范围保留为只读或限制相应操作；安全保存与目标生成分别检查。

当前运行目标为受控 Windows／Linux 主机环境，不提供真实 MCU、硬实时、功能安全或官方符合性认证。macOS 原生构建与 IPC 尚未验证，不提供本机虚拟 ECU。

## 使用顺序

1. 从内置模板新建工程，打开已有工程，或一次导入同一 ECU 的全部 ARXML。
2. 在工程树中选择对象，检查参数与引用；批量修改先查看整批影响，再应用。
3. 保存前查看文件差异并确认。预览不写磁盘；来源文件被外部修改后须重新导入。
4. 选择工程之外的目录交付源码。需要构建或主机验证时，再配置对应目标的工具链；构建目录与源码目录分开。

未修改的来源文件保留原字节。重复生成会核对文件完整性，旧工程保留为备份，不覆盖用户修改；应用与来源变化会使已有预检、生成确认和下游结果失效。

## 本地开发环境

| 用途 | 工具 |
| --- | --- |
| 配置核心与桌面后端 | Rust 1.98.1、rustfmt、Clippy |
| 前端 | React／TypeScript、Node 24.19.0、npm 11.17.0 |
| 开发与验证 | CPython 3.12.9、uv、Git |
| 主机 ECU 构建 | 目标锁声明的 GCC、objdump、Git 与 CPython |

版本由 `rust-toolchain.toml`、`.node-version`、`.python-version` 及 Cargo、npm、uv 锁文件固定。源码开发与核心测试需要合法的本地 XSD、MOD 和样例档案，位置见[规范资料入口](docs/official/README.md)；这些档案不随仓库或安装包分发。

<details>
<summary>平台依赖</summary>

- **Windows**：Rust MSVC、Visual Studio C++ Build Tools、WebView2；vcpkg 安装 `libxml2[iconv,zlib]:x64-windows-static-md`。设置 `VCPKG_ROOT`、`VCPKGRS_TRIPLET=x64-windows-static-md` 和包含 `libclang.dll` 的 `LIBCLANG_PATH`。
- **Ubuntu 24.04**：安装 `build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev libxml2-dev libclang-dev clang pkg-config patchelf`。WSL 构建副本、Cargo target 与 uv 缓存放在 ext4 文件系统。
- **macOS**：Xcode Command Line Tools、pkg-config/libxml2 与上述版本管理工具；原生构建与 IPC 未验证。

</details>

在仓库根目录安装依赖并检查开发环境：

```sh
uv sync --locked --group quality
npm ci --prefix ui
uv run --locked python -m autosar_tooling doctor --role workbench
npm run tauri --prefix ui -- info
```

界面开发使用 `npm run dev --prefix ui`；桌面调试使用 `npm run tauri --prefix ui -- dev`。Cargo 产物位于 `core/target/` 和 `src-tauri/target/`。

直接运行 Cargo 前，将 `AUTOSAR_PYTHON` 设置为项目虚拟环境中 CPython 的绝对路径：Windows 使用 `.venv/Scripts/python.exe`，POSIX 使用 `.venv/bin/python`。核心集成测试另设置 `RUST_TEST_THREADS=2`，避免多个主机 ECU 实例争用 CPU。

## 代码质量检查

| 检查 | 命令 |
| --- | --- |
| 前端静态检查 | `npm run lint --prefix ui` |
| TypeScript 与界面构建 | `npm run build --prefix ui` |
| 核心测试 | `cargo test --manifest-path core/Cargo.toml` |
| 桌面后端编译 | `cargo build --manifest-path src-tauri/Cargo.toml` |

完整开发检查使用统一入口，将 `<起始提交>` 替换为检查差异的基准提交：

```sh
uv run --locked python -m autosar_tooling verify --scope all --base <起始提交>
```

该入口组合格式与静态检查、测试、UI 构建和桌面后端编译。GUI／IPC 与安装包验收单独执行，不由构建或测试通过推定。

## 构建与分发

在已准备平台依赖的原生宿主上运行：

```sh
npm run tauri --prefix ui -- build
```

产物位于 `src-tauri/target/release/bundle/`。Windows 配置 MSI，Linux 配置 deb／AppImage；macOS 配置 app／dmg，但尚未完成原生验证。签名、公证与公开发行尚未验证。

安装后的应用不需要 Rust、Node、npm 或 uv。Windows 需要 WebView2，Linux 需要 WebKitGTK 4.1、Ayatana AppIndicator 和 libxml2。安装包不包含官方规范档案或编译器；ECU 构建与主机验证另需对应工具链。

## 项目结构

| 路径 | 职责 |
| --- | --- |
| `core/` | Rust 配置模型、ARXML 解析、代码生成与主机验证 |
| `src-tauri/` | Tauri 桌面后端 |
| `ui/` | React／TypeScript 配置界面 |
| `runtime/` | 随生成工程交付的 C99 主机运行时 |
| `scripts/` | Python 开发与验证工具 |
| `docs/` | 规范资料入口与技术文档 |

## 文档导航

| 文档 | 用途 |
| --- | --- |
| [主机运行时](runtime/README.md) | 运行接口、通信与诊断范围 |
| [受控 OS 目标](runtime/os/README.md) | 内核、补丁、工具链与平台范围 |
| [独立交接工程](runtime/reference-README.md) | 离线参考包与独立复验 |
| [规范资料入口](docs/official/README.md) | 本地规范档案的位置与分发限制 |
| [界面设计](DESIGN.md) | 视觉与组件规范 |
