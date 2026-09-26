# DIAG-001：读取主机 ECU 支持的 DTC 列表

**状态：**review（实现和完整门禁通过，等待独立审查）。**分支：**`feature/DIAG-001-supported-dtcs`。**基线：**`b4a5a67`。**依赖：**AUDIT-002。**关联能力：**HOST-DOCAN-DTC-01、HOST-DOCAN-SEC-DTC-01。**类型：**新增生成 ECU 的可观察诊断行为。

## 选择与范围

最近 BUILD-001 至 BUILD-004 改善保存、生成及构建交付，没有新增 ECU 运行行为；当前没有已知失败门禁或进行中的任务。本卡按当前深 CAN 诊断路线扩展单 DTC 档案：诊断器可以发现配置的监测项，即使当前状态为零。AUDIT-003 保持 proposed，不因尚未执行的全面证据审计推迟此有界运行增量。

CP/FO R24-11、无配置变体、Windows 主机虚拟 ECU、MinGW GCC。沿用已有单 DTC 与物理 DoCAN 配置及 NvM 格式，新增 `19 0A`（reportSupportedDTC）。配置单 DTC 即启用，默认/扩展会话可读，启用安全档案时无需解锁即可读；不改变已有写操作权限。Dcm/Dem 是当前自有主机子集，不声明标准 BSW ABI、第三方协议栈集成或整个诊断阶段通过。

## 依据与适用性

- R24-11 Dcm SWS §7.4.2.5.2（PDF 页 117–119）、`SWS_Dcm_01645`：0x0A 的状态筛选禁用；须报告受支持 DTC，而非仅状态匹配项。`SWS_Dcm_00828`：未启用分页缓冲时不添加虚构的零填充记录。来源：[官方 Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf)，本地对应 `docs/official/R24-11/CP/Diagnostics/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf`。标准 Dem_SetDTCFilter/GetNextFilteredDTC API 不在本主机接口声明内。
- ECUC 仍使用既有 `DcmDsdService` 的 SID 25 / `DcmDsdSidTabSubfuncAvail=true`、`DcmDspReadDTCInformation`、`DcmDemClientRef`、单个 `DemDTC` 和监测 Rx 绑定；当前工具剖面没有独立可选子功能表。保存/重开/生成须验证这个配置确实控制运行能力，不添加无法解析的新 ECUC 参数。
- 输入闭环、独立预期、C99 编译链接、失败恢复是仓库 07 号项目质量门。BSW 文件结构、MemMap、MISRA、BSWMD 及完整标准接口义务维持未验证，不能以本卡编译成功升级支持声明。主机测试不适用 MCU 链接段、电气与 ISR 时序保证。

## 可观察结果与拒绝范围

1. 请求载荷 `19 0A`，响应 `59 0A 7F <DTC 三字节> <当前状态>`，总长度 7，单个 CAN TP 单帧。状态覆盖初始 0x50、监测通过 0x00、超时 0x2F、故障重启 0x6D、清除后 0x50；关闭 DTC 设置期间仍返回已有状态。
2. 读取无状态副作用，不写 NvM；0x19/0x01 和 0x02 的零掩码语义保持零条匹配。按配置 DTC 编号返回，不能硬编码测试编号。
3. 未配置 DTC 返回 NRC 0x11；0x0A 多带状态掩码或缺子功能返回 0x13；不支持的子功能（含 0x8A 抑制位）明确返回 0x12，随后正确请求恢复。
4. 不增加多 DTC、其他诊断子功能、外部 ISO/第三方互操作或实机支持。

## 验收与交接

核心端到端测试从配置 → 保存 → 重开/XSD → 生成 → GCC → 独立字节向量验证，包括清除 DTC 配置后的拒绝。工作台独立诊断测试器加入同一行为检查，既有 UI 入口可用；本轮仅后台验收，不启动用户桌面窗口。执行 `python scripts/workflow.py verify --scope all` 与 `check`；由新的只读 Agent 会话审查源码、规范及运行证据后，决定本地集成。证据记录原生 UI 未验及完整义务未验，不升级六门声明。
