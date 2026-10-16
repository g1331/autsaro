# BswM：仲裁与动作

本篇属于 [BswM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

所选配置只有一个 IMMEDIATE 的 ComM 指示输入和三条规则（NO／SILENT／FULL 各一），每条规则的 true 列表只含一个固定的 BswMUserCallout，行为为在调用者上下文执行。其余仲裁特性（DEFERRED、事件、定时器、嵌套列表、TRIGGER、false 列表、其他动作）在生成前被拒绝，因此以配置条件处置。条款原文按固定 R24-11 SpecificationsARXML 核对。

| 对象／条件 | 处置 |
| --- | --- |
| IMMEDIATE 仲裁、端口取值与调用者上下文 | 已核对（BswM_ComM_CurrentMode 在调用者上下文按 IMMEDIATE 仲裁，BswM_Process 取最新端口值；runtime_contracts 的 comm_cdd_mode_demand_and_immediate_bswm_use_actual_lower_modes、bswm_actions_run_once_in_order_while_another_waits_outside_the_area）：SWS_BswM_00009、00013、00035、00059、00062、00281 |
| 输入期间到达的请求 | 已核对（BswM_ComM_CurrentMode 的 processing／postponed 协议：动作期间到达的每个输入在当前处理者的动作之后各仲裁一次，仲裁前取最新指示值［SWS_BswM_00281］；bswm_parallel.c 的 indicate_while_acting 断言两次推迟的输入各执行一次动作）：SWS_BswM_00069 |
| 规则、动作列表与动作调用 | 已核对（mode.rs 生成三条 BswMRule，Ecu_BswMAction 由 BswM_Process 作为唯一动作列表调用；单个 BswMActionListItem，Index 0 或省略的 Priority 按 0；生成的 Ecu_BswMAction 与 BswMUserCallout 文本一一对应）：SWS_BswM_00010、00012、00016、00017、00038、00040、00054、00147、00260、00277、SWS_BswM_CONSTR_00001 |
| 条件执行与首次评估 | 已核对（三条规则 BSWM_CONDITION、BswMRuleInitState=BSWM_FALSE、无 false 列表；BswM_Init 不评估规则，首次指示命中的规则执行其 true 列表，未命中的规则无动作；mode_contract.c 的 NO／SILENT／FULL 序列）：SWS_BswM_00066、00115 |
| 表达式只引用当前输入 | 已核对（mode.rs 的 BswMArgumentRef 只接受 BswMModeCondition，BswMConditionType 只接受 BSWM_EQUALS 且引用同一 BswMModeRequestPort；Ecu_BswMAction 不保存上次结果）：SWS_BswM_00117 |
| BSWM_AND 求值 | 已核对（mode.rs 只接受 BSWM_AND 或省略运算符，单个 BswMModeCondition 参数；Ecu_BswMAction 的 switch 对匹配模式执行，其余规则为 false 且无 false 列表）：SWS_BswM_00245 |
| 初始模式值 | 已核对（mode.rs 要求 BswMModeInitValue 存在且 BswMBswModeInitValue 为 COMM_NO_COMMUNICATION，与 ComM_Init 后的 NO 一致；唯一输入仅在指示时仲裁，首次更新前不评估规则，因此初值不需要另行写入 port_mode；BswMCompuScaleModeValue 不在内置 metadata 中）：SWS_BswM_00203 |
| DEFERRED 仲裁与主函数处理 | 不适用（配置：ECUC_BswM_00822）：SWS_BswM_00014、00060、00061、00068、00075、00282 |
| 事件请求端口与事件条件 | 不适用（配置：ECUC_BswM_00815）：SWS_BswM_00252、00253、00256 |
| 事件请求端口与 ClearEventRequest | 不适用（配置：ECUC_BswM_01052）：SWS_BswM_00193、00251、00254、00255、00257–00259、00278、00284、00285 |
| BSWM_TRIGGER 触发执行 | 不适用（配置：ECUC_BswM_00894）：SWS_BswM_00011、00023 |
| false 动作列表 | 不适用（配置：ECUC_BswM_00818）：SWS_BswM_00015、00116 |
| 嵌套动作列表与规则链接 | 不适用（配置：ECUC_BswM_00825）：SWS_BswM_00018、00019、00037、00067、00276 |
| 多条规则共享顶层动作列表 | 不适用（配置：ECUC_BswM_00817）：SWS_BswM_00223 |
| 多个顶层动作列表的优先级 | 不适用（配置：ECUC_BswM_01076）：SWS_BswM_00275 |
| AbortOnFail | 不适用（配置：ECUC_BswM_00902）：SWS_BswM_00055 |
| BSWM_AND 以外的逻辑运算符 | 不适用（配置：ECUC_BswM_00814）：SWS_BswM_00242–00244、00246–00248 |
| 未定义的模式条件 | 不适用（配置：ECUC_BswM_00932）：SWS_BswM_00064、00241 |
| 任意 BSW 函数调用动作 | 不适用（配置：ECUC_BswM_00843）：SWS_BswM_00039 |
| 定时器与等待功能 | 不适用（配置：ECUC_BswM_00856）：SWS_BswM_00220、00261–00265 |
| SW-C 模式请求、通知与端口 | 不适用（配置：ECUC_BswM_00856）：SWS_BswM_00189、00200、00201、00236 |
| 模式开关端口 | 不适用（配置：ECUC_BswM_00802）：SWS_BswM_00202、SWS_BswM_CONSTR_00006 |
| BswMCompuScaleModeValue 条件值 | 不适用（配置：ECUC_BswM_00816）：SWS_BswM_CONSTR_00002 |
| 多分区 BswM | 不适用（配置：ECUC_BswM_00895）：SWS_BswM_00320 |
| RTE／PDU 组／Deadline Monitoring 动作 | 不适用（配置：ECUC_BswM_00826）：SWS_BswM_00271–00274、SWS_BswM_CONSTR_00003、00004 |
| EcuM 驱动初始化列表动作 | 不适用（配置：ECUC_BswM_00941）：SWS_BswM_00269 |
| EthIf 动作 | 不适用（配置：ECUC_BswM_01072）：SWS_BswM_00286 |
| J1939Dcm 广播状态掩码 | 不适用（配置：ECUC_BswM_00987）：SWS_BswM_00249、00250 |
| Sd 服务组动作 | 不适用（配置：ECUC_BswM_01047）：SWS_BswM_00280、SWS_BswM_CONSTR_00005 |
| Sd 控制规则、端口与接口 | 不适用（配置：ECUC_BswM_01093）：SWS_BswM_00287–00319、SWS_BSWM_91006–91018、SWS_BswM_CONSTR_00007 |
| 运行时错误 BSWM_E_ACTION_FAILED | 不适用（配置：ECUC_BswM_01050）：SWS_BswM_00238–00240 |
| 说明性条款 | 不适用（归属：SWS_BSWModeManager 附录，无实现义务）：SWS_BswM_NA_09999 |
