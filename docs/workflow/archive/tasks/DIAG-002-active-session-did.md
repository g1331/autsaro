# DIAG-002：读取当前诊断会话 DID

**状态：**done（2026-09-26 独立审查通过）。**分支：**`feature/DIAG-002-active-session-did`。**基线：**`b359d56bb538`。**依赖：**DIAG-001。**关联能力：**HOST-DOCAN-01。**类型：**新增生成 ECU 的可观察诊断行为。

## 范围与依据

CP/FO R24-11、无变体、Windows 主机虚拟 ECU、MinGW GCC。在现有物理 DoCAN 连接和 Dcm 0x22 服务上实现保留 DID `0xF186`（ActiveDiagnosticSessionDataIdentifier），按当前活动会话返回 `62 F1 86 01` 或 `62 F1 86 03`。R24-11 [Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) §7.4.2.2、`SWS_Dcm_00085`（PDF 页 112）要求 DSP 内部管理该 DID 的读访问；本地规范位于 `docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf`。主机实现限定默认与扩展会话，已有模型禁止将 `0xF186` 配为普通 DID，不新增 ECUC 配置。

## 行为与拒绝

- 初始默认会话、`10 01` 和 S3 超时后返回 `01`；`10 03` 正响应确认后返回 `03`。诊断测试器通过 CAN TP 单帧读取，观察实际活动状态。
- 其他未配置 DID 维持 NRC `0x31`；长度不符维持 NRC `0x13`。普通配置 DID 在默认会话仍按现有权限拒绝。
- 不实现 0x28 通信控制、额外会话、第三方 Dcm ABI、真实硬件或完整标准符合性。配置链沿用已有诊断入口，不能将内部 DID 误表示为用户配置的数据源。

## 验收

配置、保存、重开、XSD/语义检查、生成、C99 编译和独立字节向量覆盖会话进入、主动返回默认、S3 回退、错误 DID 与错误长度；工作台诊断测试器也执行同一检查。运行 `python scripts/workflow.py verify --scope all`、`check`。原生桌面窗口只在隔离环境验证；未执行时如实记录。新 Agent 会话独立复核新能力和证据，再决定本地集成；六道证据门和支持等级不因本卡自动升级。

## 实际交付

生成的主机 ECU 现可通过 `22 F1 86` 读取实际活动会话。独立字节向量覆盖初始、扩展、主动切回、S3 回退和关键拒绝；工作台诊断测试器也覆盖这些路径，并避开与合法普通 DID `0xF187` 的探针碰撞。完整门禁最终通过（工作流 8/8、核心单元 2/2、端到端 48/48、UI/Tauri 构建）；Agent `diag002_review` 两次指出测试器缺口，修复后最终只读复核通过。详见 [DIAG-002 证据](../../../assurance/evidence/DIAG-002-active-session-did-2026-09-26.md)。原生桌面窗口、第三方互操作、实机与完整 BSW/MISRA 仍未验证；六道门和支持等级不升级。

下一运行候选为 0x28 普通通信控制，但须先界定 Dcm 与 ComM/BswM 的状态语义、配置与拒绝范围。本卡未改动该服务，也不据此授权完整模块实现。本轮无维护者决策阻断。
