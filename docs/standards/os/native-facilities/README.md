# 原生设施与资源生命周期

本篇记录受控OS的原生栈、上下文、展开及线程资源义务；[平台边界](../platform.md)连接C99扩展、ADR0024三处include和相关语言设施判断。固定规范、角色与处置方式沿用[OS标准入口](../README.md)。

## 源码边界与布局

Windows补丁`0018-windows-thread-setup-errors.patch`按D4.7／R17.7检查Task线程的affinity、boost、priority以及dispatcher的boost、affinity结果；失败沿现有断言或pdFAIL路径停止启动，不能进入Ready。`vPortCloseRunningThread`的priority调用同样检查，当前`INCLUDE_vTaskDelete=0`没有该删除调用路径，不声称它已被运行验证。依据[affinity](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadaffinitymask)、[boost](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriorityboost)与[priority](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriority)的零返回失败契约。Windows原生回归通过临时端口包装逐项拒绝五个启动调用，并与去掉0018的变异及正常启动对照；执行结果以实际Windows CI为准。

补丁`0019-windows-native-handoff-errors.patch`继续处置相同D4.7／R17.7义务：Task critical／yield、host中断发布、dispatcher及handler安装共同使用四个端口私有检查函数，验证mutex获取／释放、event发布／复位及无限yield等待；不改变锁或操作顺序。dispatcher的多对象WAIT_TIMEOUT继续等待，只有WAIT_OBJECT_0才能使用受保护状态；失败或abandoned结果停止OS，不能靠后续GetThreadContext成功冒充已取得mutex。现有控制线程承担停止，不新增状态／锁／分配。Windows回归保留真实正常／超时对照，逐类注入失败并与移除0019的变异比较；失败后同一actor继续调用原生端口时有独立终止哨兵，避免后续无关错误碰巧返回相同错误码。删除路径未启用的close／wait调用只作源码检查，不据此宣称运行验证。

