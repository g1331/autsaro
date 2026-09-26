# Classic CAN 配置工作台

本地桌面工作台：为一个 ECU 新建或导入 AUTOSAR Classic R24-11 ARXML 配置，编辑受支持的 CAN 帧、信号、一条主机虚拟 DoCAN 物理诊断连接与可选的单个超时故障 DTC，使用官方 XSD 校验，生成独立 C99 工程并构建主机目标。两份不同配置可作为两个独立 ECU 进程接入主机虚拟 CAN 总线；诊断连接由独立测试器驱动。**当前不是全模块 AUTOSAR 实现，也没有真实硬件验证或发布包。**

## 当前支持

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。校验、保存和生成都会复核所选的全部来源文件；任一文件被外部修改时拒绝继续使用旧跨文件配置，并提示重新导入。保存时暂存新内容，在替换前再次复核待写文件；无法安全回滚时保留原备份。
- 编辑配置后，“查看并保存 ARXML”会先列出每份来源文件将修改或保持不变，并显示修改文件的前后差异及完整原文。预览不写磁盘；用户确认后才保存。若预览后配置或来源文件变化，旧预览不能用于保存。
- 标准 11 位 Classical CAN，DLC 1–8；每 ECU 最多 32 帧、64 信号。每帧可映射多个不重叠的 1–32 位无符号 LSB0 小端信号。导入时拒绝与 Com 位段或位序冲突的 I-PDU 映射、重复的 CanIf PDU 映射，以及与 CanIf 不一致的关联 CAN 网络帧 ID 或布局；未解析的配置变体会阻止生成。Tx 帧按虚拟时钟周期发送；Rx 帧按最后有效接收时间判定超时。
- 生成工程包含独立的 Com、PduR、LSduR、CanIf、CanTp、Dcm、可选 Dem/NvM、虚拟 Can、Os 周期调度、Rte 接口及 ECU 配置；目录内的 `README.md` 和 `build.ps1` 给出离线 Windows/GCC 构建及按配置启动方法，`profile.txt` 和 `files.list` 描述生成结果，`Dcm_Externals.h` 随工程交付诊断回调声明。未配置诊断时目标仍是纯信号 ECU。构建后得到 `ecu_host.exe`。生成目录与 ARXML 源目录分开。
- `files.sha256` 保存生成文件与 `files.list` 的逐项内容摘要，供再次生成前核验；该记录不等于对第三方修改的签名认证。
- 可选诊断配置：一对不与信号帧冲突的 11 位物理 CAN ID；S3 至少 5000 ms，N_Bs/N_Cr 为正毫秒；一个扩展会话专属 DID，按配置顺序绑定 1–8 个实时 32-bit Tx Com 信号。支持 0x10 默认/扩展会话、0x3E TesterPresent（含抑制正响应）、0x22 读取 DID；0x22 在默认会话返回 NRC 0x31。可单独启用 0x2E，在扩展会话将完整的大端数据记录写入同一 DID 所绑定的 Tx Com 信号；0x22 与周期 CAN 立即反映新值，ECU 进程重启恢复配置初值。默认会话或 DID 不匹配返回 NRC 0x31，记录长度不符返回 NRC 0x13；未启用时 0x2E 返回 NRC 0x11。该写入仅为易失的主机虚拟应用状态，不写入 NvM/Flash。CanTp 支持单帧、多帧、流控、序号校验、块大小/STmin、N_Bs/N_Cr 超时与错误后恢复，N-SDU 上限 256 字节。配置可修改或移除；重新导入后仍能识别，超出该确定性子集的诊断配置会阻止生成。
- 可选 0x31/0x01 StartRoutine：先启用上述 0x2E，再设置单个 16-bit RID；仅在扩展会话中将该 DID 绑定的 Tx Com 信号恢复为配置初值。未配置时返回 NRC 0x11，默认会话或 RID 不匹配返回 NRC 0x31，StopRoutine/RequestRoutineResults 返回 NRC 0x12，请求长度错误返回 NRC 0x13；不支持选项或状态记录。该例程只修改易失状态，不是安全解锁或持久化操作。
- 可选故障记忆：诊断连接下配置一个 UDS DTC（`0x000100–0xFFFFFE`），绑定一条已有信号且超时为正的 Rx 帧。首次有效接收后超时才报告故障；Dem 状态跨 ECU 进程重启保存在主机文件。0x19/0x01 按状态掩码读取匹配 DTC 数量（当前单 DTC 配置因此返回 0 或 1）；0x19/0x02 按状态掩码在默认或扩展会话读取该 DTC；0x19/0x0A 无需状态掩码，列出配置的 DTC 及当前状态，包括状态为零的监测项；默认/扩展会话均可读，启用安全档案也无需解锁。0x0A 多带参数返回 NRC 0x13，未配置 DTC 返回 NRC 0x11，其他 0x19 子功能（含 0x8A 抑制位）不支持。0x14 仅扩展会话、仅 `0xFFFFFF` 全部清除。NvM 使用两个固定 CRC32 保护槽位、配置指纹并在状态更改确认前同步落盘；已有文件的任一槽位损坏均拒绝启动（`E NVM`），不存在的文件初始化为空状态。每次主机 ECU 进程启动代表新的操作周期，不等价于完整车辆生命周期模型。
- 配置该单 DTC 时，扩展会话还支持 `0x85/0x02` 暂停 DTC 设置、`0x85/0x01` 恢复；暂停期间监测 Rx 帧仍更新信号有效性，但不修改或持久化故障状态，已有 DTC 仍可读取及清除。切回默认会话（含 S3 超时）或 ECU 进程重启会自动恢复记录；不支持选项记录、抑制正响应位或其他子功能。未配置 DTC 时返回 NRC `0x11`，默认会话拒绝此服务。
- 可选主机安全访问：单级 `0x27/0x01` 请求 16 字节随机 seed、`0x27/0x02` 提交 16 字节 key；扩展会话内解锁。启用后，`0x2E`、`0x31/0x01`、`0x14`、`0x85` 在已满足原有会话与参数条件时，未解锁返回 NRC `0x33`。三次错误 key 后延迟 5 秒；失败计数跨主机进程重启保存在独立状态文件，状态损坏则拒绝启动。密钥是运行时提供的独立 32 字节文件，不进入 ARXML 或生成工程；该文件与状态文件不等于硬件安全存储，也不提供量产级认证保证。详见 [`runtime/README.md`](runtime/README.md)。
- 主机信号闭环检查两个 ECU 的独立信号值与位向量、CAN ID 优先顺序、丢帧后的接收超时、BUS_OFF/STARTED 恢复以及错误 DLC 拒绝。独立诊断测试器对生成的 ECU 注入物理 CAN 报文，检查会话、实时 DID 多帧载荷、流控、N_Bs/N_Cr 超时、错误序号、后续请求恢复与 S3 回退；启用写入时还检查 0x2E 多帧请求、0x22/CAN 信号更新、会话限制与重启后恢复初值；配置例程时检查 0x31/0x01 恢复后 0x22 和 Com 信号的初值以及 RID、子功能、长度和 S3 限制；配置 DTC 时另用隔离存储验证 Rx 超时后 0x19/0x01 的匹配/不匹配/清零数量及 0x19/0x02 单 DTC 报告、0x19/0x0A 在无故障/暂停记录/超时/重启/清除后的支持列表和错误请求拒绝、0x85 禁用/恢复记录、跨进程保持、会话权限清除与损坏拒绝。两种运行结果分别呈现。虚拟总线不模拟电气层或位级仲裁。

