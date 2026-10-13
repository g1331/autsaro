# 标准与规范基线

本文记录项目固定的标准版次、官方资料在本地的位置、测试预期如何从标准独立定义，以及已核定的模块义务索引。它是查找依据的入口，不是符合性报告：列出的条款表示已定位并用于设计或验收，**不表示所有适用条款已逐条核完**，也不表示实现已经符合。架构与所有权约束见 [architecture.md](architecture.md)，决定理由见 [ADR 0002](adr/0002-fixed-standard-baseline-r24-11.md)、[ADR 0008](adr/0008-misra-baseline-and-no-deviation.md)。

## 1. 固定基线

| 范围 | 基线 | 说明 |
| --- | --- | --- |
| AUTOSAR | **CP／FO R24-11** | 文件身份以官方 [CP](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256)／[FO](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256) `TR_SpecificationHashes.sha256` 为准；Schema 为 `AUTOSAR_00053.xsd`。R25-11 只作后续升级参考，不混用 |
| ECUC 定义 | `AUTOSAR_CP_MOD_ECUConfigurationParameters.zip` | SHA-256 `df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`（`core/resources/official.json`） |
| XML Schema | `AUTOSAR_FO_MMOD_XMLSchema.zip` | SHA-256 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`（同上） |
| C 语言 | **C99** | MISRA 修订支持 C11／C18 不构成升级编译标准的授权 |
| 编码规范 | **MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2** | 200 Rules＋21 Directives＝221 项；当前无已批准偏离 |
| OSEK OS | OSEK/VDX OS 2.2.3 | R24-11 SWS_OS §7.1 引用的 OSEK 基础义务须一并纳入。已核对 NXP 托管副本（SHA-256 `2ec77d3c4259797f4d0190f938131b641d415d8e0a85ce40a72e4201a9114746`）；ISO 17356-3 全文与后续勘误未核对 |

R24-11 中标记为 DRAFT 的条款（例如 LSduR 全文，`SWS_Com_91019／91020`、`SWS_Rte_91123／91127`、`SWS_COMTYPE_91001`、CanIf `91002`）按固定版本原文实现并如实标注，不宣称为成熟标准条款。

## 2. 官方资料与本地位置

- 官方 PDF、ZIP 与哈希清单只在本地查阅，由 `.gitignore` 排除，不随仓库或产品再分发。目录与核验结果见 [docs/official/README.md](official/README.md) 与 [docs/official/R24-11/README.md](official/R24-11/README.md)。
- 下载与核验：`python scripts/collect_official_specs.py R24-11`，按两份官方清单逐项校验，结果写入 `download-status.json`。校验失败或官网缺项不算已收集。
- 开发对照档案的路径与环境变量登记在 `core/resources/official.json`：XSD／MOD（`AUTOSAR_XSD_ARCHIVE`、`AUTOSAR_MOD_ARCHIVE`）供 `official-oracles` 测试检验内置规则；`AUTOSAR_CP_MOD_SpecificationsARXML.zip`（`AUTOSAR_SPECIFICATIONS_ARCHIVE`）是 §5 条款核算的条目来源。普通配置与源码生成不需要这些档案。
- 官方单份文件的统一入口为 `https://www.autosar.org/fileadmin/standards/R24-11/CP/<文件名>`。
- MISRA 正文需授权取得，不在仓库中。编码指导与 221 项编号目录见项目技能 `.agents/skills/misra-c2012/`（`references/baseline-index.json`）；它是编码辅助，不替代正文或人工评估。
- 合法正文缺失时，只阻塞依赖该正文的标准裁决；记录缺口与解除条件，不凭记忆、旧版本或现有实现补齐。

## 3. 预期从标准独立定义

