# RTE：当前主线接口与生成阶段

范围见 [RTE](README.md)。以下结论依据 SpecificationsARXML 正文及实际调用闭包，不以原文标题、现有 ABI 或生成器自己的元数据作为唯一预期。

## 输入、状态和执行

| 义务 | 处置 |
| --- | --- |
| 静态、可重复、多文件输入 | `build_plan_native` 按来源身份构建不可变计划，两个生成入口消费相同契约；输入字节不改写，无结果缓存。已核对（源码：plan、graph、contracts 与 prepared 的快照边界）：SWS_Rte_06026、02514、05000、08769、08770、08774、08775 |
| 当前通信与生命周期语义 | 初值、首次发布、0 值发布、扇出、未连接及 C/S 输出保护由 `rte_contract.c` 独立消费者验证；网络两接收者与超时由 `rte_com_contract.c` 及 `owner_probe.c` 验证；生命周期由 `schm_contract.c` 与启动失败测试验证，不能以 CAN 驱动写失败推导 COM_STOPPED。已核对（源码：rte::files 的 receiver/received/expired 与 COM 回调闭包）：SWS_Rte_06008、06009、06010、04501、04502、02515–02520、04527、03514、05021、08061、08062、08103、08104、07643、07384、08934、08592 |
| 同步直接 C/S | 参数按原顺序直接传递；无 possibleErrors 的 server 返回 void，client 返回 Std_ReturnType。单 owner、同一任务的执行闭包和 recursion 拒绝保证串行化；不为直接调用增加队列、锁或 busy 状态。已核对（源码：composition 的 callers、event 与 recursion 检查，rte 的直接调用）：SWS_Rte_06019、04515–04519、07008、07409、01017、01019、01020、06806、05108、01102、01293、01103、01104、02577、08594、01126、01132、01166、07023、07024、08913 |
| 显式非阻塞访问 | Read 使用 OUT 指针，Write 在返回前完成所有本地 receiver 的复制；网络与本地双重 producer 仍拒绝，纯本地复制没有独立的可失败服务。已核对（源码：rte 的 Read/Write 与 composition 的 endpoint 排他校验）：SWS_Rte_01091、01289、01093、01071、01280、02635、07820、01082、08413 |
| 启动与停止 | Rte_Start/Stop 始终声明并定义，只做有界条件判断。StartupHook 中先 SchM_Init/Start、后 Rte_Start、最后 SchM_StartTiming；ShutdownHook 中先 Rte_Stop，最后 SchM_Deinit。`rte_allocated` 防止同一进程重新分配；停止路径不能再次分派 runnable。已核对（源码：Rte.c、SchM_Main.c、Ecu_StackBootstrap/Retire 的真实调用闭包）：SWS_Rte_91137、91138、02569、01309、02585、01261、02570、01310、02584、01259、07574、07575、07580、07615、07616、02538、02535、02536、07586、04552、SWS_Rte_CONSTR_09035–09038 |
| 事件映射 | 直接 server 仍显式配置 EventToTaskMapping，但不带 TaskRef/Position；TimingEvent 的时间源、task、position 与 event 关系由 schedule 校验。同一 event 只能映射一次，同一 task 的 position 唯一；同 OS event bit 的实体具有相同周期与相位。已核对（源码：schedule::inspect 及 Ecu_StackRunCycle 的 source-derived 排序）：SWS_Rte_06798、06799、07843、04641、08939、07801、07806–07809、06728、06729、80076、SWS_Rte_CONSTR_08935–08938、SWS_Rte_CONSTR_09082、ECUC_Rte_09018、09019、09020、09021、09023、09024、09025、09026、09221 |
| 单实例源码组件 | 不支持多实例的输入必须显式声明 false；instance 参数从 API 和 runnable 签名省略。空 CDS 保留标准 dummy 成员，实例和 handle 为 const；兼容模式不要求为无 indirect API 的源码组件额外分配全部 PDS。已核对（源码：composition、contracts、Rte.c；PDF p551 的源码组件优化条件）：SWS_Rte_02008、02009、03806、01016、02310、02311、01007、01148、01150、06810、03724、03711、03712、03715、03793、03799、07138、06213、03723、08785 |
| 模型约束 | 每个 atomic SWC 必须有唯一 InternalBehavior；entry producer 与跨包组件名统一检查，PPort 的服务必须有 OperationInvokedEvent，调用点归属于实际 runnable。当前只接受同步 client、非并发 runnable、minimumStartInterval=0。已核对（源码：composition 的 runnable、operation、namespace 与 source object 检查）：SWS_Rte_07686、03527、07190、07191、07845、05083、08612、02733、03014 |

## 头文件、类型和名字

