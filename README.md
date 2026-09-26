# Classic CAN 配置工作台

本地桌面工作台：为一个 ECU 新建或导入 AUTOSAR Classic R24-11 ARXML 配置，编辑受支持的 CAN 帧、信号、一条主机虚拟 DoCAN 物理诊断连接与可选的单个超时故障 DTC，使用官方 XSD 校验，生成独立 C99 工程并构建主机目标。两份不同配置可作为两个独立 ECU 进程接入主机虚拟 CAN 总线；诊断连接由独立测试器驱动。**当前不是全模块 AUTOSAR 实现，也没有真实硬件验证或发布包。**

## 当前支持

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。保存时暂存新内容，并在替换前复核磁盘文件；检测到外部修改则拒绝覆盖，无法安全回滚时保留原备份。
- 标准 11 位 Classical CAN，DLC 1–8；每 ECU 最多 32 帧、64 信号。每帧可映射多个不重叠的 1–32 位无符号 LSB0 小端信号。导入时拒绝与 Com 位段或位序冲突的 I-PDU 映射、重复的 CanIf PDU 映射，以及与 CanIf 不一致的关联 CAN 网络帧 ID 或布局；未解析的配置变体会阻止生成。Tx 帧按虚拟时钟周期发送；Rx 帧按最后有效接收时间判定超时。
- 生成工程包含独立的 Com、PduR、LSduR、CanIf、CanTp、Dcm、可选 Dem/NvM、虚拟 Can、Os 周期调度、Rte 接口及 ECU 配置；`profile.txt` 和 `files.list` 描述生成结果，`Dcm_Externals.h` 随工程交付诊断回调声明。未配置诊断时目标仍是纯信号 ECU。构建后得到 `ecu_host.exe`。生成目录与 ARXML 源目录分开。
- `files.sha256` 保存生成文件与 `files.list` 的逐项内容摘要，供再次生成前核验；该记录不等于对第三方修改的签名认证。
- 可选诊断配置：一对不与信号帧冲突的 11 位物理 CAN ID；S3 至少 5000 ms，N_Bs/N_Cr 为正毫秒；一个扩展会话专属 DID，按配置顺序绑定 1–8 个实时 32-bit Tx Com 信号。支持 0x10 默认/扩展会话、0x3E TesterPresent（含抑制正响应）、0x22 读取 DID；0x22 在默认会话返回 NRC 0x31。可单独启用 0x2E，在扩展会话将完整的大端数据记录写入同一 DID 所绑定的 Tx Com 信号；0x22 与周期 CAN 立即反映新值，ECU 进程重启恢复配置初值。默认会话或 DID 不匹配返回 NRC 0x31，记录长度不符返回 NRC 0x13；未启用时 0x2E 返回 NRC 0x11。该写入仅为易失的主机虚拟应用状态，不写入 NvM/Flash，也不提供 0x27 安全解锁。CanTp 支持单帧、多帧、流控、序号校验、块大小/STmin、N_Bs/N_Cr 超时与错误后恢复，N-SDU 上限 256 字节。配置可修改或移除；重新导入后仍能识别，超出该确定性子集的诊断配置会阻止生成。
- 可选 0x31/0x01 StartRoutine：先启用上述 0x2E，再设置单个 16-bit RID；仅在扩展会话中将该 DID 绑定的 Tx Com 信号恢复为配置初值。未配置时返回 NRC 0x11，默认会话或 RID 不匹配返回 NRC 0x31，StopRoutine/RequestRoutineResults 返回 NRC 0x12，请求长度错误返回 NRC 0x13；不支持选项或状态记录。该例程只修改易失状态，不是安全解锁或持久化操作。
- 可选故障记忆：诊断连接下配置一个 UDS DTC（`0x000100–0xFFFFFE`），绑定一条已有信号且超时为正的 Rx 帧。首次有效接收后超时才报告故障；Dem 状态跨 ECU 进程重启保存在主机文件。0x19/0x01 按状态掩码读取匹配 DTC 数量（当前单 DTC 配置因此返回 0 或 1）；0x19/0x02 按状态掩码在默认或扩展会话读取该 DTC；其他 0x19 子功能不支持。0x14 仅扩展会话、仅 `0xFFFFFF` 全部清除。NvM 使用两个固定 CRC32 保护槽位、配置指纹并在状态更改确认前同步落盘；已有文件的任一槽位损坏均拒绝启动（`E NVM`），不存在的文件初始化为空状态。每次主机 ECU 进程启动代表新的操作周期，不等价于完整车辆生命周期模型。
- 主机信号闭环检查两个 ECU 的独立信号值与位向量、CAN ID 优先顺序、丢帧后的接收超时、BUS_OFF/STARTED 恢复以及错误 DLC 拒绝。独立诊断测试器对生成的 ECU 注入物理 CAN 报文，检查会话、实时 DID 多帧载荷、流控、N_Bs/N_Cr 超时、错误序号、后续请求恢复与 S3 回退；启用写入时还检查 0x2E 多帧请求、0x22/CAN 信号更新、会话限制与重启后恢复初值；配置例程时检查 0x31/0x01 恢复后 0x22 和 Com 信号的初值以及 RID、子功能、长度和 S3 限制；配置 DTC 时另用隔离存储验证 Rx 超时后 0x19/0x01 的匹配/不匹配/清零数量及 0x19/0x02 单 DTC 报告、跨进程保持、会话权限清除与损坏拒绝。两种运行结果分别呈现。虚拟总线不模拟电气层或位级仲裁。

