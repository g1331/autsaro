# Classic CAN 配置工作台

本地 AUTOSAR Classic R24-11 配置工作台：默认以产品原创的内置结构规则与模块定义打开、检查、编辑和保存真实 ARXML，并准备独立 C99 源码。日常配置与源码准备不需要官方 XSD/MOD、编译器或联网；本机预检、构建与主机行为验证另需声明的工具链。保留既有 CAN、DoCAN、诊断与标准 ECU 有限生成目标。**不是全模块 AUTOSAR 运行时、完整官方 XSD 符合性认证或真实硬件验证。**

## 当前支持

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。校验、保存和生成都会复核所选的全部来源文件；任一文件被外部修改时拒绝继续使用旧跨文件配置，并提示重新导入。保存时暂存新内容，在替换前再次复核待写文件；无法安全回滚时保留原备份。
- 编辑配置后，“查看并保存 ARXML”会先列出每份来源文件将修改或保持不变，并显示修改文件的前后差异及完整原文。预览不写磁盘；用户确认后才保存。若预览后配置或来源文件变化，旧预览不能用于保存。
- 同一工程使用文件／对象树、文档标签、属性与引用检查器及问题／生成／构建／运行／日志工具窗口。参数保持 `kind/lexeme`，未落盘默认值与 explicit 值分开；未知条件、变体、表达式和 instance-reference 不猜值、不伪装成可写。批次与实例结构变化先预览整批，再以当前输入／定义身份原子应用。
- 工程树优先显示对象名称，以图标区分包、模块、容器和数据类型；悬停可查看完整名称、类型与路径。软件“帮助”菜单内置操作步骤、工具依赖和支持范围，不要求用户取得源码仓库；生成工程附带的构建说明仍属于交付内容。
- `workbench-project.json` v1 只记录安全相对成员、应用输入及工程明确接纳的扩展身份；原 ARXML 是配置权威。`can-empty-v1`、`can-signals-v1`、`standard-ecu-v1` 是产品内置的工程模板，创建与另存为先预览，只写新空目录。第三方定义通过 `catalog.json` 显式接纳；不可变本机缓存不等于工程已经选择该定义。
- 分别报告 `source-safety`、`schema`、`definition`、`target-generation`。原生 `schema` 只覆盖产品声明的结构、顺序、基数及类型，不等于完整官方 XSD；`unsupported`、`not_run` 不冒充 `passed`。目标不支持不自动阻止安全浏览或修复；实际原生结构错误仍阻断保存。
- 标准 11 位 Classical CAN，DLC 1–8；每 ECU 最多 32 帧、64 信号。每帧可映射多个不重叠的 1–32 位无符号 LSB0 小端信号。导入时拒绝与 Com 位段或位序冲突的 I-PDU 映射、重复的 CanIf PDU 映射，以及与 CanIf 不一致的关联 CAN 网络帧 ID 或布局；未解析的配置变体会阻止生成。Tx 帧按虚拟时钟周期发送；Rx 帧按最后有效接收时间判定超时。
- 生成工程包含独立的 Com、PduR、LSduR、CanIf、CanTp、Dcm、可选 Dem/NvM、虚拟 Can、Os 周期调度、Rte 接口及 ECU 配置；目录内的 `README.md` 和 stdlib-only `tools/ecu-tool.py` 给出锁定工具链的离线构建及按配置启动方法，`profile.txt`、`target.json` 和 `files.list` 描述生成结果，`Dcm_Externals.h` 随工程交付诊断回调声明。未配置诊断时目标仍是纯信号 ECU。legacy 目标产出 `ecu_host.exe`（Windows）或 `ecu_host`（Linux）；源码、ARXML 和构建目录必须分开。Windows BCrypt 安全档案不适用于 Linux，源码准备时明确拒绝，不生成缺少安全后端的工程。
- `files.sha256` 保存生成文件与 `files.list` 的逐项内容摘要，供再次生成前核验；该记录不等于对第三方修改的签名认证。
- 可选诊断配置：一对不与信号帧冲突的 11 位物理 CAN ID；S3 至少 5000 ms，N_As/N_Bs/N_Cr 为正毫秒；一个扩展会话专属 DID，按配置顺序绑定 1–8 个实时 32-bit Tx Com 信号。支持 0x10 默认/扩展会话、0x3E TesterPresent（含抑制正响应）、0x22 读取 DID；同一 0x22 请求可列出多个 DID，按请求顺序返回当前可读的值。配置 DID 在默认会话不可读；保留 DID `0xF186` 在默认/扩展会话返回当前活动会话，S3 回退后返回默认会话，且不可配置为普通 DID。仅请求不可读 DID 时返回 NRC 0x31；请求格式错误返回 NRC 0x13，响应超出 256 字节上限返回 NRC 0x14。可单独启用 0x2E，在扩展会话将完整的大端数据记录写入同一 DID 所绑定的 Tx Com 信号；0x22 与周期 CAN 立即反映新值，ECU 进程重启恢复配置初值。默认会话或 DID 不匹配返回 NRC 0x31，记录长度不符返回 NRC 0x13；未启用时 0x2E 返回 NRC 0x11。该写入仅为易失的主机虚拟应用状态，不写入 NvM/Flash。CanTp 支持单帧、多帧、流控、序号校验、块大小/STmin、N_As/N_Bs/N_Cr 超时与错误后恢复，N-SDU 上限 256 字节。配置可修改或移除；重新导入后仍能识别，超出该确定性子集的诊断配置会阻止生成。
- 可选 0x31/0x01 StartRoutine：先启用上述 0x2E，再设置单个 16-bit RID；仅在扩展会话中将该 DID 绑定的 Tx Com 信号恢复为配置初值。未配置时返回 NRC 0x11，默认会话或 RID 不匹配返回 NRC 0x31，StopRoutine/RequestRoutineResults 返回 NRC 0x12，请求长度错误返回 NRC 0x13；不支持选项或状态记录。该例程只修改易失状态，不是安全解锁或持久化操作。
- 可选故障记忆：诊断连接下配置一个 UDS DTC（`0x000100–0xFFFFFE`），绑定一条已有信号且超时为正的 Rx 帧。首次有效接收后超时才报告故障；Dem 状态跨 ECU 进程重启保存在主机文件。0x19/0x01 按状态掩码读取匹配 DTC 数量（当前单 DTC 配置因此返回 0 或 1）；0x19/0x02 按状态掩码在默认或扩展会话读取该 DTC；0x19/0x0A 无需状态掩码，列出配置的 DTC 及当前状态，包括状态为零的监测项；默认/扩展会话均可读，启用安全档案也无需解锁。0x0A 多带参数返回 NRC 0x13，未配置 DTC 返回 NRC 0x11，其他 0x19 子功能（含 0x8A 抑制位）不支持。0x14 仅扩展会话、仅 `0xFFFFFF` 全部清除。NvM 使用两个固定 CRC32 保护槽位、配置指纹并在状态更改确认前同步落盘；已有文件的任一槽位损坏均拒绝启动（`E NVM`），不存在的文件初始化为空状态。每次主机 ECU 进程启动代表新的操作周期，不等价于完整车辆生命周期模型。
- 配置该单 DTC 时，扩展会话还支持 `0x85/0x02` 暂停 DTC 设置、`0x85/0x01` 恢复；暂停期间监测 Rx 帧仍更新信号有效性，但不修改或持久化故障状态，已有 DTC 仍可读取及清除。切回默认会话（含 S3 超时）或 ECU 进程重启会自动恢复记录；不支持选项记录、抑制正响应位或其他子功能。未配置 DTC 时返回 NRC `0x11`，默认会话拒绝此服务。
- 可选主机安全访问：单级 `0x27/0x01` 请求 16 字节随机 seed、`0x27/0x02` 提交 16 字节 key；扩展会话内解锁。启用后，`0x2E`、`0x31/0x01`、`0x14`、`0x85` 在已满足原有会话与参数条件时，未解锁返回 NRC `0x33`。三次错误 key 后延迟 5 秒；失败计数跨主机进程重启保存在独立状态文件，状态损坏则拒绝启动。密钥是运行时提供的独立 32 字节文件，不进入 ARXML 或生成工程；该文件与状态文件不等于硬件安全存储，也不提供量产级认证保证。详见 [`runtime/README.md`](runtime/README.md)。
- 主机信号闭环检查两个 ECU 的独立信号值与位向量、CAN ID 优先顺序、丢帧后的接收超时、BUS_OFF/STARTED 恢复以及错误 DLC 拒绝。独立诊断测试器对生成的 ECU 注入物理 CAN 报文，检查会话、`0xF186` 活动会话 DID、实时 DID 多帧载荷、流控、N_Bs/N_Cr 超时、错误序号、后续请求恢复与 S3 回退；启用写入时还检查 0x2E 多帧请求、0x22/CAN 信号更新、会话限制与重启后恢复初值；配置例程时检查 0x31/0x01 恢复后 0x22 和 Com 信号的初值以及 RID、子功能、长度和 S3 限制；配置 DTC 时另用隔离存储验证 Rx 超时后 0x19/0x01 的匹配/不匹配/清零数量及 0x19/0x02 单 DTC 报告、0x19/0x0A 在无故障/暂停记录/超时/重启/清除后的支持列表和错误请求拒绝、0x85 禁用/恢复记录、跨进程保持、会话权限清除与损坏拒绝。两种运行结果分别呈现。虚拟总线不模拟电气层或位级仲裁。

