# ComM：公开接口与类型

本篇属于 [ComM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

ComM 提供的 C API 与回调按标准签名实现；`ComM_GetCurrentPNCComMode` 在 PNC 关闭时对每个已配置用户返回 `COMM_E_NO_PNC_ASSIGNED`。SW-C 经 RTE 使用的 `UR_`／`UM_` 端口、`ComM_UserRequest`／`ComM_CurrentMode` 接口及模式声明组没有生成，统一记为 [#195](https://github.com/g1331/autsaro/issues/195) 的差距；BswM 与 Dcm 的模式通知已按标准实现。

| 对象／条件 | 处置 |
| --- | --- |
| ComM_Init／ComM_DeInit | 已核对（ComM.h 声明 ComM_Init(const ComM_ConfigType *)、ComM_DeInit(void)；mode_contract.c 断言重复 Init 重启通道，DeInit 仅在 NO_PENDING_REQUEST 且总线实际为 NO 时生效，其余状态保持 COMM_INIT）：SWS_ComM_00146–00147、00794 |
| ComM_GetStatus 与 ComM_InitStatusType | 已核对（ComM.h 的 COMM_UNINIT＝0、COMM_INIT＝1；mode_contract.c 断言 Init 前、成功 DeInit 后为 UNINIT，空指针 E_NOT_OK）：SWS_ComM_00242、00668 |
| Init 前的 API 行为 | 已核对（ComM.c 在 configuration 为 NULL 时 API 不执行并返回 E_NOT_OK，回调与主函数直接返回；mode_contract.c 在 ComM_Init 之前断言 request、GetCurrentComMode、inhibition_status、classification、pnc_current 返回 E_NOT_OK 且出参保持，wake_up 无副作用；该行为不依赖 ComMDevErrorDetect）：SWS_ComM_00612 |
| ComM_RequestComMode | 已核对（签名与 E_OK／E_NOT_OK 返回按标准；仅接受 NO 与 FULL，未知用户、SILENT、WITH_WAKEUP 均 E_NOT_OK，COMM_E_MODE_LIMITATION 因限制关闭而不出现；mode_contract.c）：SWS_ComM_00110 |
| ComM_GetRequestedComMode／GetCurrentComMode／GetMaxComMode | 已核对（ComM.c 同名函数；GetMaxComMode 在无抑制、无限制时恒为 FULL，单通道故最低模式即该值；未知用户与空指针 E_NOT_OK 且出参保持；mode_contract.c）：SWS_ComM_00079–00083、00085、00374 |
| ComM_GetInhibitionStatus 与 ComM_SetECUGroupClassification | 已核对（GetInhibitionStatus 对配置通道恒写 0，未知通道、空指针与 Init 前 E_NOT_OK；SetECUGroupClassification Init 后 E_OK 但无行为依赖于它［抑制功能均被拒绝］；mode_contract.c 的 inhibition_status／classification 断言）：SWS_ComM_00552、00619 |
| ComM_GetCurrentPNCComMode | 已核对（PNC 关闭，已配置用户返回 COMM_E_NO_PNC_ASSIGNED 并把同一值写入出参 ComMode：01023 要求把它作为 ComMode 交出，91002 把它列为返回值，两种读法都满足；未知用户、空指针与 Init 前 E_NOT_OK；mode_contract.c 的 pnc_current 断言）：SWS_ComM_01023、91002 |
| 回调：CommunicationAllowed、EcuM 唤醒、Dcm 诊断 | 已核对（ComM_BswM.h、ComM_EcuM.h、ComM_Dcm.h 声明并由 ComM.c 实现，其他通道与 Init 前被忽略；mode_contract.c 与 diagnostic_contract.c［真实 Dcm 调用］）：SWS_ComM_00275、00871–00874 |
| 回调函数预编译实现 | 已核对（回调均为 ComM.c 中的普通函数，没有后构建分支）：SWS_ComM_00620 |
| 调度函数 ComM_MainFunction_<Channel> | 已核对（core 以通道 ShortName 生成 ComM_MainFunction_<Channel>，由 Ecu_RuntimeConfig.c 调用 ComM_RunChannel，Init 前不执行；modes.rs 的 mode_configuration_preserves_channel_users_rules_and_static_callouts 断言符号名）：SWS_ComM_00429、00818 |
| 必需与可选接口 | 已核对（源码：ComM.c 调用 Ecu_HostBusSM［<Bus>SM］、BswM_ComM_CurrentMode 与 Dcm_ComM_*ModeEntered；NM NONE 与无 NvM 不调用 Nm_*／NvM_*，也不调用 BswM_ComM_CurrentPNCMode、BswM_ComM_InitiateReset、Det 或 Nm 的 PNC 接口）：SWS_ComM_00828–00829 |
| 服务类型、用户句柄与配置结构 | 已核对（Rte_ComM_Type.h：ComM_ModeType 取值 0–3、ComM_UserHandleType 为 uint16 且 COMM_NOT_USED_USER_ID＝65535；ComM.h：ComM_StateType 取值 0–4、ComM_ConfigType 由生成的 comm_config 传入 ComM_Init；mode_contract.c 断言类型宽度与常量）：SWS_ComM_00162、00670、00672、00674、00863 |
| 导入类型 | 已核对（源码：ComM.h 引入 ComStack_Types.h［NetworkHandleType］与经 Rte_ComM_Type.h 的 Std_Types.h；PNCHandleType 与 NvM 类型只服务未选功能，未使用）：SWS_ComM_00820 |
| Std_ReturnType 扩展与抑制状态位域 | 已核对（ComM.h 的 COMM_E_MODE_LIMITATION／MULTIPLE_PNC_ASSIGNED／NO_PNC_ASSIGNED／LEARNING_ACTIVE 为 2–5；Rte_ComM_Type.h 按 SWS_Rte_03995–03997 定义 WakeupInhibitionActive 与 LimitedToNoCom 的 _BflMask／_BflPn／_BflLn；mode_contract.c 断言取值）：SWS_ComM_00669、91027 |
| SW-C 服务端口、模式端口与 RTE 模式通知 | 差距 #195：SWS_ComM_00091、00472、00662–00663、00675、00733、00744–00745、00778、00847–00848、01000–01001、01010–01012 |
| 版本 API | 不适用（配置：ECUC_ComM_00622）：SWS_ComM_00370 |
| 开发错误检测 | 不适用（配置：ECUC_ComM_00555）：SWS_ComM_00234、00858 |
| Nm 回调与 BusSM_BusSleepMode | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00383–00392、00792、91000 |
| PNC 回调、PNC 类型与同步关闭 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00673、91001、91028–91030 |
| 动态 PNC-to-channel mapping、学习与成员更新接口 | 不适用（配置：ECUC_ComM_00895）：SWS_ComM_01034–01049、01058、91013–91026、91102–91109 |
| 模式限制接口（ComM_LimitECUToNoComMode、LimitChannelToNoComMode、Inhibit counter） | 不适用（配置：ECUC_ComM_00560）：SWS_ComM_00108、00124、00163、00224、00740–00741、00743、00752、00802–00803、01006、01009 |
| 模式限制与强制 NO 复位的配置依赖 | 不适用（配置：ECUC_ComM_00558、ECUC_ComM_00560）：SWS_ComM_00800–00801 |
| 唤醒抑制接口（ComM_PreventWakeUp、ChannelWakeup） | 不适用（配置：ECUC_ComM_00559）：SWS_ComM_00156、00742、00747、00799、01008 |
| CurrentChannelRequest 端口与 fullComRequestors | 不适用（配置：ECUC_ComM_00787）：SWS_ComM_00734–00736、00904–00906、01005、01007 |
