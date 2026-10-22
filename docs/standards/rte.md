# RTE 与 SchM

RTE 的输入、类型、接口、状态、MemMap 与生成阶段处置移至 [RTE](rte/README.md)；BSW Scheduler 的生命周期与独占区处置见 [SchM](schm.md)。两者来自同一份固定 R24-11 SWS_RTE，核算入口仍是 `python -m autosar_tooling requirements SWS_RTE`。

## 版本来源

RTE BSWMD 的 `SW-VERSION` 和模块手册的配置版本按 `SWS_Rte_05184/05185/05186` 从输入 DOC-REVISION／DATE 选择和解析，保留第四段文档及失败归零。它独立于 `Rte_Version.h` 发布的生成器实现软件版本（项目共享发行 owner `Autsaro_Version.h`），手册明确区分两者。`SWS_Rte_05189` 的 RteCodeVendorId 来源传递与正式使用方身份继续由 #59 承接；不得将使用方自动等同实现供应方。SchM 的 build-version 语义及 false 配置边界见 [SchM](schm.md)。