新建工程的 EcuC PDU 配置按 R24-11 MOD 建立一个虚拟核心与全局 `EcucPduCollection/Pdu`；配置 Com 帧时还提供必需的 `ComGeneral`。`ComPduIdRef` 与 CanIf Tx/Rx PDU 引用指向独立的全局 Pdu 容器；系统 `I-SIGNAL-I-PDU`、`N-PDU`、`DCM-I-PDU` 保留各自语义，全局 Pdu 到系统 PDU 的关系由版本化**工具专属 SDG** 记录。固定主机剖面现在还生成虚拟 Mcu 时钟、Can 控制器及 Rx/Tx 硬件对象、CanIf 驱动/HOH/缓冲和必需的 PDU 参数与引用；导入时按同一剖面检查，不符合者只读阻断。这些虚拟时钟、基地址和位时序值不表示真实 MCU 配置，运行时代码也不是第三方 CanIf/Can 标准 ABI；尚无第三方协议栈或硬件互操作证据。诊断的 Dcm/CanTp/N-PDU 路径仍仅对本主机目标验证；可选写入使用 DcmDspDidWrite 与逐数据回调。主机专属的 0x31/0x01 例程把 RID 和固定扩展会话记录在 DID 工具 SDG 中，**不**输出 DcmDsd 0x31 服务或 DcmDspRoutine/StartRoutine/CommonAuthorization；第三方 Dcm 不能依据该 ARXML 获得此例程。DcmDspData 与 Com 信号、Dem 事件与监测 Rx 帧的绑定也由工具 SDG 记录。可选 0x27 的 ECUC 行仅描述主机目标的固定单级档案；所列主机回调并非第三方 Dcm ABI 的互操作证明。主机 NvM 不建模真实 Ea/Fee/MemIf 物理目标或分区引用，也未建模 ComM/EcuM/BswM/CanSM；未实现其他例程/写入 DID/持久写入、完整诊断子功能、功能寻址或完整 ISO 14229/15765 一致性。无真实硬件或标准符合性证明；不支持 29 位 CAN、签名/大端信号、真实芯片驱动与未解析配置变体。界面“通过”仅对应所运行的主机路径，详见 [`runtime/README.md`](runtime/README.md)。

