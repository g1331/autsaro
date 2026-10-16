# Det

本篇属于[标准与规范基线](README.md)。按 [ADR 0017](../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_DefaultErrorTracer` 的全部 50 条（含 2 条 obsolete）；编号来自固定 SpecificationsARXML。条款处置不等于完整 AUTOSAR／MISRA 符合或认证；通用义务另按 [BSW General：Det](bsw-general/det.md) 核算。

原文：`AUTOSAR_CP_SWS_DefaultErrorTracer.pdf`，SHA-256：`f5caa8cb8f2b9ef2e2578a96d940abb795fbf7dfc09a1899b0b81797c2835e1f`。接口依据第 20–28 页，VFB 服务依据第 28–31 页，初始化和报告顺序依据第 32–33 页时序图，ECUC 依据第 34–40 页及固定 MOD。

## 已选配置与使用

Det 接收 BSW 的开发错误和运行时错误。`Det_ReportRuntimeError` 把四个身份参数依次传给全部配置 callout，忽略其返回值并返回 `E_OK`；尚未初始化时直接返回 `E_OK`。`Det_ReportError` 调用开发 hooks 后停止执行，尚未初始化时直接停止；受控主机由私有 `Det_HostHalt` 终止 ECU 进程。每个线程有自己的开发报告递归边界，递归报告直接停止，不重复调用 hooks。

`Det_Init` 重置并选择不可变配置；NULL 表示空 hooks。并发 Init 不受支持，配置对象及 hook 数组须存活到使用它们的最后一次报告完成。运行时和开发报告在 `DET_STATE` 中只读取状态，随后在区外调用 hooks；报告可并发，运行时 callout 可再次报告。当前不记录日志，不提供读记录、故障注入或清空记录等标准外公开 API。`Det_Start` 存在；所选实现没有需要它启动的存储，因而无需额外动作。

生成器要求显式 Det 模块及 `DetGeneral/DetVersionInfoApi=false`，`DetForwardToDlt` 可省略或为 false。`DetNotification/DetReportRuntimeErrorCallout` 必须恰为 `Ecu_DetRuntimeCallout`，该 callout 把 CanTp 的发送／接收超时送入 HostBatch 收据；其他模块、API 或错误不会改变收据。它在 `generated/include/Det_Externals.h` 声明，在 `generated/src/Det_Callouts.c` 定义，使用独立的 `DET_START_SEC_ECU_DETRUNTIMECALLOUT_CODE` 段。

当前静态交付的 `Det_MemMap.h` 固定了这个 callout 的段标记与 `.text.det.Ecu_DetRuntimeCallout`，只适用于上述唯一通知选择。以后放开集成方 callout 或开发 hook 前，须按实际配置生成 MemMap，使原型段标记与函数名称一致、每个 callout 有独立段；依据 R24-11 BSW General §7.1.13 的 00135／00136，影响和退出条件由 [#206](https://github.com/g1331/autsaro/issues/206) 跟踪。该扩展未排期，不改变当前配置下的条款处置。

目标限制在计划层表达，内置 ECUC 定义保留官方类型、多重性、默认值与数值范围。当前拒绝启用 DLT、版本 API、其他 runtime callout 和开发 hook；C 层标准消费者仍可用 `Det_ConfigType` 验证任意有界、不可变的 hook 列表。可选 `DetConfigSet` 可为空，非空 `DetModule` 在生成前拒绝；因此当前没有经 VFB 连接的 SWC Det 服务，也不丢弃其 ModuleId／InstanceId。

## 接口与报告义务

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

C 分析保留原始源码诊断与人工 `not_assessed`。MISRA 授权正文及 221 项评估由 #57／#58 跟踪；版本身份、完整 BSWMD／随包模块文档与跨模块 Symbolic Name Value 分别由 #59／#159／#160 跟踪。这里的处置不关闭这些共享差距，也不提供真实 MCU／中断、功能安全或认证证据。
