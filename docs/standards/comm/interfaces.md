# ComM：公开接口与类型

本篇属于 [ComM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

ComM 提供的 C API 与回调按标准签名实现；`ComM_GetCurrentPNCComMode` 在 PNC 关闭时对每个已配置用户返回 `COMM_E_NO_PNC_ASSIGNED`。`ComMService` 经共同类型化 C/S、port-defined argument 和显式 SWC-BSW mapping 提供五项用户服务；每个用户有独立的 `UR_{user_name}`／`UM_{user_name}` 端口。通知按保存的用户／通道关系发布，当前配置只接纳一个 CDD／NM NONE 通道，多通道最低模式聚合由 [#464](https://github.com/g1331/autsaro/issues/464) 承接。

R24-11 的 `SWS_ComM_01000` 表把 Request／GetRequested 的两项 API 映射交叉书写；本实现按同条 IN／OUT 参数、`SWS_ComM_00745` 连接及两项 API 正文绑定同名 API。`SWS_ComM_00662` 与 00744 的旧示例使用 UM000 序列，01010／01011 的端口定义表使用 ECUC 用户名；交付采用端口定义表的名称，用户顺序由配置对象路径确定，端口常量独立使用 ComMUserIdentifier。`ComMMode` 的 ALPHABETIC_ORDER 为 FULL＝0、NO＝1、SILENT＝2，桥接层按名称转换 ComM C 枚举。

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
| 服务组件外部视图与内部行为 | 已核对（两份参考 services.arxml 的 ComMService、五项 operation-invoked runnable、ComMMapping 及每用户 port argument；comm_service_has_standard_operations_and_alphabetic_mode_values 核对所选真实 ComM.c producer）：SWS_ComM_00744–00745 |
| 用户请求接口及端口 | 已核对（ComM_UserRequest 五项操作、参数和各自错误集合；UR_HostUser 的 ComM_UserHandleType 常量；comm_service_rejects_nonstandard_input_at_its_source 定位拒绝错误方向和用户句柄；rte_comm_contract.c 经两个 R-port 请求并回读独立用户）：SWS_ComM_00848、01000、01011 |
| 模式接口、模式声明与每用户端口 | 已核对（ComM_CurrentMode/currentMode、ComMMode 的 ALPHABETIC_ORDER 和默认 NO、UM_HostUser；comm_two_users_keep_distinct_handles_publishers_and_mapping_relations 核对两用户独立 publisher；声明／初值反例定位到 ComMMode）：SWS_ComM_00733、00847、01001、01010、01012 |
| 用户顺序与模式端口对应 | 已核对（mode.rs 按配置路径排序用户，comm_service.rs 逐一比较 UR_／UM_ 名称和用户句柄；名称按上述端口定义表处理，rte_comm_contract.c 分别观察用户 0／43 的模式端口）：SWS_ComM_00662 |
| ComM 承担逐用户 fanout | 已核对（ComM_NotifyUsers 遍历不可变 user_modes，按各用户 channels 关系选择通道模式；rte_comm_contract.c 的两个独立 SchM publisher 计数均随实际模式变化增加）：SWS_ComM_00091 |
| 主模式变化及初始化例外 | 已核对（ComM_Publish 的 indicated／comm_published 去重保留；rte_comm_contract.c 检查初始 NO 零通知、SILENT／FULL／NO 变化、重复 FULL 零新增以及回调重入后的最终 NO）：SWS_ComM_00472 |
| BusSM 实际模式向上传播 | 已核对（ComM_BusSM_ModeIndication 经单飞 ComM_Publish 在 COMM_STATE 外调用用户桥、BswM_ComM_CurrentMode 与 ComM_NotifyDcm；rte_comm_contract.c 读回 RTE 实际模式，mode_contract.c／bussm_parallel.c 保留真实下层模式及 BswM／Dcm 通知回归）：SWS_ComM_00675 |
| 每用户 UM 端口对应正确 API | 已核对（ecu_config::files 为各用户生成独立 Ecu_ComMMode_UM_* 桥接函数，分别调用对应 SchM_Switch_ComM_UM_*；rte_comm_contract.c 同时观察两个 Rte_Mode，错绑第二用户 publisher 的变异被拒绝）：SWS_ComM_00778 |
| 一个用户关联多个通道时的最低模式聚合 | 差距 #464：SWS_ComM_00663（当前一个通道的关系计算由 mode.rs 的 channels 和 ComM_NotifyUsers 覆盖；comm_multichannel_configuration_is_rejected_at_the_added_channel 在第二通道源对象拒绝，多通道聚合和隔离待承接项实现） |
| 版本 API | 不适用（配置：ECUC_ComM_00622）：SWS_ComM_00370 |
| 开发错误检测 | 不适用（配置：ECUC_ComM_00555）：SWS_ComM_00234、00858 |
| Nm 回调与 BusSM_BusSleepMode | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00383–00392、00792、91000 |
| PNC 回调、PNC 类型与同步关闭 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00673、91001、91028–91030 |
| 动态 PNC-to-channel mapping、学习与成员更新接口 | 不适用（配置：ECUC_ComM_00895）：SWS_ComM_01034–01049、01058、91013–91026、91102–91109 |
| 模式限制接口（ComM_LimitECUToNoComMode、LimitChannelToNoComMode、Inhibit counter） | 不适用（配置：ECUC_ComM_00560）：SWS_ComM_00108、00124、00163、00224、00740–00741、00743、00752、00802–00803、01006、01009 |
| 模式限制与强制 NO 复位的配置依赖 | 不适用（配置：ECUC_ComM_00558、ECUC_ComM_00560）：SWS_ComM_00800–00801 |
| 唤醒抑制接口（ComM_PreventWakeUp、ChannelWakeup） | 不适用（配置：ECUC_ComM_00559）：SWS_ComM_00156、00742、00747、00799、01008 |
| CurrentChannelRequest 端口与 fullComRequestors | 不适用（配置：ECUC_ComM_00787）：SWS_ComM_00734–00736、00904–00906、01005、01007 |
