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
| R1.2 | Advisory | C99 扩展及实际消费者 | not_assessed | 主文38／PDF46；Advisory 扩展违反继续保留。当前 Backend、Os_MemMap／Os_HostTypes 的属性、TLS、asm、原子及 patch 扩展已定位，主机 provider 按 ADR 0030。#443 的 #547 接续逐实例设计理由、受控编译器保证及直接消费者；缺这三项的构造保持未评估。 |
| R1.3 | Required | 完整程序的未定义及关键未指定行为 | not_assessed | 主文38–39、AMD2.29 及修订 Appendix H。Dcm 请求／执行器、OS actor／关闭和 patch 队列的生命周期、指针范围、失败释放及并发仍需完整程序证据。#443 的 #547 按适用行为连接其他规则的已核证据，补齐余下未定义／关键未指定行为；本批局部扫描未替代这项责任。 |
| R2.1 | Required | 各程序的可达路径 | not_assessed | 主文39–41 排除预处理删除的代码。当前四 profile 两程序的 TU、定义和配置入口已定位，Dcm 收发、Bootstrap／SchM、OS 回调及 defensive 分支的完整可达证明未齐。#443 的 #547 逐入口结算活动代码的可达性，或整改／处置具体不可达语句。 |
| R2.2 | Required | 可观察输出和持久状态的生产路径 | not_assessed | 主文41–42、TC1 初始化说明、TC2.3–4 的已用转换／void 例外及 AMD4.37。ARTI 发布、持久写入、空调用及表达式仍缺逐操作的输出／时序影响证明。#443 的 #547 连接实际读方和可观察效果，删除真实死操作或完成必要处置；不按 token 出现次数判定行为影响。 |
| R2.3 | Advisory | 全程序 typedef 定义与使用 | not_assessed | 主文43；实际范围内原报告有28个物理位置，含标准公共类型、生成模式及 BswM 声明。既有公共类型15项与 generated-retention 只处理各自实体；#443 的 #547 接续完整 typedef 定义／使用／固定发布合同结算，标准保留仍保留真实 unused 及正式义务。 |
| R2.4 | Advisory | 双目标四 profile 的两个程序 | not_assessed | LSduR／PduR 匿名枚举误报按指纹处置。QueuePointers／SemaphoreData 是 0015 项目补丁中的真实未用 tag，尚待 #443 的 #547 完成安全清理。 |
| R2.5 | Advisory | define／undef 历史与实际宏使用 | not_assessed | 主文44；TC1 将 undef 计为使用，TC2.5–6 更正为 macro definitions。当前跨组件同名宏有独立 owner，固定 addon 的同名合并及覆盖会遗漏定义实例。#443 的 #547 按每个程序复放 define／undef／实际展开，逐定义连接生成来源、消费者和原保留表；替换表含名字本身未当作实际使用。 |
| R2.6 | Advisory | 函数内 label 与实际跳转 | satisfied | 主文 44 的对象是函数内标签。固定核心 CheckOtherImpl::checkUnusedLabel 跳过 expanded macro，故另检查全部当前原 token 和项目使用点实际展开：没有函数 label／goto；Dcm.c、Os_Backend.c、Os_Arti.c 等的 default 属 switch。项目补丁也无标签；当前双程序不存在需补跳转消费者的标签实体。 |
| R2.7 | Advisory | 函数定义的形参与调用契约 | satisfied | 主文44–45，AMD4.38 更正标题；逐个检查878种去重的范围内函数定义展开、975个具名形参，函数体普通 identifier 均有真实引用，无未引用形参；成员名字单列。Ecu_OsHooks／Det_Callouts 的固定 callback 用显式 void 转换消费值，其他形参连接实际计算、输出或下层调用；编译器 -Wextra/-Werror 和实际调用验证保留，函数指针类型的形参不冒充函数定义。 |
| R3.1 | Required | 所有自有／patch 物理注释与行拼接 | satisfied | 主文 45 及 AMD4.39–41 允许行注释中的 // 和 scheme://path URI。独立扫描所有物理文件及行拼接后的注释，项目注释无其余嵌套起始序列；完整 kernel 的候选均回到采用原行。版本头、MemMap、RTE 头与项目 port 补丁的注释均在本次集合中；不依赖 addon 的 URI 启发式。 |
| R4.1 | Required | 原文件和实际展开的带前缀字符／字符串 | satisfied | 主文 46–47 要求数值转义终止于下一转义或该字面量末尾，AMD2.32 删除 See also。独立识别单个字面量及转义边界，覆盖宏定义、行拼接、L 前缀及实际展开；所有自有／patch 数值转义均正确终止，未发现 SDK 宏在项目使用点引入额外候选。Os_LinuxNative 的行缓冲终止符及 HostBatch 的输入／输出字面量保持原字符值。 |
| R4.2 | Advisory | 预处理之前的全部自有／patch 物理字节 | satisfied | 主文 47 的九种三字符组适用于任意位置。逐文件检查拼接前原字节，包含注释、字符、字符串、宏定义和未选分支，均无三字符组；当前所有交付 C／头及最终 kernel 项目行已纳入。该证据补齐同号入口只检查双引号字符串的范围。 |
| R5.2 | Required | 当前同 scope／namespace 的有效名称 | satisfied | 主文 49–50、AMD2.35 的 C99 最低区分长度为 63。对全部物理标识符及实际展开的项目使用点做跨 scope 保守前缀比较，无不同拼写共用 63 字符前缀；同名声明约束由两目标实际 C99 编译检查。包含 ComM 头、所有生成 RTE 头与项目 patch；两者均外部链接及宏的例外仍分别沿 R5.1／R5.4／R5.5。 |
| R5.3 | Required | 双目标四 profile 的 C99 HostBatch／probe 中同 namespace 的嵌套 scope | satisfied | 主文50–51／PDF58–59、AMD2.36；从 452 份 GCC 展开按声明点顺序还原普通标识符与 tag 的嵌套 scope，成员各归所属聚合类型。覆盖 typedef、枚举常量、函数原型／定义形参、局部对象和宏展开声明。范围内无同 namespace 遮蔽；Linux 的 access 和 Windows 的 byte 三个物理位置均落在 ADR 0030 主机专用实现。按下节所列实际输入与归属评估。 |
| R5.4 | Required | 活动宏名及各自参数名 | satisfied | 主文 51–52、AMD2.37–38 按定义时的活动集合和 C99 63 字符判断。原文件所有宏／参数的保守前缀比较无冲突；实际 -dD 顺序重放 define／undef，逐次核对项目宏与已定义宏／参数及本宏参数，亦无冲突。包含反复撤销的 Os_MemMap、RTE_CORE 下的应用宏及 port 宏；同号入口未清 undef 的限制由该独立核查补齐。 |
| R5.6 | Required | 双目标四 profile 的 C99 HostBatch／probe，跨名称空间及 TU | not_assessed | 主文54／PDF62。#443 的 #547 已核对 Rte_Instance 和 Windows 的 CounterType、ResourceType、boolean、byte。待处理漏报的 SDK 集成重名，再完成 Rte_Instance 正式偏离。[声明证据](typedef-uniqueness.md) |
| R5.8 | Required | 外部链接名称与全部其他实体 | satisfied | 主文56／PDF64；按四 profile 的 HostBatch／probe 分别比较 external-linkage 定义名与全部函数、对象、typedef、tag、enum 及成员／形参声明，完整 GCC 行标记区分物理 owner；当前外部名无其他实体重用。Os_ArtiTasks、Ecu_OsConfig、Rte_Instance_<组件> 和回调绑定均连接唯一定义与实际消费者；普通重复声明归同一实体，SDK 原头和采用原行沿 ADR 0025。 |
| R5.9 | Advisory | 双目标四 profile 的 HostBatch／probe 内部链接名称与全部其他实体 | not_assessed | 主文57／PDF65、TC1 修正主体、AMD2.27 追加 C11。沿用第八批的完整实体清单、36 个已修重用名及 53 个头内 static inline 的同一头例外核对。#443 的 #547 本轮消除 7 个 BSW 重用名。Os_Backend.c:53 的 activations 与 Os_ArtiTaskType.activations 重名，仍是实际 Advisory 违反。 |
| R6.1 | Required | 项目位域及实际声明类型／消费者 | satisfied | 主文 58–59、AMD2.41–43 允许 C99 实现支持的显式有符号／无符号类型及 bool，可经 typedef。当前物理与实际展开的项目位域仅 Dcm_MsgAddInfoType 的 reqType／suppressPosResponse；两目标 uint8→uint8_t→unsigned char，实际 GCC C99 编译接受。Dcm 接收初始化／子功能提取写入，响应抑制读取后者；无项目 union 位域或其他 typedef 位域候选。 |
| R6.2 | Required | 有名的一位位域 | satisfied | 主文 59、AMD2.44–45 区分有名与无名位域。Dcm_MsgAddInfoType 的 reqType／suppressPosResponse 均有名、宽度 1，实际 typedef 展开为 unsigned char；Dcm 的 0／1 写入和抑制判断沿 R6.1 的直接消费者。全部物理及实际展开已核对，项目补丁无新增位域，未发现有符号一位对象。 |
| R7.1 | Required | 原 token、条件和实际展开整数 | satisfied | 主文 59–60／PDF 67–68、AMD2.27；固定 raw token 入口遗漏宏后八进制。当前实际展开仅 Linux Os_LinuxMappedStack 的 open 实参 O_RDONLY→00，按 #476 批准接入 sdk-integration 精确类别、glibc 原头及 HostBatch／probe；保持只读、descriptor／read／close 检查。物理项目／patch 的 #if／#elif 与全部实际 -dD 定义作保守传递闭包，无八进制 token 或 ## 生产者；普通展开／未选分支另核，无类别外自有八进制。头身份、使用点、程序或输入改变即重评。 |
| R7.4 | Required | 字符串到对象、返回及函数实参 | satisfied | 主文62–63、AMD2.47–48 的字符类型及 TC2.7–8 的可变实参例外。实际字符串全部回接 Arti_Record 的四个 const char*、Os_BackendAssert／Os_HostTextAppend 的 const char*、Ecu_StatusName 的 const char* 或 task_name 数组初始化；宏生成的文件名、断言文本、条件／相邻字面量同样核查。无宽字符串、取字面量地址后交可写指针或直接修改；未用 variadic 例外覆盖固定形参。 |
<!-- misra-checker-obligations-end -->

