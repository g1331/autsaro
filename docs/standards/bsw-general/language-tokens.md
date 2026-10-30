# 语法与原始诊断

## 原始 token 分类的具体误报

当前 master `3e93e436` 的 Linux standard-ecu 工件保留 20 条 R7.3：
Os_Backend 的 `OS_RESOURCE_CREATE(call)` 四处，Os_HostServices 的 `cell` 六处，
Os_HostLinux 的 `cell` 四处及 `all` 两处，Os_LinuxNative 的 `cell` 四处。
逐列回到原源码均为标识符；固定 2.21.0 的 `misra_7_3` 使用十六进制字符集合加小写 l
后缀的正则，未先确认 token 是数字，故这些名字恰好匹配。
R7.3（Required，主文印刷61／PDF69）约束数字字面量后缀，不约束这些标识符。
不能为了消除误报更名真实参数、改 addon 或删除原始记录。

同一工件的 12 条 R12.3 位于 Os_Stack.h 的七个成员声明分隔符、Os.c 的 i/j
声明、Os_Backend 的四个计数器声明和 registered/gate 成员声明。
它们在 C99 语法中是声明的逗号分隔符，不是 §6.5.17 的 comma operator；
R12.3（Advisory，主文印刷106／PDF114）禁止 comma operator；声明分隔符不属于该运算符。
类型头拆分只改变七个成员分隔符的位置，成员顺序与类型不变；其余声明不改写。

## DET 参数原型的具体误报

冻结 `ca6627b4` 的 standard-ecu 工件中，`bsw/src/Det.c` 的
`Det_ValidHooks(const Det_ErrorHookType *hooks, uint16 count)` 位于第 16 行，
两个形参均具备类型和名字；`Det.h` 中的 `Det_ErrorHookType` 也以四个具名参数定义。
R8.2（Required，主文印刷64–65／PDF72–73）要求函数原型及具名参数；
TC2.10 只追加 R8.3 交叉引用。这里既非 K&R 定义，也没有无名或空参数列表，
因此该位置的 R8.2 原始报告为误报。此判断绑定这两个实际声明，
不扩大到 SDK 回调或其他函数；保留原始报告，不修改分析器。

## 回调的 const 建议

通用 constParameterCallback 建议须按调用 ABI 复核。watch、control_actor、native_task_start、task_entry、actor_start
以及两个 Linux signal handler 和 guard_signal 的参数类型分别由原生线程或 sigaction
回调原型约束；不能把参数类型改为 const void * 等不兼容函数类型。
Os_LinuxAltStack 的 previous 和 native_call 输入由汇编约束实际读写，固定分析器未建模，
其 constParameterPointer／R2.7 建议不作为删除或收窄这些参数的依据。
这些是限定位置的工具判断，不覆盖别的 const 建议或 SDK 实现。

## 词法与接口表达的人工评估

