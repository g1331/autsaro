# C99 类型、常量与语法证据

本篇承接[标准类型](../../types.md)，保留按冻结输入取得的原文与源码证据。

## 整数常量、初始化与浮点构造

R7.5（Mandatory、C99，AMD3.16，印刷9–10／PDF17–18）约束整数常量宏的原始实参；AMD4.55只增加R7.6交叉引用。已核`ab2ea459`双平台八份补丁后工程及`eea9f25d`四份Linux工程：项目实际调用为`UINT32_C`／`UINT64_C`，实参均为单个无后缀十进制或十六进制literal，分别在32／64位无符号范围内；最大者为restart标识`0x4155544f53525431`。实际宏展开追加U／UL／ULL不等于原始实参带后缀。Counter转换宏的输入配置与生成十进制常量另沿生成器范围检查，不能用这十二份样本覆盖所有合法配置。

R7.6（Required、C99，AMD4.4，印刷7–8／PDF14–15）按目标`int`宽度限定small integer宏；AMD4.65属性表的Advisory与正文／AMD4.64不一致，保留正文Required。两目标`int`为32位、`int_least16_t`为16位。默认样本未启用RES_SCHEDULER，不能证明启用路径满足：`ecu.rs::os_configuration`原来的`UINT16_C(1)`已改为`1u`，保留`uint16 task_access`的bit0、资源优先级和指针／计数。实际四输入原生消费者与恢复小整数宏的源码观察分别验证行为和编码约束，不借用ADR例外。

R9.6（Required、C99，AMD4.5，印刷8–9／PDF15–16）由同一initializer中的chained designator触发。上述工程的项目／采用／补丁后初始化器未发现该构造；`.priorities = {[OS_INPUT_INTERRUPT] = 1u}`是两层独立花括号列表，ErrorHook快照也是单个外层designator加内部位置初始化。`sa_sigaction`的SDK宏展开虽有两级成员，实际用途是初始化后的赋值，`handler = {0}`没有chained designator。此结论不是C99语言不适用，也不由checker缺席推导。

D4.15（Required、C99，AMD3.10，印刷5–6／PDF13–14）针对可能生成NaN／Inf的浮点算术或数学函数结果。上述十二份交付C／头文件、宏及补丁后内核中，浮点类型仅见`Platform_Types.h`的float32／float64定义、两个portmacro的portFLOAT／portDOUBLE定义，以及Linux备用栈的`long double`存储／sizeof；没有浮点值运算、浮点literal或数学函数入口。诊断字符串、注释和`#error`文本不是浮点表达式，FP上下文的字节复制也不求浮点值。因此这些具体源码没有待检测的NaN／Inf结果；R14.1（Required，主文印刷117／PDF125，AMD2增加C11）也没有essentially floating循环计数器。两项都仍适用C99，不能把构造缺席写成语言豁免；系统头展开和provider内部不由本段证明。

R8.15／R8.16／R8.17仅适用C11，类别分别为Required／Advisory／Advisory（AMD3.17，印刷10–13／PDF18–21）；本项目C99源码未使用`_Alignas`／`alignas`／`stdalign.h`，三项分别不适用。GNU aligned属性仍按平台扩展和实际布局约束核查。R1.4仅适用C11（AMD2.30，印刷11–12／PDF18–19）；合成AMD3.11–13与AMD4.53–54后不能再沿旧清单批量禁止已另设指导的泛型／noreturn／对齐／线程／原子设施。当前C99不适用本条，Annex K及平台扩展仍按实际接口检查，不能据此升级语言或免除R1.1／R1.3。

## R7.3 的标识符误报

PR #371 的四份 `direct-render-mapping-final-results` 各20个位置为标识符：call四处、cell十四处、all两处，分别属于资源宏、原子／futex及多对象等待；两个程序共40次报告。固定addon未先要求数字token，误认名字末尾的l；仅裁决这些位置。后续[参数名修正](../../bsw-general/language-tokens.md#原始-token-分类的具体误报)及 #420 的声明修正保留既有语义；历史误报裁决仍成立。固定R12.3还漏报过return中的真实逗号运算符，不能从报告减少推定通过。

### R7.3／R12.3：四 profile 的有界人工证据

