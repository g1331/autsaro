# Classic CAN 配置工作台

本地桌面工作台：为一个 ECU 新建或导入 AUTOSAR Classic R24-11 ARXML 配置，编辑受支持的 CAN 帧、信号、一条主机虚拟 DoCAN 物理诊断连接与可选的单个超时故障 DTC，使用官方 XSD 校验，生成独立 C99 工程并构建主机目标。两份不同配置可作为两个独立 ECU 进程接入主机虚拟 CAN 总线；诊断连接由独立测试器驱动。**当前不是全模块 AUTOSAR 实现，也没有真实硬件验证或发布包。**

## 当前支持

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。校验、保存和生成都会复核所选的全部来源文件；任一文件被外部修改时拒绝继续使用旧跨文件配置，并提示重新导入。保存时暂存新内容，在替换前再次复核待写文件；无法安全回滚时保留原备份。
- 编辑配置后，“查看并保存 ARXML”会先列出每份来源文件将修改或保持不变，并显示修改文件的前后差异及完整原文。预览不写磁盘；用户确认后才保存。若预览后配置或来源文件变化，旧预览不能用于保存。
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

工作区只按当前 R24-11 配置模型解析和保存：Com/CanIf/CanTp/Dcm 的 ECUC PDU 引用须指向 EcuC 全局 Pdu，主机 CAN 配置须包含上述固定 Mcu/Can/CanIf 闭包，主机专属 0x31 例程只使用 DID 工具 SDG；单 DTC 配置还须包含 DcmDsd 0x85 服务、`DcmDspControlDTCSetting` 禁用选项记录及指向 DemClient 的 `DcmDemClientRef`。没有旧工具格式的自动迁移、回退或保存确认流程；缺少当前必需结构的来源文件可查看诊断，但不自动改写，不能保存或生成。未受支持却不影响有效配置的内容仍作为保留项原样保留。

受支持的 ComIPdu/ComSignal 必须直接归属本工程唯一的 `/{项目名}/ComCfg/ComConfig`，其模块定义和 ComGeneral 必须正确；重命名模块、把子容器挂到其他模块或改动父级 `DEFINITION-REF` 即使保留了可解析的 Pdu/Signal 引用，也只读阻断，不按子容器局部定义猜测所有者。

## 本地开发环境

工具版本由根目录 `rust-toolchain.toml`（Rust 1.98.1）、`.node-version`（Node 24.19.0）、`.python-version`（CPython 3.12.9）、`pyproject.toml`/`uv.lock`（Python）、`ui/package.json` 的 npm 11.17.0 engine 约束及 `ui/package-lock.json` 的包完整性记录约束。安装 Rust/rustfmt/clippy、Node/npm、uv、Git 和目标所需的 C99 GCC；在**新的终端**检查安装结果。Cargo 构建产物保留在 `core/target/` 与 `src-tauri/target/`，不改设 `CARGO_TARGET_DIR`。

- Windows：使用 Rust MSVC、Visual Studio C++ Build Tools 和 WebView2。安装 vcpkg 的 `libxml2[iconv,zlib]:x64-windows-static-md`；`VCPKG_ROOT` 指 vcpkg 根目录，`VCPKGRS_TRIPLET=x64-windows-static-md`，`LIBCLANG_PATH` 指含 `libclang.dll` 的目录。可在用户环境中设置这些变量，重新打开终端和 Agent 宿主后再检查；不要把个人安装路径写进工程。原生执行另需 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 指向目标锁声明的 GCC、objdump、Git 和 CPython 绝对路径；不回退 PATH 工具。
- Ubuntu 24.04：安装 `build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2.0-dev libssl-dev libxdo-dev libxml2-dev libclang-dev clang pkg-config patchelf`，并按锁文件安装 Rust、Node、CPython 与 uv。WSL 构建副本、Cargo target 和 uv 缓存应放在 ext4 文件系统；从 Windows 卷读取官方档案时显式传路径。固定 GCC13.3.0 的受控原生 OS 已运行全部 26 项独立 suite；Linux 生产 ECU 的封包、搬移、构建及 CAN/DID/N_Cr 独立协议也已实测。桌面 IPC 与正式 bundle 仍按后续阶段验收。
- macOS：安装 Xcode Command Line Tools、pkg-config/libxml2 和上述版本管理工具。macOS 源码工作台与包配置仍待本轮完成，原生构建和 IPC 未验证；macOS 不提供本机虚拟 ECU。

