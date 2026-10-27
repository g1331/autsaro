# MemMap：MISRA 预处理布局

### 声明与定义使用同一类别

`SWS_MemMap_00003`（固定原件页36）同时约束代码、变量和常量的声明与定义；定义已有属性、当前链接器恰好放入默认段，均不能替代声明侧的完整 START／include／声明／STOP／include。九个 BSW 模块的公共接口及回调专用头，连同 Can、CanTp、Com、ComM 的私有实现头和 Det_Host 的实现原型，使用各自模块的 CODE 对。类型定义及函数指针 typedef 自身不分配代码或对象，不附加段属性。已选CDD的Ecu_HostBusSM七个正常接口同样使用其实现的ECU_HOSTBUSSM_CODE；不因host目录给其他无BSW角色的helper套用这一义务。新增映射依赖的 AUTOSAR major/minor 检查置于完整映射对之后。

生成组件头中，RTE 接口原型使用 `RTE_<component>_CODE`，SWC runnable 原型继续使用 `<component>_CODE`；实例句柄的 extern 声明使用与 Rte.c 定义相同的 `RTE_<component>_CONST_UNSPECIFIED`。Rte_Main 的生命周期接口和配置对象分别对应既有 RTE CODE 与配置类别，COM 回调声明则连接实际分区的 CALLOUT 类别。独立 contract 包交付这些声明需要的 Rte_MemMap.h，不借 ECU 工程外部头补齐。Ecu_TargetConfig的四个正常生命周期／帧形状声明对应RTE_Ecu_CODE，测试provider留在该对之外；RuntimeConfig的BswM动作和Dcm模式通知私有前置声明分别对应RTE_Ecu_CODE／RTE_Dcm_CODE，不以定义已映射代替声明侧协议。配置条件、API 可见性、C++ linkage 和版本检查均在完整映射对之外。

CanIf／LSduR／PduR 配置头的 extern 对象与各自 PBcfg 定义共用 RTE 配置类别。SchM Enter／Exit 的声明和三个 SchM host 适配文件中的真实定义使用 `RTE_Rte_CODE`；配置生成的 Com／ComM 主函数包装器则使用与 Ecu_RuntimeConfig.c 定义一致的 `RTE_Com_CODE`／`RTE_ComM_CODE`，不能因公开名称或调用模块而误配到模块实现 CODE。Dcm 模式接口的作用域来自实际 diagnostic-session 组件，声明与定义使用同一选定模式类别。此归属不扩到共用锁 provider 的私有状态；RTE 实现描述同时收录真实 SchM 适配源。

00003不要求相邻、同类对象各自重新开闭。`config_data`只在输出恰好结束于相同scope的配置STOP时合并下一对象；不同scope或任何中间内容均建立新对，不跨条件或版本检查移动对象。CanIf的can_rx、can_tx及CanIf_Config等连续配置可因此减少多余include；对象顺序、初始化与属性保持。可合并的边界不作为必要冲突保留，逐处计数绑定合并后的交付源码。

同样的原则适用于自有OS源码：`7e1b7c9e` 的注释感知token核对确认八文件有九个同prefix／class边界，中间只有空白或说明。Os_Time的两个事件、Os_Mailbox的发布／接纳原子、Os_Backend的关闭／接纳原子分别合并原cleared类；Os.c、Os_Interrupt.c、Os_Backend.h两处、Os_Stack.h及Linux等待helper合并原CODE类。仅删除九组中间STOP/include/START/include，保留全部注释、非指令token、对象与函数顺序及外层条件；属性宏在原本只有注释的间隔继续有效，没有新增消费者。不同class、条件或实际声明隔开的段不合并。