诊断配置中的六项 CanTp N-SDU、N-PDU/FC N-PDU 引用及两项 `DcmDslProtocolRx/TxPduRef` 也按 MOD 指向相应 EcuC 全局 Pdu，`VALUE-REF DEST="ECUC-CONTAINER-VALUE"`；系统 N-PDU/DCM-I-PDU 仅保留系统描述与工具绑定。按 System Template `[constr_3448]`，这些系统 PDU 对应的全局容器和 Com 的 I-SIGNAL-I-PDU 全局容器均**不写** `DynamicLength`，系统 N-PDU 也不写 `HAS-DYNAMIC-LENGTH`：N-PDU 长度由 TP 处理，DcmIPdu 的动态长度由系统模板语义决定，不从 EcuC 布尔值推断。含此不适用字段的导入配置只读阻断，不自动改写。

生成的主机工程对每个 DID 数据提供可外部链接的 `Ecu_DcmRead_<index>`，启用 0x2E 时还提供 `Ecu_DcmWrite_<index>`；主机 Dcm 实际通过这些回调读写 Com 信号，而不只在 ARXML 中填写函数名。`Dcm_Externals.h` 声明这些回调，`include/Ecu_DcmCallbackTypes.h` 仅定义本主机剖面所需的 `Std_ReturnType` 等类型，不是完整 AUTOSAR `Std_Types.h` 或生成的 RTE 类型头，也不证明第三方 Dcm 可直接接入。0x31 仍由生成工程的内部 `Ecu_HostRestoreDid` 实现；其 R24-11 ECUC 例程函数签名参数属草案，本产品不声明该例程的标准 ECUC 集成。

既有主机生成目标仍要求当前 R24-11 闭包：Com/CanIf/CanTp/Dcm 的 ECUC PDU 引用指向 EcuC 全局 Pdu，固定主机 CAN 配置包含 Mcu/Can/CanIf 关系，主机专属 0x31 只使用 DID 工具 SDG；单 DTC 还要求 DcmDsd 0x85、`DcmDspControlDTCSetting` 禁用选项记录与真实 `DcmDemClientRef`。缺少或不能解释这些消费者时拒绝相应生成，不自动改源、不扩大运行时 allowlist。默认原生配置按分域规则允许安全浏览及未恶化违规的定义编辑；兼容 v1／开发 oracle 路径仍执行原来的固定官方资源和严格拒绝。

