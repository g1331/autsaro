# 标准发布信息的未使用宏

本篇只记录当前四 profile 中的公开声明诊断，不把接口存在等同于符合性。
授权依据是 ADR 0023 及其原用户决定；以下68项已获独立复核支持，并落实精确源码 MR12。
保留原始诊断、源码身份和 not_assessed，不改变规则或基线。

## 固定契约

SWS_BSW_00059 要求所有模块头提供受多重定义保护的发布信息 #define；SWS_BSW_00256 明列 MODULE_ID、AR_RELEASE_MAJOR／MINOR／REVISION 与 SW_MAJOR／MINOR／PATCH 等字段。表中原文要求是预处理定义，不能用 enum、私有常量或删除未消费字段代替。当前所选程序未读取的这些宏保留真实 R2.5 Advisory 违反。

宏仅发布版本／身份常量，不增加可变状态；原有公共头首包含、错误依赖版本及收件者编译验证保持。名称、字段来源、目标版次或发布接口变化时重评。供应方身份、采用代码和任意其他宏不在此记录。

| 规则 | 精确范围 | 原文及控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/BswM.h` 中 `BSWM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/BswM_Version.h` 中 `BSWM_AR_RELEASE_REVISION_VERSION`、`BSWM_SW_MAJOR_VERSION`、`BSWM_SW_MINOR_VERSION`、`BSWM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/CanIf_Version.h` 中 `CANIF_AR_RELEASE_REVISION_VERSION`、`CANIF_SW_MAJOR_VERSION`、`CANIF_SW_MINOR_VERSION`、`CANIF_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/CanTp_Version.h` 中 `CANTP_AR_RELEASE_REVISION_VERSION`、`CANTP_SW_MAJOR_VERSION`、`CANTP_SW_MINOR_VERSION`、`CANTP_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Can_Version.h` 中 `CAN_AR_RELEASE_REVISION_VERSION`、`CAN_SW_MAJOR_VERSION`、`CAN_SW_MINOR_VERSION`、`CAN_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Com.h` 中 `COM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/ComM.h` 中 `COMM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/ComStack_Version.h` 中 `COMSTACKTYPE_AR_RELEASE_REVISION_VERSION`、`COMSTACKTYPE_SW_MAJOR_VERSION`、`COMSTACKTYPE_SW_MINOR_VERSION`、`COMSTACKTYPE_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Com_Version.h` 中 `COM_AR_RELEASE_REVISION_VERSION`、`COM_SW_MAJOR_VERSION`、`COM_SW_MINOR_VERSION`、`COM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Dcm.h` 中 `DCM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Dcm_Version.h` 中 `DCM_AR_RELEASE_REVISION_VERSION`、`DCM_SW_MAJOR_VERSION`、`DCM_SW_MINOR_VERSION`、`DCM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Det.h` 中 `DET_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Det_Version.h` 中 `DET_AR_RELEASE_REVISION_VERSION`、`DET_SW_MAJOR_VERSION`、`DET_SW_MINOR_VERSION`、`DET_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/LSduR.h` 中 `LSDUR_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/LSduR_Version.h` 中 `LSDUR_AR_RELEASE_REVISION_VERSION`、`LSDUR_SW_MAJOR_VERSION`、`LSDUR_SW_MINOR_VERSION`、`LSDUR_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/MemMap_Version.h` 中 `MEMMAP_AR_RELEASE_REVISION_VERSION`、`MEMMAP_SW_MAJOR_VERSION`、`MEMMAP_SW_MINOR_VERSION`、`MEMMAP_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/PduR.h` 中 `PDUR_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/PduR_Version.h` 中 `PDUR_AR_RELEASE_REVISION_VERSION`、`PDUR_SW_MAJOR_VERSION`、`PDUR_SW_MINOR_VERSION`、`PDUR_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `PLATFORM_AR_RELEASE_REVISION_VERSION`、`PLATFORM_SW_MAJOR_VERSION`、`PLATFORM_SW_MINOR_VERSION`、`PLATFORM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Rte_Version.h` 中 `RTE_AR_RELEASE_REVISION_VERSION`、`RTE_SW_MAJOR_VERSION`、`RTE_SW_MINOR_VERSION`、`RTE_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Std_Types.h` 中 `STD_TYPES_AR_RELEASE_REVISION_VERSION`、`STD_TYPES_SW_MAJOR_VERSION`、`STD_TYPES_SW_MINOR_VERSION`、`STD_TYPES_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Arti_Version.h` 中 `ARTI_AR_RELEASE_REVISION_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Version.h` 中 `OS_AR_RELEASE_REVISION_VERSION`、`OS_SW_MAJOR_VERSION`、`OS_SW_MINOR_VERSION`、`OS_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023；独立复核支持 |

## 必需预处理常量与构造宏

以下26个现存 R2.5 位置单独核对，不继承前述68项的独立复核结论。
限定独立复核固定 `9b507f61` 支持这26项的必需宏发布范围；已补正构造宏的直接依据并落实 MR12。
PlatformTypes §7.2/7.3、8.3.1–3 明确要求三个平台选择符以 `#define` 发布；
StandardTypes §8.2.2/3 明列四个状态宏的 `#define` 形式。
把它们改成 enum、私有常量或删除未消费的名字，不能保留同一预处理接口。
这里没有据此批准全部 CPU 枚举值或其他公共状态名字的宏形式。

