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

非ISO头的参数、限定符、常量、inline和变参义务及待决定边界见[声明与SDK接口](declarations.md#sdk-声明与常量)。

## futex 返回信息

D4.7为Required（主文印刷30／PDF38，AMD2.14追加C11）；错误信息应尽快有意义地检查，只有证明函数不可能报错才适用免检条件。Linux private futex使用对齐且有存储期保证的32位字，但对应6.8.0-147.147发行源码的get_futex_key／futex_wake仍提供负错误返回；源码、发行补丁及论证留[PR调查](https://github.com/g1331/autsaro/pull/371#issuecomment-6398922631)。没有据此声称当前内核已启用故障注入，也不凭正常参数替全部结果作免检证明。

Os_HostLinux.c的私有futex_wait在终止停车、恢复停车、SuspendThread及停止ACK的共同边界区分0、-EAGAIN、-EINTR、-ETIMEDOUT；这些状态沿原子状态重新读取与原有10ms片段继续。其他结果沿既有原生致命出口退出1。Os_LinuxFutexWake在共享原生边界检查负结果，非负唤醒数保持；原始类型化Os_LinuxFutexWait仍返回kernel状态，未把错误改成成功。

Os_StackLinux.c仅在已发布fault后进入永久停车；其等待遇到不可恢复错误时直接以E_OS_STACKFAULT终止，不能返回原故障帧，不假报controllers_exhausted。没有新增输出阻塞点、期限、重试或共享状态，公开签名与handle布局不变。测试的真实零waiter唤醒、值不匹配和超时与私有返回状态注入分开记账；各检查撤销必须由精确阶段拒绝。此处不覆盖其他系统调用、provider完整异步语义或全部D4.7。

## 自有配置与保留名字

R21.2为Required（主文印刷166–167／PDF174–175，名字集合见R21.1印刷165／PDF173；TC1更正标题及示例，AMD2追加C11）。旧Os_Schedule局部量／参数time和自有Os_ConfigType.time复用标准库标识符；没有因其不是时间函数调用而销项。局部名及配置成员均改为timing，Os_Time和生命周期消费者同步；生成模板的具名初始化、成员类型／顺序、计时与调度语义不变。旧具名成员访问需更新并重新生成／构建，不能用宏别名恢复保留名。这是自有接口整改，不扩大ADR0024或批准SDK保留。

## 实现与供应方边界

实现定义、编译／需求追溯、临时聚合、Windows外部名字、线程TLS分配、声明宏及SDK保留名字见[实现与供应方边界](implementation.md)。各条的适用条件、原始诊断与待决定状态分别保留。

## 对象地址与回调的转换

R11.4、R11.5均为Advisory，当前C99适用（主文印刷97–98／PDF105–106；TC1限定R11.4为对象指针，AMD2.60及语言表扩展不取消义务）。当前switch-termination的standard-ecu原摘要中六处R11.4、五处R11.5均为真实转换，不按uintptr_t可容纳指针或void回转合法判误报；保留具体用途和原始诊断，不构成整条符合或新偏离批准。

R11.4分别来自Os_ArtiAlarmBaseReturn记录完整地址／32位trace token、Os_HostThreadLayout导出映射范围、Os_StackRecordBuffer记录逻辑缓冲范围，以及采用tasks.c的栈顶对齐表达式两次转换及对齐断言。前几处用于地址身份、范围或输出，没有把截断的trace token恢复成指针解引用；采用栈顶仍须按实际StackType_t缓冲、深度与8字节对齐前提判断。当前x64的uintptr_t、size_t及64位地址记录由实际ABI／编译收据支持，不能外推任意整数、地址或Windows未取证供应方。

R11.5的HostBridge watch恢复实际传入的静态Ecu_HostWatch地址；native_task_start恢复native_starts中的同型元素；Linux actor_start恢复reserve_handle返回的真实句柄对象。这些回调的生产者和消费者类型相配，借用和退休仍按原生生命周期篇处理，不由一次强制转换证明无竞态。mmap返回的void地址转为mapping字节视图，失败先检查MAP_FAILED后再使用；其动态设施仍保留D4.12和供应方责任。

第五处是采用内核prvTaskStatusArrayWriter从void恢复TaskStatusArrayWriterContext_t，并非TCB转换。uxTaskGetSystemState先写入xContext的数组指针及uxIndex=0，再在挂起调度期间同步遍历／调用，返回后才离开该自动对象作用域；这处未发现保存context供以后调用。调用方容量、真实遍历和输出对象仍各负其责，不能从该转换推导所有外部callback安全。C99对象指针经void回转的语言依据为N1256 §6.3.2.3p1（印刷47／PDF59），对象—整数转换另属实现定义；两者均不消除上述Advisory违反。

MAP_FAILED另触发独立R11.6 Required义务（主文印刷99／PDF107；TC2.22澄清整数转void指针属实现定义，未取消规则）。当前Os_HostCreateActor实际失败比较展开固定sys/mman.h的((void *) -1)，不适用零常量例外，也不是R11.5的对象指针恢复。失败仍先识别该哨兵再释放槽位返回NULL；改看NULL／errno、搬动声明或改用同样哨兵的mmap64均非等价整改。针对这个精确SDK构造的有界保留仍待项目决定，不扩ADR0024、不填批准表／MR12；具体身份、替代与风险留PR #371。

## 启动发布与中断交接

R13.2为Required（主文印刷109–110／PDF117–118，AMD2追加C11）。TC2.23–25要求不因已知值而忽略表达式中的访问；AMD4.17–23（印刷14–15／PDF21–22）进一步扩展线程交错、atomic访问及单次快照示例。多个原子访问也不能自动当成同一个不可分割表达式；这里还按D4.1核对真实并发路径，volatile本身不提供线程同步。Linux的xInsideInterrupt曾由dispatcher普通写入，同时由已恢复Task的portSOFTWARE_BARRIER读取；启动线程也可能在Os_PortTaskThread发布task_threads及计数之前因注册失败唤醒关闭控制器。原有退出码与协议断言没有排除检测器报告，因此不能将历史栈套件的预期退出当作TSan干净证据。

当前Os_PortTaskThread复用既有auto-reset gate：先完成Os_PortThread的句柄登记、task_threads写入与计数更新，再发出第一次gate通知；native_task_start收到后才注册栈并发布registered。创建方仍沿原有5000ms注册等待、暂停／上下文观察和第二次gate通知继续，未增加事件、容量或期限。第一次gate等待失败直接原生终止，不进入尚未完成发布的关闭遍历。注册失败发生时创建方等待registered，不继续创建下一个Task。

控制器健康状态的全部读写统一使用既有Interlocked接口；Linux中断标志保留BaseType_t及公开布局，dispatcher写入、critical路径与软件屏障读取均使用GCC顺序一致原子操作。固定GCC13.3手册§6.55（印刷733／PDF749）明确这些内建采用C++11内存模型并假定程序没有数据竞争；这不是让混合的普通访问自动安全。软件屏障的循环同时改为显式复合语句，处置实际展开后的R15.6；原先仅检查闭合宏的词法结论没有覆盖这一开放式while宏。

证据仅限实际运行的原有栈故障、暂停／恢复消费者及对应原子／事件发布链；不外推完整信号异步语义、所有provider或全部R13.2。检测器构建的正常execute入口现在拒绝stdout／stderr中的TSan、ASan、LSan与UBSan报告，即使产品退出码原本预期为7或13；故意验证检测器的专用run_native入口仍按其独立断言处理。历史原始报告和passed／error／not_assessed不改写。

## 关闭理由的并发发布

D4.1与R13.2还约束普通关闭与独立原生guard故障的交错。Os_BackendRequestShutdown的首次关闭RMW与Os_BackendSignalStackFault的RMW，均发生在其各自理由写入之前；这些RMW的顺序不能单独为后续两次普通写建立happens-before。H输入actor可与汽车Task／bootstrap独立执行，不能借单核调度排除该窗口；控制器后来修正最终理由也不能消除先前数据竞争。

shutdown_reason保留StatusType、8位清零段及原有值，全部写入使用GNU顺序一致原子store，report_and_exit原子取一次局部快照，供ShutdownHook、生命周期文本及退出码共用。编译要求__GCC_ATOMIC_CHAR_LOCK_FREE为2；Platform_Types的uint8／StatusType与本目标char同宽，不引入可被暂停者持有的锁。既有关闭通知、actor退休、栈故障优先及输出失败处理不变，不增加状态、等待或产品期限。

验证在原生测试副本中围绕真实生产写安排握手：先让bootstrap普通关闭赢得admission，再由已注册H线程触及实际guard页；两条路径都到达写入位置后才完成写，控制器仍经既有关闭观察点继续。正常、ASan／UBSan及TSan分别保留实际reason=13、H故障和双发布阶段。将同一副本的实际原子store恢复为普通写时，正常execute的检测器检查须拒绝明确指向shutdown_reason及两个发布函数的data-race报告；产品仍可能以13退出，不能只检查通用非零码。此项处置该具体并发对象，不扩为全部信号、provider或R13.2符合。

首故障快照还有独立的发布边界：每个actor先完成并发布自己的fault record，再由CAS赢家填写全局Os_Fault；另一个actor可以先递增fault_count并唤醒关闭控制器。因此，fault_count非零并不证明全局thread_id已可读取。Os_BackendShutdown现复用既有原子controller_healthy判断当前C／D是否能报告，不读取尚未发布的首故障载荷，也不等待可能已损坏的赢家。

对应原生副本让真实Task和备用控制器分别触发guard fault，在首故障全局载荷尚待发布时进入健康控制器分支。普通与ASan／UBSan检查保留两个FAULT及reason=13；TSan的正常入口还要求健康状态读取无报告，恢复全局thread_id读取的已编译副本则须由指向Os_Fault、publish_fault和Os_BackendShutdown的竞态报告拒绝。一次未使用仓库规定setarch入口的TSan启动映射失败仍是环境失败，不计产品结果。此项不把fault_count改成全局载荷完成标志。

Windows的注册记录同样区分预留与发布：stack_count只预留数组槽，新增的私有stack_published沿用Linux既有模式，在全部范围、guarantee及Os_StackCheck完成后才Interlocked发布。Observe／Report原子读取预留计数，先确认对应槽已发布再读取载荷；注册中途失败的部分记录不再因thread_id非零而被遍历。FatalExit／Report对全局／逐actor故障的published也使用Interlocked读取，与现有发布写配对。公开记录布局、容量、注册返回值及原有停止观察协议不变。这些修改尚缺新字节的真实Windows编译和执行证据；Linux检测器结果不替代它，也不证明外部工具任意并发读取ARTI记录是安全的。

## 循环与函数出口

采用内核IDLE名读取的[逐路径裁决](https://github.com/g1331/autsaro/pull/371#issuecomment-6428838743)证明当前索引最多为4；原始越界报告保留，不外推其他配置或整个R18.1。

R15.4／R15.5均为Advisory（主文印刷125–127／PDF133–135，AMD2追加C11）。补丁后xTaskIncrementTick延迟链循环的两个break分别保护空链和未到期队首；这是保留真实违反及读队首／移除前保护的理由，不是采用代码豁免。R15.5同样要求函数末尾单一return，参数guard也不例外。

对ce1d043e及scoped-mapping-final-results的standard-ecu逐字对应源码，先核对以下43个报告位置，均保留真实多出口。Os_Error.c的79／82／92／98属于Os_HookServiceAllowed：仅按本线程phase选择权限，所调用Os_HookQueryContext也是纯查询；没有锁、输出、hook调用或待清理对象。保留各phase的直接返回使权限分支局部可读，不把它判为R15.5满足。

Os_Mailbox.c的13处为101／104／107／110／114／118／121、176、197、216／220／223、235。PostInput的七个拒绝均在成功取得admission之前，不写ticket或记录；成功取得后先完整复制、发布，再清busy位，才返回。BridgeContext拒绝在生产者CAS之前；成功CAS取得的线程绑定是进程期sole-producer责任，后续容量拒绝不承诺撤销它。input_context无资源获取；TakeInput的三个早退发生在复制／推进read_index之前；InputCount的空指针拒绝不写输出。GuardService的终止路径不返回，不是遗漏调用方释放。这些先决条件的相邻拒绝可避免无效地址和未授权访问，不扩为全部Mailbox并发证明。

Os_Schedule.c的26处按实际函数核对：table_index:32、valid_action:58为只读查找；Validate:73／79／85／89／97／105／109／117／125只检查输入，不发布表状态。start_table:253／257／264、Stop:295／299、Next:329／336、GetStatus:364／368／372均在taskENTER_CRITICAL之前；进入后的各路径统一EXIT、GuardService后返回，GetStatus只在成功路径写输出。五个公开wrapper的395／413／429／447／465在内部实现已收尾后跳过递归ErrorHook处理，没有遗留局部参数地址；其他调用仍经Os_ErrorResult完成hook快照恢复。保留先决条件和ErrorHook分支的局部出口，不新增状态或改拒绝优先级。以上仅为这些位置的作者源码判断；standard-ecu其余285处及其他profile的特有位置尚不能由本段销项，原始人工字段不改写。

Os_Time.c的64处另按只读验证、临界区前拒绝、tick预留后的关闭及ErrorHook收尾逐处核对，位置与状态边界见[完整作者依据](https://github.com/g1331/autsaro/pull/371#issuecomment-6417003748)。特别区分ResetEvent失败后的协议关闭与并发关闭导致的末次CAS失败；不把它们当成无副作用的普通guard，仍保留R15.5违反。

## 栈记录与指针运算

R18.2／R18.3均为Required（主文印刷145–146／PDF153–154，AMD2追加C11）；AMD4.52将R18.3标题的对象改为表达式，没有放宽同对象关系。两个平台的Os_StackSavedContext和publish_fault都通过record与本模块stacks数组的差取得对应saved_contexts／fault_records下标。真实调用方的记录来自成功Os_StackRegister发布的current_stack，或Os_StackObserve在0至OS_NATIVE_STACKS内枚举的stacks元素；Backend只在注册成功之后保存ARTI指针，ARTI消费者读取该已保存记录。数组有21个元素，元素差可由目标ptrdiff_t表示。未发现当前闭包向这些减法传入别的对象或空指针。

Linux额外的record范围比较不能作为接受任意外部地址的安全验证器：C有序指针比较本身已要求同对象关系。这里的成立前提是上述实际来源；Windows没有这组范围比较，也依赖相同的私有前置条件。新增公开任意地址查询或退役后重用记录时必须重新设计，不能沿用这一证明。Os_StackRecordBuffer的top来自内核该Task的栈数组，top+1至多为该数组末尾后一位置；转换后的地址范围统计仍另承担R11.4及平台地址表示义务。

所读407源码标准ECU摘要的12个R18.4位置中，Os_StackSavedContext／publish_fault的两处record减stacks适用主文R18.4明确允许同数组指针相减的例外（印刷147／PDF155）；不是将全部指针运算判误报。该封存版其余十处为真实Advisory构造：两处范围上界加法、top+1、采用内核的水位扫描游标推进和六处队列步长／回绕运算。后续自有范围上界改为&stacks[OS_NATIVE_STACKS]，两个平台的栈顶改为&top[1]，采用原文建议的下标取址形式；数组关系、所指位置及R11.4转换保持，不借此批准任意指针。七处内核步长运算仍保留具体表示理由。队列的边界修复依据仍见平台篇，不由Advisory理由免除R18.1；水位扫描及其他调用配置也不能仅凭零诊断取得整体边界证明。这里保留全部原始位置与状态，只裁决已列构造。

## 自有接缝的声明一致性

具名参数、类型拼写、条件接口及声明owner的逐处依据见[声明与SDK接口](declarations.md#自有接缝的声明一致性)。
