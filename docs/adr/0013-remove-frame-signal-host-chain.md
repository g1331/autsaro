# 0013. 删除帧／信号 host 链路与工作台“主机运行”页

## 背景

帧／信号 host 链路（模板 `can-empty-v1`、`can-signals-v1`，`prepare_host_project`→`generator/render.rs`、`core/src/host/`、`arxml/host_profile.rs`、`package_host_reference`）使用轮询 `Os_Advance` 和固定 `Rte_Read/WriteSignal`，不是 AUTOSAR OS 或标准 RTE。工作台的“主机运行”页（`ui/src/pages/VirtualPage.tsx`→`host::run`／`run_diagnostic`）只读取该链路的 `profile.txt`，不能运行主线生成的 ECU。

## 决策

1. 删除整条帧／信号 host 链路：两个模板、“主机运行”页、`core/src/host/`、`render.rs`、`host_profile.rs`、`package_host_reference`。
2. 需要空白起点时，提供主线上的最小模板。
3. “在工作台内运行生成的 ECU”保留为产品目标，记为**已知差距**：在主线稳定 host-batch 契约后，基于该契约在主线上重做。在此之前，用封存工程的 `ecu-tool verify` 与原生测试验证行为。
4. 删除前把该链路中有价值的独立预期迁到主线（同 ADR 0010 第 5 条）。

## 后果

- 新建工程的模板选项减少；用户帮助与 README 同步修改。
- 重做前，工作台内没有运行入口，用户需要使用离线工具验证。
- 主线当前借用的 Can、Dem、NvM、Security 来源不随此删除，而是迁入主线（ADR 0010 第 3 条）。

## 状态

已接受（2026-10-11），已实施（#33）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（第 1 节、决定 3）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（B3）。
