# RTE 与 SchM

RTE 的输入、类型、接口、状态、MemMap 与生成阶段处置移至 [RTE](rte/README.md)；BSW Scheduler 的生命周期与独占区处置见 [SchM](schm.md)。两者来自同一份固定 R24-11 SWS_RTE，核算入口仍是 `python -m autosar_tooling requirements SWS_RTE`。

## 版本来源

RTE BSWMD 的 `SW-VERSION` 和模块手册的配置版本按 `SWS_Rte_05184/05185/05186` 从输入 DOC-REVISION／DATE 选择和解析，保留第四段文档及失败归零。它独立于 `Rte_Version.h` 发布的生成器实现软件版本（项目共享发行 owner `Autsaro_Version.h`），手册明确区分两者。

已核对（`rte_configuration::inspect` 的 `code_vendor_id` 经同一不可变计划传至 `/AUTOSAR_Rte/RteHostImplementation/VENDOR-ID`；`rte_vendor` 的独立结构化输入/输出、基数、拒绝与保存重开测试）：SWS_Rte_05189、ECUC_Rte_09086。`RteGeneration/RteCodeVendorId` 是生成代码使用方输入，恰好一个、无默认值、预编译本地整数 0..65535；显式 0 仍传为 0，缺失不补零。普通 ECU BSWMD 消费该值，独立契约包的 `contract.json` 保留同一配置计划；其他模块供应方身份不从这里取得。参数语法与传递通过不核实其分配资格；正式使用方/供应方身份、身份依赖 API 及剩余出口继续由 #59 承接，不得将代码使用方自动等同工具或 BSW 供应方。

SchM 的 build-version 语义及 false 配置边界见 [SchM](schm.md)。
