# 控制流、调用与对象生命周期的检查边界

输入、固定工具和责任沿[语言与词法](lexical.md)。同号入口为固定 addon 的
`misra_N_M`；核心入口为其准确诊断映射。控制流能力依赖实际宏展开、函数图、
scope 及值流；函数指针回调、跨 TU 服务和被调用接口的资源约束另有人工责任。
以下记录尚缺完整的修订原文合成与当前源码／直接消费者复核，逐项由 #443 承接。
R15.5 单独引用 ADR 0025 的既有 GRP 决定，原始告警仍保留。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R14.1 | Required | 浮点循环计数及控制 | not_assessed | 同号入口识别 for counter 和有限 while 浮点计数模式；间接更新、回调和其他循环形态仍需人工。#443 尚缺当前循环计数全集、实际类型／更新来源与主文及修订合成，浮点字段存在与计数用途分别判断。 |
| R14.2 | Required | for 的三个子句与循环体 | not_assessed | 同号入口检查计数变量、更新和副作用，源码 TODO 明确未检查函数调用对计数器的修改。#443 须核对当前地址逃逸、别名更新、回调和循环条件，按 TC1 等适用修订证明各子句约束。 |
| R14.3 | Required | 控制表达式的 invariant 性质 | not_assessed | 核心 alwaysTrue／alwaysFalse／compareValueOutOfTypeRangeError 报告已知条件；配置状态及运行期间取值范围仍需证明。#443 须区分预处理条件、宏 do-while 和实际循环／分支，核对有效原文例外及当前消费者。 |
| R15.1 | Advisory | 实际预处理后的 goto | not_assessed | 同号入口遍历语义 token 的 goto，识别能力与该词法性质直接对应。#443 尚缺当前所有自有／patch TU 的 token 保真与宏展开对应核对、原文修订合成及实际报告处置；全部收据齐备后可归正常工具证据。 |
| R15.2 | Required | goto 目标的前向关系 | not_assessed | 同号入口依赖 findGotoLabel；找不到目标即报告。#443 须核对实际目标查找与源序、宏生成标签和条件编译，按当前函数逐跳转确认前向约束并合成修订。 |
| R15.3 | Required | goto 与 enclosing／switch clause | not_assessed | 同号入口比较 goto／label 的 scope 并检查 switch clause。#443 须核对嵌套语句、宏标签、目标作用域和有效原文例外，补当前所有跳转的实际清理／生命周期消费者证据。 |
| R15.4 | Advisory | 循环的 break／goto 终止点数量 | not_assessed | 同号入口按 scope 递归累计 break／goto，switch 中 break 另行处理。#443 尚缺实际目标是否退出当前循环、嵌套循环及条件展开的逐语句判断和适用修订合成。 |
| R15.5 | Advisory | 自有 C 的函数退出路径 | satisfied | [ADR 0025 GRP](../../../adr/0025-misra-layered-scope.md#grp指南重分类计划)已将本条对自有目标 C（含生成 C 及项目补丁；主机层已由 [ADR 0030](../../../adr/0030-host-simulation-layer-outside-misra-scope.md) 移出范围）Disapplied；本行消费既有重分类决定，固定 addon 的非顶层 return 告警照常保留。资源回收、输出和返回语义继续由各自规则与实际消费者核查，采用原件沿供应方责任。 |
| R15.6 | Required | iteration／selection 的复合语句体 | not_assessed | 同号入口对 raw token 运行状态机，预处理指令和宏语句体可能影响原 token 与实际语法的对应。#443 须复核实际展开后的全部语句体、else-if 形式及适用修订，接续宏消费者。 |
| R15.7 | Required | if／else-if 链的最终 else | not_assessed | 同号入口沿 token／scope 查找最终 else；最后分支的副作用或说明义务仍须按原文核查。#443 尚缺当前完整链、宏展开和终止分支语义及适用修订合成。 |
| R16.1 | Required | switch 的规范结构 | not_assessed | 同号入口以 case／default 和 break 维护 scope 状态，依赖规范化 token。#443 须按原文规定的 switch 语法逐结构复核，包含空 clause、嵌套块和宏生成分支，合成适用修订。 |
| R16.2 | Required | case／default 的所属 switch | not_assessed | 同号入口只检查 case token 的 scope 类型。#443 须补 default、嵌套 selection／loop、宏标签及实际 enclosing scope 的完整核查和原文修订合成。 |
| R16.3 | Required | 各 switch clause 的终止 | not_assessed | raw 状态机接受 break／return／throw、fallthrough 注释及属性形态；哪些满足规则须按有效原文分别核定。#443 须核对当前每个 clause 的无条件 break 与实际控制流，保留工具启发式接受带来的人工责任。 |
| R16.4 | Required | 每个 switch 的 default | not_assessed | 同号入口对已识别 switch 括号／花括号查找 default，跳过内层块。#443 尚缺宏展开、嵌套结构和全部实际 switch 与工具 scope 的对应及适用修订合成。 |
| R16.5 | Required | default 的首尾位置 | not_assessed | 同号入口扫描 default 后有无 case，并接受紧随 { 的形式。#443 须核对真实 switch clause 的顺序、嵌套块和宏标签，以及适用修订后的语法范围。 |
| R16.6 | Required | switch clause 的最少数量 | not_assessed | 同号入口按 break／return／throw 和 noreturn scope 计数，计数对象与规范 clause 有差异风险。#443 须按当前实际 case／default 分组和空标签序列独立计数，合成修订并核对直接消费者。 |
| R16.7 | Required | switch 控制表达式的 essential 类型 | not_assessed | 同号入口调用 isBoolExpression；#443 尚缺 Boolean typedef／宏、条件表达式和枚举模型与 D.6 的对应，须核对当前所有 switch 及适用修订。 |
| R17.1 | Required | 变参设施的声明和使用 | not_assessed | 同号入口识别 va_list 及已知 va_* 调用名字，真实 SDK 宏可能变为内建。#443 须核对项目 typedef、宏展开、变参声明及消费者，合成适用修订；标准库内部设施沿来源责任处理。 |
| R17.2 | Required | 直接／间接递归的完整调用图 | not_assessed | 同号入口在单配置已解析函数间搜索递归，未知函数指针和跨 TU 边留下缺口。#443 须追清标准回调、Hook、同步重入、函数指针绑定和两个程序各自调用图，按原文核定全部递归路径。 |
| R17.4 | Mandatory | 非 void 函数的返回路径 | not_assessed | 核心 missingReturn 识别局部缺返回路径；非局部转移、noreturn 模型和条件展开仍需核定。#443 须核对当前所有非 void 定义及实际平台终止入口，合成适用修订；Mandatory 缺口保持阻塞。 |
| R17.5 | Required | 数组形参的实际可访问范围 | not_assessed | 核心 argumentSize 依赖已知数组尺寸和调用模型。#443 尚缺指针／长度成对参数、切片、typedef 和间接回调的实际范围证明，须追到当前读写消费者及有效修订。 |
| R17.6 | Mandatory | 数组参数声明的 static 形式 | not_assessed | 同号入口匹配 raw [ static，其他限定符顺序和宏展开须另查。#443 尚缺当前全部原型／定义／函数类型声明与实际展开，以及主文和修订的语言范围核查；Mandatory 保持原义。 |
| R17.7 | Required | 非 void 返回值的使用或显式丢弃 | not_assessed | 同号入口检查无 AST parent 的已知返回类型调用，未知／间接类型会跳过。#443 须核对当前函数指针、宏、显式 void 丢弃及错误返回的实际用途，合成适用修订。 |
| R17.8 | Advisory | 函数形参对象的修改 | not_assessed | 同号入口检查赋值／自增减 AST 左侧的 isArgument；别名写入和复杂左值仍需判断。#443 须区分修改指针形参本身与所指对象，核对当前路径和有效原文范围。 |
| R18.1 | Required | 指针算术结果所在数组范围 | not_assessed | 核心 pointerOutOfBounds 依赖已知边界。#443 须证明实际 base／长度／one-past 和对象来源，包含堆、注册栈及序列化缓冲，按 TC2 与后续修订核对，接到最终访问者。 |
| R18.2 | Required | 指针相减的对象归属 | not_assessed | 核心 comparePointers 提供部分不同对象比较候选。#443 须逐实际减法证明两端属于同一数组或适用的完整对象规则，核对别名、字节缓冲及有效原文修订。 |
| R18.3 | Required | 指针关系比较的对象归属 | not_assessed | 核心同用 comparePointers；#443 须按关系比较的原文范围独立核对数组、结构成员及联合成员，追清当前实际指针来源，不能借用 R18.2 的结论。 |
| R18.4 | Advisory | 指针的 +／-／+=／-= | not_assessed | 同号入口只对已有 valueType 的两端识别指针。#443 须核对宏、typedef、未知类型与项目真实对象范围，完整原文和修订核查后再处理具体保留实例。 |
| R18.5 | Advisory | 对象声明的指针层数 | not_assessed | 同号入口从变量 name 向前数连续名称／*，typedef 和复杂声明留下模型责任。#443 须递归展开当前 typedef、函数指针和全部对象声明，核对真实层数及修订原文。 |
| R18.6 | Required | 自动对象地址的存续范围 | not_assessed | 核心 danglingLifetime 依赖逃逸和库模型。#443 须追踪异步队列、回调、TLS、host context 和输出参数的保存／释放顺序，接到最后消费者，合成原文修订。 |
| R18.7 | Required | flexible array member 声明 | not_assessed | 同号入口只在 Struct 中识别 []，跳过部分嵌套及函数指针形态。#443 须核对 typedef、宏和实际对象布局，确认当前声明全集及有效原文修订；SDK 原头和项目使用分别处理。 |
| R18.8 | Required | variable length array 类型 | not_assessed | 同号入口只遍历 local array 的首个维度，未知常量也跳过。#443 须核对多维、typedef、参数及全部尺寸表达式，合成 AMD4 将 variably-modified pointer 另交 R18.10 后的边界。 |
| R19.1 | Mandatory | 对象赋值／复制的重叠 | not_assessed | 核心 overlappingWriteUnion／overlappingWriteFunction 覆盖部分已知重叠。#443 须按实际别名、偏移和长度证明源／目标关系，核对允许的完全重叠例外及真实库调用，合成适用修订。 |
| R19.2 | Advisory | union 的声明和实际用途 | not_assessed | 同号入口识别语义 token union，可定位已保留的声明。#443 须核对原 token／宏／typedef 的模型完整性、实际成员用途和有效原文修订；SDK union 由 #444 按头外集成责任处理。 |
<!-- misra-checker-obligations-end -->