完整位置与调查见[OS相邻段](https://github.com/g1331/autsaro/pull/371#issuecomment-6430247976)。现有memory-mapping正常入口的六项检查通过，包含真实链接的十个入口及十个编译／链接拒绝案例；源码／class／owned-header检查沿用原入口，没有新增解析器或长期编排。此删除不是新偏离；旧物理行号和诊断仍绑定旧字节，新计数须取自实际重新生成／分析的工件。

正常消费者以独立配置的 CODE 段编译实际 BSW 头，并检查每个函数定义从先前原型继承的目标文件段；RTE 消费者同时观察真实实例句柄及 Read／Write 接口的段，保留独立签名和链接回归。这里保留新增 include 产生的真实 R20.1 与映射头重入的 D4.10 责任，尚不据此增加批准表或 MR12；既有分析和逐处计数仍绑定其原始源码身份。

### 所选 CDD 的适用来源

`Ecu_HostBusSM` 不是因目录名、资产的 host 标签或函数前缀而取得 BSW 必要性。`mode.rs` 只接受 `COMM_BUS_TYPE_CDD`，`ecu_config.rs` 将实际 Request／GetCurrentComMode 函数地址交给 ComM；该模块经 CanIf 取得控制器／PDU 模式并向 ComM 发布变化。固定 COMManager 原件的 `SWS_ComM_00957／00958／00963`（页146）与 `ECUC_ComM_00888`（页191） 分别规定所选 CDD 的前缀、请求及查询接缝，没有要求项目必须选择 CDD。

固定 BSWGeneral §1.1（页9）规定无另列条件的条目适用于所有 BSW 模块；`SWS_BSW_00006`（页22）约束其实现源包含模块映射头，`00249`（页28）的普通头保护义务显式排除映射头。CDD 指南 §6.2／6.3.1（页13）要求实现遵守引用的 BSW 通用规格，同时使用“may”描述采用映射机制；不能把后一句独立改写成强制选择 CDD 或强制采用本项目属性策略的理由。这里的必要性连接到已经选定、实际作为 ComM CDD BusSM 集成的 BSW 实现及通用规格，不扩展到任意 host helper。`Ecu_HostBusSM.c` 的 cleared 数据及 CODE 两对请求，与同名映射头的关键词消费和重入主体对应；其余位置仍须逐处核对。

所用 CDD 指南／COMManager 的 SHA-256 分别为 `8d8318fc…bc8b8f883`／`564560db…cf2ef34`，均与 R24-11 CP 官方清单相符。BSW 的 `00249` 例外不是 MISRA D4.10 例外，也不替代 ADR0023 所需的独立复核或具体记录；当前没有据此添加批准表或 MR12。

### 非默认映射头的输入责任

`00028／00032`（页13–14）区分显式 MEMMAP 依赖与模块默认头名；`00033／00034／00035`（页38–39）进一步把实现的 MemorySection 连到默认头、单个依赖或具体 SectionNamePrefix 的 implementedIn。普通 SWHDR 描述、源码包含和指向另一实现的 generated artifact，不能替代 MEMMAP 类 requiredArtifact。固定 XSD 将 requiredArtifact 放在 Implementation，DependencyOnArtifact 的 category 来自 Identifiable；artifactDescriptor 自身的类别仍为 SWHDR。

实际 ARTI 原生绑定与 Os 共享 `Os_MemMap.h`，不是默认的 `Arti_MemMap.h`；生成描述现以本实现的 MEMMAP requiredArtifact 明确该依赖，并把 OS 资源前缀连接到它。LSduR／PduR 的指针存储和路由函数分别使用自身的 cleared／CODE 段，RTE 生成的路由配置另使用 `Rte_MemMap.h`。两份实现描述分别列出两个依赖和对应前缀，补齐此前只描述配置段的遗漏；不能把 RTE 产生配置对象的事实扩为模块全部实现段的 owner。此处修正描述与实际包含的联系，不改变 C 源码、属性宏或 ABI，也不据此完成逐处 MISRA 批准。

`00045`（页41）另禁止包含其他 BSW／SWC 的段；不能只证明文件名存在就略去这项约束。`00028`后的说明（页13）明确支持多个 BSW 描述贡献到同一文件，因此这里将“其他”按配置关联的贡献者之外理解：ARTI／Os 明确共享 OS 前缀，RTE 头中的 RTE_<scope> 段由 RTE 实现描述；各 SWC 的应用段仍留在各自头中。此判断须在累计复核中连同具体 requiredArtifact、前缀与实际实体核对，不作为把任意模块放进共享头的普遍许可。

### 规则与冲突条件

MISRA 主文 R20.1（Advisory，印刷页155／PDF163）按预处理前的文件顺序限制 include 之前的内容；R20.5（Advisory，页158／PDF166）限制 undef。AMD2 为两条增加 C11 适用性，不改变本实现 C99 下的上述义务；其他适用 amendment／TC 没有提供 MemMap 例外。

D4.10为Required（主文印刷32–33／PDF40–41，AMD2追加C11）；TC2.2明确示例的两种保护形式并非穷尽，没有增加MemMap例外。普通声明头仍保护其内容不重复展开，而实际MemMap分派头必须依不同START／STOP请求重入；一次性guard会吞掉后续关闭及重新开启。Os_MemMap.h只对固定属性定义做一次性保护，映射分派主体没有也不能借此声称满足D4.10。该真实冲突应连到实际调用方的重复include及对应头，而非伪装为某个函数违反；非法、嵌套和错配拒绝控制重入风险，不将这些控制当作不存在违反的证明。

固定 R24-11 的 `SWS_MemMap_00003`（页36）要求 START 定义、映射头包含、声明／定义、STOP 定义、映射头包含依次出现，并禁止序列中其他预处理代码；`SWS_MemMap_00011`（页41）要求映射头撤销 START／STOP 关键词。因此，代码之后的必要映射头包含及关键词 undef 是真实的 R20.1／R20.5 冲突。把所有映射头移到文件顶部会破坏段的开启／关闭；保留关键词则使下一次包含无法独立识别请求。映射头的非法／嵌套／错配拒绝和实际代码／存储段检查分别控制误配风险；R20.1／R20.5 按[类别记录](README.md#memmap-类别偏离)集中处置，后续源码注释由 #381／#382 承接，不把 AUTOSAR 原文对其 MISRA 例外的说明当作 MISRA 批准。

普通业务头及私有常量定义不受上述冲突理由保护。CanTp.c的19个帧格式／错误／定时常量和Dcm.c的19个缓冲／服务／状态常量均在START之前定义；原先处于VAR序列内不符合00003，不能作为必要冲突保留。移动不改变任何名字、值、声明／定义的段类；真实GCC13.3 C99预处理token逐一相等，原CanTp全双工及Dcm会话／响应失败消费者仍通过。现有映射校验器检查两模块实际指令流，将各一个真实定义移回段内的反例必须以明确preprocessing原因拒绝。`Os.h` 的 Hooks 头、生成通信配置的诊断路由头，以及 SchM 的模式／主机独占区头在实际声明前导入；对应 release 检查保留。SchM 的五个生命周期常量也在 START 之前定义，段内只保留状态声明。正常消费者仍使用同一类型、数值和 MemMap 边界；`generated_lifecycle_sections_contain_no_other_preprocessing` 依据 00003 检查实际生成的生命周期变量块。

Can.c原有43条段内条件指令分别控制queued token及Rx／Tx／bus-off轮询，不能借所选分支或主机角色豁免00003。条件现在包围完整的变量／函数映射对；三个token对象的顺序、类型及静态存储保持。可选动作通过文件内私有函数选定，原持区调用与解锁后通知保持；载荷复制共用原体，序号耗尽仍先于推进和复制。没有新增锁、共享状态、运行时配置或导出入口。未选动作的void调用显式丢弃，按R2.2的void-cast例外核对（主文印刷41／PDF49），不把空实现声称为有效副作用；测试所核轮询与即时路径的边界分别保留。

Det_Host的Windows／Linux完整Halt定义、Os_PortRestart的Linux声明，各由外层条件包围自己的完整映射对。Ecu_Target的测试／生产ShutdownHook也分别选择完整定义，退休、生命周期发布和测试观察的顺序不变。生成RuntimeConfig从同一结构化生命周期来源输出测试与生产定义，条件位于映射对外；测试版仍在每个阶段开始前注入失败并在退休前观察，生产版没有额外测试分支。`generated_lifecycle_sections_contain_no_other_preprocessing`同时检查SchM状态块及RuntimeConfig的全部Ecu CODE块；现有Python指令校验器检查上述四个静态文件，并拒绝把真实条件恢复到段内的副本。原始有错源码也须被同一检查拒绝，不能只验证新布局自洽。

上述结构判断不代替行为证据：实际Can／CanIf回调交错、queued诊断取消、COM下层链、Det终止，以及封存启动失败／逆序退休消费者分别验证保留的合同。启动阶段测试同时拒绝stderr，防止预期退出7掩盖检测器报告；普通、ASan／UBSan和TSan按各自实际运行记账。移到外层的条件仍按其真实配置选择，不将未选定义称为已运行，也不由这些检查关闭完整provider异步义务或逐处MR12责任。

同类连续对象也不要求在仅有空白的间隙关闭再开启：Os_Time、Os_Backend、Os_HostServices、Os.h及Ecu_Target中的这类块共用外层配对，保持声明顺序及每个C token的映射类；不跨不同类别、条件或其他预处理合并。只对余下实际必要的后置包含作冲突判断，不能把可删除的内部配对也写成AUTOSAR强制行。

采用内核的可选测试头、MSVC 专属分支、MPU 标记，以及 Windows 名称导入的 undef，分别有自己的条件和消费者，不属于本节的必要映射关键词判断；原始报告及其后续人工评估保留。

逐处评估还须区分00011直接要求的模块START／STOP关键词，与当前属性实现为清除临时OS_MEMMAP／active状态而使用的内部undef。后者不是该SWS直接规定的名字；不能把选定GCC属性策略写成AUTOSAR强制实现。实际替代、接口影响和Advisory保留理由应分别核查，不给全部undef统一授权。当前仅补齐这些规范边界，现按 [类别记录](README.md#memmap-类别偏离)处置 R20.1／R20.5；其他规则仍按 ADR 0023。

