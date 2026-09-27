# STD-001：Windows 主机目标的标准 Can Driver 接口切片

**状态：**active。**分支：**`fix/quality-check-findings`。**基线：**`c4a01b5cd41c`。**关联能力：**HOST-CAN-01。**类型：**标准模块交付的第一项接口与运行切片；不预先升级支持声明。

## 范围与依据

CP/FO R24-11、无配置变体、Windows 主机虚拟 ECU、MinGW GCC；一虚拟 Can 控制器、标准 11 位 Classical CAN、一个 Tx 硬件句柄、DLC 1–8。保留既有双 ECU 信号闭环的可观察报文和拒绝路径，使生成工程的 CanIf→Can 发送链实际使用标准 Can Driver 类型与入口。来源为本地 R24-11 `AUTOSAR_CP_SWS_CANDriver.pdf`：PDF 页 52–55 的 `Can_ConfigType`、`Can_PduType`、`Can_HwHandleType`、控制器状态类型，页 57 的 `SWS_Can_00223`（`Can_Init`），页 61 的 `SWS_Can_00230`（`Can_SetControllerMode`），页 67 的 `SWS_Can_91014`（`Can_GetControllerMode`），页 76 的 `SWS_Can_00233`（`Can_Write`）。通用义务依本地 `AUTOSAR_CP_SWS_BSWGeneral.pdf` 页 22 `SWS_BSW_00006`、页 37 `SWS_BSW_00115` 及仓库 [验收决定](../../assurance/acceptance-policy.md)。实现前逐项核对这些条款的适用条件与类型依赖。

## 可观察结果与拒绝

- 生成工程仍能从配置和 ARXML 往返得到相同的主机 CAN 输出；标准入口负责所声明的初始化、控制器模式和发送，旧的主机 stdin/stdout 桥接只作为目标适配层。
- 拒绝空配置、无效控制器/句柄、错误 CAN ID、DLC、空 PDU/数据指针、不允许的控制器状态和未支持的配置变体，不产生假成功或旧报文。
- 不从本切片推断 CAN FD、真实寄存器/中断/电气、第三方 CanIf 互操作、完整 Can SWS、完整 MISRA 或实机支持。标准接口之外的必需服务、配置工件和回调依赖若未闭合，维持 `documented_behavior` 和未通过的规范证据门。

## 验收与交接

核对官方 SWS 和 ECUC MOD 中此配置的 API、类型、状态、错误和生成工件；建立独立预期的正向/拒绝向量，运行生成 C99 工程与双 ECU 闭环，证明调用链确实经过新入口。运行 `python scripts/workflow.py verify --scope all`、全量基线及 `check`，记录未满足的 MemMap、BSWMD、MISRA 和其他适用义务。新能力和支持声明变化须由新的 Agent 会话独立复核；未获得完整证据不得标记任务完成或升级能力档案。
