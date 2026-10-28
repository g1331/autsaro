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

不适用（单核 SC1，所列功能未选择且配置入口拒绝，Os_Types.h 不发布其声明）：
SWS_Os_00772、SWS_Os_00773、SWS_Os_00774、SWS_Os_00775、SWS_Os_00776、
SWS_Os_00777、SWS_Os_00778、SWS_Os_00779、SWS_Os_00780、SWS_Os_00781、
SWS_Os_00787、SWS_Os_00788、SWS_Os_00791、SWS_Os_00792、SWS_Os_91000。

public-types 正常入口继续按六种头顺序编译／运行 SC1 的独立范围及指针消费者；
另逐个编译15个未支持类型的引用，拒绝必须定位到输入第2行和对应类型名。
恢复一个类型声明会使该项拒绝断言失败，不能由其他头错误冒充。
