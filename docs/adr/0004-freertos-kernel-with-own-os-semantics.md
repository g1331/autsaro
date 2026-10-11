# 0004. OS 采用固定 FreeRTOS 内核＋自有 AUTOSAR OS 语义（单核 SC1）

## 背景

参考 ECU 需要符合 R24-11 OS 契约的运行基础，并保持原生 Windows 离线交付。全新自研内核风险过高；原有轮询 `Os_Advance` 只推进虚拟时间，不是 AUTOSAR OS。候选比较（2026-09-28）：

- TOPPERS ATK2-SC1：技术基线较旧，无已核原生 Windows 端口，商用权利需另核。
- Trampoline（固定 `ff287023252bf73273067f00d2dbc8bbc9a20401`）：已有 OSEK／AUTOSAR OS 4.2 语义实现，汽车语义复用更多；但内核为 GPL V2，无原生 Windows OS 目标（POSIX 依赖 Unix 机制），所查 POSIX 栈检查为成功占位，Goil 仅支持旧 ARXML 元模型。
- ERIKA3／OpenERIKA：未取得合法完整包，不具备实施条件。
- 商业 AUTOSAR OS：需采购与授权核验，本项目未承诺费用。
- FreeRTOS V11.3.1（MIT）：已有 MSVC-MingW 主机端口。隔离实验在 Windows／GCC 16.1.0 上复现了原版内核的同级抢占恢复与资源优先级恢复反例，否决了“薄 API 封装即可满足 SC1”的假设；实验也证明原版 Windows 端口的 FreeRTOS 栈数组不是实际执行栈。

## 决策

1. OS 目标为**单核 SC1**，参考配置 Extended Status。
2. 采用固定源码 FreeRTOS Kernel（V11.3.1，commit `054e14f3397023aa83813a65aa065fc4597d481b`）加产品维护的**有限单核策略扩展**，作为唯一运行调度与上下文切换所有者；产品自有层实现 AUTOSAR OS 语义（激活请求 FIFO、Terminate／Chain、事件、资源 ceiling、Counter／Alarm／ScheduleTable、错误／Hook、ARTI），经**唯一 backend** 驱动内核就绪性与优先级。不另建选择下一 runnable 的调度器。
3. RTE／BSW／应用只调用声明的标准接口，不直接依赖 FreeRTOS。AUTOSAR 对象静态配置；Rust 核心从同一 ECUC／计划生成 OS 配置与 `FreeRTOSConfig`。
4. 内核、端口与补丁以版本、摘要、来源与许可固定，不在构建时拉取浮动版本；每项内核补丁有差距说明、独立向量和可审阅 diff，上游升级按同一向量复验。
5. 声明 SC1 的每个目标须证明真实执行栈故障监测与处理；不能用 FreeRTOS 配置栈的 guard／high-water 代替，也不能把主机义务推到 MCU 后继续宣称主机 SC1。
6. Trampoline、ATK2 等保留为参考与备选，不再并列选型。
7. **停止条件**——出现任一项即重新比较路线，而不是继续累积上层功能：无法保存全部激活请求的规范顺序；需要第二个独立 runnable 选择器；必须重写上下文切换／SMP／主要内核机制；无法在所声明目标上满足栈故障契约；补丁无法逐项复验或来源权利不闭合。普通待验证项不重新打开选型。

## 后果

- 产品承担汽车 OS 语义和内核补丁的长期维护成本，换取宽松许可、原生 Windows 支持与源码控制。
- SC1 能力须独立覆盖全部一致性类（BCC1／BCC2／ECC1／ECC2）、最小容量（两个调度表、八个软件 Counter）、错误／Hook／ISR、真实栈与 ARTI，不能缩减为参考配置实际使用的对象。
- 主机逻辑证据不提供硬实时保证；MCU 端口、ISR 与栈证据另行取得。
- 内核复用不等于标准语义已通过；FreeRTOS notification／mutex／timer 不默认等价于 AUTOSAR 事件、资源或 Alarm。

## 状态

已接受（2026-09-28）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/ARCHITECTURE-SPINE.md`（AD-5、AD-6、AD-8）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/FREERTOS-FEASIBILITY.md`、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/TRAMPOLINE-COMPARISON.md`、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/R24-11-CONTRACT.md`（合法取得与可交付路径比较）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/feasibility/`。