新建工程的 EcuC PDU 配置按 R24-11 MOD 建立一个虚拟核心与全局 `EcucPduCollection/Pdu`；配置 Com 帧时还提供必需的 `ComGeneral`。`ComPduIdRef` 与 CanIf Tx/Rx PDU 引用指向独立的全局 Pdu 容器；系统 `I-SIGNAL-I-PDU`、`N-PDU`、`DCM-I-PDU` 保留各自语义，全局 Pdu 到系统 PDU 的关系由版本化**工具专属 SDG** 记录。固定主机剖面现在还生成虚拟 Mcu 时钟、Can 控制器及 Rx/Tx 硬件对象、CanIf 驱动/HOH/缓冲和必需的 PDU 参数与引用；导入时按同一剖面检查，不符合者只读阻断。这些虚拟时钟、基地址和位时序值不表示真实 MCU 配置，运行时代码也不是第三方 CanIf/Can 标准 ABI；尚无第三方协议栈或硬件互操作证据。诊断的 Dcm/CanTp/N-PDU 路径仍仅对本主机目标验证；可选写入使用 DcmDspDidWrite 与逐数据回调。主机专属的 0x31/0x01 例程把 RID 和固定扩展会话记录在 DID 工具 SDG 中，**不**输出 DcmDsd 0x31 服务或 DcmDspRoutine/StartRoutine/CommonAuthorization；第三方 Dcm 不能依据该 ARXML 获得此例程。DcmDspData 与 Com 信号、Dem 事件与监测 Rx 帧的绑定也由工具 SDG 记录。可选 0x27 的 ECUC 行仅描述主机目标的固定单级档案；所列主机回调并非第三方 Dcm ABI 的互操作证明。主机 NvM 不建模真实 Ea/Fee/MemIf 物理目标或分区引用，也未建模 ComM/EcuM/BswM/CanSM；未实现其他例程/写入 DID/持久写入、完整诊断子功能、功能寻址或完整 ISO 14229/15765 一致性。无真实硬件或标准符合性证明；不支持 29 位 CAN、签名/大端信号、真实芯片驱动与未解析配置变体。界面“通过”仅对应所运行的主机路径，详见 [`runtime/README.md`](runtime/README.md)。

