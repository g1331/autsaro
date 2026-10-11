# 0010. 单一生成主线

## 背景

2026-10-11 核对发现同一能力有三套生成链路：帧／信号 host（`host-can-v1`）、单组件 ECU（`epic4-win64-sr-cs-v1`）与多组件 ECU（`singlecore-multi-swc-v1`）。选择条件是应用 SWC 类型数大于 1（`multi::selected()`），因此从默认内置模板新建的工程不走多组件主线；主线还借用 legacy 的 Can、Dem、NvM、Security 源码，Dcm 的 producer 身份记录与实际使用的源文件不一致。每个模块的标准整改都要在多套实现上重复投入和验证。

## 决策

1. 新生成的工程全部走当前多组件主线；单组件是 SWC 数为 1 的特例，同一计划与 emitter，不按组件数分派链路。按单 SWC 的 RTE 义务核对主线（不得隐含“至少两个组件”的连接、调度或 RTE 假设）。
2. 主线按职责命名，不再称 multi。
3. 主线拥有全部 BSW 来源：主线借用的 legacy 来源迁入主线，producer 身份与实际字节一致；迁移只改位置不改行为，ABI 与第三方身份审阅。
4. target 独立选择，不同时链接互斥的 OS／RTE／BSW producer；封存 target 的 source／include／宏／patch／main 清单是构建与分析的共同输入，`host-batch`、`probe`、`test` 的程序闭包分别核验。
5. 删除旧链路前，先把其中有价值的独立预期（协议、OS 时序、输入预期）迁到主线并通过；只验证旧链路内部行为的测试才删除。

## 后果

- 模块标准整改只在一条主线上进行，减少重复实现与重复验证。
- profile ID、交付身份、`runtime/contracts`、资产清单发生破坏性变化，写入 CHANGELOG（兼容策略见 ADR 0011）。
- 旧链路上与共用 `runtime/os` 相关的问题（如 Windows 并行 `E_OS_STATE`）不能因删除旧测试而视为不存在，须在主线复现与归属。
- CI 样本集随主线重定，至少覆盖单 SWC 与多 SWC。

## 状态

已接受（2026-10-11），实施中。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（决定 1、3）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-1）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（新基线 B1、B2）。
