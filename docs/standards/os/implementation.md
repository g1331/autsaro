# OS实现与供应方边界

本篇承接[OS标准说明](README.md)，区分所选C99实现、供应方证据及仍待决定的规则责任。

## 实现依赖、编译与需求追溯

D1.1、D2.1、D3.1均为Required，当前C99适用（主文印刷20–23／PDF28–31，AMD2.15–19及语言适用性更新）。D1.1采用AMD2.101–103重置的Appendix G.2（印刷20–23／PDF27–30）；其中C99／C11列分开，不能沿用TC1之前的J.3.11序号。实现选择被记录和理解，才构成对应依赖的证据；编译器版本、静态断言或绿色CI各自都不是全部实现定义行为的证明。

已核Linux正常编译参数为C99、O1、fexceptions和pthread，编译器发现记录为GCC13.3的x86_64-linux-gnu；CHAR_BIT为8、int为32位、long及指针为64位、字节序为little-endian，属于所选目标的事实。GCC13.3手册§4.1把stderr输出识别为诊断，§4.5明确二进制补码，§4.7描述指针／整数等宽转换的位保留；这些定义不授权越界指针、任意地址解引用或有符号溢出。字符／位域布局仍由目标ABI决定；Dcm的位域契约、原生上下文布局和实际编译断言各自保留，不能用LP64推导Windows的long或位域布局。§4.10也明确volatile访问不保证原子性；句柄、ACK、completion等共享发布依赖实际原子／锁协议，不把volatile当作同步。依据为[GCC13.3固定手册§4](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc.pdf#page=545)及实际编译器发现记录。

GCC手册将main参数、环境与多数库行为交给libc／操作系统。项目的Linux入口是hosted main，不能按freestanding入口作判断；OS_HOST_FAILURE_TESTS选择下的环境开关只按已记录格式解析，不把它们推广为任意环境名语义。pthread的TLS／线程回收、信号frame／UAPI、libgcc展开和Windows实际CRT／DLL的对应责任继续见[原生设施](native-facilities/README.md)。Linux已有libc6 2.39-0ubuntu8.9、libgcc-s1 14.2.0-4ubuntu2~24.04.1的字节／来源与相关pthread、loader、unwind源码核查，最终程序仍按自身ELF／装载绑定连接这些证据。Windows仍缺实际CRT／启动对象、导入最终解析到的系统DLL版本与映像摘要及对应线程／退出契约；最小补齐材料是相应正常入口的PE、导入／加载身份及配套实现或一手契约。检测器provider的拦截、fiber与异步调用上下文单列，不由普通provider或一次绿色运行代替。

D2.1按真实TU的编译错误判断。正常c-check先用实际参数逐TU编译检查，封存build随后实际编译／链接目标程序；生成工程成功、解析器退出或旧程序运行不替代这些步骤。GNU扩展按所选编译器及R1.2单独评估，不能把有效目标扩展统称语法错误，也不能把被编译器拒绝的生成代码交给旧绿色结果覆盖。Windows仍须由相应head实际选中的正常编译入口证明。

D3.1的项目方法是从自有接口／实际实现回到模块契约、设计与固定SWS，而非只比条款标题。Os、Time、Schedule的标准服务经后端映射到采用内核；[ADR0004](../../adr/0004-freertos-kernel-with-own-os-semantics.md)限定唯一backend、静态汽车对象和真实执行栈监测，[运行时架构](../../architecture/runtime.md)给出激活／独占区／HostBatch三层责任。HostBridge的监督与实际输出收据来自项目HostBatch合同，guard fault来自所选SC1栈合同；不能把5000ms批次监督解释为任意阻塞输出的完成保证。原生错误／回滚／停止处理属于这些合同的失败响应，不能仅因标准没有指定POSIX实现就删除。originalBswSources、patch manifest和seal证明角色／来源／字节身份，不证明需求追溯；采用内核和非ISO供应方调用仍须连到实际被选功能，未核定的ISO17356参数访问及供应方路径不据此销项。上述闭包支持这些明确的责任连接，不构成全部源代码、供应方或221项已满足。

## 非左值聚合与临时数组

R18.9为Required，当前C99适用（AMD3.39，印刷21–22／PDF29–30）；它限制非左值结构／union结果中的数组转换及修改，不等于禁止按值返回结构。当前四份生成工程的自有C／头文件、所选补丁后内核及自有Windows适配源码中，声明／typedef候选核查得到的按值聚合入口为Dcm_NextPublication、Arti_SaveAddressCapture、Os_ArtiCallerSave和Linux selected_task。前者只有kind、channel、mode及函数指针；两个ARTI快照只有整数状态和嵌套的整数地址记录；PortTask只有thread／yielded句柄。它们不含数组子对象，selected_task().thread等即时成员访问也不是临时数组转换。Task和portTASK_FUNCTION宏分别展开成void函数；不能将宏拼写当成未知聚合返回类型。

CONTEXT的FP数组、ErrorHook参数及Queue存储不因此取得一般豁免：实际上下文先保存在具名对象后按地址复制，ISR退役快照的聚合赋值是独立语句；复合字面量在C99中是有存储期的左值，其地址借用另按R18.6核查，不能靠R18.9销项。此判断是这些实际构造与调用的限定结论；候选脚本不是C语义分析器，不能证明任意新返回类型、用户组件、SDK宏或未取证Windows生成物。新增含数组的按值结果、aggregate条件表达式或立即取其成员地址时须重新检查。

## Windows 外部名字与所选工具

R1.5／Appendix F #2的Windows工具实现材料已取得：CI固定MSYS2 binutils 2.47-4包SHA256为e36c52a7fe1d104c2efd97dc74162386eabf8323f6aeb9b83aeb574b291a7da0，包BUILDINFO中的配方摘要267196b16e3c007ed71f809ca430cd4a0988010e9d05f05d71935a9acda1be8f与官方同版本source-only归档的PKGBUILD相符。GNU 2.47原始源包摘要3068128c75cda9f898ccb4211d360246e8e195ffcc9dfb655b23ae23a54800e8及八个发行补丁均与配方相符；这建立发行者声明的来源关联，未独立重建同字节工具。

x64 COFF外部名字经_bfd_coff_internal_syment_name、coff_link_hash_lookup和bfd_link_hash_lookup进入bfd_hash_lookup；hash遍历完整NUL终止串，以strcmp区分，复制len+1。coff_fix_symbol_name把超过SYMNMLEN的名字交给_bfd_stringtab_add，读端按字符串表offset返回；八字节短槽不等于整体名字上限。发行补丁未改变这些函数；i386pep配置使用pei-x86-64。此链仅支持所读实现不截短至255字符以下，其他工具/参数须重评。

实际选择已由[Windows准备日志](https://github.com/g1331/autsaro/actions/runs/41038949663/job/124006870397)连接：398初次head0928d6ff在10:13:31Z输出gcc的绝对ld查询，resolve为mingw64/x86_64-w64-mingw32/bin/ld.exe，SHA256为e5f877e604846435aedd5b1dedf9b62fbde7cfe24b65625a3d6f0d8f76ff5f80，与上述固定包ld.exe/ld.bfd.exe相同；实际版本GNU ld (GNU Binutils) 2.47.20260726，不简写为稳定版2.47。同作业核验包摘要/签名。名字显著性取上段发行源码的完整字符串/COFF链，不取版本或成功运行；该身份收据不是新head扫描或名字边界实验，不评估CRT/DLL内部。

## 线程TLS的标准分配与释放

R21.3与R22.1均为Required，C99适用（主文印刷167、172／PDF175、180；R22.1另合成AMD2.84）。主文§6.7不要求重审ISO标准库实现内部的全部规则，但接口用途和非ISO调用方的责任仍在。固定glibc 2.39-0ubuntu8.9的pthread_create／allocate_stack通过_dl_allocate_tls进入allocate_dtv，后者在elf/dl-tls.c:370调用calloc；rtld-malloc.h的inline接口经__rtld_calloc连接实际实现。这不是项目C的直接malloc调用，却不能据直接标识符缺席消除非ISO线程／TLS路径的标准分配用途。

非user_stack线程的__nptl_deallocate_stack走queue_stack；缓存回收才经__nptl_free_stacks／_dl_deallocate_tls释放DTV。源码默认阈值40MiB可调，不是运行峰值保证。join和项目执行栈munmap不等于已free全部bootstrap／TLS；当前进程期Task／controller／dispatcher永久停车后由_exit结束，操作系统回收地址空间不等于R22.1的显式释放。因此R21.3的实际用途及这条R22.1释放差距仍需处置；不能由正确的observer wait→close或D4.13顺序结论覆盖。

已有正常ELF没有直接calloc/malloc导入；LD_DEBUG以ecu_probe为reference打印的分配器binding来自lookup_malloc_symbol(main_map,...)，不作为项目直接调用或每次分配事件证明。需区分exec之前的owner Python与之后的实际ELF，源码闭包和真实绑定共同使用；普通／检测器各守自己的身份。原 D4.12 提案及[Linux 两条的补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6401003340)现由 [ADR 0025 的主机动态设施类别](native-facilities/storage-interrupts.md#主机动态设施类别偏离)接受；按其目标、来源和生命周期范围保留真实违反，不外推 Windows provider 内部。给actor静态汽车缓冲、只设bootstrap大小或等进程退出都不能消除这里的calloc／释放义务；真正改为调用方栈并退休所有相关线程还须重证guard、信号、restart及关闭合同。

## 声明宏与表达式参数

R20.7为Required（主文印刷159–160／PDF167–168，AMD2追加C11），约束参数展开形成的表达式；原文明确非表达式用途不要求括号。四份已完成样本各五处报告中，Os.h:102的ALARMCALLBACK(identifier)以identifier构成void函数声明／定义；所选portmacro.h:113–114的portTASK_FUNCTION_PROTO／portTASK_FUNCTION把function及parameters用作函数／形参声明符。它们不是算术或调用实参表达式，不因缺少无关的括号而违反该条。普通函数无法生成这些声明，D4.9的可互换前提也不成立，未援引AMD3.6的_Generic例外。

另外两处为picolibc-freertos.h:66、76的configINIT_TLS_BLOCK；当前实际宏为configUSE_PICOLIBC_TLS=0、configUSE_NEWLIB_REENTRANT=0、configUSE_C_RUNTIME_TLS_SUPPORT=0，该头的入口未选择，不能将未生效的定义计作这些程序的R20.7使用。五处位置／定义在四个工件间身份相同；当前仅局部timing重命名不改变这些头。原始五处报告分别保留，这不是全部宏、所有配置或TLS变体的R20.7通过结论。表达式实参宏仍逐处检查完整展开中的括号与求值次数，不能从声明宏的判断推出通用豁免。

## 非ISO SDK的保留名字

R21.1／R21.2（Required，主文印刷165–167／PDF173–175，含TC1更正及AMD2语言扩展）的SDK责任不能用系统目录或供应方身份销项。pthread.h的_PTHREAD_H、unwind.h的_UNWIND_H及_Unwind_Word等声明、Linux UAPI的_LINUX_FUTEX_H／_LINUX_TYPES_H与__le16等类型，是当前实际包含的具体构造。既有TU宏收据确认这些guard生效；主文§6.7对ISO标准头内容的原有边界保留，不把所有宏／声明都算作项目自有代码，也不把非ISO头整批豁免。

当前材料对应libc6-dev 2.39-0ubuntu8.9、libgcc-13-dev 13.3.0-6ubuntu2~24.04.1和linux-libc-dev 6.8.0-147.147。原SDK提案未自动覆盖这两条；保留现有ABI名字／guard的具体补充仍待人类决定与独立技术复核，不建Accepted ADR、批准表或MR12。删显式头却保留间接路径、手写声明或换名字隐藏同一设施不能作为修复；真正更改供应方头／实现须重证线程、信号帧、unwind及交付身份。该边界不扩ADR0024，也不替Windows实际provider取证。

## 汇编输入参数的实际用途

R2.7为Advisory，当前C99适用（主文印刷44／PDF52；AMD2追加C11）。当前standard-ecu七处报告全在Os_LinuxNative.c：native_call的number、first、second、third、fourth分别进入a、D、S、d、r输入约束，第四参数先移入r10，再执行syscall；Os_LinuxAltStack的stack与previous分别进入D／S约束。它们都实际参与调用，不是未使用参数；不能删除或用void强制丢弃来掩盖检查器未建模的汇编输入。

真实消费者分别为futex等待／唤醒、线程ID以及H先前altstack的查询／恢复和注册栈的安装。已有设施及故障注入证据只覆盖其指定调用和条件。此裁决仅解除这七处“未使用”判断，D4.3的隔离、GNU扩展、寄存器／clobber、原始errno返回、信号ABI与provider语义仍按平台篇独立核查；不以参数进入约束即宣称syscall实现符合。

## C90隐式声明规则与当前编译

R8.1为Required、R17.3为Mandatory，但两条均仅适用C90（主文印刷63、137–138／PDF71、145–146；AMD2的Annex H映射也明确R17.3只针对C90）。所选正常编译为-std=c99，此前standard-ecu的63条R17.3原始报告不能成为C99下的Mandatory违反或偏离申请。fault-publication新standard-ecu又在四个shutdown_reason原子store及一个load处报告五条；它们是GCC内建，同样按所选C99及真实编译接受判断，不能当成漏写普通函数声明。原始摘要和人工字段不修改。

语言适用性不许可隐式声明。实际GCC13正常probe及HostBatch编译／链接使用-Wall、-Wextra、-Werror，当前运行已成功；Det_ValidHooks在Det.c中的调用前已完整定义，Os_BackendBeforeSelect由Os_Backend.h声明，__atomic_*由目标编译器内建契约处理。GNU内建不是漏写普通函数原型的统一理由，SDK自身参数名／restrict、R8.2／R8.4和目标扩展继续独立核查。该编译证据只证明实际TU／参数接受，不能据此声称任意用户代码、Windows当前head或全部声明义务已满足。

## Advisory出口与可见性

R15.4／R15.5的循环与早退判断见[OS控制流](README.md#循环与函数出口)。fault-publication四profile的328／328／345／340处已按实际guard、持续所有权、critical释放、生成初始化退休及采用代码逐位置核对，见[完整作者依据](https://github.com/g1331/autsaro/pull/371#issuecomment-6419297337)。保留真实Advisory违反及理由，不将guard return统称误报；后续0026删除的一处另构成R2.1死分支，旧摘要不改标新字节。

R5.9为Advisory、System范围，TC1的amplification更正未取消跨TU唯一性建议。当前17个报告声明对应六模块configuration、Time与生成配置的alarms、Time／Com的values、HostLinux／Backend的handles、三份control及两份require，均保留真实私有名字重复；合法内部链接不消除该建议。当前按类型、文件与实际调用目标区分的模块词汇是保留理由，不是整条规则通过。

R8.7／R8.9同为Advisory；TC2.15明确区分定义TU与调用TU，AMD4.43–44将后者的defined改为declared。一个外部调用TU不等于只在定义TU使用；未选调用的标准API仍须逐处连接固定SWS。HostLinux的handles只在reserve_handle直接出现，尽管元素地址逃逸供其他函数使用，仍是真实R8.9候选。生成配置对象即使只在初始化函数被命名，静态存储期及后续借用也不能随作用域移动而丢失。当前142／46条报告未批量销项，具体原文、位置与作者判断集中[PR记录](https://github.com/g1331/autsaro/pull/371#issuecomment-6401292791)。

## 字符串终止与复制容量

R21.17／R21.18均为Mandatory（AMD1印刷8–9／PDF16–17；AMD2扩展C11），分别检查无界字符串访问和带size_t参数的实际正值／容量，不能混成“所有string.h调用都已核查”。Os_LinuxMappedStack的line为64字节，used只在小于63时递增；遇换行先写line[used]=NUL再调用mapped_line。后者的cursor只在已识别的分隔符之后前移，strlen之前仍指向该终止数组内部；长度不足四时不读取完整权限前缀。这支持当前这处strlen的终止边界，不是新的字符串实验或任意调用方证明。

Os_TargetPostInput先拒绝零及超过32的length，清零整个32字节记录再复制；Ecu_TargetPostFrame的dlc先限制为1–8；batch_frames只在count非零且已验证不超过256时按完整元素复制。输出的0／1／8字节整改与实际收据沿用既有证据。COM的固定uint32复制仍以真实RTE原生对象和公开对齐／容量前提为边界，native上下文的304／168／512字节证明见原生设施篇。上述逐处依据不能替代采用内核、外部接口调用方或所有配置的完整容量义务。

采用内核另有实际生效的xTaskGetHandle，其strlen(pcNameToQuery)位于INCLUDE_xTaskGetHandle=1的定义中；不能按“功能关闭”排除。当前项目／端口调用闭包没有调用该接口，因此本次程序未从该入口访问字符串，但若交付使用者调用，必须另证pcNameToQuery为有效终止串，configASSERT中的长度检查本身不能防止strlen先越界。统计格式化的strcpy／strlen则在configUSE_STATS_FORMATTING_FUNCTIONS=0的排除分支；头中示例注释不构成实际调用。未把这两种情况合并为库函数缺席。

## switch-clause的终结

R16.1／R16.3均为Required（主文印刷130–132／PDF138–140）。前者的clause语法与后者的正文都要求无条件break；TC1只更正语法字体，AMD2.69扩展语言措辞，没有增加return替代。当前固定addon把return／throw与break同等处理，零报告不能据此证明满足。自有九个switch中，Ecu_StatusName的18个clause、native_task_state的五个clause及Os_BackendState的default原来使用return，现改为局部结果加break并在switch后返回。未知状态文本仍为INTERNAL，Task waiting保护、内核状态查询次数和默认assert／fallback保持。

Os_BackendState的未知内核状态仍返回E_OS_STATE且不写调用方输出；共用critical释放后，只对成功路径执行第二次service guard。真实lifecycle消费者的私有查询包装注入eDeleted，分别观察状态、哨兵输出、一次查询、两次guard和各一次真实critical进入／退出；原有成功／拒绝生命周期及TaskHook场景继续复用。编码属性观察覆盖三个实际函数，恢复旧return须被拒绝；状态名字另按公开枚举逐值和未知码核对。执行结果按PR实际版本记录，不把辅助观察当成provider所有异常语义的证明。

采用内核的两个生效通知switch（xTaskGenericNotify／xTaskGenericNotifyFromISR，configUSE_TASK_NOTIFICATIONS=1）均按eNotifyAction枚举分派，每个非空clause最终break，default在末尾；统计格式化的状态switch由configUSE_STATS_FORMATTING_FUNCTIONS=0排除。crSTART的协程switch模板所在croutine.h不进入当前include闭包，不能把其宏定义当成已启用协程的符合证明。生成目录中的Os_Arti.c是既有自有ARTI实现的交付位置，仍按同一源码判断；实际emitter生成的Ecu_BswMAction也有default提前return，现从ecu_config.rs改为result加break，保留未知channel／mode的E_NOT_OK及三种模式调用。正常生成入口逐profile观察实际clause及共同返回，不能只改样本输出。上述条件判断不外推其他生成输入、SDK或未来配置。

R16.4的default还须在break前有语句或说明注释（主文印刷132／PDF140）。Ecu_StatusName的INTERNAL预置在switch外，单独空break不满足该要求；default现显式赋值INTERNAL，编码属性另拒绝无说明的空default。此赋值位置保持实际映射，不能以名字行为测试通过代替布局核查。

R2.1还须与上述clause终结同时满足（Required，主文印刷39–41／PDF47–49，AMD2追加C11）。native_task_state、Os_ArtiTask／Service的无条件失败断言，以及Os_Time动作分派中的致命关闭，原来使其后的break不可达；当前clause只记录valid为FALSE并break，switch后才执行原断言／关闭。正常事件、服务调用和动作顺序不变，失败仍在后续观察／报告之前停止；断言返回时的SUSPENDED后备保留。Os_ArtiService中StartOS／ShutdownOS的条件断言后仍有正常分支，因此其共同break可达，不能用过宽词法判据一并改写。源属性仅拒绝clause内紧邻break的无条件致命调用；Linux非法Task／service选择另由真实glibc断言和移除断言的副本观察，不扩张为Windows断言provider或整个R2.1完成。

## 回调的 const 建议

实际回调 ABI 与汇编参数的限定裁决见[语法与原始诊断](../bsw-general/language-tokens.md)。
