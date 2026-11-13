# 语言、词法与名称的检查边界

本篇承接[语言构造](README.md)。固定工具为 Cppcheck 2.21.0／e73bf44，
`misra.py` 的 SHA256 为 `d1f7947b6c2c6f3980314155d6a89f0e6a9435456fb3a02ac1ded25a99af3f44`。
表中同号入口指该文件的 `misra_N_M` 及其执行调用，核心入口指
`getCppcheckRules` 的诊断映射。工具能力与当前人工结论分别记录。

## 当前输入与核查方法

第三阶段第一批基于 master `40665cd7`，复用其正常
[Development checks 43410036518](https://github.com/g1331/autsaro/actions/runs/43410036518)
的八份完整分析：Linux／Windows、standard-ecu／user-application／component-pipeline／
no-diagnostic，各含 C99 HostBatch／probe。逐份验证非内核 seal、最终 kernel、92 项
producer、12 个分析驱动、程序编译数据库、addon 回执及 GCC／Cppcheck 输入闭包。
16 个程序共有 888 个 TU 实例；独立的 GCC 展开分别为 Linux 228、Windows 224 份。
原摘要保持只读，扫描结果和本组正文的当前评估身份分别绑定。

核查包含所有交付 C／头文件的物理字节、未选分支和宏定义；先处理 C 行拼接，再区分
注释、带前缀的字符串／字符、完整 pp-number 与标识符。实际展开按 GCC 行标记回接
project，kernel 以 `patchedLines` 分开项目修改和采用原行。名称前缀比较使用 C99 的
63 字符最低保证，对原文件及项目使用点的展开做全局保守比较；宏另按实际
`define`／`undef` 顺序与当时的参数集合比较。标签区分 switch 的 case／default；位域
逐个排除条件表达式冒号，并追到声明类型及读写方。SDK 内部和固定内核原件按
[ADR 0025](../../../adr/0025-misra-layered-scope.md)分层，展开在项目使用点的义务继续核查。

本次重新阅读主文 R1.2–R7.4 的正文、例外与示例（印刷 38–63／PDF 46–71），
并逐份检索、合成 AMD1–4／TC1–2；七份原件分别通过原 SHA256SUMS。
适用修订中，AMD2.27 增列 C11，当前仍采用 C99；R1.3 沿 AMD2.29 及修订后的
Appendix H；R2.2 合入 TC1 的初始化说明、TC2.3–4 的已用转换例外和 AMD4.37；
R2.5 合入 TC1 的 undef 用途与 TC2.5–6 的定义用语；R2.7 合入 AMD4.38；
R3.1 合入 AMD4.39–41 的 URI 例外。名称条目合入 AMD2.35–38，R5.9 合入 TC1
的主体更正；R6.1／R6.2 合入 AMD2.41–45；R7.2 合入 AMD2.46；R7.4 合入
AMD2.47–49 和 TC2.7–8 的可变实参例外。其余修订未改变本表条目的 C99 判据。

以下满足结论只消费所选输入的必要人工证据。剩余条目的正文合成已完成，恢复条件
集中在尚缺的实体、状态或消费者证明；原工具诊断和 221 项 assessment 保持原义。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R1.2 | Advisory | C99 扩展及实际消费者 | not_assessed | 同号入口只识别省略中项条件表达式和 statement expression。当前 Os_MemMap／Os_HostLinux／Os_LinuxNative 的属性、TLS、原子、asm、alignof 用途沿 [OS 平台](../../os/platform.md)；实际源码另有 __builtin_trap 和 sanitizer fiber 接口。#443 仍须逐 TU 将这些使用及项目 port 扩展连接具体编译器保证、设计理由和直接消费者，补齐平台篇未逐实例覆盖的部分后重评。 |
| R1.3 | Required | 完整程序的未定义及关键未指定行为 | not_assessed | 核心映射为大部分 error 类；有效义务沿主文 38–39、AMD2.29 和修订 Appendix H。当前 Os_Backend 的 actor／句柄、Os_Stack 的异常上下文及 Dcm 请求缓冲都有跨调用存活期，词法与值流结果只提供局部证据。#443 仍须按适用行为连接对象生命周期、指针范围、求值顺序、失败释放和并发消费者，逐项说明其他规则证据及余下行为。 |
| R2.1 | Required | 各程序的可达路径 | not_assessed | 核心 alwaysFalse／duplicateBreak 检查局部结构；主文 39–41 排除预处理删除的代码。Dcm_StartOfReception→Dcm_CopyRxData→Dcm_MainFunction、生成 Ecu_Execution／SchM_Main 和各 profile 的回调入口仍需分别建调用闭包。#443 须按 HostBatch／probe 核对活动配置与防御分支，确认每段可达性或登记具体违反；公开签名和编译入 TU 只定位入口。 |
| R2.2 | Required | 可观察输出和持久状态的生产路径 | not_assessed | addon 跳过宏并只识别部分恒等运算，核心补充冗余写入／条件。有效正文允许已用转换、void 转换且将初始化单列；语言扩展操作按正文视为有行为影响。当前 Os_Arti 状态发布、ComM 工作状态及 [40 处 guard](../essential-types/expressions.md)仍需逐写入连接直接读方及时间效果。#443 须证明删除候选操作会改变哪项输出，或沿 producer 修复实际死操作。 |
| R2.3 | Advisory | 全程序 typedef 定义与使用 | not_assessed | 同号入口保存 typedef CTU 摘要。Platform_Types 的 float32／float64／VoidPtr 及生成 RTE 类型已有 [具体记录](../public-types.md)，本次保留其原范围；每个程序完整 TU 已核对。#443 仍须把当前每个 typedef 实体及实际使用、标准保留或采用责任分别对照，特别是新增 ComM／应用类型，形成完整实体结算。 |
| R2.4 | Advisory | 全程序 tag 定义与实际引用 | not_assessed | 同号入口保存 tag CTU 摘要，tag 自身用途与其 typedef 的用途分别判断（主文 43）。既有三处 xSTATIC_* tag 删除及 LSduR／PduR 匿名 enum 误报沿 [声明用途](../declarations.md)。#443 仍须按当前真实定义／头身份复核其余 tag 与项目补丁；不能以 typedef 被用或 CTU 无报告代替 tag 引用证明。 |
| R2.5 | Advisory | define／undef 历史与实际宏使用 | not_assessed | TC1 明确 undef 算使用，固定入口的相应分支符合这项更正；其同名合并及后定义覆盖仍会丢失定义实例。当前组件头的 Rte_InitValue_Value_Value／RTE_RUNNABLE_Periodic 具有独立定义，[现有用途证据](../declarations.md)保留原输入。#443 须按两个程序各自的定义实例核对实际展开、条件或 undef，逐项承接未使用定义及生成保留，补齐当前 ComM／RTE 增量。 |
| R2.6 | Advisory | 函数内 label 与实际跳转 | satisfied | 主文 44 的对象是函数内标签。固定核心 CheckOtherImpl::checkUnusedLabel 跳过 expanded macro，故另检查全部当前原 token 和项目使用点实际展开：没有函数 label／goto；Dcm.c、Os_Backend.c、Os_Arti.c 等的 default 属 switch。项目补丁也无标签；当前双程序不存在需补跳转消费者的标签实体。 |
| R2.7 | Advisory | 函数定义的形参与调用契约 | not_assessed | 同号入口按函数体变量引用扣除形参；有效标题合入 AMD4.38。当前生成 Det_Callouts／Ecu_OsHooks 以及主机回调含固定签名和显式丢弃，相关 [公共接口](../../bsw-general/public-api-os.md)处置保留原实体范围。#443 仍须逐定义核对参数用途、实际丢弃及必要签名；仅引用或显式丢弃的存在不足以结算全部回调合同。 |
| R3.1 | Required | 所有自有／patch 物理注释与行拼接 | satisfied | 主文 45 及 AMD4.39–41 允许行注释中的 // 和 scheme://path URI。独立扫描所有物理文件及行拼接后的注释，项目注释无其余嵌套起始序列；完整 kernel 的候选均回到采用原行。版本头、MemMap、RTE 头与项目 port 补丁的注释均在本次集合中；不依赖 addon 的 URI 启发式。 |
| R4.1 | Required | 原文件和实际展开的带前缀字符／字符串 | satisfied | 主文 46–47 要求数值转义终止于下一转义或该字面量末尾，AMD2.32 删除 See also。独立识别单个字面量及转义边界，覆盖宏定义、行拼接、L 前缀及实际展开；所有自有／patch 数值转义均正确终止，未发现 SDK 宏在项目使用点引入额外候选。Os_LinuxNative 的行缓冲终止符及 HostBatch 的输入／输出字面量保持原字符值。 |
| R4.2 | Advisory | 预处理之前的全部自有／patch 物理字节 | satisfied | 主文 47 的九种三字符组适用于任意位置。逐文件检查拼接前原字节，包含注释、字符、字符串、宏定义和未选分支，均无三字符组；当前所有交付 C／头及最终 kernel 项目行已纳入。该证据补齐同号入口只检查双引号字符串的范围。 |
| R5.2 | Required | 当前同 scope／namespace 的有效名称 | satisfied | 主文 49–50、AMD2.35 的 C99 最低区分长度为 63。对全部物理标识符及实际展开的项目使用点做跨 scope 保守前缀比较，无不同拼写共用 63 字符前缀；同名声明约束由两目标实际 C99 编译检查。包含 ComM 头、所有生成 RTE 头与项目 patch；两者均外部链接及宏的例外仍分别沿 R5.1／R5.4／R5.5。 |
| R5.3 | Required | 同 namespace 的嵌套 scope 遮蔽 | not_assessed | 核心 shadowVariable 给出变量候选；主文 50–51、AMD2.36 限同 namespace，tag／成员等不同 namespace 依各自 scope 判断。本次前缀核查消除了不同长拼写的候选，同名 typedef、枚举常量、形参与局部对象仍需语义归属。#443 须按每个实际 TU 的声明 scope 核对内外 owner 及其消费者，包含 ComM 新函数和宏产生的声明。 |
| R5.4 | Required | 活动宏名及各自参数名 | satisfied | 主文 51–52、AMD2.37–38 按定义时的活动集合和 C99 63 字符判断。原文件所有宏／参数的保守前缀比较无冲突；实际 -dD 顺序重放 define／undef，逐次核对项目宏与已定义宏／参数及本宏参数，亦无冲突。包含反复撤销的 Os_MemMap、RTE_CORE 下的应用宏及 port 宏；同号入口未清 undef 的限制由该独立核查补齐。 |
| R5.6 | Required | 跨 namespace／TU 的 typedef 名称 | not_assessed | 主文 54 的关联 tag 例外和同头重复包含条件保持，入口保存 typedef CTU 摘要。当前 Rte_Instance 在不同组件头的定义、Platform_Types 的公共类型和 SDK 的 __sighandler_t 仍须保留真实声明来源；[历史调查](../declarations.md)只有候选比较。#443 须对两个程序分别完成跨 namespace 实体比较，逐项验证同头／关联 tag 条件及项目使用的 SDK 接口，不能仅按同名或相同类型合并。 |
| R5.8 | Required | 外部链接名称与全部其他实体 | not_assessed | 主文 56 要求外部定义名字在所有 namespace／TU 唯一，同号入口只保存 identifier CTU 摘要。本次 TU 闭包和前缀核查已完成；ComM_GetCurrentComMode 等导出、Rte／SchM 入口、局部无链接实体和宏声明仍需逐定义配对。#443 须分别核对 HostBatch／probe 的完整实体集合、条件定义与直接消费者，补出跨 namespace 的语义比较。 |
| R5.9 | Advisory | 内部链接名称与全部其他实体 | not_assessed | 主文 57–58 及 TC1 的主体更正约束内部链接定义名字；同头 static inline 有明确例外。Os_Service／Ecu_Status 的 inline 与各 C 私有函数、局部对象须按实际声明头身份比较；同号入口只保存 identifier CTU 摘要。#443 仍须逐程序完成内部实体及其余 namespace 比较，逐个识别同头 inline 例外，保留采用原件与项目补丁归属。 |
| R6.1 | Required | 项目位域及实际声明类型／消费者 | satisfied | 主文 58–59、AMD2.41–43 允许 C99 实现支持的显式有符号／无符号类型及 bool，可经 typedef。当前物理与实际展开的项目位域仅 Dcm_MsgAddInfoType 的 reqType／suppressPosResponse；两目标 uint8→uint8_t→unsigned char，实际 GCC C99 编译接受。Dcm 接收初始化／子功能提取写入，响应抑制读取后者；无项目 union 位域或其他 typedef 位域候选。 |
| R6.2 | Required | 有名的一位位域 | satisfied | 主文 59、AMD2.44–45 区分有名与无名位域。Dcm_MsgAddInfoType 的 reqType／suppressPosResponse 均有名、宽度 1，实际 typedef 展开为 unsigned char；Dcm 的 0／1 写入和抑制判断沿 R6.1 的直接消费者。全部物理及实际展开已核对，项目补丁无新增位域，未发现有符号一位对象。 |
| R7.1 | Required | 原 token、条件和实际展开整数 | satisfied | 主文 59–60／PDF 67–68、AMD2.27；固定 raw token 入口遗漏宏后八进制。当前实际展开仅 Linux Os_LinuxMappedStack 的 open 实参 O_RDONLY→00，按 #476 批准接入 sdk-integration 精确类别、glibc 原头及 HostBatch／probe；保持只读、descriptor／read／close 检查。物理项目／patch 的 #if／#elif 与全部实际 -dD 定义作保守传递闭包，无八进制 token 或 ## 生产者；普通展开／未选分支另核，无类别外自有八进制。头身份、使用点、程序或输入改变即重评。 |
| R7.4 | Required | 字符串到对象、返回及函数实参 | not_assessed | AMD2.47–49 更新宽／UTF 字面量的地址与 const 对象义务，TC2.7–8 只豁免可变参数部分。固定入口仅识别简单目标／实参／返回声明。当前 open 的路径字符串、HostBatch 输出和 assert 展开可定位；#443 仍须对普通及带前缀字面量逐个追踪 typedef、地址运算、条件表达式、间接调用和跨 TU 别名到直接消费者，分别判断固定参数与可变实参例外。 |
<!-- misra-checker-obligations-end -->

## R7.1 当前条件控制证据

本批初次沿 master 正常 run 43410036518 复核。同步 #470 后，最终复核使用成功
run 43411175434、实际测试树 73caf622 的完整输入；新增启动信号掩码使用
SIG_BLOCK／SIG_SETMASK，实际值为单个零／十进制 2，O_RDONLY 的使用点及原头保持。对每个 profile 的当前
自有／生成／应用和项目 patch，先处理 C 行拼接、注释和字符／字符串，再列出物理
`#if`／`#elif` 的数值 token 与标识符；包含未选的项目分支。实际 GCC `-E -dD`
的 define／undef 提供本次所有定义，取其保守并集并递归追踪条件可达替换列表。
该并集覆盖使用时刻的活动定义；所有候选均无八进制 token，传递链无 `##`，
因此该条词法性质无需推测被 `-E` 消去的分支结果。单个数字零及八进制转义按主文
例外分别识别。编译器特征查询由实际 GCC 内建提供，项目没有替代定义。

Linux 的 standard／user、component、no-diagnostic 分别有 813、853、827 个物理
条件，条件及传递定义涉及 289、310、297 个标识符；Windows 分别为 812、852、826
和 287、308、295。计数包含重复头条件的物理文件，作为核查范围，结论来自完整 token
及替换链检查。普通实际展开和项目 patch 另核，唯一八进制使用为
`Os_LinuxNative.c:180` 的 `O_RDONLY→00`，来自固定 glibc 原头。
[SDK 原篇](../../os/sdk.md#当前集成义务与类别处置)保留批准来源、单点／程序范围、
只读与返回检查及重评条件；正常消费者即使无告警也核查该实例。类别外八进制报告、
同一行宏 token 外的位置、新程序、新使用点或改变定义均由拒绝测试确认阻塞。
