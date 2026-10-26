# Det

本篇属于[标准与规范基线](README.md)。按 [ADR 0017](../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_DefaultErrorTracer` 的全部 50 条（含 2 条 obsolete）；编号来自固定 SpecificationsARXML。条款处置不等于完整 AUTOSAR／MISRA 符合或认证；通用义务另按 [BSW General：Det](bsw-general/det.md) 核算。

原文：`AUTOSAR_CP_SWS_DefaultErrorTracer.pdf`，SHA-256：`f5caa8cb8f2b9ef2e2578a96d940abb795fbf7dfc09a1899b0b81797c2835e1f`。接口依据第 20–28 页，VFB 服务依据第 28–31 页，初始化和报告顺序依据第 32–33 页时序图，ECUC 依据第 34–40 页及固定 MOD。

## 已选配置与使用

Det 接收 BSW 的开发错误和运行时错误。`Det_ReportRuntimeError` 把四个身份参数依次传给全部配置 callout，忽略其返回值并返回 `E_OK`；尚未初始化时直接返回 `E_OK`。`Det_ReportError` 调用开发 hooks 后停止执行，尚未初始化时直接停止；受控主机由私有 `Det_HostHalt` 使用平台进程停止，以退出码134终止 ECU，不经过 CRT 清理或用户 SIGABRT handler（平台拒绝停止的 trap 回退按失败退出处理）。每个线程有自己的开发报告递归边界，递归报告直接停止，不重复调用 hooks。

`Det_Init` 重置并选择不可变配置；NULL 表示空 hooks。并发 Init 不受支持，配置对象及 hook 数组须存活到使用它们的最后一次报告完成。运行时和开发报告在 `DET_STATE` 中只读取状态，随后在区外调用 hooks；报告可并发，运行时 callout 可再次报告。当前不记录日志，不提供读记录、故障注入或清空记录等标准外公开 API。`Det_Start` 存在；所选实现没有需要它启动的存储，因而无需额外动作。

`Det_ValidHooks`的R8.2诊断按误报处置，不套用RTE的nameless冲突：定义的两个参数hooks、count均命名，Det_ErrorHookType的四个参数也均命名。依据MISRA主文R8.2（页64–66／PDF72–74）；TC2.10／12与AMD3.63未改变这里的命名义务。固定Cppcheck 2.21.0把const函数指针typedef数组参数展开后错误解析了外层参数；最小反例保留全部名称仍报告，换成普通对象数组参数的相同外层定义不报告，真正无名声明的负对照仍报告。该结论只对应此声明和已核对的typedef，不改类型、不抑制或改写原始报告；复现及实际分析身份留在#58实施PR。

生成器要求显式 Det 模块及 `DetGeneral/DetVersionInfoApi=false`，`DetForwardToDlt` 可省略或为 false。`DetNotification/DetReportRuntimeErrorCallout` 必须恰为 `Ecu_DetRuntimeCallout`，该 callout 把 CanTp 的发送／接收超时送入 HostBatch 收据；其他模块、API 或错误不会改变收据。它在 `generated/include/Det_Externals.h` 声明，在 `generated/src/Det_Callouts.c` 定义，使用独立的 `DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE` 段。

当前静态交付的 `Det_MemMap.h` 固定了这个 callout 的段标记与 `.text.det.Ecu_DetRuntimeCallout`，只适用于上述唯一通知选择。以后放开集成方 callout 或开发 hook 前，须按实际配置生成 MemMap，使原型段标记与函数名称一致、每个 callout 有独立段；依据 R24-11 BSW General §7.1.13 的 00135／00136，影响和退出条件由 [#206](https://github.com/g1331/autsaro/issues/206) 跟踪。该扩展未排期，不改变当前配置下的条款处置。

目标限制在计划层表达，内置 ECUC 定义保留官方类型、多重性、默认值与数值范围。当前拒绝启用 DLT、版本 API、其他 runtime callout 和开发 hook；C 层标准消费者仍可用 `Det_ConfigType` 验证任意有界、不可变的 hook 列表。可选 `DetConfigSet` 可为空，非空 `DetModule` 在生成前拒绝；因此当前没有经 VFB 连接的 SWC Det 服务，也不丢弃其 ModuleId／InstanceId。

## 接口与报告义务

## ECUC 与服务端口

| 内容 | 处置与证据 |
| --- | --- |
| 公共头与配置类型 | `Det.h` 自包含，导入 Std_Types，声明标准 Init／Start／开发报告／运行时报告及实现自定义的不可变配置结构，不包含私有主机头。已核对（Det.h 的四个 API 与 Det_ConfigType）：SWS_Det_00037、SWS_Det_91001、SWS_Det_00210 |
| 初始化、通知与停止 | `det_contract.c` 通过独立的标准函数指针调用 Init／Start／两个报告服务，核对空配置、重复初始化、原身份、先后顺序、全部 hooks、嵌套 runtime 报告、忽略错误返回及真实进程停止。条款由该测试中的 `Verifies:` 核算 |
| 并发与区外调用 | `det_parallel.c` 的两个真实线程各完成 1000 次报告；callout 再进入 DET_STATE，证明通知不在区内。每线程观察计数独立，主线程未收到通知。条款由测试中的 `Verifies:` 核算 |
| callout 契约 | 主线拒绝 DetErrorHook 非空列表，不生成没有绑定实现的开发 hook 原型。 不适用（配置：ECUC_Det_00005）：SWS_Det_00181；runtime callout 使用 uint16／uint8 和 Std_ReturnType，实际生成的声明与定义由 det_callout.c 编译、链接、运行核对。集成方应返回 E_OK，Det 不允许一个异常返回阻止后续通知 |
| 附加功能 | 所选实现除了 hooks 后的停止策略，没有可选日志、错误恢复、DLT 或其他附加处理。已核对（Det.c 的两个报告函数、Det_HostHalt）：SWS_Det_00015 |
| DLT 转发 | 生成前拒绝 DetForwardToDlt=true；false／省略时不导入也不调用 Dlt。 不适用（配置：ECUC_Det_00006=true）：SWS_Det_00034、SWS_Det_91002 |
| 版本查询与空指针错误 | 生成前拒绝 DetVersionInfoApi=true；不提供 Det_GetVersionInfo，也不会产生它的 DET_E_PARAM_POINTER。 不适用（配置：ECUC_Det_00003=true）：SWS_Det_00011、SWS_Det_00052、SWS_Det_00301 |
| 已废弃的 transient fault | 原文及 SpecificationsARXML 都将此服务与 callout 标为 obsolete，主线不提供已废弃接口。 不适用（归属：R24-11 规范维护者已废弃的 transient-fault 服务与 callout）：SWS_Det_01003、SWS_Det_00187 |

## 私有开发报告状态的映射

`reporting_development`只由`Det_ReportError → Det_HostEnterDevelopmentReport`控制开发hooks的同线程递归边界。固定原件的`SWS_Det_00026`要求停止及防止递归，`00039`要求报告可重入；TLS是实现选择，不是SWS规定的设施。这个标记是已选择Det报告实现的数据，不能因其文件位于host目录或BSWMD过滤器漏列`Det_Host.c`就免除模块责任。

旧函数内static TLS虽然具有进程／线程存储期，却没有独立数据映射；外层CODE属性只映射函数。`SWS_MemMap_00003／00018`要求代码及数据按实际类别映射，`00023`（固定MemoryMapping页37）的说明要求需专用映射的函数static移到文件作用域，不能在函数体插入映射include。现将同一个`static __thread boolean`放入完整`DET_START_SEC_VAR_CLEARED_8`／STOP对：受控ELF使用`.tbss.det`，PE使用`.tls$DET`，沿用现有受控TLS目标策略。隐式零初始化、线程独立、首次TRUE／以后FALSE、从不复位及最终退出134保持；不增加对象或运行时状态。

这是实际的R8.9 Advisory文件作用域保留，不能把静态存储期本身当成块作用域不适用的理由。保留理由是该对象的数据映射与00023的函数体限制；普通once guard、把include挪进函数或让工具链隐式选段均不能同时保持该映射协议。现有模块描述补齐私有实现源及CODE、普通cleared、线程cleared和配置callout四类段；默认`Det_MemMap.h`按00033解析，不借另一模块的头冒充自己的默认头。

验证沿现有Det runtime并发入口增加实际私有状态观察：主线程首次进入后，两条真实线程仍各自首次进入成功、重复进入失败，主线程再次进入仍失败；原有runtime通知次数及公开递归／停止场景保持。目标文件另连接精确私有符号与TLS段，头文件的C／C++观察覆盖线程段配对、STOP撤销、嵌套、错配及多请求。结果按PR实际执行记账，不把线程级编译／行为观察外推完整loader／fiber语义或Windows运行。此处尚未增加批准表或MR12，逐处记录和复核仍归#58。

## MemMap 冲突的具体定位

以下技术定位绑定 `declaration-mapping-formatted-samples` 的实际交付字节，尚非批准表。R20.1／R20.5／D4.10 的有效类别、MISRA 修订依据及通用替代排除见 [MemMap 篇](memmap.md#misra-预处理布局)；原始报告保持。普通声明头仍有 once guard，可重入的对象仅是 `Det_MemMap.h` 的分派主体。

| 实际位置 | 实体、请求及必要性 |
| --- | --- |
| `runtime/bsw/include/Det.h`，`#include "Det_MemMap.h"` 第1／2次，行28／80 | Det 配置类型之后的公共 Init、Start、ReportRuntimeError、ReportError 原型；对应 `DET_START_SEC_CODE`／STOP。00003 同时要求声明与定义映射，不能以 Det.c 的属性代替头的协议。 |
| `runtime/bsw/src/Det.c`，同一指令第2–4次，行12／15／104 | 已发布配置指针的 cleared 段关闭，以及 hooks 校验、初始化与报告函数的 CODE 开闭。第一处 include 在本文件对象声明前，不列为 R20.1 违反。 |
| `runtime/host/src/Det_Host.h`，同一指令第2次，行26 | 私有首次／重复报告入口与不可返回的停止函数原型；开始处在本文件声明前，只有关闭处列为 R20.1。host 路径不取消这个 Det 实现接缝的声明责任。 |
| `runtime/host/src/Det_Host.c`，同一指令第2–8次，行21／24／35／39／48／51／58 | 同一线程标记的 VAR_CLEARED_8 关闭、报告入口 CODE 对及分别受 Windows／非 Windows 条件围住的完整停止函数 CODE 对。R20.1 按预处理前布局判断，未选平台分支的物理 include 也不能销项；TLS 开始处在本文件声明之前。 |
| `runtime/ecu/include/Det_Externals.h` → `generated/include/Det_Externals.h`，同一指令第2次，行24；`runtime/ecu/src/Ecu_DetCallout.c` → `generated/src/Det_Callouts.c`，第2次，行18 | `Ecu_DetRuntimeCallout` 的原型和定义分别关闭其独立 `DET_*_ECU_DETRUNTIMECALLOUT_CODE`。定义经 Det 的 runtime hook 到 `Ecu_TargetReportTransportTimeout`，参数筛选无状态；00023 的 callout 说明及 BSW00135 要求独立 callout 类，不改成模块通用函数类。映射来自既有 outputs 资产／交付关联，不能把交付文件名当成 Rust emitter。 |

上述共15个 R20.1 物理报告位置，在已完成 standard-ecu 中均归属 host-batch／probe。`Det_MemMap.h` 第7／15、22／34、41／49、56／64行的八个 START／STOP `#undef` 分别消费普通 cleared、线程 cleared、CODE、指定 callout 请求，直接连接00011；第19／20、38／39、53／54、68／69行的活动状态与属性撤销另按实现控制评估，不把这些内部名字说成 SWS 强制。

D4.10 的具体对象是该头首个 `@file` 及上述请求调用方：只包含一次会吞掉后续 STOP 和新段，失去00003的完整协议；移到函数体又违反00023。非法、多请求、嵌套及错配拒绝、STOP 后属性消失、目标文件 TLS 符号与独立线程首入／重入观察分别控制误配与共享状态风险。`reporting_development` 的 R8.9 保留理由仅限上节的数据映射与函数体限制。原授权来源、完整独立复核及最终注释对应仍须按 ADR0023 连接，此处没有新增 Approved 或 MR12，也不证明所有 provider、平台和配置符合。

## ECUC 与服务端口

| 内容 | 处置与证据 |
| --- | --- |
| 模块、必填通用参数与 runtime 通知 | `det_configuration_selects_the_delivered_runtime_notification` 核对普通校验、计划与实际输出；`det_optional_features_are_refused_at_their_configuration_object` 核对原对象处拒绝。条款由测试中的 `Verifies:`／`Rejects:` 核算 |
| DetForwardToDlt | 标准定义为可选 boolean，无默认值；省略或 false 均接受，true 拒绝。已核对（builtin::error_tracing、configuration::det 与合法／拒绝向量）：ECUC_Det_00006 |
| 开发 hooks | 标准函数名参数保留 0..* 多重性，目标拒绝非空列表；C 配置结构保留按顺序的函数指针数组。已核对（builtin::error_tracing、Det_ErrorHookType 与源位置拒绝测试）：ECUC_Det_00005 |
| SWC 模块与实例描述 | 官方 DetModule 为 0..*，其 ModuleId 必填且范围 4096..65535；DetModuleInstance 为 1..*，InstanceId 必填且范围 0..255、默认 0。内置定义按原文表达，生成目标在 DetModule 对象拒绝整棵有效子树。已核对（builtin::error_tracing 与 fixed_r24_11_mod_agrees_on_ecuc_integer_precision_enum_and_default）：ECUC_Det_00008、ECUC_Det_00009、ECUC_Det_00012、ECUC_Det_00013 |
| VFB 服务、端口及 port-defined arguments | 这些义务属于经 DetConfigSet/DetModule 选择的非 BSW 服务消费者，当前目标在其配置对象拒绝，未用直接 C 报告冒充 RTE 服务。 不适用（配置：ECUC_Det_00008）：SWS_Det_00200、SWS_Det_00202、SWS_Det_00203、SWS_Det_00204、SWS_Det_00205、SWS_Det_00206 |
| 附录的非适用上游需求 | 附录 A 由规范给出的非适用上游需求集合，不是产品接口或实现义务。 不适用（归属：AUTOSAR Det 规范的上游需求映射）：SWS_Det_NA_00999 |

## 验证边界

原生测试检查标准接口、报告身份、通知顺序、重入、真实线程及主机终止；生成的 callout 另以实际公开头编译、链接、运行，并核对其目标文件代码段。受影响的单 SWC／多 SWC、含诊断／无诊断样本仍须运行正常生成、原生回归和 c-check。

C 分析保留原始源码诊断与人工 `not_assessed`。固定 MISRA 原件已可用，锁与 Det 停止适配的 R21.1／R21.8 整改见[运行时架构](../architecture/runtime.md#锁失败与停止策略d-10)，不使用偏离；其余221项及采用代码评估由 #58 跟踪；版本身份由 #59 跟踪；随包 BSWMD 与模块手册从当前生成计划和公开头产生，跨模块 Symbolic Name Value 按实际配置导出。这里的处置不关闭这些共享差距，也不提供真实 MCU／中断、功能安全或认证证据。
