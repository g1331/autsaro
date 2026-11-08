# 预处理、库与资源的检查边界

输入、固定工具和责任沿[语言与词法](lexical.md)。同号入口为固定 addon，核心为
`getCppcheckRules` 的准确映射。预处理规则须核对物理指令、实际 define／undef 历史
及展开后的消费者；库规则须区分 ISO 接口、平台接口、采用头和项目自身调用。
本表逐项保留当前额外人工范围，完整修订原文合成及实际使用核查由 #443 承担。
SDK 实例沿 #444，固定 11 项的 R20.1／R21.8 沿原补充组；本表不批准保留违反。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R20.2 | Required | include 的 header-name 字符 | not_assessed | 同号入口在以 #include 空格开头的指令字符串中查反斜杠、注释符、逗号和单引号。#443 须按当前物理及宏展开后的 header-name 核对实际禁用字符、字符串范围和适用修订，避免把指令其他部分计作名字。 |
| R20.3 | Required | include 的最终参数形式 | not_assessed | 同号入口按空格拆分，明确跳过无法取得宏定义的若干间接 include。#443 须核对当前 MemMap／平台头宏展开得到的最终引号或尖括号形式、单一 header-name 及适用修订。 |
| R20.4 | Required | define 的宏名与关键字 | not_assessed | 同号入口正则只识别小写起始且至少两个字符的名字，再查 keyword 表。#443 须核对 _ 开头关键字、单字符拼写、活动指令和实际宏，合成语言／修订后的关键字集合。 |
| R20.5 | Advisory | 全部物理 undef 与实际撤销用途 | not_assessed | 同号入口识别以 #undef 空格开头的指令，具体实例还须按来源和当前输入裁决。#443 须核对当前全部撤销、宏间接形式及适用修订；MemMap 与内部属性／状态的精确诊断由 #439 的 owned-source 消费承担，原类别只覆盖登记机制。 |
| R20.6 | Required | 预处理指令与宏实参的交织 | not_assessed | 核心映射名为 preprocessorErrorDirective；该名字本身没有证明 R20.6 的宏实参内指令义务。#443 尚缺核心实际触发机制及当前跨行宏调用／指令交织的独立词法核查和修订合成。 |
| R20.7 | Required | 宏参数展开表达式的括号 | not_assessed | 同号入口按 Define 字符串和相邻字符检查，带字符串拼接与 # 的启发式分支。#443 须按每个当前使用点判断参数展开是否为表达式、括号及有效原文例外，接到实际消费者。 |
| R20.8 | Required | #if／#elif 的最终控制值 | not_assessed | 同号入口依赖 preprocessor_if_conditions.result，已知非 0／1 时报告。#443 须核对全部实际条件、活动宏、整数模型与捕获完整性，接续[条件整数](../essential-types/conditional-integers.md)，合成适用修订。 |
| R20.9 | Required | 条件表达式中的已定义标识符 | not_assessed | 同号入口检查条件展开 E，并用正则识别 defined；以下划线起始等 token 分支须额外核查。#443 须追踪当前每个条件的活动宏、defined 操作数和展开历史，合成有效原文范围。 |
| R20.10 | Advisory | define 中的 #／## 运算符 | not_assessed | 同号入口只要 expansionList 包含 # 即报告，字符串中的字符和真实运算符仍须区分。#443 须逐当前宏确认实际 stringize／paste 用途和直接消费者，按原文与修订处理具体实例。 |
| R20.11 | Required | 宏参数的 # 后紧接 ## | not_assessed | 同号入口对各参数运行正则，检查 replacement-list 的相邻运算符。#443 须核对原 token、续行／注释正规化、全部当前 define 与参数，合成有效原文及实际展开证据。 |
| R20.12 | Required | 同一参数在 paste 与其他用途间的展开 | not_assessed | 同号入口匹配 ## 和其他出现，再由相同 usefile／行／列推断实参宏使用，源码 TODO 保留精确参数关联缺口。#443 须逐当前调用核对实参是否受替换及两种用途，合成修订。 |
| R20.13 | Required | 全部物理预处理指令 | not_assessed | 同号入口使用固定白名单且额外接受 warning。#443 须按 C99 和有效修订逐指令核定许可集合及 #warning 的实际处置，核对排除分支、空指令及当前扩展用途。 |
| R20.14 | Required | 条件指令配对的物理文件身份 | not_assessed | 同号入口维护 ifStack；正常同文件 #endif 路径没有 pop，跨文件和后续嵌套需独立核对。#443 须按当前物理文件重建完整配对栈，证明每组控制指令同文件并合成修订。 |
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
