# ComM：通道状态机与行为

本篇属于 [ComM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

NM NONE 的通道状态机只有 NO（NO_PENDING_REQUEST／REQUEST_PENDING）与 FULL（NETWORK_REQUESTED／READY_SLEEP）：只有 REQUEST_PENDING 评估 CommunicationAllowed，READY_SLEEP 不回到 NO，总线模式指示不改变状态而直接传播给 BswM 与 Dcm。进入 NO（含 Init）与 FULL 时在独占区外向总线状态机按序请求。

不适用项按所拒绝的参数归类：非 NONE 的 NM 变体、非 CDD 总线、抑制与限制、NvM 依赖、PNC 与动态映射。

| 对象／条件 | 处置 |
| --- | --- |
| 通道状态机、主／子状态与默认状态 | 已核对（ComM_StateType 与 ComM.c 的 channel_state／ComM_Evaluate；mode_contract.c 逐次断言 ComM_RunChannel 返回的子状态，Init 后默认为 NO_PENDING_REQUEST）：SWS_ComM_00051、00485、00880–00881、00898–00899 |
| 三个主状态 | 已核对（ComM_StateType 含 COMM_SILENT_COM；NM NONE 的通道只使用 NO 与 FULL，SILENT 只作为实际总线模式经 ComM_BusSM_ModeIndication 上报）：SWS_ComM_00879 |
| 目标模式、用户请求与请求存储 | 已核对（requests[] 按用户保存，ComM_Demand 取所有用户与诊断需求的最高者；mode_contract.c 以用户 17／43 请求，断言 SILENT 与 WITH_WAKEUP 被 E_NOT_OK 拒绝、后一请求覆盖前一请求并可由 ComM_GetRequestedComMode 回读）：SWS_ComM_00092、00151、00500、00625、00686、00839、00867–00868 |
| 请求触发 NO_COM 子状态 | 已核对（ComM_Evaluate：NO_PENDING 在用户 FULL、Dcm 活动诊断或 EcuM 唤醒时进入 REQUEST_PENDING，需求消失即回 NO_PENDING，只有 REQUEST_PENDING 评估 CommunicationAllowed；ComMModeLimitationEnabled 为 false，限制恒关闭；mode_contract.c 与 comm_cdd_mode_demand_and_immediate_bswm_use_actual_lower_modes）：SWS_ComM_00875–00876、00893、00895–00897 |
| CommunicationAllowed 标志 | 已核对（静态变量 allowed 由 ComM_Init 置 FALSE，只经 ComM_CommunicationAllowed 改变；mode_contract.c 断言其他通道的调用被忽略、Allowed 之前停在 REQUEST_PENDING）：SWS_ComM_00884–00885 |
| FULL 子状态与最小 Full 时长 | 已核对（minimum_remaining 在进入或重回 NETWORK_REQUESTED 时按 minimum_full_ticks 装载，ComM_RunChannel 每周期递减，到期且无 FULL 用户与活动诊断才进入 READY_SLEEP，READY_SLEEP 遇需求回 NETWORK_REQUESTED；mode_contract.c 的 minimum tick 1／2／3 断言）：SWS_ComM_00882–00883、00886、00889 |
| Dcm 活动诊断视为 FULL 请求 | 已核对（ComM_DCM_ActiveDiagnostic 置 diagnostic 并参与 ComM_Demand；diagnostic_contract.c 中真实 Dcm 经 ComM_DCM_ActiveDiagnostic／ComM_DCM_InactiveDiagnostic 保持与释放通信）：SWS_ComM_00866 |
| 进入 NO／FULL 时向总线状态机请求 | 已核对（ComM_ClaimBus／ComM_IssueBus 在独占区外按序调用 comm_config.request［Ecu_HostBusSM_RequestComMode］：Init 与进入 NO 请求 NO，进入 FULL 请求 FULL，被拒绝的目标由主函数重发；comm_parallel.c 的 comm_bus_requests_and_mode_publications_stay_ordered_while_another_waits_outside_the_area）：SWS_ComM_00069、00073、00402、00845–00846、00957–00958 |
| 实际模式查询 | 已核对（ComM_GetCurrentComMode 经 comm_config.current［Ecu_HostBusSM_GetCurrentComMode］取实际模式；唯一通道，用户的最低模式即该通道模式；mode_contract.c 的 current()）：SWS_ComM_00084、00176、00963 |
| 模式传播给 BswM 与 Dcm | 已核对（ComM_BusSM_ModeIndication 经 ComM_Publish 调用 BswM_ComM_CurrentMode 与已配置的 Dcm_ComM_*ModeEntered，Init 后的默认 NO 不通知，重复模式被过滤、并发时只发布最新值；mode_contract.c 的 indications／actions 断言与 bussm_parallel.c）：SWS_ComM_00266、00313、00861、00976 |
| 内部状态机对用户不可见 | 已核对（公开头只含 ComM_StateType 类型，ComM_RunChannel 位于私有 ComM_Internal.h，用户 API 只返回请求值或总线实际模式）：SWS_ComM_00191 |
| 被动唤醒不被抑制 | 已核对（ComM_EcuM_WakeUpIndication 与 ComM_BusSM_ModeIndication 没有任何抑制分支；ComMWakeupInhibitionEnabled 为 false）：SWS_ComM_00066 |
| 不调用 Nm | 已核对（ComM.c 不包含 Nm.h、不调用任何 Nm_*；mode.rs 只接纳 ComMNmVariant 为 NONE）：SWS_ComM_00602、00667 |
| FULL 变体的 Nm 请求、释放与启动 | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00133、00288、00665、00869–00870、00888、00902–00903、01016 |
| LIGHT 变体的 NmLightTimeout 与受管通道 | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00610、00891–00892、01095–01096 |
| SLAVE／PASSIVE 变体的 READY_SLEEP 与 BusSleep | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00915、01017–01018 |
| SILENT 状态及其 Nm 回调转换 | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00071、00295–00296、00583、00637、00826、00877–00878、00894 |
| Nm 启停同步 | 不适用（配置：ECUC_ComM_00568）：SWS_ComM_00261、00599 |
| COMM_BUS_TYPE_INTERNAL 与其他总线前缀 | 不适用（配置：ECUC_ComM_00567）：SWS_ComM_00664、00671、00962 |
| OA TC10 唤醒／睡眠请求（仅 Ethernet） | 不适用（配置：ECUC_ComM_00567）：SWS_ComM_01056–01057 |
| ComMSynchronousWakeUp 为 true 的全通道唤醒 | 不适用（配置：ECUC_ComM_00695）：SWS_ComM_00694 |
| 唤醒抑制与 ComMNoWakeup 的联合条件 | 不适用（配置：ECUC_ComM_00559、ECUC_ComM_00569）：SWS_ComM_00218 |
| ComMNoWakeup 的非易失保存 | 不适用（配置：ECUC_ComM_00569）：SWS_ComM_00157 |
| 唤醒抑制 | 不适用（配置：ECUC_ComM_00559）：SWS_ComM_00219、00302 |
| 限制到 NO_COMMUNICATION 模式 | 不适用（配置：ECUC_ComM_00560）：SWS_ComM_00215、00303、00842、00890 |
| 进入 NO 后清除用户请求 | 已核对（NM NONE 只有 ComM_Init 进入 NO 主状态，ComM_Init 把全部用户请求清为 NO；Figure 7.7 中 FULL 不回到 NO；mode_contract.c 在用户 17 请求 FULL 后重新 ComM_Init，断言其请求回到 NO）：SWS_ComM_00582 |
| 抑制总述、期间的请求状态与 Inhibit counter | 不适用（配置：ECUC_ComM_00559、ECUC_ComM_00560）：SWS_ComM_00138–00143、00182、00301、00470、00488、00840、01098 |
| 强制 NO 后复位 | 不适用（配置：ECUC_ComM_00558）：SWS_ComM_00355 |
| 非易失数据读写与读取失败 | 不适用（配置：ECUC_ComM_00559、ECUC_ComM_00560、ECUC_ComM_00839、ECUC_ComM_00895）：SWS_ComM_00103、00793、00864–00865、01101、91110 |
| PNC 开关、位向量与 Nm 交互 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00825、00910–00911、00953、00979–00982、01060–01062 |
| PNC 状态机结构与触发 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00827、00907–00909、00920–00924、00972、00978、00987 |
| PNC 的 NO_COMMUNICATION 主状态与唤醒 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00926、00932、00990、01063–01066 |
| PNC 的 FULL_COMMUNICATION／REQUESTED 及网关 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00929、00938、00991、01067–01084、01097 |
| PNC 的 READY_SLEEP／PREPARE_SLEEP | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00940–00952、01085–01089 |
| PNC 与用户、通道的关系 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_00912、00994、00996、01014–01015、01022、01024–01025、01094 |
| ComM_Init 的 PNC 映射与成员默认值 | 不适用（配置：ECUC_ComM_00839）：SWS_ComM_01099–01100 |
| 动态 PNC-to-channel mapping 学习与转发 | 不适用（配置：ECUC_ComM_00895）：SWS_ComM_01026–01029、01090–01093 |
