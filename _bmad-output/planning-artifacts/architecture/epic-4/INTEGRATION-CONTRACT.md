# Epic 4：跨模块集成契约

状态：架构契约，2026-09-28。与 [spine](ARCHITECTURE-SPINE.md) 一起约束后续规格和 stories；它固定接口、所有权、时序和拒绝边界，不声明下述模型/生成器/OS 已实现。样例参数是产品参考配置，来自本次架构决定，不是 AUTOSAR 的唯一默认值。用户输入可以采用不同名称、ID 和合法配置值，但必须满足同一支持剖面。

## 1. 输入、计划和生成闭包

目标剖面 ID 固定为 epic4-win64-sr-cs-v1。支持单 ECU、单 Classical CAN 网络、一个单实例 ApplicationSwComponentType；只接受已经展开的 ECU_EXTRACT，不在此 Epic 实现任意 System→Extract 工具。输入角色沿用 [具体规范接口与行为](R24-11-CONTRACT.md)：Extract、SWC/类型、所选 BSW 描述、对应实现、ECUC 定义及配置值、应用 C99 实现、固定内核/目标档案。

解析结果首先形成只读的 ValidatedIntegrationPlan，所有生成器只消费该计划，不重新解析或补映射。它至少含输入文件角色/逻辑相对路径/原始字节 SHA-256、目标剖面及版本、完整 AUTOSAR 对象路径和实例、应用/实现数据类型映射、通信链、服务绑定、Runnable/Event/Task/Alarm、周期实体顺序、外部符号生产者、源码/工具/许可闭包。对象以完整路径＋实例身份关联，数值句柄由同一计划确定分配；重复短名不是同一对象。符号采用确定性命名，归一化碰撞是生成错误。

必须拒绝的影响剖面输入包括：目标 ECU 不唯一；引用不闭合或 DEST/类型错误；Extract 与 ECUC 的方向、长度或映射矛盾；数据类型映射缺失；多实例/重入/变体无法唯一决定；周期与所用 Alarm/ExpiryPoint 不一致；缺少实际 BSW 入口或服务使用端；未知必要参数；需要排队 S/R、隐式访问、跨 ECU C/S、mode 或 transformer。无关有效内容仍按父架构原样保留。不能用工具 SDG 补齐此目标的标准必要关系；旧主机 SDG 继续只在旧目标消费。

交付有三个明确阶段：

| 阶段 | 唯一输入与输出责任 | 失败行为 |
| --- | --- | --- |
| 组件契约 | SWC/数据类型→Rte_Type.h、组件契约头和 Runnable 声明 | 缺类型/访问/操作绑定则报对象路径，不生成猜测 API |
| ECU 集成 | 同一计划→RTE 实现、BSW 配置、SchM、OS 元数据、FreeRTOSConfig、任务入口和目标构建 | 不一致/缺生产者/重复调度则整体拒绝，不落半套工程 |
| 独立交接 | 完整输入、应用、BSW、固定内核/补丁/目标档案、构建与独立复验 | 已有目录不覆盖；沿用安全预览、来源摘要和新目录复验；缺少依赖或许可权利时不打包 |

新目标与既有 host-v1 分开标识；若增加交接角色/格式，显式升级并保持旧包读取规则，不混链两套 Os/Rte 符号。参考工程只交付产品原创文件和许可允许的依赖；官方 XSD/MOD/showcase/PDF 为外部资料，不默认复制进包。

## 2. SWC、S/R 和同步 DID

参考组件名 EchoApplication，一个实例。端口 RxValue/TxValue 的数据元素均为 Value，应用 uint32 映射到实现 uint32；未排队显式 S/R，初值 0。只运行一个不可重入周期 Runnable，周期 10 ms；它读取 RxValue，E_OK 时采用接收值，否则采用 0，再经 RTE 写 TxValue 并提交同一应用快照。失败写入不提交新快照，记录具体 RTE/Com 状态，不伪造发送成功。

