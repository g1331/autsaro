# 公共标准类型的未使用报告

本篇只记录当前四 profile 中的公开声明诊断，不把接口存在等同于符合性。
授权依据是 ADR 0023 及其原用户决定；以下为待独立复核的具体记录，尚不添加源码 MR12。
保留原始诊断、源码身份和 not_assessed，不改变规则或基线。

## 必要性与风险

固定 R24-11 定义表规定类型名、表示／范围及 Available via 头。所选程序未使用的标准声明仍真实未使用；删除或私有化会改变相应标准头。声明不启用 OS-Application／保护／可信调用／多核服务，也不把这些未选服务记为已实现。OS SC1 下相关类型的发布义务与可选服务的可用性须在本批复核中分开核实。

同名类型改为宏或复制底层类型不能同时保持公开 typedef 契约；不以增加空引用消除报告。风险限公共名称及未来误用，既有配置拒绝与 C99 公共头消费者继续约束实际支持。标准／类型表示、支持配置或调用方变化时重评。

| 规则 | 精确范围 | 原文与所保留的契约 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8` | SWS_Platform_00016 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16` | SWS_Platform_00017 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint64` | SWS_Platform_00067 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint8_least` | SWS_Platform_00023 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint16_least` | SWS_Platform_00024 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `sint32_least` | SWS_Platform_00025 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint8_least` | SWS_Platform_00020 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint16_least` | SWS_Platform_00021 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `uint32_least` | SWS_Platform_00022 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float32` | SWS_Platform_00041 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `float64` | SWS_Platform_00042 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `VoidPtr` | SWS_Platform_91001 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/Platform_Types.h` 中 `ConstVoidPtr` | SWS_Platform_91002 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ApplicationType` | SWS_Os_00772 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ApplicationStateType` | SWS_Os_00773 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ApplicationStateRefType` | SWS_Os_00774 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `TrustedFunctionIndexType` | SWS_Os_00775 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `TrustedFunctionParameterRefType` | SWS_Os_00776 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `AccessType` | SWS_Os_00777 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ObjectAccessType` | SWS_Os_00778 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ObjectTypeType` | SWS_Os_00779 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `MemoryStartAddressType` | SWS_Os_00780 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `MemorySizeType` | SWS_Os_00781 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `ProtectionReturnType` | SWS_Os_00787 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `RestartType` | SWS_Os_00788 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `SpinlockIdType` | SWS_Os_00791 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `TryToGetSpinlockType` | SWS_Os_00792 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/os/include/Os_Types.h` 中 `AreaIdType` | SWS_Os_91000 的类型定义及公开头；保留原类型、限定符和所选 x64 表示。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/LSduR.h` 中 `LSduR_StateType` | CP_SWS_LSduR_91005 要求的 UNINIT／ONLINE 枚举；LSduR 原条款为 DRAFT。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.3 | `runtime/bsw/include/PduR.h` 中 `PduR_StateType` | SWS_PduR_00742 要求的 UNINIT／ONLINE 枚举；LSduR 原条款为 DRAFT。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
