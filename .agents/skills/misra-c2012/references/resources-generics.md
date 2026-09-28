# R22–R23：资源、线程生命周期与泛型选择

没有使用相应资源/语言构造时，仍需记录其缺席依据。平台线程不得仅因 C99 编译选项就免除并发风险分析。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R22.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.1.html) | Required | 2012 | 标准库动态资源 | 每条退出路径显式释放获得的资源，失败路径同样完整。 | 获取/转移/释放图，含文件与分配器。 |
| [R22.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.2.html) | Mandatory | 2012 | 标准 free | 仅释放由对应分配机制获得的有效块，不释放栈、静态、内部或已释放地址。 | 分配来源、基指针和释放次数。 |
| [R22.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.3.html) | Required | 2012 | 多个文件流 | 同一文件不要同时通过不同流分别读写。 | 文件身份而非路径字符串，流生命周期对照。 |
| [R22.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.4.html) | Mandatory | 2012 | 只读文件流 | 写操作只作用于具有写权限的打开模式。 | fopen 模式到后续写操作的数据流。 |
| [R22.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.5.html) | Mandatory | 2012 | FILE 指针 | 使用 stdio 操作接口，不能直接解引用或查看 FILE 布局。 | FILE* 解引用、成员访问与别名。 |
| [R22.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.6.html) | Mandatory | 2012 | 已关闭文件流 | 关闭后使所有引用失效，不再次调用其流操作。 | fclose 路径、别名和回调保存。 |
| [R22.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.7.html) | Required | AMD1 | 可能返回 EOF 的函数 | 先保留完整原始返回类型，再与 EOF 比较，之后才转换字符。 | 返回值转换和比较顺序。 |
| [R22.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.8.html) | Required | AMD1 | 会设置 errno 的调用 | 调用前清零 errno，避免旧错误混入当前判断。 | 紧邻调用前的控制/数据流。 |
| [R22.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.9.html) | Required | AMD1 | 会设置 errno 的调用 | 调用后按函数契约检查 errno 是否非零，并处理错误。 | 返回值与 errno 共同契约、失败路径。 |
| [R22.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.10.html) | Required | AMD1 | errno 检查 | 只根据刚执行的相关函数解释 errno，避免中间调用污染。 | 最近调用及 errno 捕获时机。 |
| [R22.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.11.html) | Required | AMD4 | C11 join/detach | 成功 join 或 detach 后不再重复执行两种操作。 | 线程身份、返回结果与生命周期状态。 |
| [R22.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.12.html) | Mandatory | AMD4 | C11 线程/同步/TSS 对象 | 通过对应标准接口操作不透明资源，不直接解释内部存储。 | 对象访问方式和所有别名。 |
| [R22.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.13.html) | Required | AMD4 | C11 线程资源存储 | 为对象提供覆盖所有使用线程的存储期。 | 创建、线程退出、最后使用与作用域结束。 |
| [R22.14](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.14.html) | Mandatory | AMD4 | C11 同步对象 | 成功初始化互斥锁/条件变量后才使用。 | 初始化返回值与每条访问路径。 |
| [R22.15](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.15.html) | Required | AMD4 | C11 同步/TSS 销毁 | 等全部使用者线程结束后再销毁共享同步与线程局部存储资源。 | 使用者集合、终止顺序及销毁时机。 |
| [R22.16](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.16.html) | Required | AMD4 | C11 mutex 锁持有 | 获得锁的线程负责明确解锁，各失败分支也保持配对。 | 所有锁/解锁路径及线程身份。 |
| [R22.17](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.17.html) | Required | AMD4 | C11 解锁/条件等待 | 解锁或调用条件等待前确认当前线程已持有对应锁。 | 持锁状态到 mtx_unlock/cnd_wait/cnd_timedwait。 |
| [R22.18](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.18.html) | Required | AMD4 | C11 非递归锁 | 防止同线程重入或回调再次获取非递归互斥锁。 | 锁类型与间接调用重入图。 |
| [R22.19](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.19.html) | Required | AMD4 | C11 条件变量 | 同一条件变量只关联一把明确互斥锁。 | 所有等待入口的条件变量/锁配对。 |
| [R22.20](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule22.20.html) | Mandatory | AMD4 | C11 TSS | 成功创建线程特定存储标识后才读取/写入。 | tss_create 结果和所有 tss_get/tss_set 路径。 |
| [R23.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.1.html) | Advisory | AMD3 | C11 _Generic | 将泛型分派集中于有契约的宏，避免散落调用逻辑。 | 泛型选择在宏定义/展开中的位置。 |
| [R23.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.2.html) | Required | AMD3 | C11 非宏 _Generic | 控制表达式不携带潜在副作用，即使语言不执行它。 | 修改、volatile 和有副作用调用。 |
| [R23.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.3.html) | Advisory | AMD3 | C11 _Generic | 至少提供实际非 default 类型分派，避免无意义泛型。 | association 列表。 |
| [R23.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.4.html) | Required | AMD3 | C11 _Generic | association 使用允许且不重复兼容的适当类型。 | 完整对象类型、限定符、数组/指针与约束。 |
| [R23.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.5.html) | Advisory | AMD3 | C11 _Generic | 不依靠隐式指针类型转换来决定泛型分支。 | 控制表达式原始类型和候选指针类型。 |
| [R23.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.6.html) | Required | AMD3 | C11 _Generic | 控制表达式的 essential 与标准类型相匹配。 | 常量、枚举、窄类型与类型模型对照。 |
| [R23.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.7.html) | Advisory | AMD3 | C11 泛型宏 | 宏对实参的运行求值只发生一次，防止副作用重复。 | 控制表达式与选中分支联合展开分析。 |
| [R23.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule23.8.html) | Required | AMD3 | C11 _Generic | default association 固定在列表首端或末端。 | association 顺序。 |
