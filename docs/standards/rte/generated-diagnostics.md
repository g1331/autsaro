# 生成 C 的诊断责任与处置

本篇属于 [RTE 与生成契约](README.md)，承接 #382。只覆盖四个正常分析 profile 的 emitter／模板输出，目标为 controlled Linux／Windows x64、C99、HostBatch 与 probe。直接拷贝的 runtime、内核／补丁、SDK 和无 checker 人工项分别由 #381／#383／#384／#385 承接；模板原行仍属本项，已有精确裁决可以引用；原始分析、程序归属、`passed=false`、`not_assessed` 和比较拒绝保持。

## 来源与字节身份

`plan` 的验证结果由 `contracts::contract_headers`、`rte::files`、`ecu_config::files`、`memmap`、`ecu::configure_os` 等 producer 消费；`outputs` 对每个路径只接受一个生成方，`ecu` 封存交付，分析入口按 `target.json` 的 TU、参数和 include 顺序准备实际程序。PR 记录实际 head、四份 `files.sha256` 摘要与原始诊断位置；运行结果不虚构 Rust emitter 行号。

`outputs::TEMPLATES` 保留的 runtime 原行与 producer 新增行分别追溯。例如 `generated/src/Os_Arti.c` 的固定实现来自 `runtime/os/src/Os_Arti.c`，`generated/include/Rte_Type.h` 中新增的 `Rte.h` 包含则来自 `contracts::contract_headers`。目录名不能替代来源判定，也不能用未合并的 #381 字节代表本分支。

## 普通包含与生命周期整改

`SWS_Rte_01163` 明确要求 RTE Types Header 包含 RTE Header。`contracts::contract_headers` 将 `#include "Rte.h"` 与模板的 `Std_Types.h` 包含一起输出；应用类型仍在 guard 内。它原先跟随 CounterType／TimeInMicrosecondsType typedef，构成预处理前适用的 R20.1（主文印刷152／PDF160）违反，不属于 MemMap 类别。依赖保留，普通包含移到声明之前。

`rte.rs` 原先的两个布尔对象仅表示未分配、已启动、已停止三种状态。现在由单 owner 的 `uint8 rte_lifecycle` 表示：清零对应未分配；Start 成功置运行；Stop 在已分配时置停止，不恢复未分配。先 Stop 后首次 Start、重复 Start、重复 Stop、停止后读输出不变及停止后拒绝再次 Start 由真实 RTE 消费者验证。BSWMD 的静态对象和预留名字同步至同一状态，避免引入第二个生命周期 owner；原 `rte_allocated` 单函数 R8.9 来源被消除。

## 初始化配置对象的 R8.9

R8.9 是 Advisory、System（2013主文印刷71／PDF79）。它按标识符出现的函数数量判断，静态存储期本身不提供例外。这些对象只由 `Ecu_StackBootstrap` 直接命名，保留真实违反。

固定 SpecificationsARXML 的 `SWS_MemMap_00003` 要求声明／定义的 START→include→实体→STOP→include 完整序列；`00023` 禁止函数体内包含映射头。保持当前预编译配置定义及其所属段时，不能把定义及完整映射一起移进初始化函数。只保留 CODE 属性、使用默认段、永久保留段宏或把对象改成未映射的临时配置，都不保持当前合同。已撤回块内 MemMap 实验，未将其编译或结构检查作为规范证据。

以下定义由 `ecu_config::init_data` 生成，输出在 `generated/src/Ecu_RuntimeConfig.c`，各用 `RTE_<scope>_CONFIG_DATA_PREBUILD_UNSPECIFIED` 完整映射。standard-ecu、user-application、component-pipeline 使用全部八个；no-diagnostic 不生成 CanTp／Dcm 两个。测试和生产两种 bootstrap 复用同一来源，不因条件排除而删除物理位置。