1. 测试预期从固定版次的官方原文与独立向量定义；不从现有源码、ABI 或资产清单或生成器输出自动生成唯一预期。
2. 对标准的核对与对实现内部一致性的检查分开记录；后者不能证明前者。
3. 公开接口用独立 C consumer 编译、链接并调用固定函数指针，覆盖成功、拒绝、关键状态变化与失败后输出保护；不以函数桩、源码文本断言或纯编译代替行为。
4. 对外使用标准名称的接口须满足标准签名与语义；改名、typedef 或空壳接口不算对齐。
5. 主机行为、XSD 校验、MISRA 检查都不能代替 AUTOSAR 模块符合性；ECUC metadata 数值范围通过也不等于配置语义合法。
6. 修改或扩展 BSW、RTE、OS、配置模型和 C 生成逻辑前，核对所涉模块在所选配置下的公开接口、类型、配置、状态机、错误与回调义务及上下层接缝；超出当前范围或受目标条件阻塞的差距写明依据、影响与退出条件，不只写“主机专属”。
7. MISRA：工具可用、扫描完整、源码诊断与人工评估分别证明；原始诊断保留，采用代码不自动排除；申请偏离前核对适用的 amendment／TC 与实际调用路径，偏离只能由维护者批准。

## 4. 证据分层

以下结论各自需要独立证据，不能由另一项推定：主机接口与逻辑行为；Windows 与 Linux 各自的执行；原生 Tauri／IPC 与安装包；离线接收者复验；MCU 实时性、真实 ISR 与内存段放置；第三方模块互操作；完整规范义务；MISRA 符合性；功能安全与认证。未运行记为未执行，失败记为失败。

## 5. 模块义务索引

本节是条款处置在文档一侧的记录（[ADR 0017](adr/0017-requirement-disposition-in-tests-and-standards.md)）。模块整改时，该模块文档在 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 中的全部条目（含 `ECUC_`、`CONSTR`、`NA` 前缀）都须处置。只有紧跟显式标记、且写在对应来源中的编号才计入处置。测试在注释中写 `Verifies: <编号>`。本节对应模块行写以下几种：`已核对（<核对方式>）：` 记录审阅确认的条目；`不适用（配置：<ECUC 参数编号>[=<值>]）：` 记录产品不支持的配置下才适用的条目，理由中的每个参数都须有带 `Rejects: <ECUC 参数编号>` 注释的拒绝测试；`不适用（归属：<承担者>）：` 记录属于其他模块、集成方或配置工具的条目；`差距 #<Issue>：` 记录差距。分组处置可写成表格，但每个编号单元格须以标记开头。核算用下面的命令现算，结果不入库：

```powershell
uv run --locked python -m autosar_tooling requirements SWS_CANInterface
```

参数为文档名（`AUTOSAR_CP_<文档名>.pdf`）。命令输出各类别的条目数，并列出未处置条目（标注 draft／obsolete）、缺少拒绝测试的配置条件与失效引用。失效引用指带有该文档的前缀、但 R24-11 中没有的编号；未加标记的编号也会检查。三者任一非空即失败；`--stale-only` 只检查失效引用。一个标记后可列多个完整编号，用 `、`、`,`、`／` 或空格分隔，编号可加反引号；同前缀的后续编号可用 `／`、`,` 续写，或用 `–` 表示范围，如 `不适用（<理由>）：SWS_<Mod>_00001／00003、SWS_<Mod>_00907–00910`（示例中的 `<Mod>` 不构成引用）。

尚未按此整改的行沿用原核定记录：模块上下文中的简写（如“Dcm 00806”“RTE 05088”）指该模块 SWS 的同号条款，不计入处置，其中的完整编号也只是定位记录，整改时改为带标记的完整编号；`ECUC_*`、`CONSTR`／`constr_*`、`TPS_*` 等前缀保持原样。SHA-256 为 2026-10-09 实际取得、与 CP 官方清单匹配的文件身份。

