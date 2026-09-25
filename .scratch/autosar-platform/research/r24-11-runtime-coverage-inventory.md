# CP R24-11 运行模块覆盖盘点（比例基线，资料收集）

> 状态：**尚不能计算 overall 25%**。本页是供后续核定分母的可追溯清单，不是 AUTOSAR 一致性声明，也不声明本项目达到任一百分比。资料来源为使用者本地官方材料；`docs/official/` 下 PDF/ZIP 属本地忽略资料，不随仓库版本控制或分发。链接仅在本地材料存在时可用。

## 口径与限制

1. **先分辨发布工件与运行模块。** CP R24-11 Release Overview §3/Table 3.1 将发布物划分为 19 个 cluster，表内混有 SWS、RS、SRS、ASWS、TPS、MOD、EXP、TR、示例等；cluster、文档或 PDF 数均不是运行模块数。来源：[CP Release Overview §3/Table 3.1，pp.19–27](../../../docs/official/R24-11/CP/ReleaseDocumentation/AUTOSAR_CP_TR_ReleaseOverview.pdf#page=19)，尤其列 cluster 的 §3 起始处与各页 `Long Name / File Name / Life cycle changes` 表；本地抽取文本 lines 475–752。
2. **模块候选的规范锚点。** BSW General Appendix A 将 List of Basic Software Modules 分为 A.1 Libraries、A.2 Modules、A.3 SpecialFiles；A.2 是官方模块目录，A.1 是库，A.3 是特殊文件。它本身不标各条当前 VALID/DRAFT/OBSOLETE 生命周期，也不表示某个模块为独立可链接的可执行映像。来源：[BSW General §A，Tables A.1–A.3，pp.95–99](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf#page=95)，本地抽取文本 lines 2894–3074。
3. **不从空白推导 VALID。** Overview 表的 `Life cycle changes` 栏描述发布生命周期变更；空白不能当成对每个模块的明确 `VALID` 标记。故以下状态取值规则为：Overview 明确标 obsolete 的列为 `OBSOLETE`；明确写了 draft 的配置/要求条目列作具体 draft 对象；其他模块条目标 `UNKNOWN—需查该模块 R24-11 权威状态`。官方模块清单或 SWS 文档显示 `published` 只能证明文档发布状态，不等同于模块/条目生命周期状态。这里**没有据空白产生 VALID 分类，也不发布模块总数/分母或比例**。
4. **“可执行模块族”与生命周期要在计分前定型。** A.2 是官方 module-level 清单；本页先逐项保留其短名，不能把复合协议族名称合并来虚增进度。A.1 库、Cdd、SchM 是否构成独立可执行覆盖单元，须在计分前依据对应规范/交付方式作出并记录决定，之后不得按结果临时排除。凡来源确认属于当前 VALID runtime 的模块族，即使产品尚未实现、用户 profile 不支持，也留在总目标中并计零；A.3 类型/特殊文件不是独立模块单元。不得因实现范围或已命名 profile 缩小 overall 分母。

## A.2 官方模块候选清单及项目行为/目标证据

**生命周期标记**：`OBSOLETE` = Overview §3 明确列明 obsolete；`UNKNOWN` = 目前所读 Overview/Appendix 未证明 VALID，也未找到其 obsolete/draft 声明；UNKNOWN 不表示有效或无效。表中 `§3 lead` 仅给 cluster 页码与长名称/表内文件名线索，不保证对应 SWS 的实际文件 ID；尤其 Cdd、SchM 等没有直接的一对一 SWS 行。逐项查 Release Overview 表的 `File Name`、实际本地规范文件名及规范元数据，填入并核对确切 source ID 前，任何行不得用于冻结清单或分母。

**目标列**：`H` = 当前主机虚拟目标的行为记录；`M1`、`M2` = 将来逐个指定的 MCU/板卡+工具链档案。所有行的 M1/M2 均为 `TBD—未指定、未验证`，不是支持声明。未被 README 当前行为明确覆盖的 H 项均为 `TBD—本清单未找到用户可观察实现行为证据`；存在具体受限行为时只记行为，不升级成模块整体支持。

| 家族/cluster | 模块（Appendix A.2 短名；规范长名） | 生命周期 | §3 lead（长名称/文件名线索；实际 source ID 待核） | 当前 H：README 可见行为（仅行为） | M1 | M2 |
|---|---|---|---|---|---|---|
| IO | Adc — ADC Driver | UNKNOWN | p.24–25, SWS ADC Driver | TBD | TBD | TBD |
| SWArch/SystemServices | Arti — AUTOSAR Run-Time Interface | UNKNOWN | p.26, SWS AUTOSAR Run-Time Interface | TBD | TBD | TBD |
| ModeManagement | BswM — BSW Mode Manager | UNKNOWN | p.26, SWS Basic Software Mode Manager | TBD | TBD | TBD |
| SystemServices | SchM — BSW Scheduler Module | UNKNOWN—未见独立 SWS 条目 | Appendix A.2 p.95–99; §3 SWS mapping TBD | TBD | TBD | TBD |
| Memory | BndM — Bulk NvData Manager | UNKNOWN | p.24, SWS Bulk NvData Manager | TBD | TBD | TBD |
| 通信 / Communication | Mirror — Bus Mirroring | UNKNOWN | p.20–23, SWS Bus Mirroring | TBD | TBD | TBD |
| 通信 / Communication | Can — CAN Driver | UNKNOWN | p.20–21, SWS CAN Driver | 主机虚拟 CAN 场景可配置 11-bit CAN frame 的信号收发、超时及 BUS_OFF/STARTED 故障恢复（README:8,13）；不是硬件 Can driver 证据 | TBD | TBD |
| 通信 / Communication | CanIf — CAN Interface | UNKNOWN | p.20–21, SWS CAN Interface | 主机信号/诊断组合有受限 CanIf ECUC 与虚拟行为（README:9,15）；不能据此称完整 CanIf | TBD | TBD |
| 通信 / Communication | CanNm — CAN Network Management | UNKNOWN | p.20–21, SWS CAN Network Management | TBD | TBD | TBD |
| 通信 / Communication | CanSM — CAN State Manager | UNKNOWN | p.20–21, SWS CAN State Manager | TBD | TBD | TBD |
| 通信 / Communication | CanTrcv — CAN Transceiver Driver | UNKNOWN | p.20–21, SWS CAN Transceiver Driver | TBD | TBD | TBD |
| 通信 / Communication | CanTp — CAN Transport Layer | UNKNOWN | p.20–21, SWS CAN Transport Layer | 主机诊断配置支持受限 DoCAN 多帧、流控、序号、计时及错误恢复（README:10,13）；非完整 CanTp 证明 | TBD | TBD |
| 通信 / Communication | CanXL — CAN XL Driver | UNKNOWN | p.20–21, SWS CAN XL Driver | TBD | TBD | TBD |
| 通信 / Communication | CanXLTrcv — CAN XL Transceiver Driver | UNKNOWN | p.20–21, SWS CAN XL Transceiver Driver | TBD | TBD | TBD |
| 通信 / Communication | CV2x — Cellular Vehicle-2-X Driver | UNKNOWN | p.21–23, SWS Cellular V2X Driver | TBD | TBD | TBD |
| 通信 / Communication | ISO15118Chrg — Charging Manager | UNKNOWN | p.20–23, SWS ISO15118 Charging | TBD | TBD | TBD |
| 通信 / Communication | CnV2xM — Chinese Vehicle-2-X Management | UNKNOWN | p.20–23, SWS Chinese V2X Management | TBD | TBD | TBD |
| 通信 / Communication | CnV2xMsg — Chinese Vehicle-2-X Message | UNKNOWN | p.20–23, SWS Chinese V2X Message | TBD | TBD | TBD |
| 通信 / Communication | CnV2xNet — Chinese Vehicle-2-X Network | UNKNOWN | p.20–23, SWS Chinese V2X Network | TBD | TBD | TBD |
| 通信 / Communication | CnV2xSec — Chinese Vehicle-2-X Security | UNKNOWN | p.20–23, SWS Chinese V2X Security | TBD | TBD | TBD |
| 通信 / Communication | Com — COM | UNKNOWN | p.20–21, SWS COM | 主机信号 ECU 配置与信号值/位向量检查（README:8–9,13）；不是全 COM 语义/选项 | TBD | TBD |
| 通信 / Communication | ComXf — COM Based Transformer | UNKNOWN | p.21–22, SWS COM Based Transformer | TBD | TBD | TBD |
| SystemServices | ComM — COM Manager | UNKNOWN | p.26–27, SWS Communication Manager | TBD | TBD | TBD |
| Complex Drivers | Cdd — Complex Drivers | UNKNOWN—类别，非单一标准实现 | Appendix A.2 p.96；§3 无单个 CDD SWS | TBD | TBD | TBD |
| MCAL | CorTst — Core Test | UNKNOWN | p.25, SWS Core Test | TBD | TBD | TBD |
| Crypto | Crypto — Crypto Driver | UNKNOWN | p.23, SWS Crypto Driver | TBD | TBD | TBD |
| Crypto | CryIf — Crypto Interface | UNKNOWN | p.23, SWS Crypto Interface | TBD | TBD | TBD |
| Crypto | Csm — Crypto Service Manager | UNKNOWN | p.23, SWS Crypto Service Manager | TBD | TBD | TBD |
| 通信 / Communication | Dds — Data Distribution Service | UNKNOWN | p.21–23, SWS Data Distribution Service | TBD | TBD | TBD |
| SystemServices | Det — Default Error Tracer | UNKNOWN | p.26–27, SWS Default Error Tracer | TBD | TBD | TBD |
| Diagnostics | Dcm — Diagnostic Communication Manager | UNKNOWN | p.23, SWS Diagnostic Communication Manager | README:10–15 有限主机 UDS 服务/场景；不表示 Dcm 全部服务或 ECUC 配置成熟。`DcmDspRoutineFncSignature=ROUTINE_FNC_NORMAL` 为 DRAFT 配置选项，见 `.scratch/autosar-platform/issues/07-assurance-gates.md:54`；只标该选项 draft，不标 Dcm 整体 | TBD | TBD |
| Diagnostics | Dem — Diagnostic Event Manager | UNKNOWN | p.23, SWS Diagnostic Event Manager | README:12–13 有限单 DTC、超时与主机文件重启保存行为；非完整 Dem/NvM 或实体 NVM 声明 | TBD | TBD |
| 通信 / Communication | Dlt — Diagnostic Log and Trace | UNKNOWN | p.21–23, SWS Diagnostic Log and Trace | TBD | TBD | TBD |
| 通信 / Communication | DoIP — Diagnostic over IP | UNKNOWN | p.21–23, SWS Diagnostic over IP | TBD | TBD | TBD |
| IO | Dio — DIO Driver | UNKNOWN | p.24–25, SWS DIO Driver | TBD | TBD | TBD |
| 通信 / Communication | E2EXf — E2E Transformer | UNKNOWN | p.22, SWS E2E Transformer | TBD | TBD | TBD |
| ModeManagement | EcuM — ECU State Manager | UNKNOWN | p.26, SWS ECU State Manager | TBD | TBD | TBD |
| Memory | Ea — EEPROM Abstraction | UNKNOWN | p.25, SWS EEPROM Abstraction | TBD | TBD | TBD |
| Memory | Eep — EEPROM Driver | **OBSOLETE** | p.25, SWS EEPROM Driver；Overview 明文 obsolete | TBD | TBD | TBD |
| Ethernet | Eth — Ethernet Driver | UNKNOWN | p.22, SWS Ethernet Driver | TBD | TBD | TBD |
| Ethernet | EthIf — Ethernet Interface | UNKNOWN | p.22, SWS Ethernet Interface | TBD | TBD | TBD |
| Ethernet | EthSM — Ethernet State Manager | UNKNOWN | p.22, SWS Ethernet State Manager | TBD | TBD | TBD |
| Ethernet | EthSwt — Ethernet Switch Driver | UNKNOWN | p.22, SWS Ethernet Switch Driver | TBD | TBD | TBD |
| Ethernet | EthTrcv — Ethernet Transceiver Driver | UNKNOWN | p.22, SWS Ethernet Transceiver Driver | TBD | TBD | TBD |
| Security | Fw — Firewall | UNKNOWN | p.23, SWS Firewall | TBD | TBD | TBD |
| Memory | Fls — Flash Driver | **OBSOLETE** | p.25, SWS Flash Driver；Overview 明文 obsolete | TBD | TBD | TBD |
| Memory | Fee — Flash EEPROM Emulation | UNKNOWN | p.25, SWS Flash EEPROM Emulation | TBD | TBD | TBD |
| Memory | FlsTst — Flash Test | UNKNOWN | p.25, SWS Flash Test | TBD | TBD | TBD |
| FlexRay | FrArTp — FlexRay AUTOSAR Transport Layer | UNKNOWN | p.22, SWS FlexRay AUTOSAR Transport Layer | TBD | TBD | TBD |
| FlexRay | Fr — FlexRay Driver | UNKNOWN | p.22, SWS FlexRay Driver | TBD | TBD | TBD |
| FlexRay | FrIf — FlexRay Interface | UNKNOWN | p.22, SWS FlexRay Interface | TBD | TBD | TBD |
| FlexRay | FrTp — FlexRay ISO Transport Layer | UNKNOWN | p.22, SWS FlexRay ISO Transport Layer | TBD | TBD | TBD |
| FlexRay | FrNm — FlexRay Network Management | UNKNOWN | p.22, SWS FlexRay Network Management | TBD | TBD | TBD |
| FlexRay | FrSM — FlexRay State Manager | UNKNOWN | p.22, SWS FlexRay State Manager | TBD | TBD | TBD |
| FlexRay | FrTrcv — FlexRay Transceiver Driver | UNKNOWN | p.22, SWS FlexRay Transceiver Driver | TBD | TBD | TBD |
| SystemServices | FiM — Function Inhibition Manager | UNKNOWN | p.27, SWS Function Inhibition Manager | TBD | TBD | TBD |
| MCAL | Gpt — GPT Driver | UNKNOWN | p.25, SWS GPT Driver | TBD | TBD | TBD |
| SystemServices | HTMSS — HW Test Manager on start up and shutdown | UNKNOWN | p.27, SWS Hardware Test Manager | TBD | TBD | TBD |
| Communication | I2C — I2C Driver | UNKNOWN | p.22, SWS I2C Driver | TBD | TBD | TBD |
| IO | Icu — ICU Driver | UNKNOWN | p.24–25, SWS ICU Driver | TBD | TBD | TBD |
| Communication | IEEE1722Tp — IEEE1722 Transport Layer | UNKNOWN | p.22, SWS IEEE1722 Transport Protocol Module | TBD | TBD | TBD |
| Security | IdsM — Intrusion Detection System Manager | UNKNOWN | p.23, SWS Intrusion Detection System Manager | TBD | TBD | TBD |
| IO | IoHwAb — IO HW Abstraction | UNKNOWN | p.24–25, SWS I/O Hardware Abstraction | TBD | TBD | TBD |
| Communication | IpduM — IPDU Multiplexer | UNKNOWN | p.21–22, SWS I-PDU Multiplexer | TBD | TBD | TBD |
| Crypto | KeyM — Key Manager | UNKNOWN | p.23, SWS Key Manager | TBD | TBD | TBD |
| Communication | LdCom — Large Data COM | UNKNOWN | p.22, SWS Large Data COM | TBD | TBD | TBD |
| LIN | Lin — LIN Driver | UNKNOWN | p.22, SWS LIN Driver | TBD | TBD | TBD |
| LIN | LinIf — LIN Interface | UNKNOWN | p.22, SWS LIN Interface | TBD | TBD | TBD |
| LIN | LinSM — LIN State Manager | UNKNOWN | p.22, SWS LIN State Manager | TBD | TBD | TBD |
| LIN | LinTrcv — LIN Transceiver Driver | UNKNOWN | p.22, SWS LIN Transceiver Driver | TBD | TBD | TBD |
| Communication | LSduR — LSDU Router | UNKNOWN | p.22, SWS Linklayer Sdu Routing Module | TBD | TBD | TBD |
| Communication/Security | Mka — MACsec Key Agreement | UNKNOWN | p.22, SWS MACsec Key Agreement | TBD | TBD | TBD |
| MCAL | Mcu — MCU Driver | UNKNOWN | p.25, SWS MCU Driver | TBD | TBD | TBD |
| Memory | MemIf — Memory Abstraction Interface | UNKNOWN | p.25, SWS Memory Abstraction Interface | TBD | TBD | TBD |
| Memory | MemAcc — Memory Access | UNKNOWN | p.25, SWS Memory Access | TBD | TBD | TBD |
| Memory | Mem — Memory Driver | UNKNOWN | p.25, SWS Memory Driver | TBD | TBD | TBD |
| Communication | Nm — Network Management Interface | UNKNOWN | p.22, SWS Network Management Interface | TBD | TBD | TBD |
| Memory | NvM — NVRAM Manager | UNKNOWN | p.25, SWS NVRAM Manager | README:12–13 uses host file and CRC slots for a single DTC profile; no physical NvM backend evidence | TBD | TBD |
| IO | Ocu — OCU Driver | UNKNOWN | p.24–25, SWS OCU Driver | TBD | TBD | TBD |
| SystemServices | Os — OS | UNKNOWN | p.27, SWS Operating System | README:9–13 uses host periodic scheduling/virtual ECU process behavior; not AUTOSAR OS conformance or an RTOS/MCU profile | TBD | TBD |
| Communication | PduR — PDU Router | UNKNOWN | p.21–22, SWS PDU Router | README:9,15 includes generated/configured path in host profile; not full PduR behavior | TBD | TBD |
| IO | Port — Port Driver | UNKNOWN | p.24–25, SWS Port Driver | TBD | TBD | TBD |
| IO | Pwm — PWM Driver | UNKNOWN | p.24–25, SWS PWM Driver | TBD | TBD | TBD |
| Memory | RamTst — RAM Test | UNKNOWN | p.25, SWS RAM Test | TBD | TBD | TBD |
| RTE | Rte — RTE | UNKNOWN | p.26 lead, Specification of RTE | README:9 仅描述生成工程含 RTE 接口；实现证据为固定 runtime `Rte.h`/`Rte.c` 与 host generic signal adapter，不是 SWC-specific RTE generation | TBD | TBD |
| J1939 | J1939Dcm — SAE J1939 Diagnostic Communication Manager | UNKNOWN | p.23, SWS SAE J1939 Diagnostic Communication Manager | TBD | TBD | TBD |
| J1939 | J1939Fscp — SAE J1939 Functional Safety Communication Protocol | UNKNOWN | p.22, SWS functional-safety protocol handler for SAE J1939 | TBD | TBD | TBD |
| J1939 | J1939Nm — SAE J1939 Network Management | UNKNOWN | p.22, SWS Network Management for SAE J1939 | TBD | TBD | TBD |
| J1939 | J1939Rm — SAE J1939 Request Manager | UNKNOWN | p.22, SWS Request Manager for SAE J1939 | TBD | TBD | TBD |
| J1939 | J1939Tp — SAE J1939 Transport Layer | UNKNOWN | p.22, SWS Transport Layer for SAE J1939 | TBD | TBD | TBD |
| Communication | SecOC — Secure Onboard Communication | UNKNOWN | p.21–23, SWS Secure Onboard Communication | TBD | TBD | TBD |
| Communication | Sd — Service Discovery | UNKNOWN | p.23, SWS Service Discovery | TBD | TBD | TBD |
| Communication | SoAd — Socket Adaptor | UNKNOWN | p.23, SWS Socket Adaptor | TBD | TBD | TBD |
| SystemServices | SwCluC — Software Cluster Connection | UNKNOWN | p.27, SWS Software Cluster Connection Module | TBD | TBD | TBD |
| Communication | SomeIpXf — SOME/IP Transformer | UNKNOWN | p.23, SWS SOME/IP Transformer | TBD | TBD | TBD |
| Communication | SomeIpTp — SOME/IP Transport Protocol | UNKNOWN | p.23, SWS SOME/IP Transport Protocol | TBD | TBD | TBD |
| Communication | Spi — SPI Handler Driver | UNKNOWN | p.21–23, SWS SPI Handler/Driver | TBD | TBD | TBD |
| GlobalTime | StbM — Synchronized Time-Base Manager | UNKNOWN | p.24, SWS Synchronized Time-Base Manager | TBD | TBD | TBD |
| Communication | TcpIp — TCP/IP Stack | UNKNOWN | p.23, SWS TCP/IP Stack | TBD | TBD | TBD |
| SystemServices | Tm — Time Service | UNKNOWN | p.27, SWS Time Service | TBD | TBD | TBD |
| GlobalTime | CanTSyn — Time Sync over CAN | UNKNOWN | p.24, SWS Time Synchronization over CAN | TBD | TBD | TBD |
| GlobalTime | EthTSyn — Time Sync over Ethernet | UNKNOWN | p.24, SWS Time Synchronization over Ethernet | TBD | TBD | TBD |
| GlobalTime | FrTSyn — Time Sync over FlexRay | UNKNOWN | p.24, SWS Time Synchronization over FlexRay | TBD | TBD | TBD |
| Communication | Ttcan — TTCAN Driver | UNKNOWN | p.23, SWS TTCAN Driver | TBD | TBD | TBD |
| Communication | TtcanIf — TTCAN Interface | UNKNOWN | p.23, SWS TTCAN Interface | TBD | TBD | TBD |
| Communication | UdpNm — UDP Network Management | UNKNOWN | p.23, SWS UDP Network Management | TBD | TBD | TBD |
| V2X | V2xBtp — Vehicle-2-X Basic Transport | UNKNOWN | p.23, SWS Vehicle-2-X Basic Transport | TBD | TBD | TBD |
| V2X | V2xDM — Vehicle-2-X Data Manager | UNKNOWN | p.23, SWS Vehicle-2-X Data Manager | TBD | TBD | TBD |
| V2X | V2xFac — Vehicle-2-X Facilities | UNKNOWN | p.23, SWS Vehicle-2-X Facilities | TBD | TBD | TBD |
| V2X | V2xGn — Vehicle-2-X GeoNetworking | UNKNOWN | p.23, SWS Vehicle-2-X Geo Networking | TBD | TBD | TBD |
| V2X | V2xM — Vehicle-2-X Management | UNKNOWN | p.23, SWS Vehicle-2-X Management | TBD | TBD | TBD |
| Safety | Wdg — Watchdog Driver | UNKNOWN | p.26, SWS Watchdog Driver | TBD | TBD | TBD |
| Safety | WdgIf — Watchdog Interface | UNKNOWN | p.26, SWS Watchdog Interface | TBD | TBD | TBD |
| Safety | WdgM — Watchdog Manager | UNKNOWN | p.27, SWS Watchdog Manager | TBD | TBD | TBD |
| Communication | WEth — Wireless Ethernet Driver | UNKNOWN | p.23, SWS Wireless Ethernet Driver | TBD | TBD | TBD |
| Communication | WEthTrcv — Wireless Ethernet Transceiver Driver | UNKNOWN | p.23, SWS Wireless Ethernet Transceiver Driver | TBD | TBD | TBD |
| Communication | Xcp — XCP | UNKNOWN | p.23, SWS XCP | TBD | TBD | TBD |

### A.1 库清单（单独登记，不混入 A.2 模块数）

依据 BSW General Appendix A.1 / Table A.1, p.95。库是在运行代码中调用的能力候选；本页不据此认定其为独立 BSW 模块或当前有效，生命周期状态均待各规范核验。

| 库短名 — 名称 | 生命周期 | §3 规范定位 | H | M1 | M2 |
|---|---|---|---|---|---|
| Bfx — BFX Library | UNKNOWN | p.24, SWS Bit Handling Routines | TBD | TBD | TBD |
| Bmc — BSW Multicore Library | UNKNOWN | p.24, SWS Basic Software Multicore Library | TBD | TBD | TBD |
| Crc — CRC Library | UNKNOWN | p.24, SWS CRC Routines | TBD | TBD | TBD |
| E2E — SW-C End-to-End Communication Protection Library | UNKNOWN | p.24, SWS End-to-End Communication Protection Library | TBD | TBD | TBD |
| Efx — Extended Fixed Point Library | UNKNOWN | p.24, SWS Extended Fixed Point Routines | TBD | TBD | TBD |
| Ifl — Floating Point Interpolation Library | UNKNOWN | p.24, SWS Floating Point Interpolation Routines | TBD | TBD | TBD |
| Ifx — Fixed Point Interpolation Library | UNKNOWN | p.24, SWS Fixed Point Interpolation Routines | TBD | TBD | TBD |
| Msf — Memory Library | UNKNOWN | p.24, SWS MSFLibrary | TBD | TBD | TBD |
| Mfl — Floating Point Math Library | UNKNOWN | p.24, SWS Floating Point Math Routines | TBD | TBD | TBD |
| Mfx — Fixed Point Math Library | UNKNOWN | p.24, SWS Fixed Point Math Routines | TBD | TBD | TBD |

## Runtime-inventory exclusions (not silently deleted)

- **Release/meta/docs/configuration tooling, not runtime-module rows:** the 19 release clusters themselves; ReleaseDocumentation and Specification Hashes; ApplicationInterfaces models/examples/guides; BSW General rule documents, type specifications and UML; requirements (RS), requirements/specification use cases (SRS), methodology/templates (`TPS`, ECUC XML parameter model, `MOD`), explanations/guides (`EXP`), reports/examples (`TR`), and `ASWS_TransformerGeneral`. They remain important inputs/evidence, but do not become executable modules by appearing in Table 3.1.
- **A.3 SpecialFiles, not module rows:** ComStack_Types, MemMap, Platform_Types, Std_Types (Appendix A.3, Table A.3, p.99); track as shared integration artifacts in applicable module evidence, not as module-family units.
- **Explicit obsolete non-module artifact:** `AUTOSAR_CP_RS_Features` is marked obsolete in §3 p.24. It is an RS entry, not an executable module and not included in the candidate inventory.
- **DRAFT is item-level unless a source marks a whole module/spec draft.** Known R24-11 example: Dcm `ECUC_Dcm_01215` / `DcmDspRoutineFncSignature` and its `ROUTINE_FNC_NORMAL` option are marked DRAFT in Dcm SWS; project record gives exact extracted text lines 25099–25126 and clarifies the scope at `.scratch/autosar-platform/issues/07-assurance-gates.md:54`. Do not reclassify Dcm/RoutineControl as a whole. Additional draft items/modules must be found by inspecting each spec and metadata, and entered at the narrowest source-supported level.

## 当前用户可见场景（只录行为，不代表模块整体通过）

Source: [`README.md`](../../../README.md), particularly lines 1–15 and 42–49 (user flow). Current host-profile behavior to carry into matrix:

- Create/import/edit/save multi-file AUTOSAR Classic R24-11 ARXML; validate local `AUTOSAR_00053.xsd`; preserve unsupported valid content while rejecting unresolved variants that affect generation (README:3,7–9,15).
- Configure up to 32 frames/64 signals per ECU, standard 11-bit Classical CAN, transmit/receive signals and test virtual timeouts/controller state; run two separately generated host ECU processes on virtual CAN and compare signal/vector behavior (README:8–9,13,47).
- Optional physical DoCAN diagnostic connection with bounded 0x10/0x22/0x3E service behavior and CanTp multi-frame/flow-control/error recovery (README:10,13).
- Optional extension-session volatile 0x2E update of bound Tx signals; optional bounded 0x31/0x01 routine; optional single timeout DTC with restricted 0x19/0x14 behavior and host-file persistence (README:11–13).
- Explicit non-claims: no physical hardware, complete UDS/ISO compliance, 0x27 security access, physical NvM/Flash backend, full service/module support, or release package (README:3,15).

Target column grounding: `.scratch/autosar-platform/issues/05-first-verified-profile.md:11–15,19–21` sets R24-11/FO and host virtual ECU as current target; the NXP board is only a candidate. Therefore **H is behavior-only** and M1/M2 remain empty evidence placeholders until exact hardware, board, toolchain, configuration, and independent target results are recorded.

## 建议的无权重 25% 度量办法（暂不计算）

1. **先闭合完整当前有效目标分母。** 以 §3、Appendix A 和每项权威生命周期来源逐个核实所有当前 VALID runtime module families，并逐条记录唯一性、实际 source/file ID、状态及其依据。最终 overall 模块目标包含所有 source-confirmed 当前 VALID runtime modules；不支持、未实现、未覆盖的行仍在分母且得分为零。计分前必须裁定 A.1 libraries、Cdd、SchM 的模块/可执行单元边界及重复/别名；不能在看到分数后再选择排除。DRAFT/OBSOLETE 不算当前 VALID 分母，但要独立列出状态和来源；UNKNOWN 阻止确认分母，不能当成有效或无效处理。
2. **先固定三个目标轴及 profiles。** 逐项书面定义 H（主机虚拟）、M1、M2（各自的具体目标/工具链）和纳入评估的功能 profiles，形成固定的 `lifecycle × target × profile × module-family` 网格。当前只可报 H/M1/M2 各自的证据状态；M1/M2 仍为未定义/未验证占位。profile 只细分并标明支持范围，不得用它删去 overall VALID runtime 模块目标中的不支持模块。
3. **各轴保留逐格证据，不给项目内任意权重。** 固定网格中的每个当前 VALID module-family × target × profile 都必须有行；未支持、未实现、当前目标不适用或无合格证据均记零，不得删除该模块行或以 N/A 缩小模块分母。单项 requirement/evidence 可记有依据的 N/A，但 coverage row 仍保留。每行至少记录配置子集与不支持项、场景/依赖、适用需求/ECUC/SWS、ARXML/schema/跨文件/往返、生成文件/API/类型/callback 闭包、构建/链接/适用静态质量、独立预期正向/边界/拒绝/恢复、该 target/toolchain 实证。标记 `planned / implemented / evidence pending / failed / reviewed pass`；未支持或没有合格证据即该格计零。H、M1、M2 不得互借证据。
4. **不计算 scalar overall%。** 在所有源确认的 VALID module families、A.1/Cdd/SchM 边界、DRAFT/OBSOLETE 状态、M1/M2 目标及 profiles 均已固定之前，overall 分子/分母/25% 一律留空。完成固定后先分别报告 H、M1、M2 覆盖与未完成清单，并公布每轴原始格数、独立生命周期排除项及边界；当前 VALID 模块行不得按 target/profile 的“不适用”排除。仅当管理层再定义跨目标的聚合规则后才可产生 overall 单值，不能暗含按目标/模块加权或用 H 覆盖掩盖 M1/M2 缺口。不要用文档数、cluster 数、代码行数或任意难度权重。

## 完成此盘点所需核查步骤（留作数据收集方法）

- 以 Overview §3/Table 3.1（pp.19–27）建立全部发布文档及其 `Long Name`、实际 `File Name`、文档类型、cluster、lifecycle-change 标记索引；§3 条目只能当 lead，在映射成每个 A.2/A.1 runtime candidate 的准确规范文件 ID 前不冻结分母。
- 与 BSW General Appendix A Tables A.1–A.3（pp.95–99）的 libraries、A.2 module names/abbreviations/IDs/layers、A.3 special files 双向对账；先裁定 A.1 库与 Cdd/SchM 的可执行覆盖单位定义，记录重复/别名裁决，再按决策固定分母。
- 对每个 A.2/A.1 候选，读取对应 CP R24-11 SWS 实际文件及元数据的状态、known limitations/change history；检查适用 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 中相应 `DocStatus` / `DocStatusDraftExtension`，但不得将文档 `published` 等同模块状态。检查规范中 DRAFT/OBSOLETE 项并精确登记其对象范围。每个候选行必须填确切规范 source/file ID；若无独立 SWS，写明官方来源与分类决定，不拼造 `AUTOSAR_CP_SWS_<name>` 文件名。
- 逐家族记录 SWS 生命周期证据与 Overview §4/Table 4.1 限制，核实完整的当前 VALID runtime module set；任何不支持的有效模块仍保留在每个适用 target 轴的整体目标中且记零。状态未知时分母仍未闭合。限制须进入受影响 profile。
- 将 README 用户场景填到具体模块/target/profile 行；“代码存在”“ARXML 写出模块容器”“能编译”不得作为 H/M1/M2 behavior/evidence pass。RTE 主机行为只能按实际固定 runtime 与 generic signal adapter 描述，不能写成 SWC-specific RTE generation。

## 直接来源定位

- [CP R24-11 Release Overview §3/Table 3.1, pp.19–27](../../../docs/official/R24-11/CP/ReleaseDocumentation/AUTOSAR_CP_TR_ReleaseOverview.pdf#page=19)：19 clusters；混合规格/模型/指南清单；§3 p.25 标 `AUTOSAR_CP_SWS_EEPROMDriver`、`AUTOSAR_CP_SWS_FlashDriver` obsolete；`AUTOSAR_CP_RS_Features` 亦 obsolete 但为 RS。
- [BSW General Appendix A, Tables A.1–A.3, pp.95–99](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf#page=95)：官方库、BSW module 与特殊文件目录。A.1/A.2/A.3 在本地 PDF 抽取文本 lines 2894–3074。
- [R24-11 Specifications ARXML ZIP](../../../docs/official/R24-11/CP/ReleaseDocumentation/AUTOSAR_CP_MOD_SpecificationsARXML.zip)：官方模型化规范元数据，本地忽略；逐规范检查文档状态字段时使用。`DocStatus=published` 只作为文档状态来源。
- [`README.md`](../../../README.md#L1)：当前产品行为、范围与非声明。
- [首个受支持档案 `.scratch/autosar-platform/issues/05-first-verified-profile.md`](../issues/05-first-verified-profile.md#L11)：目标 profile 边界。
- [内部验收门槛 `.scratch/autosar-platform/issues/07-assurance-gates.md`](../issues/07-assurance-gates.md#L34)：按 module/function × configuration × target/toolchain 分档的证据要求。
- [扩展路线 `.scratch/autosar-platform/issues/08-expansion-route.md`](../issues/08-expansion-route.md#L20)：模块级适用矩阵、VALID/DRAFT/OBSOLETE 分列、逐 profile 验收路线。
