# Classic CAN 配置工作台

本地桌面工作台：为一个 ECU 新建或导入 AUTOSAR Classic R24-11 ARXML 配置，编辑受支持的 CAN 帧与信号，使用官方 XSD 校验，生成独立 C99 工程并构建主机目标。两份不同配置可作为两个独立 ECU 进程接入主机虚拟 CAN 总线。**当前不是全模块 AUTOSAR 实现，也没有真实硬件验证或发布包。**

## 当前支持

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。保存时暂存新内容，并在替换前复核磁盘文件；检测到外部修改则拒绝覆盖，无法安全回滚时保留原备份。
- 标准 11 位 Classical CAN，DLC 1–8；每 ECU 最多 32 帧、64 信号。每帧可映射多个不重叠的 1–32 位无符号 LSB0 小端信号。导入时拒绝与 Com 位段或位序冲突的 I-PDU 映射、重复的 CanIf PDU 映射，以及与 CanIf 不一致的关联 CAN 网络帧 ID 或布局；未解析的配置变体会阻止生成。Tx 帧按虚拟时钟周期发送；Rx 帧按最后有效接收时间判定超时。
- 生成工程包含独立的 Com、PduR、CanIf、虚拟 Can、Os 周期调度、Rte 接口及 ECU 配置；`profile.txt` 和 `files.list` 描述生成结果。构建后得到 `ecu_host.exe`。生成目录与 ARXML 源目录分开。
- 主机闭环检查两个 ECU 的独立信号值与位向量、CAN ID 优先顺序、丢帧后的接收超时、BUS_OFF/STARTED 恢复以及错误 DLC 拒绝。虚拟总线不模拟电气层或位级仲裁。

不支持 29 位 CAN、签名/大端信号、真实芯片驱动、未解析的配置变体及任意 AUTOSAR 模块配置。界面上的“通过”只对应本次主机运行，不代表上板结果。具体 C99 协议与边界见 [`runtime/README.md`](runtime/README.md)。

## 本地开发环境（Windows）

需要 Rust stable **MSVC** 工具链、Visual Studio C++ Build Tools、WebView2、Node.js/npm、C99 GCC，以及 libxml2 和 libclang。`libxml` Rust 依赖使用 vcpkg 的 `x64-windows-static` libxml2，bindgen 需要可用的 `libclang.dll`。

根据本机安装情况设置以下环境变量：

- `VCPKG_ROOT` 指向 vcpkg 根目录，`VCPKGRS_TRIPLET` 设为 `x64-windows-static`。
- `LIBCLANG_PATH` 指向包含 `libclang.dll` 的目录。
- `AUTOSAR_CC` 可选，指向用于构建生成工程的 GCC 可执行文件；未设置时使用 `PATH` 中的 `gcc`。
- `CARGO_HOME`、`RUSTUP_HOME` 可按需控制 Rust 工具链位置；Cargo 构建产物默认位于 `core/target/` 和 `src-tauri/target/`，已由 `.gitignore` 忽略。`cargo`、`rustc`、`gcc`、`node`、`npm` 仍须能在当前会话调用。

本地官方材料须由使用者自行放置，不随源码分发：

- XSD：`docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`（包含 `AUTOSAR_00053.xsd` 与 `xml.xsd`）。
- 集成测试样例：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip`。测试会导入其中 15 份跨文件 ARXML。

```powershell
npm ci --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

调试桌面程序时，先在一个终端运行 `npm run dev --prefix ui`（端口 `127.0.0.1:1420`），另一个终端运行 `src-tauri/target/debug/autosar-config-desktop.exe`。当前验证的是源码目录中的本地调试程序；生成器从仓库 `runtime/` 复制目标源码，桌面程序从上述本地路径取得 XSD。没有可脱离源码目录使用的安装包。

## 使用顺序

1. 在起始页新建项目并选择保存 ARXML 的目录，或一次选中同一 ECU 的所有 `.arxml` 文件导入。导入按所选文件集合建模，不自动识别并拆分其他 ECU 的文件。
2. “配置”页选择帧/信号，在右侧检查器修改并应用；先保存，再到“诊断”页运行校验。未应用的检查器输入不会悄悄写入 ARXML。
3. “生成与构建”页选择**独立的空目录**生成工程，再构建主机目标；生成前须无阻断错误。输出目录不得覆盖其他文件。
4. 为另一个 ECU 重复前述步骤，配置互补的 Tx/Rx CAN ID 与 DLC。在“虚拟运行”页选择对端已构建工程目录，运行双 ECU 闭环并查看事件与日志。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。
