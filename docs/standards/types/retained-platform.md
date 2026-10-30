# 平台与OS常量的预处理形式

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

R2.5为Advisory/System，保留表列宏及原替换tokens，而非声称每个值均由标准强制为宏。enum、按配置裁剪等替代仍可行；保留预处理使用合同不增加对应保护／多核等运行能力。既有类型／值域与六包含顺序消费者约束不变。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE_8` | shape=`#define CPU_TYPE_8 8u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE_16` | shape=`#define CPU_TYPE_16 16u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE_32` | shape=`#define CPU_TYPE_32 32u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE_64` | shape=`#define CPU_TYPE_64 64u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `MSB_FIRST` | shape=`#define MSB_FIRST 0u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `LSB_FIRST` | shape=`#define LSB_FIRST 1u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `HIGH_BYTE_FIRST` | shape=`#define HIGH_BYTE_FIRST 0u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `LOW_BYTE_FIRST` | shape=`#define LOW_BYTE_FIRST 1u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `OS_CORE_ID_MASTER` | shape=`#define OS_CORE_ID_MASTER OS_CORE_ID_0`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `SCHEDULETABLE_WAITING` | shape=`#define SCHEDULETABLE_WAITING 2u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `SCHEDULETABLE_RUNNING_AND_SYNCHRONOUS` | shape=`#define SCHEDULETABLE_RUNNING_AND_SYNCHRONOUS 4u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_PROTECTION_MEMORY` | shape=`#define E_OS_PROTECTION_MEMORY 12u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_CORE` | shape=`#define E_OS_CORE 14u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_NESTING_DEADLOCK` | shape=`#define E_OS_NESTING_DEADLOCK 15u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_PROTECTION_LOCKED` | shape=`#define E_OS_PROTECTION_LOCKED 16u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_SPINLOCK` | shape=`#define E_OS_SPINLOCK 17u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_SERVICEID` | shape=`#define E_OS_SERVICEID 18u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_PROTECTION_EXCEPTION` | shape=`#define E_OS_PROTECTION_EXCEPTION 19u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_INTERFERENCE_DEADLOCK` | shape=`#define E_OS_INTERFERENCE_DEADLOCK 20u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_PROTECTION_TIME` | shape=`#define E_OS_PROTECTION_TIME 21u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Types.h` 中 `E_OS_PROTECTION_ARRIVAL` | shape=`#define E_OS_PROTECTION_ARRIVAL 22u`; 范围 `All/Both/Both`；保留当前预处理接口及原数值／选择含义；不添加虚假消费，不扩大未支持运行能力。 | 维护者 2026-10-30精确124键决定 |
