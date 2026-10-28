# 标准发布信息的未使用宏

本篇只记录当前四 profile 中的公开声明诊断，不把接口存在等同于符合性。
授权依据是 ADR 0023 及其原用户决定；以下为待独立复核的具体记录，尚不添加源码 MR12。
保留原始诊断、源码身份和 not_assessed，不改变规则或基线。

## 固定契约

SWS_BSW_00059 要求所有模块头提供受多重定义保护的发布信息 #define；SWS_BSW_00256 明列 MODULE_ID、AR_RELEASE_MAJOR／MINOR／REVISION 与 SW_MAJOR／MINOR／PATCH 等字段。表中原文要求是预处理定义，不能用 enum、私有常量或删除未消费字段代替。当前所选程序未读取的这些宏保留真实 R2.5 Advisory 违反。

宏仅发布版本／身份常量，不增加可变状态；原有公共头首包含、错误依赖版本及收件者编译验证保持。名称、字段来源、目标版次或发布接口变化时重评。供应方身份、采用代码和任意其他宏不在此记录。

| 规则 | 精确范围 | 原文及控制 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/BswM.h` 中 `BSWM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/BswM_Version.h` 中 `BSWM_AR_RELEASE_REVISION_VERSION`、`BSWM_SW_MAJOR_VERSION`、`BSWM_SW_MINOR_VERSION`、`BSWM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/CanIf_Version.h` 中 `CANIF_AR_RELEASE_REVISION_VERSION`、`CANIF_SW_MAJOR_VERSION`、`CANIF_SW_MINOR_VERSION`、`CANIF_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/CanTp_Version.h` 中 `CANTP_AR_RELEASE_REVISION_VERSION`、`CANTP_SW_MAJOR_VERSION`、`CANTP_SW_MINOR_VERSION`、`CANTP_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Can_Version.h` 中 `CAN_AR_RELEASE_REVISION_VERSION`、`CAN_SW_MAJOR_VERSION`、`CAN_SW_MINOR_VERSION`、`CAN_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Com.h` 中 `COM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/ComM.h` 中 `COMM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/ComStack_Version.h` 中 `COMSTACKTYPE_AR_RELEASE_REVISION_VERSION`、`COMSTACKTYPE_SW_MAJOR_VERSION`、`COMSTACKTYPE_SW_MINOR_VERSION`、`COMSTACKTYPE_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Com_Version.h` 中 `COM_AR_RELEASE_REVISION_VERSION`、`COM_SW_MAJOR_VERSION`、`COM_SW_MINOR_VERSION`、`COM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Dcm.h` 中 `DCM_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Dcm_Version.h` 中 `DCM_AR_RELEASE_REVISION_VERSION`、`DCM_SW_MAJOR_VERSION`、`DCM_SW_MINOR_VERSION`、`DCM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Det.h` 中 `DET_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Det_Version.h` 中 `DET_AR_RELEASE_REVISION_VERSION`、`DET_SW_MAJOR_VERSION`、`DET_SW_MINOR_VERSION`、`DET_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/LSduR.h` 中 `LSDUR_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/LSduR_Version.h` 中 `LSDUR_AR_RELEASE_REVISION_VERSION`、`LSDUR_SW_MAJOR_VERSION`、`LSDUR_SW_MINOR_VERSION`、`LSDUR_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/MemMap_Version.h` 中 `MEMMAP_AR_RELEASE_REVISION_VERSION`、`MEMMAP_SW_MAJOR_VERSION`、`MEMMAP_SW_MINOR_VERSION`、`MEMMAP_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/PduR.h` 中 `PDUR_MODULE_ID` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/PduR_Version.h` 中 `PDUR_AR_RELEASE_REVISION_VERSION`、`PDUR_SW_MAJOR_VERSION`、`PDUR_SW_MINOR_VERSION`、`PDUR_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Platform_Types.h` 中 `PLATFORM_AR_RELEASE_REVISION_VERSION`、`PLATFORM_SW_MAJOR_VERSION`、`PLATFORM_SW_MINOR_VERSION`、`PLATFORM_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Rte_Version.h` 中 `RTE_AR_RELEASE_REVISION_VERSION`、`RTE_SW_MAJOR_VERSION`、`RTE_SW_MINOR_VERSION`、`RTE_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/bsw/include/Std_Types.h` 中 `STD_TYPES_AR_RELEASE_REVISION_VERSION`、`STD_TYPES_SW_MAJOR_VERSION`、`STD_TYPES_SW_MINOR_VERSION`、`STD_TYPES_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Arti_Version.h` 中 `ARTI_AR_RELEASE_REVISION_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
| MISRA C:2012 Rule 2.5 | `runtime/os/include/Os_Version.h` 中 `OS_AR_RELEASE_REVISION_VERSION`、`OS_SW_MAJOR_VERSION`、`OS_SW_MINOR_VERSION`、`OS_SW_PATCH_VERSION` | SWS_BSW_00059／00256；保留固定4.10.0及本模块软件／身份字段，首包含消费者观察。 | 维护者 原用户决定，ADR 0023（待本批独立复核） |