| 边界 | 固定形状与行为 |
| --- | --- |
| S/R 读取 | Std_ReturnType Rte_Read_RxValue_Value(uint32* data)。首次接收前给出初值 0 与 RTE_E_NEVER_RECEIVED；首次有效接收后 E_OK；配置的接收 deadline 到期保留最后值并给出 RTE_E_MAX_AGE_EXCEEDED。没有私有 valid 参数 |
| S/R 写入 | Std_ReturnType Rte_Write_TxValue_Value(uint32 data)。映射所选 Com 的声明 SendSignal 服务；成功与失败由标准 RTE/Com 契约转换，不能直接包装测试器写 Com 当作应用 |
| 无效值剖面 | 不配置 invalidValue，不提供 Rte_Invalidate；handleNeverReceived 为 TRUE，handleTimeoutType=none；Com 的超时动作固定为 NONE，保留最后接收值。Rx 超时 30 ms，首次接收前不启动 Com deadline（ComFirstTimeout=0）；需要其他无效/替代机制的输入拒绝 |
| 应用状态 | 快照只有 EchoApplication 写，初始化为 0；包含值及提交 epoch。周期 Runnable 和 DID 服务器在同一 Task_Ecu 串行执行，不能各持一份应用值 |
| DID 服务 | DcmDspDataUsePort=USE_DATA_SYNCH_CLIENT_SERVER；DataServices_ApplicationValue，isService=true；同一应用实例的提供端口与 OperationInvokedEvent 绑定服务器 Runnable |
| DID 类型 | DcmDspDataType=UINT8_N，固定 32 bit/4 byte；OUT 参数使用生成的 Dcm_DataElement_ApplicationValueType（uint8[4]）；不配置动态长度、ConditionCheckRead、WriteData 或额外应用操作 |
| 服务调用 | Dcm 侧通过声明的服务使用端和生成 Rte_Call 进入服务器，在调用 Task_Ecu 内同步执行，不激活另一个服务器任务；所有使用端/提供端/操作/符号在计划闭合 |

同步 ReadData 的 Dcm 可配置 API 形状为 Std_ReturnType Xxx_ReadData(uint8* Data)，没有 OpStatus 或 NRC 输出；正常应用服务器总是填入有效数据并返回 E_OK。其 C/S 描述允许 RTE 自身基础设施失败，这不授权服务器以 E_NOT_OK 报应用值无效。快照在通信开放前初始化，因此初值及接收超时后的替代值均可读。生成 RTE 失败属于基础设施故障，应按声明服务路径处理/停止，不伪造正响应，也不添加私有返回码。

应用服务器将快照编码为四字节大端；Dcm 拼接 SID、DID 与已编码 Data，不再对该 UINT8_N 数据重复交换端序。CAN 的 uint32 使用已声明的 LSB0 小端位序。这是两条传输契约，不能因底层都是四字节而共享裸内存表示。Dcm 保持会话/长度/支持项等协议检查；首个 DID 可在默认和扩展会话读取。未配置的服务/操作明确拒绝，不扩大旧主机的标准支持声明。

依据：RTE §4.3.1.7/SWS_Rte_08104（p317）约束 handleTimeoutType=none 的保留行为；R24-11 Dcm §8.7.3.2.1，SWS_Dcm_00793（p320），C/S 变体与错误说明 §8.8.3.2（p415）；RTE 的 NEVER_RECEIVED 与 MAX_AGE_EXCEEDED 定义/配置行为，以及 SWS_Rte_07804/07805。已重新直接读取本机官方原件；网页读取超时不由旧回调代替契约。

## 3. 唯一运行与共享状态所有者

参考运行配置使用一个 Extended Task_Ecu，AUTOSTART、activation limit=1、优先级 1、抢占属性 FULL；Ev_Work、Ev_App 和 Ev_IO 三个事件。SystemCounter 每个逻辑毫秒递增，Alarm_Work 在 1 ms 开始并每 1 ms 设置 Ev_Work，Alarm_App 在 10 ms 开始并每 10 ms 设置 Ev_App。RTE TimingEvent 绑定 Alarm_App，不由模板临时另选调度表。参考少量对象不缩减完整 SC1 能力测试。

通信 ready 后，Task_Ecu 是参考目标中应用和 BSW 可变状态的唯一执行者。启动阶段仅 StartupHook 在外部输入隔离、任务尚未运行的条件下建立初值；关闭阶段先停止任务和输入，再由 ShutdownHook 收尾。主机 bridge、模拟 ISR、内核隐藏任务和监测线程均不得调用 BSW/SWC 或直接修改其状态；只提交已复制的输入/输出确认记录或设置事件。没有用软件 runnable 选择器替代 FreeRTOS 的 ready/Running 裁定。Task_Ecu 的固定业务调用顺序不是第二套 OS 调度器。

