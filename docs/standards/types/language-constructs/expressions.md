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

## 当前表达式与消费者

正常 master run 43418599046 的实际树为 `76a4f77c`，八份完整分析按目标 Git 换行
核对 seal、最终 kernel、92 项 producer、12 个驱动、16 个程序／888 个 C99 TU
及 GCC／Cppcheck／SDK 输入闭包；原身份与 evaluate 和 CI 保存值逐项一致。
当前 master 的 #470 启动掩码增量另以正常成功 run 43411175434 的完整工程复核：
`startup_mask` 及 block_actor_signals／actor_body 的调用改变信号交付时点，
没有新增初始化列表、增减或 sizeof；恢复信号前后仍有独立的调用和错误检查。
最终绑定记录扫描来源与当前评估正文／驱动的身份，原分析摘要只读。

452 份真实 GCC 展开按行标记回接项目使用点，kernel 按实际 patchedLines 区分；
保留重复 TU 的次数，独立词法括号配对后逐个检查操作数和 full-expression。
初始输入中 Linux 每 profile 有 384 次 sizeof、310 次增减，Windows 分别为
311／293 次；初始化列表 Linux 四 profile 为 147／147／152／148 次，Windows 为
137／137／142／138 次（顺序为 standard、user、component、no-diagnostic）。
列表的子列表分别计数；另核 Os_ErrorContextType 的 {0} 和 Time 的两个
Os_CounterWorkType 复合字面量，后两者只读普通 counter／child 局部量。去重后两目标共有 117 种列表拼写、114 种增减片段和
65 种 sizeof 操作数；相同拼写仍按声明类型和来源逐位置判断。

- CanTp_Admit／CanTp_Perform 的 PduInfoType 只读取普通 Work 字段、帧地址与长度，
  PduR／LSduR 调用在列表之外；Can／CanIf 的 payload／TxPduId 同样是普通参数。
- Os_ArtiCallerSave 先单独取得 Arti_AddressCapture，再将普通 TLS 深度／状态和
  address 放入 previous；Os_Error 的 sources 只保存地址和参数，Time 的 observed
  只读固定 completion_event／attention_event 两个非 volatile HANDLE。
- Ecu 配置表保存 literal、enum、对象／函数地址；回调取址没有调用副作用。
  TextField 的列表含 storage 与 sizeof(storage)，后续输出函数才修改缓冲。
- 增减对象为循环索引、队列／协议计数、嵌套深度和观测计数，均为独立语句或
  for 迭代；Os_Schedule::process 的 if(++transitions > 常量) 只有这一处修改。
- sizeof 的固定数组、对象／成员及类型用来限定复制、输出和原生 API 返回大小；
  Os_LinuxNative、Os_Atomic／Arti 的 char[条件 ? 1 : -1] 是常量布局断言。
  __builtin_offsetof／__alignof__ 是布局运算，未引入函数调用或 VLA 大小副作用。