诊断配置中的六项 CanTp N-SDU、N-PDU/FC N-PDU 引用及两项 `DcmDslProtocolRx/TxPduRef` 也按 MOD 指向相应 EcuC 全局 Pdu，`VALUE-REF DEST="ECUC-CONTAINER-VALUE"`；系统 N-PDU/DCM-I-PDU 仅保留系统描述与工具绑定。按 System Template `[constr_3448]`，这些系统 PDU 对应的全局容器和 Com 的 I-SIGNAL-I-PDU 全局容器均**不写** `DynamicLength`，系统 N-PDU 也不写 `HAS-DYNAMIC-LENGTH`：N-PDU 长度由 TP 处理，DcmIPdu 的动态长度由系统模板语义决定，不从 EcuC 布尔值推断。含此不适用字段的导入配置只读阻断，不自动改写。

生成的主机工程对每个 DID 数据提供可外部链接的 `Ecu_DcmRead_<index>`，启用 0x2E 时还提供 `Ecu_DcmWrite_<index>`；主机 Dcm 实际通过这些回调读写 Com 信号，而不只在 ARXML 中填写函数名。`Dcm_Externals.h` 声明这些回调，`include/Ecu_DcmCallbackTypes.h` 仅定义本主机剖面所需的 `Std_ReturnType` 等类型，不是完整 AUTOSAR `Std_Types.h` 或生成的 RTE 类型头，也不证明第三方 Dcm 可直接接入。0x31 仍由生成工程的内部 `Ecu_HostRestoreDid` 实现；其 R24-11 ECUC 例程函数签名参数属草案，本产品不声明该例程的标准 ECUC 集成。

工作区只按当前 R24-11 配置模型解析和保存：Com/CanIf/CanTp/Dcm 的 ECUC PDU 引用须指向 EcuC 全局 Pdu，主机 CAN 配置须包含上述固定 Mcu/Can/CanIf 闭包，主机专属 0x31 例程只使用 DID 工具 SDG；单 DTC 配置还须包含 DcmDsd 0x85 服务、`DcmDspControlDTCSetting` 禁用选项记录及指向 DemClient 的 `DcmDemClientRef`。没有旧工具格式的自动迁移、回退或保存确认流程；缺少当前必需结构的来源文件可查看诊断，但不自动改写，不能保存或生成。未受支持却不影响有效配置的内容仍作为保留项原样保留。

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
- ECUC 配置闭包测试：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`。测试以其中的模块定义核对必需参数、重数和引用目标。

```powershell
npm ci --prefix ui
npm run build --prefix ui
cargo test --manifest-path core/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

