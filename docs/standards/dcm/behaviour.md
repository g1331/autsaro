# Dcm：传输、会话与服务

本篇属于 [Dcm](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

同步 ReadData 使用调用期独立缓冲区，执行结束后只结算同一 request generation 的结果。Init 不允许旧 TP 生命周期混入新连接：已进入下层的发送必须先静默结束；尚在发送调用栈中的旧身份则保留到确认，旧 CopyTx 明确失败。

PduR 发送在 Dcm 区内一次判定 FullCom 并提交 admission，这是发送与模式通知、Init 之间唯一的排序点。之后到达的 NoCom／SilentCom 只禁止下一次 admission，已提交的下层调用保留其确认身份；不持锁调用 PduR 时这一窗口无法消除，再次检查只会移动它。

主线仅配置同步、四字节、OPAQUE 数据读接口，不承诺应用异步处理，也不在没有 OpStatus 的回调之外发明 pending 协议。应用须在同步调用内提供有效数据或替代值；RTE 基础设施失败时丢弃部分数据并按通用失败响应处理。

| 对象／条件 | 处置 |
| --- | --- |
| 未选择的 UDS／OBD 服务 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00004–00005、00008、00077、00135、00243–00249、00255–00260、00279、00284、00286–00287、00289、00293、00295、00330、00373、00376–00382、00386、00388–00389、00392–00414、00417–00423、00464–00470、00473、00488–00496、00499、00502–00505、00511–00512、00519、00562–00571、00574、00579–00581、00587–00590、00594、00621–00623、00628、00632、00639–00641、00643–00647、00680–00682、00684、00701、00703–00705、00707–00708、00718、00723–00726、00751–00753、00757–00759、00766、00783–00786、00818、00821–00823、00825–00826、00828–00829、00834、00837–00841、00852–00861、00866–00869、00894、00944、00947–00948、00956–00958、00966–00967、00972–00974、01043、01051–01053、01055–01057、01060–01061、01063、01067、01077–01083、01088、01090–01091、01129–01134、01139–01141、01158、01167、01169–01171、01173、01191–01193、01226–01230、01232–01233、01240、01252–01254、01263、01265、01268–01269、01273–01275、01277、01330–01334、01343–01346、01358–01364、01390、01392–01393、01399–01400、01402–01406、01408、01410、01417–01422、01433–01458、01496、01534、01545–01547、01554–01557、01564、01580–01586、01591–01598、01600–01601、01607–01612、01614–01620、01623–01632、01640、01642–01665、SWS_Dcm_CONSTR_06018、06030、06039、06045–06046、06048、06050–06051、06059–06060、06063–06066、06068–06069、06071–06073、06080–06082、06084–06086、06097、06099–06100、06104–06107 |
| SecurityAccess 与持久化 attempt counter | 不适用（配置：ECUC_Dcm_00759）：SWS_Dcm_00252、00321、00323–00325、00659–00660、00862–00863、01150、01154–01156、01349–01357、01397、01599、SWS_Dcm_CONSTR_06074–06078 |
| Authentication、角色、白名单与异步加密 | 不适用（配置：ECUC_Dcm_01151）：SWS_Dcm_01459–01473、01475–01476、01493、01503–01504、01506–01518、01521–01533、01536–01542、01544、01551、01559–01562、01565、01579、01588、01621、01681–01683、SWS_Dcm_CONSTR_06087–06088、06091–06096、06098 |
| 周期诊断发送与确认 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00122–00123、00125–00126、00721–00722、00820、00843、01096–01099、01101、01103–01106、01111–01112、01115、01117–01118、01552–01553、01568–01578、01613 |
| Generic connection 与 MetaData 寻址 | 不适用（配置：ECUC_Dcm_01081）：SWS_Dcm_00848–00849、01347–01348、01429、SWS_Dcm_CONSTR_06044 |
| Secure coding 与 BndM | 不适用（配置：ECUC_Dcm_01245）：SWS_Dcm_01684–01696、SWS_Dcm_CONSTR_06108–06113 |
| 安全事件上报 | 不适用（配置：ECUC_Dcm_01187）：SWS_Dcm_01589–01590、01702–01753 |
| 制造商请求通知 | 不适用（配置：ECUC_Dcm_00681）：SWS_Dcm_00218、00462–00463、01172、01321 |
| 供应商请求通知 | 不适用（配置：ECUC_Dcm_00816）：SWS_Dcm_00516–00518、01322 |
| 环境条件与 mode rule | 不适用（配置：ECUC_Dcm_00932）：SWS_Dcm_00782、00808、00810–00815、00942、01119、SWS_Dcm_CONSTR_06028–06029、06089–06090 |
| 复合／嵌套 DID 数据 | 不适用（配置：ECUC_Dcm_01231）：SWS_Dcm_01633–01639 |
| Sender/Receiver、NvData 与 ECU signal 数据访问 | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00964、SWS_Dcm_CONSTR_06053 |
| OpStatus／pending 异步处理 | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00120、00527–00530、00760、01184、01187–01189、01412–01413 |
| Bootloader 与重启恢复交互 | 不适用（配置：ECUC_Dcm_00815）：SWS_Dcm_00532、00535–00537、00592、00654、00715、00719–00720、00767–00768、00995、00997、01163–01164、01175、01177–01183、01185、01423–01425 |
| 可选 VIN 预读与 GetVin | 不适用（配置：ECUC_Dcm_00984）：SWS_Dcm_01174 |
| DID range | 不适用（配置：ECUC_Dcm_00937）：SWS_Dcm_CONSTR_06020–06021 |
| 非 OPAQUE 数据端序 | 不适用（配置：ECUC_Dcm_00986）：SWS_Dcm_00971 |
| 其他元素宽度 | 不适用（配置：ECUC_Dcm_00985）：SWS_Dcm_CONSTR_06035–06036 |
| Routine 信号 | 不适用（配置：ECUC_Dcm_00640）：SWS_Dcm_CONSTR_06008、06011 |
| PID 信号 | 不适用（配置：ECUC_Dcm_00626）：SWS_Dcm_CONSTR_06012、06031、06040–06041 |
| S/R 可变长度约束 | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_CONSTR_06026 |
| 多客户端与协议资源 | 不适用（配置：ECUC_Dcm_00695、ECUC_Dcm_00705）：SWS_Dcm_01674–01675 |
| 拒绝第二请求的 NRC21 选项 | 不适用（配置：ECUC_Dcm_00914）：SWS_Dcm_01676 |
| 周期发送 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_01072–01073 |
| 无 Tx 的连接 | 不适用（配置：ECUC_Dcm_00711）：SWS_Dcm_01166 |
| Paged buffer | 不适用（配置：ECUC_Dcm_00776）：SWS_Dcm_00028、01058、SWS_Dcm_CONSTR_06055 |
| SecurityAccess 配置 | 不适用（配置：ECUC_Dcm_00759）：SWS_Dcm_CONSTR_06083、06101 |
| Authentication 可选状态转换 | 不适用（配置：ECUC_Dcm_01151）：SWS_Dcm_01481–01489 |
| S3 覆盖配置 | 不适用（配置：ECUC_Dcm_01240）：SWS_Dcm_01680 |
| 异步 force response pending | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_01143 |
| RoE 与周期模式门控 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00149–00150、00154–00155、00159–00160、00162 |
| 外置服务 OpStatus | 不适用（配置：ECUC_Dcm_00777）：SWS_Dcm_00732–00733、00735 |
| 功能寻址：NRC 抑制与并发 TesterPresent（仅以功能寻址接收） | 不适用（配置：ECUC_Dcm_00710）：SWS_Dcm_00001、01666–01667 |
| 请求通知与确认 | 不适用（配置：ECUC_Dcm_00681、ECUC_Dcm_00816）：SWS_Dcm_00677–00678、00741–00742 |
| ISO 计时及DSL直接闭包 | 已核对（下文固定原文与角色；dcm_contract.c response_deadline／session_transport_boundaries，生成配置与封存协议消费者）：SWS_Dcm_00027、00030 |
| ISO 请求检查与NRC顺序 | 已核对（ISO14229-1图5/6/15；Dcm_ReadData先核算总容量，dcm_contract.c read_validation_priority／failure_and_dispatch）：SWS_Dcm_00827、01075、01622 |
| ISO 消息接收 | 已核对（ISO14229-2 §7/8及图13；dcm_contract.c admission_and_copy／session_transport_boundaries）：SWS_Dcm_01671 |
| ISO S3生命周期 | 已核对（ISO14229-2表6/10及图13；dcm_contract.c session_transport_boundaries）：SWS_Dcm_01668 |
| ISO 会话连接归属 | 已核对（单物理连接配置拒绝及dcm_contract.c session_connection_identity，外来PDU不续期）：SWS_Dcm_01669 |
| UDS 服务表（所选 0x10／0x22／0x3E 由各服务条目验证，其余 SID 生成前拒绝，由 R7 承接） | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00442、01641 |
| 多协议优先级、抢占及跨协议并行 | 不适用（配置：ECUC_Dcm_00695）：SWS_Dcm_00015、00079、00460、00575–00576、00727、00729、01046–01047、01050、01367、01369–01372、01602–01605、SWS_Dcm_CONSTR_06102–06103 |
| Types | 已核对（diagnostic.rs 只接纳 UINT8_N／OPAQUE；Dcm.c 原样透传数据字节）：SWS_Dcm_00969–00970 |
| Data types constraints | 已核对（源码：diagnostic.rs 要求 UINT8_N 的 DcmDspDataByteSize 为 4，缺省即拒绝）：SWS_Dcm_CONSTR_06002 |
| Dcm_StartOfReception | 已核对（dcm_contract.c：StartOfReception／TpRxIndication 的结果映射）：SWS_Dcm_01672–01673 |
| Support of segmented response (paged-buffer) | 已核对（超长响应回 NRC 0x14，见 dcm_contract.c ordered_reads_and_limits；E_BUSY 见 diagnostic_contract.c）：SWS_Dcm_01059、01186 |
| Manage security level | 已核对（Dcm_GetSecurityLevel 返回 LOCKED；dcm_contract.c main：Init 后为 0）：SWS_Dcm_00020、00033；不适用（配置：ECUC_Dcm_00759）：SWS_Dcm_00139、01329 |
| Manage authentication state | 已核对（每连接一个 authentication 发布者，Dcm_Init 发布 deauthenticated；dcm_contract.c／diagnostic_contract.c 断言）：SWS_Dcm_01477、01479–01480 |
| Allow to modify timings | 已核对（源码：Dcm 不设最小响应延迟，即 P2min／P2*min 为 0）：SWS_Dcm_00143；已核对（diagnostic.rs 取 S3 5000 ms，Overwrite 由 ECUC_Dcm_01240 拒绝）：SWS_Dcm_01679 |
| Different service tables | 已核对（diagnostic.rs 解析 DcmDslProtocolSIDTable；diagnostics.rs 核对表内容）：SWS_Dcm_00035 |
| Parallel diagnostic protocol processing | 已核对（inspect_transport 校验 DCM_DEFAULT_SESSION／DCM_EXTENDED_DIAGNOSTIC_SESSION 与 level 1／3）：SWS_Dcm_CONSTR_06000–06001 |
| Communication Mode Handling | 已核对（Dcm.h 提供 Dcm_SetActiveDiagnostic；dcm_contract.c gates_and_reentrancy 验证）：SWS_Dcm_CONSTR_06027 |
| Introduction | 已核对（dcm_contract.c failure_and_dispatch：NRC 0x11／0x12／0x13／0x31）：SWS_Dcm_00178 |
| Support checking the diagnostic service identifier and adapting the diagnostic message | 已核对（源码：Dcm_Process 内置分派，未列 SID 回 0x11；diagnostic.rs 拒绝重复 SID）：SWS_Dcm_00196、SWS_Dcm_CONSTR_06047 |
| Verification functionality | 已核对（源码：仅 SID 校验；其余检查的配置源由 ECUC_Dcm_00681／01151／00734／00733／00816／00918 拒绝）：SWS_Dcm_01535 |
| 未配置请求通知回调 | 不适用（配置：ECUC_Dcm_00681、ECUC_Dcm_00816）：SWS_Dcm_01474 |
| Distribution of diagnostic message to DSP submodule | 已核对（源码：Dcm_Process 按 SID 调用 Dcm_SessionRequest／Dcm_ReadData）：SWS_Dcm_00221 |
| Initiate transmission | 已核对（dcm_contract.c sessions_and_keepalive：确认转交会话处理，E_OK 提交）：SWS_Dcm_00236 |
| Assemble response | 已核对（源码：resData／resDataLen 不含 SID，Dcm_ProcessRequest 补 SID 字节）：SWS_Dcm_00039 |
| Paged buffer 响应长度 | 不适用（配置：ECUC_Dcm_00776）：SWS_Dcm_00038 |
| 带 ErrorCode 的应用服务 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00777）：SWS_Dcm_00228、01414–01415 |
| Negative Response Codes handling | 已核对（dcm_contract.c ordered_reads_and_limits：无支持 DID 回 NRC 0x31）：SWS_Dcm_00275 |
| Diagnostic mode declaration groups | 已核对（artifacts.rs 生成 provided mode group；contracts.rs 核对）：SWS_Dcm_00777、00781、00807、01327；不适用（配置：ECUC_Dcm_00759）：SWS_Dcm_01328 |
| CommunicationControl mode prototype | 不适用（配置：ECUC_Dcm_00900）：SWS_Dcm_00780 |
| ResponseOnEvent mode prototype | 不适用（配置：ECUC_Dcm_00858）：SWS_Dcm_00933–00934 |
| Service 0x10 - Diagnostic Session Control | 已核对（dcm_contract.c sessions_and_keepalive／failure_and_dispatch）：SWS_Dcm_00250 |
| Service 0x22 - ReadDataByIdentifier | 已核对（dcm_contract.c ordered_reads_and_limits）：SWS_Dcm_00253 |
| DID 认证权限 | 不适用（配置：ECUC_Dcm_01151）：SWS_Dcm_01548–01549 |
| 动态 DID 来源 | 不适用（配置：ECUC_Dcm_00612）：SWS_Dcm_00651–00653、00864–00865 |
| 已禁用的 DID | 不适用（配置：ECUC_Dcm_00805）：SWS_Dcm_00561 |
| DID security filter | 不适用（配置：ECUC_Dcm_00614）：SWS_Dcm_00435 |
| DID mode rule | 不适用（配置：ECUC_Dcm_00917）：SWS_Dcm_00819 |
| ConditionCheckRead | 不适用（配置：ECUC_Dcm_00955）：SWS_Dcm_00439 |
| 引用其他 DID | 不适用（配置：ECUC_Dcm_00606）：SWS_Dcm_00440、SWS_Dcm_CONSTR_06023 |
| 非 C/S ECU signal／FNC | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_CONSTR_06057、06061–06062 |
| 未指定／其他端序 | 不适用（配置：ECUC_Dcm_00986）：SWS_Dcm_CONSTR_06058 |
| 显式 DID 长度 | 不适用（配置：ECUC_Dcm_01099）：SWS_Dcm_01431 |
| 原子 DID 与 BndM | 不适用（配置：ECUC_Dcm_01122）：SWS_Dcm_01587 |
| ECU signal、S/R 与原子数据接口 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_01122）：SWS_Dcm_00578、00638、01432 |
| 动态数据长度 | 不适用（配置：ECUC_Dcm_00985）：SWS_Dcm_00436 |
| 默认端序回退 | 不适用（配置：ECUC_Dcm_00986）：SWS_Dcm_CONSTR_06070 |
| OBD mirror 模式 | 不适用（配置：ECUC_Dcm_01061）：SWS_Dcm_00481–00483、01195 |
| OBD DID | 不适用（配置：ECUC_Dcm_01099）：SWS_Dcm_01384 |
| OBD DID support info | 不适用（配置：ECUC_Dcm_01104）：SWS_Dcm_01382–01383 |
| 非 OPAQUE 数据序列化 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00986）：SWS_Dcm_01386 |
| Error Classification | 已核对（源码：Dcm.c 不报告章节外的错误值）：SWS_Dcm_00044 |
| Development Errors | 不适用（配置：ECUC_Dcm_00823）：SWS_Dcm_00040 |
| Runtime Errors | 不适用（配置：ECUC_Dcm_00735、ECUC_Dcm_00713、ECUC_Dcm_01245）：SWS_Dcm_01416 |
| 无非易失数据服务 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00971、ECUC_Dcm_01151、ECUC_Dcm_00759）：SWS_Dcm_00870、01048 |
| 同步 ReadData 不接纳 pending | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00024、00119、01567 |
| 无 service／subservice session filter | 不适用（配置：ECUC_Dcm_00734、ECUC_Dcm_00804）：SWS_Dcm_00211、00616 |
| 无 service／subservice security filter | 不适用（配置：ECUC_Dcm_00733、ECUC_Dcm_00812）：SWS_Dcm_00217、00617 |
| 无 service／subservice mode rule | 不适用（配置：ECUC_Dcm_00918、ECUC_Dcm_00924）：SWS_Dcm_00773–00774 |
| Not applicable requirements | 已核对（原始 SpecificationsARXML 中的 NA 条目；是规范本身的追溯表，不是本实现未核查条目的豁免）：SWS_Dcm_NA_00999 |

`responsePending` 时清除 SPRMIB 的义务不在所选同步处理路径上。不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00203

## 跨标准原文与适用闭包

原件身份、收到字节摘要及来源限制集中在 [#57](https://github.com/g1331/autsaro/issues/57#issuecomment-6366451292)。本次使用 ISO 14229-1:2013（第二版）、14229-2:2021（第二版）及15765-3:2004（第一版）；2013会话层副本不替代2021。原件保存在仓库外持久资料库，全文／页面不进入产品Git或CI。收到字节匹配不是出版方独立认证或完整符合性结论。

ISO14229-1 §7.5图5/6（印刷页20/21）、§9.2图7（37页）、§10.2图15（110页），以及ISO14229-2 §9.3图7（17页）、§10.1.4.2图13（29–31页）均按渲染原页核对。适用角色为单物理UDS server，默认／扩展会话、0x10/0x22/0x3E；不把客户端计时、功能寻址、多客户端或未选服务的能力写成已实现。15765-3 §6.1/6.2沿用UDS应用服务，§6.3表3/4/8及物理时序的网络无关部分对应同一S3和传输失败处理；CAN专属传输状态由CanTp承担；§9.2.1表27–29与§9.2.5表34的会话／TesterPresent子功能及P2编码与所选值一致，§9.3.1未增加0x22限制。R24-11明确的P2/P2*最小值0（00143）、S3缺省5000ms（01679）、FullCom门控（01142）继续成立。

配置从`diagnostic.rs::inspect_transport/inspect_service`进入同一校验与生成计划，拒绝功能寻址、第二协议／连接、异步数据端口及未选SID；两会话使用同一P2/P2*，P2*须可按10ms编码。`schedule.rs`固定1ms counter与BSW周期，`ecu_config.rs`生成Dcm配置和PduR TP路由。实际接缝为CanIf→LSduR→CanTp→PduR→Dcm，发送按反向路径调用下层。独立C consumer使用生成RTE类型头，封存HostBatch按公开CAN帧输入验证真实应用及网络闭包。

- **请求顺序与消息。** 图5先检查SID；图6先检查最小两字节、子功能是否支持，再进入具体服务的精确长度检查。0x10／0x3E在私有请求副本中去除SPRMIB，正响应可以抑制，负响应必须发送。0x10响应携带P2的毫秒值和P2*的10ms值，正响应成功确认才发布会话；抑制时本地完成。重复请求当前会话仍成功并重启其计时；无SecurityAccess、RoE、周期诊断状态需要迁移。
- **0x22。** §10.2表146／图15要求先验最小长度、偶数DID字节数与最大DID数，然后筛选支持DID并检查总响应容量，最后读取。未知DID跳过，已支持项保持输入顺序和重复次数；无支持项回0x31，容量不足回0x14，不调用应用。F186为当前会话，应用数据固定四字节；无可选安全／条件过滤。RTE失败按既有通用失败返回0x10并丢弃部分数据；它不能遮盖先发生的格式／容量拒绝。
- **计时与消息边界。** ISO14229-2 §7/8及§9表3/4/6将StartOfReception映射到T_DataSOM.ind、TpRxIndication映射到T_Data.ind、PduR_DcmTransmit映射到T_Data.req、TpTxConfirmation映射到T_Data.conf。S3仅在空闲非默认会话递减，接收开始后暂停，最终响应确认（含失败）或抑制响应本地完成后重新装载；接收错误也重启而不执行请求。未知服务仍续期，外来PDU不能续期。§9.7表10要求发送错误后不重传，实际拒绝／确认失败均释放且不重试。
- **P2与P2*。** §9.1.1 REQ5.4/5.6、§9.2及图13区分最终响应与扩展pending等待。本配置同步处理，不接纳OpStatus／DCM_E_PENDING；P2*按0x10广告，不启动异步pending计时器。P2从完整接收后起算，FullCom在最后允许主函数周期到达仍可发送，过期则丢弃未发答案并释放。函数周期和同步应用执行预算属于所选受控调度条件；逻辑周期验收不外推宿主硬实时或实机时延保证。

`dcm_contract.c`覆盖容量不足与应用失败同时发生时的NRC优先级、零应用调用、精确容量、长度／最大DID数优先级、S3接收与确认等待暂停、两类传输失败后重启、未知服务／抑制TesterPresent续期、外来PDU、会话确认及P2门控边界。`diagnostics.rs`／`diagnostic_configuration.rs`保留相应配置拒绝，`integration::protocol`以封存ECU核对S3边界和物理帧。测试／变异及分析原始结果记录在PR；条款处置仅限上述选择，#58的221项评估、#59身份和#187服务端口保持各自出口。
