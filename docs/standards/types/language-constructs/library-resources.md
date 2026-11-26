# 标准库与资源的当前义务

## 输入与原文

第六批沿 #512，基线 master `cf8ddc4f`；实际输入为最近成功 master 必需
[run 43962056190](https://github.com/g1331/autsaro/actions/runs/43962056190)
（`755a002e`）的双目标四 profile。八份工程的 HostBatch／probe 共 888 个 C99 TU；
GCC -MD 并集、Cppcheck 文件回执和最终 patchedLines 沿 ADR 0030 选择目标自有、
生成／应用和项目补丁。1,442 个自有文件实例含目标 TU 包含的 Ecu_HostBatch.h；
主机实现和 port 沿原范围，SDK／内核原行沿 ADR 0025。
其后 master 仅 Windows host stack 的生产变化在范围外，原件身份与 producer 增量另核。

七份固定原件通过 SHA256SUMS。逐条读取主文 R21.1–21.12、R22.1–22.6
（印刷 165–177／PDF 173–185），AMD1 §2 R21.13–21.20、R22.7–22.10
（印刷 4–16／PDF 12–24）的正文、amplification、例外、理由和例子。
合成 TC1 的 R21.1／21.2／21.7／21.9 勘误、AMD2.27／78–85、
AMD3.40–48／67–70 和 TC2.36–38；AMD4 新增的并发条目归原 66 项，
其追加交叉引用没有改变本表 C99 义务。R21.8 仍在固定 11 项组。

物理扫描逐字符保留续行到物理行的映射，去注释保留其中全部换行；组合反例包含
多行注释、注释内伪调用、宏续行和续行行注释。每条记录均回查原文件对应标识符或
指令，再按该物理行选择 patchedLines。检查完整标识符引用，含取址、替换表及未选
分支，另按 GCC 行标记核对 452 份实际展开；仅库实现内部的声明／宏保持其 SDK 来源。
固定 e73bf44 的同号 addon、executeCheck 及核心规则映射逐项核对，能力边界如下表。

实际库引用集中于 Com.c:238／255 的 memcpy、Os_Mailbox.c:126／127 的
memset／memcpy、Os_Backend.c:2390 的 memset、Ecu_HostBatch.h:65 的 memset。
Os_Backend.c:1730–1736 的 getenv／strtoul／errno 属 OS_HOST_FAILURE_TESTS
条件分支，四 profile 正常展开未选；物理分支仍核对 const、清零、检查及最后消费者。
没有把主机目录里的同类调用移入满足证据，也没有因头文件位于 integration 而漏掉它。

本批结算 28 条 satisfied；R21.1／R21.2 的完整保留名称／声明集合证据转入
[#539](https://github.com/g1331/autsaro/issues/539)，两条保持 not_assessed，继续归 #443。
本次新增例外批准为零。原始诊断、passed／error、程序退出码、221 项 assessment 和
completeCompliance 保持原义；全组为 71 satisfied、3 not_applicable、70 not_assessed。

## R21.1／R21.2 保留名称接续核查

[#539](https://github.com/g1331/autsaro/issues/539) 取 master `dc1ce02b` 的
[必需 CI 工件](https://github.com/g1331/autsaro/actions/runs/44814255256)。
Linux／Windows 各四个 profile，含 HostBatch 和 probe 的 888 个程序 TU 实例，
对应 452 份独立 GCC C99 展开。每份封存源码取 GCC `-MD` 与 Cppcheck 项目文件并集，
集成头按八份目标 TU 的依赖并集判定。最终内核中的项目补丁按物理
`patchedLines` 选取，端口补丁按 ADR 0030 归主机层，内核原行按 ADR 0025 归采用代码。

名称目录从 [WG14 N1256](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1256.pdf)
§7.2–§7.25 建候选，Annex B 辅助定位，按 §7.1.3 和 §6.2.1–3 记录标准中名字的
作用域、名称空间和链接。比较覆盖标准宏、文件作用域普通名与 tag，含
stdint／inttypes 的宽度族。§7.26 Future Library Directions 排除在本规则目录之外。
`NDEBUG` 是 §7.2 允许调用者提供的控制宏，单独核对。MISRA 的前导下划线条件
和 `defined` 指令条件另外检查。比如 `int32_t` 属标准 typedef，
`uint32` 是项目 typedef，`mode` 只是 fopen 的形参名。

R21.1 的物理指令经续行拼接、去注释及原行回查，包含未选分支。八份样本的自有
及目标补丁 define／undef 次数按 profile 为 2,108／2,108／2,252／2,171，
两平台一致，均无保留名命中。活动定义和撤销按 GCC `-dD` 行标记核查，
与 `-dM` 的期末状态交叉检查。Linux 的 `_FORTIFY_SOURCE`、`_REENTRANT`
和 Windows 的 `_REENTRANT` 来自编译器命令行环境。Cppcheck 的发现宏参数
反映 GCC 或头文件提供的名字。两种来源均单独归属。

R21.2 的声明候选按对象、函数、typedef、tag、成员、枚举常量、形参及标签核对，
记录名字位置与其声明作用域。实际 GCC 展开中的项目位置另按标识符核对，
以区分宏生成的声明、标准库引用与自有声明。Linux 的辅助解析覆盖全部 TU，
Windows 的 GCC intrinsic 使辅助解析在工具链头内报错，SDK 的声明全集仍待
用目标 GCC 的声明与作用域记录补齐。项目声明候选未发现当前需要改名的源位置。
`memcpy`／`memset` 是对标准头所声明函数的调用，`__atomic_*_n` 是
GCC 内建引用；Windows `_Interlocked*` 来自主机接口展开。项目范围内所见
`__attribute__`／`__thread` 属编译器语法。未选故障分支的 `errno`、
`getenv`、`strtoul` 也只作引用。

`Os_Backend.c` 的实际 TU 在 Linux 导入 `pthread.h`，在 Windows 导入
`windows.h`。Linux `pthread.h:19` 的 `_PTHREAD_H` guard 和
`pthread.h:548` 的 `__pthread_unwind_buf_t` typedef，以及 Windows
`windows.h:7` 的 `_WINDOWS_` guard 和 `minwinbase.h:13` 的
`_SECURITY_ATTRIBUTES` tag，均随非 ISO SDK 原头进入两种程序。这些声明和
定义按 SDK 来源及集成责任处理。Linux `unwind.h` 与 futex UAPI 的所核实例
从主机实现进入，具体已核位置和待决定事项见
[SDK 供应方边界](../../os/implementation.md#非iso-sdk的保留名字)。
八份报告的这两条原始诊断均为零，`passed=false`、`completeCompliance=false`
和项目验收 `blocked` 照原始报告保留。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R21.1 | Required | 物理 define／undef 与保留名称 | not_assessed | 主文 165–166／PDF 173–174，TC1 补 R20.5 引用，AMD2.27 扩展语言。#539 的八份物理指令与 GCC 活动宏未发现自有保留名定义或撤销，排除 Future Library Directions，NDEBUG 单列。非 ISO SDK 的 _PTHREAD_H／_WINDOWS_ 等实际引入待完成集成处置，继续归 #443。 |
| R21.2 | Required | 对象／类型／tag／函数等声明 | not_assessed | 主文 166–167／PDF 174–175，TC1 修正 reserved macro 标题及 BUILTIN 示例，AMD2.27。使用与声明分开，namespace／scope 回查实际 GCC 项目声明。#539 的非 ISO SDK 声明需补齐 Windows 目标 GCC 的声明集合及集成处置，继续归 #443。 |
| R21.3 | Required | 标准动态分配标识符及宏 | satisfied | 主文 167／PDF 175；AMD2.78 加 aligned_alloc。物理及实际展开没有 calloc／malloc／realloc／aligned_alloc／free 引用、取址或同名替换表。内核原件的 pvPortMalloc／vPortFree 沿采用责任，项目补丁无标准分配入口；主机 stack 实现按 ADR 0030。固定入口只查四个直接调用，独立扫描补 aligned_alloc 和间接入口。 |
| R21.4 | Required | setjmp 头与全部设施 | satisfied | 主文 168／PDF 176；AMD3.67 明确禁止包含 setjmp.h 及其全部设施。物理 include 含引号形式、替换表与实际展开均无 setjmp／longjmp／jmp_buf；GCC／analyzer 文件回执中项目无该包含。目标 Backend 调用 Os_Stack／port 的接缝保留，平台上下文实现沿 ADR 0030；同号 findInclude 的单一尖括号检查由独立证据补齐。 |
| R21.5 | Required | signal 头与 ISO 信号设施 | satisfied | 主文 168／PDF 176；AMD3.68 同时禁止 signal.h 包含和设施。范围内物理／展开无该 include、signal／raise／sig_atomic_t 或 SIG_DFL／SIG_ERR／SIG_IGN 等 ISO 信号宏。POSIX 信号处理集中于 host／port；Os_Backend 的平台适配声明与调用未自行引入 ISO 信号设施。固定入口只找 include，独立引用扫描同时覆盖取址及宏。 |
| R21.6 | Required | stdio 与 wide I/O 标识符 | satisfied | 主文 168–169／PDF 176–177；AMD2.79 覆盖 stdio.h 及 wchar.h 的 wide I/O。范围内物理、宏和实际展开无 printf／snprintf／fread／fwrite 等标准 I/O 名字或间接入口；HostBatch 输出在 integration 的范围外 .c，范围内 Ecu_HostBatch.h 仅有 memset。固定入口跳过部分 function token，当前结论由实际引用补齐，未为内存格式化设例外。 |
| R21.7 | Required | 四种简易转换接口 | satisfied | 主文 169／PDF 177；TC1 修标题，AMD2.80 列全 atof／atoi／atol／atoll。全部物理引用及实际展开无四名字的调用、取址或宏；故障测试分支使用 strtoul，按 R22.8／22.9 先清零、立即检查，并验证首字符、结束指针、UINT_MAX 与非零后交 fail_resource。固定直接调用匹配的不足由完整名字检查补齐。 |
| R21.9 | Required | bsearch／qsort 标识符 | satisfied | 主文 170／PDF 178；TC1 修标题，AMD2.27。物理替换表、取址及实际展开无 bsearch／qsort；OS 就绪顺序由项目 ready_sequences 和内核调度服务消费，未通过标准比较器调用。固定入口仅匹配名字后左括号，当前证据也排除函数指针入口。 |
| R21.10 | Required | time 头及日期时间设施 | satisfied | 主文 170／PDF 178；AMD2.82 与 AMD3.69–70 合成包含禁令、全部 time.h 设施和 wcsftime。范围内无 time.h include、clock_t／time_t／tm、标准日期函数、CLOCKS_PER_SEC 或 wcsftime 引用；GCC 展开另核。主机 clock_gettime／timespec 实现位于 host／port，目标接口使用 Os_TickType 等自身类型；SDK 头内部沿采用责任。 |
| R21.11 | Advisory | tgmath.h 包含 | satisfied | 主文 171／PDF 179 被 AMD3.40–43 改为 Advisory，并将 amplification 收窄为 tgmath.h 不应被包含。全部物理 include 和真实文件回执无该头；没有以 math.h／complex.h 同名函数推出违反。当前 typedef 浮点存储无类型泛型数学；固定 findInclude 与独立引号／传递检查一致。 |
| R21.12 | Required | fenv.h 及全部环境设施 | satisfied | 主文 171–172／PDF 179–180；AMD3.44–48 改为 Required，扩大到全部 fenv.h。范围内无该头、fenv_t／fexcept_t、FE_* 设施或 fegetenv／fesetround 等名字；实际展开及 pragma 另核。浮点 typedef／存储未操作浮点环境；固定工具仅检查原五函数及 fexcept_t 的局限保持。 |
| R21.13 | Mandatory | ctype 输入值域 | satisfied | AMD1 印刷 4／PDF 12；AMD2.27。物理标识符、替换表及实际展开无 ctype.h 的 14 个分类／转换接口引用，含 isblank／tolower／toupper 与取址；ASCII 解析实现处于 integration 范围外 .c。项目没有待证明的 EOF／unsigned char 实参；核心 invalidFunctionArg 的有限模型未单独作为结算依据。 |
| R21.14 | Required | memcmp 与空终止字符串 | satisfied | AMD1 印刷 4–5／PDF 12–13；明确 char 数组含 NUL 与 unsigned 字节数组的区别。物理及实际展开均无 memcmp 引用、取址或替换表，Com 和 Mailbox 仅复制，比较用标量。固定 addon 以 strcpy 等推断字符串的别名缺口由无入口证据补齐。 |
| R21.15 | Required | memcpy／memmove／memcmp 两端类型 | satisfied | AMD1 印刷 6／PDF 14。仅三个 memcpy 位置，无 memmove／memcmp。Com_SendSignal 两参数分别为 void *／const void *，Com_ReceiveSignal 为 void *／const void *，均为 qualified 或 unqualified void 的兼容版本；四 profile 的 Rte.c 由 rte.rs 产生 UINT32 data／value／rte_value_0，接到 signal_values 的 uint32。Mailbox 为 uint8 *／const uint8 *，指向类型一致；SDK typedef 展开确认 unsigned char。没有借固定 addon 遇到 void 就跳过的分支证明类型兼容。 |
| R21.16 | Required | memcmp 的对象类别 | satisfied | AMD1 印刷 6–7／PDF 14–15，允许指针、essential signed／unsigned／Boolean／enum。物理、宏及实际展开无 memcmp，结构快照通过 memset 初始化和成员赋值传出；没有用表示字节判断结构／浮点相等。固定入口跳过未知或多层指针的限制仍记录。 |
| R21.17 | Mandatory | 12 种无长度字符串接口的边界 | satisfied | AMD1 印刷 8／PDF 16 的 12 种接口完整列表为 strcat／strchr／strcmp／strcoll／strcpy／strcspn／strlen／strpbrk／strrchr／strspn／strstr／strtok。物理及实际展开均无这些名字的引用、取址或替换表。未选故障分支用 strtoul 的结束指针判断，非本条列表；核心 bufferAccessOutOfBounds 之外已独立核对入口缺席。 |
| R21.18 | Mandatory | string.h 的 size_t 参数 | satisfied | AMD1 印刷 9／PDF 17 要求正值且不超过最小对象。当前仅三个 memcpy 和三个 memset：Com 的 sizeof(signal_values[index]) 为正的四字节，Rte producer 及实际调用均给 uint32 存储；Mailbox 拒绝 length==0 或超过 OS_INPUT_PAYLOAD，record->data 容量 32，Ecu_Target 三处传完整 Ecu_Input（另有 Ecu_MailboxInput 容量编译断言） 的 sizeof，复制前缓冲有效。其余 memset 分别用 sizeof(record->data)、sizeof(snapshot)、sizeof(*batch)，对应完整对象；HostBatch 空指针先拒绝。无其他长度接口或间接入口，动态／零长缺口未由核心模型代替。 |
| R21.19 | Mandatory | 内部存储的 const 使用 | satisfied | AMD1 印刷 10–11／PDF 18–19；TC2.36 删除旧 R21.8 引用。物理唯一入口为未选 read_resource_failure 的 getenv，返回值绑定 const char *failure，只交 strtoul 的 const 输入及字符读取，未写入或传出；无 localeconv／setlocale／strerror。当前实际展开无此入口，内部 lconv 成员指针的写路径也无候选；未借 addon 局部赋值模式代替别名核查。 |
| R21.20 | Mandatory | 内部存储的失效顺序 | satisfied | AMD1 印刷 11–12／PDF 19–20；TC2.37 合并 asctime／ctime、gmtime／localtime，localeconv 后 setlocale 同组。物理唯一 getenv 在 read_resource_failure 内消费完后仅保存 uint32 fail_resource，不保存指针，也无第二次相关调用；真实展开未选。其余七函数无引用，跨分支和别名没有未结算候选；固定 addon 遇花括号清空局部表的缺口由该完整闭包补齐。 |
| R21.21 | Required | system 命令接口 | satisfied | AMD2.83 印刷 18／PDF 25，禁止 system 名字及同名宏展开。物理、替换表、取址及实际展开均无该标识符；C 工程运行入口通过宿主构建工具启动，未在目标程序调用 system。固定入口只查单参数直接调用，本次无入口结论覆盖间接引用。 |
| R22.1 | Required | 标准库动态资源获取与释放 | satisfied | 主文 172–173／PDF 180–181；AMD2.84 明确 malloc／calloc／realloc／aligned_alloc／fopen。物理及实际展开无五个入口，目标对象为静态存储或调用者借用，Mailbox／Com 都复制后返回。主机 stack 和文件实现沿 ADR 0030，内核分配器原行沿采用责任；无项目标准资源释放闭包待补，核心 leak 模型非唯一依据。 |
| R22.2 | Mandatory | free／realloc 的来源和重复释放 | satisfied | 主文 174／PDF 182；AMD2.27 增 C11 范围。物理及实际展开无 free／realloc 引用、取址或宏，项目补丁没有标准释放入口；范围内没有标准释放实参或别名。内核 vPortFree 原行及平台资源释放沿各自来源；核心 autovarInvalidDeallocation 未替代该扫描。 |
| R22.3 | Required | 同文件跨流读写 | satisfied | 主文 175／PDF 183 的 amplification 同时考虑环境类似设施。目标自有／补丁无 fopen／freopen、FILE、标准流或平台 open／CreateFile 文件获取；物理及真实展开均核对。HostBatch 的标准输入输出与 Linux /proc 只读访问在范围外实现，目标接缝无文件句柄；无同文件多流身份候选，固定 incompatibleFileOpen 模型之外已查环境入口。 |
| R22.4 | Mandatory | 只读流的写入 | satisfied | 主文 176／PDF 184。范围内无 FILE 或标准／平台文件流获取和写入；Ecu_HostBatch.h 为内存 staging，memset 不操作流。真实展开与原物理引用一致；输入 mailbox 的发布和 CAN 缓冲写入不引入文件流。writeReadOnlyFile 的模式模型没有承担缺失的句柄追踪。 |
| R22.5 | Mandatory | FILE 指针直接／间接解引用 | satisfied | 主文 176–177／PDF 184–185；AMD2.85 更新理由，禁止直接或 memcpy／memcmp 间接读取 FILE。当前物理和实际展开无 FILE 自有声明、typedef 或引用；三个 memcpy 都是信号／字节缓冲，三个 memset 是完整项目对象，已逐对象核查。SDK 内部 FILE 布局沿采用责任；固定入口的简单类型起始匹配非唯一依据。 |
| R22.6 | Mandatory | 关闭后流指针的使用 | satisfied | 主文 177／PDF 185。物理、宏和实际展开无 fclose／freopen／FILE 或文件流别名；目标接口未传递关闭后的流指针。无跨调用保存及生命周期终点待核对；核心 useClosedFile 的局限由入口和对象缺席证据补齐。 |
| R22.7 | Required | EOF 比较与返回值转换 | satisfied | AMD1 印刷 12–13／PDF 20–21，包含间接转换；feof／ferror 是独立允许设计。当前物理替换表、表达式及实际展开无 EOF 比较，也无项目 stdio 返回链；字符协议解析在 integration .c 范围外。固定 bool／char／short 兄弟类型检查未替代该完整入口核查。 |
| R22.8 | Required | errno-setting 调用前状态 | satisfied | AMD1 印刷 13–14／PDF 21–22 含环境扩展及已知为零例外。范围内物理唯一 errno-setting 调用为 read_resource_failure 的 strtoul，所有到达路径紧邻 errno=0，中间无调用；getenv 在清零之前。四 profile 真实展开未选该故障分支，其余平台调用的 errno 操作在 host／port，直接错误码接口另核；固定顺序状态模型未代替路径证据。 |
| R22.9 | Required | errno-setting 调用后检查 | satisfied | AMD1 印刷 14–15／PDF 22–23；TC2.38 修正为测试 errno。read_resource_failure 的 strtoul 后紧接 if(errno!=0)，在 *failure、*end、value 的结果消费及其他调用前检查；错误路径交 Os_BackendShutdown，成功才写 fail_resource。当前实际展开无其他范围内 errno-setting 调用，返回值足以判断的原文例外未用于跳过本次检查；固定入口分隔符启发式非唯一依据。 |
| R22.10 | Required | errno 测试的最后调用 | satisfied | AMD1 印刷 15–16／PDF 23–24。物理唯一 errno 比较的前一业务调用为 strtoul，期间无其他函数；errno 的 libc 宏访问保持 SDK 身份，未当成业务调用重排。故障分支最终只保存整数，正常四 profile 展开没有范围内 errno 测试；固定入口右花括号清历史的限制由直接闭包核查补齐。 |
<!-- misra-checker-obligations-end -->
