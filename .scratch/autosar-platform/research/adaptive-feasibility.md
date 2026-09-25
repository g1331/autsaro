# AUTOSAR Adaptive 可行性：后续产品决策输入

调查日期：2026-09-24。范围仅为独立、由 AI 编写的开源产品是否应在 **Classic 分阶段覆盖之后**考虑 Adaptive Platform（AP）；不承诺实现，也不以现有/已放弃的栈充当产品基础。下述“已核实”指官方发布材料或项目一手文档的陈述及本次可访问性检查；“判断”是产品推论，不是符合性或法律意见。

## 已核实：规范与可获取工件

- AUTOSAR 官网将 **R25-11** 列为当前 AP 版本，并提供按版本/平台/架构元素过滤的文档入口。R25-11 [AP Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf) 第 3 章列出 RS、SWS、EXP、TPS、TR、MOD 等文件；[Platform Design](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf) 明确仅为总览，细节需各 RS/SWS/TR/EXP。因此公开的 PDF 足以建立规范目录，却不是单份可直接执行的实现说明。[AP 官方入口](https://www.autosar.org/standards/adaptive-platform)。
- 本次直接请求并打开官方文件，核实 [R25-11 Foundation XML Schema ZIP](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip) 含 `AUTOSAR_00054.xsd`、`xml.xsd`、目录示例及说明；说明称只有前者是该包的 AUTOSAR 标准文件，且该 XSD 同时覆盖 CP/AP R25-11。官方 [FO Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf) 还列出共享元模型、序列化规则和 XML Schema 补充材料。**有 XSD 不等于有全部语义约束、生成器或运行时**。
- 本次也直接请求并打开 [AP General Blueprints ZIP](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_MOD_GeneralBlueprints.zip)：含 Communication/Execution/State/Update 等 `.arxml` 蓝图；[Machine Configuration Parameters ZIP](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_MOD_MachineConfigurationParameters.zip) 含一个机器配置参数 `.arxml`。二者与 [Manifest TPS](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TPS_ManifestSpecification.pdf)、[Machine Configuration TPS](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TPS_MachineConfiguration.pdf) 共同构成建模线索，不能把蓝图当完整项目部署实例。AP 总览第 4 章仍列已知技术缺陷，例如 Remote Persistency 服务接口尚无蓝图；“全部资料完备”未经证明。[AP Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf)。
- AUTOSAR 发布页和 PDF 均对资料给出“information only”、著作权/IPR 与商业利用许可提示。公开下载、可阅读、可独立编写开源实现、可再分发规范内容是**不同问题**；本调查不推断哪种产品利用行为获许可，具体许可/商标边界须单独确认。[官方提示](https://www.autosar.org/standards/adaptive-platform)、[Platform Design 免责声明](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf)。

## 已核实：运行环境与 Classic 的根本差别

| 维度 | Classic | Adaptive |
| --- | --- | --- |
| 面向对象与规范粒度 | 深嵌入、硬实时/安全约束 ECU；规范偏模块化 | 高性能计算 ECU；规定功能集群及对应用的 ARA 接口，内部软件架构留给实现者；不是把 Classic 配置器换一套模板。[AP Release Overview §1.3、§1.5](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf) |
| 通信/部署 | 以静态集成和信号通信为主，但 CP **也支持** Ethernet/SOME-IP，不能断言完全无服务通信 | SOA；服务可在设计、启动或运行时绑定；通信管理含服务注册/发现、事件/方法/字段，C++ proxy/skeleton 由接口模型生成。可完全静态配置，也可动态发现，不能断言所有 AP 部署都动态。[Platform Design §§3.1–3.4、8.2–8.5](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf) |
| 执行形态 | 主要深嵌入 ECU 工作流 | 独立进程/线程、隔离与地址空间；应用 OS API 是 POSIX PSE51 子集及 C++ 标准库，平台集群可用更多 OS API；R25-11 最低 C++17。OS 还须支持多进程、隔离、资源组等，不是“任意 Linux + 生成的 C++”即 AP。[OSI §§7–8](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_SWS_OperatingSystemInterface.pdf)、[AP Release Overview §1.5.4](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf) |
| 交付配置 | Classic 路径不应直接照搬 | Application Design 与 Execution、Service Instance、Machine Manifest 分工：启动/资源、服务传输/E2E/安全、网络/服务发现/平台配置；EM 依 Machine/Execution Manifest 启动集群及应用。配置文件、可执行程序、运行时须相互匹配。[Platform Design §§4.4–4.8、6.2](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf) |

独立产品若声称**输出可运行 AP 软件**，至少要定目标 POSIX OS/工具链/C++ ABI，提供或明确兼容的 ARA 功能集群（尤其 EM、CM、必要的 SM）、通信网络绑定与服务发现/IPC、模型到 proxy/skeleton 与各 Manifest 的生成、打包部署/启动和跨机器验证；认证/安全等级又是独立系统集成问题。仅输出 ARXML 或 C++ 源码的产品范围较小，但不能宣称完整 AP 软件。此为依据 [AP 设计中的启动、通信和 manifest 关系](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf) 的**工程判断**。设备驱动、部分内部 FC 交互和 OS 启停细节被规范留作实现相关，需选平台并实测；不能靠规范 PDF 推出可移植二进制。[Platform Design §§4.2、5–8](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf)、[OSI §7.2](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_SWS_OperatingSystemInterface.pdf)。

官方 [CAPI 1.0 发布](https://github.com/AUTOSAR/capi/releases/tag/v1.0.0) 和 [项目 README](https://github.com/AUTOSAR/capi) 说明已有社区源码、代码生成器及 Unix-like/Docker 演示，主要基于 **R20-11**、部分 R23-11 接口；与当前 R25-11 不是同一版本。它证明真实运行链涉及 SDK 打包、应用构建、机器配置、进程启动与状态切换，但本产品的独立 AI 创作前提下**不复制/复用该栈或其代码**；它的发布不解决本项目的兼容、授权或完整性判断。官网称其为 community-source、README 有额外使用/参与条件，不能仅因仓库公开即称为可纳入本项目的开源依赖。[CAPI 官方页](https://www.autosar.org/capi)。

## 未知项与建议

1. **产品边界未定**：仅生成应用及配置，还是交付完整 ARA/OS 集成与运行环境？目标 AP 版本、首批功能集群、SOME/IP/DDS/IPC 绑定、目标硬件/OS/工具链及互通对手尚未指定；这些决定工作量，不能给出可信人月或符合性结论。[AP 规范目录](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf)。
2. **资料覆盖需做样本闭环**：选一组服务接口、应用设计、三个 Manifest 与机器配置，逐项用 R25-11 XSD、TPS、SWS 检查约束和代码生成/部署语义；蓝图不是即用的整车项目，XSD 验证也不证明行为正确。[Schema ZIP](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip)、[Blueprints ZIP](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_MOD_GeneralBlueprints.zip)、[Platform Design §4](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf)。
3. **许可与验证仍未定**：独立发布/商用/标识与引用规范内容的边界需取得可适用的书面解释；目标运行时、互通测试设备与验收标准未确定，官方规范的公开可下载不等于已经证明符合性。[AUTOSAR 使用提示](https://www.autosar.org/standards/adaptive-platform)。

**建议（产品判断）**：保持该项为研究，不进入当前 Classic 阶段实现。只有 Classic 阶段的产物和用户需求显示 AP 应用/配置生成有明确价值，并确定目标版本、运行时/OS 对手、可测互通样例及可接受的发布边界后，再为 AP 单独开产品决策票；先做限定范围的规范—工件—运行闭环，不预先承诺完整 AP。AP 的功能集群、进程运行时和部署模型是新的产品面，而不是 Classic 的自然增量。[AP Release Overview §1.5](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_TR_ReleaseOverview.pdf)、[Platform Design §§4–8](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_AP_EXP_PlatformDesign.pdf)。
