# 类型、转换与表达式的检查边界

输入与固定工具沿[语言与词法](lexical.md)。本表补充其余 checker-covered 项的能力
定位和人工恢复条件；既有[essential type 证据](../essential-types/README.md)及
[主机地址](../../os/native-facilities/addresses.md)保留具体表达式和来源身份。
原 11 项中的 R10.1／R10.3／R10.6／R13.5 仍沿原补充组，R7.2／R7.3／R12.3
沿[原局部记录](README.md#三项局部证据的当前消费)。以下能力审阅均未完成全部修订
原文与当前直接消费者的逐表达式证明，恢复责任为 #443，状态保留未评估。

同号入口指固定 `misra.py`，核心指 `getCppcheckRules`。表达式没有类型／值流时的
`continue` 分支是实际能力限制；类型 helper 的结果还须与 C99 的 effective／essential
type、枚举、Boolean／character、常量例外及修订后的复合表达式模型分别核对。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R10.2 | Required | character 的加减运算 | not_assessed | 同号入口递归识别字符加法，并用 essential category 检查两个操作数。#443 须核对 typedef char、宏／条件表达式、character 与数值用途及结果消费者，合成适用修订；仅字符字面量识别不足以证明全部用途。 |
| R10.4 | Required | usual arithmetic conversion 的操作数 | not_assessed | 同号入口枚举算术、按位、比较和条件冒号，使用 essential category 并跳过缺类型的 AST。#443 须逐实际运算核查 Boolean／enum／character 类别、复合赋值及常量例外，接续真实数据类型与适用修订。 |
| R10.5 | Advisory | essential category 间的显式转换 | not_assessed | 同号入口以 bool／char／floating／signed／unsigned 分类检查 cast，包含常量 1 的例外分支。#443 须核对命名 enum、浮点／字符类别及有效原文例外的完整性，追踪当前每个转换的值域和下游用途。 |
| R10.7 | Required | 复合表达式与另一操作数的宽度 | not_assessed | 同号入口只沿已识别算术 AST 和部分赋值父节点比较 essential bits。#443 须核对移位、按位、条件表达式及常量表达式边界，补当前跨宽度表达式和消费者证明，并合成修订。 |
| R10.8 | Required | 复合表达式结果的 cast | not_assessed | 同号入口枚举复合操作符，类别不一致或更宽时报告，未知 essential type 时跳过。#443 须核对常量表达式、枚举／Boolean、嵌套条件和实际结果宽度，接续 producer 及固定修订原文的例外。 |
| R11.1 | Required | 函数指针的全部转换 | not_assessed | 同号入口比较 get_function_pointer_type 的两端，目标非函数指针即报告；既有 (void)sink 是原文允许的具体误报。#443 须按真实原型、调用约定和赋值／实参／返回路径核对全部转换及 AMD2 例外，具体丢弃见原篇。 |
| R11.2 | Required | 不完整对象类型的指针转换 | not_assessed | 同号入口收集 struct 前置声明并比较可识别的指针类型名，范围受源码拼写和 typeScope 限制。#443 须核对 typedef、不完整 union／对象和完整化时点，追踪实际消费者，合成适用修订。 |
| R11.3 | Required | 不同对象类型指针的 cast | not_assessed | 同号入口检查 record scope 或相同指针层数的基本类型差异，并跳过 void。#443 须核对多层指针、character 访问例外、对齐和别名用途，逐实际转换追到解引用消费者及修订原文。 |
| R11.4 | Advisory | 对象指针与整数之间的转换 | not_assessed | 同号入口检查显式 cast，对常量宏可将报告折回唯一 define。#443 须补隐式及复杂表达式、真实 LP64／LLP64 宽度和使用点关系；既有主机地址类别只覆盖登记实例，其他转换逐项核查。 |
| R11.5 | Advisory | void * 到对象指针的转换 | not_assessed | 同号入口检查部分赋值及 cast，直接 malloc／calloc／realloc／free 形态另行跳过。#443 须按有效原文核对例外、参数／返回和别名路径，以及恢复对象类型与生命周期，接续实际主机接口消费者。 |
| R11.6 | Required | void 指针与算术类型之间的 cast | not_assessed | 同号入口只检查一层 void 指针，已知零值有独立分支。#443 须核对浮点、整数表示、空指针例外和地址往返保证，连接主机地址类别及实际接口，合成适用修订与表外实例。 |
| R11.7 | Required | 对象指针与非整数算术类型的 cast | not_assessed | 同号入口依赖 valueType，并跳过源表达式还有 astOperand1 的形态。#443 须复核复合／嵌套转换、enum／void 例外与实际输入，按固定原文追到直接消费者。 |
| R11.8 | Required | 移除指向对象的 const／volatile | not_assessed | 同号入口比较 cast 和已解析函数实参的 constness 最低位。#443 须逐指针层核对 volatile、typedef、赋值／返回及间接调用中的限定符变化，补当前使用与修订原文证据。 |
| R11.9 | Required | 整数空指针常量的使用形式 | not_assessed | 同号入口只遍历 =／比较／条件 AST 的部分方向，用值流识别零和 NULL。#443 须核对反向比较、实参／返回、多层宏和项目 NULL_PTR 的实际展开，按原文区分 void * 空指针与整数空指针常量。 |
| R12.1 | Advisory | 运算符优先级与显式括号 | not_assessed | 同号入口用 precedence／parentheses helper，另有 raw sizeof 辅助入口。#443 须核对宏实参、预处理条件和展开后括号的实际对应，按修订后的原文例外判断，补当前全部相关表达式证据。 |
| R12.2 | Required | 移位计数与左操作数 essential 宽度 | not_assessed | 同号入口取 RHS 已知正最大值；最大值为零或没有值流时跳过，负计数也需额外核查。#443 须证明每个当前计数的上下界和左值宽度，包含宏、动态输入及 LP64／LLP64 差异并合成修订。 |
| R12.4 | Advisory | 无符号常量表达式的 wrap-around | not_assessed | 同号入口只在已知常量的 +／-／* 计算中检查，宽度大于等于 64 时跳过，并特殊遍历条件表达式。#443 须独立核对 64 位、全部常量子表达式和有效分支，接续实际常量值、模型及修订原文。 |
| R13.1 | Required | initializer list 的持久副作用 | not_assessed | 同号入口扫描初始化列表的 ++／--／赋值 token，未证明被调函数的持久副作用。#443 须追踪当前 initializer 的函数、volatile 和宏展开，核对指定索引／复合字面量的范围及适用修订。 |
| R13.2 | Required | 允许求值顺序下的值及副作用一致性 | not_assessed | 核心 unknownEvaluationOrder 能报告具体冲突，别名与跨调用状态仍需人工。#443 须沿实际自有／patch 表达式追踪 volatile、回调、原子访问及共享对象读写，按主文与修订比较所有允许顺序。 |
| R13.3 | Advisory | ++／-- 所在 full expression 的副作用 | not_assessed | 同号入口向上遍历到逗号／分号边界后 countSideEffects，依赖其 AST 和副作用模型。#443 须核对函数调用、volatile 及真正 full-expression 边界，并补当前直接消费者与适用修订。 |
| R13.4 | Advisory | 赋值表达式结果的后续使用 | not_assessed | 同号入口仅检查 = 且有 AST parent 的若干位置，compound assignment 和规范例外须另核。#443 尚缺当前全部赋值结果、链式赋值／参数／控制用途和修订合成。 |
| R13.6 | Required | sizeof 操作数中的持久副作用 | not_assessed | addon 调用 countSideEffectsRecursive，核心补 sizeofCalculation；函数模型与 VLA 求值边界仍需原文判断。#443 须核对当前 sizeof 的宏／函数／volatile 操作数和实际类型，合成适用修订并区分是否求值。 |
<!-- misra-checker-obligations-end -->
