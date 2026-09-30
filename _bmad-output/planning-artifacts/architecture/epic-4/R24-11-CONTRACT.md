# Epic 4：R24-11 规范核定与候选路径

核定日期：2026-09-28。用途：供维护者决定实施路径，并使后续 spec/story 引用同一输入、接口和证据契约。本文区分“官方义务”“产品剖面建议”“当前代码事实”；它不是符合性报告，也没有可交付的新输入或产品 OS 端口；隔离机制实验见 [可行性调查](FREERTOS-FEASIBILITY.md)。架构规则见 [spine](ARCHITECTURE-SPINE.md)，决策理由与未决项见 [.memlog.md](.memlog.md)。

## 来源与核查方法

本机九份 R24-11 官方 PDF 均重新计算 SHA-256，并匹配 `docs/official/R24-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256`；同时读取官方 ECUC ZIP 的 Os/Rte/Dcm 定义。部分官网 PDF 的网页读取超时，具体条款以已核验本机原件为依据。原件留在被忽略的官方资料目录，文字提取和候选下载留在本机临时目录，均不进入规划待提交文件。

| 官方文件 | 本次定位 | 对 Epic 4 的约束 |
| --- | --- | --- |
| [TR_Methodology](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TR_Methodology.pdf) | §2.7.2.2.1–2.7.2.2.2，p102–104；`TR_METH_01114/01116/01117` | ECU 配置输入包含 ECU Extract 与所选 BSW 实现的模块交付信息；配置值覆盖该 ECU 的 BSW；每个模块须选择实现 |
| [TPS_SystemTemplate](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TPS_SystemTemplate.pdf) | System.category 的 `ECU_EXTRACT`，§14；`TPS_SYST_01006` | Extract 是面向目标 ECU 的展开系统描述，不是一个随意命名的 ECUC 文件；保留 ECU 实例、应用及通信映射 |
| [TPS_SoftwareComponentTemplate](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TPS_SoftwareComponentTemplate.pdf) | ApplicationSwComponentType、P/RPortPrototype、数据类型映射、SwcInternalBehavior、RunnableEntity、TimingEvent、OperationInvokedEvent | 组件接口、访问、行为和实例不能从 Com 信号名字猜测 |
| [TPS_BSWModuleDescriptionTemplate](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TPS_BSWModuleDescriptionTemplate.pdf) | BswModuleDescription、BswImplementation、BswInternalBehavior、BswModuleEntry、BswSchedulableEntity、BswEvent | 描述须对应真实自有 BSW 实现、入口和调度要求；官方 ECUC 定义不代替实现描述 |
| [TPS_ECUConfiguration](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TPS_ECUConfiguration.pdf) 与 `MOD_ECUConfigurationParameters.zip` | ECUC 定义/值、重数、引用、配置类；读取 Os/Rte/Dcm 模块 | 校验 XSD 之外的引用、值与实现约束；厂商差异定义须与标准来源区分 |
| [SWS_RTE](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_RTE.pdf) | §3.1 契约阶段（p92起）、§3.2 生成阶段；`SWS_Rte_07804/07805`；`ECUC_Rte_09024`（Alarm）/`ECUC_Rte_09026`（ExpiryPoint）；`09025` 是 OsEvent 引用 | 组件 API 先由组件描述产生，再按 ECU 配置集成；TimingEvent 尊重已配置 Alarm/ExpiryPoint 与偏移 |
| [SWS_DiagnosticCommunicationManager](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) | §7.4.2 的 0x22；`SWS_Dcm_00433–00437`；`ECUC_Dcm_00713/00669` | DID 访问有标准函数和 C/S 等可选机制；先核对 DID/会话/长度等条件，再调用已选数据访问契约 |
| [SWS_OS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_OS.pdf) / [RS_OS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_RS_OS.pdf) | §7.1/7.3/7.5/7.11/7.16/7.17、§8/10；等级与证据矩阵如下 | 先声明等级与配置，再选择实现；主机逻辑验证不代替 MCU 要求 |

