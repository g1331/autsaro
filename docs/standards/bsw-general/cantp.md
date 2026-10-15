# BSW General：CanTp

本篇属于 [BSW General](README.md)，标记与核算方法见[标准与规范基线 §5](../README.md#5-条款处置与核算)。

`SWS_BSWGeneral` 的全部 150 条对 CanTp 单独处置，标记带 `[CanTp]`。由测试验证的条目见 `core/tests/fixtures/bsw-contracts/cantp_parallel.c`（`SWS_BSW_00134` 独占区只保护模块数据）。“配置”类的依据与 [CanTp](../cantp.md) 篇相同，对应参数在生成前被拒绝。

| 分组 | 依据 | 处置 |
| --- | --- | --- |
| 命名与文件 | 模块缩写与实现前缀为 `CanTp`，文件按 `<Mip>[_<Ie>]` 命名，文件名区分大小写且互不只差大小写 | 已核对（源码）[CanTp]：SWS_BSW_00101、00102、00103、00170、00171 |
| 导入与导出 | 只包含所需头文件；`CanTp.h` 只导出调用方与生成配置需要的类型和声明 | 已核对（源码）[CanTp]：SWS_BSW_00104、00105 |
| 实现源文件 | `CanTp.c` 包含 `CanTp.h`、`CanTp_MemMap.h` 与 `SchM_CanTp.h`，内部函数与数据为 static；不向 Dem 报告；经 `PduR_CanTp.h`、`LSduR_CanTp.h` 与 `Det.h` 调用相邻模块 | 已核对（源码）[CanTp]：SWS_BSW_00004、00060、00005、00006、00007、00008、00010、00129、00130、00131 |
| 中断与硬件 | 不实现 ISR、中断框架或硬件寄存器访问，不依赖微控制器 | 已核对（源码）[CanTp]：SWS_BSW_00003、00017、00181、00016、00021、00019、00018、00043、00068、00137、00167、00041、00065、00182、00179、00188、00119 |
| 配置源文件 | 只有预编译参数，没有 link-time 或 post-build 配置源文件；配置生成在 `Ecu_RuntimeConfig.c` 的只读 `tp_config` 中，实现经配置指针访问 | 已核对（生成样本）[CanTp]：SWS_BSW_00013、00014、00015、00063、00184、00056、00057、00160、00117 |
| 实现头文件 | `CanTp.h` 声明全部 API，带 include 保护并直接包含 `Std_Types.h`；主函数声明在 `SchM_CanTp.h`；模块不是 AUTOSAR Service，不调用 RTE，不提供 callout | 已核对（源码）[CanTp]：SWS_BSW_00020、00110、00023、00024、00048、00254、00210、00249、00025、00069、00040、00180、00218、00238、00135、00136 |
| 编码约束 | 只用 AUTOSAR 整数类型与 `boolean` 的赋值、返回和相等比较；不直接用编译器关键字；没有编译开关；宏按 `CANTP_` 大写命名，类型按 `CanTp_<Tn>Type` 命名；没有外部链接的全局量；不调用调度函数，除 SchM 区外不使用 OS 服务 | 已核对（源码）[CanTp]：SWS_BSW_00120、00142、00121、00123、00124、00146、00127、00133、00138、00257、00147 |
| 输入指针 | 公开 API 的输入指针都带 `const` | 已核对（源码）[CanTp]：SWS_BSW_00186 |
| C 标准 | 交付源码与契约夹具以 `-std=c99 -pedantic -Werror` 编译，外部标识符在 31 个字符以内 | 已核对（构建）[CanTp]：SWS_BSW_00234 |
| 配置方式 | 由工具从 ECUC 生成人可读的 C 配置，只提供 VARIANT-PRE-COMPILE，可选功能不实现，对应开关只接受 `false`；参数名沿用 ECUC 定义 | 已核对（配置模型）[CanTp]：SWS_BSW_00116、00029、00126、00237、00157、00183 |
| 配置指针 | 只有生成的启动序列把 `&tp_config` 传给 `CanTp_Init` | 已核对（生成样本）[CanTp]：SWS_BSW_00158 |
| 调度函数 | 唯一的 `CanTp_MainFunction` 无参数、返回 void，不进入等待；未初始化时不推进计时，状态已清空，不调用其他模块 | 已核对（生成样本）[CanTp]：SWS_BSW_00153、00154、00156；已核对（源码）[CanTp]：SWS_BSW_00037 |
| 初始化与 De-Init | 启动序列在第 7 阶段调用一次 `CanTp_Init`；只有已到达该阶段的关闭流程调用一次 De-Init 函数 `CanTp_Shutdown`；两者都在区内一次性设定配置指针与连接状态 | 已核对（生成样本）[CanTp]：SWS_BSW_00230、00231、00232、00233；已核对（源码）[CanTp]：SWS_BSW_00071、00072 |
| 回调形式 | `CanTp_RxIndication` 与 `CanTp_TxConfirmation` 返回 void | 已核对（源码）[CanTp]：SWS_BSW_00172、00173 |
| 多分区 | 主线只有一个分区 | 已核对（配置模型）[CanTp]：SWS_BSW_00190、00191、00192 |
| 错误分类与专有错误 | 分类定义与文档格式，没有实现义务；实现没有专有错误 | 已核对（规范原文）[CanTp]：SWS_BSW_00144、00209、00073、00125 |
| 开发错误开关 | `CanTpDevErrorDetect` 只接受默认值 `false`，实现不检测也不报告开发错误 | 已核对（配置模型）[CanTp]：SWS_BSW_00042 |
| 运行时错误 | 超时与协议错误以 uint8 错误值经 `Det_ReportRuntimeError` 报告，在区外调用 | 已核对（源码）[CanTp]：SWS_BSW_00219、00222 |
| 版本信息 API | `CanTpVersionInfoApi` 在预编译期配置、只接受默认值 `false` | 已核对（配置模型）[CanTp]：SWS_BSW_00051、00236；不适用（配置：ECUC_CanTp_00283=true）[CanTp]：SWS_BSW_00064、00052、00164 |
| 安全事件开关 | `CanTpEnableSecurityEventReporting` 位于 `CanTpGeneral`，只接受默认值 `false` | 已核对（配置模型）[CanTp]：SWS_BSW_00250 |
| 初始化调用方 |  | 不适用（归属：EcuM 角色——主线没有 EcuM，生成的 `Ecu_StackBootstrap` 承担启动与关闭顺序）[CanTp]：SWS_BSW_00150、00152 |
| 跨模块版本检查与发布信息 |  | 差距 #59[CanTp]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA |  | 差距 #58[CanTp]：SWS_BSW_00115 |
| BSWMD 与模块文档 | 包括独占区与主函数调用顺序的说明 | 差距 #159[CanTp]：SWS_BSW_00001、00002、00038、00061、00054 |
| Symbolic Name Value |  | 差距 #160[CanTp]：SWS_BSW_00200 |
| 开发错误 |  | 不适用（配置：ECUC_CanTp_00239=true）[CanTp]：SWS_BSW_00255、00201、00203、00045、00243、00049、00212、00050 |
| 扩展生产错误 | CanTp SWS 第 7 章按模板记录扩展生产错误；只有配置了 Dem 事件引用时才报告 | 已核对（规范原文）[CanTp]：SWS_BSW_00204；不适用（配置：ECUC_CanTp_00307）[CanTp]：SWS_BSW_00205、00143、00046、00066、00206、00207、00208 |
| 安全事件 |  | 不适用（配置：ECUC_CanTp_00318=true）[CanTp]：SWS_BSW_00244、00251、00252、00253、00258 |
| metadata |  | 不适用（配置：ECUC_CanTp_00303）[CanTp]：SWS_BSW_00239、00240、00241、00242 |
| post-build 多变体 |  | 不适用（配置：ECUC_CanTp_00306 声明 VARIANT-POST-BUILD）[CanTp]：SWS_BSW_00226、00227、00228 |