### 5.1 方法论、输入角色与模板

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_TR_Methodology.pdf` | §2.7.2.2.1–2.7.2.2.2（p102–104）；`TR_METH_01114/01116/01117` | ECU 配置输入包含 ECU Extract 与所选 BSW 实现的模块交付信息；每个模块须选择实现 |
| `AUTOSAR_CP_TPS_SystemTemplate.pdf`（`5267a329104c2dcb029b3f713afa376733466a1b225a33498036a2f1ef017777`） | `ECU_EXTRACT`（§14）、`TPS_SYST_01006`；`TPS_SYST_01001`、`constr_5486／5487`；`[constr_3448]` | Extract 是面向目标 ECU 的展开系统描述；SWC→ECU 映射；DataMapping 与 SenderReceiverToSignalMapping 的实例／端口／信号身份；本剖面全局 Pdu 不输出 `DynamicLength` |
| `AUTOSAR_CP_TPS_SoftwareComponentTemplate.pdf`（`ea86a1b4b43c4ec0319b706e8696412ffb920cb6cc6422a911bedc1f0c9d59da`） | §6.3／6.4、`constr_1068／1069` | 变量与 assembly 兼容性、端口归属、runnable／event／access 与同步 operation 关系；组件接口不能从 Com 信号名推断 |
| `AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf`（`786ccab650f516f541e8d29b41f5c00e57ccd3de6ac1cad70db268f3ec7c5c28`） | 表 4.1／4.17；`04016／04017`；`constr4019`、`4020`；`constr_10260–10264`；TPS 5.11 | BSW entry／schedulable entity／exclusive area 与真实符号、task 上下文匹配；implementedEntry／expectedEntry 属于 ModuleDescription；entry 须显式声明 callType、executionContext、reentrant、synchronous、swServiceImplPolicy |
| `AUTOSAR_CP_TPS_ECUConfiguration.pdf` 与 MOD ZIP | ECUC 定义／值、重数、引用、配置类 | 校验 XSD 之外的引用、值与实现约束；厂商差异定义与标准来源分开 |
| `AUTOSAR_CP_SWS_BSWGeneral.pdf`（`4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b`） | `SWS_BSW_00006`、`SWS_BSW_00115`、`[SWS_BSW_00103]`、`[SWS_BSW_00170]`／`[SWS_BSW_00171]`、`SWS_BSW_00036`；表 13.2 模块 ID | 实现源包含 `<Mip>_MemMap.h`；文件名区分大小写且不能只靠大小写区分；跨模块头文件版本检查；模块 ID：Can 80、CanIf 60、CanTp 35、Com 50、ComM 12、Dcm 53、PduR 51、Rte 2、Os 1、BswM 42、CDD 255，LSduR 132 |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |
| `AUTOSAR_CP_SWS_MemoryMapping.pdf`（`9dc38d360e4a154f627ede71cc5207f7be193c55cbb9f87875e2731af14c891c`） | `00020／00028／00029／00032`（`[SWS_MemMap_00032]`／`[SWS_MemMap_00029]`） | 每个实际模块与每个 SWC type 的 MemMap 由 Memory Mapping 按输入配置提供；主机段映射不代表 MCU 段放置 |

### 5.2 RTE 与 SchM

文档：`AUTOSAR_CP_SWS_RTE.pdf`（`d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）。

| 义务区域 | 条款 |
| --- | --- |
| 契约阶段与生成阶段 | §3.1（p92 起）、§3.2 |
| 实例约束 | §4.1.4.2–4.1.4.4、`SWS_Rte_02008／02009` |
| 任务映射与位置 | §4.2.3、`ECUC_Rte_09023`；TimingEvent 绑定 `ECUC_Rte_09024`（Alarm）／`ECUC_Rte_09026`（ExpiryPoint）、`09025`（OsEvent 引用）；`CONSTR08936`／`08938`（未显式启用映射的 application／BSW 事件不得持有 task ref） |
| 标准 Read／Call 与服务调用 | §5.6.10／5.6.13；`SWS_Rte_06019`、`04515`；`SWS_Rte_07027`（USE-VOID）、`CONSTR1286`（primitive IN 限制） |
| 接收状态与初值 | `SWS_Rte_07804／07805`；§4.3.1.7／`SWS_Rte_08104`（p317，`handleTimeoutType=none` 保留）；`SWS_Rte_04501／04502`（P／R 初值冲突时 R 优先）；`06009／06010`；`SWS_Rte_07381–07383`（本地 `handleNeverReceived=true`，未实现） |
| COM 状态与通知 | `SWS_Rte_06830／07822`（仅 `COM_SERVICE_NOT_AVAILABLE` 映射 `COM_STOPPED`）；`SWS_Rte_08061／08062／08103／08104`（timeout 通知）；`SWS_Rte_91123／91127`（DRAFT，`Rte_COMCbk`／`Rte_COMCbkRxTOut`） |
| 服务器形状 | §5.7.5.6／`SWS_Rte_08913`（无 possibleErrors 的 server entry 返回 void） |
| 生命周期 | `Rte91137／02569／01309`、`91138／02570／01310`（Start／Stop）；`09035–09038` |
| 内存段 | `05088`、`07422／07424`、`04615／07710`（draft）、`07426／07427`、`07592`、`05089／05090` |
| BSWMD 与依赖 | `08404`、`05177`、`05179／05180` |
| Mode | `07255／07260`、`07256`／`constr09049`、`07261` |
| SchM | §6.5；`91170–91173／07271／04547／04550／07275`（Init／Start／StartTiming／Deinit）；`09055–09057`；`06750／06755` |