本地官方材料须由使用者自行合法放置，不随源码或安装包分发：

- XSD：`docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`（包含 `AUTOSAR_00053.xsd` 与 `xml.xsd`）。
- MOD：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`。核心计划使用显式 XSD/MOD 路径并核对固定 R24-11 SHA-256；CLI 可传 `--xsd-archive`/`--mod-archive`，进程变量 `AUTOSAR_XSD_ARCHIVE`/`AUTOSAR_MOD_ARCHIVE` 优先于桌面配置但不会写回。桌面应用配置目录（Tauri `app_config_dir`）的 `settings.json` 可写 `{"xsdArchive":"<XSD绝对路径>","modArchive":"<MOD绝对路径>"}`；设置界面将在后续阶段提供。未配置时明确报告缺少资源，不查找 checkout 默认路径。
- 集成样例：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip`。核心测试另读取上述两项合法规范档案。

从新克隆的源码根目录运行（须预先准备平台原生依赖与官方档案）：

```sh
uv sync --locked --group quality
npm ci --prefix ui
uv run --locked python -m autosar_tooling doctor --role workbench
npm run tauri --prefix ui -- info
uv run --locked python -m autosar_tooling verify --scope all --base <本轮起始提交>
```

`doctor` 只读报告 `ready`、`missing`、`version_mismatch` 或 `not_applicable`，缺少必需项返回非零并提示安装/配置；不下载规范或更改系统。`npm run tauri --prefix ui -- dev` 使用现有 Tauri/Vite hooks，在**独立桌面会话**交互调试；自动化不得在当前用户桌面弹窗或抢焦点。这里不会执行 `create-tauri-app` 或 `tauri init --force` 重建已有工作台。当前 Windows 生成与构建仍依赖源码目录，正式无checkout bundle 将在本轮最后阶段验证。

受控外部命令已有统一的 `core::execution` 与 `ecu_tools.process` 入口。开发聚合器将当前锁定的 `sys.executable` 作为 `AUTOSAR_PYTHON` 传给 Cargo；直接运行 Cargo 时，须把该变量设为 CPython 的**绝对路径**（开发环境 Windows 为 `.venv/Scripts/python.exe`，POSIX 为 `.venv/bin/python`），不得依赖 PATH 猜测解释器。可运行 `uv run --locked python -m unittest autosar_tooling.test_process` 与 `cargo test --locked --manifest-path core/Cargo.toml execution::tests` 检查真实父/子/孙进程的退出、超时、取消及故障清理。Windows 使用先登记后恢复的 Job；POSIX 保证已登记的合作进程组及未逃逸后代在绝对单调期限内关闭。未登记的 `setsid`/daemon 逃逸不视为成功清理，Linux 反例返回 `cleanup_unconfirmed`。产品构建、离线验证与 legacy 主机运行均已使用这一受控执行入口。

命令完成与进程回收分开判断：默认 `require_tree_exit` 用于 ECU、协议及生命周期检查，主进程退出后遗留活后代仍失败；开发聚合器仅对 Cargo test/build/Clippy 显式选择 `close_tree_on_exit`，主命令结束后 owner 有界关闭其私有 Job/已登记 scope，确认无残留后保留原退出码。发生实际回收时输出 `cleanup=confirmed descendants=reclaimed`；非零、超时、取消、逃逸或无法确认清理均不能转成成功。不按进程名加白名单，不影响同级任务或用户已有进程，不要求关闭机器级编译器遥测。内部测试与产品命令仍维持各自严格契约。