| 对象／scope | 实际消费者与生命周期 |
| --- | --- |
| `driver`／Can | `Can_Init` 在调用中读取并复制 sink；不保存该配置对象地址。保留的是本预编译配置定义及 Can 配置段，不以地址逃逸作为理由。 |
| `com_config`／Com | `Com_Init` 保存 `com_configuration`；收发、模式和超时持续消费至 DeInit。 |
| `tp_config`／CanTp | `CanTp_Init` 保存 `cantp_configuration`；收发状态机与计时器消费至 Shutdown。 |
| `dcm_config`／Dcm | `Dcm_Init` 保存 `dcm_configuration`；请求、容量、时序和认证回调在 Init 返回后使用。 |
| `bus_config`／Ecu_HostBusSM | Init 保存 `bussm_configuration`；模式请求与状态查询在后续周期使用，DeInit 清除。 |
| `comm_config`／ComM | `ComM_Init` 保存 `comm_configuration`；通道、用户和模式回调持续消费至 DeInit。 |
| `bswm_config`／BswM | `BswM_Init` 保存 `bswm_configuration`；后续模式通知调用实际 action，Deinit 清除。 |
| `det_config`／Det | `Det_Init` 保存 `det_configuration`；后续错误报告读取运行时 callout 表。 |

风险是配置可见范围过宽及段归属／寿命错误。对象均为本 TU 的 `static const`；公开模块只取得 const 指针，没有外部可修改配置入口。真实生成工程在 bootstrap 返回后运行通信／诊断／模式与错误路径，配置段由实际符号／二进制核对，完整映射由共用协议检查保护。改动 Init 的复制／保留机制、对象所属段、预编译配置合同或目标布局时重评。

本项按 [ADR0023](../../adr/0023-autosar-required-conflict-disposition.md) 的既有 AUTOSAR 必需冲突授权逐对象评估，来源为 [2026-10-17决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)，不引入“所有 static 对象”类别。源码 MR12 只说明上述机制；独立审查 head、发现和处置记录于实施 PR。项目技术处置不等于正式 MISRA 符合声明。

## 原文明确要求的类型和预处理接口

本轮在固定 SpecificationsARXML 正文及 ECUC MOD 原件核对以下构造；R2.3／R2.5 都是 Advisory、System（2013主文印刷43–44／PDF51–52），这里仍承认实际未使用，而非误报。替换为 enum、局部 typedef、私有函数或仅生成当前调用使用的名字，会分别改变明确要求的类型位置、typedef／define 形式或完整发布集合。

| 规则与具体接口 | producer 与必需义务 |
| --- | --- |
| R2.3：`Rte_EchoApplication.h`／`Rte_Ingress.h` 的 `Rte_Instance` | `contracts::contract_headers`；01007要求application header定义实例handle类型，01148／01150固定CDS指针及typedef名字。与既有R5.6分别处置。 |
| R2.3：`Rte_Dcm_Type.h` 的 `Dcm_OpStatusType`、`Dcm_ConfirmationStatusType`、`Dcm_ProtocolType` | `contracts::dcm_type_header`保留模板定义；00984／00983／00979的完整类型条目均为uint8、Variation无条件、指定发布头。没有用某个当前回调未消费来删除完整类型合同。 |
| R2.3：同头的 `Dcm_DataElement_ApplicationValue_ArrayType`、`Dcm_DataArrayTypeUint8_ApplicationValueType` | `composition::diagnostic_types`；91063／01121的名字、uint8元素与数组条件；三个诊断profile实际为`UINT8_N`、`USE_DATA_ELEMENT_SPECIFIC_INTERFACES`、`USE_DATA_SYNCH_CLIENT_SERVER`、四字节。无诊断profile不生成这两项。 |
| R2.5：CanIf／CanTp／Com／Dcm／PduR配置头的`<Mip>Conf_...`名字 | `symbolic_names::Names`→`ecu_config::files`；TPS_ECUC_02108明确要求有效PreCompile且symbolicNameValue=true的值以define发布。已核MOD的CanTpRxFcNPduId、ComIPduHandleId、DcmDslProtocolType、DcmDslProtocolRxConnectionId、DcmDspSessionLevel、PduRDestPduHandleId均为该条件；实际源码值与所选配置一致。合成LSduR句柄不借用此依据。 |
| R2.5：`Rte_InitValue_...` | `contracts::contract_headers`；05078明确规定已选非队列VALUE数据初值宏、端口／数据元素名字及后缀。 |
| R2.5：`RTE_RUNNABLE_...` | 同producer；06713明确要求每个RunnableEntity的define，保留真实symbol映射。 |
| R2.5：`Os_Counter.h` 的`OS_TICKS2NS/US/MS/SEC_SystemCounter(ticks)` | `ecu::configure_os`；00393明确要求四个转换宏的格式，当前参数单次求值，分辨率及uint64结果范围不变。 |
| R2.5：`Rte.h` 的`Rte_IsInfrastructureError`、`Rte_HasOverlayedError`、`Rte_ApplicationError` | `rte_types::standard`渲染模板；07404／07405／07406明确规定三个宏及位语义，不替换为另一接口形状。 |