调试桌面程序时，先在一个终端运行 `npm run dev --prefix ui`（端口 `127.0.0.1:1420`），另一个终端运行 `src-tauri/target/debug/autosar-config-desktop.exe`。当前验证的是源码目录中的本地调试程序；生成器从仓库 `runtime/` 复制目标源码，桌面程序从上述本地路径取得 XSD。没有可脱离源码目录使用的安装包。

## 使用顺序

1. 在起始页新建项目并选择保存 ARXML 的目录，或一次选中同一 ECU 的所有 `.arxml` 文件导入。导入按所选文件集合建模，不自动识别并拆分其他 ECU 的文件。
2. “配置”页选择帧/信号，在右侧检查器修改并应用；如需诊断，在同页设置物理 CAN ID、计时器、DID 与有序 32-bit Tx 信号，可勾选“允许扩展会话写入此 DID”以启用易失 0x2E，再填写可选 RID 启用 0x31/0x01 初值复位例程；应用诊断配置后可选配一个监测 Rx 帧超时的 DTC。启用写入或配置 DTC 后，可勾选“用 0x27 保护状态更改”，并再次应用诊断配置。先保存，再到“诊断”页运行校验。未应用的草稿不会悄悄写入 ARXML。
3. “生成与构建”页首次选择**独立的空目录**，先查看完整工程文件的新增、内容变化和内容不变状态，检查变化文件的前后文本，再确认生成并构建主机目标。取消预览不写入输出目录；预览后配置、运行源码或旧输出变化时须重新预览。生成前须无阻断错误。再次生成可使用未改动、具备完整性记录的原输出目录（配置项目允许变化）；输出目录不得覆盖其他文件。
4. 在“虚拟运行”页，对已配置诊断的当前工程点击“验证诊断连接”，由独立测试器检查生成的 ECU；配置故障记忆时，测试器使用隔离 NvM 文件跨进程验证故障读出与清除；启用 0x27 时，另用隔离密钥与安全状态文件验证解锁、拒绝、延时和重启，不修改真实 ECU 状态。不需要对端 ECU。若要验证信号总线，则为另一个 ECU 配置互补的 Tx/Rx CAN ID 与 DLC，选择其已构建工程目录，另行运行双 ECU 闭环。

重复生成前，工具会核对 `files.list` 的准确文件名、`files.sha256` 中每份生成文件及清单自身的 SHA-256、缺失/额外文件与目录；不匹配即拒绝替换并保留原目录。旧版没有完整性记录的生成目录不能直接覆盖，请选新的空目录并自行保留旧目录。构建得到的 `ecu_host.exe`/`ecu_host` 不属于生成源码清单；存在该二进制时也拒绝重复生成。请优先选择新的空输出目录；只有文件所有者明确决定并将旧产物移走后，才可重用原目录，工具不会自动删除二进制。

再次生成成功时，原目录**不会被删除**：它会移到同一父目录下独占的新备份位置；返回结果和“生成与构建”页面显示其完整路径。多次生成会积累多份旧工程，由文件所有者确认无用后自行归档或清理，工具不自动回收。生成失败时，未移动的原目录保持原位；若已经移走，则报错中列出可恢复的旧目录路径。失败的临时生成目录也保留并在报错中给出路径，供所有者检查后自行处理。勿将这些备份目录误当成本次构建工程。

构建也拒绝覆盖已有的 `ecu_host.exe`/`ecu_host`：请先选择新的空输出目录生成与构建，或由文件所有者明确移走原二进制。编译先写入同级独占临时目录，再以不覆盖目标的硬链接安装；文件系统不支持硬链接或并发出现同名二进制时直接报错，不降级为覆盖。失败时临时产物路径随错误返回，成功后只清理工具自己的临时产物；清理失败会在构建日志中给出路径。

工作台构建会在编译前和安装二进制前核对生成目录的 `files.list`、`files.sha256`、所列文件内容以及额外文件。生成后外部改动了源码、清单或目录内容时，工作台拒绝把该目录标为本次配置的构建结果；请重新生成到空目录。此核对用于发现误改，摘要记录不是签名认证；离线交付目录中的 `build.ps1` 由接收者自行运行，不执行工作台的这项核对。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。
