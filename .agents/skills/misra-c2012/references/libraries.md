# R21：标准库边界

受限库使用要定位到实际调用，不把“主机文件”一概排除。R21.6 等必要 Required 偏离仍需批准；其他资源规则即使采用库偏离也继续核对。

| 编号与参考 | 类别 | 新增于 | 适用条件 | 编码动作 | 核查方式 |
| --- | --- | --- | --- | --- | --- |
| [R21.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.1.html) | Required | 2012 | 宏操作 | 不定义或撤销实现/标准库保留的名字。 | 标准和实现的保留宏集合。 |
| [R21.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.2.html) | Required | 2012 | 声明命名 | 应用接口不占用标准/实现保留名字。 | 命名空间、下划线规则和库名称。 |
| [R21.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.3.html) | Required | 2012 | 标准分配函数 | 采用有界存储，避免 malloc/calloc/realloc/free 等相关调用。 | 直接/包装调用及 aligned_alloc 修订。 |
| [R21.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.4.html) | Required | 2012 | setjmp 机制 | 采用明确返回和状态处理，避免 setjmp/longjmp 跨越正常资源路径。 | 头文件和间接包装。 |
| [R21.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.5.html) | Required | 2012 | 标准信号机制 | 不引入 signal.h 的异步处理模式，平台机制另建立契约。 | 标准头文件/函数与平台适配范围。 |
| [R21.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.6.html) | Required | 2012 | 标准 I/O | 产品代码优先明确适配接口；必要主机协议/文件 I/O 记录限定范围偏离。 | 全部 stdio 调用及修订后的允许例外。 |
| [R21.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.7.html) | Required | 2012 | 简易数值转换 | 使用可检测错误和范围的转换方法，避免 atoi/atol/atoll/atof。 | 转换入口与端点、范围、errno 处理。 |
| [R21.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.8.html) | Required | 2012 | 标准终止函数 | 普通模块通过明确错误路径返回，避免 abort/exit/_Exit/quick_exit。 | 调用图、主机启动边界及必要偏离。 |
| [R21.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.9.html) | Required | 2012 | 库排序/查找 | 使用已论证的有界算法，避免 bsearch/qsort 黑盒契约。 | 调用入口、比较器及算法上界。 |
| [R21.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.10.html) | Required | 2012 | 标准日期时间 | 使用项目可控时间契约，避免标准时间/日期库。 | 头文件、函数及时间来源。 |
| [R21.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.11.html) | Advisory | 2012 | tgmath | 可行时选用明确类型的数学函数，避免隐含泛型分派。 | 头文件和类型泛型宏，类别为 Advisory。 |
| [R21.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.12.html) | Required | 2012 | fenv | 不通过标准浮点环境 API 隐式改变计算模型。 | 头文件和环境状态调用；修订类别为 Required。 |
| [R21.13](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.13.html) | Mandatory | AMD1 | ctype | 实参先保持 EOF 或有效 unsigned char 值，不把负 signed char 直接传入。 | 输入范围，避免提前转换破坏 EOF。 |
| [R21.14](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.14.html) | Required | AMD1 | 字符串比较 | 用符合字符串契约的比较逻辑，不用 memcmp 代替零终结字符串比较。 | 字符串/字节块角色及长度。 |
| [R21.15](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.15.html) | Required | AMD1 | 内存操作两端类型 | memcpy/memmove/memcmp 的两端对象采用兼容的指向类型。 | 类型展开与允许例外，不能仅因 void* 形参接受就放行。 |
| [R21.16](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.16.html) | Required | AMD1 | memcmp 数据类型 | 仅对允许的 essential 类别或指针类型作该类比较。 | 指向对象类别和表示/填充字节风险。 |
| [R21.17](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.17.html) | Mandatory | AMD1 | 字符串处理 | 证明读写和终结符都在真实对象边界内。 | 每个字符串参数实际容量及终结位置。 |
| [R21.18](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.18.html) | Mandatory | AMD1 | string.h 长度 | 长度参数同时满足操作语义和所有涉及对象容量。 | n 的范围、元素/字节单位及多个缓冲区。 |
| [R21.19](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.19.html) | Mandatory | AMD1 | 库返回的内部数据 | 将相关库内部返回值视为只读，不能修改其存储。 | localeconv/getenv/setlocale/strerror 返回值写入路径。 |
| [R21.20](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.20.html) | Mandatory | AMD1 | 库内部存储寿命 | 下一次相关调用前复制必要数据，不继续使用失效的内部返回指针。 | asctime/ctime/gmtime/localtime 等相关调用顺序。 |
| [R21.21](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.21.html) | Required | AMD2 | 系统命令 | 不通过 system 启动 shell 命令，采用明确平台接口与独立授权。 | 直接/包装调用及输入来源。 |
| [R21.22](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.22.html) | Mandatory | AMD3 | tgmath 操作数 | 若保留类型泛型数学，给操作数选择允许的 essential 类别。 | 宏实参类型及适用标准。 |
| [R21.23](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.23.html) | Required | AMD3 | 多参 tgmath | 同一多参数泛型调用采用一致的标准类型。 | 所有实参的标准类型而非仅 essential 类别。 |
| [R21.24](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.24.html) | Required | AMD3 | 随机函数 | 不使用标准 rand/srand 类接口承载需可靠性的行为。 | 标准调用清单与项目随机源契约。 |
| [R21.25](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.25.html) | Required | AMD4 | C11 内存同步 | 对相关同步操作使用顺序一致内存序，不悄悄放宽为 relaxed。 | 显式 memory_order、默认操作及包装语义。 |
| [R21.26](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.26.html) | Required | AMD4 | C11 定时互斥锁 | mtx_timedlock 仅接受支持定时锁的已初始化互斥对象。 | 初始化类型标志和实际调用对应关系。 |
