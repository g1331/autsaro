# 2026-09-28 Epic 4 实施就绪判断

**Epic 4 架构已收口；故事级实施就绪为 NOT READY，Epic 4 保持 backlog。**这两个结论分别回答“设计能否指导实施”和“是否已有可直接派发的故事”。本轮完成架构与规划，没有开始功能开发、引入生产 OS 依赖或升级能力声明。

## 当前事实

HEAD `817063e` 与 sprint 一致：Epic 1/3 完成，Epic 2/Story 2.1 暂缓，Epic 4–6 backlog。现有 `runtime/src/Rte.c` 仍是固定 Com wrapper，`Os.c` 仍为 Os_Advance 轮询；新架构是新增目标设计，不能据其声明源码已支持。能力档案七项 documented_behavior 不变。本轮未重跑 Epic 3 构建、行为或 GUI 验收。

## 架构与实施关口

权威规则见 [spine](architecture/epic-4/ARCHITECTURE-SPINE.md)，具体边界见 [集成契约](architecture/epic-4/INTEGRATION-CONTRACT.md)，官方依据见 [规范矩阵](architecture/epic-4/R24-11-CONTRACT.md)。

| 关口 | 架构结果 | 实施证据及归属 |
| --- | --- | --- |
| G4-1 输入角色/来源 | PASS：原创单 ECU Extract、SWC/类型、实际 BSW 描述/定义、ECUC 值与依赖闭包固定 | W0 生成原创正反输入、验证 XSD/语义、逐项条款与来源清单；当前无新 fixture 验收 |
| G4-2 SWC/RTE/DID | PASS：显式 uint32 S/R、四字节同步 C/S、签名/返回/快照/端序固定 | W2 契约生成与闭包，W4 独立 CAN/UDS 向量 |
| G4-3 OS 路线/等级 | PASS：已确认固定 FreeRTOS＋自有语义＋有限单核扩展，SC1/Extended Status 目标 | W1 核心 FIFO、原子转换、事件/资源等；W5 全部适用 SC1 容量/Hook/ARTI/错误与等级出口 |
| G4-4 Windows/交付 | PASS：固定内核版本/摘要、MIT、原生 Windows x64/GCC、真实线程栈策略与停止条件 | W1 先证实真实栈捕获/关闭和受控端口；W5 完整许可、源码/补丁/工具档案和新目录独立复验 |
| G4-5 时间/共享状态 | PASS：逐毫秒请求/确认、HostBatchV1、单 Task_Ecu、固定同刻顺序、容量和故障固定 | W1 受控 tick；W3 生成集成；W4 输入/超时/确认/应用可见性验证 |
| G4-6 可派发故事 | NOT READY：工作包及依赖已固定，但尚无故事级规格、产物和验收工件 | 下一实施规划据 W0–W5 拆故事；各故事进入前具备其具体输入/依赖/验收，不要求自身或整个 OS 先已实现 |

架构 PASS 表示设计已核定，不能读取为对应功能或标准符合性 PASS。研究结果见 [FreeRTOS 调查](architecture/epic-4/FREERTOS-FEASIBILITY.md)：原版同级顺序/资源恢复反例否决薄封装；66 条隔离观测不证明完整 FIFO、SC1 或生产栈监测。

## 依赖和停止条件

W0 输入基线和 W1 backend 基础可从最终架构开始规格化；W2 依赖 W0，可与 W1 并行。W3 集成须等 W1 核心语义、受控时间和真实栈验证，以及 W2 生成闭包通过。W4 验证应用闭环；W5 完成全部适用 SC1 与独立交接出口。完整 SC1 是 Epic 退出门，不是架构定稿或 W1 开始的前置条件。

只有有限扩展失去维护边界、需要替换主要调度器，或真实 Windows 栈契约被证实无法满足时才重新评估路线；普通未实现项不再打开候选选型。Trampoline/ATK2 保留参考与备选。

Epic 5 的 NM、DTC/NvM 与持久化关口及 Epic 6 的板卡/合法 MCU 依赖/端口/实机证据仍未解除。Epic 2 保持暂缓。本轮不派发 bmad-build，不改变 sprint 或 capability 状态。