Linux actor 的字节映射与数值栈界限保留真实 R11.4／R11.5 Advisory 违反，
其来源、限制与风险控制见[主机地址表示类别处置](#主机地址表示类别处置)。
原 void mapping 引出的 R11.6 已随字节存储类型消除，不能将它与余下转换混同。

`Os_PortRestartLanding`是保留Task-loop帧内的汇编续行点，不是按C原型调用的函数。该符号现在只在本对象内可见；C侧通过真正遵循调用ABI的`Os_PortRestartAddress`取得native PC：Linux为uintptr_t，Windows为RtlUnwindEx要求的void指针表示。叶函数在原`.os_code`内用RIP相对LEA返回RAX，保留其他寄存器及配对call/return，不需要C函数指针转整数或void指针。依据R11.1（Required，主文页93–94／PDF101–102；AMD2.58只修订隐式转换例外说明），不使用该条的新例外；实际native扩展仍按[平台扩展边界](../platform.md#c99-平台扩展的使用边界)的R1.2／D4.2／D4.3评估。原生restart验证10000次同一物理栈重入、拒绝与外来异常，并分别检查全局C入口和本地续行点的代码段身份；错误PC变异必须被实际restart成功判据拒绝。该边界仍依赖受控x64 ABI，Windows执行及硬件SHSTK按实际结果记录。

Linux 备用信号栈原匿名 union 仅借 long double 成员取得对齐，实际消费者为 `Os_StackRegister → Os_LinuxAltStack`。R19.2（Advisory、C99，主文页154–155／PDF162–163；AMD2 只增加 C11 适用性及行为索引）直接约束 union 关键字，不因没有交叉成员读取而不适用。现改为仅含 long double 存储数组的 struct，每个 actor 仍取得原来的 65536 字节和原生对齐；C 代码不读取浮点值，不新增平台属性或转换。真实源的编译布局对照检查容量、对齐与槽数，降低对齐和缩短容量的反例分别拒绝；ARTI 消费者另检查发布地址的实际对齐。该整改不改变注册、信号及故障关闭路径。

`Os_ErrorParametersType` 同样改为各服务参数各占独立字段的 struct。服务包装按名初始化，`Os_ErrorResult` 复制参数和服务标识，getter 保持“匹配的 ErrorHook 内”这一前置条件；`Os_ErrorContextSave/Restore` 连同 phase 保存及恢复，C 消费者不依赖原来的重叠表示。所选 GCC x64 的对象从24变272字节，现有 ErrorHook 各访问开关、嵌套拒绝及 ISR 消费者检查实际字段；丢失参数和损坏服务标识的编译变异分别被现有完整协议判据拒绝，进程 exit0 不能替代该判据。

采用内核的 `QueueDefinition.u` 在共同受控补丁中也改为独立字段，保留 `xQueue`／`xSemaphore` 名称及全部原有分支。所选配置为静态分配开启、动态分配和 mutex 关闭，真实 FIFO／peek／满空／reset 路径消费 `xQueue`，不需要另一组字段的存储表示；未通过启用其他配置来替代所选配置的验证。该目标的 StaticQueue_t 从192变208字节，u从16变32字节；静态缓冲仍直接使用同一完整 QueueDefinition 类型。补丁摘要及双平台派生内核身份随源重新计算，原上游文件保持固定身份。两处布局均需重新生成交付物并整体重编；此逐处 R19.2 整改不外推系统头、未选变体或整个采用内核符合性。

## 实际provider及异步调用

目标编译器身份不等于最终运行库身份。一次正常Linux生产probe的实际绑定与官方包字节对照确认libc6为2.39-0ubuntu8.9、libgcc-s1为14.2.0-4ubuntu2~24.04.1；不能把GCC13编译器版本写成libgcc版本。该次ELF／source seal、加载器、库摘要、真实编译argv与运行收据集中[PR #371](https://github.com/g1331/autsaro/pull/371#issuecomment-6394646433)。它证明这一份运行的provider，不锁定所有Ubuntu安装，也不外推Windows或检测器拦截库。provider／ABI／编译与链接参数变化必须重评相关判断。

对应发行源码已按Ubuntu archive签名的InRelease→Sources.xz→原始源码及发行补丁摘要核对；实际libc／loader／libgcc与官方deb解包字节一致。标准库的实现内容与项目的调用分别按MISRA主文§6.7判断，不能据此免除第三方机制的D4.12或平台ABI／错误语义。libgcc的GNU展开接口另沿实际实现核对，不冒充ISO标准库设施。

Linux返回型停车handler保存并恢复被打断actor的errno；futex／gettid／altstack边界使用既有类型化UAPI，直接保留原生返回信息。`Os_LinuxCaptureContext`只读取截至64位内核mask的304字节前缀及独立512字节传统FP，先构造初始化的局部快照、重定位到目标自有FP，再赋给目标，支持self-copy；SignalSp只读取截至Rsp的168字节。旧ELF的memmove绑定证据不冒充新复制布局的执行证明，停车handler仍使用__errno_location。对应glibc的`manual/string.texi`分别将memcpy／memmove标为AS-Safe，x86 IFUNC选择进入字节复制实现；该次ELF的NOW绑定先于handler使用。此依据与实际地址／长度、发布顺序、信号屏蔽及errno恢复共同支持当前闭包，不把它扩成任意库调用、任意动态加载或全部调度的证明。

guard handler仅对注册actor的实际guard地址发布预分配记录、真实SP及首故障状态，然后永久停车；未识别故障原生退出。该路径没有格式化输出、堆分配或汽车回调。正常关闭报告中的有界字节数／预分配容量、失败回退与任意阻塞输出的时间保证分开；现有HostBatch监督及退出后备的验证边界见[平台记录](../platform.md#三处有界-linux-平台保留)，不为probe或所有终止输出新增期限。

对应运行内核6.8.0-147-generic的Ubuntu linux 6.8.0-147.147源码及发行补丁也按同一签名Sources核验。x64_setup_rt_frame将第三实参设为frame.uc地址，uc_mcontext.fpstate另指向fpu__alloc_mathframe分配的FP区域；[Linux sigaction接口](https://man7.org/linux/man-pages/man2/sigaction.2.html)将第三实参明确为转成void指针的ucontext_t。实际TU中的UAPI struct ucontext使用ambient glibc sigset_t，sizeof并非内核帧长度。现以固定偏移和布局断言限定读取；glibc完整sigset、内嵌FP及SSP尾部不从信号帧整块复制。

Os_StackObserve只接受CONTEXT_CONTROL并检查真实Rsp；原生设施对照另外观察GPR、512字节传统FP内容及指针重定位。项目没有用该观察快照调用setcontext或恢复整个XSAVE区，不能把512字节副本称为完整扩展状态恢复。actor继续执行由原始信号帧的sigreturn承担，内核按其实际FP／XSAVE记录恢复。现有self-copy、指针独立性和内容对照支持这些具体消费者，不能替代所有信号交错、其他内核／libc布局或新恢复消费者的评估。

## 展开器先于可抢占执行初始化

对应GCC14的`unwind.inc::_Unwind_RaiseException → uw_init_context`及`unwind-dw2.c::uw_init_context_1`通过共享`once_regsizes`／__gthread_once初始化寄存器表；实际libgcc二进制也调用pthread_once。对应glibc实现使其他线程等待进行中的初始化。若把首次调用留在释放汽车临界区后的Task展开中，OS可能暂停初始化者，再让另一Task进入同一等待，形成对被暂停者的依赖。单Task样本或短时通过不能排除受支持多Task配置中的该窗口。

Linux的`prepare_native_runtime`在创建控制器／Task及启动调度器之前调用`Os_PortRestartPrepare`，以真实_Unwind_Backtrace遍历当前启动栈；无状态回调返回_URC_NO_REASON，只有正常到达栈尾才允许继续，其他结果沿E_OS_STATE关闭。没有增加共享标志、锁、actor、重试或期限，也没有在临界区内包住非局部展开；激活结算和同一Task／TCB／物理栈的restart路径保持。依据为D4.1（Required，主文24–25／PDF32–33）、D4.7（Required，30／38）与D4.13（Advisory，35–36／43–44），AMD2的C11扩展不改变当前C99义务。

原生观察复用正常lifecycle夹具，保留真实backtrace与pthread_create；检查真实帧遍历先于该夹具的控制器／Task创建、正常Ready和初始化错误时不得Ready。生产probe／HostBatch在StartOS之前已创建H输入线程，本项不承诺早于所有native线程；该输入闭包不调用展开器。缺准备调用、伪造栈尾结果而跳过真实provider、忽略初始化失败的编译副本分别由精确阶段拒绝。旧源码对照观察的是准备协议缺席，不冒称已复现生产死锁。执行与检测器结果按PR中的实际源码及运行范围记录；此项不消除provider中其他锁、运行中加载新DSO或新展开实现的再评估责任。Windows的SEH路径不使用该GNU准备操作。

当前生产ELF与libc有NOW标志，实际libgcc_s没有；不能从主程序的-z now推导所有依赖均提前绑定。GCC14的_RaiseException经uw_identify_context调用_GetCFA，实际反汇编保留自身PLT；单actor普通restart消费另观察到GetCFA绑定回同一libgcc。glibc2.39的_dl_fixup以线程本地GSCOPE标志保护查找，reset仅在必要时唤醒等待者；依赖登记明确跳过self引用。这个普通FUNC的自身关系不要求新装载依赖，不把尚未绑定本身等同于新的全局互斥等待；但也不将本次源核对扩成全部PLT、符号插入／审计、动态装卸或所有异步交错安全。该收据与上述once准备分别记账。

## 动态存储及回收责任

原文、调用闭包和已接受的 D4.12＋R21.3／R22.1 类别记录见[动态设施](storage-interrupts.md#主机动态设施类别偏离)。

## 嵌套中断与递归的实际支持范围

实际支持配置、风险控制与 R17.2 类别记录见[同步嵌套 ISR](storage-interrupts.md#同步嵌套-isr-类别偏离)。

## 自有回调声明的参数名

R8.2为Required、当前C99适用（主文64–65／PDF72–73）；命名要求也适用于函数指针中的参数声明。自有的LPTHREAD_START_ROUTINE、Os_TimeConfigType.error_hook及Os_PortTaskLoop内层函数参数现分别命名为argument／Error／context，没有固定SWS要求它们匿名。只改变参数声明名字，不改变返回／参数类型、布局、符号或ABI；不以固定checker没有报告这些位置判为满足。原RTE无名role参数及Det误报按其各自已核闭包处理，此处不借用它们的处置。SDK自身声明和未读取的其他声明仍按各自来源与适用性判断，不随这三处整改自动通过。

R8.3（Required，主文66–67／PDF74–75）的参数名比较独立于上述命名义务。实际声明／定义核对发现Os_BackendSignalStackFault的role／failed_role和Linux WaitForMultipleObjects的handles／observed不一致，现只将两个声明统一到定义名字，类型及调用不变。Windows／Linux Can_InitLock／Can_InitAreas的不同签名属于互斥平台分支，两份main属于不同可执行程序，不能作为同一声明链误报；函数指针参数的名字也不要求与所指函数一致。该判断来自具体声明及实际编译边界，不把候选检索或checker零报告扩为全部声明符合。

SDK声明及宏的独立Required缺口见[SDK声明与常量](../README.md#sdk-声明与常量)；不随自有声明整改自动通过。

## 标准流对象与原生句柄

R21.6为Required；R22.5／R22.6为Mandatory（主文印刷176–177／PDF184–185），禁止解引用FILE及关闭后继续使用其指针。R22.7为Required（AMD1印刷12／PDF20），限制EOF与未经修改的标准库返回值比较；AMD2追加C11，并由AMD2.85澄清FILE对象复制的理由。这些条款在当前C99适用，不能写成语言不适用。

当前四份callback-parameter生成工程的自有／生成C及头文件没有FILE、EOF或stdio调用。两个目标的实际补丁后内核保留六处snprintf，但都在configUSE_STATS_FORMATTING_FUNCTIONS大于0的分支；当前FreeRTOS.h默认该值为0，实际TU宏收据一致，configGENERATE_RUN_TIME_STATS也为0，不能把这些排除位置当成当前用途，亦不删除上游功能。configUSE_TRACE_FACILITY为1并不单独启用格式化。这里支持当前源码的标准流对象／调用缺席，不豁免SDK或provider内部。

HostBatch读取STDIN_FILENO，HostIo及故障输出写STDOUT_FILENO／STDERR_FILENO；Linux事件由单个eventfd读写，栈范围查询只以O_RDONLY打开/proc/self/maps，Windows使用原生句柄。它们不构成FILE对象，也不能因此免除D4.7／D4.13及平台资源契约。R22.3明确指出相似执行环境设施可能适用；将其完整结论扩展到任意外部启动方式，仍需知道实际继承的输入／输出对象、访问模式与是否别名，不能由没有fopen或绿色CI推出。既有受管测试使用分离管道的证据只覆盖那些具体启动，不新增用户输出期限或任意文件重定向保证。

R21.8的有效Required文本禁止abort／exit／_Exit／quick_exit（AMD2.81，印刷16／PDF23），当前C99适用。四份callback-role语料及双目标补丁后内核没有这些标识符或宏用途；ExitProcess最终走POSIX _exit或Win32入口，不能按名称相似冒充标准库函数，也不因此证明provider内部或任意退出路径符合。


## 主机地址表示类别处置

规则 R11.4、R11.5 均为 Advisory（主文印刷97–98／PDF105–106；
TC1 将 R11.4 限定到对象指针，AMD2.60 扩充 stdint 说明）。
uintptr_t 的可表示性没有构成 R11.4 例外，R11.5 同时覆盖显式 cast 与隐式赋值。
依 [ADR 0025](../../../adr/0025-misra-layered-scope.md) 的自有主机适配层及
[项目决定第1节](https://github.com/g1331/autsaro/issues/58#issuecomment-6437338057)，
以下记录限宿主接口决定的地址表示，不借用动态设施的 Required 类别授权。

范围仅 linux-x64-controlled-v1、四 profile 的原生 OS／HostBatch／probe 与相同实现的
native 消费者：`Os_HostCreateActor` 将 mmap 返回的 void 指针保存为 unsigned char 映射；
`Os_HostStackRange` 将该映射地址转为 uintptr_t；`Os_StackRecordBuffer` 将静态内核缓冲的
one-past 地址转为 uintptr_t。均由自有代码实施，不能整体移交 SDK 来源责任。

mmap 的固定 ABI 返回 void 指针；guard、ASan fiber 和实际栈入口需要同一映射内的字节偏移。
改成整数搬运、memcpy 指针表示或换一层 helper 不消除该转换。
栈保护比较的是来自不同存储对象的 native 地址区间（包括真实 RSP 和静态内核缓冲），
不能用 C 对同一数组内指针的大小比较替代。数值区间是既有 ARTI／原生栈合同的输入，
在受控 x64 上 uintptr_t 完整容纳地址；结果不转换回对象指针解引用。

风险是错误对齐、截短、溢出及跨对象计算错误。mmap 保证页对齐，映射长度与 guard
来自同一次成功创建，失败和回滚保持所有权；字节偏移限该映射，内核缓冲由实际静态
Task 存储提供。既有 native_stack／ARTI 消费者观察物理栈、注册区间和汽车缓冲不重叠，
host boundary 检查创建、errno、失败回收和句柄复用；检测器与目标 CI 另按实际运行留证。
可修复的 watch／Task／Linux actor 回调参数不在此类别：它们使用既有静态对象或按地址
匹配固定 typed 槽，消除反向 void 转换，并保留异常参数拒绝的反例。

重评条件为目标 ABI／地址宽度、provider、mmap flags／容量、栈布局／guard、ARTI
数值合同或缓冲所有权变化；Windows 地址转换不由这份 Linux 证据代为覆盖。
保留原始诊断和分析身份，类别处置不表示源码符合这两条规则。


类别实例由 `c_standard.host_address_instances` 逐源码确认：仅在上述三个命名函数体内，
匹配固定的 mmap 赋值、映射地址转换和 one-past 缓冲地址转换；文件、函数或表达式改变、
缺失／重复实例均拒绝，MR12 规则及就近注释双向关联。类别文档缺失同样拒绝。
`test_host_address_category_requires_exact_owner_expression_and_annotation` 检查正常实例、
缺注释、错规则、错表达式／函数、函数体外、重复及缺记录反例。
这些项目检查只识别已审阅的来源模式，不替代 C 类型／行为分析、调用合同或 MISRA 判断。