这些定位项是架构适用性入口，**不是所有适用条款已逐项核完**；选定实现后必须建立条款→配置→实现→证据的完整差距表。尤其 OS SWS §7.1 引用的 OSEK OS 基础义务需一并纳入，不能仅检索 `SWS_Os` 标识。RS→SWS 的追踪关系不等于某内核已通过测试。

## 参考输入与产物责任

推荐产品自行创作一个最小 ECU Extract 及示例应用，避免以官方 showcase 再分发权作为交付前提。官方 showcase 仅供本地建模比对；现有导入其 15 份文件的测试不证明下面契约已实现。样例尚未编写、XSD/语义验证或运行；下表是核定的输入角色与已采用支持范围，实际正反例归属 W0 输入工作包。

| 输入角色 | 来源/首版建议 | 必需内容与拒绝 | 输出及责任 |
| --- | --- | --- | --- |
| ECU Extract | 产品原创参考样例；用户可提供相同受支持契约；接受预先提取的单 ECU | `System.category=ECU_EXTRACT`、目标 EcuInstance、已展开原子 SWC 实例与 ECU 映射、单 CAN 的信号→I-Signal/I-PDU→Frame/触发与方向映射；缺目标/跨 ECU引用不闭合/多个目标未选则拒绝 | 核心解析与生成计划；首版不由工具完成任意系统提取 |
| SWC 与数据类型描述 | 产品原创最小 ApplicationSwComponentType | 单实例；P/R 端口与接口、应用/实现类型映射、数据访问、Runnable/Event、实现符号；方向/类型/引用/重入不匹配或未支持访问机制拒绝 | 契约头文件、RTE 类型与 ECU 专属实现；应用实现由产品参考代码或用户提供 |
| BSW 交付信息 | 产品维护自有 Com/CAN/DoCAN/Dcm 及实际启用系统模块；OS 由已选实现提供 | 对应实际实现的模块入口/内部行为、调度实体/Event、配置定义与依赖；不存在的源码/符号/版本不补空壳 | 可复用 BSW 源码＋生成配置、SchM 与服务连接；不采购另一套 BSW 协议栈 |
| ECUC 值及定义来源 | 工作台/用户；R24-11 标准 MOD 为外部来源，实现差异定义单列 | 包含本 ECU 启用模块及 Rte/Os 对象；`DEFINITION-REF`、配置类/重数/目标及跨模块引用正确；Extract 与 ECUC 信号、帧、类型或方向矛盾拒绝 | 一份受校验 ECU 配置和目标集成计划；不以官方 MOD 内容宣布自有实现支持全部参数 |
| RTE/BSW/OS 映射 | ECUC 中显式建立；由核心校验 | RteEventToTaskMapping / RteBswEventToTaskMapping、任务位置、周期及 Alarm/ExpiryPoint、ExclusiveArea；缺映射/生产者/服务实现拒绝 | 生成任务体、调度接口与 OS 配置；配置器的输入是可追溯派生产物，不另作配置权威 |
| OS、端口、配置器、工具链 | 已选 FreeRTOS 路线；固定实际交付组合 | 原生 Windows 路径、对象配置和 ABI 必须实证；许可或符号闭包不明时不打包 | 外部依赖档案、源码或明确取得方式、配置产物与独立构建入口 |
| XSD/官方规范与样例 | 用户合法取得；本地外部参考 | R24-11 `AUTOSAR_00053.xsd`；缺失时不声称输入已验证；不静默使用另一版 | 校验结果与来源/摘要记录；官方原件不隐含进入参考包 |

同一逻辑模型可分多个 ARXML 文件；上表不是强制文件名或“每角色一个文件”。文件边界、对象身份与来源须保留；输入原件和最终产物一同追溯，不能只交付转换后的 OS 配置。

输入来源链沿用 Epic 3 的逻辑相对文件身份与原始字节 SHA-256，文件所属工作区是引用解析域；对象使用完整 AUTOSAR 路径及实例引用，不用规范化模型摘要替代源文件完整性。新角色/目标元数据若扩展交接格式须显式版本化；参考剖面标识、角色及依赖版次可追溯，不能以文件名字猜测角色。

