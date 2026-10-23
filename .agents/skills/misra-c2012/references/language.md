# R1–R7：语言、未使用代码、命名与常量

适用条件用于查找构造；缺少某构造须通过实际源文件和配置证明。每行核查均需要真实证据，工具诊断只是其中一部分。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R1.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule1.1.html) | Required | 2012 | C90/C99/C11；全部翻译单元 | 生成合法 C99 声明与表达式；规模不超目标翻译限制。 | 严格目标编译、语言约束与翻译规模检查。 |
| [R1.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule1.2.html) | Advisory | 2012 | C90/C99/C11；编译器扩展 | 优先标准 C99；平台扩展集中并说明必要性。 | 扩展语法、内建函数和 pragma 清单。 |
| [R1.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule1.3.html) | Required | 2012 | C90/C99/C11；全项目 | 排除未定义及会影响结果的关键未指定行为。 | 数据流、求值顺序、生命周期与边界分析。 |
| [R1.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule1.4.html) | Required | AMD2 | C11；Annex K 边界检查接口 | AMD3/AMD4 已解除对泛型、无返回、对齐、线程和原子的本条总禁令；仍不使用 Annex K 设施（允许将 __STDC_WANT_LIB_EXT1__ 定义为0）。其他规则和项目 C99 目标继续成立。 | 语言功能清单及修订后的许可范围。 |
| [R1.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule1.5.html) | Required | AMD3 | C99/C11；过时语言形式 | 按所选 C 标准的过时特性及 AMD3 附录F逐项检查语言和库设施，替换旧式声明等构造。 | 按适用 C 标准的 obsolescent 列表检查。 |
| [R2.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.1.html) | Required | 2012 | C90/C99/C11；控制流 | 修正无法执行的分支或错误条件，保留必要配置路径的证明。 | 可达性分析，覆盖实际构建变体。 |
| [R2.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.2.html) | Required | 2012 | C90/C99/C11；语句和表达式 | 删除可执行但无行为效果的操作；初始化不按赋值当作 dead code，结果被使用的 cast 及向 void 转换按允许形式判断。 | 影响分析，区分不可达与可达但无效果代码。 |
| [R2.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.3.html) | Advisory | 2012 | C90/C99/C11；类型声明 | 仅维护实现或接口需要的 typedef，避免生成多余类型。 | 所有作用域及受支持配置中的引用核查。 |
| [R2.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.4.html) | Advisory | 2012 | C90/C99/C11；struct/union/enum 标签 | 清理没有实际用途的标签，保留标准接口必需部分的理由。 | 标签引用分析。 |
| [R2.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.5.html) | Advisory | 2012 | C90/C99/C11；宏定义 | 清理无用途宏，不能误删在其他配置或生成器使用的符号。 | 检查全部配置中的使用；#undef 也算使用，不能仅凭缺少展开判 unused。 |
| [R2.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.6.html) | Advisory | 2012 | C90/C99/C11；标签 | 删除未被跳转使用的普通标签。 | 标签引用及控制流检查。 |
| [R2.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.7.html) | Advisory | 2012 | C90/C99/C11；参数 | 接口允许时移除无用参数；ABI 固定时保留并解释。 | 实际参数用途与外部签名契约。 |
| [R2.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule2.8.html) | Advisory | AMD4 | C90/C99/C11；对象定义 | 不生成无用途存储对象；必要外部入口/配置对象有用途依据。 | 定义与使用关系、链接及配置检查。 |
| [R3.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule3.1.html) | Required | 2012 | C90/C99/C11；注释 | 避免嵌套注释标记；URL 按 AMD4 的精确例外核对。 | 词法检查及 URL 例外审查。 |
| [R3.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule3.2.html) | Required | 2012 | C99/C11；行注释 | 不在行注释末尾放会继续拼接下一行的反斜杠。 | 预处理前的行拼接检查。 |
| [R4.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule4.1.html) | Required | 2012 | C90/C99/C11；字符串/字符转义 | 明确终结数值转义，必要时拆分字面量以防后续字符被吞入。 | 字符串词法与生成文本检查。 |
| [R4.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule4.2.html) | Advisory | 2012 | C90/C99/C11；源码字符组合 | 避免会被解释为 trigraph 的字符序列。 | 原始源码及生成输出的词法扫描。 |
| [R5.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.1.html) | Required | 2012 | C90/C99/C11；外部符号 | 生成全项目唯一且符合目标有效字符数的外部名字。 | 目标链接器/编译器有效字符规则及符号表。 |
| [R5.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.2.html) | Required | 2012 | C90/C99/C11；同一作用域/命名空间 | 避免目标截断后碰撞的局部标识符。 | 有效字符范围内的名称比较。 |
| [R5.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.3.html) | Required | 2012 | C90/C99/C11；嵌套作用域 | 使用不同职责名称，避免内部声明遮蔽外层标识符。 | 作用域解析和 shadow 检查。 |
| [R5.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.4.html) | Required | 2012 | C90/C99/C11；宏命名 | 避免宏及其参数在有效字符范围内产生命名冲突。 | 预处理符号与目标有效长度检查。 |
| [R5.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.5.html) | Required | 2012 | C90/C99/C11；宏与普通名字 | 宏采用独立命名约定，避免与对象/函数名字碰撞。 | 宏展开前后的符号集合比对。 |
| [R5.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.6.html) | Required | 2012 | C90/C99/C11；typedef 名称 | 为类型名称建立项目范围的唯一命名。 | 跨文件 typedef 索引。 |
| [R5.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.7.html) | Required | 2012 | C90/C99/C11；标签名 | struct/union/enum 标签使用独立且唯一的名字。 | 标签索引与可见范围检查。 |
| [R5.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.8.html) | Required | 2012 | C90/C99/C11；外部链接名字 | 外部接口命名不复用到其他对象/函数定义。 | 全项目符号及链接冲突检查。 |
| [R5.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule5.9.html) | Advisory | 2012 | C90/C99/C11；内部链接名字 | 内部函数/对象仍选择明确唯一的名字，减少跨模块歧义。 | 项目级 static 符号索引。 |
| [R6.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule6.1.html) | Required | 2012 | C90/C99/C11；位域 | 显式选用规范允许、目标支持的位域类型，不依赖默认符号性。 | 类型、宽度及实现定义依据。 |
| [R6.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule6.2.html) | Required | 2012 | C90/C99/C11；单比特命名位域 | 用合适的无符号表示承载单比特值。 | 位域声明及可表示值检查。 |
| [R6.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule6.3.html) | Required | AMD3 | C90/C99/C11；union 内位域 | 不直接把位域声明为 union 成员；非 union 成员内部子对象的位域不受本条直接禁止，仍核对其他布局规则。 | 聚合定义和生成类型检查。 |
| [R7.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.1.html) | Required | 2012 | C90/C99/C11；数值字面量 | 用十进制或明确十六进制表示，避免前导零造成八进制误读。 | 字面量词法检查，区分字符转义。 |
| [R7.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.2.html) | Required | 2012 | C90/C99/C11；无符号常量 | 对表示为无符号类型的整数常量用正确 U 后缀。 | 按目标候选类型判断，不能只看赋值目标。 |
| [R7.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.3.html) | Required | 2012 | C90/C99/C11；long 后缀 | 用大写 L，避免小写 l 与数字 1 混淆。 | 字面量后缀扫描。 |
| [R7.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.4.html) | Required | 2012 | C90/C99/C11；字符串字面量引用 | 通过 const 字符指针访问不可修改的字面量。 | 指针声明、赋值和写入路径检查。 |
| [R7.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.5.html) | Mandatory | AMD3 | C99/C11；stdint 常量宏 | 实参使用无后缀整数字面量且数值能由宏所指精确宽度类型表示；负号放在宏外。 | 宏原始实参、值域与展开检查，并结合 R7.1/R7.6。 |
| [R7.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.6.html) | Required | AMD4 | C99/C11；小宽度常量宏 | 按目标 int 宽度避免 small integer 常量宏；32 位 int 下涉及 INT8_C/UINT8_C/INT16_C/UINT16_C。 | 同时检查目标更宽时的 INT32_C/UINT32_C、宏包装和展开；不重定义保留宏规避诊断。 |
