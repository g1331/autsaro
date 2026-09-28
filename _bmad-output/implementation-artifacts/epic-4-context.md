# Epic 4 Context: 标准参考输入与示例应用经 RTE 和已声明 OS 运行主机 ECU

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

从核定 R24-11 输入生成可独立构建的 Windows 主机 ECU，让示例 SWC 经生成 RTE 和已声明 OS 运行 CAN 与物理诊断；安全往返配置、独立复验应用行为、闭合全部适用单核 SC1 证据并在新目录重建。规划通过不代表功能通过。

## Stories

- Story 4.1: 建立原创参考输入与可定位的拒绝样例
- Story 4.2: 固定标准义务、依赖来源及独立验收预期
- Story 4.3: 建立固定内核的唯一 backend 与静态生命周期基础
- Story 4.4: 保留全部激活请求 FIFO 与实际内核状态
- Story 4.5: 原子终止、链式移交与新入口执行
- Story 4.6: 实现非抢占、内部资源及资源上限恢复
- Story 4.7: 保持事件所有权与无丢失唤醒
- Story 4.8: 逐毫秒推进受控时间与 Counter/Alarm
- Story 4.9: 捕获 Windows 真实执行栈故障并关闭目标
- Story 4.10: 从标准输入建立唯一校验集成计划
- Story 4.11: 由 SWC 描述生成组件接口与同步服务类型
- Story 4.12: 接入标准输入的安全编辑、保存与重开
- Story 4.13: 从唯一计划生成可链接 ECU 集成工程
- Story 4.14: 通过 HostBatchV1 运行并闭合输出确认
- Story 4.15: 运行同一应用实例的 S/R、快照和同步 DID
- Story 4.16: 独立验证边界、诊断恢复与旧目标回归
- Story 4.17: 完成 Counter/ScheduleTable 能力及 SC1 容量
- Story 4.18: 补齐生命周期、Extended Status、Hook 和中断义务
- Story 4.19: 生成并验证 R24-11 ARTI 描述与 Hook
- Story 4.20: 关闭适用 C 工件、模块描述和静态质量证据
- Story 4.21: 交付新目标可重建包与准确工作台状态
- Story 4.22: 独立复核完整等级与工程交接出口

## Requirements & Constraints

原创单 ECU Extract、ECUC/BSW/SWC 描述及映射是新输入范围。引用、类型、变体、生产者/消费者、符号不闭合时须定位拒绝，保护原输入与旧输出。主机证据不外推 MCU、硬实时、ASIL、完整 MISRA 或官方符合性；失败和 not_run 分别记录。隔离桌面不可用时原生 UI/IPC 保持未验证。

单核 SC1/Extended Status 是完整出口，样例对象数不能缩减等级容量；最小八个软件 Counter、两个 ScheduleTable。中间集成通过只称有界主机集成。全部适用门通过前不标 Epic done。Epic 2 暂缓，Epic 5/6 不扩展。

## Technical Decisions

固定 FreeRTOS V11.3.1 / `054e14f3397023aa83813a65aa065fc4597d481b`、Windows x64/GCC 16.1.0。唯一 backend 复用真实内核就绪队列、运行状态和上下文切换，自有层实现汽车语义及有限单核策略扩展；不另建 runnable 选择器。研究补丁不作生产证据。静态汽车对象及内核对象不等于 Windows 全进程无分配。

`epic4-win64-sr-cs-v1` 以 ARXML 为权威，组件契约生成先行，集成只消费只读 ValidatedIntegrationPlan。单实例不可重入应用，显式未排队 uint32 S/R；同实例同步 C/S DID 0x1234，四字节 UINT8_N ReadData、大端快照、正常 E_OK。单 Extended Task_Ecu 独占应用及 BSW 可变状态。

HostBatchV1 / controlled_logical_ms 以逐毫秒 ticket/确认推进；同 epoch 输入先于 deadline，应用先于 Dcm。单批最多 256 消息、跨度 1000 ms，输出队列 256，COMMIT watchdog 5000 ms；错误批次在改变状态前拒绝。运行输出失败/溢出/超时/初始化失败关闭目标，不声称已执行批次回滚。阻塞 I/O 位于宿主桥。旧轮询目标保留回归，不与新 OS/RTE 同时链接。

交付固定源码、补丁、摘要、来源、MIT 与工具许可，离线构建不拉浮动依赖；官方 Schema/MOD/showcase 是合法取得的外部校验参考，不自动分发。

## UX & Interaction Patterns

展示输入角色、来源、支持边界及定位错误；安全预览、往返保存和导出保护旧目录。分别呈现保存、校验、生成、构建、主机行为、等级审查、实机验证状态。缺依赖/权利不明/验证失败不显示完整交付成功。后台验收，不占用户桌面。

## Cross-Story Dependencies

4.1→4.2；4.3→4.4→4.5→4.6→4.7→4.8。4.9 只依赖 4.3，优先提前验栈。4.10 依赖 4.1/4.2，再到 4.11→4.12，可与 W1 独立推进。

W3/4.13 必须等 4.3–4.9 和 4.10–4.12 全通过，再 4.14→4.15→4.16。4.17 依赖 4.8/4.2，4.18 依赖 4.3–4.9/4.2；4.19 等 4.17/4.18，4.20 等 4.16–4.19，再 4.21→4.22。最终须由非实现者新目录重建并独立复验行为、失败路径及全部适用等级证据。

无法保存全请求 FIFO、需第二调度器/重写主要内核机制、真实 Windows 栈契约不满足或来源权利不闭合时停止累积 W3 后续集成，保留证据交维护者；不自行重新选型或削减 SC1。