### 5.3 OS

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3。目标等级为单核 SC1，须覆盖等级能力与最小容量，不缩减为参考配置实际使用的对象。

| 义务族 | 条款 | 要点 |
| --- | --- | --- |
| 一致性类与容量 | OSEK §3.2（p13–14）；`SWS_Os_00241`、Table 7.4（p129–131） | BCC1／BCC2／ECC1／ECC2；SC1 至少两个调度表、八个软件 Counter |
| 激活顺序与抢占恢复 | OSEK §4.3／4.5（p19–20） | 全部激活请求按优先级内请求顺序记录，不只记数量 |
| 终止／链式／错误返回 | OSEK §4.7、§13.2.3.2–3（p23／51–52）；R24-11 任务缺失结束条款（p74–75） | 合法调用不返回原应用帧；错误调用返回并保留源实例与目标状态 |
| 非抢占与内部资源 | OSEK §4.6、§8.7 | 逐任务属性、内部 ceiling 的释放／重新取得 |
| 事件 | OSEK §7、§13.5（p26–28／60–62） | 自有 mask；检查与进入 WAITING、Set 与 ready 转换同一原子域 |
| 资源 | OSEK §8；R24-11 §7.9.21（p105）；`SWS_Os_00850`（RES_SCHEDULER） | 固定 ceiling／LIFO；不能以 mutex 继承代替 |
| Counter／Alarm／ScheduleTable | R24-11 §7.3、§7.5–7.7、§8／10；`SWS_Os_00560` | 回绕、零偏移、同 tick 动作顺序、相对／绝对起点 |
| 栈 | §7.5；`SWS_Os_00067／00068／00396` | Task／Cat2 ISR 栈故障检测；无 ProtectionHook 时调用 ShutdownOS |
| 生命周期、ISR、错误、Hook | §7.1／8／10；OSEK §6 | StartOS／ShutdownOS、Extended Status、中断禁用配对、调用层级 |
| ARTI | §7.16／7.17；`SWS_Os_00858／00829／00836／00837` | 生成描述与实际对象一致；旧日志宏不替代 ARTI |
| 扩展功能 | `SWS_Os_00240`、§7.11 | 首版不承诺 SC2–4、时间／内存保护、多核或 IOC |
| RTE 调用的 OS 服务 | `OS00383／00392` | GetCounterValue／GetElapsedValue 的同步／重入属性 |

### 5.4 通信栈

