# AUDIT-002：主机 DoCAN 档案拆分与基础连接审计

**状态：**done（2026-09-26 独立审查通过）。**分支：**`audit/AUDIT-002-docan-baseline`。**基线：**`dc8d0ef8c559`。**依赖：**AUDIT-001。**关联能力：**HOST-DOCAN-01。**类型：**证据审计，不升级支持声明。[实际证据与审查](../evidence/AUDIT-002-docan-baseline-2026-09-26.md)。

## 本卡的确定范围

审计 CP/FO R24-11、Windows/GCC 虚拟 ECU 的一条标准 11 位 Classical CAN 物理连接：不同的请求/响应 ID，S3 ≥ 5000 ms、正 N_Bs/N_Cr，一个由 1–8 个 32-bit Tx Com 信号组成的扩展会话 DID；仅 `0x10` 默认/扩展会话、`0x3E` TesterPresent、`0x22` 读取 DID，以及对应的单帧/多帧 CanTp 路径。本卡选用 2 个信号、`0x700/0x708`、S3=5000 ms、N_Bs=N_Cr=200 ms、DID=`0x1234`；无写入、DTC、例程或安全访问，不选配置变体。

输入来自工作台新建并保存的 ARXML，使用本地 R24-11 `AUTOSAR_00053.xsd` 与 ECUC MOD；宿主目标是仓库 `runtime/` 的虚拟 ECU，不声称第三方 Dcm/CanTp/CanIf ABI、标准模块完整性、ISO 一致性、实机或安全等级。适用结构重点为 MOD 的 CanTp Rx/Tx N-SDU、N-PDU/FC 引用及 Dcm DSL/DSD/DSP DID 引用；行为核对 R24-11 CAN Transport Layer SWS 的分段、流控、序号与 N_Bs/N_Cr，以及 Dcm SWS 的会话、TesterPresent、ReadDataByIdentifier 局部行为。具体可定位条款与不适用理由写入证据，不把局部行为当整个服务符合性。

## 验收与拒绝

1. 重开所保存的 ARXML，核对 XSD、引用、DID 信号顺序与无改动保留；对冲突 CAN ID、未支持的 CanTp padding、旧系统 PDU 引用/错误动态长度等输入，确认拒绝生成且不创建输出目录。
2. 比较重复生成的完整文件清单及内容，从独立输出目录以 C99 编译链接；核对读回调和主机调用链，不从构建成功推断第三方模块或 MISRA 通过。
3. 用独立预期的 `0x22` 载荷核对默认会话 NRC、扩展会话正响应、多帧/FC、TesterPresent 抑制、N_Bs/N_Cr 超时、错误序号、恢复与 S3 回退；分别标明自动测试和真实界面是否执行。
4. 按六门记录执行、缺口与拒绝，明确其他选项的独立档案。证据由新 Agent 会话只读复核；审计可在支持声明仍未通过时收口。

## 不共用本卡结论的选项

`0x2E` 易失写入、依赖写入的 `0x31/0x01`、单 DTC 的 `0x19/0x14/0x85` 与 NvM、依赖写入或 DTC 的 `0x27` 分别改变回调、状态、权限与失败路径。组合配置（例如 DTC＋写入或安全＋例程）还须按实际选择单独验收；本卡的基础连接证据不能传递给这些门。优先续作单 DTC 持久化档案，因为故障状态损坏、写入失败和重启恢复需要独立检查。

| 档案 | 在基础连接上启用的选项 | 独有的关键证据 |
| --- | --- | --- |
| `HOST-DOCAN-01` | 无 | 只读 DID、会话与单/多帧传输 |
| `HOST-DOCAN-WRITE-01` | `0x2E` | 写回调、CAN 即时值、重启后易失性 |
| `HOST-DOCAN-ROUTINE-01` | `0x2E`＋`0x31/0x01` | 工具 SDG 与 RID/会话拒绝、初值恢复 |
| `HOST-DOCAN-DTC-01` | 单 DTC | `0x19/0x14/0x85`、NvM 损坏与重启 |
| `HOST-DOCAN-SEC-WRITE-01` | `0x2E`＋`0x27` | 密钥及失败状态、延时与写入权限 |
| `HOST-DOCAN-SEC-DTC-01` | 单 DTC＋`0x27` | 安全状态和 DTC 状态各自的持久化与拒绝 |

这些是明确选定的档案，不是全部配置组合的覆盖声明；同时启用多项选项、改变 DID 信号数或选用其他目标时需另记录该组合的证据。

## 收口

固定两信号基础连接的 ARXML、完整再生成清单、GCC 构建与报文/故障恢复得到可复核证据；审查者 Agent `audit002_review` 核对源码、测试和官方条款位置并复跑定向用例，允许以审计结论收口。`HOST-DOCAN-01` 仍是 `documented_behavior`，六门均未标通过；原生诊断用户流程、外部多文件输入、逐模块标准工件与静态义务仍缺。下一任务为 [AUDIT-003 单 DTC/NvM 档案](AUDIT-003-host-dtc.md)。
