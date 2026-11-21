# 声明与初始化的检查边界

输入、固定工具、来源责任和待完成项的解释沿[语言与词法](lexical.md)。
同号入口以解析出的函数、变量、类型、scope 和值流为输入；CTU 必须按两个程序分别收齐。
既有[声明用途](../declarations.md)、[公共类型](../public-types.md)与
[初始化层级](../../bsw-general/language.md#初始化层级与实际求值)保留各自实体证据。
本表逐项明确恢复条件，尚未完成的实际核查由 #443 承担。

R8.1 已重新阅读主文印刷 63–64／PDF 71–72 的正文及适用性，明确仅适用 C90。
AMD2.27 的语言扩展表没有为 R8.1 追加 C11；其余 AMD1／AMD3／AMD4／TC1／TC2
检索未发现修改该语言范围的条目。所选 CI 两目标实际编译数据库使用 C99，
因此该条语言适用性成立。C99 的声明约束继续由编译器、R1.1、R8.2／R8.4 承担。
各行按同一七份原件合成有效正文：主文印刷 64–81／PDF 72–89；AMD2.27
追加 C11，AMD2.49／50 只更新 R9.4／R9.5 的标准引用；TC1 为 R8.4 增加 main 例外；
TC2.10–15 为 R8.2／R8.3 增交叉引用、明确未命名形参例外并为 R8.7 增例。
AMD3.63 补 R8.2 的 R1.5 引用；AMD4.42 补 R8.6 的对齐引用，43–46 将 R8.9
改为声明，47–50 明确 R9.4 的指定／后续位置式覆盖，56–58 为 R9.1 排除 _Atomic
并为 R9.2 追加 R9.6 引用。AMD1 及其余 TC／AMD 没有修改本表相关 C99 义务。
固定 e73bf44 的同号函数／executeCheck、CTU 摘要和核心映射均已核对。

本次真实 GCC 展开按来源标记回到自有、生成／应用及项目 patch；采用原行沿原组。
SDK 原头保持供应方责任，自有调用及已登记宏沿 sdk-integration；R8.2 的已知非 ISO
内层无名形参继续保留实质缺口。以下满足项以实际构造和消费者为依据。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R8.1 | Required | 双目标四 profile 的 C99 HostBatch／probe | not_applicable | 主文 63–64／PDF 71–72 仅适用 C90；AMD2.27 未扩展本条，AMD1／3／4 与 TC1／2 未改变语言范围。当前实际 TU 使用 -std=c99；同号 addon 的 implicit-int 诊断仍保留，C99 声明和扩展义务继续按各自规则核查。 |
| R8.2 | Required | 双目标四 profile 的 C99 HostBatch／probe，函数原型、定义和嵌套回调 | retained | 主文64–66、TC2.10、AMD3.63。按 452 份 GCC 展开核对普通原型、指针 typedef 及嵌套回调的形参；范围内保留 SWS_Rte_06705 的无名 role。Det_ErrorHookType 与外层 Det_ValidHooks 形参具名，Det 精确误报独立处置。详见[实体和实际 CI](../../rte/generated-diagnostics.md#r82-当前保留实体)，当前输入须经正式批准绑定。 |
| R8.3 | Required | 同一对象／函数的全部声明 | not_assessed | 主文66–67／PDF74–75、TC2.11–14；底层类型相同仍须同一类型名字，未命名参数的命名例外单列。第八批统一 Os_ArtiTaskStacks 的 struct／typedef 拼写和 Os_Closing 的 LONG／Os_Atomic32Type；0015 项目 patch 在 task.h 用 TaskHandle_t，而 tasks.c 用 TCB_t *，真实 Required 义务保持。#503 保留 0015 的真实静态对象布局和端口消费者，#443 的 #547 继续核对这对声明；其余同一实体的签名及限定符按两个程序逐 TU 合并复核。 |
| R8.4 | Required | 外部链接定义处可见的兼容声明 | satisfied | 主文67–68、TC1 main 例外。逐实际 TU 检查外部函数／对象定义前的声明及包含顺序；TASK 宏自身先产生原型，FreeRTOS 两个 idle hook 由实际 task.h 提供，RTE／应用定义经各自公共头，Os_Closing 与 ARTI 对象沿私有／公开 owner。C99 编译检查兼容类型，定义前声明集合无缺项；原型名／typedef 拼写的其他 R8.3 义务仍单列。 |
| R8.5 | Required | 外部对象／函数声明的头文件 owner | satisfied | 主文69／PDF77 只约束非定义声明。逐程序按物理文件／行合并头重入和同一实体；第八批删除 Backend.h 的 vApplicationTickHook 重复原型，复用 task.h。Os_BackendStackFault 的声明移入既有 Windows provider 头，与唯一 Windows 定义和 Stack 消费一致；其他公共数组、CDS、配置和服务均由一个物理 owner 声明，未将定义当成第二个原型。 |
| R8.6 | Required | 每个程序的外部定义唯一性 | satisfied | 主文69–70、AMD4.42 的交叉引用。HostBatch／probe 分别按实际链接 TU 合并外部对象和函数，tentative 与最终定义按同一 TU 结算；移除 Linux 上不存在定义的 Os_BackendStackFault 声明，Windows 保持同一回调。Os_PortTaskLoop／Os_PortRestartAddress 的两个 C 入口由所选 provider 的汇编 .global／标签唯一定义，实际链接及原生重启消费者核查。当前定义无重复／缺失，未以链接成功替代未引用声明检查。 |
| R8.7 | Advisory | 外部链接实体的跨 TU 真实引用 | not_assessed | 主文70、TC2.15；当前范围内68个物理报告及 CTU 漏报候选仍须按 TU 去重。取址、ARTI 工具视图、公开 BSW／生成包装和实际应用调用需逐实体关联，定义与引用同 TU 的候选保留真实 Advisory。#443 的 #547 接续完整引用集合和必要签名／可见性的逐项处置，历史私有化和原精确表仅覆盖各自实体。 |
| R8.9 | Advisory | 只在单个函数出现的对象标识符 | not_assessed | 主文71–72、AMD4.43–46 约束声明。ecu_can_driver 只在 Ecu_StackBootstrap 使用，按 SWS_MemMap_00003／00018／00023 保留[精确处置](../storage.md#生成-can-配置的有界处置)。ecu_comm_users 和 [resource_owner](../storage.md#r89-资源所有者数组误报) 均有文件作用域初始化引用。#443 的 #547 接续其余 Init 配置、ARTI 对象和既有精确记录的声明／段／消费者核对。 |
| R8.10 | Required | inline 函数的存储类 | satisfied | 主文 72；AMD2.27 追加 C11，其他修订未改变 static 义务。同号入口检查 isInlineKeyword／isStatic；独立检查实际全部展开中的 inline／__inline／__inline__，自有／生成／patch 均显式 static，包括 Os.h 服务、Os_StackIndex、Ecu_StatusName、Os_Counter.h 换算和 Rte_Os.h 桥；实际调用分别回接 Os_Stack.c、Ecu 入口及 Rte_OsService.c。SDK 原头内部沿原采用责任。 |
| R8.11 | Advisory | 外部链接数组声明的显式大小 | satisfied | 主文 72 只约束非定义声明，AMD2.27 追加 C11；同号入口的 extern [] 检查另由完整实际展开补齐。全部外部数组为 Arti_Events[4096]、Os_ArtiTasks／Stacks／Contexts[16]、Os_InterruptVectorTable[32]，头与定义使用同一容量；Arti.c 发布、Backend 观测和中断分派直接消费。Rte_Type 的 Dcm 数组 typedef、Ecu_MailboxInput 及 Can_TxSinkType 数组形参已另查，无额外外部数组别名声明。 |
| R8.12 | Required | 枚举列表的隐式值唯一性 | satisfied | 主文 73，AMD2.27 追加 C11；同号入口依赖 values／valueType，另按实际全部枚举体独立求值。CanTp 状态、ComM、Dcm 发布、PduR、Ecu_Status 和 OS hook／host 枚举均全显式或初始零后单调递增，隐式项无重复；ARTI／OS 宽度及对齐 sizeof 断言枚举全显式，允许相同值。消费者为相应状态比较／switch，生成 Ecu_Config 值与配置消费一致；无项目 patch 新增隐式冲突。 |
| R8.13 | Advisory | 指针所指对象的可修改性 | not_assessed | 主文73–74 同时约束指针和数组元素；写入或复制到可写指针的允许条件按实际值流核定。#443 的 #547 从 Det／Dcm／Can 和 OS 接缝的全部指针、数组，经别名、赋值、内存复制、callback 和成功／失败输出追到最终写方或可修改接口；标准 ABI 和 SDK 输出保持，局部 constPointer 零报告未替代系统证明。 |
| R8.14 | Required | 原声明及实际宏展开的 restrict | satisfied | 主文 75，AMD2.27 追加 C11；同号入口只查 raw restrict。物理自有文件及实际全部展开另查 restrict／__restrict／__restrict__、宏及 typedef，无项目或 patch 限定用途；Os_LinuxNative.c 的 memcpy 和 pthread／SDK 声明接缝保持原头，ISO 库内部按 §6.7，非 ISO 原头沿采用责任。未以别名安全或包装创造规则例外。 |
| R9.1 | Mandatory | 双目标四 profile 的 C99 自动对象及逐成员／元素 | satisfied | [首次读取核查](#r91-首次读取核查)覆盖 Dcm 取消与重入、OS 部分输出、BSW 传输和实际生成消费者。Windows 整体复制已修；主文75–76 与 AMD4.56–57 适用边界见下。 |
| R9.2 | Required | aggregate／union 初始值的层级 | satisfied | 主文 77–78、AMD4.58；[类型化初始化核查](#r92r93-类型化初始化核查)覆盖嵌套结构、union、typedef 数组及 compound literal，逐项核对四类例外。 |
| R9.3 | Required | 数组的显式元素初始化与例外 | satisfied | 主文 78–79；[类型化初始化核查](#r92r93-类型化初始化核查)覆盖容量—元素映射及 {0}、全指定和字符串例外。 |
| R9.4 | Required | 每个子对象的重复初始化 | satisfied | 主文 79–80＋AMD2.49／AMD4.47–50；固定 misra_9 的 904／numInits 模型之外，逐个检查当前展开指定体及子列表。Os_Config 的 tasks／counters 各结构、hooks／time_config／Ecu_OsConfig、Os_ErrorParametersType 的单个 union 成员、Linux timespec／pollfd／process_handle 均无同一子对象重复指定，也无指定回退后的位置覆盖；priorities 仅 [30u]=1u。其余全位置／零初值各消费一次，无 patch 新增重复目标。 |
| R9.5 | Required | 有 designated initializer 数组的显式大小 | satisfied | 主文 80–81＋AMD2.50 仅移除固定节号；固定 misra_9 的 905 模型另核实际指定目标。唯一数组元素 designator 为 interrupts.priorities[OS_INPUT_INTERRUPT]，实际 [30u]，Os_IsrConfigType.priorities[OS_INTERRUPT_VECTOR_COUNT] 显式容量 32；Backend 中断优先级直接消费。tasks[]／counters[] 的 designator 位于各结构元素内部，数组本身用位置式列表；无其他项目／patch 数组元素指定或 flexible-array 初始化。 |
<!-- misra-checker-obligations-end -->

## 第八批的当前结算

早期调查基线为 master `47d14ed0` 和 [run 43978366447](https://github.com/g1331/autsaro/actions/runs/43978366447)。
[PR #552](https://github.com/g1331/autsaro/pull/552) 的实际检出 `ddd59c52` 与
扫描 head `22aebb40` 同树，[run 44029684376](https://github.com/g1331/autsaro/actions/runs/44029684376)
覆盖双目标四 profile 的 888 个 TU 实例、452 份 GCC 展开。按 GCC 行标记回查物理
owner，分别统计声明、定义、引用及来源。878 种定义的 975 个命名形参均有引用。
235／239 种整数拼写及 Windows 24 次 INFINITE 按各自类型和 SDK 处置核对。

本批 25 项中 7 项满足，18 项移交 [#547](https://github.com/g1331/autsaro/issues/547)。
#552 改 36 个重用名并统一 Os_Closing／ARTI 声明，保持对象类型、存储期、段及外部 ABI。
其中生成名称由原 emitter 修复。R8.9 driver 真实违反由[批准的有界处置](../storage.md#生成-can-配置的有界处置)覆盖。
后续 #547 又改 7 个名称，[剩余 R5.9 证据](lexical.md#r59-私有对象改名)保留实际范围。
原始诊断与严格比较保留。

## R9.1 首次读取核查

主文印刷 75–76／PDF 83–84 把数组元素和结构成员当作独立对象。AMD2.27 扩至 C11，
AMD4.56–57 将 `_Atomic` 交 R9.7；其余修订未改变这里的 C99 义务。
[双平台实际输入](https://github.com/g1331/autsaro/actions/runs/44518089096/attempts/2)
包含 888 个 HostBatch／probe 实例和 452 份 GCC 展开。按行标记合并同一行的宏展开，
按物理声明和展开形态去重，得到 1347 条自动声明记录，其中 358 条有延后赋值的声明项。
按类型检查初始化覆盖，并跟踪循环已写前缀、分支状态和返回值。Linux
`Os_LinuxNative.c` 的 `__builtin_offsetof` 位于未求值的 ABI 检查，另核原表达式及复制长度。
固定 Cppcheck 将 R9.1 映射到 `uninitvar`，八份结果均无本条诊断，路径证据如下。

- **Dcm 请求**。完整成功接收才进入 `DCM_REQUEST`。`Dcm_PrepareWork` 设置消息的两个指针、
  长度、容量、标志和 ID，以及配置、会话、generation 和取消标记。SID 和子功能从静态槽
  复制，DID 对只读已接收且通过偶数长度检查的两个元素。正响应在增加长度前写全相应的
  session／TesterPresent／可用性数据。应用路径经生成 RTE 到 `Ingress_ReadData` 或
  `EchoApplication_ReadData`，成功写四字节；拒绝只发送 `Dcm_Negative` 写好的三字节。
- **Dcm 取消与重入**。`dcm_actors.executing` 序列化回调，Init 更新 generation，旧执行者
  无法提交。未领取请求时不读 work；响应只复制已发布长度。`Dcm_NextPublication` 每次
  初始化全部成员，再按标签读取已选字段。测试覆盖部分写入后失败、回调内 Init 和重入。
- **OS 部分输出**。Windows 三个观察入口在成功捕获后才读取，失败走退出或
  `configASSERT`；`VirtualQuery` 返回完整大小后才读取成员。`Os_StackObserve` 只复制
  flags、CS、SS、EFlags、RSP、RIP，其余成员初始化为零。Windows 原生夹具用哨兵证伪
  整体复制。Linux `Os_LinuxCaptureContext` 先写内核前缀，再读 RSP／FP 指针，glibc 快照
  的其余成员预先置零。两端 port 先写线程和事件字段，再按完整大小保存或取回。
- **OS 服务与重入**。`Os_BackendInspect` 先清零整个快照，失败提前返回；先写
  `previous[i]`、`completed[i]` 再遍历同一任务范围。`advance_counter` 每次入栈写全
  三个成员。计数器、告警、事件和表状态的成功出口写输出，ARTI 失败出口只使用哨兵，
  ErrorHook 保存参数指针，嵌套中断先完整保存再恢复。`GetElapsedValue` 先读已提供的
  INOUT 样本，拒绝不读 OUT。`Os_TargetTakeInput` 的生产者先置零整个 data 再复制有效前缀。
- **其他运行时与生成代码**。`CanTp_WorkType` 按 kind 选取各分支已写的字段；
  下层和接收分支写全八个 frame 元素。`Com_Pack` 写四字节后才允许发送，
  `Can_InternalTransmitPdu` 写齐 PDU 成员，payload 尾部保留零。路由拒绝分支不读
  未交付输出。`Ecu_StackBootstrap` 仅成功取回控制器模式才读 actual；SchM 在区内
  取值后返回。协议解析按已写 token 数读取，HostBatch／probe 只消费成功交付的
  记录、票据和长度；文本及平台读取缓冲只消费写入的前缀。SDK 内部与采用原行
  沿 ADR 0025，主机路径核查沿 ADR 0030 记录。

## R9.2／R9.3 类型化初始化核查

主文印刷 77–79／PDF 85–87、AMD2.27 与 AMD4.58 按 C99 合成。初值按实际
子对象类型判定。R9.2 检查显式初始化的 aggregate／union 层级，逐项核对
`{0}`、字符串、自动结构／union 的兼容类型表达式及部分指定子对象例外。
R9.3 对数组的部分显式初始化只接受 `{0}`、全指定初始化或字符串。

[双平台取证](https://github.com/g1331/autsaro/actions/runs/44625907914/attempts/1)
覆盖 452 份 GCC 展开、888 个 HostBatch／probe TU 实例。Clang 按 GCC 行标记同时
核对显式括号和类型化子对象，数组容量由展开后的类型取得，含 typedef 和函数指针数组。
Linux 228 份完整解析。Windows 224 份中有 120 份在 GCC SDK 的 intrinsic
实现中触发 Clang 恢复，项目文件没有解析错误或损坏的初始化节点。按物理位置、
类型和展开形式去重，记录 210 项，其中 38 个数组列表显式覆盖全部元素。
补丁归属取 `patchedLines`，SDK、采用原行及主机实现沿 ADR 0025／0030。固定
Cppcheck 的两条原始诊断均为零。单独置于调用实参中的 compound literal，省略
typedef 数组的内层括号或只初始化其三项中的两项，固定 addon 均漏报。等价的变量
声明分别触发 R9.2 与 R9.3。类型化核查检出两个实参反例。

- `Os_ErrorParametersType` 按服务选择同一 union 成员，成员结构有内层括号，
  由错误记录传给 ErrorHook。三处 compound literal 分别清空错误上下文、建立
  计数器工作栈的根项和子项。ARTI 的兼容结构表达式交给自动对象。
- `Ecu_OsConfig` 的任务、计数器及告警数组由 OS 启动和计时服务消费，位置列表
  与推导容量一致。`priorities[32]` 通过 `[30u] = 1u` 指定输入中断，全部采用
  指定式。`task_name` 使用字符串并随任务配置交付。
- `Os_TimeValidate` 的 `increments[8][8]` 整体 `{0}`，内层省括号与隐式填零
  同时满足两条例外。`Os_InterruptVectorTable[32]` 和调度访问数组也以 `{0}`
  初始化。Can payload 八项与 CanIf errors 两项按各自数组容量逐元素初始化。

## 当前输入重评

第二批的 run 43411175434／实际测试合并树73caf622，以及第七批的
[run 43996223375](https://github.com/g1331/autsaro/actions/runs/43996223375)／扫描 head
330b3fb2，保留各自历史输入和处置。第七批结算78／3／63；本批修复名字与声明、
增加生成配置的精确 R8.9 关联后，旧正文／解析器／producer 身份均按当前输入重新评估。

第八批的八份验收重算与 source／producer 拒绝结果归 #514 的实施 PR。
下列上下文接续[第九批](control-flow.md#第九批的当前结算)，绑定双平台四 profile
的实际分析输入。逐份核对 seal、最终 kernel、producer、各程序 TU 和实际
GCC／Cppcheck 输入。Windows 以实际检出的换行重算，原始 summary 保留。

<!-- source-assessment-context-begin -->
{
  "id": "checker-obligations-547-r59",
  "evidence": {
    "head": "4290542118626cbd111bf81b5e5e1ea51e290ccf",
    "receipt": "https://github.com/g1331/autsaro/actions/runs/44672571288/attempts/1"
  },
  "cases": [
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "217e7d19ba3cc0d2d16c8dcef2e6654adf1ec766ca431f54422a60b356d9afaf"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "bdfcfec80f807ad7e9cfaf896e5a3ec0f8fdff1e31274577e3874dcb5ffc0fb0"
    },
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "12264f2749adf533a795106c16128228a61e125c9c2030434101d7d336d4975e"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "7b1b44b47f5dd571d86dfacead71427695ec8202304639f4b048c6c93ccc5de2"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "e3787b25ca234a922d95d9224b507a2904736c7df798d9a789648f9962b6066b"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "e151d7eca723dad2ded670042dd2b05f156b2650bfde30618fca0ee7aa0d2705"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "4c1b71e87683014013600d1390ec0176131fc525e4a4c8442a28934f9a06deab"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "0c3cf42df659792d204c952944119a71998b504990abe7395b788b6b505c94eb"
    }
  ],
  "records": {
    "kind": "checker-obligations",
    "retainedSha256": "dc145841e206867098e08d05e5abcde4d7294e2542b05c15e40252e17f1d7944"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