| 模块与文档 | 条款 | 义务要点 |
| --- | --- | --- |
| COM：`AUTOSAR_CP_SWS_COM.pdf`（`8addd33ad1e625b4e9d01afc938131d424f7161f18075f44d2a5fcfa925c5190`） | §7.3／8.3／8.4；`SWS_Com_00432`（Init）、`00130`（DeInit）、`00194`（GetStatus）、`00197`（SendSignal）、`00198`（ReceiveSignal）、`00123`（RxIndication）、`00001`（TriggerTransmit）；`SWS_Com_00840`；`SWS_Com_00772`；`00300／00301`；`00348／00861／00388`；`SWS_Com_91019／91020`（DRAFT）；ECUC `00181`（MDT）、`00387`（ComIPduCallout） | 标准 callback 形状为 `void Com_RxIndication(PduIdType,const PduInfoType*)` 与 `Std_ReturnType Com_TriggerTransmit(PduIdType,PduInfoType*)`（pull-copy）；未分组 Tx PDU 在 Com_Init 隐式启动且不可停止；只有属于启用 Rx DM 的 group 的 PDU 才启用 monitoring；IMMEDIATE 与 DEFERRED 语义不同；TriggerIPDUSend 手动请求与零 minimum delay |
| PduR：`AUTOSAR_CP_SWS_PDURouter.pdf`（`3f97e95f6b36c876daeddc1bd90db7d0b24ebd0aa652078349d323ad6664ad6f`） | `SWS_PduR_00334`（Init／PBConfigType）；`00406`（upper transmit）；`00507／00518／00375`；`ECUC_PduR_00311／00322`；`00241` | `PduR_<User:Up>Transmit` 为 `Std_ReturnType(PduIdType,const PduInfoType*)`；lower Rx 为 `void(PduIdType,const PduInfoType*)`；TP StartOfReception／CopyRxData／CopyTxData 的 BufReq 与 buffer-size 闭包；PBConfigType 由模块所属 `PduR_PBcfg.c` 生产 |
| LSduR：`AUTOSAR_CP_SWS_LSduRouter.pdf`（`2cd776bc9d54dd8d7f12fa300f24bbf4b790560f9fe7cc9702d20c76b24fdd3a`，R24-11 DRAFT） | `91006`（Init）、`91009`（upper transmit）、`91011`（Rx）、`91013`（confirmation）、`91014`（pull）；`00008／00012／00015–19`；`00024／00025`（zero cost）；`00035` | 一对一、同 partition、无 metadata 的 IF 路由翻译真实 PDU ID 并原样传播 lower 返回值；LSduR 无 Rx／Tx 长度检查或 Tx buffering；未选 ZeroCostOperation |
| CanIf：`AUTOSAR_CP_SWS_CANInterface.pdf`（`84bf624c9e497d765653c8df4d62a6e3835efb64f76c742d727b177a64c10f53`） | `SWS_CANIF_00001／00003／00229／00005／00008／00009／00864／00865／00866／00874／00318`；`91002`（DeInit，draft）；`00382`、`00168`；`00073／00489`、`00739`；`00893`、`00900`；`SWS_CANIF_91001／00898／00899`（GetControllerErrorState，0x4b）；`91003／91004`（0x4d／0x4e）、`00907–00910`；`SWS_CANIF_00653`；`00218`；`00006` | Init 取 `CanIf_ConfigType*`；Transmit 服务 ID 0x49（RxIndication 0x14），不同 PduId 可重入、同 PduId 不可重入；Init 后 PDU mode 为 OFFLINE，STOPPED／BusOff→TX_OFFLINE，SLEEP→OFFLINE，SetPduMode 须在 STARTED；OFFLINE Tx、长度拒绝等须报告 runtime error；controller ID 从 0 起 |
| Can：`AUTOSAR_CP_SWS_CANDriver.pdf`（`d04d35b80d8f445b66f960fd9b5be4f02f324e73cc4be239887e8c1235b172aa`） | `00233`（typed Can_Write）；`00212–214／00275`；`00282`；`00370／00373`；`00218`；`SWS_Can_00227`、`00109`、`ECUC_Can_00314`、`00183`；`SWS_Can_00368`、`00369`；`SWS_Can_00234`、`SWS_Can_00279`、`SWS_CAN_00496`；`ECUC_Can_00316`、`ECUC_Can_00326`；`ECUC_Can_00437／00438／00484`、`00355`、`00486`；`SWS_Can_00441／00442` | 实际 BUSY 与已复制帧；STOP 取消；polling mode indication；BusOff polling 由 `Can_MainFunction_BusOff`；driver 发出的 Mode／BusOff 使用 abstract CanIf ID；driver controller ID 从 0 无间隔，HRH／HTH 从 0 无间隔 |
| CanTp：`AUTOSAR_CP_SWS_CANTransportLayer.pdf`（`7334d633b02c443aacbe0ca25e20c319e977104eeb4bf8e67ac0358cfea34a22`） | `SWS_CanTp_00208／00211／00212／00213／00214／00215`；`00027／00168／00170／00238／00273／00202／00200`；`SWS_CanTp_00315`；`00223`／`ECUC_CanTp_00251`；`SWS_CanTp_00343`；`00205`；`00184／00280`；`00204／00298／00299`、`00075／00355`、`00057`、`00344–00349`；`00352／00229`；`00257／00260–00263`；`ECUC_CanTp_00301／00268` | Init 取 `const CanTp_ConfigType*`；OFF→ON／idle，reinit 丢连接，Shutdown 无 PduR 通知；已接纳 Tx／Rx 必有最终通知，拒绝 admission 无最终通知；收到 FC(WAIT) 重启 N_Bs，WFTmax 限制本端发出的 WAIT；lower E_NOT_OK 终止发送，与 upper CopyTxData BUSY 的 N_Cs 内重试区分；CanTp 自身 NSdu ID 与 PduR callback ID 属于不同命名空间 |

