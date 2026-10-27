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

## RTE 核心与存储段的具体请求

以下继续绑定 `8a26e3b0` 与同组四样本。方括号为同一文件中完整 `#include "Rte_MemMap.h"` 的物理出现序号，含条件分支，不是TU次数。表中区分实体的必要映射与尚可整改的inline配对；R20.1仍按预处理前布局判断，不因RTE_CORE或运行时未启动而消失。尚未列入本节的组件应用头、SWC实现及ECU配置请求另负逐处责任，不以本节代替全部RTE记录。

| 文件、请求与位置 | 具体实体与class |
| --- | --- |
| `generated/src/Rte.c`，[2]：标准／用户行60，组件62，无诊断54 | 关闭 `RTE_Rte_VAR_CLEARED_UNSPECIFIED` 的rte_started／rte_allocated。第一次包含在本文件对象前，不是R20.1；Start校验生命周期并拒绝重复启动，Stop只停止，不把allocated复位。 |
| 同文件，标准／用户[3]/[4]行62／68、[5]/[6]行70／74、[7]/[8]行76／82 | `RTE_EchoApplication` 的CONST包围静态CDS与公开const句柄，VAR_INIT包围接收值，VAR_CLEARED包围received／expired状态。实际Read／COM通知消费同一对象，不能用句柄未读就声称CDS没有被取址。 |
| 同文件，标准／用户[9]/[10]行84／89、[11]/[12]行91／113、[13]/[14]行115／158、[15]/[16]行160／180、[17]/[18]行182／213 | 依次为DcmService的CONST CDS／句柄、RTE_Rte的Start／Stop CODE、EchoApplication的Read／Write／状态接口CODE、DcmService的同步client CODE、OwnerPartition的COM回调CALLOUT_CODE。同步client按实际连接调用应用ReadData，通知按句柄更新接收状态，均与已配置前缀及对象一致。 |
| 同文件，组件与无诊断[3]/[4]、[5]/[6]、[7]/[8] | Ingress的CONST CDS／句柄、VAR_INIT接收值、VAR_CLEARED接收状态。组件行64／68、70／74、76／82；无诊断56／60、62／66、68／74。 |
| 同文件，组件与无诊断[9]/[10]、[11]/[12]、[13]/[14]、[15]/[16] | Process的CONST CDS／句柄与VAR_INIT接收值，以及Observe的CONST CDS／句柄与VAR_INIT接收值。组件行84／88、90／94、96／100、102／108；无诊断76／80、82／86、88／92、94／100。接收值来自各自ComSpec；同步连接和正常多组件分发消费这些独立对象。 |
| 同文件，仅组件[17]/[18]行110／115 | DcmService的CONST CDS／句柄；无诊断没有该组件对象，不另造对应位置。 |
| 同文件，组件[19]/[20]、[21]/[22]、[23]/[24]、[25]/[26]，行117／139、141／196、198／225、227／276 | 依次为RTE_Rte生命周期、Ingress、Process、Observe的CODE；无诊断对应[17]/[18]、[19]/[20]、[21]/[22]、[23]/[24]，行102／124、126／181、183／210、212／261。各应用API进入同一owner下的真实buffer／server路径，不把不同组件前缀合并成一个类型。 |
| 同文件，仅组件[27]/[28]行278／298；组件[29]/[30]行300／331、无诊断[25]/[26]行263／294 | 前者是DcmService同步client CODE；后者各为OwnerPartition的COM回调CALLOUT_CODE。无诊断没有前者，但仍有实际COM接收／超时通知。 |
| `generated/src/SchM_Main.c`，[2]行14、[3]/[4]行16／19、[5]/[6]行21／58 | 关闭schm_lifecycle的RTE_Rte cleared段，完整包围Ecu_SchMConfig的prebuild配置段及SchM_Init／Deinit的CODE。ConfigPtr由生命周期入口验证，不凭常量对象存在证明初始化成功。 |
| 同文件，[7]/[8]行61／67、[9]/[10]行69／104 | 会话／认证两个模式对象的VAR_CLEARED及实际Switch／Mode定义的CODE。前三profile使用RTE_DcmService，无诊断使用fallback前缀RTE_Dcm；来自同一个dcm_scope选择，声明与定义共用它。模式对象以SCHM_MODE保护，不能按模式接口名把这四处算入Dcm.c自身段。 |
| `generated/include/Rte_Main.h`，[1]/[2]行14／118、[3]/[4]行120／123 | 已有SchM_ConfigType后，分别包围RTE／SchM生命周期CODE原型与Ecu_SchMConfig的prebuild extern；两种声明均须保留自己的完整段，不能由定义侧替代。 |
| `generated/include/Rte_Os.h`，[1]/[2]行43／73；`generated/src/Rte_OsService.c`，[2]行15 | 冻结头的这两处仅包围始终static inline的client，两个普通provider原型反而位于段外，不能将其记为已满足的必要冲突。00003明确允许始终inline的代码不包装；source行15仍关闭实际provider定义的RTE_Rte_CODE，其第一次包含在定义前。provider调用真实GetCounterValue／GetElapsedValue，输出与失败语义仍按OS契约判断。 |
| `generated/include/Rte_Com.h`，[2]行49 | 关闭OwnerPartition的接收／超时通知CALLOUT_CODE原型，与Rte.c的真实callback定义相接；第一次包含之前只有指令／注释，不计R20.1。 |

