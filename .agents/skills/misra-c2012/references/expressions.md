# R10–R14：essential type、指针转换与求值

先区分 C 标准类型与 essential type；例外不能仅凭“编译器允许”推导。目标宽度、复合表达式与线程交错都影响判断。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R10.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.1.html) | Required | 2012 | 运算符操作数 | 按用途分离布尔、字符、枚举与数值；对每种运算检查 essential type 合法性。 | 运算符约束表与类型模型，包含逻辑指针用法。 |
| [R10.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.2.html) | Required | 2012 | 字符算术 | 字符只作有明确字符语义的偏移/差值，不当作通用整数。 | 字符类型参与的加减操作及允许组合。 |
| [R10.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.3.html) | Required | 2012 | 赋值/参数/返回 | 不把结果隐式压窄或换 essential 类别；范围充分也要核对允许例外。 | 来源/目标宽度、类别及常量例外。 |
| [R10.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.4.html) | Required | 2012 | 通常算术转换 | 运算前统一合适的 essential 类别，不靠隐式 signed/unsigned 混算。 | 二元运算和比较的双侧类型。 |
| [R10.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.5.html) | Advisory | 2012 | 显式转换 | 只作语义合理且范围成立的转换；不要靠 cast 掩盖类别错误。 | 全部 cast 的源/目标类别与目的。 |
| [R10.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.6.html) | Required | 2012 | 复合表达式赋值 | 在计算前选定所需宽度，避免算完后借宽目标扩大结果。 | 复合表达式 essential 类型与赋值目标。 |
| [R10.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.7.html) | Required | 2012 | 复合表达式参与运算 | 避免窄复合结果再与更宽操作数混算，先设计统一运算域。 | 表达式树和参与通常算术转换的类型。 |
| [R10.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.8.html) | Required | 2012 | 复合结果 cast | 不把完整复合结果转换到更宽或不同 essential 类别来补救。 | 区分单操作数转换与复合表达式转换。 |
| [R11.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.1.html) | Required | 2012 | 函数指针转换 | 函数指针与其他类型隔离，回调通过兼容原型连接。 | 回调赋值和显式/隐式转换路径。 |
| [R11.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.2.html) | Required | 2012 | 不完整类型指针 | 用正确不透明类型接口，不通过其他类型重解释内部对象。 | 转换两端未限定指向类型及允许例外。 |
| [R11.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.3.html) | Required | 2012 | 对象指针转换 | 不通过不同对象类型指针进行布局重解释。 | 类型、对齐、别名规则与精确例外。 |
| [R11.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.4.html) | Advisory | 2012 | 对象指针与整数 | 优先保留指针语义；硬件地址适配限定目标并说明必要偏离。 | 双向转换、地址宽度和平台依据。 |
| [R11.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.5.html) | Advisory | 2012 | void 指针恢复 | 优先使用有类型接口，恢复对象类型需有明确依据。 | void* 来源、目标和所有权约束。 |
| [R11.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.6.html) | Required | 2012 | void 指针与算术类型 | 不以 void* 作为任意数值的中转容器。 | cast 和隐藏在宏中的互转。 |
| [R11.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.7.html) | Required | 2012 | 对象指针与非整数算术 | 禁止用浮点/复数表示地址，保持类别分离。 | 指针与浮点/复数转换扫描。 |
| [R11.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.8.html) | Required | 2012 | 指向类型限定符 | 保留 const、volatile 和适用的 atomic 限定，接口不要丢弃约束。 | 各层指向类型的显式/隐式转换。 |
| [R11.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.9.html) | Required | 2012 | 空指针常量 | 使用项目支持的 NULL 表达整数空指针常量；聚合零初始化核对例外。 | 空值赋值、参数和聚合初始化。 |
| [R11.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.10.html) | Required | AMD4 | C11 atomic 类型 | 不对不完整的 void 类型施加原子资格。 | _Atomic 声明及 typedef 展开。 |
| [R12.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.1.html) | Advisory | 2012 | 混合运算符 | 用恰当括号表达计算顺序，避免读者猜优先级。 | 表达式优先级及 _Alignof 等修订要求。 |
| [R12.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.2.html) | Required | 2012 | 移位 | 在移位前证明位数处于左侧 essential 宽度的有效范围。 | 路径范围分析，不能只用提升后 int 宽度。 |
| [R12.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.3.html) | Advisory | 2012 | 逗号表达式 | 拆成独立语句；区分分隔符与逗号运算符。 | AST 扫描，不靠字符搜索定论。 |
| [R12.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.4.html) | Advisory | 2012 | 无符号常量运算 | 生成计算正确且不回绕的常量，必要时在操作数阶段选宽类型。 | 目标模型下的常量求值。 |
| [R12.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.5.html) | Mandatory | AMD1 | 数组形式参数 | 容量来自契约/显式参数，不以 sizeof 数组形参计算元素数。 | 形参声明与 sizeof 操作数关联。 |
| [R12.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule12.6.html) | Required | AMD4 | C11 原子聚合 | 通过整个原子对象的适用操作读取/更新，避免直访成员。 | 原子 struct/union 成员访问。 |
| [R13.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.1.html) | Required | 2012 | 初始化列表 | 初始化值计算不携带持续副作用，先求值再初始化。 | 函数、volatile、修改操作的副作用分析。 |
| [R13.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.2.html) | Required | 2012 | 有副作用表达式/并发 | 不依赖求值次序或线程交错得到预期结果，建立独立语句和同步。 | 所有允许次序、别名与交错分析。 |
| [R13.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.3.html) | Advisory | 2012 | 递增/递减 | 含 ++/-- 的完整表达式不混入额外副作用。 | full expression 边界与调用副作用。 |
| [R13.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.4.html) | Advisory | 2012 | 赋值表达式 | 赋值独立执行，再使用变量值判断或计算。 | 赋值结果是否被其他运算消费。 |
| [R13.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.5.html) | Required | 2012 | 短路逻辑 | &&/逻辑或右侧避免持续副作用，不让执行与否改变隐藏状态。 | 调用副作用、volatile 读取及路径检查。 |
| [R13.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.6.html) | Required | 2012 | sizeof | sizeof 内不放潜在副作用，即使当前目标不求值。 | 操作数树及 VLA 相关语义。 |
| [R14.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule14.1.html) | Required | 2012 | 循环计数 | 用整数等适当计数方式，避免浮点误差决定迭代次数。 | 计数器 essential 类别与更新。 |
| [R14.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule14.2.html) | Required | 2012 | for 循环 | 明确初始化、结束条件和单一可理解的计数推进。 | 三个子句、循环体修改和修订允许形式。 |
| [R14.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule14.3.html) | Required | 2012 | 控制条件 | 修正永真/永假条件，必要永循环等允许形式另核对。 | 常量传播和条件的实际输入域。 |
| [R14.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule14.4.html) | Required | 2012 | if/迭代控制 | 将数值/指针状态明确比较成布尔条件。 | 控制表达式的 essential Boolean 类型。 |