既有信号生成剖面的 ComIPdu/ComSignal 必须直接归属本工程唯一的 `/{项目名}/ComCfg/ComConfig`，模块定义及 ComGeneral 必须正确；重命名模块、把子容器挂到其他模块或改动父级 `DEFINITION-REF` 会阻断该目标生成，不按局部定义猜测所有者。通用配置编辑另按真实定义与分域违规规则判断，不把生成剖面的限制当成全工程编辑总门。

## 本地开发环境

工具版本由根目录 `rust-toolchain.toml`（Rust 1.98.1）、`.node-version`（Node 24.19.0）、`.python-version`（CPython 3.12.9）、`pyproject.toml`/`uv.lock`（Python）、`ui/package.json` 的 npm 11.17.0 engine 约束及 `ui/package-lock.json` 的包完整性记录约束。安装 Rust/rustfmt/clippy、Node/npm、uv、Git 和目标所需的 C99 GCC；在**新的终端**检查安装结果。Cargo 构建产物保留在 `core/target/` 与 `src-tauri/target/`，不改设 `CARGO_TARGET_DIR`。

- Windows：使用 Rust MSVC、Visual Studio C++ Build Tools 和 WebView2。安装 vcpkg 的 `libxml2[iconv,zlib]:x64-windows-static-md`；`VCPKG_ROOT` 指 vcpkg 根目录，`VCPKGRS_TRIPLET=x64-windows-static-md`，`LIBCLANG_PATH` 指含 `libclang.dll` 的目录。可在用户环境中设置这些变量，重新打开终端和 Agent 宿主后再检查；不要把个人安装路径写进工程。原生执行另需 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 指向目标锁声明的 GCC、objdump、Git 和 CPython 绝对路径；不回退 PATH 工具。
- Ubuntu 24.04：安装 `build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev libxml2-dev libclang-dev clang pkg-config patchelf`，并按锁文件安装 Rust、Node、CPython 与 uv。WSL 构建副本、Cargo target 和 uv 缓存放在 ext4 文件系统；开发 oracle 从 Windows 卷读取官方档案时显式传路径。此前固定 GCC13.3.0 的受控 OS 26 项 suite、生产 ECU 独立协议及 deb／解包 AppImage 隔离桌面路径已有历史实测；当前 Epic 7 仍以本次冻结产物的实际复验为准。
- macOS：安装 Xcode Command Line Tools、pkg-config/libxml2 和上述版本管理工具。源码工作台代码路径、隔离 IPC 测试入口与 app/dmg 配置已实现；macOS 原生构建、bundle 和 IPC 未验证，不提供本机虚拟 ECU。

本地官方材料须由使用者自行合法放置，不随源码或安装包分发：

- XSD：`docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`（包含 `AUTOSAR_00053.xsd` 与 `xml.xsd`）。
- MOD：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`。开发 oracle 和兼容 v1 路径使用显式 XSD/MOD 并核对固定 R24-11 SHA-256；`AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE` 不替换默认内置规则权威。外观、规则与模块定义、执行工具是独立设置类别；外观／工具原子保存到 Tauri `app_config_dir/settings.json`，工具环境覆盖优先且不写回。`AUTOSAR_CONFIG_DIR` 可指定独立绝对配置目录，原生验收必须显式设置，不能仅修改 `APPDATA`。
- 集成样例：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip`。核心测试另读取上述两项合法规范档案。

从新克隆的源码根目录运行（须预先准备平台原生依赖与官方档案）：

```sh
uv sync --locked --group quality
npm ci --prefix ui
uv run --locked python -m autosar_tooling doctor --role workbench
npm run tauri --prefix ui -- info
uv run --locked python -m autosar_tooling verify --scope all --base <本轮起始提交>
```

`doctor` 只读报告 `ready`、`missing`、`version_mismatch` 或 `not_applicable`，缺少必需项返回非零并提示安装/配置；不下载规范或更改系统。`npm run tauri --prefix ui -- dev` 使用现有 Tauri/Vite hooks，在**独立桌面会话**交互调试；自动化不得在当前用户桌面弹窗或抢焦点。这里不会执行 `create-tauri-app` 或 `tauri init --force` 重建已有工作台。开发构建使用 Vite；发行包使用内嵌界面及可信运行资源，真实安装包路径必须另行原生验收，不能从开发构建成功推定。

