# RTE 与生成契约

本篇属于[标准与规范基线](../README.md)，按[ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md)处置固定 R24-11 `SWS_RTE` 的条目。原文为 `AUTOSAR_CP_SWS_RTE.pdf`（SHA-256 `d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）与固定 SpecificationsARXML；编号目录包含 2884 条，含 draft／obsolete 状态。

主线选择单核、单 owner、源码 SWC 每类型单实例、显式非队列 uint32 S/R、同步直接 C/S、固定四字节 OUT 数组、TimingEvent 与预编译配置。契约阶段与 ECU 阶段消费同一个已验证 `CompositionContract`；普通生成不需要编译器或官方档案。原文的工具能力要求与某次配置的运行义务分开：未支持的其他机制保留在 [#212](https://github.com/g1331/autsaro/issues/212)，不把拒绝某个可选配置等同于实现该机制。

- [接口、状态与输入](contracts.md)：当前主线的接口、类型、初始化与生成阶段。
- [其他通信机制](communication.md)、[其他生成能力](generator.md)、[其他执行与配置](execution.md)：非主线能力与进入条件。
- [测量、标定与生成开关](optional-generation.md)、[Implementation Plug-In](plugins.md)：按原文条件区分已关闭配置和未提供的工具能力。
- [SchM](../schm.md)：BSW Scheduler 生命周期和独占区。
- [BSW General：Rte](../bsw-general/rte.md)：按 Rte 单独核算通用义务。

运行 `python -m autosar_tooling requirements SWS_RTE` 现算处置；结果不是符合性证书。源码分析的真实诊断与 MISRA 人工评估由 #58 保留，不能用条款核算或 CI 绿色替代。

## 单实例句柄的实际用途

R2.8为Advisory、System（AMD4.3，印刷7／PDF14）：对象定义及其声明可删除而程序仍能编译时，属于未使用对象。当前四份实际ECU源码中，standard-ecu／user-application各两个、component-pipeline四个、no-diagnostic三个Rte_Inst_*指针对象只出现于生成头的extern声明和Rte.c定义，没有C消费者；保留其真实未使用状态。被这些指针初始化引用的私有Rte_Instance_* CDS不是同一个对象，不能仅因句柄无人读取便说CDS也无引用。

固定SWS_RTE的SWS_Rte_03711要求每个SWC实例的CDS，03712规定私有Rte_Instance_*前缀，03715要求const，03793规定单实例句柄的Rte_Inst_<cts>名字；它们支持保留公开句柄合同，但不能据此声称当前独立const指针对象是唯一合法表示。当前交付ABI保留这些对象，理由是组件头的一致句柄入口及既有集成合同；不为制造用途增加函数、宏或无效读取，也不把保留理由写成R2.8不适用、偏离批准或全部对象评估完成。其他对象及未选配置仍按实际引用、存储期和工具消费者分别判断。

## 组件实例类型的同名冲突

R5.6为Required／System（2013主文印刷54／PDF62，AMD2追加C11）。主文只允许同一头被多个源文件包含而重复声明同一typedef；三个不同组件头各自定义不同CDS指针，不符合这个允许条件。typedef与其关联tag同名的例外也不适用：这里的typedef为`Rte_Instance`，tag分别为`Rte_CDS_Ingress`、`Rte_CDS_Process`和`Rte_CDS_Observe`。AMD1–4／TC1–2未增加本构造所需的跨组件例外，保留真实违反。

固定RTE原件页696的`SWS_Rte_91176／01148／01150`规定应用头提供实例handle、底层为组件CDS指针、typedef使用固定`Rte_[Byps_]Instance`名字；当前bypass=NONE。页597的`01007`要求在应用头定义该类型，单实例时省略调用参数不能据此删掉类型。页656的`02311`还直接要求`typedef struct Rte_[Byps_]CDS_<cts> Rte_[Byps_]CDS_<cts>;`，因此不能把目前均为dummy布局的CDS合并成一个公共struct类型来消除重名。页659的`03724`规定空CDS的dummy成员，不取消逐组件类型。原页696的类型表与相关正文已核对，版本及摘要同本篇基线。

在`declaration-mapping-formatted-samples`的两个多组件工程中，实际位置及消费者如下；每行分别适用于component-pipeline和no-diagnostic，共六个物理定义位置，不按两个程序重复计数：

| 生成位置与底层类型 | 实际定义所在TU |
| --- | --- |
| `generated/include/Rte_Ingress.h:26`：`typedef const Rte_CDS_Ingress *Rte_Instance;` | `app/Ingress/Ingress.c` |
| `generated/include/Rte_Process.h:26`：`typedef const Rte_CDS_Process *Rte_Instance;` | `app/Process/Process.c` |
| `generated/include/Rte_Observe.h:26`：`typedef const Rte_CDS_Observe *Rte_Instance;` | `app/Observe/Observe.c` |

两程序的真实TU清单均包含这三条应用源，目标编译参数没有全局RTE_CORE；源文件直接包含各自应用头。RTE核心TU定义RTE_CORE而排除该typedef，不能据此排除应用TU中的定义。standard-ecu／user-application只有EchoApplication这一个应用TU，不把交付中未进入其他应用TU的头算成同一项目中的额外定义。生成来源是`contracts::contract_headers`，不是应用自行声明或SDK豁免。

给名字加组件后缀违反01150；改成宏别名不满足typedef要求；删除未被当前应用直接引用的类型违反01007；使用共同void指针或共同struct又失去01148／02311要求的组件CDS类型。把三个应用拆成不同可执行程序会改变本次单ECU集成合同。这些方案均不能在原所选合同内同时满足R5.6。风险是跨TU阅读时同名指向不同类型；独立应用头、C linkage、仅一个应用头的编译拒绝、CORE分支隔离，以及真实多组件的编译／链接和分发行为控制该风险，不能把这些控制说成R5.6已满足。

原项目授权为[2026-10-17决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)，本项按ADR0023逐处技术处置，不需要新增平台保留决定。评估日期为2026-10-27；该六处类型、必要性、替代和风险判断已在 `58fb105a` 累计独立复核中得到支持，后续 `58fb105a..8a26e3b0` 复核沿用未变结论。批准来源仍是原用户决定，实现者不是新增批准人；Required的技术处置与MISRA主文§5.4／§6.2.2的正式偏离及完整符合声明分别保留。

生成来源 `contracts::contract_headers` 复用实际application slot清单，仅当计划有多个应用TU且当前组件属于其中时，在其 `Rte_Instance` typedef紧邻契约中输出 `MR12 RULE 5.6 VIOLATION`，说明固定名称、不同CDS和独立头的类型隔离。对应仍为上述六处应用定义；不对单应用工程或DcmService的非应用TU添加违反注释，不改变类型、声明可见条件或ABI。SWS_BSW_00115的就近说明只标出技术依据，不是suppression；新注释与条件选择的自审、正常对照和最终累计复核按各自实际head记在PR，不冒充旧复核已检查新增实现。变更组件类型、Byps配置、头可见性、实例模型或TU组合时重新核查；R2.3的实际未使用状态另行保留。

## 共享 SchM 段的逐处责任

以下绑定 `8a26e3b0` 和四份 `complete-declarations-final-samples`，不含各模块周期原型／包装器，不是批准表。固定RTE页914–917的07250／07253规定Enter／Exit接口，07251按实际ExclusiveArea生成；页890的07596连接实体所关联MemorySection的SectionNamePrefix。当前BSWMD把三个真实host适配源登记为Rte实现，并选择 `RTE_Rte` 前缀；标准要求保留已配置前缀，不强制选这个字面拼写，也不把调用模块名字当作实现段owner。

| 文件与 R20.1 位置 | 实体与包含闭包 |
| --- | --- |
| `runtime/host/src/SchM_CanIf_Host.c`，`#include "Rte_MemMap.h"` 第2次，行30 | 关闭Can、CanIf、Det、Com、PduR、LSduR、CanTp七组Enter／Exit定义的 `RTE_Rte_CODE`；Can进入既有驱动锁，其余进入相应HostArea。 |
| `runtime/host/src/SchM_Diagnostic_Host.c`，同一指令第2次，行10 | 关闭Dcm一组Enter／Exit定义。 |
| `runtime/host/src/SchM_Mode_Host.c`，同一指令第2次，行15 | 关闭ComM、BswM两组Enter／Exit定义。以上各文件第一次包含在本文件定义前，不列为R20.1。 |
| `generated/include/SchM_CanIf.h`、`SchM_CanTp.h`、`SchM_Com.h`、`SchM_ComM.h`、`SchM_Det.h`、`SchM_LSduR.h`、`SchM_PduR.h`，同一指令第2次，均行30 | 分别关闭相应Enter／Exit原型的 `RTE_Rte_CODE`，与真实定义属性匹配。它们的第一次包含仅在预处理指令后。 |
| `generated/include/SchM_Can.h`，同一指令第1／2次，行75／95；`SchM_BswM.h`，第1／2次，行30／50 | 此前已有各模块周期原型，故共享Enter／Exit段的两端均为R20.1；前面的Can／BswM映射头仍归各自模块，不重复计入。 |
| `generated/include/SchM_Dcm.h`，同一指令第2次，行39 | 关闭Dcm共享Enter／Exit原型；随后Dcm_MainFunction及会话／认证服务段另行核对，不混算。 |

上述十五个物理位置在四profile中相同，不按TU／程序重复计数。MemoryMapping00003（页36）要求声明和定义分别完整映射；把这些头都提前一次包含不能保留后续STOP与原型属性。`Rte_MemMap.h`中 `RTE_Rte_START/STOP_SEC_CODE` 的消费连接00011；活动状态及属性撤销是实现清理，另判R20.5。D4.10连接可重入分派主体，普通SchM头仍有guard；一次性保护、移进函数或默认段不能同时保持00003／00023。真实头部属性继承、STOP清理、请求拒绝和现有并发／独占区消费者控制误配及锁边界风险，不扩张为任意嵌套或provider异步证明。最终累计复核与BSW00115对应仍须完成，不新增Approved／MR12；RTE核心、组件、服务和配置段仍各自承担逐处责任。
