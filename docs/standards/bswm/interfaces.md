# BswM：公开接口与类型

本篇属于 [BswM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

公开接口只有 BswM_Init、BswM_Deinit、BswM_ComM_CurrentMode 与 BswM_MainFunction；其余输入 API 随对应 BswM<Module>Enabled 开关关闭而不提供。Det 开发错误与运行时错误上报未选择。

| 对象／条件 | 处置 |
| --- | --- |
| 初始化、反初始化与调用顺序 | 已核对（BswM_Init／BswM_Deinit 位于 BswM.h；Init 重置配置与推迟输入，processing 有意留给在途动作结算，port_mode 在每次输入时先写入再仲裁，首次输入前不被读取；仅使用 SchM_Enter／Exit_BswM_BSWM_STATE，Ecu_StackBootstrap 在 SchM_Start 之后调用；bswm_parallel.c 的 deinitialize_while_acting、restart_while_acting）：SWS_BswM_00002、00043、00044、00118–00120 |
| BswM_ComM_CurrentMode | 已核对（BswM.h／BswM_ComM.h 中的标准签名；仅接受 BswMComMChannelRef 引用的通道及 NO／SILENT／FULL；mode_contract.c 的 bswm 函数指针签名检查）：SWS_BswM_00047、00283 |
| BswM_MainFunction | 已核对（SchM_BswM.h 声明标准签名，函数体为空：所选配置没有 DEFERRED 输入；mode_contract.c 断言 All-IMMEDIATE 主函数不重复任何动作）：SWS_BswM_00053 |
| 类型与导入类型 | 已核对（BswM.h：BswM_ModeType／BswM_UserType 为 uint16，mode_contract.c 断言尺寸；BswM_ConfigType 为结构；实现只使用 ComM.h 与 Std_Types 的 NetworkHandleType、ComM_ModeType、Std_ReturnType）：SWS_BswM_00001、00041、00042、00213–00216、00237 |
| 接口依赖与私有头文件 | 已核对（BswM.c 不调用任何外部强制或可选接口，仅调用生成的动作；交付头文件只有 BswM.h、BswM_ComM.h、SchM_BswM.h 与 BswM_MemMap.h，对应 BswMComMEnabled）：SWS_BswM_00007、00008、00279 |
| BswM_GetVersionInfo | 不适用（配置：ECUC_BswM_00812）：SWS_BswM_00003 |
| BswM_RequestMode 与通用请求 | 不适用（配置：ECUC_BswM_00949）：SWS_BswM_00046 |
| 开发错误检测与 Det 上报 | 不适用（配置：ECUC_BswM_00811）：SWS_BswM_00045、00077–00103、00109–00113、00132、00133、00149、00150、00153、00154、00159、00205、00206、00208、00209、00211、00212、00227–00230、00266–00268 |
| CanSM 接口 | 不适用（配置：ECUC_BswM_00938）：SWS_BswM_00049 |
| Dcm 接口 | 不适用（配置：ECUC_BswM_00940）：SWS_BswM_00048、00158 |
| EcuM 接口 | 不适用（配置：ECUC_BswM_00941）：SWS_BswM_00131、91003、91004 |
| EthIf 接口 | 不适用（配置：ECUC_BswM_01072）：SWS_BswM_91001 |
| EthSM 接口 | 不适用（配置：ECUC_BswM_00942）：SWS_BswM_00050 |
| FrSM 接口 | 不适用（配置：ECUC_BswM_00943）：SWS_BswM_00051 |
| LinSM 接口 | 不适用（配置：ECUC_BswM_00944）：SWS_BswM_00052、00058 |
| LinTp 接口 | 不适用（配置：ECUC_BswM_00945）：SWS_BswM_00156 |
| NvM 接口 | 不适用（配置：ECUC_BswM_00946）：SWS_BswM_00104、00152 |
| J1939Dcm 接口 | 不适用（配置：ECUC_BswM_00987）：SWS_BswM_00165 |
| J1939Nm 接口 | 不适用（配置：ECUC_BswM_00965）：SWS_BswM_00194 |
| Nm 接口 | 不适用（配置：ECUC_BswM_01071）：SWS_BswM_00235、91002 |
| Sd 接口 | 不适用（配置：ECUC_BswM_01047）：SWS_BswM_00204、00207、00210 |
| ComM PNC 指示 | 不适用（配置：ECUC_ComM_00839）：SWS_BswM_00148 |
| ComM 复位指示 | 不适用（配置：ECUC_ComM_00558）：SWS_BswM_00217 |
| SoAd 接口 | 不适用（配置：ECUC_BswM_00856）：SWS_BswM_91005 |
