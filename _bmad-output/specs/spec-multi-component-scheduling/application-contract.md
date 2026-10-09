# 首批应用配置与开发契约

来源：候选 R6；继承根架构与 Epic 7 源权威、事务、预览及源码所有权契约。本配置是有界产品支持范围，不声明通用 RTE 完整符合。

## 输入与语义

| 对象 | 首批支持及拒绝边界 |
| --- | --- |
| ECU／组合 | 一个 ECU_EXTRACT、一个 ECU、一个平坦 CompositionSwComponentType；多个不同 ApplicationSwComponentType，每种一个实例；supportsMultipleInstantiation=false。重复类型实例、嵌套与 delegation 拒绝 |
| 身份 | full type path、composition instance path、port path、data element／operation path分别保留；短名不是身份。ECU 映射与 RTE instance 引用必须闭合 |
| 类型 | 显式非排队 uint32 S/R；应用 primitive→implementation→base type 映射核对。同步 C/S 的 uint32 IN／OUT／INOUT 与固定 uint8[4] OUT；无结构体、动态数组或转换。只接受同一受支持接口／数据类型，不把等宽视为兼容 |
| S/R | P→R 的 assembly connector；每个已使用 R 端点恰有一个本地生产者或一个外部信号映射，二者不能同时出现；P 可扇出。首批每个S/R端点都要求明确数值ComSpec初值，缺失拒绝，无隐式补零；首次发布前各R使用自己的初值，P与R值冲突时R优先（SWS_Rte_04501／04502）。首次write后fanout读同一last value，不合并不同R的初值。不同实例同短名仍隔离 |
| S/R freshness | 本地明确handleNeverReceived=false、aliveTimeout=0、handleTimeoutType=NONE、无invalidValue，初次Read按明确初值返回E_OK；不支持的其他组合拒绝。本地write不依赖CAN控制器。新multi接收I-PDU使用最小真实Rx group并实现标准DM及COM→RTE通知，支持已规划正数aliveTimeout；外部网络Read保持明确初值、NEVER_RECEIVED、MAX_AGE_EXCEEDED与超时保留值；COM_STOPPED只按实际COM_SERVICE_NOT_AVAILABLE映射（SWS_Rte_06830／07822），不能由CAN停止推导。未分组Tx PDU按SWS_Com_00840在Com_Init隐式启动，Rx组由生成的初始化显式启动／启用DM；CAN停止仍可Read缓冲／Write更新，底层发送失败独立。旧profile保持历史状态语义；NEVER_RECEIVED不是网络专属状态，未来本地handleNeverReceived=true须按SWS_Rte_07381–07383另行实现验收 |
| C/S | 本地同步调用、非重入服务器；一个 required operation 恰对应一个提供 operation 和 OperationInvokedEvent。每个接口至少一个operation，全部ComSpec与invoked event反向闭合；参数policy仅缺省或USE-ARGUMENT-TYPE，USE-VOID拒绝；无参数operation合法。首批不声明possible application errors，server runnable返回void（SWS_Rte_08913），客户端Rte_Call返回Std_ReturnType／E_OK或基础设施错误，不能混用两种返回形状。首批不支持递归调用环、并发、多任务服务、noReturnValueProvided或可能异步配置 |
| runnable／event | 非并发 runnable；TimingEvent 与 OperationInvokedEvent。每个周期runnable恰有一个TimingEvent，周期均为相同的正整数毫秒、offset=0；多TimingEvent指向同一runnable、其他event／offset／异周期拒绝。服务器不被周期调用或额外ActivateTask；每个OperationInvokedEvent仍必须有RteEventToTaskMapping容器，但无task／alarm／event／position引用，RteEventIsMappedToTask=false。应用与BSW周期mapping显式isMappedToTask=true并含task ref，缺省/false带task ref拒绝（RTE CONSTR08936/08938）；零offset显式与缺省等价。周期runnable的符号、访问点、event必须一致 |
| OS／SchM | 唯一 Extended Task_Ecu owner；沿既有 Alarm／ScheduleTable 支持选择同一周期触发闭包，按 RtePositionInTask 执行全部到期 runnable；BSW 消费输入→Com 更新时间→应用顺序→传输→诊断。全部状态只由 owner 访问是 SchM 无锁条件；非法 task／event／位置／周期拒绝 |
| CAN | 维持已支持的 11 位 Classical CAN／单通道／uint32 little-endian；同步 DID 仍通过实际 RTE 服务使用应用状态；已有 BSW 与受控 target 共同交付 |