桌面标题与应用菜单合并为同一条浅／深主题栏。顶部空白与标题区域可拖动，双击切换最大化；右侧提供最小化、最大化／还原和关闭，关闭仍先处理未应用草稿及未保存修改。工具窗口、工程树和检查器使用方向图标收起，悬停提示说明操作；底部工具标签及侧栏可重新展开。

工程树右边缘、检查器左边缘和底部工具窗口上边缘提供拖动分隔条，分别调整左右侧栏宽度及底栏高度；其他方向随工作区伸缩。聚焦分隔条后用方向键微调，Shift＋方向键调整更大步幅，Home／End 调到当前允许边界；双击或 Enter 恢复默认。折叠、切换工具标签及缩小后再放大窗口保留本次运行的首选尺寸，重启后恢复默认布局；尺寸边界会为编辑区保留空间。

受控外部命令已有统一的 `core::execution` 与 `ecu_tools.process` 入口。开发聚合器将当前锁定的 `sys.executable` 作为 `AUTOSAR_PYTHON` 传给 Cargo；直接运行 Cargo 时，须把该变量设为 CPython 的**绝对路径**（开发环境 Windows 为 `.venv/Scripts/python.exe`，POSIX 为 `.venv/bin/python`），不得依赖 PATH 猜测解释器。可运行 `uv run --locked python -m unittest autosar_tooling.test_process` 与 `cargo test --locked --manifest-path core/Cargo.toml execution::tests` 检查真实父/子/孙进程的退出、超时、取消及故障清理。Windows 使用先登记后恢复的 Job；POSIX 保证已登记的合作进程组及未逃逸后代在绝对单调期限内关闭。未登记的 `setsid`/daemon 逃逸不视为成功清理，Linux 反例返回 `cleanup_unconfirmed`。产品构建、离线验证与 legacy 主机运行均已使用这一受控执行入口。

命令完成与进程回收分开判断：默认 `require_tree_exit` 用于 ECU、协议及生命周期检查，主进程退出后遗留活后代仍失败；开发聚合器仅对 Cargo test/build/Clippy 显式选择 `close_tree_on_exit`，主命令结束后 owner 有界关闭其私有 Job/已登记 scope，确认无残留后保留原退出码。发生实际回收时输出 `cleanup=confirmed descendants=reclaimed`；非零、超时、取消、逃逸或无法确认清理均不能转成成功。不按进程名加白名单，不影响同级任务或用户已有进程，不要求关闭机器级编译器遥测。内部测试与产品命令仍维持各自严格契约。

当前 OS 独立消费者入口为 `uv run --locked python -m autosar_tooling os --target windows-x64-controlled-v1 --suite all`；在 Ubuntu24.04 原生环境把 target 改为 `linux-x64-controlled-v1`，不能跨宿主运行。26 项适用 suite 已分别接入 core 集成测试，ARTI 原生消费者随 stack suite 执行一次。固定内核、补丁、原生执行栈及 Windows PE／Linux ELF 差异见 [`runtime/os/README.md`](runtime/os/README.md)。生产 ECU 的两个目标、stdlib-only 离线 build/verify、Windows MSI 与 Linux deb/AppImage 的无 checkout 隔离桌面真实 IPC 均已实际运行；这些是受控原生主机能力，不是实机或标准认证。

源码准备固定 `windows-x64-controlled-v1` 与 `linux-x64-controlled-v1` 输出目标；`AssetInventory` 在构建时核对 BSW、OS、FreeRTOS 原件、补丁、目标锁与交付素材的可信摘要。显式目录加载以编译进工作台的清单核对，不相信目录自报的散列。`prepare_ecu_project`/`prepare_host_project` 返回纯内存源码、`autosar-build-target-v1` 元数据与输入/目标/资源 fingerprint，预检为 `not_run`；不调用 Git/GCC/PowerShell，也不安装源码包。原生预检是显式独立操作；非本机目标可渲染与重导入，但不能在当前宿主执行。生成后的交付闭包包含 `tools/ecu-tool.py` 和全部 stdlib 工具，接收者无需 checkout、uv、Rust 或 Node。

明确检查本机能否执行**目标**时另运行 `uv run --locked python -m autosar_tooling doctor --role native --target windows-x64-controlled-v1`，Ubuntu24.04 原生环境将 target 改为 `linux-x64-controlled-v1`。此命令才有界启动 Git、固定 GCC、binutils 与 CPython 身份查询并核对合法 XSD/MOD；缺失或身份不符返回非零。本机不能执行某目标不妨碍上述纯源码预览。

## 原生分发与图标

