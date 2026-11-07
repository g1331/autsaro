# MemMap

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_MemoryMapping.pdf`（`9dc38d360e4a154f627ede71cc5207f7be193c55cbb9f87875e2731af14c891c`）。

| 条款 | 义务要点 |
| --- | --- |
| `00020／00028／00029／00032`（`[SWS_MemMap_00032]`／`[SWS_MemMap_00029]`） | 每个实际模块与每个 SWC type 的 MemMap 由 Memory Mapping 按输入配置提供；主机段映射不代表 MCU 段放置 |

## 已知差距

- **MemMap 与段放置**：主机 PE／ELF 段映射已有实际证据，MCU 段与权限只由目标工具链与布局关闭。

## 映射请求的拒绝边界

`SWS_MemMap_00007`（固定原件页41）要求映射头以预处理检查拒绝无效关键词及嵌套／错配序列；`00011`要求消费START／STOP关键词。九个BSW映射头及Ecu_HostBusSM原先在缺少请求时进入头文件smoke声明分支，不能据编译成功证明请求合法。现改为直接拒绝；去掉仅供该分支使用的Std_Types导入和虚假声明，真实模块对外部头的版本检查继续保留。

这些头与生成的RTE／SWC映射头在分派并消费一个支持的请求后，检查是否还残留另一个支持的请求；残留即编译失败，不能在同一次包含中默默选择第一个分支。此处检查同一映射头支持的关键词，不声称能识别任意拼错的宏名。原有active状态、嵌套／错配检查及属性保持。Can与Os原有的单请求检查已经成立，保留为回归对照；Os的40关键词检查未重复实现。

现有原生应用头入口对十四组交付映射头分别以C／C++编译实际CODE及VAR配对、缺少请求和同时START／STOP三种输入，并检查请求及临时属性在STOP后消失。删除缺请求或多请求拒绝的测试副本必须先被真实编译器接受，再由对应场景断言拒绝；编译准备失败不算变异证据。这些检查不替代最终段布局、其他配置、MCU放置或逐处MISRA冲突处置。

## MISRA 预处理布局

声明／定义归属、CDD、映射头输入和原文修订见[布局说明](layout.md)。

## MemMap 类别偏离

依 [ADR 0025](../../adr/0025-misra-layered-scope.md)及[项目决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6437338057)，R20.1（Advisory，include 的位置）／R20.5（Advisory，undef）作为一项 AUTOSAR MemMap 类别偏离记录，不作逐包含必要性报告。

范围是 runtime 自有 `.c`／`.h` 和四 profile 的生成 C／头中的 `<Mip>_START_SEC_<class>`、同一映射头 include、内容、匹配 STOP 和 include；对应可重入 `<Mip>_MemMap.h` 内消费请求及清除本次属性／active 状态的 undef 属同一协议。来源包括 BSW、OS、RTE、SWC 和已选 CDD，覆盖受控 Linux／Windows 的 HostBatch／probe 及复用实现的原生消费者；不含无关 undef、普通头或任意预处理布局。

理由是 SWS_BSW_00006 的模块包含与 SWS_MemMap_00003／00011 的完整映射及请求消费机制；内部属性宏是当前实现选择，不声称其拼写由 SWS 强制。规则修订、替代方案与声明归属沿[布局说明](layout.md#规则与冲突条件)的既有证据。

风险是错段、漏 STOP 导致后续声明继承属性、嵌套或条件截断导致配置相关布局。控制是现有 memory-mapping 检查器共用的 `c_standard.verify_memory_mapping_pattern`：逐 token 检查每个字面 MemMap 包含均紧接请求、START／STOP prefix 与 class 相同、两次 header 相同、区内无其他预处理、最终闭合；包含注释、续行和未激活分支。`quality` 每次检查 runtime，`quality --c-samples <四样本目录>` 同时覆盖实际生成 `.c`／`.h`（包括 runtime 拷贝与 app，kernel 仍属采用代码）。OS 原有对象 class、声明归属、编译／链接和段验证保持。正例与缺 STOP／最终 include、区内条件、嵌套、错 class／header 的变异由 `test_c_standard.py` 运行。

映射协议、producer、段类别、目标工具链／链接布局或来源责任变化时重评；新增位置必须通过模式检查，模式外错误照常修复。既有原始诊断照常可见，实际源码注释由后续自有／生成代码整改沿 C 编码规范添加。本记录不覆盖 D4.10 或其他规则。

## D4.10 可重入头的正式记录

本记录按 AMD2.4／AMD2.5 修订后的 §6.2.2 及 MISRA Compliance:2020 第 4 节接续既有逐处技术评估。基线为 `c3bff230`，语言 C99，目标为 `linux-x64-controlled-v1`／`windows-x64-controlled-v1`，配置为正常四 profile 的 HostBatch／probe。指定技术权威的同意沿 [2026-10-17 的决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)及 ADR 0023 的条件授权；项目实现者于 2026-11-07 逐项评估。原文及修改后源码闭包的独立复核、实际平台验收共同确定本次已闭合实例，精确 head、完成状态、发现与处置集中在实施 PR。

**规则与允许条件。** D4.10 为 Required，主文印刷 32–33／PDF 40–41 要求防止头内容重复纳入。TC2.2（印刷 2／PDF 8）放宽的是保护形式的示例范围；AMD2 增加 C11 适用性，其余 AMD1–4／TC1–2 未改变此 C99 义务。首个文件注释中的 MR12 DIR 4.10 只说明可重入机制及控制。本项保持真实违反，采用可逐实例识别的偏离记录。

AMD2.4 撤换主文旧第 5 节，AMD2.5 将 Required 偏离程序引用改为 MISRA Compliance:2020 第 4 节（均为 AMD2 印刷 4／PDF 11，当前 C99 同样适用）。现行 §4.1–4.4（印刷 12–14／PDF 20–22）要求明确的程序、可识别实例、理由、语言背景、风险与预防要求，并取得指定技术权威同意；通用 permit 与具体 record 分开。本项按 §4.4 Reason 1 论证功能正确性及所选映射合同的兼容性，以原文必要性、替代比较、协议和实际段检查维持安全约束；对比吞掉 STOP 的整体 guard 保持更好的总体质量。旧 §5.4 仅作历史定位。

**位置与来源。** 十二个固定头为 BSW 的 Can、CanIf、CanTp、Com、ComM、Dcm、Det、LSduR、PduR、BswM，以及 Os、Ecu_HostBusSM 的映射头；其原 `runtime/` 路径、`@file` 记录、包含者和 MR12 已在各 [BSW 通用模块篇](../bsw-general/README.md)逐项登记。本项复用这些位置与注释。固定头经 `outputs::template_of` 登记到 MEMMAP producer 后以原字节进入 `generated/include/`。

生成位置另限 `generated/include/Rte_MemMap.h`、`EchoApplication_MemMap.h`、`DcmService_MemMap.h`、`Ingress_MemMap.h`、`Process_MemMap.h`、`Observe_MemMap.h` 的首个 `@file`。RTE 头由 `core/src/integration/memmap.rs::rte` 生成，其余由 `component` 生成；独立 contract 包的具体实例见下节。standard-ecu／user-application 使用 Rte、EchoApplication、DcmService；component-pipeline 使用 Rte、Ingress、Process、Observe、DcmService；no-diagnostic 使用 Rte、Ingress、Process、Observe，加上固定头分别为 15／15／17／16 个。`Rte_DcmService.h` 也以 DcmService 头包装 CODE 声明。实际文件、字节、TU 包含及每次请求以对应正常封存样本及 PR 工件为准，新增配置须重新评估。

**必需性与替代。** 固定 R24-11 SpecificationsARXML 的 `SWS_MemMap_00003` 要求 START／include／实体／STOP／include，`00011` 要求撤销请求，`00007` 要求预处理阶段拒绝非法、嵌套与错配。`00029` 要求为输入中的每个 SWC type 提供专用头，`00027` 要求支持其 MemorySection 的开闭；因此 DcmService 头的发布义务与当前有没有应用 TU 分开判断。BSWGeneral `00006` 要求模块映射头，`00249` 对普通头的整体 guard 要求显式排除映射头。条目正文已从固定 ARXML 核对；MemoryMapping PDF 36–41 的机制与对应示例一致。

整头一次性 guard 或 pragma once 会吞掉 STOP 及后续 START；删除第二次包含、把包含集中到文件首部，均失去上述请求处理。仅保护固定属性定义而让分派重入，正是当前保留的 D4.10 违反。拆成不同 START／STOP 文件会改变所选模块／SWC 的单一映射头及 implementedIn 合同；把分派移到另一个被包含文件仍需在原头逐次执行。普通声明头、版本头及可合并的多余请求保持各自整改责任。

**风险、控制与验证。** 重复处理可能遗留属性、重复开启或错配，进而放错段或影响后续实体。头内仅处理预处理请求／属性，固定定义按现有 guard 保护；每次消费请求，校验 active／class，STOP 清除本次属性及状态。生成 RTE 以共享 active 防跨 scope 嵌套，SWC 以各自 active 保护。实际声明／定义映射仍由原 Code／Var 消费者约束；平台段布局、关键词／类别、producer、模块实体或包含闭包变化即重评。

现有 `application_headers_enforce_runnable_visibility_memmap_and_imported_releases` 用真实 C／C++ 编译 CODE→STOP→VAR 重入、请求消费、STOP 清理及非法／多请求；`bsw_function_declarations_preserve_configured_code_placement` 和 `component_application_definitions_preserve_configured_sections` 观察实际目标文件段。`check_memory_mapping` 核对 OS 对象及段，已有成功编译后的属性删除反例；`test_memory_mapping_protocol_and_mutations` 拒绝缺 STOP、错 header／class 及嵌套。生成注释须由实际交付头检查及删除反例核对，测试结果和实际双平台范围在 PR 记录。

原始分析保留。本项仅接续 D4.10；人工组的 D4.12、其他规则及未完成的来源责任继续按各自记录处理。

### 独立 contract 的具体实例

本实例以 `678fae273e5d39e22f74fc9cc1b6d7041e84295b` 的
`core/tests/fixtures/component-pipeline/arxml/` 九份输入为范围，语言 C99。
2026-11-08 由项目实现者重新核对上述 D4.10、TC2.2、AMD2.4／2.5、
Compliance:2020 §4.1–4.4 及固定 SpecificationsARXML 的 MemMap
00003／00007／00011／00027／00029 正文；规则、原授权、Reason 1、
替代方案和风险控制沿本节共用记录。当前实例的独立复核收据仍由
[#440](https://github.com/g1331/autsaro/issues/440)承接；复核须指明实际 head、
原文、包含闭包、发现与处置，完成状态以实施 PR 收据为准。

**输入与输出身份。** 正常 Cargo `it` 的
`source_derived_composition_contract_is_deterministic_and_keeps_local_identity`
从上述目录读取输入，以 builtin 定义建立已校验 plan，经
`component_contract_files` → `composition_contract_files` → `memmap::rte`
生成 `include/Rte_MemMap.h`。设置 `AUTOSAR_CONTRACT_OUTPUT` 为不存在的新目录，
由 `preview`／`generate_previewed` 安装真实交付包；`contract.json` 保留源摘要及组件身份。
此次 RTE 头 SHA-256 为
`b067502e2a3e1798ec5437d38019ee0560be3f2d463e3ef82c473dde72996c42`，
首个 `@file` 的 MR12 DIR 4.10 记录在第 1–5 行，分派从第 7 行开始。
四个 SWC 头由 `memmap::component` 生成，各自首个 `@file` 同样受交付测试约束。
输入摘要、五个头摘要和实际消费者位置集中在实施 PR。

**请求与包含闭包。** 本输入的 RTE 表恰有十组 START／STOP：
Rte 的 CODE、CONFIG_DATA_PREBUILD_UNSPECIFIED，以及
DcmService／Ingress／Observe／Process 各自的 CODE、CONST_UNSPECIFIED。
四个 `Rte_<组件>.h` 均经 Rte_MemMap 包围 `Rte_Inst_<组件>` 常量指针声明，
并用对应 CODE 请求包围 RTE API 声明；runnable 声明另由同组件 SWC 头包围。
Ingress／Process／Observe／DcmService 的 RTE 实际包含次数分别为 8／6／8／4，
即合计 13 对；Rte scope 的两组属于包提供的能力，本输入的应用头未请求它们。
这里逐次执行的包含、请求消费与 STOP 清理沿 00003／00011；
版本头自身保留普通 guard。独立包只有声明，真实 API 定义与 ECU 内存对象由后续阶段提供。

**观察与证伪。** 正常 `native` 的
`standalone_contract_memmap_consumes_requests_and_reenters` 在新目录安装该包，
分别编译四个应用头，再次执行各自 CODE 开闭，断言请求、属性及 scope／共享 active 已撤销；
目标 `objdump` 观察独立消费者 `contract_code` 落在 `.rte_code`。
缺请求、跨 scope 嵌套、错配 STOP、同次多请求分别匹配本头的预处理错误；
四个 scope 均检查完整开闭后再次 STOP 报 `Unmatched RTE memory section`。
整头 guard／pragma once 会使合法应用头及再次请求失败；删除 STOP 清理同样被消费断言拒绝。
`it` 检查五个实际映射头集合及首段规则号、必需请求、嵌套／错配与清理理由，
删除或改错说明由同一正常入口定位到具体交付文件。

独立 contract 按自己的输入和消费者取证；四 ECU 的目标／程序、人工表与批准上下文
沿原记录保留。当前本地证据为 Linux GCC 编译／目标文件观察，Windows 的执行结果由
实施 PR 必需 CI 收据记录。producer、输入、请求集合、声明所有者、工具链／布局或
文件说明改变时重评；未完成独立复核前保持本实例的正式义务待完成。

## 条件指令与宏实参的源码边界

R20.4／20.6／20.13／20.14均为Required，当前C99适用（主文印刷157、158、163、164／PDF165、166、171、172；AMD2追加C11，TC2.35只更正20.14示例文件名）。最新四份Linux交付语料的自有／生成C/H与所选补丁后内核分别为219／219／229／226份。按实际指令词法及括号／条件嵌套核对，没有发现使用C99关键字或项目使用的GNU关键字别名定义宏，也没有跨文件配对的条件指令。所有实际包含源中的指令种类均为C99允许形式；这不把pragma的实现内容自动判为可移植。

两条出现在tasks.c内的条件指令位于eTaskGetState的普通if表达式中，外层if不是宏；其中listLIST_ITEM_CONTAINER的宏实参在指令前已经闭合，故不属于R20.6禁止构造。已核宏替换列表的括号均配对，没有发现借对象宏单独打开实参列表的另一条路径。compat/include/StackMacros.h的GNU warning不是C99指令，但实际tasks.c直接包含include/stack_macros.h，兼容头未进入所选TU，不能把归档文件存在当成当前调用；启用该兼容入口须另评。

R20.9（Required，印刷161／PDF169）的原文示例明确允许defined(B)与短路的B比较同处表达式；因此Os_Backend的测试开关保护不因未选择测试宏就构成未定义值求值。编译器预定义、命令行定义及头内定义仍须按实际来源追踪，不能把这个例子扩成所有未知名字都有默认零的许可。本节支持所核源码／条件，SDK、其他配置和系统头展开不随之销项；此前只检测行首无缩进指令的初版候选结果未作证据，最终核查包含缩进指令。

## 具体报告的分类边界

新standard-ecu的完成工件unsigned-masks-final-results保留643个R20.1物理报告位置：641个实际MemMap包含，另两个是kernel/tasks.c末尾的tasks_test_access_functions.h和freertos_tasks_c_additions.h。后两条虽未选择，R20.1明确在预处理前应用，因此不能按关闭功能销项；也不属于AUTOSAR必要冲突。其位置服务于访问该TU私有类型／函数的上游测试与扩展接口，保留是Advisory可维护性取舍，不是测试头已进入生产TU或已获ADR0023批准。

同一工件334个R20.5报告按真实指令区分为：十五个MemMap头中的148个模块START／STOP关键词、183个内部属性／active状态撤销，以及三个内核MPU_WRAPPERS_INCLUDED_FROM_API_FILE撤销。这里只按物理报告位置分类；某个分支实际执行#undef，仍须连接相应TU的具体包含请求，不能把所有分支都当作运行过。

148个关键词对应00011直接义务。183个内部撤销须按具体消费者区分。属性／active主状态的撤销避免旧属性或活动状态遗留，使后续合法请求可进入且段外对象不继承旧类；OS_MEMMAP_CODE_ACTIVE则只由现有memory_mapping夹具观察，不能把这一辅助标记单独说成生产映射不可缺少。改成永久可用的类别属性并移除状态检查是另一实现设计，并非无行为影响的删除；须重核所有生产者、段外拒绝和交付映射合同。这里按主文§6.2.3保留Advisory违反及具体理由，不声称这些内部名字由SWS强制规定。三个采用内核marker在头处理后撤销，以免后续实现继续按API-wrapper角色展开；它们也不属于MemMap授权。

上述分类按本篇类别记录处理 R20.1／R20.5；实际 include、START／STOP 及被映射实体仍可追溯，D4.10 另按可重入头及实际调用方核查。原有同类连续块已合并的证据保留，不把可删除的多余配对列为必要。累计技术复核前不填批准表／MR12；原始诊断、Required限制及SWS_BSW_00115均保持。

`0d627485`的四个漏报位置仍按旧字节保留：Os_Host.h:59、Os_PortRestart.h:16、Os_Schedule.c:6、Os_Vector.h:17。其typedef属于普通C内容，固定addon的语义tokenlist不能代替预处理前判断。后续[类型归属整改](https://github.com/g1331/autsaro/pull/371#issuecomment-6433452771)将native ABI别名与PC／向量类型放入Os_HostTypes.h／Os_PortTypes.h，两个类型头均在typedef前完成include且无MemMap；原服务／向量声明仍在原映射对。Schedule直接使用既有ARTI状态类型。这是消除实际前置构造，不新增保留；私有别名／文件划分不称为SWS强制，旧计数不改成新head结果。

其余类型前缀及C++链接包装的首次位置另见[类型前缀与首次映射请求](../types/declarations.md#类型前缀与首次映射请求)，不与实体后的STOP包含共用不可兼得结论。