### R5.3 实际声明与作用域

基线 `3a57a066` 的[双平台原始工件](https://github.com/g1331/autsaro/actions/runs/44513871093)
含 16 个程序、888 个 TU 实例，Linux／Windows 分别有 228／224 份不同 TU 的 GCC 展开。
普通标识符与 tag 按声明点和外层作用域核对，定义的参数与函数体最外层同属一个作用域，
原型参数限于本原型。成员跟随各自 struct／union，标签在函数范围内，各自使用 C99 的名称空间。
用 63 字符比较活动名字。用项目的 TU 包含闭包及内核补丁位置区分项目、采用原行和主机专用
实现，SDK 声明仍参与目标使用点的名字查找。RTE 应用各自包含的 typedef 与 Det 回调形参
也进入作用域盘点，外部名字的跨 TU 唯一性分别按 R5.6／R5.8 核对。

Linux `os/src/host/linux/Os_HostLinux.c:1175` 的形参 `access` 遮蔽 glibc 的
`unistd.h:287` 函数。Windows `integration/src/ecu_host_batch.c:13/95` 的形参和局部变量
`byte` 遮蔽 MinGW 的 `rpcndr.h:93` typedef。三个位置都是主机专用实现。
这批 R5.3 结论覆盖 ADR 0030 范围内的实际声明，包含生成 C、应用和项目内核补丁；
范围内没有发现遮蔽。配置、实际头文件、宏展开或声明作用域改变时重核。

#### Linux 解析恢复与包含闭包

228 份 Linux 展开中，四个 profile 各有四份 TU 触发辅助语法解析恢复。下表列出
`compiler-origins-<编号>.stdout.log` 的编号，先列 standard-ecu／user-application，
再列 component-pipeline／no-diagnostic。头数取实际 `projectTranslationUnits`，不计 TU 入口。

| TU | 工件编号 | 原解析错误节点／位置 | 交付头数 |
| --- | --- | --- | --- |
| `os/src/Os_Backend.c` | 37／39 | 1 个，glibc `bits/stdlib.h:45` | 51 |
| `os/src/host/linux/Os_HostLinux.c` | 46／48 | 3 个，`asm-generic/int-ll64.h:20/26` 和 `linux/types.h:12` | 20 |
| `os/src/host/linux/Os_LinuxNative.c` | 47／49 | SDK 同上 3 个，另有本文件第 63–109 行的 18 个 | 9 |
| `os/src/host/linux/Os_StackLinux.c` | 49／51 | 上述三个 SDK 位置和 `bits/stdlib.h:45`，共 4 个 | 50 |

Clang 18.1.3 对这 16 份 GCC 展开逐 TU 以 C99 成功检查语法，保留 GCC 行标记及全部
头文件展开，仅去掉 `-dD` 输出的宏定义和撤销行。辅助作用域解析使用等宽替换处理 SDK 的
`__signed__`、`__int128` 和常量 `__typeof` 转换，以及 Native TU 内 14 个
`__builtin_offsetof` 表达式。后者保留类型名和换行，成员路径只含标识符和点号。
这一步用于恢复名字和作用域，采用原始展开的 Clang 检查核对实际语法和类型。
16 份完整 TU 重放均无解析错误或缺失节点。跳过声明数为 0，交付头中的声明随各自 TU
一并重新比较，SDK 的外层声明也参与名字查找。重放只复现上述主机专用 `access` 遮蔽。

#### Windows SDK 解析恢复

224 份 Windows 展开中，每个 profile 的同一组 30 个 TU 在 SDK 内部触发解析恢复。
辅助解析等宽合并 `intrin-impl.h` 的 20 处相邻 asm 约束字符串，并将两个
`avx512fp16*intrin.h` 中的 6 处 `_Float16 _Complex` 换为可解析的类型写法，保留参数与
变量名。120 份受影响 Windows TU 完整重放后，解析错误和跳过声明数均为 0，仍只复现
两个主机专用 `byte` 遮蔽。原始 SDK 和 CI 工件继续保留。

### R5.9 私有对象改名

文件与行号对应原声明，改名后物理行保持。以实际 GCC 展开和 Cppcheck 声明绑定核对
直接引用，成员／局部名称保持原样。双目标四 profile 的配对扫描与逐诊断证据见 #547
的实施 PR。历史 R10.3／R10.4 仍按原规则报告。

| 内部链接声明 | 原名 | 当前名称 | 直接消费者 |
| --- | --- | --- | --- |
| CanTp.c:164 | running | cantp_running | 进入 runner 时认领，工作队列耗尽时释放 |
| Det.c:10 | initialized | det_ready | Init 发布配置和就绪状态，ReportError／ReportRuntimeError 检查就绪 |
| Com.c:34 | reloaded_before_main | com_rx_reloaded | RxIndication 记录本周期重载，MainFunctionRx 读取并清除 |
| Com.c:33 | remaining | com_ticks | RxIndication 重载超时计数，MainFunctionRx 递减并在到期后重载 |
| Dcm.c:101 | pending_session | dcm_pending_session | 请求结果写入对象，抑制响应或成功发送确认时提交会话 |
| Dcm.c:87 | state | dcm_s | TP 接收、主调度和发送确认读写请求状态 |
| Dcm.c:100 | session | dcm_ses | 会话设置、公开查询、DID 消费及 S3 到期逻辑读取或更新当前会话 |

Os_Backend.c:956 的 `Os_TargetTestPending` 只在 `OS_ACTIVATION_TESTS` 开启时编译。
当前两目标的 HostBatch 和 probe 都未开启该开关，改名证明缺少此处的 GCC 展开与
Cppcheck 绑定。保留 `activations`，待两侧证据覆盖该测试分支并通过现有证明后继续改名。

## R2.4 枚举误报与 tag 引用

主文印刷 43／PDF 51 将 tag 与 typedef 名分开。AMD2 增列 C11，当前按 C99 判定。
固定 addon 的 `MisraTagName` 用 Cppcheck scope 的 `className` 和左花括号位置建表。
在 `LSduR.h:14:14`（`LSduR_StateType`）和 `PduR.h:30:14`（`PduR_StateType`），
实际声明均以 `typedef enum {` 开始，因此没有 tag 声明。该误报与类型别名的 R2.3
记录分别裁决。验收按封存原件、诊断位置、程序实际头文件依赖和逐程序指纹归档，保留原报告。
双目标四 profile 的 RTE CDS tag 均由独立 typedef 引用。`Os_NativeStackType` 由
`Os_ArtiTaskStacks` 的声明和定义直接引用。队列及内核原件见[采用篇](../../os/adoption.md#r24-内核-tag)。

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
