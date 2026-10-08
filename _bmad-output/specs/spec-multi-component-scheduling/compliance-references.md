# R24-11 契约核定与直接依赖差距

2026-10-09 已实际取得九份官方PDF并与CP官方SHA-256清单匹配；原件仅保留本机，不入Git。以下是配置适用性与独立验收依据，不由ABI inventory或绿色测试替代。

| 官方文档 | SHA-256 | 本范围依据 |
| --- | --- | --- |
| AUTOSAR_CP_SWS_RTE.pdf | d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769 | §4.1.4.2–4.1.4.4、SWS_Rte_02008／02009实例约束；§4.2.3／ECUC_Rte_09023位置与task；§5.6.10／5.6.13标准Read／Call；SWS_Rte_06019、04515服务调用与串行化；§6.5 SchM |
| AUTOSAR_CP_TPS_SoftwareComponentTemplate.pdf | ea86a1b4b43c4ec0319b706e8696412ffb920cb6cc6422a911bedc1f0c9d59da | §6.3／6.4、constr_1068／1069变量与assembly兼容性；port ownership、runnable／event／access与同步operation关系 |
| AUTOSAR_CP_TPS_SystemTemplate.pdf | 5267a329104c2dcb029b3f713afa376733466a1b225a33498036a2f1ef017777 | TPS_SYST_01001、constr_5486／5487 SWC→ECU映射；DataMapping与SenderReceiverToSignalMapping的实例／端口／信号身份 |
| AUTOSAR_CP_SWS_OS.pdf | 6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44 | 已固定单核SC1；本次映射的task／event／alarm／schedule table、调用层级与资源所有权保持标准语义，实际tick由既有backend裁定 |
| AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf | 786ccab650f516f541e8d29b41f5c00e57ccd3de6ac1cad70db268f3ec7c5c28 | 声明的BSW entry／schedulable entity／exclusive area与真实符号、task上下文匹配，不能由新RTE生成猜接口 |
| AUTOSAR_CP_SWS_BSWGeneral.pdf | 4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b | SWS_BSW_00006内存映射与SWS_BSW_00115编码义务按实际BSW角色适用，C质量和运行契约分别验证 |
| AUTOSAR_CP_SWS_COM.pdf | 8addd33ad1e625b4e9d01afc938131d424f7161f18075f44d2a5fcfa925c5190 | §7.3／8.3／8.4；SWS_Com_00432 Init、00130 DeInit、00194 GetStatus、00197 SendSignal、00198 ReceiveSignal、00123 RxIndication、00001 TriggerTransmit，group／deadline按实际ECUC适用 |
| AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf | cf5aeee78fda6e5a25f982f04cb146ce4a75586de7ba41913d46b0cea1cc2407 | 既有DID USE_DATA_SYNCH_CLIENT_SERVER／UINT8_N消费者及无possibleErrors的生成server接缝，同源RTE回调和独立UDS向量 |
| AUTOSAR_CP_SWS_PDURouter.pdf | 3f97e95f6b36c876daeddc1bd90db7d0b24ebd0aa652078349d323ad6664ad6f | SWS_PduR_00334 Init/PostBuild config；00406 upper transmit；lower Rx/Trigger及TP buffer callbacks，实际COM/Dcm运输接缝 |

官方入口均为 `https://www.autosar.org/fileadmin/standards/R24-11/CP/<上述文件名>`。ECUC原件 `AUTOSAR_CP_MOD_ECUConfigurationParameters.zip` 已取得；身份必须匹配仓库official.json固定df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb。RteSwComponentInstance／RteEventToTaskMapping、OsTask／OsEvent／OsAlarm和所用Com关系按实际定义核对；研发oracle缺失不改变普通用户的内置规则入口。

## 当前代码与必须闭合的差距

