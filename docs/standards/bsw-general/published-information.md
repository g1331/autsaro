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

## 必需预处理常量与构造宏（待复核）

以下26个现存 R2.5 位置单独核对，不继承前述68项的独立复核结论。
它们尚未添加 MR12，待本批限定技术复核后才进入批准关联。
PlatformTypes §7.2/7.3、8.3.1–3 明确要求三个平台选择符以 `#define` 发布；
StandardTypes §8.2.2/3 明列四个状态宏的 `#define` 形式。
把它们改成 enum、私有常量或删除未消费的名字，不能保留同一预处理接口。
这里没有据此批准全部 CPU 枚举值或其他公共状态名字的宏形式。

OSEK OS 2.2.3 §11.2（印刷41页）明确要求服务标识 accessor，以及第一参数为 object identifier 的服务 accessor；
§13.8.4 把访问限制在 ErrorHook。下表只包括 OSEK 本身定义的12个首对象参数，不把其他参数、
输出指针或 AUTOSAR 扩展服务一起推定为必需。
四个 DeclareX 构造按 §13.2.2.1、13.4.2.1、13.5.2.1、13.6.2.1 及 §14.2.7 保留兼容语法；
ISR 与 ALARMCALLBACK 分别按 SWS_Os_00815、OSEK §13.6.5 保留定义形式。
构造参数位于声明/定义位置，不能替换成 enum 或有运行效果的函数调用。

R2.5 为 Advisory/System；主文印刷44/PDF52，TC1 将撤销计作用途，TC2.5/6 明确对象为宏定义。
这里承认当前程序中的真实未使用，不以公开可见或测试中的使用改写生产程序结果。
风险为公开预处理面；数值/展开式和原有消费者保持。类型、所选功能、宏展开、标准版次或包含闭包变化时重评。
授权拟按 ADR0023 的既有 AUTOSAR/OSEK 必需冲突处理，不请求新项目例外；技术必要性仍须独立复核。
ErrorHook 这里只处置宏发布的 R2.5，明确不批准参数副本的 R18.6，后者继续引用既有待决提案。

| 规则 | 精确范围 | 原文及控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `CPU_TYPE`、`CPU_BIT_ORDER`、`CPU_BYTE_ORDER` | SWS_Platform_00044/00045/00064、00043/00038、00046/00039；固定x64选择不变，公开预处理值保持。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Std_Types.h` 中 `STD_HIGH`、`STD_LOW`、`STD_ACTIVE`、`STD_IDLE` | SWS_Std_00007/00013 明列四个define及数值；保留头内常量表达式。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os.h` 中 `DeclareTask`、`DeclareResource`、`DeclareEvent`、`DeclareAlarm` | OSEK §13.2.2.1/13.4.2.1/13.5.2.1/13.6.2.1、§14.2.7；public-compatibility保持文件/块作用域声明及参数零求值。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os.h` 中 `ISR`、`ALARMCALLBACK` | SWS_Os_00815、OSEK §13.6.5；entry-bodies实际定义并运行入口，函数/ID绑定保持。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSErrorGetServiceId` | OSEK §11.2、13.8.4；error-hooks观察实际失败服务标识。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSError_GetTaskState_TaskID`、`OSError_ActivateTask_TaskID`、`OSError_ChainTask_TaskID`、`OSError_GetResource_ResID`、`OSError_ReleaseResource_ResID`、`OSError_SetEvent_TaskID`、`OSError_GetEvent_TaskID` | OSEK §11.2，§13.2/13.4/13.5 的首对象参数；现有error-hooks的服务/参数绑定保持。 | 待独立复核；ADR0023原授权范围 |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Hooks.h` 中 `OSError_GetAlarmBase_AlarmID`、`OSError_GetAlarm_AlarmID`、`OSError_SetRelAlarm_AlarmID`、`OSError_SetAbsAlarm_AlarmID`、`OSError_CancelAlarm_AlarmID` | OSEK §11.2、13.6 的首AlarmID；不把后续参数/输出指针纳入。 | 待独立复核；ADR0023原授权范围 |

## 构造宏的字面入口绑定（待复核）

Os.h 的 TASK/ISR 同时生成私有包装入口和用户函数体，OS_TASK_ENTRY/OS_ISR_ENTRY 在其他TU绑定同一入口。
现有标准ID为公开标量名字，`TASK(配置名)`、`ISR(配置名)`与服务调用共享配置短名；
直接以参数展开结果作函数名，会展开成ID数值；保留未展开的名字又会与该ID占用同一个C普通名字空间。
四个位置用 `##` 把字面配置名映射到不同函数名。普通外层宏转交会先展开ID，不能作为等价修复。
把构造改成普通函数、统一一个包装名字或按调用点生成任意计数名，不能保留多入口及跨TU绑定。
私有前缀本身不是SWS要求；候选必要性是固定C99标量ID和标准构造语法之间需要一一命名绑定。
改变ID表示/公开短名或给每个工程生成另一套绑定属于可研究的交付合同变更，不能当成已经等价。

R20.10为Advisory（主文印刷162/PDF170）；这里承认真实拼接，尚不添加MR12。
请在本批限定复核中核对SWS_Os_00815、OSEK §13.2.5/13.3及实际C名字空间/宏展开与替代边界；
若必需冲突推导不足，继续未闭合，不沿用R2.5发布义务代替这项判断。
既有entry-bodies、public-compatibility和完整映射消费者提供多入口、单次定义、身份及跨TU观察；
配置命名、ID表示、包装ABI、支持宏或预处理器变化时重评。
ARTI_TRACE的字符串化另属当前跟踪表示，不在这四项候选中，也不由此批准。

| 规则 | 精确范围 | 原文及控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 20.10 | `runtime/os/include/Os.h` 中 `TASK`、`OS_TASK_ENTRY`、`ISR`、`OS_ISR_ENTRY` | OSEK任务/ISR构造及SWS_Os_00815；固定C99公共标量ID与字面配置名到不同入口的一一绑定，真实entry-bodies及跨TU消费者保持。 | 待独立复核；ADR0023是否覆盖须由必要性推导决定 |