新建工程的 EcuC PDU 配置按 R24-11 MOD 建立一个虚拟核心与全局 `EcucPduCollection/Pdu`；配置 Com 帧时还提供必需的 `ComGeneral`。`ComPduIdRef` 与现有 CanIf Tx/Rx PDU 引用指向独立的全局 Pdu 容器；系统 `I-SIGNAL-I-PDU`、`N-PDU`、`DCM-I-PDU` 保留其各自语义，全局 Pdu 到系统 PDU 的关系由版本化**工具专属 SDG** 明确记录，不冒充标准直接引用。**这只修复 Com/EcuC PDU 子集，不等于完整 CanIf/CAN ECUC**：CanIf 仍缺少必需的控制器/驱动、HOH/缓冲和完整根/PDU 参数，也没有真实 CAN 硬件，因此不能交付第三方 CanIf/Can 驱动互操作。诊断的 Dcm/CanTp/N-PDU 路径仍仅对本主机目标验证；可选写入使用 DcmDspDidWrite 与逐数据回调。主机专属的 0x31/0x01 例程把 RID 和固定扩展会话记录在 DID 工具 SDG 中，**不**输出 DcmDsd 0x31 服务或 DcmDspRoutine/StartRoutine/CommonAuthorization；第三方 Dcm 不能依据该 ARXML 获得此例程。DcmDspData 与 Com 信号、Dem 事件与监测 Rx 帧的绑定也由工具 SDG 记录。主机 NvM 不建模真实 Ea/Fee/MemIf 物理目标或分区引用，也未建模 ComM/EcuM/BswM/CanSM；未实现 0x27 安全、其他例程/写入 DID/持久写入、完整诊断子功能、功能寻址或完整 ISO 14229/15765 一致性。无真实硬件或标准符合性证明；不支持 29 位 CAN、签名/大端信号、真实芯片驱动与未解析配置变体。界面“通过”仅对应所运行的主机路径，详见 [`runtime/README.md`](runtime/README.md)。

诊断配置中的六项 CanTp N-SDU、N-PDU/FC N-PDU 引用及两项 `DcmDslProtocolRx/TxPduRef` 也按 MOD 指向相应 EcuC 全局 Pdu，`VALUE-REF DEST="ECUC-CONTAINER-VALUE"`；系统 N-PDU/DCM-I-PDU 仅保留系统描述与工具绑定。按 System Template `[constr_3448]`，这些系统 PDU 对应的全局容器和 Com 的 I-SIGNAL-I-PDU 全局容器均**不写** `DynamicLength`，系统 N-PDU 也不写 `HAS-DYNAMIC-LENGTH`：N-PDU 长度由 TP 处理，DcmIPdu 的动态长度由系统模板语义决定，不从 EcuC 布尔值推断。含此不适用字段的导入配置只读阻断，不自动改写。

生成的主机工程对每个 DID 数据提供可外部链接的 `Ecu_DcmRead_<index>`，启用 0x2E 时还提供 `Ecu_DcmWrite_<index>`；主机 Dcm 实际通过这些回调读写 Com 信号，而不只在 ARXML 中填写函数名。`Dcm_Externals.h` 声明这些回调，`include/Ecu_DcmCallbackTypes.h` 仅定义本主机剖面所需的 `Std_ReturnType` 等类型，不是完整 AUTOSAR `Std_Types.h` 或生成的 RTE 类型头，也不证明第三方 Dcm 可直接接入。0x31 仍由生成工程的内部 `Ecu_HostRestoreDid` 实现；其 R24-11 ECUC 例程函数签名参数属草案，本产品不声明该例程的标准 ECUC 集成。