采用原行及 SDK 原头内部沿 ADR 0025 的原责任；项目使用点的展开已纳入上述核查。
Windows 完整 SDK 的 Clang 解析遇到 GCC 内建函数兼容错误，未据该失败作阴性判断。
新增类型化转换、宏用途、配置／TU、patch、SDK 或目标模型时重评。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R10.2 | Required | character 加减与结果消费者 | not_assessed | 主文 85＋TC2.19 限定另一操作数 rank≤int；固定入口的字符识别偏重 literal／加法链。Ecu_HostBatch::digit 先转 int，map_address 在范围检查后作 char 差并转 unsigned；HostText 从 digits 数组选字符。既有证据只覆盖这些表达式。#443 恢复须按完整 typedef／变量／宏及条件结果清单追齐 character 运算，核对 rank 和下游数值／字符用途。 |
| R10.4 | Required | usual arithmetic conversion 的全部操作数 | not_assessed | 主文 87–88＋AMD2.52／AMD3.31–32；字符加减例外和实／复浮点例外分别判断，没有 signed 常量通用例外。同号入口跳过缺类型 AST，嵌套 Boolean／character 结果有既有误报。CanTp 状态、Os_TargetWaitTick waiting、complete_activation 与角色字符条件式只复用原篇精确裁决。#443 恢复须逐其余算术／比较／复合赋值和 ?: 对齐命名 enum、Boolean、character 与实际类别，补完整结果消费者。 |
| R10.5 | Advisory | 显式 essential category 转换 | not_assessed | 主文 89–90＋TC1、AMD2.53–55、AMD3.33 的有效矩阵；0／1 例外限 signed／unsigned 常量。固定 helper 没有完整命名 enum／实复浮点模型，常量分支只识别 1。CanTp 的 (boolean)TRUE 为同类 Boolean，HostBatch 的字符先转 int、原生 bitset 的 signed／unsigned 转换各按原类型。#443 恢复须收齐当前所有 cast 的源／目标类别及值域，含 enum 和嵌套宏，按有效矩阵逐项结算。 |
| R10.7 | Required | 复合表达式参与 usual arithmetic conversion | not_assessed | 主文 90–92、TC1 §8.10.3＋AMD3.34；常量表达式排除，取反与非恒定一元正负号按修订处理。同号入口偏重算术 AST／部分赋值父节点。HostBatch::number 在计算前扩展 next，map_address 以 uintptr_t 累积，Com 在移位前转 uint32，Target 先转 size_t 再乘。#443 仍须逐当前移位／按位／条件和复合赋值列出两侧 essential 宽度，补全表外运算与直接消费者。 |
| R10.8 | Required | 复合结果的显式 cast | not_assessed | 主文 92＋TC1 复合边界／AMD3.35 实复浮点例外。固定入口没有先排除常量表达式；Backend 两处和 Mailbox 一处 (LONG)~(uint32)1u 沿原篇常量误报判断，清位仍交 InterlockedAnd。#443 恢复须收齐其余非恒定 cast 的源类别／宽度、嵌套 ?: 和一元正负号及结果用途，原三处裁决只覆盖相应表达式。 |
| R11.1 | Required | 函数指针的显式与隐式转换 | not_assessed | 主文 93–95 三项例外，AMD2.58 仅更新函数设计符说明。固定函数类型 helper 仍需完整原型核对；Ecu_Execution.c 的 (void)sink 沿原丢弃例外，Det_Callouts、RTE Runnable、OS actor／sink 回调可定位。#443 恢复须沿每个赋值／参数／返回和间接调用确认兼容原型／调用约定，区分 NULL 常量、函数到自身指针及 void 丢弃。 |
| R11.2 | Required | 不完整类型指针的转换 | not_assessed | 主文 95＋TC1 无限定类型，例外允许 NULL、同一不完整类型及 void 丢弃；void 指针沿其他规则。固定入口只收 struct 前置拼写，不能覆盖全部 typedef／union 或完整化时点。当前 Os_HostHandleType→HANDLE→Os_HostLinux 定义及 Det／RTE 回调类型可追溯；#443 仍须按每个转换处的实际可见类型核对完整化、隐式参数／返回和直接消费者。 |
| R11.3 | Required | 不同对象类型指针的 conversion | not_assessed | 主文 96–97、AMD2.59／TC2.21＋AMD4.7–9；有效义务覆盖隐式 conversion，character 访问例外限非 atomic 源对象。同号入口只查 cast，void、部分多层类型会跳过。Host native 上下文／字节复制及 Can PduInfo 数据指针须与实际目标类型区分。#443 恢复须为全部显式／隐式转换追到解引用／复制方，核对逐层无限定类型、字符访问例外、对齐和别名；仅增加 const 的同类型转换另按原文判断。 |
| R11.4 | Advisory | 对象指针与整数之间的转换 | not_assessed | 主文 97–98＋TC1 限对象指针／AMD2.60；uintptr_t 可表示性未提供通用例外。同号入口只查显式 cast。Os_HostStackRange、Os_StackRecordBuffer 与 VirtualQuery 地址沿原主机地址类别，Os_Arti 的数值地址合同分别保留。#443 恢复须收齐隐式及复杂表达式，核对 LP64／LLP64、NULL 例外、表外地址及实际消费者；原八实例记录保持精确范围。 |
| R11.5 | Advisory | void * 到对象指针的显式／隐式转换 | not_assessed | 主文 98 的例外仅为 void * 空指针；malloc 等真实对象恢复仍须结算。同号入口跳过直接分配函数形态，参数／返回也需补查。Os_HostCreateActor 的 mmap→unsigned char 映射沿原地址类别，Linux 返回 NULL 与 kernel task handle 分别核对原位置。#443 恢复须收齐别名、参数／返回、分配与恢复类型，证明对齐、生命周期及直接解引用用途。 |
| R11.6 | Required | void 指针与算术类型的 cast | not_assessed | 主文 99＋TC2.22 明确整数到 void 指针为实现定义行为；零整数常量是具体例外。同号入口只查一层 void 指针。Windows Os_StackRegister／Check／Observe→VirtualQuery 的地址和 AllocationBase→reserve_low 沿既有主机地址类别。#443 恢复须收齐其余 cast、浮点与表外往返，核对宽度及实际 API／解引用消费者，保留 Required 正式义务。 |
| R11.7 | Required | 对象指针与 Boolean／character／enum／floating 的 cast | not_assessed | 主文 99–100 对非整数算术类型另含 essentially Boolean、character、enum；缺少浮点不足以排除此条。同号入口跳过源有 astOperand1 的嵌套形式，整数／enum 判断也不足以表示有效类别。当前主机地址使用 uintptr_t／void 路径，角色字符为字符条件结果。#443 恢复须按完整 cast 清单排查四类源／目标与多层指针，逐个连接宏、typedef 和下游使用。 |
| R11.8 | Required | 指向对象的限定符保留 | not_assessed | 主文 100–101＋AMD2.61、AMD4.10–14：有效义务覆盖 conversion 及 const／volatile／_Atomic，restrict 移除另有原文说明。固定入口只比较 cast 和部分已解析实参的最低 constness 位。Linux futex 的 const volatile int32_t *、原生上下文与 Dcm 输出分别回接其声明；#443 仍须逐指针层收齐赋值／返回／间接调用及所有别名，证明限定符保留或整改具体丢弃。 |
| R11.9 | Required | 整数空指针常量的形式 | not_assessed | 主文 101–102＋TC1 {0} 例外；(void *)0 的零在 cast 内，允许自定义宏包裹该形式。NULL_PTR／NULL 当前实际展开 ((void *)0)，Os_Error sources、CanTp PduInfo 及配置函数指针初值分别核对；固定入口未覆盖全部反向比较、实参／返回及嵌套宏。#443 恢复须完成类型化空指针常量清单，区分完整 NULL 展开、void * 形式、{0} 和其他整数零，追齐每个消费者。 |
| R12.1 | Advisory | 优先级及显式括号 | not_assessed | 主文 103–105＋AMD2.62 的有效优先级表已看原页；只要求规定区间的高优先级子操作数括号和 sizeof 括号。同号入口用 precedence／parentheses 及 raw sizeof。map_address 的 sizeof(result)*2u、HostBatch 数字运算和 Windows 写流的既有括号可定位。#443 仍须核对所有项目宏展开及当前 #if／#elif 的语法树与原括号映射，逐表判断；条件被 -E 消去后的输出缺少这部分证据。 |
| R12.2 | Required | 移位计数与左值 essential 宽度 | not_assessed | 主文 105–106 明确按 essential 宽度，1u 的窄 essential type 与标准 int 提升分开；同号入口跳过零最大值／缺值流，负计数需另查。Com 字节打包先转 uint32，HostBatch::number 以乘法累积，Os_Interrupt 的位掩码有实际向量输入。#443 恢复须按全部动态／宏移位给出 RHS 下界和上界、左侧 UTLR／cast 后宽度及 LP64／LLP64 模型，连接输入校验和直接消费者。 |
| R12.4 | Advisory | 实际求值的无符号常量表达式回绕 | not_assessed | 主文 106–107＋TC1 同时要求常量约束与语义；读取 const 对象仍非该常量表达式，未求值分支按原文排除。同号入口仅算已知 +／-／*，宽度≥64 跳过。内核 xTickCount 运行时回绕沿既有 R14.3 具体判断；清位取反与 literal 常量应另按实际子表达式。#443 恢复须独立求值全部 64 位及条件／宏常量子式，含当前 #if／#elif 的求值分支和整数模型。 |
| R13.1 | Required | 当前全部 initializer list 的持久副作用 | satisfied | 主文 108＋AMD2.63；固定入口只识别 ++／--／赋值，独立枚举实际列表并核对普通字段／参数、地址、sizeof 和回调取址。CanTp_Admit／Perform 的 PduInfo、Os_ArtiCallerSave previous、Os_Error sources、Time observed 与 Ecu 配置均无调用、写入或 volatile 读取；Backend 的 Os_ErrorContextType{0} 与 Time 的 Os_CounterWorkType{counter／child,0u,标志} 复合字面量同样无副作用。PduR／LSduR、ARTI 捕获和输出调用均在列表之外；具体范围与次数见上文。 |
| R13.2 | Required | 求值顺序及线程交错下的表达式 | not_assessed | 主文 109–110、TC1／AMD2.64、TC2.23–25＋AMD4.17–23；全部子式均须考虑，已知 false 不免除对象访问约束，atomic 访问和线程交错也纳入。核心 unknownEvaluationOrder 不能完成别名／跨调用证明。当前 Interlocked、futex、Time 完成与 Backend actor 回调分别有同步协议；#443 恢复须逐 full-expression 追实际对象读写、volatile／原子次数、回调别名和允许交错，结合 #470 启动掩码时点核查。 |
| R13.3 | Advisory | 当前 ++／-- 的 full expression | satisfied | 主文 111–112；调用按副作用计，未求值子式也检查。同号入口在逗号／分号停下，另独立核对全部实际展开的完整语句／for 迭代。HostBatch／BSW 计数、OS 队列／深度和 patch 的 xByte 均单独增减；Os_Schedule::process 的 ++transitions 仅与常量比较，声明为普通局部量，full-expression 没有另一调用、写入或 volatile 访问。 |
| R13.4 | Advisory | 赋值结果的后续使用 | not_assessed | 主文 112–113 覆盖简单／复合赋值且含未求值子式，无另设通用例外。固定入口只查 = 的部分 AST parent。原篇 listSET_LIST_ITEM_OWNER／VALUE、queue 锁与 task 选择只复用既有展开；当前 Os_HostLinux 的 status 调用和 HostBatch 字段写入为独立语句。#443 恢复须收齐全部 = 及复合赋值的 parent／结果消费者，逐个排查条件、返回、索引、实参、链式赋值及未选表达式。 |
| R13.6 | Required | 当前全部 sizeof 操作数 | satisfied | 主文 114–115＋AMD2.65、TC2.26／40／42 的 Required 类别；持久性不限于实际求值，函数调用算副作用。独立枚举实际展开的类型、数组／对象成员、原生返回大小和布局断言，均无调用或写入；Os_LinuxNative、Os_Atomic／Arti 的 char[条件 ? 1 : -1] 只读编译期布局常量，非 VLA 副作用。复制／输出与 VirtualQuery 等调用都在 sizeof 之外；未借 volatile 非 VLA 例外扩大范围。 |
<!-- misra-checker-obligations-end -->