在各自原生宿主、已准备上述开发依赖的源码根目录执行 `npm run tauri --prefix ui -- build`。Tauri 自动合并 `src-tauri/tauri.<platform>.conf.json`：Windows 生成中文 MSI（`zh-CN`，保留中文产品名）；Linux 生成 deb/AppImage，包名使用 `Classic CAN Workbench`；macOS 配置 app/dmg，但尚无原生验证。产物位于 `src-tauri/target/release/bundle/`；没有签名、公证或远端发布声明。

运行已提取或安装的应用不需要 npm、uv、Rust 或 checkout；Windows 需要 WebView2，Linux 需要 WebKitGTK 4.1、Ayatana AppIndicator 与 libxml2。普通配置、保存、原生检查及源码交付不依赖官方档案或外部编译器；开发 oracle／兼容 v1 重导入才需要合法匹配的 XSD/MOD。原生 ECU 预检、构建和行为验证另需声明的 CPython 3.12.9、GCC、objdump 与 Git。安装包不携带官方档案或编译器。无 FUSE 时可解包 AppImage 后运行 `squashfs-root/AppRun`。

发行包复验使用现有独立入口，附加 `--installed --source-checkout <发行构建时的原始源码路径>`；只能搬离本次 owned 构建副本，不得搬动用户工作树。`--binary` 指 checkout 外的真实解包应用；`--builtin-only` 分支使用无官方资源、无开发工具／编译器的配置环境，独立消费者阶段再提供目标工具链。默认 oracle／v1 分支保留合法参考资源及原拒绝。应用从私有空 cwd/config 与最小 PATH 启动，不使用 Vite；外部自动化 driver 不进入产品环境。各次实际出口结论写对应 BMad 工件，build/test 不替代原生发行验收。

此前验收采用 Windows MSI 行政解包（`msiexec /a <MSI> /qn TARGETDIR=<私有目录>`）及 Linux deb 解包（`dpkg-deb -x <deb> <私有目录>`）／AppImage `AppRun`。历史结果不自动适用于当前变更；本次 Epic 7 的双平台出口以当前 spec 实际运行记录为准。系统级安装／卸载、升级、签名、公证、远端发布及 WebView2 首装下载不由行政解包推定。

暂定身份使用透明珊瑚 A 图形：`ui/public/logo-app.png`、浅／深色 `logo-workbench.png`／`logo-workbench-dark.png` 与 `src-tauri/icons/` 的 PNG/ICO/ICNS。启动、导航、favicon、窗口和包配置不再引用旧 ECU 芯片 SVG。固定 ICO/ICNS 不宣称随 OS 自动切换明暗。

## 界面设计基线

正式工作台沿用以下设计与交互基线；冻结原型仍只作评审参考：

- [视觉系统](DESIGN.md)：浅深主题、语义角色色、字体、布局及组件规范；根目录为后续实施的视觉入口，日期工作区为冻结评审快照。
- [交互规格](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md)：当前操作映射、两种输入剖面、草稿／保存／结果生命周期、关键流程及扩展接入规则。
- [交互原型](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/index.html)与[汽车电子图标提案](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/logo.html)：设计评审材料，全部项目数据与操作结果在内存中模拟，刷新重置。

从仓库根目录启动独立静态预览：

```powershell
uv run --locked python -m http.server 1421 --bind 127.0.0.1 --directory _bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups
```

访问 `http://127.0.0.1:1421/` 与 `http://127.0.0.1:1421/logo.html`。原型不读取官方档案、不写 ARXML、不生成真实源码、不编译或运行 ECU；它不替代正式 UI／IPC 的实际验收。

## 代码质量检查

Epic 7 当前按开发收尾推进：使用既有单元／集成测试、静态检查、UI 构建和桌面后端编译／Clippy 检查，不启动桌面程序或执行真机、完整发行场景。发行包的环境与主要使用链验收留给后续 CI 阶段，单独记录结果；不为当前开发新增隔离或验收设施。当前实现与未验发行范围见 [Epic 7 实施规格](_bmad-output/implementation-artifacts/spec-epic-7-configurator.md)。

Windows GBK 终端可使用 `uv run --locked python -X utf8 -m autosar_tooling verify --scope all --base <起始提交>`，避免打印构建工具的 Unicode 日志时因终端编码中断；无需修改系统编码。