SchM 保留模块边界和 ExclusiveArea 描述。只有在计划证明相关状态完全属于 Task_Ecu、没有 ISR/其他任务访问时，单执行者 ExclusiveArea 才可消除实际锁；跨域的主机 mailbox 使用目标原子协议，OS 对象使用内核临界协议。若用户改变任务映射使该证明不成立，此剖面拒绝，而不是自动套 FreeRTOS mutex。OS 的 Resource 服务在独立配置向量验证；不因参考 ECU 没有跨任务 BSW 共享而省略能力。

启动顺序固定为：验证目标与配置→分配静态 OS 元数据/注册端口资源与真实栈档案→StartOS→StartupHook 内按依赖初始化目标驱动、CAN/CanIf、路由/CanTp、Com、Dcm→RTE/应用初值建立→控制器 STARTED 和通信开放→Ready 发布→Task_Ecu 等待事件。两个 Alarm 由 ECUC AUTOSTART 配置在 StartOS 中建立，逻辑 tick 在 Ready 之前保持关闭，不在 StartupHook 调用 SetRelAlarm/ActivateTask/SetEvent/WaitEvent。初始化入口不得假设 Task 上下文，所有传递调用须通过 OS 允许调用层级检查。单一启动状态为 Initializing/Ready/Failed；任一步失败先记录初始化诊断并发布 Failed，通信和 tick 入口保持闭锁，经 StartupHook 允许的 ShutdownOS(E_OS_STATE) 进入关闭，不返回正常调度路径。StartOS 不被包装成有私有错误返回值的接口；宿主控制面通过独立状态/故障 mailbox 接收结果。

事件位只是唤醒提示，消息数和完成状态以有界 mailbox/ticket 为准。生产者先完整发布记录，再 SetEvent(Ev_IO)；tick 与应用 Alarm 分别置 Ev_Work/Ev_App。Task_Ecu 使用 WaitEvent(Ev_Work|Ev_App|Ev_IO)，随后 GetEvent、ClearEvent 已观察位，再排空对应已发布记录；读取/清除期间到达的新记录仍在 mailbox，之后到达的 SetEvent 保持置位，不能用“一个事件位”表示只有一条消息。每轮结束重新检查所有队列、tick ticket 和事件，再进入 WaitEvent；禁止先判空后无条件清除新事件。backend 必须实现无丢失唤醒的原子发布/检查边界，独立向量验证清除前后到达的竞态。Ev_App 只对应已登记的逻辑 tick；Ev_IO 单独唤醒仅处理输入/确认，不重复周期工作。COMMIT 静止点要求 Task_Ecu 已重新等待、该批及 tick ticket 完成、输入/输出/确认队列为空且没有未确认 PDU；每个确认包含唯一发送 ticket 与 PDU handle，逐条消费，事件合并不合并确认。

一次逻辑 tick 的周期工作固定为：先完成该 epoch 的输入及 TxConfirmation 回调→Can_MainFunction_Wakeup 等已声明驱动周期工作→CanTp_MainFunction→Com 接收 deadline/状态更新→若 Ev_App 则执行应用 Runnable 与快照提交→Com 周期发送→Dcm_MainFunction→处理由输出成功触发的确认回调直至静止点。已有模块若需要额外周期入口，其 BSW 描述须显式加入同一顺序；缺失不能忽略。同 epoch 确认的二次处理只执行回调，不重复推进 tick 或重跑周期 Runnable。

到达超时边界的有效输入先被处理，随后才计算 deadline。应用快照在该 tick 的 Dcm 读取前提交，因此同 tick 的 CAN 值和 DID 值来自同一次应用提交。ReferenceCan/Com/CanTp 的签名兼容适配仍由自有 BSW 承担，禁止在 RTE 暗中散布私有驱动 API。

## 4. Windows 时间、输入与输出契约

首目标固定 Windows x64/GCC 16.1.0，时间模式 controlled_logical_ms。逻辑 epoch 为 uint64 毫秒，从 0 开始；SystemCounter 为独立标准 Counter，对 uint64 epoch 取配置模数（参考 maxAllowedValue=65535、ticksPerBase=1、minCycle=1）。FreeRTOS tick 为 32 bit；两者由每次已确认的 1 ms tick 同步推进，不能把其整数值当作永远相等，也不能靠墙钟测时。

该目标使用产品维护的 MSVC-MingW 端口派生配置，关闭墙钟 timer 自主生成 tick，进程使用 NORMAL 优先级。控制面逐个请求模拟 tick，并等内核计数、Counter 动作、Task_Ecu 周期工作及输出确认到达静止点后，再推进下一 tick。不能把多个 tick 同时 OR 入 Windows pending bitmask后假定计数无损；每个 tick 有单调 ticket/完成确认。隐藏 idle 不调用 BSW，关闭 timer daemon。