以下使用[共同配置、原件及冻结证据](../os/native-facilities/storage-interrupts.md#人工评估的配置与证据范围)。
Advisory仍尽合理可行遵循；主文§6.2.3（印刷13／PDF21）允许用替代安排记录不符合，而非必须走正式偏离。
本轮的有界工程选择只保留下列明确机制及真实违反，不改变类别或R15.5以外的GRP，不借 #381 的124键扩大授权。

<!-- misra-manual-begin -->
| 指南 | 有效类别 | 分析范围／语言 | 状态 | 义务、限定结论与证据 |
| --- | --- | --- | --- | --- |
| D4.4 | Advisory | 指令／C99 | satisfied | 不以注释停用实现（主文27；AMD2扩展语言）。[实际词法边界](language.md#注释与字面量的实际词法边界)中的Doxygen示例是说明而非停用实现；commented_defines输出生效define，未选择功能以条件编译表达。所选307种交付文件字节与正常producer没有另发现注释停用实现。 |
| D4.5 | Advisory | 指令／C99 | retained | 同namespace且重叠可见的名字应避免字形歧义（主文27–28）。[既有重名整改](../os/declarations.md#条件接口与声明owner)保留标准COM_INIT/Com_Init、COMM_INIT/ComM_Init及本机HANDLE/handle、CONTEXT/context、SIZE_T/size_t；标准发布名/原生类型与项目局部角色有类型和模块前缀区分。仅这些实际名字选择保留，理由是避免改变标准/原生接缝和既有调用合同；风险为读错名字，控制为类型检查、声明契约和真实双平台消费者。新增同形私有名字须整改，不将不同成员namespace或已结束原型作用域并入。 |
| D4.6 | Advisory | 指令／C99 | retained | 数值类型使用宽度/符号typedef（主文28–29；TC1排除plain char，AMD2.23–25）。汽车值沿Platform_Types/stdint；仅[原生表示](../os/configuration.md#原生类型表示与封装)的BOOL/int、pthread错误int、syscall long及altstack long-double存储保留ABI类型及自然对齐。主文允许为控制外接口作取舍，typedef/main例外不扩至任意局部数值；平台/ABI变化重评，已知宽度本身不消除违反。 |
| D4.8 | Advisory | 指令／C99 | not_assessed | 仅某类型在一个TU中的全部指针都不解引用时，应隐藏完整布局（主文30–31，TC1澄清）。[HANDLE不完整类型](../os/configuration.md#原生类型表示与封装)成立；公开配置/CONTEXT有完整存储及字段消费者，但这不能代替每一类型/TU组合的全部指针闭包。尚缺该集合与必要公开表示的逐对象依据，不能把一个未解引用参数或公开ABI概括成全项满足/类别保留。 |
| D4.9 | Advisory | 指令／C99 | satisfied | 仅普通函数在语法/约束上可互换时优先函数（主文31–32；AMD2.26，AMD3.6增加_Generic例外）。[声明宏](../os/implementation.md#声明宏与表达式参数)与[入口/跟踪宏](../os/platform.md#入口及跟踪宏的实际展开)承担声明符、拼接/字符串化、调用点文件行及当前RSP身份，不能由普通调用替代；其余服务包装/计时转换沿实际static inline。没有使用_Generic例外来覆盖普通表达式宏。 |
| R1.1 | Required | TU／C99 | satisfied | 不违反C语法/约束/实现翻译上限（主文37–38；AMD2.27）。正常入口逐TU的-Wall/-Wextra/-Werror编译与真实链接/运行覆盖所列封存源码；[受控扩展](../os/platform.md#c99-平台扩展的使用边界)由所选GCC接受，原文明确允许实现支持的扩展。扩展的R1.2建议和ABI/运行语义另核，不以生成成功代替编译。 |
| R1.5 | Required | System／C99 | satisfied | 不使用所选C版次的过时特性（AMD3.14、60及Appendix F）。[C99构造检查](../types.md#c99过时形式与名称有效长度)无K&R/空参数/gets等；[Windows实际ld](../os/implementation.md#windows-外部名字与所选工具)的2.47.20260726字节与已核发行包一致，COFF/hash链按完整名字处理。Linux已核ELF/hash链同样不截短至255以下；不靠当前名字短、编译成功或版本推出实现上限。 |
| R3.2 | Required | TU／C99 | satisfied | 行注释内不可反斜杠续行（主文46；AMD2.31删除说明Note）。所列双平台两组八份C/H原始物理行及307种文件字节的续行候选为空；[Doxygen转换及宏续行](language.md#注释与字面量的实际词法边界)逐构造区分，宏指令/完整块注释中的续行不属行注释。 |
| R12.5 | Mandatory | TU／C99 | satisfied | 不对数组形式形参本身取sizeof（AMD1§2.1，3；AMD2.27）。[逐函数数组形参](language.md#sizeof的对象与数组形参)连接Can data、Host data、ExecutionTransmit、Target frame、read_line、内核名称检索和PipelineTest_Copy；实际sizeof取得局部数组、元素或具名对象，容量另验证。不是一般禁止sizeof表达式，也未使用Mandatory偏离。 |
| R21.22 | Mandatory | TU／C99 | not_applicable | tgmath操作数须为允许的essential算术类别，部分宏不允许complex（AMD3.49，24–25；不把输出指针当操作数）。[所选库构造](README.md#源码语言边界)与两组双平台交付C/H无tgmath.h或其宏调用/展开；这是触发构造缺席，不是C99语言豁免或Mandatory偏离。 |
| R21.23 | Required | TU／C99 | not_applicable | 多操作数tgmath宏在整数提升后须有相同standard type，frexp/remquo输出参数除外（AMD3.49，25–26）。同一有限源码/实际入口没有这些宏，沿[库构造证据](README.md#源码语言边界)；不从R21.22缺诊断推定满足。 |
| R21.24 | Required | TU／C99 | not_applicable | 禁止rand/srand调用及相应宏展开（AMD3.50，26）。所选自有/补丁C/H及producer没有rand/srand用途；[固定上游与项目补丁责任](../os/adoption.md)不把原始SDK实现内名字归项目调用。引入随机入口时重评。 |
<!-- misra-manual-end -->