### 5.5 诊断与模式管理

| 模块与文档 | 条款 | 义务要点 |
| --- | --- | --- |
| Dcm：`AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf`（`cf5aeee78fda6e5a25f982f04cb146ce4a75586de7ba41913d46b0cea1cc2407`） | §7.4.2（0x22）、`SWS_Dcm_01335`／`ECUC_Dcm_00638`、`SWS_Dcm_00433–00437`、`ECUC_Dcm_00713／00669`；§8.7.3.2.1、`SWS_Dcm_00793`（p320）、§8.8.3.2（p415）；`SWS_Dcm_00037／00094／00556／00093／00092／00351／00053`；`SWS_Dcm_01069`、`01142`、`01377`、`ECUC_Dcm_00952`、`00148–00162／01324–01326`、`SWS_Dcm_01068`；`00356／00358／00360`；`00111／00241`；`01069–01071／01142／01373–01378`；`00338`、`00339`、`00520`；`00977／00978`；`01062`、`00311`、`01670`、`91019`；`00085`（F186）；`00200／00201／00204`、`00238／00240`；`00806`；`01327／00777／00807／00781`、`91021／23／27／31`、`91034／35／37／39`、`91022／24／28／32`；`01477／01478／01480`、`91067／91074／91075`；`ECUC_Dcm_00864` | `USE_DATA_SYNCH_CLIENT_SERVER`／`UINT8_N` 同步 ReadData；先核对 DID／会话／长度再调用数据访问；Dcm 拥有 request／response buffer，到 confirmation 前不复用；`Dcm_SetActiveDiagnostic` 恒 E_OK；必需 `DcmDslProtocolComMChannelRef`；NoCom／Silent／Full 按 channel 门控；内部 DID F186 读取当前会话；DID 数超过所配置 `DcmDspMaxDidToRead` 回 NRC 0x13；SPRMIB；每个 connection 的 authentication state |
| ComM：`AUTOSAR_CP_SWS_COMManager.pdf`（`564560db2899fa0f8695e5fad607f3cc477a4c84abfccfe968e916117cf2ef34`） | `SWS_ComM_00866／00873／00874／00861／00957`、`ECUC_ComM_00888`；`SWS_ComM_00962`；`00275`；`00893／00895`；`00619／00552／91002`；`00669`；ECUC `00563`；`00783`；`01022／01023` | 所选 host CDD channel（`COMM_BUS_TYPE_CDD`，`Ecu_HostBusSM`，NM NONE）为合法 provider；选择 `COMM_BUS_TYPE_CAN` 须真正 CanSM；诊断 active 为 FULL 需求；NM NONE 仍需 NETWORK_REQUESTED／READY_SLEEP 与最小 Full duration；同步唤醒为 false 时的唤醒请求状态 |
| CanSM：`AUTOSAR_CP_SWS_CANStateManager.pdf`（`9f215f2f532e94a9d8f2bbbf4ba4208d31117d1b82f5277792638f3b6f0fcceb`） | — | 仅核对 CAN BusSM 义务与 host CDD 边界；**未实现、不宣称**完整 CanSM |
| BswM：`AUTOSAR_CP_SWS_BSWModeManager.pdf`（`a6ce38abd020c3aef91bc90ebc35243b888d05b899c36eba33a2305bfbe08ab3`） | `SWS_BswM_00047／00078／00091`；`00880／00883`；`00955`；`00935`；`ECUC_BswM_00843`；`00115／00116`、`00066`；`00214／00216`；`00053`；`00075` | IMMEDIATE ComM mode input 及其规则；`BswM_ModeType`／`UserType` 为 uint16；`BswM_MainFunction` 在 `SchM_BswM.h`；无 DEFERRED 时无需周期评价但入口须存在；UserCallout 为完整固定参数调用 |
| Det：`AUTOSAR_CP_SWS_DefaultErrorTracer.pdf`（`f5caa8cb8f2b9ef2e2578a96d940abb795fbf7dfc09a1899b0b81797c2835e1f`） | `SWS_Det_00008／00010／00183／00184`；`00024`；`00014／00018／00503`；`00026／00208` | `Det_ReportRuntimeError` 恒返回 E_OK，已 Init 时调用全部配置 hooks；关闭 DevErrorDetect 不免除 runtime 报告；`Det_ReportError` 要求停止执行，与 runtime 报告不同 |

