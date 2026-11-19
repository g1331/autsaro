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
| R8.2 | Required | 公开及内部函数类型、原型与定义 | not_assessed | 主文64–66、TC2.10／AMD3.63 的交叉引用。Det_ErrorHookType 的 typedef 指针候选须与函数模型误报区分，RTE 无名 role 参数沿既有固定 SWS 处置；普通 void 原型与无参数 K&R 形式分开。#443 的 #547 补齐所有 typedef／指针／嵌套 callback 的函数类型集合与正式义务，非 ISO 原头沿 ADR 0025，host provider 按 ADR 0030。 |
| R8.3 | Required | 同一对象／函数的全部声明 | not_assessed | 主文66–67／PDF74–75、TC2.11–14；底层类型相同仍须同一类型名字，未命名参数的命名例外单列。第八批统一 Os_ArtiTaskStacks 的 struct／typedef 拼写和 Os_Closing 的 LONG／Os_Atomic32Type；0015 项目 patch 在 task.h 用 TaskHandle_t，而 tasks.c 用 TCB_t *，真实 Required 义务保持。#443 的 #547 连同 #503 核定内核声明与表示；其余同一实体的签名及限定符按两个程序逐 TU 合并复核。 |
| R8.4 | Required | 外部链接定义处可见的兼容声明 | satisfied | 主文67–68、TC1 main 例外。逐实际 TU 检查外部函数／对象定义前的声明及包含顺序；TASK 宏自身先产生原型，FreeRTOS 两个 idle hook 由实际 task.h 提供，RTE／应用定义经各自公共头，Os_Closing 与 ARTI 对象沿私有／公开 owner。C99 编译检查兼容类型，定义前声明集合无缺项；原型名／typedef 拼写的其他 R8.3 义务仍单列。 |
| R8.5 | Required | 外部对象／函数声明的头文件 owner | satisfied | 主文69／PDF77 只约束非定义声明。逐程序按物理文件／行合并头重入和同一实体；第八批删除 Backend.h 的 vApplicationTickHook 重复原型，复用 task.h。Os_BackendStackFault 的声明移入既有 Windows provider 头，与唯一 Windows 定义和 Stack 消费一致；其他公共数组、CDS、配置和服务均由一个物理 owner 声明，未将定义当成第二个原型。 |
| R8.6 | Required | 每个程序的外部定义唯一性 | satisfied | 主文69–70、AMD4.42 的交叉引用。HostBatch／probe 分别按实际链接 TU 合并外部对象和函数，tentative 与最终定义按同一 TU 结算；移除 Linux 上不存在定义的 Os_BackendStackFault 声明，Windows 保持同一回调。Os_PortTaskLoop／Os_PortRestartAddress 的两个 C 入口由所选 provider 的汇编 .global／标签唯一定义，实际链接及原生重启消费者核查。当前定义无重复／缺失，未以链接成功替代未引用声明检查。 |
| R8.7 | Advisory | 外部链接实体的跨 TU 真实引用 | not_assessed | 主文70、TC2.15；当前范围内68个物理报告及 CTU 漏报候选仍须按 TU 去重。取址、ARTI 工具视图、公开 BSW／生成包装和实际应用调用需逐实体关联，定义与引用同 TU 的候选保留真实 Advisory。#443 的 #547 接续完整引用集合和必要签名／可见性的逐项处置，历史私有化和原精确表仅覆盖各自实体。 |
| R8.9 | Advisory | 只在单个函数出现的对象标识符 | not_assessed | 主文71–72、AMD4.43–46 约束声明。ecu_can_driver 只在 Ecu_StackBootstrap 使用，按 SWS_MemMap_00003／00018／00023 的原页及本批批准加入[精确表](../storage.md#生成-can-配置的有界处置)，保留真实 Advisory；ecu_comm_users 在函数外被 comm_config 初始化引用，单函数条件未成立。#443 的 #547 接续其余 Init 配置、单函数对象及既有22项记录的全部声明／段／消费者结算。 |
| R8.10 | Required | inline 函数的存储类 | satisfied | 主文 72；AMD2.27 追加 C11，其他修订未改变 static 义务。同号入口检查 isInlineKeyword／isStatic；独立检查实际全部展开中的 inline／__inline／__inline__，自有／生成／patch 均显式 static，包括 Os.h 服务、Os_StackIndex、Ecu_StatusName、Os_Counter.h 换算和 Rte_Os.h 桥；实际调用分别回接 Os_Stack.c、Ecu 入口及 Rte_OsService.c。SDK 原头内部沿原采用责任。 |
| R8.11 | Advisory | 外部链接数组声明的显式大小 | satisfied | 主文 72 只约束非定义声明，AMD2.27 追加 C11；同号入口的 extern [] 检查另由完整实际展开补齐。全部外部数组为 Arti_Events[4096]、Os_ArtiTasks／Stacks／Contexts[16]、Os_InterruptVectorTable[32]，头与定义使用同一容量；Arti.c 发布、Backend 观测和中断分派直接消费。Rte_Type 的 Dcm 数组 typedef、Ecu_MailboxInput 及 Can_TxSinkType 数组形参已另查，无额外外部数组别名声明。 |
| R8.12 | Required | 枚举列表的隐式值唯一性 | satisfied | 主文 73，AMD2.27 追加 C11；同号入口依赖 values／valueType，另按实际全部枚举体独立求值。CanTp 状态、ComM、Dcm 发布、PduR、Ecu_Status 和 OS hook／host 枚举均全显式或初始零后单调递增，隐式项无重复；ARTI／OS 宽度及对齐 sizeof 断言枚举全显式，允许相同值。消费者为相应状态比较／switch，生成 Ecu_Config 值与配置消费一致；无项目 patch 新增隐式冲突。 |
| R8.13 | Advisory | 指针所指对象的可修改性 | not_assessed | 主文73–74 同时约束指针和数组元素；写入或复制到可写指针的允许条件按实际值流核定。#443 的 #547 从 Det／Dcm／Can 和 OS 接缝的全部指针、数组，经别名、赋值、内存复制、callback 和成功／失败输出追到最终写方或可修改接口；标准 ABI 和 SDK 输出保持，局部 constPointer 零报告未替代系统证明。 |
| R8.14 | Required | 原声明及实际宏展开的 restrict | satisfied | 主文 75，AMD2.27 追加 C11；同号入口只查 raw restrict。物理自有文件及实际全部展开另查 restrict／__restrict／__restrict__、宏及 typedef，无项目或 patch 限定用途；Os_LinuxNative.c 的 memcpy 和 pthread／SDK 声明接缝保持原头，ISO 库内部按 §6.7，非 ISO 原头沿采用责任。未以别名安全或包装创造规则例外。 |
| R9.1 | Mandatory | 自动对象读取之前的赋值路径 | not_assessed | 主文75–76、AMD4.56–57 将 _Atomic 交 R9.7；Mandatory。当前 Dcm 缓冲、Backend 的 GetThreadContext／API 输出、callback 取消与重入仍缺逐成员／元素的首次读取前赋值路径。#443 的 #547 补齐成功、失败和部分输出的定义—使用证明，真实缺口直接整改；已初始化的局部例子和零报告只作各自证据。 |
| R9.2 | Required | aggregate／union 初始值的层级 | not_assessed | 主文77–78 的 {0}、字符串、兼容结构表达式及指定子对象例外，AMD4.58 的 R9.6 交叉引用已合成。已定位208种去重 initializer-list 及3种 compound literal，含 Os_ErrorParametersType union、Ecu_OsConfig 与 Can payload；完整 typedef 数组／aggregate 的类型化子对象映射未齐。#443 的 #547 逐对象连接类型、层级与直接消费者后结算。 |
| R9.3 | Required | 数组的显式元素初始化与例外 | not_assessed | 主文78–79；{0}、全指定及字符串例外分别核查。Can payload[8] 八项、CanIf errors[2] 两项、increments 的 {0} 和 priorities[30u] 已有局部映射；全部数组／数组子对象的容量—元素集合尚未齐。#443 的 #547 补齐当前生成、应用和 patch 的类型化容量及消费者，普通尾部隐式填零未作为通用例外。 |
| R9.4 | Required | 每个子对象的重复初始化 | satisfied | 主文 79–80＋AMD2.49／AMD4.47–50；固定 misra_9 的 904／numInits 模型之外，逐个检查当前展开指定体及子列表。Os_Config 的 tasks／counters 各结构、hooks／time_config／Ecu_OsConfig、Os_ErrorParametersType 的单个 union 成员、Linux timespec／pollfd／process_handle 均无同一子对象重复指定，也无指定回退后的位置覆盖；priorities 仅 [30u]=1u。其余全位置／零初值各消费一次，无 patch 新增重复目标。 |
| R9.5 | Required | 有 designated initializer 数组的显式大小 | satisfied | 主文 80–81＋AMD2.50 仅移除固定节号；固定 misra_9 的 905 模型另核实际指定目标。唯一数组元素 designator 为 interrupts.priorities[OS_INPUT_INTERRUPT]，实际 [30u]，Os_IsrConfigType.priorities[OS_INTERRUPT_VECTOR_COUNT] 显式容量 32；Backend 中断优先级直接消费。tasks[]／counters[] 的 designator 位于各结构元素内部，数组本身用位置式列表；无其他项目／patch 数组元素指定或 flexible-array 初始化。 |
<!-- misra-checker-obligations-end -->

## 第八批的当前结算

调查基线为 master `47d14ed0`，早期材料取
[run 43978366447](https://github.com/g1331/autsaro/actions/runs/43978366447)。其 `cf8ddc4f`
已含 #538 Windows 栈修复；后续 #545 原生探针和 #546 CanTp 运算前类型扩展均回到
当前源码核对。#550 只改 Python 缓存夹具，已同步。

本批当前重评取 [PR #552 run 44029684376](https://github.com/g1331/autsaro/actions/runs/44029684376)
的双目标四 profile 完整输入：888 个 C99 TU 实例、452 份实际 GCC origins。实际检出
`ddd59c52` 的父提交为 `0f8cd603` 和扫描 head `22aebb40`，tree 与该 PR head 相同；
逐份核对 seal、最终 kernel、92 个 producer、解析器与评估正文后重算身份。
878 种函数定义的975个命名形参均有引用；235／239种整数拼写及条件传递闭包重核，
无额外自有缺 U，Windows 24 次 INFINITE 仍沿已批准 SDK 处置。最终 head 的同输入
gate、物理行对应与独立复核归本批 PR。

物理扫描先拼接续行、逐字符保留原行号，去注释保留换行，并用块注释＋续行的组合
反例验证；展开按 GCC 行标记回查原文件和原行，范围内无语法 ERROR。声明、定义、
typedef、tag、enum、成员和参数分别收齐；R8.4 保留 TU 中的先后次序，R8.5 按物理
owner 合并重复包含，R8.6 按两个独立程序核算。SDK／采用声明也参与类型和链接
接缝核对，其原件责任仍沿 ADR 0025；项目 patch 只计 patchedLines。

已修36个重用名：35个私有定义加所属模块前缀；栈记录成员改为 committed_bytes，
两个 provider 同步。应用输入快照与逻辑模式计划保持；SchM 在存储渲染处生成私有
前缀，RTE 存储描述同步，ECU 配置改在实际 producer；公开形参名字保持。
原对象类型、容量、存储期、状态所有权、锁和控制流保持。Os_Closing／ARTI 指针
声明统一类型名字；tick hook 复用 FreeRTOS 头；Windows 栈回调只由所选 provider
头发布。此处不变更公开 C 类型、布局或链接符号。
第八批留下的 8 个 R5.9 私有名由 #547 改了 7 个。OS 激活队列的名字和恢复条件见
[逐实例记录](lexical.md#r59-私有对象改名)。
#551 提供的等价证明用于判定新增，严格比较继续记录旧诊断位置。

本批25条结算7条 satisfied：R2.7、R5.8、R7.2、R7.4、R8.4、R8.5、R8.6；
18条转 [#547](https://github.com/g1331/autsaro/issues/547)，逐条原因及恢复条件见表。
完整144条为85 satisfied、3 not_applicable、56 not_assessed。R8.9 的 driver 真实
违反按[编排者批准的有界处置](../storage.md#生成-can-配置的有界处置)保留，其余未齐
责任继续阻塞。原始 passed／error、退出码、诊断、221项 assessment 和 completeCompliance
保持原义；完整验证、严格比较的逐行对应及独立复核归本批 PR。

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
    "head": "3a57a0661e7d3fb734e23a155f991c94090391c4",
    "receipt": "https://github.com/g1331/autsaro/actions/runs/44513871093/attempts/1"
  },
  "cases": [
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "cbfcbb537203718cced3b7e91e00f9c75fe11320b4a3dfd35db052e061b395c5"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "893d323e2ebb66cd4ad2c759058c6221bf55c6c1a26f2684d12130a5136bf78c"
    },
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "848ac4bd32c4f9def33a276c72031e858f6c9ecb814a89b48fe1eb93cba189a1"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "b7c07198dd4e655bb60e7c25095ad72aedc8e906e90912ca3b7236bc8555a94e"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "1abe697b82dc250e891ef0f112295a5c85bf614cef64ca0d0a3025f0fd0efcc7"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "55fa1123778602e14beb3bb51758e200c933b4955c66e67f9be33e3b23af83f8"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "f6372902a653e549346303f8aff4cc7329121d271323df79a886d9d393821202"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "af22f437d991fc6b5ce1af76b05cf45fa0d232547db2106bcf8bb687d0288238"
    }
  ],
  "records": {
    "kind": "checker-obligations"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