| 义务 | 处置 |
| --- | --- |
| 标准头关系 | Rte.h 导入 Std_Types/ComStack_Types；Rte_Type.h 和 Rte_Main.h 导入 Rte.h；每个 SWC 有 Application Types 与 DataHandle header。公开 C/C++ 入口使用 extern C，多应用头通过 RTE_APPLICATION_HEADER_FILE 拒绝，RTE_CORE 仅供内部生成源。已核对（源码：contracts::contract_headers 与 native C/C++ 消费者）：SWS_Rte_01164、08916、01412、01163、01159、08309、01158、07120、07122、07127、07678、07124、07125、07126、07131、07924、07920–07923、01006、01005、03709、03710 |
| 标准 API 和 runnable 名字 | 具体实现名为 Rte_Read/Write/Call_<cts>_<extension>，短别名映射签名不变；RTE_RUNNABLE_<shortName> 取 entry SYMBOL。RTE_RUNNABLEAPI 依据实际 consumers 限制短名和具体声明；初值宏有 uint32 的 UL 后缀。已核对（源码：composition 的 symbol producers 与 contracts::api_visibility）：SWS_Rte_03837、01274、03707、01153、02751、05078、03619、06713 |
| 类型来自完整输入 | 所有接纳且 typeEmitter 为空或 RTE 的 ImplementationDataType 都进入 Rte_Type，包括未使用的类型；同一 symbol 的不同声明拒绝。VALUE 的标准 native 声明、位宽、encoding 一致；固定数组保留 leaf ImplementationDataType 名称。外部 emitter 的 Dcm 类型由 Rte_Dcm_Type 唯一生产。已核对（源码：rte_types::inspect、contracts 与类型改名/unused/冲突测试）：SWS_Rte_01160、02648、06709、06710、07104、07105、07111、07113、06719、06724 |
| runnable 参数名字 | 没有 RunnableEntityArgument 且没有 BSW mapping 的源码 SWC，role prototype 参数不命名；定义中的局部参数仍命名。已核对（源码：contracts 的 application runnable 声明）：SWS_Rte_06703、06705 |
| 共享状态 | 全部标准 RTE/SchM 错误值与三个位域宏由 rte_types 唯一生产，SchM_Dcm 不重复定义。已核对（源码：Rte.h、rte_types::STATUSES 与独立状态向量）：SWS_Rte_01269、07712、07713、07404、07405、07406、07291、07289、07290、07562、07563、02747、07054 |
| RTE 导入与版本检查 | Rte.c 显式包含 Os.h、Com.h、Rte.h、Rte_Com.h；检查已导入标准模块的 major/minor release；版本失配编译反例逐头替换实际值。已核对（源码与 native 编译反例）：SWS_Rte_01257、03794、01279、07692、01264、03795、03796、08914、08915、01165、03007 |

## 内存与生成描述

SWC 的入口原型和应用种子使用 `<c>_MemMap.h`／`<c>_START_SEC_CODE`；RTE 自己的 buffer/API 使用 `RTE_<c>`。两者所有权不同。RTE 与生成配置共用的 `RTE_<module>` 前缀统一由 `Rte_MemMap.h` 实现，避免 Dcm 前缀被两份头映射。未知段、嵌套和不匹配的 STOP 由编译反例拒绝；没有无段 include 的虚假兜底类型。

已核对（源码：memmap 的共同 section 表、contracts、rte、ecu_config 与 BSWMD）：SWS_Rte_07194、05088、07422、07424、07426、03788、05089、05090、05192。

接收值按 `AT_DATA_DECLARATION` 在 INIT 段初始化；CLEARED 状态取 NONE，不在 Start 再重写配置初值。必填 RteGeneration、RteBswGeneral、RteInitializationBehavior 由官方 ECUC metadata 表达，目标只接受当前策略。OsTask 必须引用所选 `/Configuration/CODE` SwAddrMethod，生成任务体用 OS_START/STOP_SEC_CODE 包裹；尚未接纳的其他 SwAddrMethod 不被静默丢弃。

已核对（源码：builtin、configuration、native_configuration 与任务模板）：SWS_Rte_07075、08923、04557、ECUC_Rte_09000、09009、09061、09087、09088、09089、09086。

生成 BSWMD 描述每个实际变量与常量，MemorySections 不代替 staticMemory/constantMemory。MC-Support 在 measurement/calibration 禁用时仍输出 emulation NONE。逐 API 的 dependency 使用固定 R24-11 XSD 仍声明的 requiredEntry/expectedCallback 角色；原文 05179/05180 的 implementedEntry 是 BswModuleEntity 的角色，不能把它发明成 BswModuleDependency 的 XML 字段。该角色映射是根据正文要求的调用方向与 schema 的推断；保留 atp.Status=removed 的模型差异，不声明解决了 AUTOSAR 元模型文字冲突。

