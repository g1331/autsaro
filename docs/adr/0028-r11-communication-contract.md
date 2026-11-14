# 0028. R11 通信身份、FD 剖面与主机记录契约

## 状态与范围

已接受（2026-11-14，经独立审查）。接受依据及独立审查的发现、处置由本 ADR 的 PR 记录。

本决定落实 [#483](https://github.com/g1331/autsaro/issues/483)，输入为
[#342](https://github.com/g1331/autsaro/issues/342#issuecomment-6306779222) 和
[#343](https://github.com/g1331/autsaro/issues/343#issuecomment-6306755113) 的研究。
这是 R11 的目标契约；当前实现仍为单通道、标准 Classical CAN。
功能实施按 [#341 实施拆分](https://github.com/g1331/autsaro/issues/341) 等待 Epic 9
正式出口与相应依赖。路线顺序、#117 的混合 ID 原始出口及 #212 的共享 RTE 归属保持。

## 依据及复核方式

基线为 CP R24-11。以下原件 SHA-256 与本地官方 CP 清单对应项一致；正文从
SpecificationsARXML 的同名文档条目读取，页码指 PDF 印刷页。原件与提取全文留在仓库外。

| 原件 | SHA-256 |
| --- | --- |
| [CANDriver][CANDriver] | `d04d35b80d8f445b66f960fd9b5be4f02f324e73cc4be239887e8c1235b172aa` |
| [CANInterface][CANInterface] | `84bf624c9e497d765653c8df4d62a6e3835efb64f76c742d727b177a64c10f53` |
| [CANTransportLayer][CANTransportLayer] | `7334d633b02c443aacbe0ca25e20c319e977104eeb4bf8e67ac0358cfea34a22` |
| [SpecificationsARXML][SpecificationsARXML] | `d598e866ee481c6f16605ce6241a30674201c57ebaf231b2746d5ff6d21dd04e` |

#342 引用的编号（含续写编号与范围）已逐条重读；下列各节概括原文依据，作为设计输入。
图示已渲染查看：Can 图 7.2／7.3／7.4（页 32／40／42）；CanIf 图
7.1／7.2／7.3／7.6／7.8／7.9（页 32／38／39／52／63／68）；CanTp 图
7.2／7.8／7.9（页 28／41／45）。条款实施处置与行为证据由后续模块 PR 提供。

## 1. 总线匹配键与句柄域

**决定。** Rx 路由保存完整物理通道路径、driver／controller 引用、HRH、ID format
（STANDARD／EXTENDED）、numeric ID 及帧类型接受集合（Classical／FD）。
controller 从通道引用解析，运行时以 Mailbox 的 controller、HRH、编码 ID 查询；
一个实际接收帧只能命中一个接收路由。同通道同 HRH 同 format 同 ID 的策略集合有交集时，
在冲突源对象拒绝；Classical-only 与 FD-only 可分别映射。同值 ID 可跨通道或跨 format 复用。
同一物理通道上的接收歧义还须跨 HRH 检查，避免一个帧被重复交付；
这项唯一性为项目剖面约束。

Tx 从 CanIf Tx PDU handle 唯一反查 CanIfBufferCfg→HTH→controller→物理通道。
每个 PDU handle 带模块、方向／API 域；System／ECUC Pdu、ComIPdu、PduR／LSduR
handle、CanIf handle、CAN ID 和 CanTp connection 分别保留身份。
确认归还原请求的 swPduHandle，并由既有生命周期／请求代次约束迟到确认。

**原文依据。** `SWS_CAN_00496`（页 54）定义 Can_HwType 的 CanId、Hoh、ControllerId；
`SWS_CANIF_00877`（页 41）分别比较去除最高两位的数值 ID 与最高两位的类型；
`SWS_CANIF_00467／00468／00469／00211`（页 41）要求配置 HOH、硬件过滤引用及按传入
HRH 执行所选 BasicCAN 软件过滤。`SWS_CANIF_00466`（页 33）规定 Tx PDU 静态绑定 buffer；
图 7.1 展示 PDU／HOH／controller 引用，箭头表达引用关系。
`SWS_Can_00276`（页 40）要求保存并在确认时返回 swPduHandle。

`SWS_CANIF_00281`（页 42）规定同物理通道混合标准／扩展 ID；紧随正文的说明将该能力
列为 optional feature，并说明没有配置开关。本项目通过 #117 选择实现该能力，混合收发、
EXTENDED 配置接纳及软件过滤三项出口全部保留。29 位 ID 与 TP 的 extended／mixed
addressing 分别配置；CanTp 图 7.2／7.8 展示 PCI、N_AI 与 NSduId／LSduId 的关系。

**否决。** 全局裸 ID 判重会误拒跨通道与低数值扩展 ID；按低 11 位过滤会串线；
以 PDU handle 充当 CAN ID 或通过序号猜 controller，会把确认和数据交给错误路由。

**约束。** #117 使用这一编码与混合过滤约定；#485 保留 frame type；#486 同步迁移
signals、diagnostic trigger 查找与 plan 判重；#487 按实际连接接收 FC／CF／确认；
#488 保留 signal→PDU→Frame 的两级映射；#489 以 typed route 转发；#491 编辑入口保留同一身份。

## 2. 首批主机拓扑与模式归属

**决定。** 单 driver，每条物理通道连接一个 controller；N＝2、3、4 分别验收，
保留单通道回归。首批每个网络对应一条物理通道。多 driver、一网络多 controller 的配置
在生成前按源引用明确拒绝。计划仍保留 network、physical channel、connector、CanIf
abstract controller 与 driver controller 的独立路径，以便以后按新决定扩展拓扑。

driver 内 controller 从 0 连续；HRH／HTH 共用从 0 连续的 driver 级对象编号。
CanIf controller 自 0 起，经 CanIfCtrlCanCtrlRef 显式引用 driver controller。
验收采用错位映射，以下每对 HRH／HTH 仅为最小身份向量；参考 ECU 的 7～17 HOH、
12 Rx／5 Tx PDU 规模由 #492 累计覆盖。

| N | 按 B0…B(N−1) 的 CanIf ID→driver ID | 对应 Rx HRH／Tx HTH |
| --- | --- | --- |
| 2 | 0→1，1→0 | (2,3)，(0,1) |
| 3 | 0→2，1→1，2→0 | (4,5)，(2,3)，(0,1) |
| 4 | 0→3，1→2，2→1，3→0 | (6,7)，(4,5)，(2,3)，(0,1) |

ComM NetworkHandle、用户映射、BswM 动作、BusSM 与 DcmDslProtocolComMChannelRef
均由显式引用连接。请求、保持计时、controller／PDU mode、bus-off、恢复及通知按通道归属；
ComM 同用户多通道的最低模式聚合由 #464 实现。继续使用主机 CDD BusSM，完整 CanSM／NM
生命周期按路线图由 R8 承接。Can／CanIf／CanTp 各自拥有状态，ECU owner 保留唯一批次结算。

**原文依据。** `ECUC_Can_00316／00326`（页 105／123）分别规定 driver 内 controller
连续编号及 HRH／HTH 共同编号；`SWS_CANIF_00653` 与 `ECUC_CanIf_00647／00636`
（页 37、224／225）定义抽象 controller 域及下层引用。CanIf 图 7.2／7.3 同时展示网络与
物理通道的 1:1 和 1:n，支持本项目明确选择首批 1:1 样本。Can 图 7.2 与 CanIf 图 7.8
区分 controller 状态及 PDU channel mode；单 controller 变化和整个模块初始化分别处理。

`SWS_CanTp_00121／00122／00190／00285–00289`（页 41–42）要求 N-SDU 静态连接、
连接资源独立及正确路由；内部连接数由配置工具分析路由确定。
`SWS_CanTp_00248`（页 43）要求共享 Tx N-PDU 通过确认串行化，Rx 共享受寻址或 metadata
条件约束。首批不同物理通道使用明确独立的 N-PDU／连接映射；真实共享时执行上述条件。

**否决。** 将 N 个 controller 自动视为 N 个网络会抹掉合法标准拓扑；首批同时实现多 driver
与网络聚合增加未要求的协调成本。全局模式、通道号充当 TP connection、每 controller
重置全 ECU PDU handle 都会破坏状态或确认归属。

**约束。** #486 覆盖全部配置、生成、Can／CanIf、BusSM、ComM／BswM／Dcm 接缝，
#464 负责用户聚合；#487 负责连接资源；#489 验证 2／4 通道完整输出；#492 汇总每个 N 的证据。

## 3. FD 格式、长度与填充剖面

**决定。** controller 显式配置 nominal bitrate、可选 FD data bitrate 与 BRS；
存在有效 CanControllerFdBaudrateConfig 才启用 FD。BRS 的 true／false 都保留并由配置消费，
主机记录模拟数据通路，目标位时序和采样点约束在相应目标描述下校验。
CAN XL、TTCAN 及其参数在生成前明确拒绝。

**原文依据。** `ECUC_Can_00005／00473／00481／00475`（页 110、113、116）分别定义
nominal kbps、FD 配置容器、data kbps 与发送 BRS（false 时发送 FD 而不切换速率）。
`SWS_Can_00416`（页 53）规定 uint32 Can_IdType 最高两位为：00 标准 Classical、
01 标准 FD、10 扩展 Classical、11 扩展 FD。
数值范围分别为 0..0x7ff 与 0..0x1fffffff；例如 numeric ID 0x123 的编码依次是
0x00000123、0x40000123、0x80000123、0xc0000123。

Rx 与 Tx 分别按 `ECUC_CanIf_00596／00590`（页 203／195）使用以下接受／发送策略：

| ID format | Rx 六值中的三种后缀 | 接受帧类型 | Tx 四值中的两种后缀 | 发送帧类型 |
| --- | --- | --- | --- | --- |
| STANDARD／EXTENDED | `_CAN` | Classical 和 FD | `_CAN` | Classical |
| STANDARD／EXTENDED | `_NO_FD_CAN` | Classical | — | — |
| STANDARD／EXTENDED | `_FD_CAN` | FD | `_FD_CAN` | FD |

FD 位由配置／帧身份决定，0..8 字节的 FD 保留格式。length 使用字节数；raw DLC
只在总线编码边界转换：0..8→0..8，9..15→12、16、20、24、32、48、64 字节。
CanIf 的逻辑 PDU 长度、driver 请求长度、线上的 CAN_DL、TP 的 RX_DL／TX_DL 分别表达。
`ECUC_CanIf_00599`（页 204）定义接收长度检查及大于 8／64 的配置依赖；本 FD 剖面最大 64。
Rx 线帧只接纳合法 CAN_DL，匹配后执行所配长度检查（CanIf 图 7.6）；
未知但合法 ID 进入过滤，格式／长度非法的主机输入由 admission 拒绝。

`SWS_Can_00218`（页 78）要求 Can_Write 对 >64、非 FD controller 上 >8、FD controller
上 >8 且未置 FD 位返回 E_NOT_OK，并在启用 DET 时报告 CAN_E_PARAM_DATA_LENGTH。
`SWS_Can_00486`（页 79）只在有效 FD 配置下评价 FD 位；非 FD controller 的 ≤8 字节
直接 Can_Write 保留忽略 FD 位的规则。生成器提前拒绝 FD Tx PDU 绑定到非 FD controller；
标准 API 的上述直接调用语义照原文实现。

`SWS_Can_00502` 与 `ECUC_Can_00485`（页 79／121）要求驱动将非离散请求长度向上
补到合法长度，以所选 HOH 的 CanFdPaddingValue 填充；首批显式使用配置值或标准默认 0。
CanTp 在调用 LSduR 前自己补齐，使用 CanTpPaddingByte（`SWS_CanTp_00351`，页 45），
其 TX_DL 来自 EcuC PduLength（`00354`，页 60），首次 CAN_DL≤8 时 RX_DL=8，
否则等于首次 CAN_DL（`00350`，页 28）。CanTp 图 7.2／7.9 决定 PCI 与容量关系。
`SWS_CanTp_00344–00349`（页 44–45）的小帧 padding、短 SF 拒绝、短末 CF／FC
终止与错误报告仍随所选 padding 配置执行。

**否决。** 从 length 推导 FD 位会丢失小 FD 帧；Rx／Tx 共用枚举会改变 `_CAN` 语义；
把字节长度当 raw DLC 会错误发送；让驱动代填 TP 的 padding 会使用错误的 padding 值；
只扩大 frame 数组会继续用错 SF／FF／CF 容量。

**约束。** #117 可先交付 Classical 混合 ID；#485 负责 Can／CanIf FD、配置、驱动填充及工具；
#487 独立负责 TP FD／PCI／padding，沿用已有诊断服务；#488 依据逻辑 PDU 位边界布局；
#486／#489 保持通道及格式。长度向量为 8／9／12／13／32／33／48／49／64／65，
9／13／33／49 分别补到 12／16／48／64，65 拒绝；padding 逐字节验收，拒绝发送无确认。

## 4. 首批软件过滤

**决定。** BasicCAN HRH 使用精确 ID 列表，逐条保留 format 和帧类型接受策略。
显式配置的软件 range／mask 过滤在生成前定位拒绝；硬件 acceptance filter 的引用照常保留。
未知合法帧过滤后无上层通知；相同匹配域的策略交集在配置阶段报告双方对象。

**原文依据。** `SWS_CANIF_00030`、`00645／00646`（页 67–68）分别规定列表命中后的
Rx PDU 派生、上下界或 base/mask 范围定义、标准／扩展范围区分；图 7.9 按 HRH 分列表。
`SWS_CANIF_00852`（页 68）规定单 ID 优先于范围、小范围优先于大范围。
这些范围义务在将来选入 range／mask 支持时一并实施。

**否决。** 首批同时实现 range／mask 会引入重叠优先级和额外歧义规则；把未消费的范围配置
当成精确 ID 接收会静默改变配置。精确列表足以支撑 #117 的混合收发和软件过滤出口。
[参考 ECU](../roadmap/reference-ecu.md) 的 HRH 范围能力仍为 MCU 前目标，后续扩展沿原要求推进。

**约束。** #117／#485／#486 的过滤正反例按精确列表执行；#489 在原样 PDU 转发前完成
接收身份消歧；#491 显示真实接纳范围；#492 同时记录首批剖面与后续范围能力的交付证据。

## 5. HostBatch、验证输入与离线包身份

**决定。** 首个改变通信身份取值域的实施 PR 原子切换全部 producer／consumer：

| 当前身份 | R11 身份 | 变化原因 |
| --- | --- | --- |
| `HostBatchV1` | `HostBatchV2` | Rx／Tx／控制记录携带 channel 与 frame identity，支持 64 字节 |
| `autosar-ecu-test-inputs-v1` | `autosar-ecu-test-inputs-v2` | typed endpoint 集合及独立预期，替代四个裸 ID |
| `autosar-ecu-integration-v2`；描述 `formatVersion=2` | v3；3 | controller／HOH 集合、格式与通道引用 |
| `autosar-workbench-handoff-v4` | v5 | 新通信交付契约及新包内工具 |
| `autosar-build-target-v3` | v4 | 新 nativeDelivery 与主机协议声明 |

#117 先落地时也使用 V2 的完整身份字段，初期可限定 channel 和 Classical；#485／#486
扩展被接纳的配置组合，复用同一记录语法。包内 ecu-tool 随包封存，版本由
build-tool-inventory 的成员字节摘要、上述 handoff／target／inputs 身份和 HostBatch 握手共同绑定。
现有 inventory v2、asset-inventory v2、ownership v3、component-contract v2 维持结构版本，
更新真实成员摘要；后续若改变它们的结构，再在实施 PR 单独升版。操作系统 target ID、
工具链和固定 FreeRTOS 身份保持原值。

**行协议。** 启动发布 `READY HostBatchV2`；客户端先发送 `PROTOCOL HostBatchV2`，
收到 `PROTOCOL_OK HostBatchV2` 后才能 BEGIN。BEGIN／COMMIT 的 epoch、sequence、
ticket、批次原子拒绝、write／flush 后确认和静止点收据沿用现有语义。

- 输入：`RX <channel> <format> <fd> <id> <length> <data>`。
  channel 是计划分配且在验证输入中映射到完整物理通道路径的整数；format 为
  `STANDARD` 或 `EXTENDED`，fd 为 0／1，id 为十进制裸数值，length 为线帧字节数。
  data 为恰好 length×2 个小写十六进制字符，零长度用 `-`。Rx 长度限 Classical 0..8
  或 FD 的离散集合；逻辑非离散长度的发送填充通过真实 Can_Write 路径验收。
- 输出：保留现有 `OUT epoch=… sequence=… ticket=… pdu=…`，随后固定为
  `channel=… format=… fd=… id=… length=… data=…`，data 同输入规则。
  length 记录驱动填充后的线帧长度，逐字节观察实际 padding；pdu 为 CanIf Tx PDU handle。
- controller 模式与故障控制记录显式携带 channel，经同一映射定位；批次 epoch 继续全局唯一。
  `verification-inputs-v2` 同时保存 endpoint 的 module／direction／handle、channel 路径映射、
  format、fd policy、ID、逻辑长度、预期 wire length 和 payload；oracle 比较完整输出集合及确认。

**拒绝。** 新 parser 拒绝无握手的 V1 行、错误版本及不完整字段，当前批次按既有原子政策
整体拒绝；新 verifier 在读取结果前核对 READY，拒绝 V1 OUT。旧包按实际 handoff、target、
integration、inputs 身份及完整 seal 检查拒绝，提示从原 ARXML 重新生成。
只改版本字符串仍须因结构、引用、可信工具／资产字节或 seal 不匹配被拒绝。
拒绝不改写原包，不补 channel=0、STANDARD 或 fd=0，不在新工具中重放旧格式。

**原文依据。** `SWS_CAN_00496`、`SWS_Can_00415／00416` 及 `SWS_CANIF_00877`
要求穿过主机接缝的 controller／HOH／ID／格式与长度信息保持完整；
`SWS_Can_00276` 约束确认归属。上述 ASCII 格式及版本号为项目设计，按
[ADR 0009](0009-standard-private-and-host-interface-separation.md) 的主机适配边界和
[ADR 0011](0011-no-backward-compatibility-before-stable.md) 的旧格式拒绝政策执行。

**否决。** 只在 checkout 更新 verifier 会遗漏离线接收者；继续沿用 V1 或猜默认字段会把
旧记录误认为新证据；只比最终 echo 会漏掉中间串线与错误确认；用单个四 ID 描述任意工程
会丢失多信号／通道身份。新 oracle 仍须有独立场景预期。

**约束。** #117／#485／#486 首次改变身份时共同遵守升版契约；#487／#488／#489
补充各自真实场景与 oracle；#491 编辑及运行消费者读取同一身份；#492 用搬离 checkout
的包内工具在 Windows／Linux 分别复核，旧记录及旧包拒绝列入验收。

## 6. ABI 与交付影响

**决定及依据。** `SWS_Can_00415／00416`（页 53）已提供 uint8 length 和 uint32 ID，
`SWS_CAN_00496` 已带 controller／HOH；标准公开 API 签名与这些类型保留。
`SWS_CANIF_00466／00653` 和 TP 连接条款所需变化落在实现自有配置与主机接缝。
实施 PR 按下表逐项审阅调用方、布局、编译链接和交付字节，并更新 CHANGELOG。

| 影响面 | 必须迁移的内容 | 承接 |
| --- | --- | --- |
| 计划与描述 | 总线键、controller／HOH 集合、信号／诊断／路由引用、版本 | #117、#485、#486、#488、#489 |
| Can／CanIf 私有配置 | controller 状态及 HOH 表、Rx／Tx route、buffer／HTH 绑定、format policy | #117、#485、#486 |
| TP／模式配置 | connection 表、TX_DL／RX_DL、计时与占用、ComM／BswM／Dcm／BusSM 绑定 | #487、#486、#464 |
| host 及执行端口 | Can_Inject／输出适配参数、Ecu_BatchFrame、邮箱、输出／ticket 关联、容量与 codec | #117、#485、#486 |
| 生成 C 及构建 | 配置声明／对象、初始化与调度调用、header consumer、target source inventory | 各实施包 |
| 交付与工具 | HostBatchV2、验证输入、handoff／integration／target、ecu-tool、资产清单及 seal | #117、#485、#486、#492 |
| 编辑投影 | typed edit、linked group、输入范围、保存重开与失效身份 | #491；状态重构沿 #97 |

增加本项目结构字段可能改变 C 布局、参数及容量；独立 C consumer、实际生成工程和
离线接收者分别验证。第三方 FreeRTOS／port 的源码与身份没有随此设计改变；若实施触及
它们，单独核对采用与交付责任。宏、类型、配置表维持各自唯一 producer。

**否决。** 为新增通道改写标准 API 会破坏标准 consumer；仅重算资产摘要会遗漏 ABI 和
协议变化；只改运行时表而保留旧生成声明、工具或 oracle 会产生无法组合的交付。

**约束。** 上表所列各包以本 ADR 为共同输入，#492 汇总公开接口、两平台、离线包及
错误／恢复证据。每项实现仍按模块标准篇记录实际处置；本 ADR 交付设计决定。

[CANDriver]: https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_CANDriver.pdf

[CANInterface]: https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_CANInterface.pdf

[CANTransportLayer]: https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_CANTransportLayer.pdf

[SpecificationsARXML]: https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_MOD_SpecificationsARXML.zip
