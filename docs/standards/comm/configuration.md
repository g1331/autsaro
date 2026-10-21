# ComM：ECUC 配置与选择边界

本篇属于 [ComM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

内置 metadata 只含所选配置所需的 ComM 容器与参数；ComMGeneral 与 ComMChannel 中被拒绝的开关必须为 false，ComMBusType、ComMCDDBusPrefix 与 ComMNmVariant 只接纳 CDD、`Ecu_HostBusSM` 与 NONE，主函数周期必须等于 OS counter tick。被拒绝参数在表中以自身为依据处置；内置 metadata 未定义的 PNC、NvM、受管通道与 Ethernet 唤醒请求参数不能出现在输入中，按其所依附的拒绝参数处置。

| 对象／条件 | 处置 |
| --- | --- |
| 模块与容器结构 | 已核对（内置 ECUC metadata［builtin.rs 的 mode_management］与 mode.rs 的 one()／child()／containers()：ComM、ComMGeneral、ComMConfigSet、唯一 ComMChannel、ComMNetworkManagement、ComMUser、ComMUserPerChannel 的重数与父子关系；modes.rs 的配置接纳测试）：ECUC_ComM_00554、00565、00607、00653、00657、00879、00890 |
| 用户与通道标识、用户映射 | 已核对（内置 ECUC metadata 与 mode.rs：ComMUserIdentifier 为 0–65534 且互不相同，ComMChannelId 为 uint8，每个 ComMUser 恰有一个指向唯一通道的 ComMUserPerChannel［1–32 个用户］；modes.rs 以 Vehicle 通道、用户 19 等替代值接纳）：ECUC_ComM_00635、00654、00658、SWS_ComM_00995 |
| 主函数周期与最小 Full 时长 | 已核对（内置 ECUC metadata 与 mode.rs：ComMMainFunctionPeriod 必须等于 OS counter tick［MODE_TIMEBASE，modes.rs 以 0.002 拒绝］，ComMTMinFullComModeDuration 取 1–65000 ms 换算为 minimum_full_ticks）：ECUC_ComM_00556–00557 |
| 总线与 NM 选择 | 已核对（内置 ECUC metadata 与 mode.rs：ComMBusType 只接纳 COMM_BUS_TYPE_CDD、ComMCDDBusPrefix 只接纳 Ecu_HostBusSM、ComMNmVariant 只接纳 NONE；modes.rs 的 selected_mode_configuration_rejects_unbound_features_and_dynamic_callouts 以 CAN、OtherBusSM、FULL 在源对象处拒绝）：ECUC_ComM_00567–00568、00888 |
| ComMEcuGroupClassification | 已核对（内置 ECUC metadata［0–255，默认 3］与 mode.rs 解析为 uint8；唤醒抑制与模式限制均被拒绝，值只被校验、不进入生成输出，也不影响运行行为；modes.rs 以替代值 1 接纳）：ECUC_ComM_00563 |
| 用户与通道的分区引用 | 不适用（配置：ECUC_ComM_00786）：ECUC_ComM_00786 |
| 通道分区与跨分区调用 | 不适用（配置：ECUC_ComM_00894）：ECUC_ComM_00894、SWS_ComM_01019 |
| 只在 BusSM 与 Dcm 所在分区调用它们 | 已核对（分区引用在生成前拒绝，ComM、Ecu_HostBusSM 与 Dcm 在同一分区由同一主线调度；ComM_IssueBus 与 ComM_Publish 在调用者上下文直接调用）：SWS_ComM_01020、01059 |
| 用户到通道的静态映射 | 已核对（mode.rs 的 MODE_USER_MAPPING 要求用户经 ComMUserPerChannel 静态映射到唯一通道；生成的 comm_config.users 为 const 数组，ComM_User 按它解析）：SWS_ComM_00795–00798 |
| 配置检查、配置与实现分离、交付形式 | 已核对（mode.rs 在生成前静态校验全部 ComM／BswM 参数，ComM_Valid 在 Init 再校验；ComM.c 不含配置常量，配置由生成的 Ecu_RuntimeConfig.c 经 ComM_ConfigType 提供；runtime/bsw/src/ComM.c 以源码交付并参与生成样本编译；ComMBusType 在 metadata 中按通道配置）：SWS_ComM_00322、00419、00459–00464 |
| 发布版本信息 | `ComM.c` 独立源端 release 与 `ComM_Version.h` 三分量一致性已有编译正反例；软件政策及软件一致性仍为差距 #59：SWS_ComM_00418 |
| ComMVersionInfoApi | 不适用（配置：ECUC_ComM_00622）：ECUC_ComM_00622 |
| ComMDevErrorDetect | 不适用（配置：ECUC_ComM_00555）：ECUC_ComM_00555 |
| ComMModeLimitationEnabled | 不适用（配置：ECUC_ComM_00560）：ECUC_ComM_00560 |
| ComMWakeupInhibitionEnabled | 不适用（配置：ECUC_ComM_00559）：ECUC_ComM_00559 |
| ComMResetAfterForcingNoComm | 不适用（配置：ECUC_ComM_00558）：ECUC_ComM_00558 |
| ComMSynchronousWakeUp | 不适用（配置：ECUC_ComM_00695）：ECUC_ComM_00695 |
| ComMNoCom | 不适用（配置：ECUC_ComM_00571）：ECUC_ComM_00571、SWS_ComM_00690 |
| ComMNoWakeup | 不适用（配置：ECUC_ComM_00569）：ECUC_ComM_00569 |
| ComMNoWakeUpInhibitionNvmStorage | 不适用（配置：ECUC_ComM_00789）：ECUC_ComM_00789 |
| ComMFullCommRequestNotificationEnabled | 不适用（配置：ECUC_ComM_00787）：ECUC_ComM_00787 |
| ComMGlobalNvMBlockDescriptor | 不适用（配置：ECUC_ComM_00559、ECUC_ComM_00560、ECUC_ComM_00839、ECUC_ComM_00895）：ECUC_ComM_00783 |
| ComMDynamicPncToChannelMappingSupport | 不适用（配置：ECUC_ComM_00895）：ECUC_ComM_00895–00896 |
| ComMPncSupport 及 PNC 容器、参数与约束 | 不适用（配置：ECUC_ComM_00839）：ECUC_ComM_00839、00841–00878、00880–00887、00891–00892、00897、00899–00900、SWS_ComM_CONSTR_00001、00004–00006 |
| NmLightTimeout 与 SLAVE_PASSIVE 约束 | 不适用（配置：ECUC_ComM_00568）：ECUC_ComM_00606、SWS_ComM_CONSTR_00003 |
| Ethernet TC10 唤醒／睡眠请求 | 不适用（配置：ECUC_ComM_00567）：ECUC_ComM_00898 |
| 受管／管理通道 | 不适用（配置：ECUC_ComM_00893）：ECUC_ComM_00893、SWS_ComM_CONSTR_00002 |
| ComMDirectUserMapping | 不适用（归属：配置工具：自动创建 PNC／通道 ComMUser 属配置工具行为；产品只接纳显式 ComMUser，且内置 metadata 不定义该参数）：ECUC_ComM_00840 |
| 规范自述不适用条目 | 不适用（归属：规范文档：该条仅声明本规范无适用要求，不含 ComM 实现义务）：SWS_ComM_NA_00499 |
