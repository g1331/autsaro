# SDK 原头来源与接口责任

本篇从[声明与SDK接口](declarations.md#sdk-声明与常量)分出，记录实际采用来源、宏归属及原有具体调查。


## SDK 原头来源类别

按 [ADR 0025](../../adr/0025-misra-layered-scope.md) 的工具链／SDK 层，原头作为采用来源按 `target + provider 包名/版本 + 包内头路径 + 实际 SHA-256` 唯一分类；不逐条把 SDK 规则记为已满足。范围包括 Linux 的 libc6-dev/glibc、libgcc 开发头和实际包含的 Linux UAPI，以及 Windows 当前目标实际选择的 MinGW-w64 Windows API／CRT 头。项目源码、FreeRTOS 顺序 patch 和自有适配仍按自己的责任处理；同名文件或系统 include 路径不能产生 SDK 身份。

现有 `c-check` 先核实 sealed target/ABI，再以同一 GCC、flags 和 include 顺序对每个真实 TU 编译、取宏值并运行带行标记的 `-E -dD`，同次 `-MD` 依赖输出确认真正打开的文件，避免把行标记或搜索目录本身当成原头消费证据。`scope.sdkTranslationUnits` 记录真实包含闭包，`sdkHeaders` 记录路径、包版本、包内名字、原件校验与内容摘要，`sdkMacroInputs` 记录项目报告行当时生效的定义位置和值。Linux 对已安装 dpkg 的受支持开发包原件清单校验，Windows 对所选 compiler 安装的 MSYS2 pacman `desc/mtree` 校验；这是安装来源／字节证据，不是重新认证软件包发行签名或供应方符合性。最终运行库不等于开发头包：历史真实 libc/libgcc 绑定见[实际 provider](native-facilities/README.md#实际provider及异步调用)，不能用 GCC 版本替代库版本，亦不据 MinGW 名称猜测 UCRT 或专有 Windows SDK。

`diagnostics.sourceOrigins` 只附加来源，保留原始 id、severity、message、locations、程序出现次数与 `patched/adopted`。`macro-input` 说明该行使用了 SDK 定义，`macro-reference` 只表示未展开的宏替换列表引用，二者均不单凭同一行就判定全部违反来自 SDK；R7.2 另连接固定 addon 实际观察的 unsigned 无 U 操作数与原宏中的整数字面量（包括 SDK cast 壳及异常常量别名链），只在实际活动对象宏和值无歧义时标为 `unsigned-literal`／`sourceCategory=sdk`。例如 Windows `kernel/portable/MSVC-MingW/port.c:54` 的等待断言使用 SDK `INFINITE=0xffffffff`：保留真实 Required R7.2 和项目 patch 标记；DWORD 实参、返回值及失败行为继续由[内核采用与 patch](adoption.md)及 #383 的真实双平台消费者负责。

编译器预定义不冒充 SDK 原头：仅当活动定义来自 `<built-in>` 且名字、值和函数宏形式与该目标实际编译器的完整预定义收据一致时，记录 `compiler-predefine` 及独立 `compilerKey`；`compilerPredefineDiagnosticKeys` 单列这些来源键。项目重定义、同名前缀或值不匹配不取得该身份，调用方责任及原始规则状态不变。

包外头、未知 provider、错误目标／包关系、缺原件清单或自有修改的字节均为 `unclassified` 并保留原因。项目 `#line` 可改变报告路径，此时 `macroTraceComplete=false`，物理宏归属明确回退为未解析；不借行标记冒充 SDK 原件。活动定义按使用时刻追到传递替换依赖；其他项目宏经别名产生同值字面量、依赖缺失或循环时，归属必须回退，同一行的数值匹配不足以转移责任。跨行实参不能只凭首行确定来源，宏替换列表的引用也不用作已展开操作数的证明。缺失／多义宏操作数证据保留 `macro-input` 和未解析计数，不按文件前缀补造归属；函数宏、复杂宏链和其他规则的因果裁决不由对象常量匹配推定。`sourceClassification` 分开给出 SDK 来源键、未分类键、未分类原头及未解析宏诊断，任何一项缺口不能记为完整覆盖。当前受支持双平台四 profile 的完整来源要求由既有 CI 汇总检查；零未分类只表示这些实际输入已归属，不表示对所有 SDK 声明做过完整 MISRA 分析。Cppcheck 仍消费既有编译器宏模型，真实 GCC 展开不是另一轮完整 SDK MISRA 扫描。

风险控制为同一实际 TU 编译、原件摘要匹配、宏生效位置追踪、正常 c-check 回归和同名自有头／未知 provider／错误目标／原件修改／项目重定义反例。原头和 provider 内部实现不修改、不逐项审计，不写替代声明。目标、provider、头字节、编译选项或采用边界改变时重新分类；严格比较的 `semanticIdentity.version=2` 加入 SDK 身份及分析代码，旧工件和真实身份不符仍拒绝，不重写旧分析。原始退出、`passed=false`、`completeCompliance=false`、`not_assessed` 与 refusal 全部保留；来源类别不构成规则满足、正式偏离、供应方或项目符合性。

以下为 #371 的历史具体规则调查，按其原头／配置身份保留；其“待决定”表示当时状态，后续来源责任沿本节和 ADR 0025，不继续申请逐条 SDK 保留，也不把历史原始违反改为通过。

## 历史具体规则调查


当时SDK仍有独立的R8.2缺口：项目私有Os_HostLinux.h直接包含pthread.h，其pthread_create声明的start_routine内层void指针参数未命名；Os_PortRestart.c直接包含GCC13的unwind.h，_Unwind_Trace_Fn、_Unwind_Personality_Fn及_Unwind_Backtrace声明也保留无名参数。主文的函数指针示例明确要求命名；TC2.10只增加交叉引用，TC2.12只澄清R8.3不比较无名参数，没有取消R8.2义务。AMD2追加C11，AMD3.63补充引用，均不提供当时例外。

主文§6.7对ISO标准头内容及处理标准头时包含的文件另有边界；这里两条直接入口分别是POSIX和GNU接口，不能把整个pthread／unwind SDK归入ISO标准库豁免。删除显式包含而保留间接链、手写替代声明或修改本机SDK，均不能证明交付闭包已整改。保留对应SDK声明需新的有界项目取舍或供应方合规材料；当时只记录真实缺口，不套ADR0024、不写MR12，也不将该取舍等同MISRA合法偏离。此处列的是已核具体声明，不是整个SDK的完整违规清单。

R8.14同为Required（主文印刷75／PDF83；AMD2追加C11）。同一pthread.h的pthread_create、pthread_attr_getstack等声明使用__restrict限定参数；这是当时GNU C99下的restrict类型限定，不因拼写有下划线而消失。标准库memcpy的原文示例不提供整个POSIX头豁免。这是独立于R8.2的已确认声明缺口，既有两头文件命名取舍提案尚未覆盖此规则；不扩张原提案、不擅自移除SDK限定或填写批准记录，其他实际SDK构造仍须继续核查。

R7.1为Required（主文印刷59–60／PDF67–68；单个0例外，AMD2.27追加C11）。Os_HostLinux.c经sys/eventfd.h实际包含bits/eventfd.h；其EFD_SEMAPHORE=00000001、EFD_CLOEXEC=02000000、EFD_NONBLOCK=00004000使用八进制，后两项直接用于事件创建。前者虽未选择，枚举声明仍进入TU。stdint.h处理先结束，再包含这些枚举，不能据主文§6.7把整个POSIX头豁免。当时libc6-dev为2.39-0ubuntu8.9，两个已核pthread／eventfd头与该固定源码字节一致；精确身份、替代和[有界SDK取舍补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6397172894)留PR。当时未接受，不填批准表／MR12，不套ADR0023／0024，也不据此声称其余SDK已评估。

R8.10也是独立Required义务（主文印刷71／PDF79，AMD2追加C11）：当时pthread.h:624–630的__pthread_cleanup_routine和:1339–1343的pthread_equal均为生效的extern inline定义。前者由GNU与__EXCEPTIONS选择，后者由__USE_EXTERN_INLINES选择；正常GCC13.3 C99／O1／fexceptions宏收据确认两条条件。没有项目调用不消除已生效定义的义务，自有六个Interlocked辅助函数则均为static inline。固定libc6-dev 2.39-0ubuntu8.9头的身份、包含链及替代排除见[具体SDK补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6402928996)。仅关闭优化不能移除前者，改异常／inline语义会影响既有unwind及供应方接口；当时保留提案仍待决定，不扩ADR0024、不填批准表或MR12。

按四个真实TU原参数预处理后，还确认bits/poll2.h的poll、bits/unistd.h的read／getcwd／confstr／getgroups／ttyname_r／getlogin_r，以及bits/fcntl2.h的open为当时fortify选择下的外部inline，共八个新增R8.10位置；不是前述两个pthread函数提案的自动扩张。按实际包含栈，ISO标准头及处理它们时包含文件的实现内联仍守主文§6.7边界。原_FORTIFY_SOURCE=3、GNU inline及异常选择保持，不通过关闭硬化使这些定义消失。

R17.1同为Required、C99适用：AMD3.65–66禁止stdarg.h包含与其设施，TC2.8的变参声明示例直接标记不满足该规则。当时非ISO SDK有fcntl、open、makecontext、execle／execl／execlp、__open_alias的八个变参声明／定义位置（open在公开声明与fortify定义各一处）；生产只在Os_LinuxMappedStack中以固定O_RDONLY读取/proc/self/maps，不将其他声明记为执行。具体头字节、八个新增inline函数、变参位置、真实包含栈及替代排除见[合并SDK补充](https://github.com/g1331/autsaro/pull/371#issuecomment-6403690524)。保留提案仍待决定，现成TU预处理不是新head完整分析／provider语义或合法偏离证明。
