# 方法论、输入角色与模板

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_TR_Methodology.pdf` | §2.7.2.2.1–2.7.2.2.2（p102–104）；`TR_METH_01114/01116/01117` | ECU 配置输入包含 ECU Extract 与所选 BSW 实现的模块交付信息；每个模块须选择实现 |
| `AUTOSAR_CP_TPS_SystemTemplate.pdf`（`5267a329104c2dcb029b3f713afa376733466a1b225a33498036a2f1ef017777`） | `ECU_EXTRACT`（§14）、`TPS_SYST_01006`；`TPS_SYST_01001`、`constr_5486／5487`；`[constr_3448]` | Extract 是面向目标 ECU 的展开系统描述；SWC→ECU 映射；DataMapping 与 SenderReceiverToSignalMapping 的实例／端口／信号身份；本剖面全局 Pdu 不输出 `DynamicLength` |
| `AUTOSAR_CP_TPS_SoftwareComponentTemplate.pdf`（`ea86a1b4b43c4ec0319b706e8696412ffb920cb6cc6422a911bedc1f0c9d59da`） | §6.3／6.4、`constr_1068／1069` | 变量与 assembly 兼容性、端口归属、runnable／event／access 与同步 operation 关系；组件接口不能从 Com 信号名推断 |
| `AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf`（`786ccab650f516f541e8d29b41f5c00e57ccd3de6ac1cad70db268f3ec7c5c28`） | 表 4.1／4.17；`04016／04017`；`constr4019`、`4020`；`constr_10260–10264`；TPS 5.11 | BSW entry／schedulable entity／exclusive area 与真实符号、task 上下文匹配；implementedEntry／expectedEntry 属于 ModuleDescription；entry 须显式声明 callType、executionContext、reentrant、synchronous、swServiceImplPolicy |
| `AUTOSAR_CP_TPS_ECUConfiguration.pdf` 与 MOD ZIP | ECUC 定义／值、重数、引用、配置类 | 校验 XSD 之外的引用、值与实现约束；厂商差异定义与标准来源分开 |