当前 OS 独立消费者入口为 `uv run --locked python -m autosar_tooling os --target windows-x64-controlled-v1 --suite all`；在 Ubuntu24.04 原生环境把 target 改为 `linux-x64-controlled-v1`，不能跨宿主运行。26 项适用 suite 已分别接入 core 集成测试，ARTI 原生消费者随 stack suite 执行一次。固定内核、补丁、原生执行栈及 Windows PE／Linux ELF 差异见 [`runtime/os/README.md`](runtime/os/README.md)。生产 ECU 的两个目标与 stdlib-only 离线 build/verify 已实际运行；完整桌面与分发验收仍待本轮后续阶段。

源码准备固定 `windows-x64-controlled-v1` 与 `linux-x64-controlled-v1` 输出目标；`AssetInventory` 在构建时核对 BSW、OS、FreeRTOS 原件、补丁、目标锁与交付素材的可信摘要。显式目录加载以编译进工作台的清单核对，不相信目录自报的散列。`prepare_ecu_project`/`prepare_host_project` 返回纯内存源码、`autosar-build-target-v1` 元数据与输入/目标/资源 fingerprint，预检为 `not_run`；不调用 Git/GCC/PowerShell，也不安装源码包。原生预检是显式独立操作；非本机目标可渲染与重导入，但不能在当前宿主执行。生成后的交付闭包包含 `tools/ecu-tool.py` 和全部 stdlib 工具，接收者无需 checkout、uv、Rust 或 Node。

明确检查本机能否执行**目标**时另运行 `uv run --locked python -m autosar_tooling doctor --role native --target windows-x64-controlled-v1`，Ubuntu24.04 原生环境将 target 改为 `linux-x64-controlled-v1`。此命令才有界启动 Git、固定 GCC、binutils 与 CPython 身份查询并核对合法 XSD/MOD；缺失或身份不符返回非零。本机不能执行某目标不妨碍上述纯源码预览。

## 代码质量检查

质量工具及独立 ARTI 消费者使用的 `lxml` 由 `uv.lock` 中的 `quality` 组固定；UI Prettier/ESLint 由 npm 锁文件固定。定向检查使用 `uv run --locked python -m autosar_tooling quality --base <本轮起始提交>`，检查 UTF-8、末尾换行、空白、Python 语法与增量 rustfmt/clang-format/Prettier、主机 C99 语法。唯一聚合入口为 `uv run --locked python -m autosar_tooling verify --scope core|ui|desktop|all --base <起始提交>`：`all` 顺序组合 Git diff/check、Python unittest、npm ci、质量、Ruff、UI lint/build、core test/clippy 和 desktop build/clippy。每个命令使用独立的绝对单调 deadline、进程所有权与日志；失败报告具体子阶段、argv、观测到的退出码和日志位置，不重试或吞错。26 项原生 OS suite 只经注册的 Cargo 测试执行一次，不追加第二次 OS CLI。此构建／测试关口明确报告平台适用范围，**不**验证真实 GUI/IPC 或安装包；这些使用独立 native desktop 和 bundle 关口。

Epic 4 的生成工件测试仍以 PATH 中的 Cppcheck 2.21.0 检查实际 RTE 翻译单元，不是完整 MISRA 扫描。BMad 开发规格记录起始提交；格式检查应指定 `--base`，修改旧文件无需整体重排。交付正式能力仍以相应 BMad spec 的原生运行结果为准。

## 使用顺序

