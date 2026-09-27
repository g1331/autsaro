# DIAG-003：单次 0x22 请求读取多个 DID

**状态：**done（2026-09-27 独立复核通过）。**分支：**`feature/DIAG-003-multi-did-read`。**基线：**`5bdb5bf`。**依赖：**DIAG-002。**关联能力：**HOST-DOCAN-01。**类型：**新增生成 ECU 的可观察诊断行为。

## 范围与依据

CP/FO R24-11、无变体、Windows 主机虚拟 ECU、MinGW GCC。沿用一条物理 DoCAN 连接、一个配置的 32-bit Tx 信号 DID 和内部 `0xF186`，扩展 0x22 为每次请求读取一个或多个 DID，不新增配置项或其他 DID 数据源。R24-11 [Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) §7.4.2.6：`SWS_Dcm_00253` 定义 0x22 服务；`SWS_Dcm_00438` 和 `SWS_Dcm_00434` 要求逐个检查支持性与当前会话，全部不可读时返回 NRC 0x31。可选的 `DcmDspMaxDidToRead` `[ECUC_Dcm_00638]` 在本固定主机配置中未设置，因此只受现有 256 字节 N-SDU 容量限制；不声明完整 Dcm ECUC/ABI 或 ISO 一致性。

## 行为与拒绝

- 按请求顺序拼接可读 DID 的编号与值；不支持的 DID 被跳过。默认会话只返回 `0xF186`，扩展会话还可返回配置的实时 DID。相同 DID 重复请求时按出现次数返回。
- 请求缺少完整 DID 或字节数不成对返回 NRC 0x13；全部不可读返回 NRC 0x31；读取回调失败返回 NRC 0x22；预计响应超过 256 字节时返回 NRC 0x14，不发送截断的正响应。错误后下一条完整请求可继续处理。
- 不新增 DID 配置、多连接、功能寻址、第三方 Dcm/CanTp 互操作、实机或完整标准接口。

## 验收

从配置 → 保存 → 重开/XSD → 生成 → GCC → 主机诊断报文验证，独立预期覆盖默认与扩展会话、两个顺序、重复 DID、多帧流控、未知 DID、格式错误、容量拒绝及错误后恢复。工作台独立诊断测试器覆盖组合读取与拒绝。运行 `python scripts/workflow.py verify --scope all`、`check`；新 Agent 会话独立复核后再本地集成。原生桌面验收只在隔离环境执行；未运行时标明。六道证据门及支持等级不因本卡自动升级。

## 实际交付

生成 ECU 现可在同一次 0x22 请求中按顺序返回配置的实时 DID 和内部会话 DID；主机测试器同时验证组合读取、多帧流控与关键拒绝。完整门禁最终通过（工作流 8/8、核心单元 2/2、端到端 49/49、UI/Tauri 构建），Agent `diag003_review` 独立只读复核未发现阻断。实际向量和初次失败的处理见 [DIAG-003 证据](../evidence/DIAG-003-multi-did-read-2026-09-27.md)。原生窗口、恰好 256 字节的肯定响应、多 DID 回调失败、完整 BSW 静态义务、第三方互操作与实机仍未验证；六道门及支持声明不升级。

下一切片建议按现有诊断路线界定 0x28 普通通信控制：先落实 ComM/BswM 状态与诊断链隔离的主机配置语义，再做暂停/恢复普通 CAN 通信的可运行正反向验证。本卡不预先授权完整 ComM/BswM 模块或实机支持。本轮无维护者决策阻断。
