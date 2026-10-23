# 指令：设计、过程与并发

每行是独立的编码/核查动作，不是规范原文。适用条件是筛选提示，不能直接充当不适用结论。语言范围和有效类别按固定原文修订链核对；C11同样涵盖C18。D5.x 的规范语言范围为C11；C99平台并发仍按实际风险和项目契约分析，不能据此省掉同步。精确例外仍须查原文。核查方式描述所需证据，不承诺某工具已支持。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [D1.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012d1.1.html) | Required | 2012 | C90/C99/C11；全项目、目标相关行为 | 固定整数模型、符号性、字节序和编译器选择；对输出依赖的实现选择写明依据。 | 编译器手册、平台配置与依赖行为清单。 |
| [D2.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012d2.1.html) | Required | 2012 | C90/C99/C11；全部交付翻译单元 | 将实际交付源码连同宏、头文件按目标参数编译；不把无法编译的分支排除后宣称通过。 | 每个受支持配置的编译日志。 |
| [D3.1](https://www.mathworks.com/help/bugfinder/ug/misra-c2012-guidelines-not-checked.html) | Required | 2012 | C90/C99/C11；全部实现与生成器 | 为新增行为关联真实需求或接口契约；生成工件追溯到模板、输入配置和需求。 | 人工核对双向追踪，不能由编译结果推导。 |
| [D4.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.1.html) | Required | 2012 | C90/C99/C11；全项目的运行风险 | 在入口与资源边界处理失败，证明容量、时间、算术和生命周期限制。 | 风险路径分析及独立预期的边界测试。 |
| [D4.2](https://www.mathworks.com/help/bugfinder/ug/misra-c2012-guidelines-not-checked.html) | Advisory | 2012 | C90/C99/C11；存在汇编 | 记录汇编的功能、寄存器/内存副作用、平台前提与调用契约。 | 人工核对汇编说明和实际代码；无汇编需有范围证据。 |
| [D4.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.3.html) | Required | 2012 | C90/C99/C11；存在汇编 | 把汇编集中到有契约的适配边界，避免散落业务逻辑。 | 检索汇编入口并核对边界调用和 clobber。 |
| [D4.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.4.html) | Advisory | 2012 | C90/C99/C11；注释与停用逻辑 | 删除停用代码，历史交给版本控制；必要条件编译需可解释配置。 | 检查注释是否混入可执行代码。 |
| [D4.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.5.html) | Advisory | 2012 | C90/C99/C11；命名 | 避免仅靠易混字形区分同时可见的名字，选择能表达职责的命名。 | 人工对照命名及生成符号映射。 |
| [D4.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.6.html) | Advisory | 2012 | C90/C99/C11；数值与公共类型 | 采用表达宽度和符号的既有数值类型；plain char 不属本指令的数值类型，位域及 main 的 int 另核对允许形式。 | 类型定义及目标宽度核查，包括浮点/复数。 |
| [D4.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.7.html) | Required | 2012 | C90/C99/C11；返回错误信息的调用 | 判断实际返回契约，检测失败并传播或恢复；不以丢弃返回值隐藏错误。 | 调用路径及失败注入，区分无错误信息的返回值。 |
| [D4.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.8.html) | Advisory | 2012 | C90/C99/C11；结构/union 的全部指针均不解引用的 TU | 按该 TU 对同一类型的全部指针判断；全部均不解引用时，以不完整类型隐藏布局。 | 检查头文件暴露与各翻译单元解引用情况。 |
| [D4.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.9.html) | Advisory | 2012 | C90/C99/C11；函数式宏 | 能使用普通函数时优先函数；保留宏需核对等价性与泛型例外。 | 比较展开语义、参数求值次数及适用例外。 |
| [D4.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.10.html) | Required | 2012 | C90/C99/C11；头文件包含 | 普通头文件建立可靠防重包含机制；MemMap 等重复包含契约单独判断。 | 重复包含编译及包含链审查。 |
| [D4.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.11.html) | Required | 2012 | C90/C99/C11；库函数调用 | 调用前证明库函数的定义域、缓冲区、指针及格式参数有效。 | API 契约与输入域证明，不能只靠返回成功。 |
| [D4.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.12.html) | Required | 2012 | C90/C99/C11；内存分配机制 | 使用有界静态或调用方存储；标准、第三方及自定义动态分配机制都须核对，固定池不能仅因有上界就视为不适用。 | 标准/自定义分配器调用图和资源上界。 |
| [D4.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.13.html) | Advisory | 2012 | C90/C99/C11；有状态资源 API | 让获取、初始化、使用、关闭、释放形成明确的状态序列。 | 各退出路径与资源状态转换审查。 |
| [D4.14](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.14.html) | Required | AMD1 | C90/C99/C11；外部数据 | 在协议/文件/回调边界验证长度、取值及关系，再进入内部可信状态。 | 外部来源清单与有效/无效输入测试。 |
| [D4.15](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.15.html) | Required | AMD3 | C90/C99/C11；浮点计算 | 检测并处理可能生成的 NaN/Inf；可依据实现的传播语义延后检查，但不得进入未设计处理它们的代码。 | 浮点范围分析和非有限值测试。 |
| [D5.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012d5.1.html) | Required | AMD4 | C11；存在线程并发 | 共享状态的读写建立真实同步和所有权；不能用 volatile 代替同步。 | 并发访问图、同步证明；ISR 模型另核对。 |
| [D5.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012d5.2.html) | Required | AMD4 | C11；存在线程及同步对象 | 固定锁序与等待协议，分析所有循环等待和回调重入。 | 锁图、资源顺序与停止路径人工检查。 |
| [D5.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012d5.3.html) | Required | AMD4 | C11；存在线程创建 | 只在明确定义的程序启动阶段创建线程；运行中即使数量有界也不能据此视为符合。 | 创建入口、生命周期及实际线程数核查。 |