项目开发由仓库内固定版本 BMad 接管。[产品简述](_bmad-output/planning-artifacts/product-brief.md)、[PRD](_bmad-output/planning-artifacts/prd.md)、[架构](_bmad-output/planning-artifacts/architecture.md)与 [epics](_bmad-output/planning-artifacts/epics.md) 是当前规划入口；`_bmad-output/implementation-artifacts/sprint-status.yaml` 记录任务状态。学习层目前只在产品简述中列为待规划方向。面向使用者的[项目使用说明](docs/project/OWNER_GUIDE.md)给出日常说法；Agent 按委托范围调用仓库安装的 BMad 技能。开发、验证、复核和流程工件完全按安装的 BMad 执行，验证结论进入对应 story/spec；早期产品决定与研究分别归档于 `docs/project/archive/` 和 `docs/research/autosar-platform/`，迁移前任务及反馈保存在 Git 历史，均不参与选题。

已提交的 `.agents/skills/` 和 `_bmad/` 可直接供 Codex 使用；重新安装或更新时固定 `bmad-method@6.12.0`、BMM 和 `codex`，先核对安装器差异，不让新版本覆盖团队定制。`_bmad/config.user.toml` 是被 Git 忽略的个人安装答案；团队共用语言和配置放在 `_bmad/custom/config.toml`。从仓库打开 Codex 会话可调用 `bmad-help` 查看当前阶段，也可直接委托一个 story、epic 或规划目标。

1. 在起始页新建项目并选择保存 ARXML 的目录，或一次选中同一 ECU 的所有 `.arxml` 文件导入。导入按所选文件集合建模，不自动识别并拆分其他 ECU 的文件。
2. “配置”页选择帧/信号，在右侧检查器修改并应用；如需诊断，在同页设置物理 CAN ID、计时器、DID 与有序 32-bit Tx 信号，可勾选“允许扩展会话写入此 DID”以启用易失 0x2E，再填写可选 RID 启用 0x31/0x01 初值复位例程；应用诊断配置后可选配一个监测 Rx 帧超时的 DTC。启用写入或配置 DTC 后，可勾选“用 0x27 保护状态更改”，并再次应用诊断配置。先保存，再到“诊断”页运行校验。未应用的草稿不会悄悄写入 ARXML。
3. “生成与构建”页选择 `windows-x64-controlled-v1` 或 `linux-x64-controlled-v1`，并选择**独立的空源码目录**。先查看完整工程文件的新增、内容变化和内容不变状态，检查前后文本，再确认生成；源码准备与预览不启动编译器。另选工程之外的新空构建目录，使用显式配置的 CPython/GCC/binutils/Git 构建本机目标。取消预览不写入输出目录；预览后配置、目标、运行源码或旧输出变化时须重新预览。再次生成只接受未改动、具备完整性记录的原源码目录，不覆盖用户文件。
4. 在“虚拟运行”页，对已构建、已配置诊断的当前工程点击“验证诊断连接”，由独立测试器检查本次二进制；配置故障记忆时使用隔离 NvM 文件，启用 Windows 0x27 时另用隔离密钥与安全状态文件。不修改真实 ECU 状态，不需要对端 ECU。信号总线验证另选对端 ECU 的封存源码目录与已构建的实际二进制，运行双 ECU 闭环。

需要交给另一位工程师重建时，在“生成与构建”页另选“导出可重建主机交付包”，先预览再确认。该目录额外包含已保存的 ARXML、`handoff.json` 与完整性记录；接收者在同版工作台显式提供合法 R24-11 XSD 后，从导入页选择“导入可重建主机交付包”，再校验并生成到新目录。普通生成工程仍只交付 C99 源码。固定双 ECU 离线参考包通过 `cargo run --manifest-path core/Cargo.toml --bin package_host_reference -- "<new-output-directory>" --target <target> --xsd-archive "<absolute-licensed-XSD-archive>"` 生成；接收者仅需声明的 CPython 3.12.9、目标 GCC/binutils/Git 和包内工具，无需 checkout、uv、Rust 或 Node，详见 [`runtime/reference-README.md`](runtime/reference-README.md)。生成、预检、构建与实际行为复验是不同结果。

