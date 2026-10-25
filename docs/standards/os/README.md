# OS

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。按 [ADR 0017](../../adr/0017-requirement-disposition-in-tests-and-standards.md) 处置 R24-11 `SWS_OS` 的全部条目；核算包含已选实现、经配置拒绝证明的不适用项及明确的剩余差距，不表示完整 AUTOSAR／OSEK 符合或认证。

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3（`os223.pdf`）。编号全集取自固定 SpecificationsARXML，`ECUC_Os_` 与 `ECUC_Arti_` 条目也在其中。

## 已选配置

单核 SC1，固定一个后端（FreeRTOS 内核加本项目的 OS 适配层）。ECU 只有一个所属任务、一个软件计数器（1 ms／tick）、一个名为 OSDEFAULTAPPMODE 的应用模式；周期触发来自 RELATIVE 自动启动的闹钟或重复、不同步的调度表，动作只有对所属任务的 SetEvent；资源只有可选的虚拟 RES_SCHEDULER。Error／Pre／Post／Startup／Shutdown hook、栈检测与 ARTI 始终启用。

OS-Application、IOC、多核、spinlock、时间／内存保护、ProtectionHook、外设区域与调度表同步不属于 SC1，在生成前于源对象位置拒绝，相应条款按配置不适用。运行时还实现了更一般的 SC1 能力（多任务与多计数器、ActivateTask／callback／IncrementCounter 闹钟动作、绝对起点、单次与多点调度表、配置资源与 LINKED 资源、ISR），并有原生测试；生成器只接受上述参考形态，其余取值同样在源位置拒绝，依赖它们的条款按配置不适用，每个参数都有 `Rejects:` 测试。放开生成器由 [#198](https://github.com/g1331/autsaro/issues/198) 跟踪。生成的 OS 只包含调度实际使用的对象，未使用的 OsAlarm、OsEvent、OsAppMode 或调度表同样拒绝，不会静默丢弃。

## 处置与证据

- [SC1 内核](kernel.md)：原生套件之外经审阅确认的内核条目、规范之间的取舍与依赖生成器所拒配置的条目。
- [ECUC 配置与生成](configuration.md)：全部 `ECUC_Os_` 条目、生成器接受的形态与拒绝方式、OS 生成章节。
- [SC1 以外的能力](extensions.md)：多核、OS-Application、保护、IOC、外设、调度表同步等由配置拒绝证明不适用的条目。
- [ARTI](arti.md)：ARTI 描述文件、hook 宏与 `ECUC_Arti_` 条目。
- [受控主机平台与 MISRA 边界](platform.md)：平台扩展、原生错误处理、三处有界保留及 C11 设施的适用性。
- [原生设施与资源生命周期](native-facilities.md)：上下文／布局、实际provider、展开初始化及线程所有权。

## 核算

`requirements SWS_OS` 只剩中断向量表段（`SWS_Os_00336`）与 OS 声明的 MemMap 包装（`SWS_Os_00815`）未处置，它们随 SchM／MemMap 责任链在 [#191](https://github.com/g1331/autsaro/issues/191) 处置；SchM 与 BSW 通用义务（`SWS_BSWGeneral --module Os`）同样由 #191 处置。ShutdownHook 返回后的死循环（`SWS_Os_00425`）由目标 OS 端口承担，记为差距 [#200](https://github.com/g1331/autsaro/issues/200)。SC1 的条款按能力本身处置，覆盖等级要求的最小容量（`SWS_Os_00241`），不缩减为参考配置实际使用的对象。

## 已知差距

- 引用的 OSEK 版本 ISO 17356-3 全文未取得，严格 ISO 条款由 [#57](https://github.com/g1331/autsaro/issues/57) 跟踪；固定 FreeRTOS 内核（[ADR 0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)）的上游代码诊断标记为 `adopted` 但不排除（[ADR 0008](../../adr/0008-misra-baseline-and-no-deviation.md)），由 [#58](https://github.com/g1331/autsaro/issues/58) 跟踪。
- Windows 实际交接运行只在 CI 的 Windows 套件中执行。

## SDK 声明与常量

当前SDK仍有独立的R8.2缺口：项目私有Os_HostLinux.h直接包含pthread.h，其pthread_create声明的start_routine内层void指针参数未命名；Os_PortRestart.c直接包含GCC13的unwind.h，_Unwind_Trace_Fn、_Unwind_Personality_Fn及_Unwind_Backtrace声明也保留无名参数。主文的函数指针示例明确要求命名；TC2.10只增加交叉引用，TC2.12只澄清R8.3不比较无名参数，没有取消R8.2义务。AMD2追加C11，AMD3.63补充引用，均不提供当前例外。

主文§6.7对ISO标准头内容及处理标准头时包含的文件另有边界；这里两条直接入口分别是POSIX和GNU接口，不能把整个pthread／unwind SDK归入ISO标准库豁免。删除显式包含而保留间接链、手写替代声明或修改本机SDK，均不能证明交付闭包已整改。保留对应SDK声明需新的有界项目取舍或供应方合规材料；当前只记录真实缺口，不套ADR0024、不写MR12，也不将该取舍等同MISRA合法偏离。此处列的是已核具体声明，不是整个SDK的完整违规清单。

R8.14同为Required（主文印刷75／PDF83；AMD2追加C11）。同一pthread.h的pthread_create、pthread_attr_getstack等声明使用__restrict限定参数；这是当前GNU C99下的restrict类型限定，不因拼写有下划线而消失。标准库memcpy的原文示例不提供整个POSIX头豁免。这是独立于R8.2的已确认声明缺口，既有两头文件命名取舍提案尚未覆盖此规则；不扩张原提案、不擅自移除SDK限定或填写批准记录，其他实际SDK构造仍须继续核查。

R7.1为Required（主文印刷59–60／PDF67–68；单个0例外，AMD2.27追加C11）。Os_HostLinux.c经sys/eventfd.h实际包含bits/eventfd.h；其EFD_SEMAPHORE=00000001、EFD_CLOEXEC=02000000、EFD_NONBLOCK=00004000使用八进制，后两项直接用于事件创建。前者虽未选择，枚举声明仍进入TU。stdint.h处理先结束，再包含这些枚举，不能据主文§6.7把整个POSIX头豁免。当前libc6-dev为2.39-0ubuntu8.9，两个已核pthread／eventfd头与该固定源码字节一致；精确身份、替代和[有界SDK取舍补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6397172894)留PR。当前未接受，不填批准表／MR12，不套ADR0023／0024，也不据此声称其余SDK已评估。

## futex 返回信息

D4.7为Required（主文印刷30／PDF38，AMD2.14追加C11）；错误信息应尽快有意义地检查，只有证明函数不可能报错才适用免检条件。Linux private futex使用对齐且有存储期保证的32位字，但对应6.8.0-147.147发行源码的get_futex_key／futex_wake仍提供负错误返回；源码、发行补丁及论证留[PR调查](https://github.com/g1331/autsaro/pull/371#issuecomment-6398922631)。没有据此声称当前内核已启用故障注入，也不凭正常参数替全部结果作免检证明。

Os_HostLinux.c的私有futex_wait在终止停车、恢复停车、SuspendThread及停止ACK的共同边界区分0、-EAGAIN、-EINTR、-ETIMEDOUT；这些状态沿原子状态重新读取与原有10ms片段继续。其他结果沿既有原生致命出口退出1。Os_LinuxFutexWake在共享原生边界检查负结果，非负唤醒数保持；原始类型化Os_LinuxFutexWait仍返回kernel状态，未把错误改成成功。

Os_StackLinux.c仅在已发布fault后进入永久停车；其等待遇到不可恢复错误时直接以E_OS_STACKFAULT终止，不能返回原故障帧，不假报controllers_exhausted。没有新增输出阻塞点、期限、重试或共享状态，公开签名与handle布局不变。测试的真实零waiter唤醒、值不匹配和超时与私有返回状态注入分开记账；各检查撤销必须由精确阶段拒绝。此处不覆盖其他系统调用、provider完整异步语义或全部D4.7。

## 自有配置与保留名字

R21.2为Required（主文印刷166–167／PDF174–175，名字集合见R21.1印刷165／PDF173；TC1更正标题及示例，AMD2追加C11）。旧Os_Schedule局部量／参数time和自有Os_ConfigType.time复用标准库标识符；没有因其不是时间函数调用而销项。局部名及配置成员均改为timing，Os_Time和生命周期消费者同步；生成模板的具名初始化、成员类型／顺序、计时与调度语义不变。旧具名成员访问需更新并重新生成／构建，不能用宏别名恢复保留名。这是自有接口整改，不扩大ADR0024或批准SDK保留。

## 实现依赖、编译与需求追溯

D1.1、D2.1、D3.1均为Required，当前C99适用（主文印刷20–23／PDF28–31，AMD2.15–19及语言适用性更新）。D1.1采用AMD2.101–103重置的Appendix G.2（印刷20–23／PDF27–30）；其中C99／C11列分开，不能沿用TC1之前的J.3.11序号。实现选择被记录和理解，才构成对应依赖的证据；编译器版本、静态断言或绿色CI各自都不是全部实现定义行为的证明。

已核Linux正常编译参数为C99、O1、fexceptions和pthread，编译器发现记录为GCC13.3的x86_64-linux-gnu；CHAR_BIT为8、int为32位、long及指针为64位、字节序为little-endian，属于所选目标的事实。GCC13.3手册§4.1把stderr输出识别为诊断，§4.5明确二进制补码，§4.7描述指针／整数等宽转换的位保留；这些定义不授权越界指针、任意地址解引用或有符号溢出。字符／位域布局仍由目标ABI决定；Dcm的位域契约、原生上下文布局和实际编译断言各自保留，不能用LP64推导Windows的long或位域布局。§4.10也明确volatile访问不保证原子性；句柄、ACK、completion等共享发布依赖实际原子／锁协议，不把volatile当作同步。依据为[GCC13.3固定手册§4](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc.pdf#page=545)及实际编译器发现记录。

GCC手册将main参数、环境与多数库行为交给libc／操作系统。项目的Linux入口是hosted main，不能按freestanding入口作判断；OS_HOST_FAILURE_TESTS选择下的环境开关只按已记录格式解析，不把它们推广为任意环境名语义。pthread的TLS／线程回收、信号frame／UAPI、libgcc展开和Windows实际CRT／DLL的对应责任继续见[原生设施](native-facilities.md)。Linux已有libc6 2.39-0ubuntu8.9、libgcc-s1 14.2.0-4ubuntu2~24.04.1的字节／来源与相关pthread、loader、unwind源码核查，最终程序仍按自身ELF／装载绑定连接这些证据。Windows仍缺实际CRT／启动对象、导入最终解析到的系统DLL版本与映像摘要及对应线程／退出契约；最小补齐材料是相应正常入口的PE、导入／加载身份及配套实现或一手契约。检测器provider的拦截、fiber与异步调用上下文单列，不由普通provider或一次绿色运行代替。

D2.1按真实TU的编译错误判断。正常c-check先用实际参数逐TU编译检查，封存build随后实际编译／链接目标程序；生成工程成功、解析器退出或旧程序运行不替代这些步骤。GNU扩展按所选编译器及R1.2单独评估，不能把有效目标扩展统称语法错误，也不能把被编译器拒绝的生成代码交给旧绿色结果覆盖。Windows仍须由相应head实际选中的正常编译入口证明。

D3.1的项目方法是从自有接口／实际实现回到模块契约、设计与固定SWS，而非只比条款标题。Os、Time、Schedule的标准服务经后端映射到采用内核；[ADR0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)限定唯一backend、静态汽车对象和真实执行栈监测，[运行时架构](../../architecture/runtime.md)给出激活／独占区／HostBatch三层责任。HostBridge的监督与实际输出收据来自项目HostBatch合同，guard fault来自所选SC1栈合同；不能把5000ms批次监督解释为任意阻塞输出的完成保证。原生错误／回滚／停止处理属于这些合同的失败响应，不能仅因标准没有指定POSIX实现就删除。originalBswSources、patch manifest和seal证明角色／来源／字节身份，不证明需求追溯；采用内核和非ISO供应方调用仍须连到实际被选功能，未核定的ISO17356参数访问及供应方路径不据此销项。上述闭包支持这些明确的责任连接，不构成全部源代码、供应方或221项已满足。

## 非左值聚合与临时数组

R18.9为Required，当前C99适用（AMD3.39，印刷21–22／PDF29–30）；它限制非左值结构／union结果中的数组转换及修改，不等于禁止按值返回结构。当前四份生成工程的自有C／头文件、所选补丁后内核及自有Windows适配源码中，声明／typedef候选核查得到的按值聚合入口为Dcm_NextPublication、Arti_SaveAddressCapture、Os_ArtiCallerSave和Linux selected_task。前者只有kind、channel、mode及函数指针；两个ARTI快照只有整数状态和嵌套的整数地址记录；PortTask只有thread／yielded句柄。它们不含数组子对象，selected_task().thread等即时成员访问也不是临时数组转换。Task和portTASK_FUNCTION宏分别展开成void函数；不能将宏拼写当成未知聚合返回类型。

CONTEXT的FP数组、ErrorHook参数及Queue存储不因此取得一般豁免：实际上下文先保存在具名对象后按地址复制，ISR退役快照的聚合赋值是独立语句；复合字面量在C99中是有存储期的左值，其地址借用另按R18.6核查，不能靠R18.9销项。此判断是这些实际构造与调用的限定结论；候选脚本不是C语义分析器，不能证明任意新返回类型、用户组件、SDK宏或未取证Windows生成物。新增含数组的按值结果、aggregate条件表达式或立即取其成员地址时须重新检查。

## Windows 外部名字与所选工具

R1.5／Appendix F #2的Windows工具实现材料已取得：CI固定MSYS2 binutils 2.47-4包SHA256为e36c52a7fe1d104c2efd97dc74162386eabf8323f6aeb9b83aeb574b291a7da0，包BUILDINFO中的配方摘要267196b16e3c007ed71f809ca430cd4a0988010e9d05f05d71935a9acda1be8f与官方同版本source-only归档的PKGBUILD相符。GNU 2.47原始源包摘要3068128c75cda9f898ccb4211d360246e8e195ffcc9dfb655b23ae23a54800e8及八个发行补丁均与配方相符；这建立发行者声明的来源关联，未独立重建同字节工具。

x64 COFF外部名字经_bfd_coff_internal_syment_name、coff_link_hash_lookup和bfd_link_hash_lookup进入bfd_hash_lookup；hash遍历完整NUL终止串，以strcmp区分，复制len+1。coff_fix_symbol_name把超过SYMNMLEN的名字交给_bfd_stringtab_add，读端按字符串表offset返回；八字节短槽不等于整体名字上限。发行补丁未改变这些函数；i386pep配置使用pei-x86-64。此链支持所读实现不按小于255字符的前缀比较，不证明任意COFF工具、链接参数或未来版本。

仍须连接Windows实际编译器所选linker。现有正常CI准备步骤输出gcc -print-prog-name=ld、可解析绝对路径的版本与摘要；相对查询结果保持未核实，新的Windows执行收据未取得前不销项。固定包中ld.exe硬链接到ld.bfd.exe，实际内容SHA256为e5f877e604846435aedd5b1dedf9b62fbde7cfe24b65625a3d6f0d8f76ff5f80，后续收据须与该身份核对。本地仅做包／源文件读取，没有执行PE程序或名字边界实验；此项不替代最终CRT／系统DLL、导入解析及平台并发责任。