## 最小 SWC/RTE 与运行剖面建议

| 维度 | 推荐首版 | 证据/拒绝 |
| --- | --- | --- |
| 信号 | 单 CAN、11 位 Classical CAN、显式未排队 uint32 S/R Tx/Rx；沿用当前 LSB0 打包能力但由系统映射核对 | 独立对端固定向量证明 SWC→RTE→Com→CAN 与 CAN→Com→RTE→SWC；类型、方向、连接、端序/位宽冲突拒绝 |
| 应用执行 | 一个单实例、不可重入组件；周期 Runnable；初始值、周期、位宽和错误语义显式配置 | 生成组件 API 与集成头一致；应用只调用声明 RTE API；按任务位置、周期及偏移复验，不能由测试器直接写 Com 代替应用 |
| 应用 DID | Q2 已确认一个固定长度同步 C/S ReadData；函数机制仅为比较中的合法替代 | 对端 0x22 结果与应用快照一致；长度、会话、无效值及未支持服务有协议拒绝；不把现有私有 DID→TxSignal 绑定当成 SWC 服务描述 |
| 生命周期/共享状态 | OS 启动→BSW 初始化→RTE 启动→周期执行；应用拥有快照，RTE/SchM 执行声明同步 | OS/BSW/RTE 初始化顺序及 ready 条件、同刻事件顺序、缓冲所有者和错误传播按集成契约固定；启动前服务、队列满和重复激活须拒绝/按契约报告 |
| 诊断回归 | 一条物理 normal-addressing DoCAN；重测已声明服务及计时行为，不扩大服务清单 | Epic 4 的应用 DID 是新增集成成果；DTC/NvM、完整 NM 支持矩阵归 Epic 5；原主机用例与新 OS 目标分别取证 |

精确 CAN ID、DID 数值、周期、tick 单位、timeout、任务映射与字节预期已在 [集成契约](INTEGRATION-CONTRACT.md) 固定，不能被两个 story 各自选择。它们是产品样例参数，不是从规范推出的唯一默认值；本轮没有生成或验证这些新参数。

跨 story 不变量：DID 服务器为同一应用组件实例的提供端口/操作及 `OperationInvokedEvent` Runnable；Dcm 的服务使用端、标准接口签名、连接、生成符号按集成契约及 Dcm/RTE 定义逐项生成/校验。每个 BSW 周期实体有唯一调用者，计划显式排列其与 Runnable 的执行/可见性关系。目标入口为整个 ECU 分配统一 tick/epoch 和序号；同刻输入、计时、ISR、任务与超时顺序由统一目标契约确定，与 OS 实际语义不符或未明确时阻断生成，不使用多个私有队列的线程到达顺序作产品语义。

## SC1 能力、参考配置与目标证据

官方 `SWS_Os_00241` 与 Table 7.4（p129–131）给出 SC1 基础能力。建议选 SC1 是产品取舍，不是现有内核符合性结论。下列“配置集成”允许样例用少量对象；“等级能力”仍须独立覆盖适用能力及最小容量。