冻结基线为 #420 合并的 `8bdb1c7f4190764fa27f49a54c5779ccbfe371fb`，范围为 standard-ecu、user-application、component-pipeline、no-diagnostic 的 C99 HostBatch／probe。PDU ID／长度依次为16／16、16／16、8／32、8／8位；末项无应用DID／诊断路由，仍纳入Dcm TU。

持久原件的七份SHA256SUMS均已核：2013主文R7.3为Required（印刷61–62／PDF69–70），对象为数值literal后缀的小写l；R12.3为Advisory（印刷106／PDF114），对象为逗号运算符，包含for初值／增量及嵌套实参中的表达式。AMD2.27仅追加C11适用性，AMD1／3／4与TC1／2未改变这两条的对象、类别或例外；当前仍为C99。

证据复用 Linux decl58 四份完整扫描及 [#420 精确head运行](https://github.com/g1331/autsaro/actions/runs/41549133111) 的 Windows 四份完整分析与输入工件。逐份核对seal、非内核副本、target／程序及当前originalBswSources的C／头／补丁字节，顺序重建内核并逐字节比较。Linux的92项runtimeSources摘要与基线一致；精确身份及核查收据归 #58 本阶段PR。

责任沿[ADR 0025](../../../adr/0025-misra-layered-scope.md)区分：originalBswSources映射自有runtime；其余生成／应用C由实际producer负责；FreeRTOS V11.3.1的`054e14f3397023aa83813a65aa065fc4597d481b`原件与生产patch分开，以scope.patchedLines判断改动行，不把整个kernel算作采用原件。SDK依[原头来源](../../os/sdk.md#sdk-原头来源类别)绑定实际包、头摘要、TU及活动宏；路径或来源分类本身不是满足。

R7.3：两平台四份交付C／头（含未激活分支、补丁后内核）逐token核对，没有小写l数值后缀或追加小写l的token拼接；call／cell／all、注释和字符串不算literal。Linux四份实际GCC展开也无此后缀。Windows每份展开有八处：Backend三处CONTEXT_CONTROL、Time一处WAIT_TIMEOUT、项目patch后的MSVC-MingW port各一处WAIT_TIMEOUT／CONTEXT_CONTROL、HostBatch一处ERROR_BROKEN_PIPE，以及采用queue.c的一处SIZE_MAX。前七处由`__MSABI_LONG(x)=x ## l`产生0x1l／258l／109l；末处沿SIZE_MAX→_UI64_MAX产生0xffffffffffffffffull。源码未手写后缀不代表展开满足。

Windows头包为mingw-w64-x86_64-headers `14.0.0.r569.g34eeafb7d-1`，编译器GCC `16.1.0-5`；活动定义分别见winnt.h:1795、winerror.h:356／248、limits.h:76及_mingw_mac.h。主文§6.7（印刷15–16／PDF23–24）区分ISO标准库实现与调用方义务。SIZE_MAX是ISO C99标准接口，其内部后缀按此边界处理；不扩及非ISO Windows接口。上述七处非ISO展开的R7.3责任仍由 #58 承接，尚未完成满足／合法处置，不新增偏离。

R12.3：Linux四份实际GCC展开的56／56／58／58个TU由Clang18.1.3完整解析，无逗号运算符。Windows的55／55／57／57个TU另按行标记保留项目／采用／patch源码及使用点宏展开，C语法解析无错误；GNU属性另核为段、对齐、调用约定及noreturn／used标记，无逗号运算符。每份仅六处真实逗号表达式，均是生成Os_Arti.c的assert在标准assert.h:46展开的`(_assert(...),0)`，按§6.7归标准库内部语法；条件仍为计数非零或0。未发现该活动语法中的其他逗号运算符。声明成员、函数调用、数组初始化和宏参数均按实际语法区分，不从checker零报告推定。

消费者为资源宏、原子／等待、栈／故障、资源计数及native_starts；采用tasks／list／queue、所选port与生成配置／RTE／ARTI进入两个程序。对照识别小写后缀、return／for逗号运算符，排除大写后缀与分隔符。

Windows完整SDK内部、未展开宏的其他实例及供应方实现仍未评估。Clang的GCC16头错误不作阴性证据。新增源码／宏用途、配置、补丁、provider或target时重评。#381／#382的诊断分类及#385的66项记录不重做；155项checker-covered与raw221项仍not_assessed，passed=false／error、completeCompliance=false、retention键／形状及strict refusal不由本文改写，不声称总体符合性。


### 当前输入与三项人工证据的接续

#443 从 master `8eee27f7` 接续上述证据。完整分析复用
[#448 最终 head d46b89db 的正常运行](https://github.com/g1331/autsaro/actions/runs/42391343618)：
双目标、四 profile、HostBatch／probe，Linux 228 个 TU，Windows 224 个 TU。
八份分析均核对非内核 seal、最终补丁内核身份、完整程序和 TU 目录；
raw `passed=false`、`error=null`、两个程序退出码均为 1。
四 profile 的独立应用、PDU 宽度及无诊断配置继续按各自输入识别。

#453 阶段的 92 项 producer 中，runtime／patch 与扫描一致；composition.rs、
contracts.rs、rte.rs 已变化，涉及类型化 C/S 与 provide-port 常量。四个当前 Linux
样本各有 226／226／236／233 份 C／头，差异仅为 Rte_<SWC>.h 契约注释，移除注释
后 token 逐文件相同。具体比较见 #453；旧整体语义身份及其他生成配置各保留原范围。

原文重新核对 R7.2、R7.3、R12.3 及 AMD1–4／TC1–2：
R7.2 的主文类型表原页与 AMD2.46 合成后，仍分别判断预处理条件的
`intmax_t`／`uintmax_t` 模型和普通 C99 表达式类型；
R7.3 判断数值后缀；R12.3 判断真实逗号运算符。
类别及页码沿上文和[普通整数证据](../essential-types/ordinary-integers.md)。

#453 对 452 个 `compiler-origins-N` 输出按行标记核对自有、生成／应用及项目 patch，
排除指令、字符串与字符，完整识别整数和浮点。Windows 四 profile 共 28 次非 ISO
小写后缀来自 CONTEXT_CONTROL、WAIT_TIMEOUT、ERROR_BROKEN_PIPE；Backend
位置为 1158／1583／1597，采用 queue 的 SIZE_MAX 沿 §6.7。物理源码无新增手写
小写后缀或拼接生产者。该阶段 SDK 实例归 #444，当前决定见 [SDK 类别](../../os/sdk.md)。
R7.2 的 44 次缺 U 与条件证据另见[普通整数](../essential-types/ordinary-integers.md)
及[条件控制](../essential-types/conditional-integers.md)。

固定工具的真实最小反例从正常 `c-check` 入口执行：
`#if 0xffffffffffffffff > 0` 中的常量在所选 64 位预处理模型下缺 U，
该程序仍得到 raw `passed=true`；普通表达式中的 `0xffffffff` 会报告 R7.2。
`return (f(), (int)1l);` 会报告 R7.3，而其中真实逗号运算符没有 R12.3 报告；
改成大写后缀和无逗号的控制输入，两条目标报告消失。
这些输入均编译成功、分析 `error=null`；其他规则的报告各自保留。
反例证明这三项仍需独立证据，完整 fixture 和原始结果归实施 PR。

本批 Linux 228 个实际预处理 TU 另由 Clang 18.1.3 完整解析，逗号运算符为零。
解析前只移除 `c_check` 在原 TU 后追加的 `AUTSAR_VALUE_` 宏值探针行，
原 TU 的展开保持；遇到解析错误即终止该份核查。
Windows 当前完整 SDK 的 Clang 解析限制继续保留，R12.3 的已有有界语法证据沿上文原输入。

### 正常分析与其余人工义务

固定工具的 155 项入口分为 128 项 addon、25 项核心及 2 项共有，编号见 [PR #453 分区](https://github.com/g1331/autsaro/pull/453)。raw token、指令、
语义／值流与 CTU 分别证明相应性质；CTU 按两个程序各自保留完整 TU。
固定 11 项沿 types.md 原组，66 项沿原人工篇和 #445。
其余 144 项由 `c_standard.checker_obligations` 独立选择，
沿 `c_acceptance.evaluate` 的 `checker-obligations` 组消费。
缺行逐项返回 `checker-obligation_not_assessed`，责任为 #443；
已有行回接实际文件和行号。零诊断仍产生这些阻塞对象。

### 三项局部证据的当前消费

下表引用本篇及整数专题的冻结证据，状态反映尚缺的当前义务。
原文基线为主文 R7.2（印刷 60–61／PDF 68–69，含类型表）、R7.3（61–62／69–70）、
R12.3（106／114），合成 AMD2.27／46；七份原件摘要核验及原页核查见实施 PR。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R1.4 | Required | 双目标四 profile 的 C99 HostBatch／probe | not_applicable | AMD2.30（印刷 11／PDF 18）仅适用 C11；AMD3.11–13、AMD4.53–54 修改禁用设施，语言范围保持。当前 TU 均以 -std=c99 编译；扩展与 Annex K 义务沿各自规则核查。 |
| R7.2 | Required | 双目标四 profile 的 HostBatch／probe；自有、生成／应用及项目补丁的条件与普通整数 | not_assessed | 当前普通表达式复用 master run 43418599046 的 452 份实际 GCC 展开与原 SDK 精确类别；#470 新增 SIG_BLOCK／SIG_SETMASK 为 signed 0／2，Windows 原有每 profile 11 处缺 U 沿 sdk-integration。主文 60–61／AMD2.46 区分普通 LP64／LLP64 与预处理 intmax_t／uintmax_t；#443 恢复须在当前 include／define／undef 时序捕获 #if／#elif 实际求值式，独立控制 0xffffffff 的 signed 预处理类型和 0xffffffffffffffff 的 unsigned 缺 U，并验证捕获前后选中输出相同。普通 -E -dD 已消去条件；历史捕获及 R7.1 词法闭包各保留原证明范围。 |
| R7.3 | Required | 同一范围的物理 token、真实宏展开及直接消费者 | satisfied | 主文 61–62、AMD2.27 的对象为数值后缀，包含整数与浮点。当前所有交付 C／头及最终 kernel 项目行无手写小写 l 后缀或拼接生产者；452 份实际展开无额外实例。Windows 每 profile 七处 __MSABI_LONG 展开的 0x1l／258l／109l 均回接 [已批准 SDK 类别](../../os/sdk.md#当前集成义务与类别处置)及其精确实例：Backend／port 上下文、Time／port 等待及 HostBatch 断管消费者；沿原 sdk-integration 组消费。采用 queue 的 SIZE_MAX 沿原 ISO 接口责任，原违反与正式义务由各自组保持。 |
| R12.3 | Advisory | 同一范围的真实逗号语法，含 return、for 和嵌套实参 | not_assessed | 本篇已保留 Linux 228 个历史 TU 的完整语法解析及 Windows 历史有界证据；#443 补当前双目标语法与直接消费者关系，Windows SDK 解析限制须有实际替代证据。 |
| R17.3 | Mandatory | 同一 C99 范围，含 GNU 对齐查询与项目补丁 | not_applicable | 主文印刷 138／PDF 146 仅适用 C90，AMD2 Appendix H 继续明确此限制；当前 C99 编译、声明与扩展检查仍保留，具体诊断沿原 OS 裁决。 |
<!-- misra-checker-obligations-end -->

从 #453 到 master `a5f1dd35`，#457 改变 SchM 模式绑定生成物，#455 改变分析驱动。
该阶段 master 完整扫描为 [42424904238](https://github.com/g1331/autsaro/actions/runs/42424904238)。
旧局部证据的全文和正常工具证据可定位，但上述变化仍须按实际实体与展开建立当前关系。
该阶段本组保持未绑定；当前重评与绑定见下，三项局部证据继续逐项阻塞。

其余 139 项已按规则族登记能力边界与恢复条件：
[词法与名称](lexical.md)、[声明初始化](declarations.md)、[类型表达式](expressions.md)、
[控制流与生命周期](control-flow.md)、[预处理与库](preprocessing-library.md)。
该阶段结算 R8.1 的 C99 适用性和 R15.5 的既有 GRP，留下 137 项新增记录及三项局部记录。
恢复条件为将已登记的同号固定 checker 或核心诊断映射与全部适用修订及当前源码核对，
把正常分析足以证明的性质、语言／配置适用性或额外人工证据写回对应原主题篇，
再以独立本组上下文绑定当前输入；缺少结算的行保持未评估。
固定 11 项、原 66 项、具体告警和 SDK 各沿原有责任消费。

本阶段同步后的[当前输入重评与绑定](declarations.md#当前输入重评)覆盖双目标四 profile。


## 函数指针的显式丢弃

R11.1为Required、当前C99适用（2013主文印刷93–94／PDF101–102）。Exception 2明确允许函数指针转换成void，不能把void与void指针混同。AMD2追加C11，AMD2.58修改Exception 3的隐式转换说明，没有撤销Exception 2；其余相关修订未增加这里的限制。

四份 `direct-render-mapping-final-results` 各有一处R11.1，均为 `generated/src/Ecu_Execution.c:7:5` 的 `(void)sink`，归属host-batch／probe。实际 `Can_TxSinkType` 为返回EcuStatus、接受id／dlc／八字节const数据指针的函数指针；目标类型确为void，没有星号或转成另一函数原型。因此该具体报告适用Exception 2，是误报。这里不调用sink、不保存变换后的地址，实际ECU发送继续进入Ecu_TargetEnqueueTransmit；不由这项判断证明其他callback、参数用途或地址表示符合。

ECU producer从 `runtime/ecu/src/Ecu_Execution.c/.h` 的保留模板交付这些文件，四份分析副本及Can_CfgTypes.h均与实际模板／头字节相等；不假称Emitter输出具有直接原件映射。固定addon的misra_11_1在目标不再是函数指针时报告，该分支未先排除void，解释了这条报告。未改检查器、源码、suppression或baseline；原raw、strict refusal及221项not_assessed保留，不扩为整条R11.1或任意指针转换通过。


## 原生ABI类型与编译约束

Os_HostTypes.h集中原有atomic／handle／thread callback别名，Os_PortTypes.h集中原有PC与向量回调别名；类型头不承载对象／函数映射。LONG在Windows仍为32位long、Linux仍为int32_t；64位存储、HANDLE／DWORD／LPTHREAD_START_ROUTINE、Windows void*／Linux uintptr_t PC以及uint32(void)向量回调均保留。Schedule的私有别名直接使用原有Os_ArtiScheduleStateType，不改tables布局或工具view。

原四个Os_Atomic guard及ArtiAtomic32IsFourBytes／Os_HostLongMustBe32BitsType的char-array typedef名字没有消费者，约束作用却必须保留。现在匿名枚举的显式值使用sizeof(char[原谓词?1:-1])：受支持目标的界为常量1，失败目标因负数组界在编译期拒绝；不生成状态对象、VLA或运行时分支，Linux always-lock-free拒绝仍独立存在。六个宽度／对齐谓词未放宽，旧实现guard类型名的源码移除已记CHANGELOG。基础存储／回调别名及服务二进制ABI不变。

正常编译控制分别改变32／64位宽度与对齐，拒绝须定位到对应guard表达式；将同一guard的检查表达式替为常量1的私有副本必须编译成功，避免把其他检查的失败算作目标证据。sizeof中的Os_Atomic64Type仍是真实类型用途，不能因旧addon遗漏而一同删除。上述只处置类型归属和编译约束，GNU扩展、SDK、provider及实际Windows执行继续单列。

`48907b3b`的type-owner标准ECU完成结果另有三条新增R17.3，位于交付Os_HostTypes.h:23的两次及:30的一次`__alignof__`。同一批原R2.3由60减至56、R5.9由17减至0；R17.3则由71增至74，raw合计2862，不把净减少当作没有新报告。原比较工件为os-aliased-service，具体位置仍按各自字节身份保留。

主文R17.3（Mandatory，印刷138／PDF146）仅适用C90，当前实际TU为C99；三处是编译器对类型的对齐查询，不是遗漏函数原型。这三个GNU运算在旧guard中已经存在，改换常量表达式上下文没有新增一次运行时调用。固定addon的misra_17_3按未关联function／valueType的名字加左括号报告，没有完整建模该扩展。实际C99编译及对应约束拒绝另作证据；不手写内建声明、不新增baseline／suppression，原报告、语言适用性、GNU扩展责任和严格比较拒绝分别保留。

维护者接受的平台／OS宏形式见[精确预处理记录](../retained-platform.md)，不扩大所选配置的运行能力。



### 分批结算

词法批次见 [PR #476](https://github.com/g1331/autsaro/pull/476) 与
[lexical.md](lexical.md#当前输入与核查方法)；声明批次结算 R7.1，表达式批次结算
R13.1／R13.3／R13.6。[控制流批次](control-flow.md)再结算 8 项；完整 144 项为
28 satisfied、3 not_applicable、113 not_assessed。当前绑定见[声明篇](declarations.md#当前输入重评)。