新目标使用 HostBatchV1，不复用旧目标 T/R 命令的跳时合并语义。批包含目标剖面、epoch、连续 batch ID 和有序消息，bridge 赋予全 ECU 单调 uint64 序号。参考文本入口为 BEGIN <epoch>、RX <id> <dlc> <hex>、COMMIT；编码不影响内存契约。上一个 COMMIT 完成前不接纳下一个执行批；epoch 非递减，每批最多 256 消息，单次跨度最多 1000 ms。超容量/倒退/序号溢出/协议错误在状态改变前整体拒绝，不丢最旧项、不跳历史周期。

从 epoch a 到 b：先逐个执行 a+1 至 b-1 的 tick，在 b 先按序注入该批报文，再推进 b 的 tick；若 b=a，仅注入消息及即时回调，不再次推进 Counter/周期工作。epoch 0 没有周期工作。输入、状态、tick 和输出都记录相同 epoch/序号来源，线程到达顺序只决定接纳次序，不代替已提交批的执行顺序。

输出由 Task_Ecu/驱动复制到有界 mailbox，宿主 bridge 在 FreeRTOS 任务之外执行 stdin/stdout/socket/file I/O。只有宿主输出成功才提交匹配 PDU handle 的 TxConfirmation；不能把入 mailbox 当作发送成功。确认在当前 epoch 被 OS 事件重新送回 Task_Ecu，重复/不匹配确认拒绝。COMMIT 成功表示输入、周期工作、输出和确认闭合；输出失败或规定宿主 watchdog 到期返回目标故障并关闭 ECU，不继续虚拟时间。watchdog 用宿主单调墙钟，仅控制测试进程存活，不改变汽车计时/协议状态。输出 mailbox 容量固定为 256 条；运行中溢出是目标故障，关闭 ECU，不声称能回滚已经执行的批。watchdog 固定为每个 COMMIT 5000 ms，目标档案显式记录这些参数。

## 5. OS backend 和真实栈契约

backend 是唯一内核边界。内部服务族固定为 Activate、Finish/Chain、Wait/Wake、SetEffectivePriority、GetState、AdvanceOneTick 和 Shutdown；具体文件布局不是本架构决定。每个服务的前置检查、汽车元数据、ready 队列位置/优先级与重调度请求属于一次临界转换，观察者不得看到两套状态。合法 Finish/Chain 不返回原应用帧；错误须返回标准状态，保留源实例和目标激活/事件/资源状态。完整记录每个请求及其执行顺序，不能只记录请求计数。栈/入口重新进入策略只在 backend，应用不依赖 setjmp 或 FreeRTOS。

参考容量来自 ECUC/目标档案并静态生成；独立配置验证所有一致性类、同级 FIFO、非抢占/内部资源、资源 ceiling、事件竞态、合法调用层级、Counter/Alarm/ScheduleTable、错误与 Hook、ARTI 等 SC1 义务。FreeRTOS 仍维护实际 ready 队列并选择 Running；补丁范围限于单核 ready 政策、原子状态转换、优先级恢复和所选 Windows 端口。不得重做 CPU 上下文/SMP/MPU 或添加第二个 runnable 调度器。

Windows 真实栈策略固定为“原生线程栈注册＋实际栈边界/上下文观测＋不可恢复故障关闭”。端口必须按目标配置创建线程栈，记录实际 reserve/commit/guard 与可用边界，不能用传入的 FreeRTOS buffer 代替。配置栈容量以宿主对齐后的实际值记录，不把标准 ECUC 和实现特定宿主参数混作一个字段。

公开 API 基础为 GetCurrentThreadStackLimits、GetThreadContext/VirtualQuery 及 SetThreadStackGuarantee；端口在注册/切换和 OS 边界对照实际 SP/区域，异常路径接收真实 Windows 栈溢出记录。预分配故障记录和独立控制栈，先禁止继续调度，再以 E_OS_STACKFAULT 进入 ShutdownOS/配置的 ShutdownHook；故障路径不得等待故障线程持有的普通锁，不在损坏栈调用 BSW、分配内存或恢复应用。具体编译器异常捕获 shim 在该接口内实现并必须实测。参考 SC1 不配置 ProtectionHook；若后续配置该 Hook 则遵守标准行为另验。

