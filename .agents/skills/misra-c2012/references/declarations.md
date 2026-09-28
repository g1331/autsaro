# R8–R9：声明、链接与初始化

全项目比较使用实际交付头文件和生成工程；内部链接建议不能成为破坏公共 ABI 的理由。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R8.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.1.html) | Required | 2012 | 所有声明 | 显式写出对象和函数的完整类型，不依赖隐式 int。 | AST 与严格 C99 编译。 |
| [R8.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.2.html) | Required | 2012 | 函数声明/定义 | 使用带参数名字的原型；无参数用 void。 | 头文件、定义及函数指针类型检查。 |
| [R8.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.3.html) | Required | 2012 | 重复声明/定义 | 参数名、类型及限定符保持一致，不制造同接口不同描述。 | 跨翻译单元声明对照。 |
| [R8.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.4.html) | Required | 2012 | 外部链接定义 | 在定义前包含提供兼容声明的所属头文件。 | 实际包含链和定义位置检查。 |
| [R8.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.5.html) | Required | 2012 | 外部接口声明 | 每个外部接口由单一所属头文件声明，调用者包含它。 | 预处理前声明所有权审查。 |
| [R8.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.6.html) | Required | 2012 | 外部链接定义 | 为每个外部符号提供唯一的真实定义。 | 链接符号表、库与生成对象检查。 |
| [R8.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.7.html) | Advisory | 2012 | 外部链接可见性 | 仅在一个翻译单元使用的内部对象/函数优先收窄链接。 | 调用方和 ABI 清单，保留真正外部入口。 |
| [R8.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.8.html) | Required | 2012 | 内部链接声明 | static 声明和定义一致，避免依赖隐含链接继承。 | 全部前向声明与定义检查。 |
| [R8.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.9.html) | Advisory | 2012 | 单函数对象 | 仅一函数使用的对象优先放该块作用域，保留所需存储期。 | 名称使用范围与初始化生命周期。 |
| [R8.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.10.html) | Required | 2012 | inline 函数 | C99 inline 辅助函数采用明确的内部链接形式。 | static inline 声明和链接检查。 |
| [R8.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.11.html) | Advisory | 2012 | 外部数组声明 | 在接口中表达真实数组大小，不隐藏必需容量。 | 声明/定义的边界一致性。 |
| [R8.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.12.html) | Required | 2012 | 枚举常量 | 检查隐式枚举值不会与同列表其他值碰撞。 | 常量求值及显式/隐式混用检查。 |
| [R8.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.13.html) | Advisory | 2012 | 指针限定 | 无需修改目标对象时使用 const 指向类型。 | 写入路径与接口可修改性分析。 |
| [R8.14](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.14.html) | Required | 2012 | restrict | 不随意添加 restrict 优化承诺；使用时须处理实际约束和偏离。 | 声明及别名契约扫描。 |
| [R8.15](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.15.html) | Required | AMD3 | C11 显式对齐 | 同一对象的各声明统一对齐要求。 | 宏展开后的 _Alignas 与所有声明。 |
| [R8.16](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.16.html) | Advisory | AMD3 | C11 显式对齐 | 移除没有实际对齐意义的零对齐说明。 | 对齐常量求值。 |
| [R8.17](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule8.17.html) | Advisory | AMD3 | C11 显式对齐 | 对单个声明采用单一明确对齐规格。 | 各声明对齐说明数量检查。 |
| [R9.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.1.html) | Mandatory | 2012 | 自动存储对象 | 所有读取路径先建立确定值，包括错误分支。 | 路径敏感的未初始化读取分析。 |
| [R9.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.2.html) | Required | 2012 | 聚合/union 初始化 | 用与嵌套布局对应的初始化层级；零初始化例外单独核对。 | 类型布局与初始化树，不能机械展开全部零。 |
| [R9.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.3.html) | Required | 2012 | 数组初始化 | 保证每个元素初始状态确定；隐式零填充的允许形式按原文核对。 | 指定初始化、稀疏配置及全部元素映射。 |
| [R9.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.4.html) | Required | 2012 | 指定初始化 | 同一元素只接受一个明确初始化来源。 | 展开 designator 并检查重复目标。 |
| [R9.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.5.html) | Required | 2012 | 指定数组初始化 | 明确声明数组容量，不靠最高 designator 隐式推算。 | 声明边界与 designator 最大下标。 |
| [R9.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.6.html) | Required | AMD4 | 链式 designator | 链式指定初始化不夹杂未指定位置的初始化项。 | 初始化列表逐项检查。 |
| [R9.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.7.html) | Mandatory | AMD4 | C11 atomic 对象 | 建立有效原子初始化后才发布或读取对象。 | 初始化、发布和首次访问顺序。 |