| 义务族/出处 | 实现能力核查 | 参考配置及验收 |
| --- | --- | --- |
| OSEK OS：§7.1、`SWS_Os_00241` | 各一致性类 BCC1/BCC2/ECC1/ECC2；Basic/Extended Task、多次激活/同优先级、抢占/非抢占、Chain/Terminate、事件、资源优先级上限及合法调用层级 | 首个 ECU 只选择实际需要的任务/事件/resource；激活上限、非法调用、事件与资源语义用独立测试，不能把 FreeRTOS notification/mutex 默认当等价 |
| Counter/SWFRT：§8，Table 7.4 | IncrementCounter/GetCounterValue/GetElapsedValue、回绕与对象合法性；至少八个软件计数器容量 | 参考配置可用一个计数器；统一 tick→周期/诊断时间换算，覆盖回绕、相同 tick 与溢出/拒绝 |
| Alarm/ScheduleTable：§7.3/8/10 | OSEK Alarm、调度表状态/启动/停止/切换及适用接口；至少两个调度表容量 | 配置可选 Alarm 激活 TimingEvent，不免除实现的调度表能力；复验已配置周期、位置、偏移及容量测试 |
| Stack：§7.5；`SWS_Os_00067/00068/00396` | Task/Category2 ISR 栈故障检测；无 ProtectionHook 调 ShutdownOS，有 Hook 按规范处理 | 主机端口需能检测其真实执行栈或声明未实现；仅人为翻 flag 不证明检测义务。故障注入必须隔离并可复现 |
| 生命周期、ISR/错误/Hook：§7.1/8/10 | StartOS/ShutdownOS、启动模式、Extended Status 与适用 Hook/ISR/中断禁用配对 | 参考推荐 Extended Status、单核、无保护 OS-Application；真实 ISR 为 Epic 6 证据，主机只证明模拟边界，不能从 SC1 自动省略基础中断语义 |
| R24-11 ARTI：§7.16/7.17；`SWS_Os_00858/00829/00836/00837` | 模块描述/ORTI 容器覆盖及 Hook 结构、状态对应；评估每条适用性 | 旧 OS 的日志宏不直接替代 ARTI；生成描述与实际对象匹配，接口/状态轨迹有证据，未核项记 未执行 |
| 扩展功能：`SWS_Os_00240`、§7.11 | 提供高等级功能时其接口仍须符合对应规范 | 首版不承诺 SC2–4、时间/内存保护、多核或 IOC；不能把宿主隔离当成 AUTOSAR 保护能力 |

这张表明确义务族和验证出口，尚不是逐条完整的 SC1 符合性清单。OS-Application、ProtectionHook、ARTI 等应按完整规范与所选配置审查，不能根据等级表中一个 Yes/空格擅自豁免。

## 合法取得与可交付路径比较

以下为当前可核查的候选事实及架构判断；没有向产品集成候选内核，也未采购许可；已构建运行固定 FreeRTOS 的隔离机制实验，其他候选未运行。源码许可、AUTOSAR 知识产权、端口和实际再分发权分别检查。