四profile冻结语料中，这些构造共46个源码文本键：7个R2.3、39个R2.5（同名宏在不同profile的值不同则分别计键）。每个实际位置与程序保留在PR原始清单中；不把未报告的所有配置成员计为违反。源码MR12随对应producer输出，不改静态样本，不添加虚假读取或suppression。

风险为头文件发布面与宏的潜在误用。完整接口的真实C/C++消费者、输入名字／类型／配置拒绝、预处理常量值与宽度、宏单次求值及真实交付链接／运行分别约束它。形式／名字／配置条件、底层类型、数组大小或受支持接口集合变化时重评。授权沿ADR0023原决定；实施者完成必要性与替代核对，独立审查head／发现／处置在PR记载。项目授权与正式MISRA偏离／符合声明仍分别保留。

## 其余诊断的裁决边界

- MemMap R20.1／R20.5：逐包含请求与真实 undef 引用[既有类别](../memmap/README.md#memmap-类别偏离)，普通包含不借用此类别。
- R15.5：引用 ADR0025 的既有 Advisory GRP disapply，仍检查拒绝路径、资源和输出。
- 无名 role 参数 R8.2、组件实例类型 R5.6：分别沿 [role 参数](contracts.md#无名-role-参数的逐处核查)与[实例类型](README.md#组件实例类型的同名冲突)的已有精确记录，不扩大到其他声明。
- Boolean R10.3／R10.4：连接实际 `_Bool`、`TRUE`／`FALSE` 展开，沿[essential type](../types/essential-types.md#boolean与character)的 D.6／D.7 依据逐表达式核对，不因工具模型把所有整数常量归为 Boolean。
- R2.3／R2.5／R8.7：标准强制发布与当前程序未使用是两个事实。固定名字、typedef、宏形式及完整接口要求须分别核对；仅有外部 ABI 或既有交付形状不能自动获得 ADR0023 授权。类别外的具体保留须先形成可审阅提案，由用户决定，未决键继续计为未分类。

原始记录数、物理位置、规则＋文件＋源码行文本键以及两程序出现次数分别保留。最终四 profile 的分类与尚未闭合集合由 PR 按封存身份给出；本文不预先声明零未分类、完整221项评估或 Windows 实际执行通过。


## 尚未接受的保留候选

`Os_ArtiAlarmBaseReturn` 的 `const uint64 native_address = (uint64)address;` 是 R11.4（Advisory，2013主文印刷97／PDF105）的真实指针到整数转换。producer 是 `arti::files` 渲染 `runtime/os/src/Os_Arti.c`，不是 SDK 或 #381 的八个已记录主机地址实例。GetAlarmBase 的真实返回经过本函数，把完整受控 x64 数值交给 `Arti_CaptureAddress`，低32位交给 ARTI trace；`ecu_arti_control.c` 分别检查两者。没有从 trace token 恢复指针或解引用捕获值。uintptr_t 改写仍属 R11.4；删除完整捕获会破坏现有工具观察合同。新增保留未获具体决定，维持未分类，不添加 MR12 或批准表。

R2.3／R2.5 的公共类型、固定值和接口宏，以及 R8.7 的实例句柄和模式接口，分别保留真实未使用／单TU引用事实。固定 AUTOSAR 原文明确规定 typedef／define 形式的部分沿 ADR0023 逐项核查；只有既有库式交付／预处理契约理由的部分仍是项目保留候选，不能因“Available via”或已有测试就称为标准不可替代。实施 PR 冻结每个规则、文件、源码行、producer、profile与程序的候选集合，连同可行替代和具体风险供决定；未经接受不转为已分类。

删除私有 `Ecu_ExecutionNow` 的真实闭包影响也分别记录：生成源的无消费者 helper 消除，但不改变 `Ecu_TargetNow` 的受控时钟合同；后者仍有 owner_probe、scheduler／ARTI、copy-timeout／zero-DLC 原生消费者。若闭包变化揭示该 owned API 的 R8.7，原始新增位置与 #381 归属单独交接，不将它归为 emitter 或静默扩展既有批准。该 owned API 的精确保留与消费者范围由 [#381 固定记录](https://github.com/g1331/autsaro/blob/70f94048/docs/standards/os/native-facilities/retained-clock.md) 承接；本轮 v3 的输入身份、物理行144及两程序 raw 保持原样，不以随后纯注释修改重写旧扫描。