质量工具及独立 ARTI 消费者使用的 `lxml` 由 `uv.lock` 中的 `quality` 组固定；UI Prettier/ESLint 由 npm 锁文件固定。定向检查使用 `uv run --locked python -m autosar_tooling quality --base <本轮起始提交>`，检查 UTF-8、末尾换行、空白、Python 语法与增量 rustfmt/clang-format/Prettier、主机 C99 语法。唯一聚合入口为 `uv run --locked python -m autosar_tooling verify --scope core|ui|desktop|all --base <起始提交>`：`all` 顺序组合 Git diff/check、Python unittest、npm ci、质量、Ruff、UI lint/build、core test/clippy 和 desktop build/clippy。每个命令使用独立的绝对单调 deadline、进程所有权与日志；失败报告具体子阶段、argv、观测到的退出码和日志位置，不重试或吞错。26 项原生 OS suite 只经注册的 Cargo 测试执行一次，不追加第二次 OS CLI。此构建／测试关口明确报告平台适用范围，**不**验证真实 GUI/IPC 或安装包；这些使用独立 native desktop 和 bundle 关口。

真实 GUI/IPC 使用独立入口 `uv run --locked python -m autosar_tooling desktop --platform windows|linux|macos --binary <本次桌面程序>`，不由上述 build/test PASS 推定。Windows/Linux 运行前构建同版 `core/target/debug/package_host_reference` 和桌面程序；Windows 用独立 Desktop/Job/CDP，Linux 用私有 Xvfb 与固定 `tauri-driver 2.1.0`/原生 WebKit driver。Linux 另安装 `xvfb x11-utils xdotool webkit2gtk-driver`，并运行 `cargo install tauri-driver --version 2.1.0 --locked`。macOS 的 `native-webdriver` feature/capability 仅用于独立登录会话中的源码工作台测试，目前无原生验证；生产构建不包含它。截图、driver 日志与真实 IPC 记录留在入口报告的私有证据目录，Linux/macOS 使用用户缓存目录保留跨会话结果；结论记录在本轮 BMad spec。


Epic 4 的生成工件测试仍以 PATH 中的 Cppcheck 2.21.0 检查实际 RTE 翻译单元，不是完整 MISRA 扫描。BMad 开发规格记录起始提交；格式检查应指定 `--base`，修改旧文件无需整体重排。交付正式能力仍以相应 BMad spec 的原生运行结果为准。

## 使用顺序

项目开发由仓库内固定版本 BMad 接管。[产品简述](_bmad-output/planning-artifacts/product-brief.md)、[PRD](_bmad-output/planning-artifacts/prd.md)、[架构](_bmad-output/planning-artifacts/architecture.md)与 [epics](_bmad-output/planning-artifacts/epics.md) 是当前规划入口；`_bmad-output/implementation-artifacts/sprint-status.yaml` 记录任务状态。学习层目前只在产品简述中列为待规划方向。面向使用者的[项目使用说明](docs/project/OWNER_GUIDE.md)给出日常说法；Agent 按委托范围调用仓库安装的 BMad 技能。开发、验证、复核和流程工件完全按安装的 BMad 执行，验证结论进入对应 story/spec；早期产品决定与研究分别归档于 `docs/project/archive/` 和 `docs/research/autosar-platform/`，迁移前任务及反馈保存在 Git 历史，均不参与选题。

已提交的 `.agents/skills/` 和 `_bmad/` 可直接供 Codex 使用；重新安装或更新时固定 `bmad-method@6.12.0`、BMM 和 `codex`，先核对安装器差异，不让新版本覆盖团队定制。`_bmad/config.user.toml` 是被 Git 忽略的个人安装答案；团队共用语言和配置放在 `_bmad/custom/config.toml`。从仓库打开 Codex 会话可调用 `bmad-help` 查看当前阶段，也可直接委托一个 story、epic 或规划目标。

1. 从工程入口选择内置模板新建工程、打开成员 manifest 或一次选择同一 ECU 的全部 ARXML。直接 ARXML 可继续使用，显式另存为工程才持久化成员与扩展接纳选择；不能信任 manifest 的 profile hint 或旧通过状态。
2. 在工程树／对象表选择真实对象，按定义检查与编辑字段、引用或结构；跨对象修改先查看整个批次的实际旧／新值与入站影响，再一次应用。既有 CAN、诊断／DTC 与标准参数编辑仍使用同一 Workspace。未应用草稿不写源；切对象先处理草稿，替换工程另处理 dirty 与保存确认。
3. 保存先查看各文件差异并确认；外部改源、过期预览或未恢复备份拒绝覆盖。源码生成另选择工程之外的输出目录，预览真实文件与拥有权再确认，不启动编译。构建目录继续独立，编译／运行只在对应工具与本机目标可用时执行。
4. 在“虚拟运行”页，对已构建、已配置诊断的当前工程点击“验证诊断连接”，由独立测试器检查本次二进制；配置故障记忆时使用隔离 NvM 文件，启用 Windows 0x27 时另用隔离密钥与安全状态文件。不修改真实 ECU 状态，不需要对端 ECU。信号总线验证另选对端 ECU 的封存源码目录与已构建的实际二进制，运行双 ECU 闭环。