这是选定的端口设计与失败边界，不是已完成监测的声明。首个 backend 工作包必须在独立进程用真实栈消耗/破坏验证捕获、故障状态与无继续运行；仅改 flag 或 fake buffer 的测试无效。若目标 API/编译器组合无法实现该契约，按已确认停止条件暂停扩展集成并重新评估端口/路线，而不是悄悄删除栈义务。已核查 [Microsoft 栈边界 API](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadstacklimits)和 [异常栈保证 API](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadstackguarantee)；API 存在不证明完整方案已通过。

## 6. 产品原创参考参数与独立预期

参考参数固定如下，后续生成 fixture 不再由各 story 自选。其他合法用户参数经同一计划生成，不强迫复制这些 ID。

| 项目 | 参考值 |
| --- | --- |
| ECU/应用 | ReferenceEcu / EchoApplication，一个实例 |
| CAN 信号 | Rx 0x320，Tx 0x321，DLC=4，uint32/32 bit，bit position=0，LSB0 小端 |
| 周期/超时 | 应用与 Tx 10 ms；Rx 30 ms，ComFirstTimeout=0；初值/替代值 0 |
| DoCAN | 物理 normal addressing，11 bit；请求 0x700，响应 0x708，DLC=8；参考 ID 不与信号帧重叠 |
| 应用 DID | 0x1234，固定四字节大端快照；默认/扩展会话读；没有额外应用 C/S 操作 |
| OS | Task_Ecu Extended/FULL/priority=1/maxActivation=1；SystemCounter 1 ms；Alarm_Work 1/1 ms、Alarm_App 10/10 ms，事件 Ev_Work/Ev_App/Ev_IO |

独立测试器提供 RX 0x320/4 的 78 56 34 12，于 epoch 10 的周期提交后，应观察 Tx 0x321/4 的 78 56 34 12；0x22 0x1234 应得到 UDS 62 12 34 12 34 56 78（SF 为 07 62 12 34 12 34 56 78）。若接收时间为 0，则 epoch 30 的应用采用替代值 0；若新帧恰在 30 的批中到达则先接收后判超时，应用采用新值。初始 DID 返回四个 00。这些是已固定的未来验收预期，本轮未执行新目标。

## 7. 开发前置、实现责任与验收分层

架构完成不要求新功能已经存在。下面按工作包依赖固定证据归属，不在本轮创建 ready-for-dev stories 或执行功能开发。

| 工作包 | 输入/进入条件 | 必须交付的具体结果 | 阻断后续的范围 |
| --- | --- | --- | --- |
| W0 输入与证据基线 | 最终架构、已固定规范/内核来源 | 原创正向 ARXML 和引用/类型/变体反例；XSD/语义预期；所用标准条款清单、许可/源码/补丁清单 | 输入故事可在此开展；未通过不放行其依赖的解析/生成集成 |
| W1 OS/backend 基础 | 主路线及本契约；固定内核/编译器/许可 | 完整请求 FIFO、Finish/Chain 错误原子性、Wait/Event/Resource 与受控 tick/真实栈的可执行配置、实现、独立结果；明确余下 SC1 服务工作 | 核心语义、受控时间和真实栈关口未通过，不扩大到应用/BSW 集成；W1 不以自身尚未实现为进入阻断 |
| W2 模型及契约生成 | W0 支持剖面和正反输入 | 类型/接口/BSW 描述对应实际实现、ValidatedIntegrationPlan、契约头、闭包和拒绝诊断 | 可与 W1 独立推进；缺闭包不放行 ECU 集成生成 |
| W3 ECU/目标集成生成 | W1 关键门和 W2 闭包通过 | RTE/SchM/OS/BSW 配置、受控 Windows 入口、固定单执行者顺序、离线重建工程 | 未通过不把工程宣称为可交付参考 ECU |
| W4 应用/通信/诊断闭环 | W3 可构建工程 | 同一实例的 S/R/快照/ReadData、独立 CAN 和 UDS 字节向量、超时/错误恢复、旧目标回归 | 未通过不放行组合行为验收 |
| W5 等级与交接出口 | 已实现服务及集成结果 | 全部适用 SC1 能力/容量/错误/Hook/ARTI 条款证据、来源权利、独立新目录构建和复验 | 未通过不得声称完整 SC1、不得将 Epic 4 标 done；MCU 仍另验 |

每个可编码 story 需要具体入口/产物、依赖与可执行验收，后续规格/故事拆分以这些工作包为基础。完整 SC1 验收属于实现出口，不放在架构定稿或 W1 开始之前；关键未知端口机制则放在 W1 先证实，失败止损。研究观测不能替代上述目标证据。