- component.rs 将应用绑定到一个instance／两个uint32端口／一个DID，communication.rs只识别该组件；这阻止合法纯本地SWC。新增配置计划需按实际endpoint校验assembly与ECU归属，不能接受第二实例后仍生成一份状态。
- schedule.rs固定六实体和单TimingEvent；Ecu_Target.c的RUN_APPLICATION仅调用一个入口。需要共同派生完整位置／周期／event关系与实际任务调用，使本次所有periodic与server义务真实闭合。
- contracts.rs当前组件不区分重复端口的全局实现符号；新增组件必须使用各自header和确定的内部符号，检查保留名／文件名／guard碰撞，保持标准应用API形状。
- 已核对Com原件：当前runtime/include/Com.h的Com_RxIndication(size_t,data[8],now_ms)与Com_TriggerTransmit(size_t)不是标准callback形状；R24-11要求void Com_RxIndication(PduIdType,const PduInfoType*)与Std_ReturnType Com_TriggerTransmit(PduIdType,PduInfoType*)，后者是pull-copy而非主动发送。multi profile必须明确主机on-demand发送／时钟适配与标准callback，按真实data length、init/status和所选group/deadline行为修正生成生产者／PduR调用者／headers／符号描述／交付工件。不能只改名或typedef宣称标准对齐；新增标准接口必须实际执行对应copy／receive／生命周期语义，使用独立typed compile/link和成功／拒绝向量。旧profile以原ABI精确回归，不外推标准符合。
- 当前Rte.c.in生产Com_SendSignal／ReceiveSignal且与owner／CAN availability绑定；新multi接口须由明确Com模块责任提供，标准Send／Receive实际返回uint8，RTE按标准映射结果。SWS_Rte_06830／07822仅从COM_SERVICE_NOT_AVAILABLE映射COM_STOPPED；未分组PDU按SWS_Com_00840在Com_Init隐式启动且不可停止，CAN停机不影响缓冲Read／Write，lower TX失败独立；旧profile保留历史gate。local storage与local C/S也不继承CAN停止。Com配置类型、初始化／去初始化／status、选定周期／deadline、真实TxConfirmation和所有已消费callback属于8.2直接依赖出口，不能等R11或仅记录“host专属”后放行。实际MOD中ComIPduSignalProcessing、ComTxIPduUnusedAreasDefault、mainfunction timebase与ComSupportedIPduGroups需明确补齐，不能由不完整旧fixture推导默认；未选group／TP／dynamic signal等可选组合明确拒绝。标准Rx／deadline状态不能依赖旧host Dem存储成功，旧策略经明确适配／通知处理，不阻断标准callback。
- 旧contracts／Rte模板把无possibleErrors的server runnable声明为Std_ReturnType；R24-11 RTE§5.7.5.6／SWS_Rte_08913规定其server entry为void，客户端Rte_Call的Std_ReturnType则独立保留。新multi profile必须修正其真实应用／DID服务声明、实现、调用者与用户源码初始化；旧profile兼容入口维持原ABI，仅作为历史有界行为，不作为新契约预期。
- 网络DM具体依赖：SWS_Com_00772要求PDU属于启用Rx DM的group才启用monitoring；SWS_Com_00840的隐式start不绕过此条件。RTE SWS_Rte_08061／08062／08103／08104要求生成并处理实际COM timeout通知，NONE保留last value，不能直接沿用旧host epoch计时冒充该闭包。首批选择等待用户核定：最小Rx group＋标准DM，或无group且拒绝正数aliveTimeout；相关8.2出口在答复及真实验证前未通过。其余未选group／TP等组合的拒绝边界随实际选择明确调整，不自动扩为通用group支持。
- delivery/application/ownership/reopen目前仅接受一个Application.c槽；所有live来源、输出快照和可信slot必须共同扩展，sealed完整性不因“用户拥有”被豁免。

同类型多实例所需实例句柄／共享runnable ABI、跨task同步、任意类型／接口转换和R11多通道不适用于本首批配置：相关输入在生成前明确拒绝。后续消费者需要它们时以RTE§4.1.4.4、§5与SystemTemplate映射为依据扩展并运行独立目标验收；本次不以“host专属”推迟已支持配置的通用义务。真实MCU的内存段、驱动、中断和时间交给R10，退出条件是指定合法板卡／工具链与实际对端通过，不能继承本机证据。

MISRA基线为C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2；无授权主规范全文时如实保留逐条评估缺口，固定Cppcheck2.21工具结果不作为完整符合声明。

8.1 第二轮核对：R24-11 RTE SWS_Rte_07027 的 USE-VOID 签名与 CONSTR1286 的 primitive IN 限制不能被忽略，首批仅支持缺省/USE-ARGUMENT-TYPE并拒绝其他policy。CONSTR08936与08938分别禁止未显式启用映射的application/BSW事件持有task ref；新profile周期映射必须true，server为false且无task。固定XSD TimingEvent.offset为合法可选TIME-VALUE，显式零与缺省同属支持范围，非零拒绝。此为生成前配置闭包要求，不能替代8.2真实调度证据。

后续8.2直接接缝进一步核定：PduR原件已从官方取得并匹配CP清单SHA。当前 PduR_Init(const EcuConfig*)、PduR_DcmTransmit(data,length,now) 和 CanTp buffer helpers 与所选标准接口不能直接等同；R24-11 SWS_PduR_00334要求真实PduR_PBConfigType配置指针，00406的PduR_<User:Up>Transmit（含Dcm/Com）为Std_ReturnType(PduIdType,const PduInfoType*)；lower Rx为void(PduIdType,const PduInfoType*)，TP StartOfReception/CopyRxData/CopyTxData为所选BufReq_ReturnType/buffer-size闭包。8.2必须核对所选COM和DID的完整PduR上下层路由、init、buffer所有权与拒绝义务，同步配置/调用方/生成/公开typed消费者及真实成功和拒绝向量；主机时钟/状态通过明确适配，不侵入标准公开签名。旧profile保留其历史有界ABI，不把其绿色回归用于新profile标准证明。这不是R11可延期项；8.1声明契约未调用该运行链，没有在本阶段冒称修复。
