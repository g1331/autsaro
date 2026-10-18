# BSW General：Det

本篇属于 [BSW General](README.md)，按 Det 单独处置 R24-11 的全部 150 条；原文与核算方式见[标准基线](../README.md#5-条款处置与核算)。Det 的配置、通知和失败语义见 [Det](../det.md)，其他模块的标记不替本模块处置。

`DET_STATE` 只保护配置指针和初始化状态。两个报告函数在区内取得不可变配置的快照，在区外调用全部 hooks；Init 重置后在一个临界区中发布新配置。配置对象由调用方持有，旧配置须存活到最后一次使用它的报告结束；并发 Init 不受支持。受控主机的停止和每线程开发报告递归边界均留在私有 `Det_Host` 适配中。

| 分组 | 依据与处置 |
| --- | --- |
| 文件与前缀 | 标准模块缩写和实现前缀均为 Det；实现为 Det.c，公开头为 Det.h，主机接缝为 Det_Host.c／Det_Host.h；生成的 Det_Callouts.c 与 Det_Externals.h 提供已选通知，文件没有大小写歧义。已核对（这些源码与生成路径）[Det]：SWS_BSW_00101–00105、00170、00171 |
| 实现源与包含 | Det.c 包含 Det.h、Det_Host.h、SchM_Det.h 和自身 MemMap；只调用不可变配置中的函数指针，不直接命名集成方函数；生成的 Ecu_RuntimeConfig.c 包含声明该指针目标的 Det_Externals.h，Det_Callouts.c 同时包含其声明头和 ECU 消费者头。模块不调用 Dem。已核对（Det.c、Det_Callouts.c、Ecu_RuntimeConfig.c）[Det]：SWS_BSW_00004–00008、00010、00060、00129–00131 |
| 中断与硬件 | Det.c 不实现 ISR、interrupt frame、寄存器访问或 MCU 初始化，也不调用任何 OS 服务。主机停止策略与 TLS 在适配层中；当前主机报告来自任务或回调，不声明 MCU ISR 可用性。已核对（Det.c 的完整调用路径）[Det]：SWS_BSW_00003、00016–00019、00021、00041、00043、00065、00068、00119、00137、00167、00179、00181、00182、00188 |
| 配置源 | 只接纳预编译变体，det_config 和有界 callout 表是生成源中的只读数据；Det SWS 没有 link-time／post-build 参数，也不支持 post-build variant。已核对（ECUC_Det_00001 与 Ecu_RuntimeConfig.c）[Det]：SWS_BSW_00013–00015、00056、00057、00063、00117、00158、00160、00184、00226–00228 |
| 实现头 | Det.h 自包含，包含 Std_Types 并只公开配置类型和四个标准 API；没有主函数或 BSW C/S runnable 原型，私有主机头不被公开头间接包含。已核对（Det.h）[Det]：SWS_BSW_00020、00024、00048、00110、00210、00249 |
| RTE 服务类型 | DetConfigSet 的非空 DetModule 会在生成前拒绝，没有 SWC 服务类型和端口。 不适用（配置：ECUC_Det_00008）[Det]：SWS_BSW_00023、00147 |
| RTE 调用 | Det 不调用 RTE，也不包含应用头或经 RTE 发布 callback；直接 hook 通过标准函数指针调用。已核对（Det.c 和生成的 Det_Callouts.c）[Det]：SWS_BSW_00025、00069、00180、00218 |
| 已选 callout | Ecu_DetRuntimeCallout 的声明在 Det_Externals.h，定义在独立 Det_Callouts.c；二者使用 DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE，目标文件具有 .text.det.Ecu_DetRuntimeCallout。det_callout.c 通过实际头与 callout 源编译、链接、运行，原生测试检查目标段；条款由测试中的 Verifies[Det] 核算 |
| 编码与类型 | Det.c／Det.h 使用 AUTOSAR uint16、uint8 和 boolean，boolean 只赋值、返回及与 TRUE／FALSE 比较；普通模块没有平台关键字或可选编译分支。平台属性只在 MemMap 映射中，guard 和段选择不属于功能开关。已核对（Det.c、Det.h、Det_MemMap.h）[Det]：SWS_BSW_00120、00121、00123–00125、00127、00142、00146 |
| 指针与 C99 | Init 使用 const 配置指针，两个通知数组也不可修改；标准消费者以 C99／pedantic／Werror 独立编译、链接、执行。模块不调用其他 Main，也不直接调用 OS 服务。已核对（Det.h、Det.c 与 det_contract.c 的独立签名）[Det]：SWS_BSW_00133、00138、00186、00234、00257 |
| 独占区 | 只保护自身配置与状态，hooks 在区外；真实线程 callout 再进入 DET_STATE，不会嵌套持区或丢失通知。条款由 det_parallel.c 中的 Verifies[Det] 核算 |
| 主函数 | Det SWS 没有 MainFunction，Det.c 没有计划执行入口，也不等待其他主函数。已核对（Det API 全集与源码）[Det]：SWS_BSW_00037、00153、00154、00156 |
| 初始化顺序 | Ecu_StackBootstrap 在其他 BSW 之前调用 Det_Init、Det_Start，Init 返回前配置与状态已发布；正常启动一次初始化，没有 Det_DeInit。SWS_Det_00020 特别要求每次 Init 回到初态，独立消费者验证该模块特定义务。已核对（Ecu_StackBootstrap 与 Det_Init）[Det]：SWS_BSW_00071、00072、00230–00233 |
| 初始化调用者 | 主线没有 EcuM，生成的 Ecu_StackBootstrap 承担生命周期角色；测试中的直接 Init 不代表生产初始化入口。 不适用（归属：EcuM／BswM 生命周期角色由生成启动器承担）[Det]：SWS_BSW_00150、00152 |
| 回调提供者 | Det 提供标准报告服务，不向其他模块提供 callback；用户 hooks 的 Std_ReturnType 由 Det SWS 明确规定。已核对（SWS_Det_00009／01001／00181／00184 和 Det.h）[Det]：SWS_BSW_00172、00173 |
| 配置方式与可选功能 | 内置定义严格表达官方 ECUC 参数；实际输入只有 pre-compile，配置为可读 ARXML 和 C。DLT、版本 API、其他 hooks 和 SWC Det 服务均在源对象拒绝，不静默忽略。已核对（builtin::error_tracing、configuration::det 与普通校验）[Det]：SWS_BSW_00029、00116、00126、00157、00183、00237 |
| 多分区 | 主线只接纳一个分区；报告本身仍由真实线程测试并发安全性，不能由单 ECU owner 假定替代。已核对（分区计划与 det_parallel.c）[Det]：SWS_BSW_00190–00192 |
| 模块自身错误 | Det SWS §7.5 只允许 GetVersionInfo 的空指针开发错误，不允许 Det 自己再报告 runtime、production 或 extended production errors；未初始化 runtime 报告按专门条款直接返回，开发报告按专门条款停止。已核对（Det SWS §7.5、00024、00208 和完整报告路径）[Det]：SWS_BSW_00042、00045、00046、00049、00050、00066、00073、00125、00143、00144、00201、00203–00209、00212、00219、00222、00243、00255 |
| 安全事件 | Det SWS 没有安全事件，DetGeneral 没有对应开关，模块也不调用 IdsM。已核对（Det SWS §7.5、全部 Det ECUC 定义和 Det.c）[Det]：SWS_BSW_00244、00250–00253、00258 |
| 版本 API | DetVersionInfoApi 必填、默认 false；目标拒绝 true，不提供 Det_GetVersionInfo。已核对（builtin::error_tracing 与配置正反例）[Det]：SWS_BSW_00051、00236；不适用（配置：ECUC_Det_00003=true）[Det]：SWS_BSW_00052、00064、00164 |
| MetaData | Det 不处理 PDU、不引用 EcuC 全局 PDU 或分配 MetaData；它只按值传递四个标准报告身份。已核对（Det 全部 API 与 ECUC 参数）[Det]：SWS_BSW_00239–00242 |
| 发布版本信息 | 实现仍没有跨模块版本宏检查；身份和版本的共同整改由原 Issue 负责。差距 #59[Det]：SWS_BSW_00036、00059、00161、00162、00256 |
| MISRA | 保留源码分析原始结果；完整授权原文、221 项人工评估与主机 abort 的评估尚未闭合。差距 #58[Det]：SWS_BSW_00115 |
| 完整 BSWMD／模块文档 | 当前 BSW 描述仍未完整表达 Det 的所有入口、DET_STATE、callout 与随包模块说明；无 ModeDeclarationGroup。差距 #159[Det]：SWS_BSW_00001、00002、00038、00040、00054、00061；已核对（Det 没有模式声明或模式接口）[Det]：SWS_BSW_00238 |
| Symbolic Name Value | 所选配置仅有 DetGeneral／DetNotification，没有 DetModule 实例及需消费的 Symbolic Name Value；BSW 发布的固定 ModuleId 不是当前 ECUC 的 DetModuleId 值。导出器识别 DetModuleId，但本配置不生成没有提供值的 Det_Cfg.h，也不虚构配置实例。已核对（配置记录与实际生成头／消费者）[Det]：SWS_BSW_00200 |
