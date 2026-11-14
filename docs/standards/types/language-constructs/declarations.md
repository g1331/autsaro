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
| R8.2 | Required | 公开及内部函数类型、原型与定义 | not_assessed | 主文 64–66；TC2.10／AMD3.63 仅加交叉引用。固定入口按函数模型及 raw token 检查，Det.c:16 的 Det_ErrorHookType、Rte_EchoApplication.h:87 的 Runnable 类型候选须按内层命名复核；当前实际包含 pthread_create／pthread_key_create 及 UAPI __signalfn_t 的 SDK 无名内层参数沿原声明篇和 #444。#443 恢复须收齐 typedef／指针／回调原型的实际完整集合，区分模型误报与真实缺名；所列 SDK 缺口没有新增处置。 |
| R8.3 | Required | 同一对象／函数的全部声明 | not_assessed | 主文 66–67；TC2.11–14 允许未命名形参的命名例外，R8.2 仍单列。核心 funcArgNamesDifferent 仅提供名字候选。当前 Det_ErrorHookType→Det_Invoke、RTE 服务声明→Rte_OsService.c 及 SchM／BSW 公共头分别可追溯；#443 仍须按每个实际程序合并全部声明，逐个比较 typedef 名称、限定符和形参名，含条件头与 patch；函数指针的形参名与目标函数分开判断。 |
| R8.4 | Required | 外部链接定义处可见的兼容声明 | not_assessed | 主文 67–68＋TC1 main 例外；同号入口以 tokenDef／token 和变量定义形态判断。当前 Os_LinuxMappedStack 的 Os_LinuxNative.h、生成 Rte_OsService.c 的 Rte_Os.h 与 ComM.c 的 ComM.h 可见；#443 仍须逐个外部函数及对象定义核对先前兼容声明和真实 include 次序，覆盖生成桥、应用 Runnable、条件接口及项目补丁；仅实际编译与零报告仍缺逐定义映射。 |
| R8.5 | Required | 外部对象／函数声明的头文件 owner | not_assessed | 主文 69 只约束非定义声明；同号入口保存 identifier CTU 摘要。当前五个外部数组的 owner 和 Rte_Instance_<组件> 的应用头可定位，重复 include 应归同一物理声明。#443 恢复须为完整外部声明集合建立单一物理 owner，区分头重入、条件声明与独立手写声明，并逐个连接定义／调用方；当前 CTU 输出没有完整类型化声明清单。 |
| R8.6 | Required | 每个程序的外部定义唯一性 | not_assessed | 主文 69–70＋AMD4.42 追加 R8.15 引用；同号入口保存 identifier CTU 摘要。16 个程序的 TU 与回执闭合，Rte_Instance_<组件>、Ecu_OsConfig 和 Os_InterruptVectorTable 的实际定义可定位；#443 仍须逐程序结算全部函数／对象的唯一定义，包含 tentative、inline、条件及未引用声明，独立 HostBatch／probe 各自核算。 |
| R8.7 | Advisory | 外部链接实体的跨 TU 真实引用 | not_assessed | 主文 70＋TC2.15 的示例；固定 usage CTU 去重不足以覆盖全部单 TU 实体。当前 standard-ecu 原报告 Linux／Windows 为 101／100 项，含 BswM.c:54、Can.c:298 和生成导出；#465 的 ComM 端口与当前 CDS 取址已连接实际调用。#443 恢复须逐实体区分真正跨 TU 引用、函数地址和仅声明用途，按原链接可见性篇完成收窄或有界记录。 |
| R8.9 | Advisory | 只在单个函数出现的对象标识符 | not_assessed | 主文 71–72＋AMD4.43–46 约束声明；固定入口仅统计函数体内全局 static 对象。当前 standard-ecu 原报告 Linux／Windows 为 33／32 项，含 Can.c:113、Com.c:54 和 Ecu_RuntimeConfig.c 配置；Os_Error.c error_active 的既有 TLS／MemMap 记录保持。#443 仍须按每个名字的全部声明与函数用途核查，而非只数读写或定义；连接初始化、回调、TLS 段合同及实际消费者后结算。 |
| R8.10 | Required | inline 函数的存储类 | satisfied | 主文 72；AMD2.27 追加 C11，其他修订未改变 static 义务。同号入口检查 isInlineKeyword／isStatic；独立检查实际全部展开中的 inline／__inline／__inline__，自有／生成／patch 均显式 static，包括 Os.h 服务、Os_StackIndex、Ecu_StatusName、Os_Counter.h 换算和 Rte_Os.h 桥；实际调用分别回接 Os_Stack.c、Ecu 入口及 Rte_OsService.c。SDK 原头内部沿原采用责任。 |
| R8.11 | Advisory | 外部链接数组声明的显式大小 | satisfied | 主文 72 只约束非定义声明，AMD2.27 追加 C11；同号入口的 extern [] 检查另由完整实际展开补齐。全部外部数组为 Arti_Events[4096]、Os_ArtiTasks／Stacks／Contexts[16]、Os_InterruptVectorTable[32]，头与定义使用同一容量；Arti.c 发布、Backend 观测和中断分派直接消费。Rte_Type 的 Dcm 数组 typedef、Ecu_MailboxInput 及 Can_TxSinkType 数组形参已另查，无额外外部数组别名声明。 |
| R8.12 | Required | 枚举列表的隐式值唯一性 | satisfied | 主文 73，AMD2.27 追加 C11；同号入口依赖 values／valueType，另按实际全部枚举体独立求值。CanTp 状态、ComM、Dcm 发布、PduR、Ecu_Status 和 OS hook／host 枚举均全显式或初始零后单调递增，隐式项无重复；ARTI／OS 宽度及对齐 sizeof 断言枚举全显式，允许相同值。消费者为相应状态比较／switch，生成 Ecu_Config 值与配置消费一致；无项目 patch 新增隐式冲突。 |
| R8.13 | Advisory | 指针所指对象的可修改性 | not_assessed | 主文 73–74 同时约束指针和数组元素，可因写入或复制到非 const 指针保留可修改类型；核心 constPointer 只给局部候选。当前 Det_Invoke、Dcm_MsgContextType 数据、Can_TxSinkType 数组形参及 OS SDK 输出可定位；#443 恢复须从每个当前实参／数组经别名、赋值与内存复制追到最终写方／可修改接口，结合回调及失败输出合同逐个结算。 |
| R8.14 | Required | 原声明及实际宏展开的 restrict | satisfied | 主文 75，AMD2.27 追加 C11；同号入口只查 raw restrict。物理自有文件及实际全部展开另查 restrict／__restrict／__restrict__、宏及 typedef，无项目或 patch 限定用途；Os_LinuxNative.c 的 memcpy 和 pthread／SDK 声明接缝保持原头，ISO 库内部按 §6.7，非 ISO 原头沿采用责任。未以别名安全或包装创造规则例外。 |
| R9.1 | Mandatory | 自动对象读取之前的赋值路径 | not_assessed | 主文 75–76＋AMD4.56／57 将 _Atomic 交 R9.7；核心 uninitvar 依赖值流和库输出模型。当前 Os_LinuxMappedStack 的 data 仅在 read>0 后访问，previous_low／high 和 used 初始化；count 先赋 read 结果，sp 由实际 asm 输出；新增 block_actor_signals 的 blocked／startup_mask 只在相应初始化成功后消费；其他自动对象含 Windows GetThreadContext／VirtualQuery、Dcm 缓冲与回调输出仍需逐元素／成员核查。#443 恢复须按每条成功／失败／部分输出路径证明首次读取前赋值；Mandatory 保留整改责任。 |
| R9.2 | Required | aggregate／union 初始值的层级 | not_assessed | 主文 77–78 的 {0}／字符串／兼容结构表达式／指定子对象例外有效，AMD4.58 仅追加 R9.6 引用。固定 misra_9 的 902 模型另核真实 Os_ErrorParametersType union、Ecu_OsConfig、Linux timespec／pollfd 与 Can payload；当前指定体保持层级。#443 仍缺全部普通 aggregate、compound literal 及 typedef 数组的类型化子对象映射，须逐对象连接初始值与直接消费者后结算。 |
| R9.3 | Required | 数组的显式元素初始化与例外 | not_assessed | 主文 78–79；{0}、全指定及字符串三类例外保持。固定 misra_9 的 903 模型另核 Can payload[8] 的八项、CanIf errors[2]、Os_Time increments 的 {0}、observed[2] 的两个句柄及 priorities 的单项稀疏指定；索引消费者回接实际 CAN、计数和等待逻辑。#443 恢复须补全部数组／数组子对象及 typedef 数组的容量—元素映射，包含当前应用／配置与 patch 宏容量，逐个应用例外。 |
| R9.4 | Required | 每个子对象的重复初始化 | satisfied | 主文 79–80＋AMD2.49／AMD4.47–50；固定 misra_9 的 904／numInits 模型之外，逐个检查当前展开指定体及子列表。Os_Config 的 tasks／counters 各结构、hooks／time_config／Ecu_OsConfig、Os_ErrorParametersType 的单个 union 成员、Linux timespec／pollfd／process_handle 均无同一子对象重复指定，也无指定回退后的位置覆盖；priorities 仅 [30u]=1u。其余全位置／零初值各消费一次，无 patch 新增重复目标。 |
| R9.5 | Required | 有 designated initializer 数组的显式大小 | satisfied | 主文 80–81＋AMD2.50 仅移除固定节号；固定 misra_9 的 905 模型另核实际指定目标。唯一数组元素 designator 为 interrupts.priorities[OS_INPUT_INTERRUPT]，实际 [30u]，Os_IsrConfigType.priorities[OS_INTERRUPT_VECTOR_COUNT] 显式容量 32；Backend 中断优先级直接消费。tasks[]／counters[] 的 designator 位于各结构元素内部，数组本身用位置式列表；无其他项目／patch 数组元素指定或 flexible-array 初始化。 |
<!-- misra-checker-obligations-end -->