已核对（源码：artifacts、rte_memory；输入版本和逐对象/逐 API 测试）：SWS_Rte_05086、05165、08404、05166、05167、05177、05181、05187–05190、07085、05182、05183、05087、05118、05137、05138、05179、05180、05184、05185、05186。

RTE 配置版本从 module ADMIN-DATA 的 DOC-REVISION 取值，多个 revision 按 DATE 的实际时刻选择最新；相同时刻歧义、无效日期或无可解析标签时回到 0.0.0。第四段仅作文档 suffix，不参与数值版本。生成工具自身身份仍由源码摘要记录，二者不混用。

## 已选关闭项和保留的标准边界

SchM_GetVersionInfo 只在 RteSchMVersionInfoApi=true 时存在；当前目标在参数处拒绝 true。其余版本信息通用差距仍见 #59。

不适用（配置：ECUC_Rte_09062=true）：SWS_Rte_91174、07278、07279、07280、07281。

Rte_Init/StartTiming 只在配置 InitializationRunnableBatch 时生成；当前主线没有初始化 batch，不能增加两个无输入的空壳接口。未来开放此配置与其他执行机制见 [其他执行与配置](execution.md)。

标准要求的 nameless runnable role prototype、固定状态/初值/runnable 宏、const CDS/handle 和 MemMap 重复包含会产生 MISRA 8.2、2.3/2.5、5.6、8.7、20.1/20.5 等诊断。保留必需契约与原始诊断；这不是偏离已批准。全部源码诊断裁决与 221 项必要人工评估仍归 #58。

### 无名 role 参数的逐处核查

所选四个主线样本中，`EchoApplication_ReadData`（standard-ecu／user-application）、`Ingress_ReadData`和`Process_Transform`（component-pipeline／no-diagnostic）的application runnable没有`RunnableEntityArgument`或BSW mapping。固定R24-11 SWS_RTE（SHA-256 `d9b95dfa8ae5c94418382835b7f5da9fd53e0c76c07b5d14f139c8a58dd39769`）§5.7.3页806–807的`SWS_Rte_06703`／`06705`分别规定有symbol时命名、两种来源都没有时生成无名参数。`contracts::contract_headers`据此在这些声明中保留类型而省略名字；诊断样本的ReadData经RTE client／Dcm调用应用实现，Transform经同步client调用应用server；no-diagnostic只保留Ingress的应用声明，不存在Dcm调用闭包，不能借诊断样本证明其执行。

MISRA 2013主文R8.2（Required，页64–66／PDF72–74）仍要求声明参数命名；TC2.10只增加8.3交叉引用，TC2.12对8.3的无名参数例外不豁免8.2，AMD2只扩充语言适用性，AMD3.63只增加交叉引用。这里是真实冲突，不能标成误报；增加声明参数名会违反06705，删除声明会失去公开契约，改变为无参数则破坏C/S签名。风险是省略名字降低接口可读性；按顺序记录方向／存储义务、固定参数类型并由真实应用TU编译／链接验证约束它。`source_derived_composition_contract_is_deterministic_and_keeps_local_identity`同时断言单一数组参数与IN／OUT／INOUT三参数原型，补名反例应失败。

此结论适用ADR0023已有授权范围；逐项独立复核和实际源码注释完成前，仍不登记为已批准偏离。全部221项人工评估、其他无名参数和其他配置分别核查。


已核对（源码：每个组件独立 Application Header、标准 typedef 与状态返回，configuration/schedule 的唯一来源）：SWS_Rte_01004、01012、01034、01146、01157、01161–01162、01167、01169、01197、01260、01262、01266、01314、01316、01318、01368–01370、02200–02202、02204、02207、07284、07288、07500–07501、07533–07535、07593–07596、08600–08601。

已核对（生成样本：Rte_Cfg.h 通过 Rte.h 被实际消费者包含，配置头导入 Std_Types.h）：SWS_Rte_01321、07641。

已核对（源码：配置记录中的 SWC/BSW 实例与外部引用、显式事件映射和 position）：ECUC_Rte_09002、09004、09005、09065、09066、09067、09068、09069、09070、09071。

已核对（所选 generation 参数明确关闭 measurement、calibration、emulation，输出 MC-Support NONE）：SWS_Rte_03903、03971、03942。
启用能力与独立开启 measurement/calibration 的工具模式仍未支持，见其他生成能力。

差距 #58：SWS_Rte_01168。

已核对（源码：Rte.h 的固定标准状态值与 bit accessor、直接 C/S 的 application error 转发、const instance 类型）：SWS_Rte_02593、02573、07143、01058、02594、04582、01060、01064、01317、01061、01065、02739、02757、08065、08725、08726、08551、01389、02571、02702、02312、91176。

