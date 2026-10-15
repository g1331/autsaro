# Dcm：传输、会话与服务

本篇属于 [Dcm](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

同步 ReadData 使用调用期独立缓冲区，执行结束后只结算同一 request generation 的结果。Init 不允许旧 TP 生命周期混入新连接：已进入下层的发送必须先静默结束；尚在发送调用栈中的旧身份则保留到确认，旧 CopyTx 明确失败。

PduR 发送先预留，再在 Dcm 区内提交 admission；NoCom／SilentCom 可撤销尚未提交的预留。并发调用以提交点排序，已提交的下层调用保留其确认身份；区外调用期间的后续模式通知禁止下一次 admission，不撤销正在执行的 TP 调用。

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
| 功能寻址 NRC 抑制 | 不适用（配置：ECUC_Dcm_00710）：SWS_Dcm_00001 |
| 请求通知与确认 | 不适用（配置：ECUC_Dcm_00681、ECUC_Dcm_00816）：SWS_Dcm_00677–00678、00741–00742 |
| ISO 引用的独立原文裁决 | 差距 #57：SWS_Dcm_00027、00030、00827、01075、01622、01666、01668–01669、01671 |
| UDS Services | 已核对（配置只接纳 0x10／0x22／0x3E；其余服务按 #33／#124 移交 R7 并拒绝，不声明完整 UDS 支持）：SWS_Dcm_00442、01641 |
| 多协议优先级、抢占及跨协议并行 | 不适用（配置：ECUC_Dcm_00695）：SWS_Dcm_00015、00079、00460、00575–00576、00727、00729、01046–01047、01050、01367、01369–01372、01602–01605、SWS_Dcm_CONSTR_06102–06103 |
| Types | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00969–00970 |
| Data types constraints | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_CONSTR_06002 |
| Dcm_StartOfReception | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_01672–01673、01677–01678 |
| Support of segmented response (paged-buffer) | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_01059、01186 |
| Manage security level | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00020、00033、00139、01329 |
| Manage authentication state | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_01477、01479–01480 |
| Allow to modify timings | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00143、01679 |
| Different service tables | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00035 |
| Parallel diagnostic protocol processing | 已核对（inspect_transport 校验 DCM_DEFAULT_SESSION／DCM_EXTENDED_DIAGNOSTIC_SESSION 与 level 1／3）：SWS_Dcm_CONSTR_06000–06001 |
| Communication Mode Handling | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_CONSTR_06027 |
| Introduction | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00178 |
| Support checking the diagnostic service identifier and adapting the diagnostic message | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00196、SWS_Dcm_CONSTR_06047 |
| Verification functionality | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_01535 |
| 未配置请求通知回调 | 不适用（配置：ECUC_Dcm_00681、ECUC_Dcm_00816）：SWS_Dcm_01474 |
| Distribution of diagnostic message to DSP submodule | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00221 |
| Initiate transmission | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00236 |
| Assemble response | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00039 |
| Paged buffer 响应长度 | 不适用（配置：ECUC_Dcm_00776）：SWS_Dcm_00038 |
| 带 ErrorCode 的应用服务 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00777）：SWS_Dcm_00228、01414–01415 |
| Negative Response Codes handling | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00275 |
| Diagnostic mode declaration groups | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00777、00781、00807、01327–01328 |
| CommunicationControl mode prototype | 不适用（配置：ECUC_Dcm_00900）：SWS_Dcm_00780 |
| ResponseOnEvent mode prototype | 不适用（配置：ECUC_Dcm_00858）：SWS_Dcm_00933–00934 |
| Service 0x10 - Diagnostic Session Control | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00250 |
| Service 0x22 - ReadDataByIdentifier | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00253 |
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
| OBD mirror 模式 | 不适用（配置：ECUC_Dcm_01061）：SWS_Dcm_00481、00483、01195 |
| OBD DID | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00482、01384 |
| OBD DID support info | 不适用（配置：ECUC_Dcm_01104）：SWS_Dcm_01382–01383 |
| 非 OPAQUE 数据序列化 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00986）：SWS_Dcm_01386 |
| Error Classification | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00044 |
| Development Errors | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_00040 |
| Runtime Errors | 已核对（Dcm.c、Dcm.h、Rte_Dcm_Type.h 与 ecu_config／artifacts 生成样本；详见概览与服务篇）：SWS_Dcm_01416 |
| 无非易失数据服务 | 不适用（配置：ECUC_Dcm_00713、ECUC_Dcm_00971、ECUC_Dcm_01151、ECUC_Dcm_00759）：SWS_Dcm_00870、01048 |
| 同步 ReadData 不接纳 pending | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00024、00119、01567 |
| 无 service／subservice session filter | 不适用（配置：ECUC_Dcm_00734、ECUC_Dcm_00804）：SWS_Dcm_00211、00616 |
| 无 service／subservice security filter | 不适用（配置：ECUC_Dcm_00733、ECUC_Dcm_00812）：SWS_Dcm_00217、00617 |
| 无 service／subservice mode rule | 不适用（配置：ECUC_Dcm_00918、ECUC_Dcm_00924）：SWS_Dcm_00773–00774 |
| Not applicable requirements | 已核对（原始 SpecificationsARXML 中的 NA 条目；是规范本身的追溯表，不是本实现未核查条目的豁免）：SWS_Dcm_NA_00999 |

`responsePending` 时清除 SPRMIB 的义务不在所选同步处理路径上。不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00203
