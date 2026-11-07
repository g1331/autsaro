# 标准类型、平台类型与通信栈类型

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |

公开类型的 R2.3 处置提案见[标准类型声明](types/public-types.md)，其独立复核状态与源码注释阶段分开记录。

## C99 essential type 与实际常量

Boolean／character诊断、原生位掩码及复合表达式的具体判断见[essential type与表达式](types/essential-types.md)。

## 源码诊断的语法边界

R7.3／R12.3 的历史报告裁决与选定源码的人工证据见[下文](#r73-的标识符误报)。局部误报、源码修正和规则评估分别成立，不能用报告消失替代后者。

`Ecu_ExecutionTransmit` 的 sink 为 `Can_TxSinkType` 函数指针，`(void)sink` 只显式丢弃此参数；实际发送沿 Can 的 flush → Ecu_ExecutionTransmit → Ecu_TargetEnqueueTransmit 进入受控队列。R11.1（Required，主文页93–94／PDF101–102）的例外2明确允许函数指针转 void，此处没有转成 void 指针、整数或其他函数指针类型；AMD2.58 只修订例外3的说明，不撤销例外2。固定 addon 对丢弃函数指针与改成不兼容函数指针的对照均报告；只对前者按既有原文例外裁决，不扩大为通用指针转换许可。

当前standard-ecu的Det.c:16一条R8.2报告也须按真实声明判断：Det_ValidHooks的hooks／count均具名，Det.h中的Det_ErrorHookType四个参数为ModuleId、InstanceId、ApiId、ErrorId，没有省略的参数类型或名字。主文R8.2（Required，印刷64–66／PDF72–74）及适用修订不要求把已具名的函数指针typedef重新内联进形参。本处按源码判为误报，不与生成runnable的真实nameless role冲突、SDK的无名参数或整个R8.2混为同一结论；原始报告保留。

## 数组初始化与重复指定

R9.3／R9.4的具体初始化闭包见[初始化层级](bsw-general/language.md#初始化层级与实际求值)。补充原文边界：主文印刷78–80／PDF86–88中，R9.3明确允许`{0}`、仅由指定初始化器组成的稀疏数组和字符串初始化；R9.4按AMD4.47–50（印刷33／PDF40）还须检查指定位置之后的位置初始化是否重新覆盖先前子对象，不能只比较成员名文本。两条均为Required、当前C99适用，AMD2.27追加C11。

`f1986d7d`四份os-aliased-service工程复核保留同一结论：priorities容量为32、唯一指定索引为30，符合稀疏例外；tasks／counters／alarms及路由等数组由列表推导长度。相应自有源码没有新增重复指定或回退后的位置覆盖；采用tasks.c仍按实际单核及关闭运行时统计的配置判断。不外推SDK初始化宏、多核或任意用户应用，原始人工状态保持。

## C11 原子对象的适用边界

AMD4 新增的 R9.7（Mandatory，AMD4.6，印刷页10／PDF17）、R11.10（Required，AMD4.15，页12／PDF19）、R12.6（Required，AMD4.16，页13／PDF20）及 R21.25（Required，AMD4.32，页18–19／PDF25–26）均适用于 C11。已核对的双平台四样本及当前四个 Linux 样本，交付 C／头文件与采用内核均没有 `_Atomic`、标准 `atomic_*`／`memory_order*` 标识符或 `stdatomic.h` 入口；逐文件词法收据与冻结源码身份留在 PR #371，不外推系统头的预处理闭包。

R9.7 没有需要按该条初始化的 C11 原子对象；R11.10 没有施加 `_Atomic` 的 void 不完整类型；R12.6 没有被直接访问成员的原子结构体／联合体；R21.25 没有使用其规定内存序的 C11 标准库同步操作，故在上述源码范围分别不适用。这不是 Mandatory 豁免，也不把 GNU 原子内建视为 C11 标准库函数。Linux 实际使用的 `__atomic_*` 操作、发布顺序与 lock-free 前提仍按[平台扩展及并发边界](os/platform.md#c99-平台扩展的使用边界)核查；这些操作存在的事实没有从收据中排除。

## 泛型选择的适用边界

AMD3.51 的 R23.1–R23.8 均只适用于 C11 泛型选择（AMD3 印刷页27–37／PDF35–45）。已核对的双平台四样本及当前四个 C99 生成样本，其交付 C／头文件和采用内核没有 `_Generic`，也没有 `tgmath.h` 泛型宏入口；这是实际构造的缺席依据，不仅是编译参数的语言标签。冻结源码身份及逐文件核对收据留在 PR #371。

R23.1（Advisory）的泛型宏展开位置与 R23.2（Required）的非宏控制表达式副作用条件均不存在，故在上述源码范围不适用。R23.3（Advisory）没有可检查的非 default association，R23.4（Required）没有 association 类型列表，分别不适用。R23.5（Advisory）没有基于指针类型的泛型选择，R23.6（Required）没有需要比较 essential／standard type 的控制表达式，分别不适用；未借用 R23.6 的整数常量例外。R23.7（Advisory）没有泛型宏实参的求值次数，R23.8（Required）没有 default association 的位置，分别不适用；未借用 R23.7 的常量结果例外。

这八项的适用性结论只针对上述源码；新增泛型选择或引入会展开它的库宏时重新核查。C99 中已有的 essential type、宏参数求值、指针转换和持久副作用仍按各自适用规则评估，原始 checker 的 `not_assessed` 目录状态不由这段人工结论改写。

## 整数常量、初始化与浮点构造

冻结原文与实例见[语言构造](types/language-constructs.md#整数常量初始化与浮点构造)。

## R7.3 的标识符误报

### R7.3／R12.3：四 profile 的有界人工证据

历史误报、双目标语法闭包与供应方遗留见[语言构造](types/language-constructs.md#r73-的标识符误报)。

## 函数指针的显式丢弃

原文例外与实际消费者见[函数指针丢弃](types/language-constructs.md#函数指针的显式丢弃)。

## 未使用声明与真实消费者

公共声明、条件定义及真实使用的判断见[声明用途](types/declarations.md#未使用声明与真实消费者)。

## C99过时形式与名称有效长度

R1.5为Required，本仓库C99适用（AMD3.14，印刷7／PDF15；AMD3.60重置的Appendix F，印刷41／PDF49）。C99对应#1–5、#7–9；表中的ATOMIC_VAR_INIT和零长度realloc属于C18，不能混入此判断。所核四个Linux工程的实际声明、生成入口和采用补丁闭包没有旧式空参数／K&R定义、标准storage-class位置倒置、bool/true/false重定义或gets／受限ungetc调用；POSIX原件中的prvFatalError已由所选补丁整段删除，不拿未补丁输入判交付违反。此源码结论不涵盖任意外部应用。

Appendix F #2／C99 §6.11.3另要求核对外部名字的实现有效长度，不能由名字较短或GCC版本替代。[GCC13.3实现说明](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc/Identifiers-implementation.html)将外部名字长度交给linker。当前解析的GNU ld为binutils2.42-4ubuntu2.10，字节与官方deb一致；对应发行源码的bfd_link_hash_lookup进入bfd_hash_lookup，按完整字符串计算长度、复制len+1并用strcmp比较，没有小于255字符的前缀限制。已核发行补丁未改变这两个函数；ELF名字使用字符串表，当前构建没有名字截断选项或编译器搜索路径覆盖。该证据支持这一Linux工具组合，不是所有linker的属性。Windows2.47-4的实现材料及实际linker选择边界见[OS](os/implementation.md#windows-外部名字与所选工具)；没有运行名字边界实验。完整身份与原文证据集中PR #371。

callback-parameter旧7处R2.4均为匿名类型误报；新增xSTATIC_TIMER为真实未用tag，现由0015省去，保留StaticTimer_t及布局（Breaking说明同步）。原报告保留，typedef用途另按R2.3判断。

## 链接可见性与实际引用

外部接口、私有helper及检查器的TU去重限制见[链接可见性](types/declarations.md#链接可见性与实际引用)。

## 单函数对象与存储期

块作用域、静态／TLS存储期与独立MemMap段的关系见[单函数对象](types/declarations.md#单函数对象与存储期)。

## 原生ABI类型与编译约束

原始类型、编译拒绝及反例见[ABI 类型证据](types/language-constructs.md#原生abi类型与编译约束)。

## 固定 11 项 checker 限制的补充评估

本组评估 C99 的 standard-ecu、user-application、component-pipeline、no-diagnostic，分别连接 Linux／Windows 受控 x64 的 HostBatch 与 probe；包括两个程序的全部 TU、补充单元、实际宏展开及补丁后内核。八份输入的 seal、producer、工具／SDK 与逐项核查收据归本次 PR。上下文独立绑定本组，整体状态为 pending：R5.5／R5.7／R10.1 的 SDK 接缝仍由 #444 承接，R20.1 保留项的正式义务仍开放。

责任沿 ADR 0025 分开：自有 runtime、生成／应用输出及项目 patch 逐构造判断；固定 FreeRTOS 原件引用其采用记录，实际集成和项目修改另查；SDK 实现沿供应方责任，其进入自有表达式的宏及接口用途仍在本组范围。内部实现缺口与调用方义务分别记录，#441／#444 接续相应责任。

原文为 March 2013 Third Edition 与 AMD1–4／TC1–2，七份原件逐一通过原 SHA256SUMS。下表定位用“印刷页／PDF 页”，全部 Required，R20.1 为 Advisory。AMD2.27 的适用性表追加 C11，本组仍按 C99；其余实质修订逐项列出。旧证据引用其冻结范围，当前差异及可证伪控制集中在 PR。

<!-- misra-checker-assessment-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R5.1 | Required | 双目标四样本；外部对象及函数，按每个程序分别判断 | satisfied | 主文 48–49／56–57，AMD2.33–34：按实际实现的有效字符区分。包含 StartScheduleTable 的长函数名；Windows 固定 ld 的完整 COFF／hash 名字链与当前准备日志相接，Linux 采用完整 ELF／hash 名字链；无新增截短、搜索路径或名字变换。既有实现证据见[名称有效长度](#c99过时形式与名称有效长度)。 |
| R5.5 | Required | 自有、项目 patch 与实际 SDK 宏／标识符接缝 | not_assessed | 主文 53／61，AMD2.39 与 §6.7：追溯实际 define 历史，含其后 undef，排除从未定义的 MPU／多核分支。自有及 patch 的配置结论复用[OS 声明](os/declarations.md#sdk宏名与实际标识符)。Linux 的 sa_sigaction／si_addr 头外展开与非标准头引入的同名对象仍有真实义务；#444 核定当前精确实例及处置后恢复，本组未批准新例外。 |
| R5.7 | Required | 跨 TU 的真实 tag、其他 namespace 与关联 typedef | not_assessed | 主文 55／63，AMD2.40：同头重复定义与关联 typedef 例外分别判断。自有 Os_NativeStackType、Os_HostHandleType 及各 Rte_CDS tag 沿真实声明 owner 核对；项目 patch 不再保留无用途 xSTATIC_TIMER tag。SDK 的 sigaction 等 tag／函数同名候选须按 §6.7 和实际头外使用由 #444 完成裁决，保留该具体义务。 |
| R8.8 | Required | 自有及所选 patch 的内部链接函数全部声明／定义 | satisfied | 主文 70–71／78–79；AMD3.64 追加交叉引用。按 TU 先确定内部链接实体，再逐声明检查 static，包括头内 inline 与后续定义；现有声明的 static 属性保持。Linux 当前完整预处理函数声明复核与[原函数闭包](os/declarations.md#条件接口与声明owner)相接；Windows 平台定义从对应源码及所选 patch 核对。 |
| R10.1 | Required | 自有与 patch 的 signed 位运算、~、逻辑操作数及实际宏用途 | not_assessed | 主文 82–84／90–92；TC1 补逻辑指针限制，TC2.17–18，AMD3.20–26，AMD4.51。自有原生 mask 先进入 unsigned 域，~ 操作数为 uint32，逻辑参数为 Boolean 或显式比较，见[原生位掩码](types/essential-types/expressions.md#原生位掩码)。Windows 的 CONTEXT_CONTROL 实际展开为 0x100000 与 0x1l 的按位 OR，两个 signed 操作数违反本条；每 profile 的 Backend 三处及项目 port patch 一处转 #444，尚无合法处置。 |
| R10.3 | Required | 自有与 patch 的参数、返回及 case-label 隐式转换 | satisfied | 主文 86–87／94–95；TC1 常量 rank 与 case 例外，TC2.20 要求按控制表达式 essential 类型；AMD3.27–30 的 real→complex 例外无实际用途。返回／实参按目标类型追溯，Boolean 常量按 D.6；本次补正 Os_TargetTrace 的六个生产字符标记实参，显式转为接口的 uint8，字符值、调用次数及接口保持。实际 case 常量按各控制类型和值域核对。Windows CONTEXT_CONTROL 的常量结果1048577按 D.7 取 STLR 为 signed int，可由 DWORD 表示，适用 TC1 常量例外；内部 OR 的 R10.1 义务另列。 |
| R10.6 | Required | 自有与 patch 的参数／返回等赋值语义中的复合表达式 | satisfied | 主文 90–91／98–99、§8.10.3 与 TC1：~ 为复合运算符，常量表达式除外；cast 结果和单值扩展分别处理。HostBatch number、monotonic_ms、COM 打包与 Target 长度均在计算前选宽；Dcm 长度按各 profile 的 PduLengthType 和 Dcm_MsgLenType 区分。实际表达式接[既有宽度闭包](types/essential-types/expressions.md#复合表达式与赋值结果)，CONTEXT_CONTROL 为常量表达式，依 §8.10.3 按常量边界处理；运算宽度按实际操作数判断。 |
| R13.5 | Required | 自有及 patch 逻辑 RHS 的调用／volatile 访问闭包 | satisfied | 主文 113–114／121–122 与 Glossary 224／232；TC1 Appendix J 14／20：按函数可能的持久副作用判断。当前 RHS 的 CanTp、Dcm、Det、OS 配置／TLS 查询闭包接[BSW General](bsw-general/README.md#逻辑右侧的持续副作用)，strncmp 按标准库只读契约；有锁、输出、原子状态变化及事件消费的调用保持独立求值。真实 errno 宏另沿普通 TLS 对象读取判断；SDK 内部实现由 #444 承接。 |
| R14.4 | Required | 自有及 patch 的 if、while、do、for 控制表达式 | satisfied | 主文 121–122／129–130：for 存在条件时须为 essentially Boolean；省略条件合法。当前控制为 Boolean、比较／逻辑结果，单次宏循环的 FALSE／false 按 D.6 识别；CanTp 的 const boolean elapsed 保持 Boolean。实际内核宏展开接[断言／循环整改](os/kernel.md)，未把数值 0／1 当作通用 Boolean 常量。 |
| R20.1 | Advisory | 原文件预处理前的全部 include，含排除分支与 typedef 前缀 | retained | 主文 155／163、AMD2.27：从物理文件顺序判断。实体后的 MemMap STOP、类型后的 START 及条件 C++ 包装分别接[声明布局](types/declarations.md#类型前缀与首次映射请求)和[MemMap 类别](memmap/README.md#memmap-类别偏离)。保留实际违反与正式待完成状态，已有必要性与布局检查各自保留；#439 的逐位置消费另接续。 |
| R21.8 | Required | 自有源码、生成／应用输出、所选 patch 与终止入口宏用途 | satisfied | AMD1 移除 getenv；AMD2.81 整项替换为 abort／exit／_Exit／quick_exit（16／23），包括这些标识符和同名宏展开。当前交付源码没有这些项目用途；ExitProcess 沿 Os_HostExitProcess 进入 Win32 ExitProcess 或 POSIX _exit。assert 的标准库内部实现与项目直接调用分别处理，见[原生设施](os/native-facilities/README.md)；检测器／测试源码不冒充本组生产输入。 |
<!-- misra-checker-assessment-end -->

当前完整 Linux TU 可由 Clang 18.1.3 解析；Windows 的 GCC 16 intrinsic 实现与 Clang 18 有不兼容错误，这部分只保留候选，Windows 源码／项目 patch 及真实 GCC CI 各自核对。错误输出未用作缺席证明。SDK tag／宏的当前精确处置交 #444；其恢复条件是逐实例原文裁决、实际接口闭包与所需正式义务齐备，再重新评估并绑定。

补充记录只描述上述 checker 缺口；原 66／155 分区、221 项 assessment 和原始分析结果由正常入口保留。正文、源码、producer、配置、程序／TU、目标、工具或 provider 变化时重评本组，pending 持续阻止验收。
