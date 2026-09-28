# R18–R20：数组、存储重叠与预处理

检查源码和所有实际宏配置；防重包含和 MemMap 等协议必须按其实际义务判断。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R18.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.1.html) | Required | 2012 | 指针算术 | 保持在所属数组允许位置内；一过尾可构造但不可解引用。 | 基对象、长度、偏移及路径范围证明。 |
| [R18.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.2.html) | Required | 2012 | 指针相减 | 只对同一数组的有效相关位置求差。 | 两端基对象关系和 ptrdiff_t 范围。 |
| [R18.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.3.html) | Required | 2012 | 指针有序比较 | 仅在规范允许的同对象关系内比较大小。 | 两端对象来源及结构成员等精确例外。 |
| [R18.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.4.html) | Advisory | 2012 | 指针加减 | 优先有界下标访问，避免复杂指针推进。 | 指针 +、-、+=、-= 及替代方案。 |
| [R18.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.5.html) | Advisory | 2012 | 多层指针 | 简化超过两层的间接结构，必要设计保留理由。 | typedef 展开后的真实指针层数。 |
| [R18.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.6.html) | Required | 2012 | 自动/线程局部对象地址 | 不把短命对象地址保留到更长生命周期的对象或回调中。 | 地址逃逸、返回、全局赋值和异步使用。 |
| [R18.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.7.html) | Required | 2012 | 柔性数组成员 | 用明确容量和长度的数据模型，避免 flexible array member。 | 类型布局和生成声明。 |
| [R18.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.8.html) | Required | 2012 | VLA | 使用编译期容量或调用方缓冲区，不按运行值声明数组。 | 声明边界是否为整数常量表达式。 |
| [R18.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.9.html) | Required | AMD3 | 临时寿命对象 | 不让临时对象中的数组衰减为可能失效的指针。 | 值返回聚合中的数组访问和临时寿命。 |
| [R18.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.10.html) | Mandatory | AMD4 | 可变修改数组指针 | 不通过指向可变修改数组的指针绕过 VLA 限制。 | 参数/typedef 的 variably-modified 类型。 |
| [R19.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule19.1.html) | Mandatory | 2012 | 赋值/复制 | 证明源和目标满足非重叠要求；准确区分标准允许的复制形式。 | 内存区间与自赋值/库函数例外。 |
| [R19.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule19.2.html) | Advisory | 2012 | union | 优先明确的标记/字段模型，避免靠 union 重解释数据。 | union 用途、成员状态及替代设计。 |
| [R20.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.1.html) | Advisory | 2012 | include 布局 | 在源文件主体声明前组织 includes，不在代码中间插入普通头文件。 | 原始源码顺序及内存映射必要理由。 |
| [R20.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.2.html) | Required | 2012 | 头文件路径 | 使用可移植、无特殊注释或引用字符的文件名。 | include 路径字面量，区分定界符与文件名。 |
| [R20.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.3.html) | Required | 2012 | include 语法 | 使用标准的尖括号或双引号包含形式。 | 预处理后的 include token。 |
| [R20.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.4.html) | Required | 2012 | 宏名字 | 不用宏覆盖 C 关键字。 | 预处理定义集合。 |
| [R20.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.5.html) | Advisory | 2012 | undef | 减少撤销宏；MemMap 切换等必要协议按有效类别说明。 | undef 位置、生命周期与项目重分类。 |
| [R20.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.6.html) | Required | 2012 | 宏调用实参 | 不在宏实参中嵌入形似预处理指令的 token。 | 原始 token 流而非仅展开结果。 |
| [R20.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.7.html) | Required | 2012 | 宏参数展开 | 保护形成表达式的参数展开，避免运算优先级改变意义。 | 展开上下文，声明类宏等例外须核对。 |
| [R20.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.8.html) | Required | 2012 | if/elif 预处理条件 | 写成明确的 0/1 结果，不依赖其他非零值被视为真。 | 所有构建配置下预处理常量求值。 |
| [R20.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.9.html) | Required | 2012 | 预处理条件名字 | 使用前定义配置符号，不靠未定义名字自动视为零。 | 宏集合、defined 的精确允许用法。 |
| [R20.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.10.html) | Advisory | 2012 | 宏拼接/字符串化 | 优先普通声明/函数；需要 #/## 时解释不可替代性。 | 宏定义及全部调用展开。 |
| [R20.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.11.html) | Required | 2012 | # 与 ## 邻接 | 不让同参数的字符串化和紧邻拼接产生歧义序列。 | 替换列表 token 顺序。 |
| [R20.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.12.html) | Required | 2012 | 可继续替换的宏参数 | 对参与 #/## 的参数保持统一使用方式，避免同时普通展开造成不一致。 | 参数在替换列表的所有出现位置。 |
| [R20.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.13.html) | Required | 2012 | 预处理行 | 以 # 开始的行使用有效指令，不借此放置任意语法。 | 原始源码指令及严格预处理。 |
| [R20.14](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.14.html) | Required | 2012 | 条件编译跨文件 | 在同一文件中匹配条件指令，不把结束指令放到被包含文件。 | 文件级 if/elif/else/endif 栈。 |
