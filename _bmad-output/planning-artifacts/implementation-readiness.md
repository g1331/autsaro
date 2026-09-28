# 2026-09-28 Epic 4 实施就绪判断

**Epic 4 架构已收口；W0–W5 共 22 条故事已确认并通过本范围规划校验；故事规划流程已完成，Epic 4 实施保持 backlog。**规划通过表示已有具体输入、依赖、产物和可执行验收要求，不表示全部故事前置结果已完成或已有 ready-for-dev 开发工件。本轮只完成故事规划与记录同步，没有开始功能开发、引入生产 OS 依赖或升级能力声明。

## 当前事实

当前已提交架构基线为 `e5b84a6`，Epic 3 的实现提交为 `817063e`；sprint 仍记录 Epic 1/3 完成、Epic 2/Story 2.1 暂缓、Epic 4–6 backlog。现有 `runtime/src/Rte.c` 仍是固定 Com wrapper，`Os.c` 仍为 Os_Advance 轮询；新架构是新增目标设计，不能据其声明源码已支持。能力档案七项 documented_behavior 不变。本轮未重跑 Epic 3 构建、行为或 GUI 验收；22 个新测试入口是未来实施要求，功能验收均为 not_run。

## 架构与实施关口

权威规则见 [spine](architecture/epic-4/ARCHITECTURE-SPINE.md)，具体边界见 [集成契约](architecture/epic-4/INTEGRATION-CONTRACT.md)，官方依据见 [规范矩阵](architecture/epic-4/R24-11-CONTRACT.md)。

| 关口 | 架构结果 | 实施证据及归属 |
| --- | --- | --- |
| G4-1 输入角色/来源 | PASS：原创单 ECU Extract、SWC/类型、实际 BSW 描述/定义、ECUC 值与依赖闭包固定 | W0 生成原创正反输入、验证 XSD/语义、逐项条款与来源清单；当前无新 fixture 验收 |
| G4-2 SWC/RTE/DID | PASS：显式 uint32 S/R、四字节同步 C/S、签名/返回/快照/端序固定 | W2 契约生成与闭包，W4 独立 CAN/UDS 向量 |
| G4-3 OS 路线/等级 | PASS：已确认固定 FreeRTOS＋自有语义＋有限单核扩展，SC1/Extended Status 目标 | W1 核心 FIFO、原子转换、事件/资源等；W5 全部适用 SC1 容量/Hook/ARTI/错误与等级出口 |
| G4-4 Windows/交付 | PASS：固定内核版本/摘要、MIT、原生 Windows x64/GCC、真实线程栈策略与停止条件 | W1 先证实真实栈捕获/关闭和受控端口；W5 完整许可、源码/补丁/工具档案和新目录独立复验 |
| G4-5 时间/共享状态 | PASS：逐毫秒请求/确认、HostBatchV1、单 Task_Ecu、固定同刻顺序、容量和故障固定 | W1 受控 tick；W3 生成集成；W4 输入/超时/确认/应用可见性验证 |
| G4-6 故事规划及进入条件 | 规划 PASS：22 条故事已有依赖、产物、独立预期、执行入口和正反验收；未创建 ready-for-dev 工件 | [故事索引](epics.md#w0w5-故事索引与实施规则)：W0=4.1–4.2、W1=4.3–4.9、W2=4.10–4.12、W3=4.13–4.14、W4=4.15–4.16、W5=4.17–4.22；实际进入前形成开发规格、冻结起始提交及检查具体依赖，不要求自身或整个 OS 先已实现 |

架构 PASS 表示设计已核定，不能读取为对应功能或标准符合性 PASS。研究结果见 [FreeRTOS 调查](architecture/epic-4/FREERTOS-FEASIBILITY.md)：原版同级顺序/资源恢复反例否决薄封装；66 条隔离观测不证明完整 FIFO、SC1 或生产栈监测。

## 依赖和停止条件

W0 输入基线和 W1 backend 基础可从最终架构开始；W2 依赖 W0，可与 W1 并行。4.9 真实栈验证只依赖 4.3，可提前止损；W3/4.13 集成必须等待 4.3–4.9 核心语义、受控时间、真实栈和 4.10–4.12 输入/契约闭包全部通过。W4 验证应用闭环；W5 完成全部适用 SC1 与独立交接出口，其中 4.17/4.18 可按已实现 OS 基础独立推进。完整 SC1 是 Epic 退出门，不是架构定稿或 W1 开始的前置条件。

只有有限扩展失去维护边界、需要替换主要调度器，或真实 Windows 栈契约被证实无法满足时才重新评估路线；普通未实现项不再打开候选选型。Trampoline/ATK2 保留参考与备选。

Epic 5 的 NM、DTC/NvM 与持久化关口及 Epic 6 的板卡/合法 MCU 依赖/端口/实机证据仍未解除。Epic 2 保持暂缓。本轮不派发 bmad-build，不改变 sprint 或 capability 状态。