标准 ECU 输入进入“生成与构建”后，使用“预览 ECU 交付”核对实际文件，再“确认生成 ECU”。默认包含显式 `autosar-ecu-handoff-v1` 元数据、原始输入、完整固定源码与许可；旧 `autosar-host-handoff-v1` 仍按原入口读取。新包可整体搬移，通过“重导入 ECU 交接包”重新建立计划和逐字节核对生成结果，再生成到另一空目录。接收方需要同版工作台及合法取得、身份匹配的 R24-11 XSD/MOD；官方原件和编译器不随包分发。

“本机预检”独立实际编译并返回绑定源码身份的 `not_run|passed|failed`；非本机目标为 `not_run`，不标成功。“构建 ECU”使用独立空目录和包内目标工具链，Windows 得到 `ecu_host_batch.exe`，Linux 得到 `ecu_host_batch`。“验证 ECU 主机行为”独立重建并逐字节核对 CAN/DID、至多两个 DID、真实 N_Cr 超时恢复及非法批次。离线入口为 `<CPython3.12.9> tools/ecu-tool.py verify --project <封存源码目录> --build-directory <工程之外的新空目录>`。两平台生产协议已通过原生核心入口实测；Linux 桌面 IPC 与正式 bundle 不由该结果推定。当前工程完整 SC1 复验及实机状态不从这些有界向量推断。

重复生成前，工具核对 `files.list` 的准确文件名、`files.sha256` 中每份生成文件及清单自身的 SHA-256、缺失/额外文件与目录；不匹配即拒绝替换并保留原目录。旧版没有完整性记录的生成目录不能直接覆盖，请选新空目录并保留旧目录。二进制和编译日志只进入工程之外的构建目录；源码包中出现未列二进制、文件或链接时拒绝重建和重导入，不自动清理。

再次生成成功时，原目录**不会被删除**：它会移到同一父目录下独占的新备份位置；返回结果和“生成与构建”页面显示其完整路径。多次生成会积累多份旧工程，由文件所有者确认无用后自行归档或清理，工具不自动回收。生成失败时，未移动的原目录保持原位；若已经移走，则报错中列出可恢复的旧目录路径。失败的临时生成目录也保留并在报错中给出路径，供所有者检查后自行处理。勿将这些备份目录误当成本次构建工程。

构建要求工程之外的新空目录，拒绝覆盖已有二进制、日志或其他所有者文件。编译先写入构建目录内独占的私有目录，再以不覆盖目标的硬链接安装；文件系统不支持硬链接或并发出现同名二进制时直接报错，不降级为覆盖。失败保留实际输出和受管日志；成功只清理工具自己的临时编译产物。

工作台与离线 `tools/ecu-tool.py build` 都在编译前和安装二进制前核对源码清单、SHA-256 与缺失/额外文件，拒绝链接/reparse point；内核补丁只应用到外部构建目录中的私有副本。生成后源码、目标或清单被改动时，工作台拒绝把该目录标为本次配置的结果，请重新生成到空目录。摘要用于完整性检查，不是发布者签名认证。所有实际命令以 argv、绝对工具路径和单调截止时间在 owned scope 中执行，不使用 PowerShell 拼接或 taskkill 清理。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。

Epic4最终独立交接的正式入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_handoff -- --exact`；全部主机等级与旧目标回归使用完整核心测试。独立执行和复核结果记录在BMad 4.22及sprint；任意用户工程的一次生成、构建或包内CAN／DID检查不会自动成为完整SC1或实机验证。

Epic4的22条story已完成，固定`epic4-win64-sr-cs-v1`目标的全部适用主机SC1行为、跨story集成与非实现者交接已通过BMad验证／复核。结论限定CP/FO R24-11、Windows x64、GCC 16.1.0、固定FreeRTOS及声明配置；没有推定任意工程已复验、MCU、硬实时、ASIL、完整MISRA或官方符合性认证。