支持多组件不要求每个 SWC 都具有 CAN 端口或 DID 服务；纯本地组件、仅生产者、仅消费者及服务提供／调用组件是必要输入形状。不得复制完整旧 EchoApplication 和它的专属 DID 约束给每个组件来代替模型扩展。

## 生成与源码归属

同一个私有、不可变、已验证计划消费所有输入关系，形成组件头、类型、RTE 实现、SchM、OS／BSW 配置、target 和构建清单。生成器不再猜测连接或自行重读 XML。一个外部符号有唯一生产者；C 标识符、文件名及 include guard 的碰撞必须拒绝。各 SWC 头只暴露自己的标准 API／runnable 声明；内部实现符号包含确定的组件身份，避免相同端口短名冲突。

用户源槽由可信计划派生，至少含 producerSlot、componentPath、sourcePaths、generatedHeaders、entrySymbols。多组件槽使用产品语义名称与 full instance identity，不能用 Epic 编号或前端拼接替代。live 输入是用户拥有源码，sealed 输出是快照；初始化只创建尚不存在的成员，绑定完整预览和当前成员／字节。不能初始化时自动覆盖或把 stale 源码当作有效。

扩展既有 ApplicationInput／InputSnapshot 数组及 ownership 校验，保持旧单槽精确读取规则。新 multi profile 显式标识 `singlecore-multi-swc-v1`；可以沿用 v2 的既有多条输入形状，但只允许当前可信计划公开的槽／输出路径；若实际格式变化必须显式版本分派。确认前复核每份 live 源及 manifest／规则／catalog 身份；导入时在新 live 工程目录恢复来源，不将 sealed payload 变为可写输入。

源码准备仍不依赖编译器或官方档案，构建／运行是可选用户动作但必要开发验收。沿已有事务编辑、对象检查、问题导航和源码初始化／预览入口呈现多组件对象与槽，不添加另一套配置或调度 UI；实际需要变更 UI 时继承 DESIGN 与 EXPERIENCE 并验证真实数据、窄窗口、长名称和中英语言。

## 后续 R11 接缝

本地 endpoint=(instance, port, data element／operation)，网络 mapping 另携 network／channel／system signal／Com signal／PDU／frame 完整身份。应用类型、实现类型与总线 bit length／endianness分别核对，不以 uint32 槽或 controller=0 充当未来通道身份。S/R 的本地 storage 与网络 transport 分离；后续 R11 对 signed／更宽类型、布局、29 位、FD、多通道分别核定转换与 channel 隔离，不继承本次 runtime 支持。

## 修改影响范围

已定位 integration/component、communication、schedule、plan、contracts、ecu 的单实例假设；generation application slots、arxml/application、delivery／ownership／reopen 与包内工具需同步核对。native rules、模板、原创 fixtures、所有 plan consumers 和 IPC／UI 按实际 DTO 影响同步迁移。仅修改本次必要调用链；不得更新 Epic 7 done／发行结论或无关运行模块。

8.2 的所选诊断模式依赖：单一 host CDD ComM channel（COMM_BUS_TYPE_CDD／Ecu_HostBusSM／NM NONE），保持真实 System CAN 身份。真实 BusSM 控制/查询 Can／CanIf模式，ComM消费诊断活动与 ordinary request，Dcm按 NoCom／Silent／Full通知门控，BswM消费所选模式规则；配置、状态与回调共同派生并独立验证。无完整 CanSM 或硬件模式完成符合声明，固定 R24-11 对 CDD prefix 的许可与后续 CAN variant 退出见 compliance-references。
