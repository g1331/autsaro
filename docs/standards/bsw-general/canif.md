# BSW General：CanIf

本篇属于 [BSW General](README.md)，标记与核算方法见[标准与规范基线 §5](../README.md#5-条款处置与核算)。

`SWS_BSWGeneral` 的全部 150 条对 CanIf 单独处置，标记带 `[CanIf]`。由测试验证的条目见 `core/tests/fixtures/bsw-contracts/canif_contract.c`（`SWS_BSW_00071` Init 结束时进入已初始化状态、`00186` 输入指针为 const、`00219`／`00222` 以 uint8 向 Det 报告运行时错误）与 `canif_parallel.c`（`00134` 独占区只保护模块数据）。“配置”类的依据与 [CanIf](../canif.md) 篇相同，对应参数在生成前被拒绝。

| 分组 | 依据 | 处置 |
| --- | --- | --- |
| 命名与文件 | 模块缩写与实现前缀为 `CanIf`，文件按 `<Mip>[_<Ie>]` 命名，文件名区分大小写且互不只差大小写 | 已核对（源码）[CanIf]：SWS_BSW_00101、00102、00103、00170、00171 |
| 导入与导出 | 只包含所需头文件；`CanIf.h` 只声明标准服务、回调、运行时错误值与生成配置需要的类型 | 已核对（源码）[CanIf]：SWS_BSW_00104、00105 |
| 实现源文件 | `CanIf.c` 包含 `CanIf.h`、`CanIf_MemMap.h` 与 `SchM_CanIf.h`，内部函数与数据为 static；不向 Dem 报告；调用的 Can 服务、LSduR 回调与 Det 服务经 `Can.h`、`LSduR_CanIf.h`、`Det.h` 声明，上层模式与 bus-off 通知经生成配置中的函数指针调用 | 已核对（源码）[CanIf]：SWS_BSW_00004、00060、00005、00006、00007、00008、00010、00129、00130、00131 |
| 中断与硬件 | 不实现 ISR、中断框架或硬件寄存器访问，不依赖微控制器 | 已核对（源码）[CanIf]：SWS_BSW_00003、00017、00181、00016、00021、00019、00018、00043、00068、00137、00167、00041、00065、00182、00179、00188、00119 |
| 配置源文件 | 只有预编译参数，没有 link-time 配置；配置集生成在 `CanIf_PBcfg.c` 的只读 `CanIf_Config` 中并由 `CanIf_Cfg.h` 声明，实现经配置指针访问 | 已核对（生成样本）[CanIf]：SWS_BSW_00013、00014、00015、00063、00184、00056、00057、00160、00117 |
| 实现头文件 | `CanIf.h` 声明全部 API，带 include 保护并直接包含 `Std_Types.h`；模块没有主函数，不是 AUTOSAR Service，不提供 callout | 已核对（源码）[CanIf]：SWS_BSW_00020、00110、00023、00024、00048、00254、00210、00249、00025、00069、00040、00180、00218、00238、00135、00136 |
| 编码约束 | 只用 AUTOSAR 整数类型与 `boolean` 的赋值、返回和相等比较；不直接用编译器关键字；没有编译开关；宏与枚举按 `CANIF_` 大写命名，类型按 `CanIf_<Tn>Type` 命名；没有外部链接的全局量；不调用调度函数，除 SchM 区外不使用 OS 服务 | 已核对（源码）[CanIf]：SWS_BSW_00120、00142、00121、00123、00124、00146、00127、00133、00138、00257、00147 |
| C 标准 | 交付源码与契约夹具以 `-std=c99 -pedantic -Werror` 编译，外部标识符在 31 个字符以内 | 已核对（构建）[CanIf]：SWS_BSW_00234 |
| 配置方式 | 由工具从 ECUC 生成人可读的 C 配置，只提供 VARIANT-PRE-COMPILE，可选功能不实现，对应开关只接受 `false`；参数名沿用 ECUC 定义 | 已核对（配置模型）[CanIf]：SWS_BSW_00116、00029、00126、00237、00157、00183 |
| 配置指针 | 只有生成的启动序列把 `&CanIf_Config` 传给 `CanIf_Init` | 已核对（生成样本）[CanIf]：SWS_BSW_00158 |
| 调度函数 | CanIf 没有调度函数 | 已核对（源码）[CanIf]：SWS_BSW_00037、00153、00154、00156、00054 |
| 初始化与 De-Init | 启动序列在 Can 之后调用一次 `CanIf_Init`；只有已到达该阶段的关闭流程调用一次 `CanIf_DeInit`；DeInit 在区内一次性回到未初始化状态 | 已核对（生成样本）[CanIf]：SWS_BSW_00230、00231、00232、00233；已核对（源码）[CanIf]：SWS_BSW_00072 |
| 回调形式 | CanIf 提供的回调都返回 void，参数由 CanIf SWS 规定 | 已核对（规范原文）[CanIf]：SWS_BSW_00172、00173 |
| 多分区 | 主线只有一个分区 | 已核对（配置模型）[CanIf]：SWS_BSW_00190、00191、00192 |
| 错误分类与专有错误 | 分类定义与文档格式，没有实现义务；实现没有专有错误，运行时错误值按 `CANIF_E_<EN>` 命名 | 已核对（规范原文）[CanIf]：SWS_BSW_00144、00209、00073、00125 |
| 开发错误开关 | `CanIfDevErrorDetect` 只接受 false，实现不检测也不报告开发错误 | 已核对（源码）[CanIf]：SWS_BSW_00042 |
| 生产错误 | CanIf SWS 没有定义生产错误 | 已核对（规范原文）[CanIf]：SWS_BSW_00204、00205、00143、00046、00066、00206、00207、00208 |
| 版本信息 API | `CanIfVersionInfoApi` 在预编译期配置、只接受默认值 `false` | 已核对（配置模型）[CanIf]：SWS_BSW_00051、00236；不适用（配置：ECUC_CanIf_00613=true）[CanIf]：SWS_BSW_00064、00052、00164 |
| 初始化调用方 |  | 不适用（归属：EcuM 角色——主线没有 EcuM，生成的 `Ecu_StackBootstrap` 承担启动与关闭顺序）[CanIf]：SWS_BSW_00150、00152 |
| 跨模块版本检查与发布信息 |  | 差距 #59[CanIf]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA |  | 差距 #58[CanIf]：SWS_BSW_00115 |
| BSWMD 与模块文档 | 包括独占区说明 | 差距 #159[CanIf]：SWS_BSW_00001、00002、00038、00061 |
| Symbolic Name Value | CanIf_Cfg.h 唯一导出所选配置符号；生成配置从实际提供方的配置头导入，symbolic_handles_compile_link_and_run_renamed_nondefault_mapping 编译、链接并运行。已核对（生成头与真实消费者）[CanIf]：SWS_BSW_00200 |
| 开发错误 |  | 不适用（配置：ECUC_CanIf_00614=true）[CanIf]：SWS_BSW_00255、00201、00203、00045、00243、00049、00212、00050 |
| 安全事件 |  | 不适用（配置：ECUC_CanIf_00848=true）[CanIf]：SWS_BSW_00244、00250、00251、00252、00253、00258 |
| metadata |  | 不适用（配置：ECUC_CanIf_00824）[CanIf]：SWS_BSW_00239、00240、00241、00242 |
| post-build 多变体 |  | 不适用（配置：ECUC_CanIf_00244 声明 VARIANT-POST-BUILD）[CanIf]：SWS_BSW_00226、00227、00228 |
