# Dcm：ECUC 配置与选择边界

本篇属于 [Dcm](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

内置 metadata 保留已纳入定义的类型、默认值、范围和重数；集成器另核对消费语义。只允许物理 UDS-on-CAN、单协议／单连接、有 Rx 和 Tx、会话 1／3 和服务 0x10／0x22／0x3E。应用 DID 可省略；诊断可完整关闭；半路由拒绝。

主线应用数据固定四字节 UINT8_N、显式 OPAQUE、单信号 offset 0、只读且没有动态定义、权限／mode-rule／ConditionCheckRead 扩展。DID session refs 必须匹配支持的会话；S3 未覆写时为 5000 ms。未知或未消费字段在源位置拒绝，不默认为已实现。Dcm 还核对 DemGeneral 中的 DemOBDSupport，拒绝 OBD-relevant ECU 声明，避免把 no-OBD 的 F6xx 解释扩展到其他 ECU。

| 对象／条件 | 处置 |
| --- | --- |
| SecurityAccess 与持久化 attempt counter | 不适用（配置：ECUC_Dcm_00759）：ECUC_Dcm_00725–00726、00754–00755、00757、00759–00760、00762、00764、00967–00969、01048–01050、01101、01210 |
| Authentication、角色、白名单与异步加密 | 不适用（配置：ECUC_Dcm_01151）：ECUC_Dcm_01151–01164、01166–01171、01173–01178、01186、01211–01212、01241–01244 |
| Secure coding 与 BndM | 不适用（配置：ECUC_Dcm_01245）：ECUC_Dcm_01245–01249 |
| 安全事件上报 | 不适用（配置：ECUC_Dcm_01187）：ECUC_Dcm_01188、01250–01275 |
| 制造商请求通知 | 不适用（配置：ECUC_Dcm_00681）：ECUC_Dcm_00681 |
| 供应商请求通知 | 不适用（配置：ECUC_Dcm_00816）：ECUC_Dcm_00816 |
| 环境条件与 mode rule | 不适用（配置：ECUC_Dcm_00932）：ECUC_Dcm_00925–00932、00949、01037、01123–01129、01179 |
| 复合／嵌套 DID 数据 | 不适用（配置：ECUC_Dcm_01231）：ECUC_Dcm_01231 |
| Sender/Receiver、NvData 与 ECU signal 数据访问 | 不适用（配置：ECUC_Dcm_00713）：ECUC_Dcm_00989–00992、01009–01011 |
| DID range | 不适用（配置：ECUC_Dcm_00937）：ECUC_Dcm_00937–00941、00945–00948、00950、01067 |
| 内存访问与 upload/download | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00784–00791、00793、00911、00913、00916、00920、00963–00964、01068–01077、01086–01089、01132–01133、01137–01138 |
| 文件传输 | 不适用（配置：ECUC_Dcm_01034）：ECUC_Dcm_01034–01036、01130–01131 |
| 非 ReadData 数据操作与 I/O control | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00616–00621、00623–00625、00922–00923、01057–01060、01142–01143 |
| ReadScalingInformation | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00993 |
| VehicleInformation | 不适用（配置：ECUC_Dcm_00630）：ECUC_Dcm_00630–00631、00727、00888–00891、01051 |
| RoutineControl | 不适用（配置：ECUC_Dcm_00640）：ECUC_Dcm_00640–00641、00664、00724、00752–00753、00807、00831–00848、00850、00867、00881–00885、01013–01017、01021–01023、01052–01054、01063、01090–01091、01093–01096、01116–01121、01144–01146、01215–01216、01219–01230 |
| OBD RequestControl | 不适用（配置：ECUC_Dcm_00637）：ECUC_Dcm_00637、00656、00722–00723、01078 |
| DTC 服务、内存选择与 ControlDTCSetting | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00935–00936、00965、01064–01066、01147–01150、01214 |
| CommunicationControl | 不适用（配置：ECUC_Dcm_00900）：ECUC_Dcm_00900–00905、00943–00944、01030–01033、01045–01046 |
| CommonAuthorization | 不适用（配置：ECUC_Dcm_01025）：ECUC_Dcm_01025–01028 |
| OBD DID support info | 不适用（配置：ECUC_Dcm_01102、ECUC_Dcm_01104）：ECUC_Dcm_01097–01098、01100、01102–01104 |
| 替代／scaling 数据接口 | 不适用（配置：ECUC_Dcm_00713）：ECUC_Dcm_00994–01001、01055–01056、01062、01084–01085、01233 |
| OBD PID 数据 | 不适用（配置：ECUC_Dcm_00626）：ECUC_Dcm_00626–00627、00629、00720、00806、00865、00870–00876、00887、00893–00895、01012、01018、01080、01107–01108 |
| ECU Reset 配置 | 不适用（配置：ECUC_Dcm_01111）：ECUC_Dcm_01039、01111–01113 |
| ResponseOnEvent 配置 | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00858、01234–01239 |
| 周期数据配置 | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_00741–00742、00862、00897、00957–00960、00962、01180–01183 |
| DcmDspDid | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00601–00602、00604、00805、01099、01122 |
| Dcm | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_01082 |
| DcmConfigSet | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00819 |
| DcmPageBufferCfg | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00775–00776 |
| DcmDsd | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00688 |
| DcmDsdService | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00689、00735、00737、01044 |
| 外置服务函数 | 不适用（配置：ECUC_Dcm_00777）：ECUC_Dcm_00777 |
| 认证角色 | 不适用（配置：ECUC_Dcm_01151）：ECUC_Dcm_01139 |
| service mode rule | 不适用（配置：ECUC_Dcm_00918）：ECUC_Dcm_00918 |
| service security filter | 不适用（配置：ECUC_Dcm_00733）：ECUC_Dcm_00733 |
| service session filter | 不适用（配置：ECUC_Dcm_00734）：ECUC_Dcm_00734 |
| DcmDsdServiceTable | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00732、00736 |
| DcmDsdSubService | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00802–00803、01047 |
| 外置 subservice 函数 | 不适用（配置：ECUC_Dcm_00777）：ECUC_Dcm_00942 |
| subservice mode rule | 不适用（配置：ECUC_Dcm_00924）：ECUC_Dcm_00924 |
| subservice role | 不适用（配置：ECUC_Dcm_01151）：ECUC_Dcm_01140 |
| subservice security filter | 不适用（配置：ECUC_Dcm_00812）：ECUC_Dcm_00812 |
| subservice session filter | 不适用（配置：ECUC_Dcm_00804）：ECUC_Dcm_00804 |
| DcmDsl | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00690 |
| DcmDslBuffer | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 的64字节容量约束；Dcm_Init 独立 uint16 容量检查8..256及通信长度可表示性，dcm_lengths.c 验证255／256边界和拒绝保留状态）：ECUC_Dcm_00738–00739 |
| DcmDslDiagResp | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00691、00693、00914 |
| DcmDslProtocol | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00694 |
| DcmDslProtocolRow | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00695、00699、00701–00702、00704、00728–00729、01043、01110、01114 |
| Paged buffer 最大响应 | 不适用（配置：ECUC_Dcm_00776）：ECUC_Dcm_01020 |
| RoE 传输类型 | 不适用（配置：ECUC_Dcm_00858）：ECUC_Dcm_00700 |
| Generic connections ECU address | 不适用（配置：ECUC_Dcm_01081）：ECUC_Dcm_01081 |
| Dem client | 不适用（配置：ECUC_Dcm_00735）：ECUC_Dcm_01083 |
| DcmDslConnection | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00705 |
| DcmDslMainConnection | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00706、00826、00952、01115 |
| 周期连接引用 | 不适用（配置：ECUC_Dcm_00957）：ECUC_Dcm_00707 |
| DcmDslProtocolRx | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00687、00709–00710、00770 |
| DcmDslProtocolTx | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00711、00772、00864 |
| DcmDsp | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00638、00712、00987、01061 |
| DDDID 来源检查 | 不适用（配置：ECUC_Dcm_00612）：ECUC_Dcm_00966 |
| 周期 DID | 不适用（配置：ECUC_Dcm_00957）：ECUC_Dcm_00956 |
| ECU rapid shutdown | 不适用（配置：ECUC_Dcm_01111）：ECUC_Dcm_00818 |
| 原子 BndM 引用 | 不适用（配置：ECUC_Dcm_01122）：ECUC_Dcm_01185 |
| 引用其他 DID | 不适用（配置：ECUC_Dcm_00606）：ECUC_Dcm_00606 |
| DcmDspDidInfo | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00607、00612 |
| 动态 DID 来源上限 | 不适用（配置：ECUC_Dcm_00612）：ECUC_Dcm_00970 |
| DcmDspDidRead | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00613、00615 |
| DID mode rule | 不适用（配置：ECUC_Dcm_00917）：ECUC_Dcm_00917 |
| DID role | 不适用（配置：ECUC_Dcm_01151）：ECUC_Dcm_01141 |
| DID security filter | 不适用（配置：ECUC_Dcm_00614）：ECUC_Dcm_00614 |
| DcmDspDidSignal | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00808、00813、01105 |
| 复合信号引用 | 不适用（配置：ECUC_Dcm_01231）：ECUC_Dcm_01232 |
| DcmDspData | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00713、00811、00869、00955、00985–00986、01106 |
| 非 C/S 与外置 FNC | 不适用（配置：ECUC_Dcm_00713）：ECUC_Dcm_00669–00677、00824–00825、00988 |
| DcmDspDataInfo | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00611、00810 |
| DcmDspSession | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00769 |
| DcmDspSessionRow | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00765–00768、00815 |
| DcmGeneral | 已核对（内置 ECUC metadata、configuration 参数／引用白名单和 diagnostic 选择约束；未列入主线的字段拒绝而非忽略）：ECUC_Dcm_00600、00820–00823、00971、01187 |
| 自定义 C 回调头文件 | 不适用（配置：ECUC_Dcm_01019）：ECUC_Dcm_01019 |
| S3 覆盖值 | 不适用（配置：ECUC_Dcm_01240）：ECUC_Dcm_01240 |
| 可选 VIN 引用 | 不适用（配置：ECUC_Dcm_00984）：ECUC_Dcm_00984 |