本节Rte.c分别有17／17／29／25个R20.1位置，其余五个文件每profile各合计17个，不与共享SchM十五处或模块配置位置重算。来源是RTE存储／接口emitter、SchM生命周期／模式emitter及OS服务emitter；模板借用、渲染和直接交付身份仍按实际integration元数据区分。

把请求移到文件顶或只保留一个全局CODE段，会丢失所选scope、存储类别、原型继承或CALLOUT归属；一次性guard吞掉后续STOP，函数体内包含违反00023。相同scope／class且真正相邻的对象已经合并，不将可去除的重复配对当必要冲突。生成头部属性继承、真实句柄／对象段检查、正常多组件及生命周期／COM／诊断消费者分别提供静态和行为控制；R20.5关键词消费与内部清理、D4.10重入主体和最终BSW00115对应仍须按各自对象落实，不批量写成已批准或整体符合。

普通OS service原型的遗漏由 `os_service::files` 在原生成点修正：现有同一个CODE START提前到首个provider原型前，两个原型增加与定义一致的RTE_Rte_CODE；inline client、同一STOP及版本检查保留，不增加配对。首个include不再位于普通声明之后，不能继续列为无法整改的R20.1；上表行号与计数仍是8a26e3b0冻结证据，不冒充修复后分析。正常生成的两种Counter名字、真实原型属性继承及OS service成功／拒绝消费者分别验证该修改，原失败和新字节验证按实际收据保留。

## 应用头的实体与请求

以下绑定 `ac356523` 的四份 `mapped-os-service-final-samples`；后续OS同类段合并不改变这些头。序号按每个文件中所有映射头include合计，行号包括R5.6的就近说明。各对前一行分别为相应START／STOP；这些位置均在CDS类型声明之后，是真实R20.1，不因RTE_CORE或runnable选择而按布局销项。此处是技术依据，尚非批准表。

| `generated/include/` 下的头及profile | include序号与行；实体／class |
| --- | --- |
| `Rte_EchoApplication.h`，标准／用户 | [1]/[2] 29/33：`Rte_Inst_EchoApplication` extern的RTE_EchoApplication_CONST_UNSPECIFIED；[3]/[4] 45/80：Periodic、ReadData两个SWC原型的EchoApplication_CODE；[5]/[6] 86/105：RxValue Read；[7]/[8] 114/133：TxValue Write，后两对为RTE_EchoApplication_CODE。 |
| `Rte_DcmService.h`，三个诊断profile | [1]/[2] 29/32：`Rte_Inst_DcmService` extern的RTE_DcmService_CONST_UNSPECIFIED；[3]/[4] 41/61：DcmService_ReadData的DcmService_CODE；[5]/[6] 66/87：Rte_Call_DcmService_DataServices_ApplicationValue_ReadData的RTE_DcmService_CODE。无诊断没有该头。 |
| `Rte_Ingress.h`，组件／无诊断 | [1]/[2] 32/35：`Rte_Inst_Ingress` extern的RTE_Ingress_CONST_UNSPECIFIED；[3]/[4] 48/83：Periodic、ReadData两个Ingress_CODE原型；[5]/[6] 89/108：RxValue Read；[7]/[8] 117/136：TxValue Write；[9]/[10] 145/163：Value Write，后三对为RTE_Ingress_CODE。 |
| `Rte_Process.h`，组件／无诊断 | [1]/[2] 32/35：`Rte_Inst_Process` extern的RTE_Process_CONST_UNSPECIFIED；[3]/[4] 46/84：Periodic、Transform两个Process_CODE原型；[5]/[6] 90/110：Value Read；[7]/[8] 119/137：Result Write，后两对为RTE_Process_CODE。 |
| `Rte_Observe.h`，组件／无诊断 | [1]/[2] 32/35：`Rte_Inst_Observe` extern的RTE_Observe_CONST_UNSPECIFIED；[3]/[4] 45/62：Observe_Periodic的Observe_CODE；[5]/[6] 66/90：ResultService Transform client；[7]/[8] 100/120：Result Read；[9]/[10] 127/147：Value Read，后三对为RTE_Observe_CODE。 |

这里分别是14／14／34／28个物理include，不按程序或TU重复计数。`contract_headers`生成CDS与接口，同一生成器通过`api_visibility`按实际runnable选择生成原型；Rte.c提供同scope的实例句柄及RTE函数定义，应用实现提供SWC runnable定义。同步client沿生成连接进入Process_Transform或应用ReadData；COM通知／周期任务分别消费对应Read／Write与runnable，不把外部应用原型误归RTE实现CODE。固定RTE的03711／03712要求CDS实例及其内部名字，03793要求单实例公开句柄名；这些对象与R5.6的typedef是不同实体，不能因类型重名处置而免除其段责任。

MemoryMapping00003要求这些普通声明完整映射；RTE的07596将前缀连到已配置MemorySection，不强制采用本项目字面前缀。不同API段之间有runnable可见性条件及别名定义；把整个条件块移进一对会在段内引入其他预处理指令，而无条件暴露所有原型会改变应用头合同。提前一次包含、默认段或只映射定义均不能保持同一合同。实例CONST、SWC CODE与RTE CODE也不能相互替代。正常37头段继承与STOP清理、应用头唯一性／可见性和实际多组件消费者分别控制属性、声明与分发风险；本表没有把静态观察当作独立运行。关键词撤销连接00011，内部属性撤销仍按实现清理判断，D4.10归实际重入分派头；新增配置、runnable条件、实例模型或前缀时重新评估，最终累计复核及BSW00115对应仍保留。
