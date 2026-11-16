# 预处理、库与资源的检查边界

输入、固定工具和责任沿[语言与词法](lexical.md)。同号入口为固定 addon，核心为
`getCppcheckRules` 的准确映射。预处理规则须核对物理指令、实际 define／undef 历史
及展开后的消费者；库规则须区分 ISO 接口、平台接口、采用头和项目自身调用。
R20.2–R20.14 的第五批核查见下；R21–R22 的额外人工范围继续归 #443。
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

本批结算 8 条 satisfied，5 条转入 [#536](https://github.com/g1331/autsaro/issues/536)：
R20.5／R20.10 有真实 Advisory 违反及待补处置，R20.7／R20.8／R20.9 尚缺完整
语法／时序证据。#536 逐条列出恢复条件；本表保留 5 条 not_assessed。
R20.10 的五宏保留尚待用户决定，本批新增批准为零。固定工具原始诊断、passed／error、
程序退出码、221 项 assessment 和 completeCompliance 保持原义。

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
| R20.10 | Advisory | define 中的 #／## 运算符 | not_assessed | 主文 162／PDF 170；AMD2.27 追加 C11。真实 Advisory 违反为 Os.h 的 TASK／OS_TASK_ENTRY／ISR／OS_ISR_ENTRY（##）及 Arti.h 的 ARTI_TRACE（四次 #）。前者由 ecu.rs／Ecu_Config.c.in 同名绑定生成入口，后者由 arti.rs 生成 hook 调用并交 Arti_Record 保存字面身份；原 OS configuration／platform／arti 篇只有技术保留理由。#536 承接用户决定及当前精确处置，继续归 #443；本批未批准新例外。 |
| R20.11 | Required | 宏参数的 # 后紧接 ## | satisfied | 主文 162／PDF 170；AMD2.27 追加 C11。行拼接、注释移除后逐 token 核查五个带运算符的自有宏：ARTI_TRACE 仅有四个 # parameter；四个 OS 入口宏仅有 prefix ## identifier，没有 # parameter ## 序列。范围内项目补丁和其他宏无 #／## 运算符；字符串中的 # 单独排除。实际入口及 ARTI 调用展开分别回接函数绑定和 Arti_Record。 |
| R20.12 | Required | 同一参数在 paste 与其他用途间的展开 | satisfied | 主文 163／PDF 171；AMD2.27 追加 C11。不依赖实参是否恰好为宏：ARTI_TRACE 的 context／class_name／instance／event 在替换表中只作 # 操作数，两数值参数只作括号表达式；TASK／ISR 的重复 identifier 每次均只作 ## 操作数，OS_TASK_ENTRY／OS_ISR_ENTRY 同样。无参数混用操作数与普通替换；生成入口／hook 和实际展开保持此分工，项目补丁无其他候选。 |
| R20.13 | Required | 全部物理预处理指令 | satisfied | 主文 163–164／PDF 171–172；AMD2.27 追加 C11。对范围内物理逻辑行先去注释，再检查首 token #，含未选分支；仅 include／define／undef／if／ifdef／ifndef／elif／else／endif／error，无 warning、未知指令或非标准首 token。所有 include 形式、define 名／参数及条件结构分别核查，正常 C99 编译补活动语法；未借 addon 额外接受 warning 的白名单结算。 |
| R20.14 | Required | 条件指令配对的物理文件身份 | satisfied | 主文 164–165／PDF 172–173；AMD2.27 追加 C11，TC2.35 只修正示例末尾文件名。逐物理文件重建完整 if／ifdef／ifndef 栈，else／elif 关联当前栈顶，endif 弹栈，全部文件末栈为空，含未选分支及项目补丁上下文。MemMap 多次包含各自完整配对，未跨文件借 endif；独立栈避免固定 addon 正常同文件 endif 未 pop 的实现缺口。 |
| R21.1 | Required | 保留宏标识符的 define／undef | not_assessed | 同号入口以正则检查部分 _ 名和标准库名字。#443 须核对完整保留名称规则、库头条件和当前活动定义／撤销，区分项目与 SDK 来源，并合成修订及例外。 |
| R21.2 | Required | 保留标识符的声明 | not_assessed | 同号入口检查 builtin 宏名和标准库函数名字，覆盖小于全部对象／typedef／tag 的保留名称义务。#443 尚缺完整 namespace／scope 及标准头关联核查，接续实际自有／patch 声明与修订。 |
| R21.3 | Required | 标准动态分配接口与直接调用 | not_assessed | 同号入口按已识别调用名匹配 malloc／calloc／realloc／free。#443 须核对函数指针／宏、其他标准分配接口及适用修订，连接[主机动态设施](../../os/native-facilities/storage-interrupts.md)的原实例和实际释放闭包。 |
| R21.4 | Required | setjmp.h 的设施使用 | not_assessed | 同号入口只通过 findInclude 查找 <setjmp.h>。#443 须核对传递包含、引号形式、宏和实际 setjmp／longjmp 消费者，区分已采用的展开／平台异常机制，合成有效原文与修订。 |
| R21.5 | Required | signal.h 的设施使用 | not_assessed | 同号入口只查 <signal.h> include。#443 须核对 signal／raise 等 ISO 设施与 POSIX sigaction／宿主信号机制的实际边界，连接[主机设施](../../os/native-facilities/README.md)的调用者和修订原文。 |
| R21.6 | Required | 标准 I/O 设施的实际使用 | not_assessed | 同号入口用 stdio.h／wchar.h 名录过滤直接调用，并跳过部分有 function 标记的 token。#443 须核对间接调用、宏、类型／对象及非 ISO 平台 I/O 的责任，连接 HostBatch 读写及有效修订。 |
| R21.7 | Required | atoi 家族转换 | not_assessed | 同号入口匹配 atof／atoi／atol／atoll 直接调用。#443 须核对别名／函数指针、展开后名字及项目当前转换入口，按原文与修订逐用途核定；错误检查由实际替代接口承担。 |
| R21.9 | Required | bsearch／qsort 的使用 | not_assessed | 同号入口只查两个名字后紧接左括号的 token。#443 须核对宏、取址后间接调用及实际排序／查询消费者，合成原文和适用修订并确认当前输入。 |
| R21.10 | Required | 标准日期／时间设施 | not_assessed | 同号入口查 <time.h> 和 wcsftime 直接调用。#443 须核对传递包含、类型、宏、函数指针和 POSIX clock_gettime 等非 ISO 用途边界，连接实际时钟消费者及修订原文。 |
| R21.11 | Advisory | tgmath.h 泛型设施 | not_assessed | 同号入口只查 <tgmath.h>。#443 须将既有 C99／无泛型源码证据接到当前所有实际包含及宏展开，核对原文和修订；新增 C11 泛型规则仍归原 66 项，不在本组重复。 |
| R21.12 | Required | 浮点异常状态设施 | not_assessed | 同号入口仅在检测到 <fenv.h> 后检查 fexcept_t 和五个已知函数。#443 须核对传递包含、宏／间接调用、类型及当前浮点消费者，并合成有效原文与适用修订。 |
| R21.13 | Mandatory | ctype 参数的合法值域 | not_assessed | 核心 invalidFunctionArg 依赖库模型和值流。#443 须逐当前 ctype 调用证明为 EOF 或 unsigned-char 可表示值，包含 char 符号性、宏与间接调用，合成修订；Mandatory 缺口保持阻塞。 |
| R21.14 | Required | memcmp 与空终止字符串 | not_assessed | 同号入口通过部分 str*／sprintf 等调用推断 string buffer，再检查 memcmp 的 varId／literal。#443 须核对别名、结构成员和真实字符串用途，按原文允许条件逐调用证明，合成修订。 |
| R21.15 | Required | 内存复制／比较的两端指针类型 | not_assessed | 同号入口对 memcpy／memmove／memcmp 比较 valueType.type，任一 void 即跳过。#443 须按实际指向类型、限定符、结构身份和宏／间接调用核查两端，合成原文例外与修订。 |
| R21.16 | Required | memcmp 比较对象的 essential 类型 | not_assessed | 同号入口接受 signed／unsigned／bool／enum 类别，跳过多层指针及未知类型。#443 须核对当前实际对象、填充和数组元素类型，按原文及修订逐用途判断，连接最终比较结果消费者。 |
| R21.17 | Mandatory | 字符串库访问的对象边界 | not_assessed | 核心 bufferAccessOutOfBounds 依赖尺寸和值流。#443 须逐真实字符串参数证明容量、终止符及读取／写入界限，覆盖动态长度、失败返回及别名，核对有效原文和修订。 |
| R21.18 | Mandatory | size_t 长度参数的适当取值 | not_assessed | 核心同用 bufferAccessOutOfBounds；#443 须按本条独立追踪当前 memcpy／比较／字符串接口的长度来源、对象大小及终止要求，包含零长度和目标类型转换，合成修订。 |
| R21.19 | Mandatory | 标准库内部存储的修改 | not_assessed | 同号入口主要检查若干返回指针赋值到非 const 指针及 lconv 成员赋值。#443 须追踪别名、返回链和实际写入者，逐内部对象证明未修改，合成原文及修订；Mandatory 保持逐用途责任。 |
| R21.20 | Mandatory | 标准库内部存储被后续调用失效 | not_assessed | 同号入口用局部 assigned／invalid 表，遇到花括号、return 等即清空。#443 须跨分支／函数及别名核对当前返回存储的最后使用和下一失效调用，按原文及修订证明生命周期。 |
| R21.21 | Required | system 命令执行 | not_assessed | 同号入口只检查名字为 system 且单参数的直接调用。#443 须核对宏、函数指针及当前宿主 API 用途与原文范围，合成适用修订；工具测试中的进程启动与交付 C 各按真实输入判断。 |
| R22.1 | Required | 动态资源的释放闭包 | not_assessed | 核心 memleak／resourceLeak 等报告可建模路径。#443 须核对当前分配／获取到最终释放、转移所有权、取消与失败路径，连接主机对象和补丁资源，合成有效原文修订。 |
| R22.2 | Mandatory | 释放对象的分配来源 | not_assessed | 核心 autovarInvalidDeallocation 检查部分非法释放。#443 须逐当前释放证明来源、基址及配对 provider，覆盖别名、栈／静态对象及所有权转移，合成原文和修订。 |
| R22.3 | Required | 同一文件的同时打开 | not_assessed | 核心 incompatibleFileOpen 依赖文件名／模式模型。#443 须核对当前路径身份、同文件别名及各打开模式和关闭时序，连接直接读写消费者，合成原文与修订。 |
| R22.4 | Mandatory | 只读流上的写操作 | not_assessed | 核心 writeReadOnlyFile 给出已知模式冲突。#443 须追踪当前 FILE／平台句柄别名、实际打开模式到每个写入接口，核对宏／间接调用及修订；Mandatory 保持原义。 |
| R22.5 | Mandatory | FILE 指针的解引用 | not_assessed | 同号入口只检查 *／成员访问且变量类型起始为 FILE * 的形式。#443 须核对 typedef、多级别名、宏和所有当前访问路径，合成原文及适用修订，SDK 内部实现沿采用责任。 |
| R22.6 | Mandatory | 已关闭流指针的使用 | not_assessed | 核心 useClosedFile 依赖流状态和函数模型。#443 须追踪别名、跨调用传递、关闭失败与生命周期终点，核对每个当前消费者及有效修订；平台句柄另按实际契约处理。 |
| R22.7 | Required | EOF 比较及未修改的 int 返回值 | not_assessed | 同号入口只识别 EOF 比较及 bool／char／short 的兄弟表达式类型，未追踪返回值在比较前被修改的全部路径。#443 须按当前 I/O 返回链、宽度和有符号性独立核对，并合成原文修订。 |
| R22.8 | Required | errno-setting 调用前的清零 | not_assessed | 同号入口只按 token 顺序识别 errno = 0 和固定函数表。#443 须核对 errno 宏实际展开、分支、间接调用及所选平台哪些函数设置 errno，追踪当前调用前状态并合成修订。 |
| R22.9 | Required | errno-setting 调用后的检查 | not_assessed | 同号入口在分隔符后寻找 errno 比较，缺少一般控制流和实际成功／失败语义证明。#443 须核对当前每次相关调用的检查时点、中间调用及返回值，合成有效原文与修订。 |
| R22.10 | Required | errno 比较的有效来源 | not_assessed | 同号入口保存最后一个已识别函数名，遇到右花括号清除。#443 须追踪当前真实调用、条件分支、宏和直接消费者，确认每次 errno 比较受相应契约支持，并合成修订原文。 |
<!-- misra-checker-obligations-end -->
