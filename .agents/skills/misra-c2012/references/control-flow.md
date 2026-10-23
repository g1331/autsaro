# R15–R17：控制流与函数

Mandatory/Required/Advisory 采用修订后的类别；函数调用、数组形参和返回路径必须结合实际契约。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R15.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.1.html) | Advisory | 2012 | C90/C99/C11；跳转控制 | 优先结构化控制流；保留 goto 要说明可读性和适用理由。 | goto 清单及项目重分类计划。 |
| [R15.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.2.html) | Required | 2012 | C90/C99/C11；goto | 跳向同函数中后面的标签，不借 goto 建立回跳循环。 | 标签位置和控制流边。 |
| [R15.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.3.html) | Required | 2012 | C90/C99/C11；goto | 目标位于同块或包含跳转的外层块，避免跳入内部作用域。 | 跳转两端作用域关系。 |
| [R15.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.4.html) | Advisory | 2012 | C90/C99/C11；循环提前退出 | 每个循环至多一个用于终止它的 break/goto；按 Advisory 处理额外出口及理由。 | 对每一循环单独计数，不把 continue 当成退出。 |
| [R15.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.5.html) | Advisory | 2012 | C90/C99/C11；函数出口 | 尽可行性集中出口；必要多出口明确理由，不能误标 Mandatory。 | 出口位置、资源清理和有效类别。 |
| [R15.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.6.html) | Required | 2012 | C90/C99/C11；分支/循环体 | 用复合语句表达受控语句，避免缩进造成语义误读。 | AST 中分支/循环体结构。 |
| [R15.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule15.7.html) | Required | 2012 | C90/C99/C11；if-else-if 链 | 完整处理末尾剩余情况；无操作路径用有意义说明表达意图。 | 最终 else 及其效果/说明。 |
| [R16.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.1.html) | Required | 2012 | C90/C99/C11；switch | switch 的整体布局符合规范，明确控制值、标签及终结方式。 | 结合 R16.2–R16.7 审查整体语句。 |
| [R16.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.2.html) | Required | 2012 | C90/C99/C11；switch 标签 | case/default 直接属于 switch 主体，避免深层嵌套标签。 | 标签父节点和块层级。 |
| [R16.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.3.html) | Required | 2012 | C90/C99/C11；switch 分支 | 非空分支明确终止；任何贯穿按精确允许形式或偏离处理。 | 各 clause 的无条件 break 要求与例外。 |
| [R16.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.4.html) | Required | 2012 | C90/C99/C11；switch | 覆盖缺省输入，提供 default 分支。 | default 存在性和错误处理语义。 |
| [R16.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.5.html) | Required | 2012 | C90/C99/C11；default 位置 | 将 default 放在首端或末端，保持固定布局。 | clause 顺序。 |
| [R16.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.6.html) | Required | 2012 | C90/C99/C11；switch 选择数 | switch 至少有两个 switch-clause，否则采用合适的 if 或直接语句。 | 按 clause 而非标签数统计；多个 case 共享单一路径不能凑成两个 clause。 |
| [R16.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule16.7.html) | Required | 2012 | C90/C99/C11；switch 控制值 | 布尔选择用 if，switch 使用合适的非布尔离散域。 | 控制表达式 essential 类别。 |
| [R17.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.1.html) | Required | 2012 | C90/C99/C11；可变参数 | 不包含 stdarg.h，也不使用其设施；已有包装须追到 va_list/va_*，不能靠包装隐藏。 | 头文件、va_list/va_* 和包装调用。 |
| [R17.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.2.html) | Required | 2012 | C90/C99/C11；调用图 | 使用有界迭代，不引入直接或间接递归。 | 调用图闭环，涵盖回调/函数指针边界。 |
| [R17.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.3.html) | Mandatory | 2012 | C90；隐式函数声明 | C90 调用前提供兼容原型，不依赖隐式声明；本规则仅适用 C90。C99 诊断先核对所选语言和真实原型，以 R1.1/R8.2/R8.4 等独立判断，不记为本条 Mandatory 偏离。 | 严格编译和包含关系。 |
| [R17.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.4.html) | Mandatory | 2012 | C90/C99/C11；非 void 函数 | 每条退出路径给出符合契约的返回值；C99 main 例外另核对。 | 路径完备性和异常退出契约。 |
| [R17.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.5.html) | Required | 2012 | C90/C99/C11；数组形式参数 | 形参指定数组大小时，从实参所指位置起至少有该数目元素；访问上界较小不能代替形参的大小承诺。 | 调用方容量、形参契约及访问上界。 |
| [R17.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.6.html) | Mandatory | 2012 | C99/C11；数组形参声明 | 不在参数方括号中使用 static 容量承诺。 | 参数 declarator，包括宏展开。 |
| [R17.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.7.html) | Required | 2012 | C90/C99/C11；非 void 调用 | 使用返回值；有意丢弃的允许形式不能替代 D4.7 的错误处理。 | 调用表达式用途及错误信息契约。 |
| [R17.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.8.html) | Advisory | 2012 | C90/C99/C11；参数变量 | 保留输入参数值，需推进游标/状态时使用本地变量。 | 参数对象本身的赋值，不混淆指向对象写入。 |
| [R17.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.9.html) | Mandatory | AMD3 | C11；C11 _Noreturn | 确保所有路径确实终止，不返回调用者。 | 完整路径与依赖函数契约。 |
| [R17.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.10.html) | Required | AMD3 | C11；C11 _Noreturn | 无返回函数的类型采用 void。 | 声明、定义及回调兼容性。 |
| [R17.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.11.html) | Advisory | AMD3 | C11；C11 无返回函数 | 用明确的无返回说明表达真实控制流；不把扩展自动当标准形式。 | main 不在本条建议内，不能为其加 _Noreturn；其余核对声明和真实不可返回路径。 |
| [R17.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.12.html) | Advisory | AMD3 | C90/C99/C11；函数名用作值 | 调用使用参数列表，取地址显式用 &，避免隐含函数衰减。 | 函数 designator 的使用上下文。 |
| [R17.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.13.html) | Required | AMD3 | C90/C99/C11；函数类型 | 不限定函数类型本身；需要限定函数指针对象时放在正确层级。 | typedef、返回类型与指针限定层级。 |