默认 R5 源码交接使用 `autosar-workbench-handoff-v2`，精确封存规则三字段身份、必需接纳扩展、输入、应用 snapshot 与拥有权；同版工作台在新空目录重建原始成员，重新校验、重渲染并逐字节核对。没有把包内 JSON 当成可执行规则，也不把旧官方资源摘要转换成内置规则摘要。原 `autosar-host-handoff-v1`／`autosar-ecu-handoff-v1` 继续精确分派，并保留各自的合法 XSD/MOD 与完整性要求；不会自动升级旧包。

标准 ECU 的 live 应用初始化先展示真实 `epic4-single-application-v1` 槽描述符、源码与 manifest 变化，明确确认后只创建不存在的文件。声明的 `applicationInputs` 后续从当前用户字节形成不可改的封存 snapshot；生成器不写回 live 源，不覆盖改过的生成文件或未知 owner／清单版本。应用改动也会使旧生成确认和下游结果失效。

固定旧目标双 ECU 离线参考包仍可用 `package_host_reference` 与合法固定 XSD 显式生成，详见 [`runtime/reference-README.md`](runtime/reference-README.md)。生成、预检、构建和实际行为复验分别报告；配置内置定义可读不等于目标可生成。

“本机预检”独立实际编译并返回绑定源码身份的 `not_run|passed|failed`；非本机目标为 `not_run`，不标成功。“构建 ECU”使用独立空目录和包内目标工具链，Windows 得到 `ecu_host_batch.exe`，Linux 得到 `ecu_host_batch`。“验证 ECU 主机行为”独立重建并逐字节核对 CAN/DID、至多两个 DID、真实 N_Cr 超时恢复及非法批次。离线入口为 `<CPython3.12.9> tools/ecu-tool.py verify --project <封存源码目录> --build-directory <工程之外的新空目录>`。两平台生产协议已通过原生核心入口实测；Linux 桌面 IPC 与正式 bundle 不由该结果推定。当前工程完整 SC1 复验及实机状态不从这些有界向量推断。

重复生成前，工具核对 `files.list` 的准确文件名、`files.sha256` 中每份生成文件及清单自身的 SHA-256、缺失/额外文件与目录；不匹配即拒绝替换并保留原目录。旧版没有完整性记录的生成目录不能直接覆盖，请选新空目录并保留旧目录。二进制和编译日志只进入工程之外的构建目录；源码包中出现未列二进制、文件或链接时拒绝重建和重导入，不自动清理。

再次生成成功时，原目录**不会被删除**：它会移到同一父目录下独占的新备份位置；返回结果和“生成与构建”页面显示其完整路径。多次生成会积累多份旧工程，由文件所有者确认无用后自行归档或清理，工具不自动回收。生成失败时，未移动的原目录保持原位；若已经移走，则报错中列出可恢复的旧目录路径。失败的临时生成目录也保留并在报错中给出路径，供所有者检查后自行处理。勿将这些备份目录误当成本次构建工程。

构建要求工程之外的新空目录，拒绝覆盖已有二进制、日志或其他所有者文件。编译先写入构建目录内独占的私有目录，再以不覆盖目标的硬链接安装；文件系统不支持硬链接或并发出现同名二进制时直接报错，不降级为覆盖。失败保留实际输出和受管日志；成功只清理工具自己的临时编译产物。

工作台与离线 `tools/ecu-tool.py build` 都在编译前和安装二进制前核对源码清单、SHA-256 与缺失/额外文件，拒绝链接/reparse point；内核补丁只应用到外部构建目录中的私有副本。生成后源码、目标或清单被改动时，工作台拒绝把该目录标为本次配置的结果，请重新生成到空目录。摘要用于完整性检查，不是发布者签名认证。所有实际命令以 argv、绝对工具路径和单调截止时间在 owned scope 中执行，不使用 PowerShell 拼接或 taskkill 清理。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。

Epic4最终独立交接的正式入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_handoff -- --exact`；全部主机等级与旧目标回归使用完整核心测试。独立执行和复核结果记录在BMad 4.22及sprint；任意用户工程的一次生成、构建或包内CAN／DID检查不会自动成为完整SC1或实机验证。

Epic4的22条story已完成，固定`epic4-win64-sr-cs-v1`目标的全部适用主机SC1行为、跨story集成与非实现者交接已通过BMad验证／复核。结论限定CP/FO R24-11、Windows x64、GCC 16.1.0、固定FreeRTOS及声明配置；没有推定任意工程已复验、MCU、硬实时、ASIL、完整MISRA或官方符合性认证。