## 6. 多组件剖面的已核定义务

当前主线（多组件剖面，按 [ADR 0010](adr/0010-single-generation-mainline.md) 将同时承接单 SWC 工程）的首批支持配置如下。范围外的配置在生成前明确拒绝，不静默降级；无关有效内容原样保留。

| 维度 | 支持范围与依据 | 拒绝 |
| --- | --- | --- |
| 组合与实例 | 一个 `ECU_EXTRACT`、一个 ECU、一个平坦 CompositionSwComponentType；多个不同 ApplicationSwComponentType，每种一个实例，`supportsMultipleInstantiation=false`（RTE §4.1.4.2–4.1.4.4、`SWS_Rte_02008／02009`） | 同类型重复实例、嵌套与 delegation |
| 类型 | 显式非排队 uint32 S/R；同步 C/S 的 uint32 IN／OUT／INOUT 与固定 uint8[4] OUT；应用 primitive→implementation→base type 映射逐项核对（`constr_1068／1069`） | 等宽但不同接口／类型、缺映射；结构体、动态数组或转换 |
| 本地 S/R | 每个已使用 R 端点恰有一个本地 P 或一个网络映射，P 可扇出；每个端点要求明确 ComSpec 初值，首次发布前各 R 用自己的初值，冲突时 R 优先（`SWS_Rte_04501／04502`）；`handleNeverReceived=false`、`aliveTimeout=0`、`handleTimeoutType=NONE`、无 invalidValue | 缺初值；本地 `handleNeverReceived=true`（待按 `SWS_Rte_07381–07383` 实现） |
| 网络 S/R | `IMMEDIATE` 信号处理（COM `00300`）；最小真实 Rx I-PDU group＋标准 reception DM＋COM→RTE 通知（`SWS_Com_00772`、`SWS_Rte_08061／08062／08103／08104`）；未分组 Tx PDU 按 `SWS_Com_00840` 隐式启动；`COM_STOPPED` 只由实际 COM 状态映射（`SWS_Rte_06830／07822`），CAN 停止不等于 COM 不可用；`ComSignalInitValue` 为 0，RTE 接收初值独立取自 R ComSpec | `DEFERRED`（COM `00301`）；显式 `ComIPduHandleId`；未选 group 变体、COM TP、动态信号 |
| 同步 C/S | 本地同步调用，非重入服务器；服务器在调用者 owner 上下文执行；无 possibleErrors 时 server entry 返回 void（`SWS_Rte_08913`），客户端 `Rte_Call` 返回 `Std_ReturnType`；参数 policy 仅缺省或 USE-ARGUMENT-TYPE（`SWS_Rte_07027`、`CONSTR1286`） | USE-VOID；调用环、并发、异步、跨任务；空 OUT／INOUT 指针（不修改输出） |
| 调度 | 每个周期 runnable 恰有一个 TimingEvent，全部共用正整数毫秒周期、offset=0，映射到唯一 Extended `Task_Ecu` 的合法 OsEvent 与 Alarm／ExpiryPoint；周期 mapping 显式 `isMappedToTask=true` 并带 task ref；OperationInvokedEvent 的 mapping 为 false 且无 task 引用（`CONSTR08936`／`08938`） | 同 runnable 多 TimingEvent、不同周期或非零 offset、重复位置、task 不属 owner、周期与 alarm 不符 |
| 分区 | 显式唯一 EcucPartition ID0，关联唯一 CoreId0，包含全部 APPLICATION／SERVICE root-context 实例；不启用 OsApplication | 多分区 |
| 诊断路由 | 完整 physical Dcm→PduR→CanTp 路由，可无应用 DID（仍提供内部 F186、0x10／0x3E，未知 DID 返回 NRC 0x31，Dcm `00085`）；两条诊断路由均不选为 network-only | 半条路由或单个端点 |
| CAN | 11 位 Classical CAN、单控制器单通道、static normal addressing、单 physical DoCAN 连接、`CanIfPublicTxBuffering=false`；controller 与 HOH 编号从 0 连续（`ECUC_Can_00316`、`SWS_CANIF_00653`、`ECUC_Can_00326`） | 29 位、CAN FD、多通道、metadata、动态 ID；非连续 controller／HOH 编号 |
| 名字 | 组件类型与 EcucPartition 短名不得占用已交付 runtime 的内存 scope（Rte、Ecu、Can、CanIf、CanTp、Com、Dcm、PduR、LSduR、ComM、BswM、Ecu_HostBusSM）；C 标识符、文件名、include guard 碰撞拒绝 | `CONTRACT_NAME_COLLISION` 等生成前诊断 |

