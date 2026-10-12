# 变更记录

**简体中文** · [English](CHANGELOG.md)

## Unreleased

- 根目录面向读者的文档默认使用英文，完整中文版以 `.zh-CN.md` 后缀保留；更新双向语言入口。

- 项目原创代码与文档采用 Apache-2.0，补齐包元数据及交付工程的 LICENSE/NOTICE；第三方保留原许可。
- 开发直接使用 npm、Cargo 和 uv，移除重复任务编排；工具源码、测试和可选技能维护分开组织。
- 开发 Python 支持 3.12+，离线工具支持 3.11+，不再要求特定补丁版本；前端使用 Vitest。
- 拆分工程、编辑、交付与设置逻辑，保留 IPC 和原子提交边界；草稿变化使旧批次预览失效。
- 资源路径和工具变化产生新的可信身份，旧封存包仍需匹配原生产者，不就地改写。
- 拆分基础、官方对照、原生运行测试，增加前端行为回归。
- 增加贡献指南、基础 CI、专用验收入口和资产维护说明。
- 补齐运行时字节身份所需的 Git 行尾规则。
- 只含一个应用组件的工程改由多组件主线规划；只有服务组件的组合以 `APPLICATION_COMPONENT_MISSING` 拒绝。
- 主线 Dcm 对 DID 数超过所配置 `DcmDspMaxDidToRead` 的 ReadDataByIdentifier 请求回 NRC 0x13（SWS_Dcm_01335）。`Dcm_ConfigType` 新增 `max_did_to_read`（未配置为 0），计划描述记录 `maxDidToRead`。
- 主线 CanTp 超时时报告 `CANTP_E_RX_COM`／`CANTP_E_TX_COM` 运行时错误（SWS_CanTp_00229）；生成的主机目标经 Det runtime callout 接收，并作为 HostBatch 传输失败报告。
- 含一个应用组件、一个 32 位 CAN 接收与发送信号及应用 DID 的主线工程交付 `verification/inputs.json`，`ecu-tool verify` 据此检查参考回显行为。
- 主线计划对大于 255 的 CanIf PDU 句柄以 `CANIF_HANDLE_WIDTH` 拒绝（交付的 `PduIdType` 为 `uint8_t`），不再报告 `CAN_ID_CONFLICT`；缺失的选定 BSW 入口在引用它的模块描述处报告，不再指向无关对象。
- 主线规划以 `SYMBOL_PRODUCER_DUPLICATE` 拒绝遮蔽生成 OS 计数器服务生产者（`OsService_<Counter>_*`、`Rte_Call_OsService_*`）的 runnable 符号。
- **不兼容：**ECU 生成主线 profile 更名为 `singlecore-ecu-v1`（原 `singlecore-multi-swc-v1`），所有内置规则校验的工程都在主线上规划，包括只有一个应用组件的工程。
- **不兼容：**内置模板 `standard-ecu`（原 `standard-ecu-v1`）在主线上创建含 ComM／BswM 模式管理的单应用组件工程；其仅创建一次的应用种子即 `ecu-tool verify` 检查的参考回显应用。
- **不兼容：**以 `epic4-win64-sr-cs-v1` 或 `singlecore-multi-swc-v1` 记录的工作台工程与原生交付包在打开时拒绝，不修改文件；桌面端不再导入 `autosar-ecu-handoff-v1`。其中的 ARXML 源文件可导入新工程。`host-can-v1` 暂时保留。
- 集成面板的 CAN ID 与应用周期编辑也适用于主线工程；Com Tx 主函数与应用共用触发器时，其时基和发送 PDU 周期随应用周期一同修改。

早期开发结果保留在 Git 历史和相关实施工件中；本记录不重新声明历史发行或验收结果。
