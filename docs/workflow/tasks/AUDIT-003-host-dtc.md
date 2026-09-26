# AUDIT-003：主机单 DTC 与 NvM 档案审计

**状态：**proposed。**依赖：**AUDIT-002。**关联能力：**HOST-DOCAN-DTC-01。**类型：**证据审计；不扩展服务或支持声明。前两张审计卡均已收口而支持声明未升级，本卡暂不作为下一项自动领取任务；当 DTC 实现、故障复现或支持声明确需这些证据时再转为 `ready`。

## 确定组合

CP/FO R24-11、Windows/GCC 虚拟 ECU；沿用基础 11 位物理 DoCAN 连接与只读 DID，另配置一个 `0x123456` DTC，绑定带信号的 `0x456` Rx 帧、正超时 50 ms，使用两个槽位的主机文件 NvM。无 `0x2E`、`0x31`、`0x27`，无配置变体。当前实现的 UDS 路径仅含 `0x19/0x01`、`0x19/0x02`、`0x14` 全部清除、`0x85/0x01` 与 `0x85/0x02`；运行时按每次进程启动划分操作周期。其他 DTC 数量、其他服务子功能、真实 Flash/EEPROM、第三方 Dem/NvM 和硬件目标均属拒绝或未验证范围。

结构核对 R24-11 ECUC MOD 中 `/AUTOSAR/EcucDefs/Dem/DemConfigSet/DemDTCAttributes`、`DemDTC`、`DemEventParameter`、`/AUTOSAR/EcucDefs/NvM/NvMBlockDescriptor`、Dcm 的 `DcmDsdService`、`DcmDspControlDTCSetting` 与 `DcmDemClientRef`；行为条款从本地 R24-11 Dem/Dcm/NvM SWS 按所选子功能逐项定位并判断适用性。工具 SDG 绑定监测 Rx 帧不等于标准 Dem 事件源配置，文件槽位也不等于真实 NvM 设备。

## 审计交付

1. 固定并保存可重建的 ARXML/XSD/MOD 输入；重开核对 DTC 编码、监控帧引用与会话规则，拒绝错误监控对象、超出一个 DTC 或缺少必需 ECUC 结构且不改写源文件。
2. 比较完整生成工件、回调与配置闭包，从独立目录 GCC C99 编译链接。记录主机 Dem/NvM 角色及标准 BSWMD、MemMap、MISRA 等未满足项。
3. 用独立预期核对首次有效 Rx 后超时、0x19 状态掩码计数和 DTC 读出、0x85 暂停/恢复、0x14 权限和清除、跨进程保持；注入槽位损坏、配置指纹不匹配、写入失败，确认错误不伪装为空状态或成功。区分自动测试、实际原生界面、外部测试器执行情况。
4. 逐门记录输入、预期、输出、拒绝和未运行项，由新 Agent 会话独立复核。审计可在 `HOST-DOCAN-DTC-01` 仍为 `documented_behavior` 时收口；不从基础连接档案继承通过门。
