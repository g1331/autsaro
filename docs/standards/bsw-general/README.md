# BSW General

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。`SWS_BSWGeneral` 的义务对每个 BSW 模块成立，按模块分篇处置，标记带模块限定。LSduR、PduR、Com、Can、CanIf、CanTp、Dcm、ComM、BswM、Os、Det 与 Rte 已逐条处置；其余模块下表列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_BSWGeneral.pdf`（`4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_BSW_00006`、`SWS_BSW_00115`、`[SWS_BSW_00103]`、`[SWS_BSW_00170]`／`[SWS_BSW_00171]`、`SWS_BSW_00036`；附录 A.2 模块 ID | 实现源包含 `<Mip>_MemMap.h`；文件名区分大小写且不能只靠大小写区分；跨模块头文件版本检查；模块 ID：Can 80、CanIf 60、CanTp 35、Com 50、ComM 12、Dcm 53、PduR 51、Rte 2、Os 1、BswM 42、Det 15、CDD 255，LSduR 132 |

## 按模块处置

| 模块 | 处置 |
| --- | --- |
| [LSduR 与 PduR](lsdur-pdur.md) | 全部 150 条，标记带 `[LSduR]`／`[PduR]` |
| [Com](com.md) | 全部 150 条，标记带 `[Com]` |
| [Can](can.md) | 全部 150 条，标记带 `[Can]` |
| [CanIf](canif.md) | 全部 150 条，标记带 `[CanIf]` |
| [CanTp](cantp.md) | 全部 150 条，标记带 `[CanTp]` |
| [Dcm](dcm.md) | 全部 150 条，标记带 `[Dcm]` |
| [ComM](comm.md) | 全部 150 条，标记带 `[ComM]` |
| [BswM](bswm.md) | 全部 150 条，标记带 `[BswM]` |
| [Os](os.md) | 全部 150 条，标记带 `[Os]` |
| [Det](det.md) | 全部 150 条，标记带 `[Det]` |
| [Rte](rte.md) | 全部 150 条，标记带 `[Rte]` |

## 已知差距

- **`SWS_BSW_00036` 跨模块头文件版本检查**：当前静态／生成公共头取得所属 owner 的 release，适用直接外部头逐边检查 major/minor；独立编译拒绝见 `core/tests/native/integration/release_contracts.rs`。正式身份与完整发布信息仍归 #59。
- **当前交付说明**：选定实现的 Host_Implementation.arxml 从实际交付公开头和不可变计划描述入口、独占区及调度；MODULES.md 导航逐模块手册、API 与同源配置说明。服务端口支持边界仍由 #187／#195 跟踪，版本身份仍由 #59 跟踪。
- **Symbolic Name Value**：提供方的配置头按 TPS_ECUC_02108 唯一导出所选句柄，生成配置包含提供方头并使用这些符号；LSduR 保持由路由计划合成的配置边界。

## Det 报告身份

`runtime/bsw/include/` 的各模块主头提供 `<MIP>_MODULE_ID`，取值按 R24-11 `SWS_BSW_00256` 与附录 A.2 独立核对。当前报告路径为 Can、CanIf 与 CanTp；服务 ID 在各自主头按原 API 表命名：

| 模块／路径 | 服务宏后缀和值 | 原文 API 表 |
| --- | --- | --- |
| Can 丢帧轮询 | `SID_MAIN_FUNCTION_READ=0x08` | `SWS_Can_00226` |
| CanIf 发送拒绝 | `SID_TRANSMIT=0x49` | `SWS_CANIF_00005` |
| CanIf 接收 DLC 拒绝 | `SID_RX_INDICATION=0x14` | `SWS_CANIF_00006` |
| CanTp 定时处理 | `SID_MAIN_FUNCTION=0x06` | `SWS_CanTp_00213` |
| CanTp 接收处理 | `SID_RX_INDICATION=0x42` | `SWS_CanTp_00214` |
| CanTp 取消发送／接收 | `SID_CANCEL_TRANSMIT=0x4a`／`SID_CANCEL_RECEIVE=0x4c` | `SWS_CanTp_00246`／`SWS_CanTp_00257` |

CanTp 的内部 `CanTp_Report` 保留服务参数与零错误过滤；Det 入口签名、通知与终止行为不变。`bsw-contracts` 消费者按原文数值断言头宏与实际通知，预期不引用生产宏。宏为增量公开信息，不改变函数 ABI、配置或类型布局；重新生成交付包以取得新头。完整 published information、正式 vendor 身份与 API 出口仍由 #59 承接。

## 发布版次与头文件 owner

R24-11 MOD 中 Arti、BswM、Can、CanIf、CanTp、Com、ComM、Dcm、Det、LSduR、MemMap、Os、PduR、Rte 的 `ECUC-MODULE-DEF/ADMIN-DATA/DOC-REVISIONS/DOC-REVISION/REVISION-LABEL` 均为 `4.10.0`。原件 ZIP 身份见标准基线；数字来自这些 revision，不能从 R24-11 的日期或 Cargo 软件版本换算。共享类型保留既有 `STD_TYPES`、`PLATFORM`、`COMSTACKTYPE` 前缀。

模块主头、callback、Cfg 与私有头从本模块 `_Version.h` 取得 release；RTE 生成的应用契约、`Rte_*` 与 `SchM_*` 属 RTE，`Rte_Dcm_Type.h` 的模式类型仍由 Dcm 提供。所有 `_MemMap.h` 由 Memory Mapping 产生，引用 `MemMap_Version.h`；该身份头的 guard 不包裹 START/STOP 处理。契约包与完整 ECU 使用同一组资产及 emitter，不复制另一组输出常量。host helper、ISO C／平台 API 与 FreeRTOS 保持实际角色，不伪造 AUTOSAR 身份。

00036 的拒绝只比较 major/minor；revision-only 与跨供应方 vendor／SW 差异可接受。Det、LSduR、Platform、Comtype 原文 NA 对 SRS_BSW_00004 的排除保留为消费者边界；其他适用模块仍须检查导入它们的头。独立 TU 首 include、边隔离毒化与真实源编译分别检验发布值、每条检查的诊断及编译闭包。ComM_00418 的源／头 release 与 software 三分量分别校验；源端保留独立快照，头单改／缺失拒绝，同步升级成功。正式身份仍由 #59 承接。

## 软件发行版本

自有 BSW、OS、共享类型、MemMap 与 RTE 生成器实现统一发行，初值 **0.1.0**；唯一数值来源是交付的 `runtime/bsw/include/Autsaro_Version.h`。各所属头通过既有 owner 头发布 `<MIP>_SW_MAJOR_VERSION`／`SW_MINOR_VERSION`／`SW_PATCH_VERSION`，Rust 构建读取同一头供 BSWMD 和模块手册使用。Cargo 包版本独立，不再隐式决定 BSW 版本。这个共享头不是另一个 AUTOSAR 模块，不声明 module/vendor ID，也不产生跨模块软件版本相等义务。

版本由维护者在发行变更中更新，按 `SWS_BSW_00162`：双向兼容修复递增 patch、向下兼容扩展递增 minor、不兼容接口变化递增 major；递增高位时低位归零。三个分量均在 uint8 范围内（`SWS_BSW_00256`），构建拒绝缺失、重复或越界的 owner 字段。初值保留已有 Can 与实现描述的声明；它不表示原型已符合或已经稳定。ComM 源内的软件快照随发行同步更新，不能替换为头宏别名，否则 `SWS_ComM_00418` 比较成为恒真。

ARTI 工具实现独立发行，初值 **1.0.0**，唯一 owner 为 `Arti_Version.h`；API、BSWMD 和手册消费该来源。RTE BSWMD／手册的配置版本仍按 `SWS_Rte_05184/05185/05186` 来自输入 DOC-REVISION，第四段只用于文档；头中生成器实现版本不是该配置版本。SchM build-version 的双输入语义见 [SchM](../schm.md)。

正常测试在 `descriptions.rs` 检查独立字面量的 BSWMD／manual，在 `release_contracts.rs` 编译所有实际公共头／重复 MemMap、逐分量破坏 ComM 一致性、接受同步升级，并运行 ARTI 哨兵消费者及逐赋值删除反例。版本 owner 的传递测试允许合法同步升级，不把固定初值当不可升级的协议。

这里只闭合软件发布来源；`SWS_BSW_00059/00256/00161` 的正式 vendor 部分仍欠缺，保留原有差距标记。项目尚未申请 vendor ID，不添加虚构 VENDOR_ID，不启用依赖真实身份的版本 API，不替代 RteCodeVendorId 的使用方身份。
