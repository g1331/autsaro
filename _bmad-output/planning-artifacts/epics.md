# 首阶段 epics 与 stories

## Epic 1: 主机 CAN 标准接口与可运行信号链

用户价值：生成的虚拟 ECU 经明确的 CanIf→Can 接口收发 11 位 Classical CAN，遇到无效输入或状态时给出真实拒绝。对应 PRD R2、R3、R4；限定 R24-11、一个虚拟控制器、一个 Tx 句柄、DLC 1–8、无变体。阶段出口以双 ECU 信号闭环、无效请求拒绝、独立构建和未满足义务的准确标注判断。

### Story 1.1: Complete STD-001 host Can Driver slice

承接[在途任务卡](../../docs/workflow/tasks/STD-001-host-can-driver-api.md)与[已有证据](../../docs/workflow/evidence/STD-001-host-can-driver-api-2026-09-27.md)，迁移时保持 in-progress，不重复实现已提交代码。核对 R24-11 `AUTOSAR_CP_SWS_CANDriver.pdf` 的 `SWS_Can_00223`、`SWS_Can_00230`、`SWS_Can_91014`、`SWS_Can_00233`，及 `AUTOSAR_CP_SWS_BSWGeneral.pdf` 的 `SWS_BSW_00006`、`SWS_BSW_00115`；ECUC/MOD 依赖及其他适用义务按任务卡补齐。

验收：生成工程的 CanIf→Can 发送确实进入标准入口并保留可观察报文；正向双 ECU 结果与独立预期一致；空配置、错误句柄/ID/DLC、空 PDU 或数据、非法状态及影响生成的变体被拒绝；运行增量质量门、记录基线红项和未验证范围；新的独立复核结论写入 story 记录。未完成门禁不得升级 `HOST-CAN-01` 声明。

## Epic 2: 有界诊断故障记忆的下一项运行能力

用户价值：从现有单 DTC 主机档案扩展为两个可区分的故障状态，诊断请求能分别读出并清除，重启后的主机存储结果可重现。对应 PRD R2、R3、R4；仅沿既有 11 位物理 DoCAN 和主机文件 NvM 实现，第三方 Dem/NvM 与真实 Flash 不在本 epic 声明内。Epic 1 无需依赖 Epic 2；对应 PRD 中的深入 CAN 诊断方向。

### Story 2.1: Independent host state and diagnostic reporting for two DTCs

在既有单 DTC 配置上扩展到最多两个不同 DTC；分别绑定既有支持的 Rx 监测源，保留每个事件独立的状态、`0x19/0x01` 数量和 `0x19/0x02` 列表行为。先从本地 R24-11 Dem、Dcm、NvM SWS 和 ECUC MOD 定位该子集的适用条款、对象与拒绝条件；若标准语义与当前主机设计冲突，修正此 story 后再编码，不猜测或伪装符合标准。

验收：双 DTC 的配置往返与生成工程闭包成立；独立触发其中一个故障时只改变对应状态，两个故障分别触发时数量与列表正确；重复 DTC、第三个 DTC、无效监测引用及不支持的诊断子功能明确拒绝；运行主机正反向与跨进程检查，记录仍欠缺的规范、界面和硬件证据。先前提出的 `AUDIT-003` 仅作为这项实现所需的证据线索，不单独抢占下一开发项。
