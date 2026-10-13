# 0009. 标准公共面、模块私有状态与主机适配分离

## 背景

主机运行时最初在标准头中混入主机输出、注入和测试控制（例如 `Can.h` 同时含标准服务与 `CanTxSink`、`HostFlush`、`Inject`），部分 legacy 接口只是改名而非标准签名。这让“标准名称”与“标准语义”脱节，也让模块之间共享可变状态、拆文件时容易丢失锁协议。

## 决策

1. 对外使用 AUTOSAR 标准名称的接口必须满足适用的标准签名与语义；标准允许实现自定的部分按模块职责设计。改名、typedef 或空壳接口不算对齐。
2. 每个协议模块拥有自身状态与静态配置引用；RTE 拥有本地数据与初始／发布状态；OS／backend 拥有调度；ECU 执行适配拥有批次 admission、队列与收据。两个模块不拥有同一可变状态，私有状态不向应用暴露。
3. 标准公共头、模块私有声明（`<Mip>_Internal.h` 与 `.c` 同处）、主机注入／时钟／输出适配分别组织；标准上下层回调通过模块专用声明连接。
4. 平台原语只经声明的适配边界提供；通用 BSW 不直接使用平台线程或 FreeRTOS API。
5. 故障注入与测试控制只编译进测试构建；生产入口拒绝测试控制源。
6. 共享状态的保护沿既有 SchM exclusive area 协议，不能以“单 owner”假设删除锁；拆分驱动与上层状态锁时核对双向调用与锁序。独占区的划分与持区调用规则见 [ADR 0018](0018-per-module-exclusive-areas-without-cross-module-locking.md)。

## 后果

- 标准整改须同步修改生成来源、配置、调用方与交付工件，并经 ABI 审阅，而不只是头文件改名。
- 主机执行边界清楚后，未来 MCU 适配可以替换适配层而不改标准公共面。
- 有界 host CDD BusSM、`Det_Host` 等主机接缝须明确标注，不冒充完整标准模块。

## 状态

已接受（2026-09 起逐步形成，2026-10-10 统一表述）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture.md`（既有架构与增量记录）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-2）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/BASELINE.md`（模块、状态与直接依赖）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/INTEGRATION-CONTRACT.md`（§3）。
