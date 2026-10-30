# 公共标准类型的未使用报告

本篇只记录当前四 profile 中的公开声明诊断，不把接口存在等同于符合性。
授权依据是 ADR 0023 及其原用户决定；以下15项已获独立复核支持，并落实精确源码 MR12。
保留原始诊断、源码身份和 not_assessed，不改变规则或基线。

## 必要性与风险

SWS_Platform_00002 明确要求平台抽象类型与符号在 Platform_Types.h 发布；固定 R24-11 的各类型定义表进一步规定类型名、表示／范围及 Available via 头。所选程序未使用的标准声明仍真实未使用；删除或私有化会改变相应标准头。声明不启用 OS-Application／保护／可信调用／多核服务，也不把这些未选服务记为已实现。单核 SC1 未选功能的类型另按下节裁剪，不援引发布冲突。

同名类型改为宏或复制底层类型不能同时保持公开 typedef 契约；不以增加空引用消除报告。风险限公共名称及未来误用，既有配置拒绝与 C99 公共头消费者继续约束实际支持。标准／类型表示、支持配置或调用方变化时重评。

| 规则 | 精确范围 | 原文与所保留的契约 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8` | SWS_Platform_00016 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16` | SWS_Platform_00017 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint64` | SWS_Platform_00067 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8_least` | SWS_Platform_00023 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16_least` | SWS_Platform_00024 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint32_least` | SWS_Platform_00025 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint8_least` | SWS_Platform_00020 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint16_least` | SWS_Platform_00021 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint32_least` | SWS_Platform_00022 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float32` | SWS_Platform_00041 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float64` | SWS_Platform_00042 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `VoidPtr` | SWS_Platform_91001 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `ConstVoidPtr` | SWS_Platform_91002 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/LSduR.h` 中 `LSduR_StateType` | CP_SWS_LSduR_91005 要求的 UNINIT／ONLINE 枚举；LSduR 原条款为 DRAFT。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/PduR.h` 中 `PduR_StateType` | SWS_PduR_00742 要求的 UNINIT／ONLINE 枚举。 | 维护者 原用户决定，ADR 0023；独立复核支持 |

## 单核 SC1 未选功能的声明裁剪

当前目标由配置入口拒绝 OS-Application、可信调用、保护、多核 spinlock 和 peripheral 区域。
复核固定 SWS_Os_00241 的 PDF130–131 表格、00764、00516、00542 后，未发现当前配置无条件发布
这些类型的义务。原声明只有 public_types.c 的清单消费者，已移除15个 typedef 及专用宏，
不新增虚假功能开关。支持功能的类型／服务签名和对象布局保持；源码合同变化见 CHANGELOG。

不适用（配置：ECUC_Os_00259 只接受 SC1、目标固定单核，所列功能未选择且配置入口拒绝，Os_Types.h 不发布其声明）：SWS_Os_00772、SWS_Os_00773、SWS_Os_00774、SWS_Os_00775、SWS_Os_00776、
SWS_Os_00777、SWS_Os_00778、SWS_Os_00779、SWS_Os_00780、SWS_Os_00781、
SWS_Os_00787、SWS_Os_00788、SWS_Os_00791、SWS_Os_00792、SWS_Os_91000。

public-types 正常入口继续按六种头顺序编译／运行 SC1 的独立范围及指针消费者；
另逐个编译15个未支持类型的引用，拒绝必须定位到输入第2行和对应类型名。
恢复一个类型声明会使该项拒绝断言失败，不能由其他头错误冒充。

## 类型与初始化的人工评估

使用[共同配置、原件与冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)；
本节不扩大上表R2.3的范围。`TU`为Single Translation Unit，`System`须核对跨TU身份；
C11条目逐项按规定对象和当前C99源码构造判断，不豁免GNU扩展及自有调用方。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| R6.3 | Required | TU／C99 | satisfied | 位域不能直接为union成员，union内struct子对象不在禁止集合（AMD3.15，8–9）。实际Dcm_MsgAddInfoType的两个uint8位域直接属于struct；[类型/位域依据](../dcm/interfaces.md)及双平台所选项目/补丁声明没有直接union位域，SDK原件不改归自有。 |
| R7.5 | Mandatory | TU／C99 | satisfied | 整数常量宏实参为单个无后缀literal且在相应exact-width范围（AMD3.16，9–10；AMD4.55仅交叉引用）。[整数常量闭包](../types.md#整数常量初始化与浮点构造)及本轮两组八份交付分别28/26组实参均合法，最大restart值0x4155544f53525431。ecu.rs::configure_os把u32配置转u64后以十进制输出，tick_ms乘1000000不超u64；非负有界输入不产生符号/后缀/表达式实参。 |
| R7.6 | Required | TU／C99 | satisfied | 不用宽度小于int的minimum-width常量宏（AMD4.4，7–8；正文/AMD4.64为Required，AMD4.65表冲突不降级）。两目标int32；[启用RES_SCHEDULER路径](../types.md#整数常量初始化与浮点构造)已以1u替换UINT16_C(1)，实际producer及两组交付没有INT8/UINT8/INT16/UINT16_C用途。无诊断并非证据。 |
| R8.15 | Required | System／C11 | not_applicable | 同一对象各声明的显式alignment须在宏展开后词法相同（AMD3.17，10–11）。所选自有/补丁源码无_Alignas/alignas/stdalign.h；[对齐边界](../types.md#整数常量初始化与浮点构造)中的GNU aligned另按ABI与实际对象布局核查。 |
| R8.16 | Advisory | TU／C11 | not_applicable | alignment表达式不能取零（AMD3.17，12）。同一源码集合没有C11 alignment说明符；不把GNU对齐参数或没有alignment说明符当作显式零。 |
| R8.17 | Advisory | TU／C11 | not_applicable | 一个声明至多一个显式alignment说明符（AMD3.17，12–13）。同一源码没有该说明符，故没有需要合并的多个alignment；平台属性仍守实际ABI约束。 |
| R9.6 | Required | TU／C99 | not_applicable | chained designator所在initializer list要求各项显式指定，内层独立花括号有原文例外（AMD4.5，8–9）。[初始化构造](../types.md#整数常量初始化与浮点构造)中.priorities={[OS_INPUT_INTERRUPT]=1u}是两层列表，当前无混用chained/隐式位置项；sigaction SDK宏用于赋值，不是chained initializer。 |
| R9.7 | Mandatory | System／C11 | not_applicable | 原子对象先初始化后访问；静态默认初始化有效，非静态用声明初始化或atomic_init（AMD4.6，10–11）。[原子对象边界](../types.md#c11-原子对象的适用边界)及当前交付无_Atomic/stdatomic.h/标准atomic入口；GNU__atomic不是C11原子对象初始化。 |
| R11.10 | Required | TU／C11 | not_applicable | 不能把_Atomic施加于不完整void（AMD4.15，12）。同一有限源码无该类型；普通void指针转换仍受R11系列及[精确地址类别](../os/native-facilities/addresses.md)约束。 |
| R12.6 | Required | TU／C11 | not_applicable | 不可直接访问原子struct/union成员，应以整个对象的原子操作处理（AMD4.16，13）。同一源码无原子聚合对象，CONTEXT的FP/字节复制不是原子成员访问。 |
| R17.13 | Required | TU／C99 | satisfied | 不能限定函数类型；限定返回类型/函数指针对象不违反本条（AMD3.38，20–21）。[真实callback typedef](../os/configuration.md#c99-数组与回调类型)均为函数指针，配置const限定指针或数据对象；Task声明宏形成函数入口，没有const/volatile/restrict限定的函数typedef。 |
| R23.1 | Advisory | TU／C11 | not_applicable | generic selection只应从函数式宏展开，控制表达式取宏实参（AMD3.51，27–28）。[泛型边界](../types.md#泛型选择的适用边界)及当前所选C/H无_Generic；未用宏例外来批准普通重复求值。 |
| R23.2 | Required | TU／C11 | not_applicable | 非宏generic控制表达式不得有潜在副作用，函数调用也计入（AMD3.51，28–29）。同一源码无generic选择，因而无待核控制表达式；普通sizeof/逻辑表达式仍独立评估。 |
| R23.3 | Advisory | TU／C11 | not_applicable | generic至少一个非default association（AMD3.51，29–30）。同一源码没有association，不由C99标签推定已有列表合法。 |
| R23.4 | Required | TU／C11 | not_applicable | association类型不能是会经lvalue转换消失的顶层限定、原子、数组、函数或匿名struct/union定义（AMD3.51，30–31）。同一源码无association；不混淆指向限定对象的指针类型。 |
| R23.5 | Advisory | TU／C11 | not_applicable | 指针generic选择不依赖隐式指针转换决定default（AMD3.51，32–33）。同一源码没有指针选择，算术选择本身也不触发本条。 |
| R23.6 | Required | TU／C11 | not_applicable | controlling expression的essential/standard type须匹配，特定非char/非Boolean整数常量表达式有例外（AMD3.51，33–35）。同一源码没有该表达式，未借常量例外扩大普通转换许可。 |
| R23.7 | Advisory | TU／C11 | not_applicable | generic宏实参应在整个展开中恰求值一次；各association均为整数常量表达式的type query有例外（AMD3.51，35–36）。同一源码无generic宏，没有以宏名或无副作用推定任意重复求值可接受。 |
| R23.8 | Required | TU／C11 | not_applicable | 有default时须在association首或尾（AMD3.51，36–37）。同一源码没有association/default；没有default本身不违反此条。 |
<!-- misra-manual-end -->