## 当前输入重评

第三阶段第二批先核对 master `40665cd7` 的正常 run 43410036518。
同步 #470／#472 后基于 master `2ae6778d`，最终使用 #470 成功
[run 43411175434](https://github.com/g1331/autsaro/actions/runs/43411175434) 的八份完整工件；
准备日志确认实际检出 `73caf622`（40665cd7 与 8b644773 的测试合并树）。
该树与当前交付源码及 producer 相同，后续词法正文与 CI 分片增量分别重评。
按两目标 Git 换行核验 seal、最终 kernel、92 项 producer、12 个驱动、两个程序的
编译数据库／files.txt／addon 回执及 GCC／Cppcheck 实际输入闭包；888 个程序 TU
实例均为 C99，基线身份及 evaluate 与 CI 保存值相同。原始摘要保持只读。

第二批完成时，完整 144 项为 17 项 satisfied、3 项 not_applicable、124 项 not_assessed。
第三批[表达式核查](expressions.md)结算三项后为 20／3／121，第四批
[控制流核查](control-flow.md)结算十五项后为 35／3／106；本篇原条目状态保持。
本篇为 6 项满足、1 项既有 C99 不适用、11 项阻塞；R7.1 另在 lexical.md 完成
当前条件控制证据并连接 SDK 已批准类别。逐项恢复条件继续归 #443。

当前消费者仅扩充既有 SDK 精确规则选择及 R7.1 token 位置核对；扫描来源与当前
评估代码身份分别核验，重评副本只更新这一个驱动摘要，诊断／程序结果及其余语义
输入保持。本组和 SDK 组分别按当前正文及驱动重绑；其余三组也复核当前驱动变化
后的局部身份与既有具体消费，原批准实体、正式义务及未评估／表外阻塞保持；历史过期的精确记录按当前绑定恢复。
<!-- source-assessment-context-begin -->
{
  "id": "checker-obligations-443-control-flow",
  "evidence": {
    "head": "c93d0aa8145135178b3749f5307360c45fd58b55",
    "receipt": "https://github.com/g1331/autsaro/issues/443#issuecomment-6872195716"
  },
  "cases": [
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "8c1f6f0c2fac87638363a035eaab70dfd21e08e27ed90870dc8a5fc3322b2fc9"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "b038f6333ac1d4eca6f4b61d67da39f0068495f2361b49aea9681051523a6fed"
    },
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "d2121f33163011ec73087e524b3470ef6a6d2e09ae8c9309c7ac3a7aaeb8bae4"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "fbe7ead692e2666bf1cbe71b3b40c2f91804cc3218b6ff2526379dfa80bfbd8a"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "d816d01d63712ac8f14e81b33b7a1cd344b017b913510c9028c49e5c4dc5222d"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "8e82e7c3fbf2b90c5985dde3d682f8f34858b7f3063afaf3299c92b62ac62b3d"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "8ae4c160fa9837086f2bb66aef029dc5c0253f7dfb90c817c371dd51a87629fb"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "34d4b9afb7b6530242e163d7bfa9abc328cf93144f0ac7790eb64cf8563166cd"
    }
  ],
  "records": {
    "kind": "checker-obligations"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