ECUC metadata 的数值范围通过不等于配置语义合法；非等值 ID（如 controller 7／9）的模块向量只检验命名空间机制，不是合法 source profile 的验收证据。

## 7. 已知未核查项与差距

以下不能由现有记录推定为已完成，也不能以“主机专属”为由延期；各项退出条件是取得合法正文、完成对应模块的全文核对与独立验收。

- **逐条全文审计未完成。** 上表是已定位条款，不是完整适用矩阵；各模块全部必需 API、状态、错误、回调、BSWMD、配置语义与可选条件须在模块整改时闭合，未处置条目以 §5 的核算命令为准。
- **`SWS_BSW_00036` 跨模块头文件版本检查**：已读 CanIf 头与多组件模块无对应版本宏或预处理检查，是明确的审计候选。
- **Can published information**：项目尚无已分配的 AUTOSAR vendor ID，也无经核对的 R24-11 数字发布宏映射；`CAN_VERSION_INFO_API` 默认关闭，启用即拒绝。取得正式身份后才能实现 GetVersionInfo 与外部版本交叉检查（见 [runtime/README.md](../runtime/README.md)）。
- **Dem／NvM／Security**：不在主线程序中，已随旧链路删除（[ADR 0013](adr/0013-remove-frame-signal-host-chain.md)）；以后加入主线时须先核定标准公共面、存储链与 Security 的项目责任。
- **MemMap 与段放置**：主机 PE／ELF 段映射已有实际证据，MCU 段与权限只由目标工具链与布局关闭。
- **OS**：完整 SC1 条款逐项证据、ARTI 适用性与 OSEK 引用版本（ISO 17356-3）全文未核对；Windows 实际交接运行未在每次变更后执行。
- **MISRA**：221 项 assessment 当前均为 `not_assessed`，生成工程源码分析 `passed=false`；固定 addon 的已知限制（R21.8 仍报告 `getenv`，终止函数清单缺 `_Exit`／`quick_exit`，errno／mmap 模型差异）只作核查线索。
- **资料可用性**：部分工作树只有官方资料索引而无原件；R24-11 `TR_ChangeDocumentation.zip` 不在官方清单内且未取得；`AUTOSAR_CP_TR_ModelingShowCases.pdf` 已引用（跨源 ARPackage 片段合法）但未加入已匹配 SHA-256 的文件身份声明。
- **网络管理**：完整 Nm、CanNm、CanSM 未实现；当前 host CDD BusSM 的模式行为不代表 CAN BusSM 符合性。
