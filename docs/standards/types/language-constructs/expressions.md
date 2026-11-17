# 类型、转换与表达式的检查边界

输入与固定工具沿[语言与词法](lexical.md)。本篇核对双目标四 profile 的 C99
HostBatch／probe；自有 runtime、生成／应用及项目 patch 按实际来源分别检查。
原 11 项中的 R10.1／R10.3／R10.6／R13.5 继续沿原补充组，R7.2／R12.3
沿[局部记录](README.md#三项局部证据的当前消费)。未完成项的恢复责任为 #443。

## 有效正文与取证范围

七份原件通过原 SHA256SUMS。主文 R10.2–R13.6 为印刷 85–115／PDF 93–123；
类型类别与运算结果另按 §8.10、附录 D.6／D.7。已查看 R10.5、R12.1 的原页表，
并合成 AMD3.33 替换后的转换矩阵。AMD2.27 追加 C11 适用性，当前仍为 C99；
AMD2.51–65 分别更新 essential type、转换、优先级与副作用说明。
TC1 为复合运算补一元取反、正负号及非复合结果边界，为 R11.2 补无限定类型，
为 R11.9 补 {0} 例外，为 R12.4 明确常量表达式的语义。
TC2.16 明确 R10 对指针的范围，TC2.19 限定字符算术另一操作数的 rank，
TC2.22 修正 void 指针与整数转换的实现定义措辞，TC2.23–26 补完整表达式与
R13.6 的 Required 类别，TC2.46 重写 D.7.9。
AMD3.19–35 扩充实／复浮点类别、转换矩阵及例外；AMD4.7–14 将 R11.3／R11.8
扩至 conversion 并处理 _Atomic，AMD4.17–23 为 R13.2 增线程交错与 atomic 访问。
AMD1 和其他修订没有改变本表相关 C99 判据。

同号入口指固定 e73bf44 的 `misra.py`；R13.2 另沿核心 unknownEvaluationOrder。
类型未知时跳过、函数模型及有限值流的能力边界逐行记录。既有
[essential type](../essential-types/expressions.md)与
[主机地址](../../os/native-facilities/addresses.md)只复用各自登记的表达式和来源；
Boolean 的 TRUE／FALSE 映射与 character 条件式按 D.6／D.7 判断，当前工具
数值化宏导致的报告仍保留。这里的满足结论依据实际构造和直接消费者。

## 第七批当前输入与类型推导

第七批 #513 从 master `d958d57e` 重取 20 条未评估项。基线为最近成功 master 必需
[run 43978366447](https://github.com/g1331/autsaro/actions/runs/43978366447)
（实际 checkout `cf8ddc4f`）的双目标四 profile：16 个程序共 888 个 C99 TU，
452 份 GCC origins 含补充单元。八份 seal、最终 kernel、92 项 producer、原身份和
原 evaluate 按 LF／CRLF 及各自解析器核对；原 summary 保持只读。

物理扫描逐字符保留续行到物理行的映射，去注释保留其中全部换行；每条记录均回查
原文件对应 token，再选择 patchedLines。GCC 展开保留全部项目文件（含采用原行和
范围外声明）以保持语法，仅按使用点筛选范围内节点；SDK 展开保留原始回执另核。
GNU attribute／declspec 的语法移除保持位置及换行，所有范围内语法错误须为零。
转换同时枚举 `(type)operand` 和 `(type)(expression)`，后者与括号函数设计符的
语法歧义逐项核对；赋值和逗号用语法节点区分初始化器、实参与运算符。
组合反例含注释伪 token、注释／宏续行、嵌套 cast、return／for 逗号与复合赋值结果。

附录 D.6 按标准 literal 类型取得 STLR／UTLR，命名 enum 每种独立；TRUE／FALSE
沿项目 Boolean 定义，单字符常量为 character。D.7 的比较／逻辑结果为 Boolean，
条件式先比较两分支的本质类型；TC2.46 的 D.7.9 决定字符加减及非恒定整数运算结果。
C 的整数提升与这些本质类型分别推导；指针表达式按 TC2.16 进入 R11。

- character 实体只见 role／origin／failed_role、文字指针／storage 和布局断言。
  role 用于比较、传参与赋值，文字缓冲交 Os_HostTextAppend／Field／Byte；未选的
  read_resource_failure 只比较 *failure、*end。范围内没有 character 加减；旧
  HostBatch::digit／map_address 的主机实现沿 ADR 0030，uint8 为 unsigned。
- 算术 cast 的目标只有 signed／unsigned、boolean 和 char，float32／float64 仅基础
  typedef。CanTp 的 `(boolean)TRUE` 是同类 Boolean；isOsStarted 先以
  `result ? 1u : 0u` 生成 unsigned，再交 ARTI，未把 Boolean cast 为数字。
  Backend 的 `'T' : ('B' : 'I')` 按 D.6／D.7 为 character，外层 char 保持同类；
  两次 `(uint8)'R'／'I'` 按 AMD3.33 允许 character→unsigned。
- CanTp_DataLength 原 `(uint16)(data[0] & 0x0fu)` 的两个操作数本质均为 unsigned
  char，非恒定 & 的结果为 8 位复合值，扩成 uint16 是真实 R10.8 违反，固定工具
  漏报。当前改成 `(uint16)data[0] & 0x0fu`：运算前扩展，结果为 uint16。
  后续 low*256u+data[1] 的最高 rank 为 uint16，最外层 uint16 保持同宽。
- 其他复合 cast：Echo／Ingress 的 uint32 移位及 committed+1u→uint8，Com 的
  uint32 右移／&→uint8；CanTp 的 uint8 序号／队列取模→uint8、uint16 长度
  /256u 或 %256u→uint8；Dcm 的 uint16 DID／时限分拆→uint8，uint32 缓冲
  差值／resDataLen+1u→PduLengthType（各 profile 为 8／16／32 位）。均保持
  unsigned 类别且目标同宽或更窄。Dcm 的 0x80u 依 D.6 为 unsigned char，动态
  右移依 D.7 保持该类型，cast uint8 同宽。Backend／Mailbox 三处
  `(LONG)~(uint32)1u` 是常量表达式，依 TC1 §8.10.3 排除复合表达式；原报告保留。
- void* cast 只有 NULL／NULL_PTR 的零形式和对象地址→void*；对象指针→uint64
  位于 Os_ArtiAlarmBaseReturn，随后经 Arti_CaptureAddress 保存完整地址、token
  交 Arti_Record，R11.4 的真实违反转 #544。未发现 void* 与非零算术值的 cast，
  也没有对象指针与 Boolean、character、enum 或实／复浮点的 cast；多层指针逐层
  区分，字节指针转换和地址观察分别由 R11.3／R11.4 承担。
- 全部简单／复合赋值结果只作表达式语句或 for 子句。tasks.c:1902／2957 的
  listSET_LIST_ITEM_VALUE 展开虽然有外层括号，最终仍为独立语句，没有结果消费者。
  真实逗号仅为 Windows Os_Arti.c 六处标准 assert 展开的 `(_assert(...),0)`；
  按主文 §6.7 识别标准库内部，调用方条件是深度非零或常量零，R12.3 见索引篇。

旧 R13.1／R13.3／R13.6 的列表、独立增减与 sizeof 证据保留原具体消费者：
CanTp PduInfo、Os_ArtiCallerSave、Os_Error sources、Time observed／CounterWork，
以及 Os_Schedule 的 ++transitions；本次 CanTp 改动未增加这些构造。
七条结算（含索引篇 R12.3），其余十三条转 [#544](https://github.com/g1331/autsaro/issues/544)
并保持 not_assessed；本次新增例外批准为零。修复后的实际输入和当前正文按声明篇
绑定，旧 CanTp 字节的基线只用于比较。原始诊断、passed／error、程序退出码、
221 项 assessment 与 completeCompliance 保持原义，完整组为 78／3／63。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R10.2 | Required | character 加减与结果消费者 | satisfied | 主文 85＋TC2.19；附录 D.6／D.7、TC2.46。完整 character 声明、literal、条件结果和使用点均已定位：role／origin／failed_role 仅比较、传参、赋值，缓冲由 Os_HostText 接口消费；read_resource_failure 未选分支也只有字符比较。范围内无 character 加减；uint8 是 unsigned，HostBatch::digit／map_address 主机实现按 ADR 0030。 |
| R10.4 | Required | usual arithmetic conversion 的全部操作数 | not_assessed | 主文 87–88＋AMD2.52／AMD3.31–32；字符加减例外和实／复浮点例外分别判断，没有 signed 常量通用例外。同号入口跳过缺类型 AST，嵌套 Boolean／character 结果有既有误报。CanTp 状态、Os_TargetWaitTick waiting、complete_activation 与角色字符条件式只复用原篇精确裁决。#544 承接，继续归 #443；恢复须逐其余算术／比较／复合赋值和 ?: 对齐命名 enum、Boolean、character 与实际类别，补完整结果消费者。 |
| R10.5 | Advisory | 显式 essential category 转换 | satisfied | 主文 89–90＋TC1、AMD2.53–55、AMD3.33 有效矩阵。全部算术 cast 无 enum／浮点目标；signed／unsigned 互转和 enum→uint32 可允许，(boolean)TRUE 保持 Boolean；isOsStarted 用 result ? 1u : 0u 生成数字。Backend 角色条件式按 D.6／D.7 保持 character，(uint8)字符 literal 属允许的 character→unsigned；未选故障分支数值转换同样核对。 |
| R10.7 | Required | 复合表达式参与 usual arithmetic conversion | not_assessed | 主文 90–92、TC1 §8.10.3＋AMD3.34；常量表达式排除，取反与非恒定一元正负号按修订处理。同号入口偏重算术 AST／部分赋值父节点。HostBatch::number 在计算前扩展 next，map_address 以 uintptr_t 累积，Com 在移位前转 uint32，Target 先转 size_t 再乘。#544 承接，继续归 #443；仍须逐当前移位／按位／条件和复合赋值列出两侧 essential 宽度，补全表外运算与直接消费者。 |
| R10.8 | Required | 复合结果的显式 cast | satisfied | 主文 92＋TC1 复合边界、AMD3.35。CanTp_DataLength 原 uint8 & UTLR(0x0fu) 的结果为 unsigned char，cast uint16 真实违反已改为 (uint16)data[0] & 0x0fu，运算前扩展；low*256u+data[1] 为 uint16。其余 Echo／Ingress、Com、CanTp、Dcm 的非恒定复合 cast 均 unsigned→同宽或更窄 unsigned，PduLengthType 8／16／32 位分别核对；(LONG)~(uint32)1u 的三处原报告为常量表达式，保留误报裁决。 |
| R11.1 | Required | 函数指针的显式与隐式转换 | not_assessed | 主文 93–95 三项例外，AMD2.58 仅更新函数设计符说明。固定函数类型 helper 仍需完整原型核对；Ecu_Execution.c 的 (void)sink 沿原丢弃例外，Det_Callouts、RTE Runnable、OS actor／sink 回调可定位。#544 承接，继续归 #443；恢复须沿每个赋值／参数／返回和间接调用确认兼容原型／调用约定，区分 NULL 常量、函数到自身指针及 void 丢弃。 |
| R11.2 | Required | 不完整类型指针的转换 | not_assessed | 主文 95＋TC1 无限定类型，例外允许 NULL、同一不完整类型及 void 丢弃；void 指针沿其他规则。固定入口只收 struct 前置拼写，不能覆盖全部 typedef／union 或完整化时点。当前 Os_HostHandleType→HANDLE→Os_HostLinux 定义及 Det／RTE 回调类型可追溯；#544 承接，继续归 #443；仍须按每个转换处的实际可见类型核对完整化、隐式参数／返回和直接消费者。 |
| R11.3 | Required | 不同对象类型指针的 conversion | not_assessed | 主文 96–97、AMD2.59／TC2.21＋AMD4.7–9；有效义务覆盖隐式 conversion，character 访问例外限非 atomic 源对象。同号入口只查 cast，void、部分多层类型会跳过。Host native 上下文／字节复制及 Can PduInfo 数据指针须与实际目标类型区分。#544 承接，继续归 #443；恢复须为全部显式／隐式转换追到解引用／复制方，核对逐层无限定类型、字符访问例外、对齐和别名；仅增加 const 的同类型转换另按原文判断。 |
| R11.4 | Advisory | 对象指针与整数之间的转换 | not_assessed | 主文 97–98＋TC1 限对象指针／AMD2.60；uintptr_t 可表示性未提供通用例外。同号入口只查显式 cast。Os_HostStackRange、Os_StackRecordBuffer 与 VirtualQuery 的旧主机实例已转历史；当前 Os_ArtiAlarmBaseReturn 的 (uint64)address 仍为真实 Advisory 违反，capture 与 Arti_Record 消费链保留，未批准新例外。#544 承接，继续归 #443；恢复须收齐隐式及复杂表达式，核对 LP64／LLP64、NULL 例外、表外地址及实际消费者；原八实例记录保持精确范围。 |
| R11.5 | Advisory | void * 到对象指针的显式／隐式转换 | not_assessed | 主文 98 的例外仅为 void * 空指针；malloc 等真实对象恢复仍须结算。同号入口跳过直接分配函数形态，参数／返回也需补查。Os_HostCreateActor 的 mmap→unsigned char 映射沿原地址类别，Linux 返回 NULL 与 kernel task handle 分别核对原位置。#544 承接，继续归 #443；恢复须收齐别名、参数／返回、分配与恢复类型，证明对齐、生命周期及直接解引用用途。 |
| R11.6 | Required | void 指针与算术类型的 cast | satisfied | 主文 99＋TC2.22；全部 cast 含嵌套括号、宏展开和未选分支。void* 仅有零整数常量的 (void *)0 与对象地址→void*，前者为原文例外，后者不属于算术类型转换；无其他 void*／算术 cast。Os_ArtiAlarmBaseReturn 的源是 AlarmBaseRefType 对象指针，交 R11.4；旧 VirtualQuery／AllocationBase 实例按 #502 转历史。 |
| R11.7 | Required | 对象指针与 Boolean／character／enum／floating 的 cast | satisfied | 主文 99–100；Boolean、character、enum、实／复浮点分别核对完整 cast 清单和 typedef。对象地址只观察为 uint64，NULL 为 void* 零，char* 的字符访问仍为指针到指针；角色 char cast 的源是 character 条件结果，非指针。无对象指针与上述四类非整数算术类型之间的 cast；固定入口跳过嵌套源的缺口由完整语法与源类型回查补齐。 |
| R11.8 | Required | 指向对象的限定符保留 | not_assessed | 主文 100–101＋AMD2.61、AMD4.10–14：有效义务覆盖 conversion 及 const／volatile／_Atomic，restrict 移除另有原文说明。固定入口只比较 cast 和部分已解析实参的最低 constness 位。Linux futex 的 const volatile int32_t *、原生上下文与 Dcm 输出分别回接其声明；#544 承接，继续归 #443；仍须逐指针层收齐赋值／返回／间接调用及所有别名，证明限定符保留或整改具体丢弃。 |
| R11.9 | Required | 整数空指针常量的形式 | not_assessed | 主文 101–102＋TC1 {0} 例外；(void *)0 的零在 cast 内，允许自定义宏包裹该形式。NULL_PTR／NULL 当前实际展开 ((void *)0)，Os_Error sources、CanTp PduInfo 及配置函数指针初值分别核对；固定入口未覆盖全部反向比较、实参／返回及嵌套宏。#544 承接，继续归 #443；恢复须完成类型化空指针常量清单，区分完整 NULL 展开、void * 形式、{0} 和其他整数零，追齐每个消费者。 |
| R12.1 | Advisory | 优先级及显式括号 | not_assessed | 主文 103–105＋AMD2.62 的有效优先级表已看原页；只要求规定区间的高优先级子操作数括号和 sizeof 括号。同号入口用 precedence／parentheses 及 raw sizeof。map_address 的 sizeof(result)*2u、HostBatch 数字运算和 Windows 写流的既有括号可定位。#544 承接，继续归 #443；仍须核对所有项目宏展开及当前 #if／#elif 的语法树与原括号映射，逐表判断；条件被 -E 消去后的输出缺少这部分证据。 |
| R12.2 | Required | 移位计数与左值 essential 宽度 | not_assessed | 主文 105–106 明确按 essential 宽度，1u 的窄 essential type 与标准 int 提升分开；同号入口跳过零最大值／缺值流，负计数需另查。Com 字节打包先转 uint32，HostBatch::number 以乘法累积，Os_Interrupt 的位掩码有实际向量输入。#544 承接，继续归 #443；恢复须按全部动态／宏移位给出 RHS 下界和上界、左侧 UTLR／cast 后宽度及 LP64／LLP64 模型，连接输入校验和直接消费者。 |
| R12.4 | Advisory | 实际求值的无符号常量表达式回绕 | not_assessed | 主文 106–107＋TC1 同时要求常量约束与语义；读取 const 对象仍非该常量表达式，未求值分支按原文排除。同号入口仅算已知 +／-／*，宽度≥64 跳过。内核 xTickCount 运行时回绕沿既有 R14.3 具体判断；清位取反与 literal 常量应另按实际子表达式。#544 承接，继续归 #443；恢复须独立求值全部 64 位及条件／宏常量子式，含当前 #if／#elif 的求值分支和整数模型。 |
| R13.1 | Required | 当前全部 initializer list 的持久副作用 | satisfied | 主文 108＋AMD2.63；固定入口只识别 ++／--／赋值，独立枚举实际列表并核对普通字段／参数、地址、sizeof 和回调取址。CanTp_Admit／Perform 的 PduInfo、Os_ArtiCallerSave previous、Os_Error sources、Time observed 与 Ecu 配置均无调用、写入或 volatile 读取；Backend 的 Os_ErrorContextType{0} 与 Time 的 Os_CounterWorkType{counter／child,0u,标志} 复合字面量同样无副作用。PduR／LSduR、ARTI 捕获和输出调用均在列表之外；具体范围与次数见上文。 |
| R13.2 | Required | 求值顺序及线程交错下的表达式 | not_assessed | 主文 109–110、TC1／AMD2.64、TC2.23–25＋AMD4.17–23；全部子式均须考虑，已知 false 不免除对象访问约束，atomic 访问和线程交错也纳入。核心 unknownEvaluationOrder 不能完成别名／跨调用证明。当前 Interlocked、futex、Time 完成与 Backend actor 回调分别有同步协议；#544 承接，继续归 #443；恢复须逐 full-expression 追实际对象读写、volatile／原子次数、回调别名和允许交错，结合 #470 启动掩码时点核查。 |
| R13.3 | Advisory | 当前 ++／-- 的 full expression | satisfied | 主文 111–112；调用按副作用计，未求值子式也检查。同号入口在逗号／分号停下，另独立核对全部实际展开的完整语句／for 迭代。HostBatch／BSW 计数、OS 队列／深度和 patch 的 xByte 均单独增减；Os_Schedule::process 的 ++transitions 仅与常量比较，声明为普通局部量，full-expression 没有另一调用、写入或 volatile 访问。 |
| R13.4 | Advisory | 赋值结果的后续使用 | satisfied | 主文 112–113；简单及复合赋值逐一检查 parent 和最终结果消费者，未选物理分支另核。全部仅作独立表达式语句或 for 子句；tasks.c:1902／2957 的 listSET_LIST_ITEM_VALUE 外层括号逐层剥离后仍是独立语句，无链式赋值、条件、返回、实参或未求值子式使用赋值结果。固定入口只查 = 的缺口由复合赋值及语法全量检查补齐。 |
| R13.6 | Required | 当前全部 sizeof 操作数 | satisfied | 主文 114–115＋AMD2.65、TC2.26／40／42 的 Required 类别；持久性不限于实际求值，函数调用算副作用。独立枚举实际展开的类型、数组／对象成员、原生返回大小和布局断言，均无调用或写入；Os_LinuxNative、Os_Atomic／Arti 的 char[条件 ? 1 : -1] 只读编译期布局常量，非 VLA 副作用。复制／输出与 VirtualQuery 等调用都在 sizeof 之外；未借 volatile 非 VLA 例外扩大范围。 |
<!-- misra-checker-obligations-end -->
