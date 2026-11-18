# 预处理、库与资源的检查边界

输入、固定工具和责任沿[语言与词法](lexical.md)。同号入口为固定 addon，核心为
`getCppcheckRules` 的准确映射。预处理规则须核对物理指令、实际 define／undef 历史
及展开后的消费者；库规则须区分 ISO 接口、平台接口、采用头和项目自身调用。
R20.2–R20.14 的第五批核查见下；R21–R22 见[库与资源](library-resources.md)。
SDK 实例沿 #444，固定 11 项的 R20.1／R21.8 沿原补充组；本表不批准保留违反。


## 第五批的当前预处理输入

基线 master `755a002e`；输入取正常成功 [run 43928564790](https://github.com/g1331/autsaro/actions/runs/43928564790)
的八份补丁后工程，实际 checkout 为 `75c80eba`。其后交付 C、生成器生产源码和本篇
记录相同；进程取证／测试增量按实际 producer 摘要另核。两目标四 profile 的
HostBatch／probe 共 888 个 C99 TU，依 GCC -MD 八份并集及 patchedLines 调用
现有范围判定：目标自有／生成／应用和补丁行承担本表义务，主机目录与采用原行沿
ADR 0030／0025。SDK 原头沿已登记来源，自有宏／使用点单独核查。

七份固定原件通过原 SHA256SUMS；主文印刷 156–165／PDF 164–173 的全文、例子及
amplification 逐条核对。AMD2.27 的 C11 范围追加、AMD2.77 和 TC2.35 的例子修正
已合成；AMD1／3／4、TC1 没有改变本批义务。当前仍为 C99。

物理扫描覆盖 1,442 个范围内自有文件实例；行拼接保留逻辑行到起始物理行的映射，
去注释保留其中全部换行。每条记录均回查原文件对应指令，再按最终内核的准确
patchedLines 选择，共 1,264 个范围内内核指令位置；条件配对另读取完整文件。头名字、最终 include
形式、关键字宏名、宏参数运算符、有效指令和同文件条件栈分别核查。GCC 实际展开
与依赖回执连接入口函数和 ARTI 记录消费者，生成物另回接 ecu.rs、arti.rs 及模板。

第五批结算 8 条 satisfied，5 条转入 [#536](https://github.com/g1331/autsaro/issues/536)：
R20.5／R20.10 有真实 Advisory 违反及待补处置，R20.7／R20.8／R20.9 尚缺完整
语法／时序证据。#536 逐条列出恢复条件。固定工具原始诊断、passed／error、程序退出码、221 项
assessment 和 completeCompliance 保持原义。

## 第六批的逐事件取证

`c-check` 在当前封存 ECU 分析中保留 `preprocessing.json.gz`。采集复用 CPPcheck
输入准备时取得的 GCC 带行标记输出与 `-Wundef` 诊断。它在临时副本中让同一目标 GCC
报告受评估的每次 `#if`、`#elif`，并核对探针前后输出与实际文件进入、返回顺序。
原分析副本和编译产物照常使用。CI 的完整分析工件包含逐 TU 的展开摘要、物理文件及行、
条件展开、当时的定义来源和条件值。宏状态取自按顺序出现的 define／undef，重复
include 保留每次进入。收据在摘要中另列，采集错误由 CI 报出。

GCC 按 C99 预处理整数模型判别 0、1 与其余值，复用 R7.2 的整数控制。在原
`#elif` 位置进入探针须先耗尽此前分支，外层排除的条件也不进入探针。用原 GCC
`-Wundef` 诊断检查确实求值的未定义名字，再核对展开后仍存的标识符是否由其同名
`defined(B)` 保护。条件文本、命令行与预定义宏分别留来源。夹具覆盖 2U、命令行
宏、版本预定义宏、重复包含之间的撤销重定义、受保护与未保护的缺失名和跳过的分支。

Linux 本地先按 49cd7b2a 的交付生产代码复用 PR #558 封存样本及实际 GCC 输入。
下表按每份分析中的唯一 TU 计数。纳入的条件值均为 0 或 1。每个 profile 各有 260 个
条件事件的展开仍含未定义标识符，分别回查 19 种表达式的同名 `defined` 短路保护，
GCC 对实际求值无 `-Wundef` 报告。两平台验收使用本 PR 的冻结 CI 收据。

| Linux profile | TU | 条件求值 | MemMap 请求撤销 | MemMap 内部状态／属性撤销 |
| --- | ---: | ---: | ---: | ---: |
| standard-ecu | 56 | 36,933 | 2,452 | 3,281 |
| user-application | 56 | 36,933 | 2,452 | 3,281 |
| component-pipeline | 58 | 43,870 | 2,538 | 3,410 |
| no-diagnostic | 58 | 39,759 | 2,500 | 3,352 |

在 [PR #558 的八份基准工件](https://github.com/g1331/autsaro/actions/runs/44285205267)
中，R20.5 范围内物理位置全部落在交付映射头内。按标准 ECU、用户应用、组件流水线、
无诊断的顺序，START／STOP 请求撤销分别是 152、152、172、160 处，配对段属性与
状态撤销是 188、188、218、200 处。每条内部撤销都回查到同一头、同一类的 START
分支定义。源码验收核对交付来源、实际 TU 包含闭包和成对请求，再将相应源位置连接到
[ADR 0025 类别记录](../../memmap/README.md#memmap-类别偏离)及当前 owned-source 绑定。
本地事件中，请求由包含者定义，内部状态／属性由同一映射头定义。
FreeRTOS 采用原行上的三个 wrapper marker 和 Windows 主机头的四处名字隔离分别
维持原归属。这里的物理诊断数包括分析器额外配置，实际经过的请求由逐 TU 事件核对。

同八份工件每份有 77 个范围内函数式宏定义位置。48 个 ErrorHook getter 和
`traceTASK_SWITCHED_IN` 无形式参数。四个 Declare 宏丢弃标识符，`ALARMCALLBACK`
把实参放在函数声明标识符位置。Cppcheck 每份在 `Os.h:120` 的 R20.7 报告保留，
`entry-bodies` 的 `ALARMCALLBACK(AlarmBody)` 展开为声明／定义，行为由入口调用验证。
ARTI 的两载荷、RTE 的状态掩码、计数器转换、断言与资源创建的表达式替换使用独立括号。
项目内核补丁的 `taskOS_SELECT_READY` 在 `listGET_OWNER_OF_NEXT_ENTRY` 中用
`(pxTCB)`、`(pxList)`，另一变体在 `listGET_OWNER_OF_HEAD_ENTRY` 和
`traceOS_TASK_SELECTED` 中保护列表表达式及下次调度前的当前任务。
`taskOS_INSERT_READY` 的单实参分支在 `prvOsInsertReadyTask( task )` 的括号内，
另一变体经 `listINSERT_END` 展开在成员访问与列表下标处保留括号。
本轮最终结论随两平台的实际展开收据绑定。

在 #536 用户决定下保留 [四个 OS 入口宏与 ARTI_TRACE](../../bsw-general/published-information.md#构造宏的字面入口绑定)。
R20.10 为真实 Advisory 违反。早期批准锁定的五个定义、展开及直接消费者继续适用。
此批以当前两目标四 profile 的冻结输入重核既有裁决，范围仍限这五个定义。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R20.2 | Required | include 的 header-name 字符 | satisfied | 主文 156／PDF 164；AMD2.27 追加 C11。按当前物理 include 的 header-name 内部核查，八输入共 663 种文件／指令组合均为单一直接引号或尖括号形式，名字无单引号、双引号、反斜杠、斜杠星号或双斜杠；定界引号单独剥离。含 0017 的正斜杠 FreeRTOS 路径、生成 Rte／MemMap 头及其 producer；固定 addon 的逗号检查与本条正文分开。 |
| R20.3 | Required | include 的最终参数形式 | satisfied | 主文 156–157／PDF 164–165；AMD2.27 追加 C11。逐条核对当前范围内 include，全部为直接单一 header-name，无宏间接包含或多字符串拼接；预处理后形式保持原引号／尖括号。generated/include 的模块映射头、bsw／os 头、应用及 0017 修改行均已核对，实际 GCC／Cppcheck 文件依赖回接目标 TU；未用 addon 跳过间接 include 的分支推出结论。 |
| R20.4 | Required | define 的宏名与关键字 | satisfied | 主文 157／PDF 165，包括语言扩展关键字；AMD2.77 仅修正 inline 示例，AMD2.27 增加 C11。按全部物理 define 的完整宏名，独立比较 C99／C11 关键字及实际 GCC asm、typeof、attribute、inline、restrict、volatile、thread 等扩展拼写，无同名定义。OS_TASK_ENTRY／TASK／ISR 和 ARTI_TRACE 是普通名字；采用 SDK／内核原件的宏沿 ADR 0025，项目补丁另查。 |
| R20.5 | Advisory | 全部物理 undef 与实际撤销用途 | not_assessed | 主文 158／PDF 166；AMD2.27 增加 C11，未给 MemMap 语言例外。当前真实 Advisory 违反集中在模块／Rte MemMap 的 START／STOP、段属性与 active 状态撤销；关键词沿 ADR 0025 的既有类别，内部属性与状态分别沿原 MemMap 机制记录。#536 承接本条全部请求／撤销时序与精确处置覆盖，继续归 #443；Os_Windows.h 已属范围外，采用内核原行另沿 ADR 0030。 |
| R20.6 | Required | 预处理指令与宏实参的交织 | satisfied | 主文 158–159／PDF 166–167；AMD2.27 追加 C11。原 token 按逻辑指令和括号栈核查所有范围内文件，含未选分支；唯一指令位于未闭合括号的候选是 tasks.c::eTaskGetState 的普通 if 控制括号，由 0003 activation-ready-policy 插入，前导 token 为 if，未处于宏实参。所有范围内宏替换表的括号均配平，无对象宏单独打开实参的路径；自有／生成及项目补丁没有宏实参内指令。核心 preprocessorErrorDirective 名称未作为完整证明。 |
| R20.7 | Required | 宏参数展开表达式的括号 | not_assessed | 主文 159–160／PDF 167–168；AMD2.27 追加 C11。按完全展开后的表达式核查，原文允许实参自带括号，声明／成员／拼接用途分开。ARTI_TRACE 两数值参数均括号保护，TASK／ISR 参数组成函数名；ErrorHook getter、声明／类型宏和项目补丁的完整实参到展开语法映射仍缺，#536 承接逐参数与消费者证据，继续归 #443；固定字符串启发式检查保留。 |
| R20.8 | Required | #if／#elif 的最终控制值 | not_assessed | 主文 160–161／PDF 168–169；AMD2.27 追加 C11。只要求实际求值的条件结果为 0 或 1，排除分支中未求值条件按原 amplification 处理。当前版本比较与 MemMap defined 条件已定位，但普通 -E 消去条件，尚缺完整求值事件、短路与 intmax_t／uintmax_t 控制；#536 与条件整数篇接续该捕获及输出一致性证据，继续归 #443。 |
| R20.9 | Required | 条件表达式中的已定义标识符 | not_assessed | 主文 161／PDF 169；AMD2.27 追加 C11。命令行／预定义宏可建立定义，defined 操作数与 defined(B) 且 B 的原文例子分别处理。当前 -D、版本头和 MemMap define／undef 链可定位，尚缺每个实际求值时刻的活动宏、剩余标识符及短路关联；#536 承接逐事件证据，继续归 #443，原 addon 对下划线名字的缺口保留。 |
| R20.10 | Advisory | define 中的 #／## 运算符 | not_assessed | 主文 162／PDF 170；AMD2.27 追加 C11。真实 Advisory 违反为 Os.h 的 TASK／OS_TASK_ENTRY／ISR／OS_ISR_ENTRY（##）及 Arti.h 的 ARTI_TRACE（四次 #）。前者由 ecu.rs／Ecu_Config.c.in 同名绑定生成入口，后者由 arti.rs 生成 hook 调用并交 Arti_Record 保存字面身份；原精确决定和 #536 本轮决定均限这五宏；已登记，当前逐 TU 证据与冻结绑定待两平台收据，继续归 #443。 |
| R20.11 | Required | 宏参数的 # 后紧接 ## | satisfied | 主文 162／PDF 170；AMD2.27 追加 C11。行拼接、注释移除后逐 token 核查五个带运算符的自有宏：ARTI_TRACE 仅有四个 # parameter；四个 OS 入口宏仅有 prefix ## identifier，没有 # parameter ## 序列。范围内项目补丁和其他宏无 #／## 运算符；字符串中的 # 单独排除。实际入口及 ARTI 调用展开分别回接函数绑定和 Arti_Record。 |
| R20.12 | Required | 同一参数在 paste 与其他用途间的展开 | satisfied | 主文 163／PDF 171；AMD2.27 追加 C11。不依赖实参是否恰好为宏：ARTI_TRACE 的 context／class_name／instance／event 在替换表中只作 # 操作数，两数值参数只作括号表达式；TASK／ISR 的重复 identifier 每次均只作 ## 操作数，OS_TASK_ENTRY／OS_ISR_ENTRY 同样。无参数混用操作数与普通替换；生成入口／hook 和实际展开保持此分工，项目补丁无其他候选。 |
| R20.13 | Required | 全部物理预处理指令 | satisfied | 主文 163–164／PDF 171–172；AMD2.27 追加 C11。对范围内物理逻辑行先去注释，再检查首 token #，含未选分支；仅 include／define／undef／if／ifdef／ifndef／elif／else／endif／error，无 warning、未知指令或非标准首 token。所有 include 形式、define 名／参数及条件结构分别核查，正常 C99 编译补活动语法；未借 addon 额外接受 warning 的白名单结算。 |
| R20.14 | Required | 条件指令配对的物理文件身份 | satisfied | 主文 164–165／PDF 172–173；AMD2.27 追加 C11，TC2.35 只修正示例末尾文件名。逐物理文件重建完整 if／ifdef／ifndef 栈，else／elif 关联当前栈顶，endif 弹栈，全部文件末栈为空，含未选分支及项目补丁上下文。MemMap 多次包含各自完整配对，未跨文件借 endif；独立栈避免固定 addon 正常同文件 endif 未 pop 的实现缺口。 |
<!-- misra-checker-obligations-end -->