OSEK OS 2.2.3 §11.2（印刷41页）明确要求服务标识 accessor，以及第一参数为 object identifier 的服务 accessor；
§13.8.4 把访问限制在 ErrorHook。下表只包括 OSEK 本身定义的12个首对象参数，不把其他参数、
输出指针或 AUTOSAR 扩展服务一起推定为必需。
四个 DeclareX 由 R24-11 OS §7.1.2.1 直接要求作为无功能兼容宏提供，OSEK §13.2.2.1、13.4.2.1、
13.5.2.1、13.6.2.1 给出语法；非规范的 §14.2.7 仅作兼容背景。ISR 与 ALARMCALLBACK 的直接依据
分别为 OSEK §13.3.3、§13.6.5；SWS_Os_00815 只约束声明的内存映射包围，不证明构造或拼接必要性。
构造参数位于声明/定义位置，不能替换成 enum 或有运行效果的函数调用。

R2.5 为 Advisory/System；主文印刷44/PDF52，TC1 将撤销计作用途，TC2.5/6 明确对象为宏定义。
这里承认当前程序中的真实未使用，不以公开可见或测试中的使用改写生产程序结果。
风险为公开预处理面；数值/展开式和原有消费者保持。类型、所选功能、宏展开、标准版次或包含闭包变化时重评。
授权按 ADR0023 的既有 AUTOSAR/OSEK 必需冲突处理；实现者完成具体评估，独立复核仅支持列明的宏发布义务。
ErrorHook 这里只处置宏发布的 R2.5，明确不批准参数副本的 R18.6，后者继续引用既有待决提案。

| 规则 | 精确范围 | 原文及控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE`、`CPU_BIT_ORDER`、`CPU_BYTE_ORDER` | SWS_Platform_00044/00045/00064、00043/00038、00046/00039；固定x64选择不变，公开预处理值保持。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Std_Types.h` 中 `STD_HIGH`、`STD_LOW`、`STD_ACTIVE`、`STD_IDLE` | SWS_Std_00007/00013 明列四个define及数值；保留头内常量表达式。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os.h` 中 `DeclareTask`、`DeclareResource`、`DeclareEvent`、`DeclareAlarm` | OS §7.1.2.1及OSEK §13.2.2.1/13.4.2.1/13.5.2.1/13.6.2.1；public-compatibility保持文件/块作用域声明及参数零求值。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os.h` 中 `ISR`、`ALARMCALLBACK` | OSEK §13.3.3/13.6.5；SWS_Os_00815仅作映射约束；entry-bodies实际定义并运行入口，函数/ID绑定保持。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSErrorGetServiceId` | OSEK §11.2、13.8.4；error-hooks观察实际失败服务标识。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSError_GetTaskState_TaskID`、`OSError_ActivateTask_TaskID`、`OSError_ChainTask_TaskID`、`OSError_GetResource_ResID`、`OSError_ReleaseResource_ResID`、`OSError_SetEvent_TaskID`、`OSError_GetEvent_TaskID` | OSEK §11.2，§13.2/13.4/13.5 的首对象参数；现有error-hooks的服务/参数绑定保持。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSError_GetAlarmBase_AlarmID`、`OSError_GetAlarm_AlarmID`、`OSError_SetRelAlarm_AlarmID`、`OSError_SetAbsAlarm_AlarmID`、`OSError_CancelAlarm_AlarmID` | OSEK §11.2、13.6 的首AlarmID；不把后续参数/输出指针纳入。 | 维护者 原用户决定，ADR0023；独立复核9b507f61支持，引用已更正 |

## 构造宏的字面入口绑定

R20.10为Advisory/Single Translation Unit（主文印刷162/PDF170）。本节按2026-10-30精确决定保留四个入口宏和ARTI_TRACE的当前内容，不把项目选用的数字ID／私有前缀或字符串化说成标准唯一形式。OSEK §12.1允许对象表示由实现选择，SWS_Os_00815只约束MemMap；其他ID表示、生成绑定或显式trace名字仍是可行替代。

TASK／ISR构造、OS_TASK_ENTRY／OS_ISR_ENTRY设计符继续提供同一跨TU入口；ARTI_TRACE保持class／event命名。既有entry-bodies、public-compatibility、真实ARTI及完整映射消费者约束多入口、单次定义、身份与代码段；配置命名、ID表示、包装ABI、宏内容、目标或程序改变时重评。

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Arti.h` 中 `ARTI_TRACE` | shape=`#define ARTI_TRACE(context, class_name, instance, instance_parameter, event, event_parameter) Arti_Record(#context, #class_name, #instance, #event, (instance_parameter), (event_parameter))`; 范围 `All/Both/Both`；保持现有跨TU入口绑定或trace名字构造；不宣称这是标准唯一实现。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Os.h` 中 `TASK` | shape=`#define TASK(identifier) OS_CODE void Os_TaskEntry_##identifier(void); OS_CODE void Os_TaskEntry_##identifier(void)`; 范围 `All/Both/Both`；保持现有跨TU入口绑定或trace名字构造；不宣称这是标准唯一实现。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Os.h` 中 `OS_TASK_ENTRY` | shape=`#define OS_TASK_ENTRY(identifier) Os_TaskEntry_##identifier`; 范围 `All/Both/Both`；保持现有跨TU入口绑定或trace名字构造；不宣称这是标准唯一实现。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Os.h` 中 `ISR` | shape=`#define ISR(identifier) OS_CODE void Os_IsrEntry_##identifier(void); OS_CODE void Os_IsrEntry_##identifier(void)`; 范围 `All/Both/Both`；保持现有跨TU入口绑定或trace名字构造；不宣称这是标准唯一实现。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Os.h` 中 `OS_ISR_ENTRY` | shape=`#define OS_ISR_ENTRY(identifier) Os_IsrEntry_##identifier`; 范围 `All/Both/Both`；保持现有跨TU入口绑定或trace名字构造；不宣称这是标准唯一实现。 | 维护者 2026-10-30精确124键决定 |
