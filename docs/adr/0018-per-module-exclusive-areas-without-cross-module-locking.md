# 0018. 独占区按模块划分，持区期间不调用其他模块

## 背景

主线 BSW 的 SchM 独占区实际是一把跨模块共享的递归锁（[#121](https://github.com/g1331/autsaro/issues/121) 调查，基线 `9378bfe`）：

- 声明了 `CanIf_CANIF_STATE`、`ComM_COMM_STATE`、`BswM_BSWM_STATE`、`Dcm_DCM_STATE`、`CanTp_CANTP_STATE` 五个区，主机适配把后四个全部映射到 CanIf 的区，即同一把递归 mutex（`Can_HostStateLock`）。Can 驱动、Com、PduR、LSduR、Det、主机 BusSM 和生成的 `SchM.c` 没有自己的区，直接进入 CanIf 的区；Com 的信号读写、Rx 与主函数不进入任何区。
- 持区期间调用其他模块：CanIf 调 `Can_Write`／`Can_SetControllerMode`／`Can_Get*`、LSduR 确认与 `Det_ReportRuntimeError`，并经 PduR 一路进入 Com 通知、CanTp、Dcm；CanTp 调 `PduR_CanTp*` 与 `LSduR_CanTpTransmit`；Dcm 调 `PduR_DcmTransmit`、`ComM_DCM_*`、`SchM_Switch_Dcm_*`；ComM 调 BusSM 与 `BswM_ComM_CurrentMode`；BswM 调 `action`；Det 在区内调用用户 callout。一条 Rx 或 TxConfirmation 链从 Can 驱动一直持锁到 Com 通知返回。
- 代码依赖递归：`Com_Init` 在区内调用 `Com_DeInit`，BusSM、ComM、BswM、CanIf 的互相调用都嵌套进入同一个区。
- 取消、恰一次确认和迟到回调的正确性依赖“所有模块共享同一把递归锁”，没有写在任何模块描述中。BSWMD 只给 CanIf、CanTp、ComM、Dcm 声明区，且把 `canEnter` 挂在模块的每一个入口上；BswM 使用 `BSWM_STATE` 却未声明，其余模块使用 CanIf 的区也未声明。

标准依据：

- `SWS_BSW_00134`：模块的独占区只保护模块内部数据；`SWS_BSW_00038`：独占区在 BSW 模块描述中定义并记录。TPS BSWMD 5.5／`InternalBehavior.exclusiveArea` 说明独占区“属于该模块（component resp. module）”。
- `SWS_Rte_07250`／`07251`：`SchM_Enter_<bsnp>_<name>` 为模块自己声明且有 `canEnter` 的区生成；备注“BSW Scheduler 不要求支持同一区的嵌套进入”，不同区只可按相反顺序退出（`SWS_Rte_07252`、`CONSTR_09047`）；`SWS_Rte_07579`：任务、cat1 与 cat2 中断上下文都须可用，实现机制由 `RteExclusiveAreaImplMechanism` 选定。
- `SWS_CANIF_00033`：CanIf 用 `SchM_Enter_CanIf_<area>` 保护 Tx L-PDU buffer；`SWS_CanTp_00236`、CanIf 各回调的上下文注记、CanDrv（回调按 ISR 上下文实现）与 Com 的 Rx／TxConfirmation 注记都允许回调来自中断；`SWS_BSW_00167`：中断上下文及其回调的运行时间应尽量短。
- 标准没有逐字禁止持区调用其他模块；TPS 的 `BswDirectCallPoint.calledFromWithinExclusiveAreaNestingOrder` 甚至允许描述这类调用点。下面第 2 条是依据 `00134`、`00167` 与 MCU 上下文约束采取的更严格设计规则，不是标准明文。

到 MCU（R10）时 SchM 通常以关中断或 OS Resource 实现：各模块映射到不同的锁，现有保证会失效或出现锁序死锁；映射到同一把锁，关中断时间覆盖整条调用链且长度不可预测。越晚改动，波及的模块与测试越多。

## 决策

1. **每个模块只进入自己的区，区只保护本模块的私有数据。** 有可变状态的模块各有一个区，名为 `<MODULE>_STATE`，符号 `SchM_Enter_<Module>_<MODULE>_STATE`：Can、CanIf、LSduR、PduR、Com、CanTp、Dcm、ComM、BswM、Det、主机 BusSM，以及生成 SchM 持有的模式数据。只保存一个常量配置指针的模块（LSduR、PduR）也用自己的区发布和快照该指针，不假定指针读写原子。
2. **持区期间不调用其他模块。** 区内只允许访问本模块变量、调用本模块 `static` 函数和退出区。禁止的有：任何其他模块的函数（含 `Det_Report*`）、经配置函数指针的回调／通知／callout、`SchM_Switch_*`、OS 与平台 API。固定写法为“区内快照与决定（更新自身状态，需要时登记待办）→ 退出 → 调用 → 需要时再进入结算”。
3. **区不嵌套、不重入。** 模块不同时持有两个区，同一区不嵌套。公开函数进入区，内部辅助函数以“已持有”为前提；区内不调用本模块的其他公开函数（`Com_Init` 改调内部复位）。因此没有锁序问题，SchM 映射也不要求可重入。
4. **原先隐含在共享锁里的保证改由模块自有协议提供：**

   | 保证 | 新机制 |
   | --- | --- |
   | 状态不撕裂：Init／DeInit／模式变化与在途回调不会读到半改状态 | 配置是 const，回调只持快照；状态只在所属模块的区内改；入口先在自己的区内核对初始化状态 |
   | 请求结果恰一次，被拒请求不产生结果 | 占用后调用、调用后结算：CanIf Tx 台账（预留→`Can_Write`→结算，取消只认领已接受的请求）与取消代数；CanTp `lower_pending`／`pending_lower_id`；Dcm `DCM_TRANSMITTING`；ComM `bus_result`。迟到的确认在台账中找不到占用即丢弃，不会算到新请求上 |
   | 有序发布（BswM 动作、Dcm 会话模式、ComM 当前模式、BusSM 发布） | 单飞冲刷：区内记录期望值，同一时刻只有一个上下文调用并循环到稳定，其余上下文只记录。`SWS_BswM_00061` 的“推迟”因此对跨线程也成立 |
   | 回调内重入（回调中再调用下层、取消或重新初始化） | 区在调用前已释放，不依赖递归 |
   | 迟到的事件（Rx、模式、bus-off） | 事件可能在取消返回之后到达；按电平触发处理：处理函数从当前真实状态重新推导（`Ecu_HostBusSM` 已如此），入口先核对自身初始化状态，迟到事件是无害的幂等操作 |

   取消与在途回调之间不再有串行化：不承诺“取消返回后不再送达旧世代事件”，只承诺上表中的结果与状态保证。拒绝的方案见下。
5. **集成声明。** BSWMD 中每个模块声明且只声明实际使用的 `ExclusiveArea`，`canEnter` 只挂在实际进入该区的入口上，并生成对应的 `SchM_<Mip>.h`（含 Com、Can、PduR、LSduR、Det、BswM）。每个区声明三项约束：从不嵌套、持区时间有界且不含外部调用、可从任务与回调（可能为中断）上下文进入，因此只能映射到对这些上下文有效的机制（`ALL_INTERRUPT_BLOCKING`、`OS_INTERRUPT_BLOCKING` 等，不能是仅限任务的 OS Resource）；区到机制的映射由集成配置给出，模块代码不隐含。主机目标把每个区映射到各自独立的非递归锁，适配层检测嵌套（线程持有任一区时再次进入）并报告失败，使第 2、3 条由所有原生测试执行，不靠审阅。
6. **例外。** 目前没有。确需持区调用其他模块时，在本 ADR 追加例外条目（依据、时间上界、锁序），并在 BSWMD 以 `calledFromWithinExclusiveAreaNestingOrder` 声明调用点。
7. **中断上下文的时长（`SWS_BSW_00167`）。** 本决定保证每个区内的执行时间有界，与调用链长度无关；回调链自身的总耗时取决于 Can 驱动选用轮询还是中断，由 R10 集成评估。主机主线的 CAN Rx、Tx 与 bus-off 为轮询。
8. **迁移。** 分阶段进行，迁移期间未迁移模块仍映射到旧共享锁。已迁移模块的区是叶子锁（持有时不取其他锁，区内不调用他人），因此与仍持共享锁的模块并存不会死锁。全部迁移后删除旧映射并强制开启嵌套检测。不保留旧映射的兼容（[ADR 0011](0011-no-backward-compatibility-before-stable.md)）。

拒绝的方案：

- **保留单把共享锁并在集成说明中声明。** 违反 `00134`，MCU 上关中断时间覆盖整条调用链。
- **每模块一把锁但保留持锁调用（锁层级）。** 仍须声明嵌套顺序并要求可重入映射，关中断时间仍覆盖调用链。
- **取消时等待在途回调排空。** 回调内部可合法地重入取消与重新初始化（现有 `can_callback_lifetime` 覆盖），等待会自死锁。

## 后果

- 细化 [ADR 0009](0009-standard-private-and-host-interface-separation.md) 第 6 点：保护依旧不能以“单 owner”假设删除，但保护的形态改为按模块的区加上述协议；“拆分驱动与上层状态锁时核对双向调用及锁序”的前提（上层共享一把状态锁）随迁移消失。迁移的第一个实施 PR 同步改写 `AGENTS.md` 中对应条目，并改写 `can_callback_lifetime` 的预期：由“竞争者被共享锁挡住”改为在 `SchM_Exit_*` 之后、进入消费者之前的窗口暂停，验证第 4 条的结果保证。
- `SchM_*` 符号是集成接口：新增 Com、Can、PduR、LSduR、Det 的 Enter／Exit，各模块不再使用 CanIf 的区，交付头文件、BSWMD 与资产摘要随之变化，按 ABI 审阅并写入 CHANGELOG，不用摘要更新吸收契约变化。
- 区外的窗口（如 CanIf 预留之后、`Can_Write` 返回之前）由第 4 条的协议兜住，这是主要实现风险；用真实线程握手测试覆盖，暂停点取各模块的 `SchM_Exit_*`，并保留重入取消的用例。
- 同一模块内“区内读下层、再改自身状态”的判断（如 `CanIf_SetPduMode` 读取驱动实际模式）在迁移时改为依据模块自身状态或占用结算，不能简单把下层调用移出区。
- 区划分与持锁调用不再构成 `SWS_BSW_00134`／`00038` 的差距后，处置写在对应测试的 `Verifies:` 或 `standards.md` 的差距记录中，与 [#116](https://github.com/g1331/autsaro/issues/116) 协调；本 ADR 本身不构成处置。

## 状态

已接受（2026-10-14），实施中（[#121](https://github.com/g1331/autsaro/issues/121)：基础设施、Can 驱动与 CanIf 接缝、Det 的区；LSduR／PduR、Com、CanTp、Dcm、ComM／BswM 的迁移并入 #41、#42、#43、#44、#45 各模块 Issue）。