| 路径 | 已核事实与来源 | 未关闭门/影响 | 建议 |
| --- | --- | --- | --- |
| TOPPERS ATK2-SC1 | [官方公开包](https://www.toppers.jp/atk2-download.html) 1.4.2；[官方技术基线](https://www.toppers.jp/atk2.html) 为 4.0 Rev3。实际下载非依赖部包并查看 Os.h/任务、计数器、调度表源码及版权头 | 尚无已核原生 Windows/S32K344 端口；R24-11 差距尤其 ARTI 待补；[公开测试套件获取有条件](https://www.toppers.jp/atk2.html)。[源码条件](https://www.toppers.jp/license.html)不授 AUTOSAR IP；[项目明确提醒商用权利并不称其为无条件开源](https://www.toppers.jp/autosar.html) | 保留为候选及语义研究对象；旧版差距、公开维护/测试与目标支持不足以直接绑定。当前推荐已调整为 FreeRTOS＋自有语义实现 |
| Trampoline | 固定 ff287023252bf73273067f00d2dbc8bbc9a20401 源码已读取；已有 OSEK/OS 4.2 任务、事件、resource、调度表及断言测试；所查内核 GPL V2、模板许可混合 | 未见原生 Windows OS 目标；POSIX 依赖 Unix 机制；Goil 3.1.16 及旧 ARXML 不证明 R24-11 支持；所查 POSIX 栈检查为成功占位；完整差距/许可组合/端口待证 | 比较后保留参考/备选，未采用；可减少汽车语义开发，不能直接替换当前 Windows 路径。[具体比较](TRAMPOLINE-COMPARISON.md) |
| ERIKA3 / OpenERIKA | [ERIKA3 许可页](https://www.erika-enterprise.com/erika3/licensing/)区分 GPL、第三方例外及付费应用链接/商业方案；[发布方说明 OpenERIKA 为 APD、成员获取](https://erika.tuxfamily.org/drupal/content/welcome-erika-enterprise-v2x-website) | 没有取得当前合法完整包和配置器；本次上游仓库读取失败；Windows/MCU 与 R24-11 证据未核。不能把 ERIKA2 的链接例外套给 ERIKA3，或以“Open”推断开放获取 | 有权利/来源条件的备选，当前不具实施就绪 |
| FreeRTOS＋自有 AUTOSAR OS 兼容实现 | [Kernel 当前 release](https://github.com/FreeRTOS/FreeRTOS-Kernel/releases) V11.3.1；[MIT](https://raw.githubusercontent.com/FreeRTOS/FreeRTOS-Kernel/V11.3.1/LICENSE.md)；[Windows simulator 源码](https://github.com/FreeRTOS/FreeRTOS-Kernel/blob/V11.3.1/portable/MSVC-MingW/port.c) | Windows 主机便利；需自有实现 OSEK 激活/终止/事件/resource、计数器/调度表、状态/错误/Hook/ARTI，测试与维护成本最高。[官方主机说明](https://en.freertos.org/Documentation/03-Libraries/02-FreeRTOS-plus/FreeRTOS-Plus/WolfSSL/FreeRTOS_WolfSSL_Example)不保证实时行为；完整兼容不是 API 改名 | **主路线已采用，生产依赖未引入**：复用内核与端口机制，产品自有汽车 OS 语义；先关闭下列可行性门。原生 FreeRTOS 实验不满足 Epic 4 OS 出口 |
| 合法商业 AUTOSAR OS | [ETAS RTA-CAR 12.3.2](https://www.etas.com/ww/en/downloads/software-downloads-overview/rta-car-v12-3-2/)列 RTA-OS12.3.0；[官方 VRTA/MinGW 工程示例](https://www.etas.com/ww/media/a_downloads/etas-rta-fbl-stla-2-0-0-user-manual-r01-en-202407.pdf)显示主机路线 | 当前版本匹配的 Windows/MCU 端口、配置器、R24-11 覆盖、采购与接收者授权/分发权须实际取得并核验；旧示例不是该组合已通过 | 若有预算，优先询价取得授权包的工程路径；本轮不联系厂商或承诺费用 |

实际读取的 ATK2-SC1 1.4.2 非依赖部包摘要为 `ae02b8433f83f77661e8f202101c2b5f6cf8e12aadfe94ab666aef9b527a2e1d`。包内有 `StartOS`、计数器和调度表实现；未搜索到 ARTI 标识，非依赖部包本身没有 Windows/POSIX 目标目录。这只说明已查看的包及搜索结果，不推定整个项目无端口，也不以符号存在证明语义通过。FreeRTOS V11.3.1 实际源码和隔离构建结果已核查，见 [可行性报告](FREERTOS-FEASIBILITY.md)；不加入产品依赖。其他候选未获得可审计包版本，不能填造哈希。

## 选择与就绪出口

**Q1 已确认（主路线定案）：**以 SC1 为目标，采用 FreeRTOS 固定源码内核＋产品自有 AUTOSAR OS 语义实现＋Rust 静态配置生成＋独立 backend/目标端口。维护者希望自主掌控，也明确指出全新内核风险过高；因此复用成熟的上下文切换、调度和端口基础，避免同时自研所有 CPU 机制。路线已获维护者确认，生产依赖尚未引入；不能静默退成原生 FreeRTOS 或轮询器。本轮反例已否决“薄封装足够”的假设；已决定维护有限单核 ready 策略与原子转换扩展，保持唯一运行调度器。有限机制可运行与完整 SC1 可交付是不同结论。

| 复用与自研边界 | 可行性关口 |
| --- | --- |
| 内核负责运行调度与上下文；产品层负责汽车 OS 对象与契约 | FreeRTOS 管理实际就绪队列/Running 与抢占，产品层按经过证明的状态映射驱动就绪性/优先级，不另选下一 runnable；重复激活、TerminateTask/ChainTask 的状态/栈重新开始必须满足汽车契约；本轮的稳定 wrapper/非局部退出仅验证机制，不作为生产重启方案；全部激活请求按优先级内的请求顺序保存，不能以计数＋无条件尾部重排代替 |
| 固定优先级和内核同步机制可以复用，但不直接作为标准证明 | 同优先级/非抢占、资源优先级上限/嵌套、事件等待与清除竞态、ISR/调用层级由标准差距向量证明；FreeRTOS mutex/timer/notification 不默认等价 |
| 自有计数器/告警/调度表、错误/Hook 与 ARTI；通过 backend 接入内核 | 明确同 tick 顺序、回绕、容量及错误传播；内核 tick 与汽车计时由目标契约转换，不产生第二个调度器；原生栈检查须核实目标覆盖范围 |
| Rust 统一生成静态对象和 OS/FreeRTOS 配置 | 同一源 ECUC/计划为权威；FreeRTOS 对象优先静态创建，内核/端口隐藏任务、内部/宿主分配、编译器 ABI 与 32/64 位需求须核查 |
| 源码控制通过固定快照、许可记录和独立补丁维护获得 | V11.3.1 的实际 commit `054e14f3397023aa83813a65aa065fc4597d481b`、归档/关键源码摘要和 GCC 16.1.0 已固定，Windows 隔离研究构建已通过。架构路线已采用，生产依赖和交付组合仍须核定。公共 API 优先，确需内核修改时逐项审阅和验证；若需广泛替换主要调度机制，触发停止条件后重新评估路线 |

独立向量必须区分合法与非法的激活/终止/ChainTask、原子移交和栈重启、同优先级顺序、嵌套资源及 ISR 调用层级；不能只用一个参考 ECU 的简单周期场景证明全部能力。

原生 Windows MSVC-MingW 端口用于有界逻辑验证，运行时严格避免被调度任务中的宿主阻塞 I/O；其源码明确不提供真实实时行为。原版端口没有实际执行栈监测证据；主机 SC1 声明也须先解决真实线程栈和故障处理，不能以 MCU 证据代替。硬件 ISR、时序及 MCU 栈故障检测另建 MCU 证据；QEMU/CPU 模拟如需引入也是独立待决工具路径，本轮不安装、不改变交付依赖。自研语义实现减少厂商绑定，但其正确性、内核补丁和标准/依赖权利仍由产品负责。

**Q2 已确认：**维护者选择一个组件的显式 S/R＋一个同步 C/S 应用 DID。它同时验证周期通信和标准 BSW 服务接入；S/R＋Dcm 同步函数可减小首版生成面，但必须准确缩减 C/S 声明。

Q1/Q2 已确认，架构和跨模块契约已收口。后续按 [集成契约 W0–W5](INTEGRATION-CONTRACT.md) 规格化 stories、建立原创输入/条款/来源清单并交付实现证据；完整 SC1 属于 W5 退出门，真实栈与核心语义属于 W1 先验关口，不以尚未实现阻断架构定稿。交付声明仍按实际证据，不降低 FR-6。

当前只读核查确认 HEAD `817063e` 的 Epic 3 已有可重建包、重开和离线参考入口，三个 story 与 sprint 均为 done；依据为该提交、[_bmad-output 中三份 story 记录](../../../implementation-artifacts/3-3-verify-delivered-host-behavior-without-the-workbench.md)与当前源码，本轮未重跑其构建/运行/GUI 验收。原规划“生成目录不含 ARXML”已陈旧。`Rte.c` 和 `Os.c` 仍分别是固定 Com wrapper 和 `Os_Advance` 轮询，因此 Epic 4 仍 **FAIL / backlog / 无 ready-for-dev stories**。Epic 5/6 的 NM、诊断持久化和 MCU 门不由此解除；BMad 实现记录不升级。