旧工具保存的 Com/CanIf/CanTp/Dcm 引用直接指向系统 I-PDU/N-PDU/DCM-I-PDU。对**完整识别的本工具旧形状**，导入只在内存暂存迁移：按已验证的引用关系添加 EcuC 全局 Pdu、ComGeneral/ComIPdu 必需字段并重定向 ECUC 引用，保留系统 PDU 与不相关的源文件/注释；含旧 0x31 例程时须同时能够安全转成 DID 工具记录。工作区显示“未保存/待确认”，磁盘原文件不变，确认前禁止编辑与生成，保存须明确确认且复用外部修改检测和事务回滚。未知扩展、外部依赖、用户已有 EcuC 模块、未解析变体或任一例程步骤不安全时，整个旧工程保持原样只读并定位问题，不猜测短名或覆盖来源。已暂存的转换仍**不**补齐 CanIf/CAN 控制器、HOH、缓冲等必需配置。

受支持的 ComIPdu/ComSignal 必须直接归属本工程唯一的 `/{项目名}/ComCfg/ComConfig`，其模块定义和 ComGeneral 必须正确；重命名模块、把子容器挂到其他模块或改动父级 `DEFINITION-REF` 即使保留了可解析的 Pdu/Signal 引用，也只读阻断，不按子容器局部定义猜测所有者。

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
2. “配置”页选择帧/信号，在右侧检查器修改并应用；如需诊断，在同页设置物理 CAN ID、计时器、DID 与有序 32-bit Tx 信号，可勾选“允许扩展会话写入此 DID”以启用易失 0x2E，再填写可选 RID 启用 0x31/0x01 初值复位例程；应用诊断配置后可选配一个监测 Rx 帧超时的 DTC。先保存，再到“诊断”页运行校验。未应用的草稿不会悄悄写入 ARXML。
3. “生成与构建”页首次选择**独立的空目录**生成工程，再构建主机目标；生成前须无阻断错误。再次生成可使用未改动、具备完整性记录的原输出目录（配置项目允许变化）；输出目录不得覆盖其他文件。
4. 在“虚拟运行”页，对已配置诊断的当前工程点击“验证诊断连接”，由独立测试器检查生成的 ECU；配置故障记忆时，测试器使用隔离 NvM 文件跨进程验证故障读出与清除，不修改真实 ECU 状态。不需要对端 ECU。若要验证信号总线，则为另一个 ECU 配置互补的 Tx/Rx CAN ID 与 DLC，选择其已构建工程目录，另行运行双 ECU 闭环。

重复生成前，工具会核对 `files.list` 的准确文件名、`files.sha256` 中每份生成文件及清单自身的 SHA-256、缺失/额外文件与目录；不匹配即拒绝替换并保留原目录。旧版没有完整性记录的生成目录不能直接覆盖，请选新的空目录并自行保留旧目录。构建得到的 `ecu_host.exe`/`ecu_host` 不属于生成源码清单；存在该二进制时也拒绝重复生成。请优先选择新的空输出目录；只有文件所有者明确决定并将旧产物移走后，才可重用原目录，工具不会自动删除二进制。

再次生成成功时，原目录**不会被删除**：它会移到同一父目录下独占的新备份位置；返回结果和“生成与构建”页面显示其完整路径。多次生成会积累多份旧工程，由文件所有者确认无用后自行归档或清理，工具不自动回收。生成失败时，未移动的原目录保持原位；若已经移走，则报错中列出可恢复的旧目录路径。失败的临时生成目录也保留并在报错中给出路径，供所有者检查后自行处理。勿将这些备份目录误当成本次构建工程。

构建也拒绝覆盖已有的 `ecu_host.exe`/`ecu_host`：请先选择新的空输出目录生成与构建，或由文件所有者明确移走原二进制。编译先写入同级独占临时目录，再以不覆盖目标的硬链接安装；文件系统不支持硬链接或并发出现同名二进制时直接报错，不降级为覆盖。失败时临时产物路径随错误返回，成功后只清理工具自己的临时产物；清理失败会在构建日志中给出路径。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。