已核对（源码：Application Header 标准文件名、compatibility-only 参数和实际 Read/Write/Call 的消费者过滤；CODE 任务体由 RTE 生成链的 Ecu_Config.c 承担）：SWS_Rte_01003、01151、07140、01277、06200、06201、02254、08417、07824、07826、08414、08321、SWS_Rte_CONSTR_09015、SWS_Rte_CONSTR_09021、SWS_Rte_CONSTR_09024。

已核对（定义与所选值：RteGeneration 的官方必填参数、RteBswGeneral 的版本开关均在 normal validation 与目标校验消费）：ECUC_Rte_09007、09008、09010、09011、09012、09014、09015、09062、09085、09113、09126、09163。

模块互联头包含对应 `SchM_<module>_Type.h`，Types Header 导入 Rte_Type.h；实际模块头与新增认证模式声明均有 C++ linkage。未使用的类型头不分配对象。RTE 生成描述同时列出实际的 SchM 头和集成配置依赖。

已核对（源码：ecu_config::schm_header、静态模板与生成 Types Header；native 头编译）：SWS_Rte_07503、07295、07500、07501、07502、07504–07510、07297–07299。

已核对（源码：Rte.c 的标准 Rte_ 外部 API 名字、SchM 的专门前缀与输入类型例外；COM user 的 handle-bearing 通知）：SWS_Rte_01171、04616、04620、91123、91127。

已核对（源码：composition 的全输入名字与类型拒绝，rte_configuration 的严格输入边界且不改写源）：SWS_Rte_01000、03893、04632、05149–05150、06711–06712、06715–06716、07642。

已核对（源码：受信 Dcm typeEmitter 共享 producer，其他外部头和前缀保留类型明确拒绝）：SWS_Rte_CONSTR_09097。

已核对（源码：TimingEvent、直接 server 与 BSW 实体共用 schedule 的排序和完整映射检查）：SWS_Rte_01131、02203、04542、05115、07282、07514–07518。

已核对（源码：BSW 映射的引用与 task 布尔值均由 schedule 消费）：ECUC_Rte_09064、09222。

已核对（源码：真实 COM 回调、符号名和 primitive 网络端点；无 transformer 的当前分支）：SWS_Rte_02703、03530、03759、04504–04505、06023。

已核对（源码：各模块独占区及其 Enter/Exit，SchM_Main 的模式区在 Rte BSWMD 独立声明）：SWS_Rte_07522–07523。

已核对（源码：receiver 的初值在声明时分配，未配置 SwAddrMethod 的当前形状）：SWS_Rte_07046。

已核对（源码：compatibility-only 接纳值，契约与 ECU 的相同 API producer）：SWS_Rte_01195、01234、05053。

已核对（源码：Application Header 的 C linkage 与 CDS；类型头不分配对象）：SWS_Rte_01011、03786、07121、07421。

已核对（源码：partition CALLOUT_CODE 包裹实际 COM callbacks；其他 callback 未接纳）：SWS_Rte_04615、07427、07710。

已核对（源码：共享 section 表的 segment/alignment 关键词与实际 CODE 原型 MemMap）：SWS_Rte_07195、07830、08787。

已核对（源码：十个已交付 BSW 源文件直接包含本模块 SchM 互联头）：SWS_Rte_CONSTR_09059。

已核对（源码：immediate Dcm bridge 的 Switch、Mode 签名、状态和 BSWMD groups）：SWS_Rte_07255–07256、07258、07260–07262。

已核对（源码：Dcm bridge 的 managed/accessed groups 与真实使用者）：SWS_Rte_CONSTR_09049–09050。

已核对（生成样本：Rte_DataHandleType.h 直接包含 Rte_Type.h；独立 C/C++ 类型消费者）：SWS_Rte_80009。

已核对（所选值：VFB trace=false 是无跟踪模式；bypass=NONE 不生成 Byps_ infix）：SWS_Rte_01327、07835、07841。

已核对（生成样本：MC-Support 只含自足的 NONE，且没有第二种 emulation 方法）：SWS_Rte_03909、05119。

已核对（所选值：严格初值/配置模式与缺席默认严格路径；非严格补全能力仍归 #212）：SWS_Rte_05148、07680。

Rte_Instance 的标准短名在不同组件的独立 Application Header 中重复定义，会触发跨翻译单元的 5.6 唯一名字诊断；它们不会在同一应用翻译单元混用。该名字由接口契约要求，不能通过随意改名消除诊断。

已核对（源码：RTE-owned Rte.c/Rte_*.h 的文件名；Ecu_* 配置与目标胶水是集成依赖而非 RTE 模块文件）：SWS_Rte_07139。

CDS 本体按 SWS_Rte_03711/03712/03715 保留私有 const 存储，不在应用头导出存储对象；应用通过 SWS_Rte_03793 的公开 Rte_Inst_<component> 访问。原生符号表与拒绝编译消费者核对该链接边界，避免把非必需的外部对象归为 8.7 标准冲突。
