# 控制流、调用与对象生命周期的检查边界

输入与责任沿[语言与词法](lexical.md)。同号入口为固定 addon 的
`misra_N_M`；核心入口为其准确诊断映射。控制流能力依赖实际宏展开、函数图、
scope 及值流；函数指针回调、跨 TU 服务和被调用接口的资源约束另有人工责任。
第三阶段第四批覆盖本篇原有的 33 个 not_assessed 条目；R15.5 保持 ADR 0025 的既有
GRP 结论。R14.4 属固定 11 项 checker-limitations，R17.3 的 C99 不适用记录在索引，
R17.9–R17.13／R18.9–R18.10 属原 66 项，均沿原组消费。

## 有效正文与当前输入

七份授权原件逐一通过原 SHA256SUMS。主文 §8.14 的 loop counter 定义及 R14–R19
为印刷 115–155／PDF 123–163；R16.1 的 switch 文法另渲染 PDF 138 页核对。
AMD2.27 扩充 C11 适用性，66–67 更新 R14.2 的语言措辞，68–69 更新 R15.3／R16.1，
70–74 更新变参、返回和数组形参，75–76 明确单对象的数组算术模型。
TC1 修正 R14.2 第一子句的持久副作用、R15.6 示例、R15.7 的函数调用副作用及
R16.1 文法排版；R19.1 只修例子。TC2.27–28 收紧 R14.3 的 do-while 常量假例外，
29 修 R15.7 交叉引用，30 增 C99 main 例外，31–32／41／43 将 R17.5 定为 Required，
33–34 明确字符指针访问对象表示的 sizeof(T) 边界。
AMD3.65–66 将 R17.1 明确为禁止包含 stdarg.h 及使用其设施；AMD4.24 扩 R18.6
到 thread-local，25–30 将 R18.8 限于 VLA，52 将 R18.3 的对象改为表达式。
AMD1 及其余修订没有改变本表相关 C99 义务。

本批以 master `200569224bf0f1c59aeb96e1647abe9b19638eec` 为基线，取正常成功
[run 43555839362](https://github.com/g1331/autsaro/actions/runs/43555839362) 的双目标
四 profile、HostBatch／probe 实际输入：888 个程序 TU、452 份 GCC 展开及 1,846 个
物理 C／头文件实例。物理源码包含未选分支；实际 GCC 展开按行标记
回接自有 runtime、生成／应用及 patchedLines 中的项目补丁。FreeRTOS 原行与 SDK
原头沿 ADR 0025 的来源责任，自有使用点仍核查；ISO 库内部沿主文 §6.7。
浮点 typedef、数组存储与求值用途分别检查；空 [] 逐项区分推导容量的数组定义、
数组形参与结构成员。没有相应语法构造的满足结论以独立枚举为据。
输入变化时重新评估。

本批结算 15 项 satisfied，其余 18 项保留具体恢复条件；全组为
35 satisfied、3 not_applicable、106 not_assessed。本篇加上既有 R15.5 共 16 项满足。
本组当前输入绑定沿[声明篇](declarations.md#当前输入重评)，未评估项继续由 #443 承接。


## 当前 switch 全集

实际展开有 184 个 switch 实例、18 种语法体，对应以下 10 个逻辑构造；原注释回接物理源。

| 消费者 | 实际控制类型 | clause 数 |
| --- | --- | ---: |
| Ecu_StatusName | EcuStatus enum | 18 |
| Ecu_BswMAction | ComM_ModeType→uint8 | 4 |
| Os_ArtiTask | Os_ArtiTaskEventType enum | 7 |
| Os_ArtiHook | Os_HookPhaseType enum | 6 |
| Os_ArtiService | OSServiceIdType→uint8 | 42 |
| native_task_state | eTaskGetState 返回 eTaskState enum | 5 |
| Os_BackendState | actual 为 eTaskState enum | 5 |
| update_owner_mask | operation 为 uint32 | 6 |
| action | alarm.action 为 uint8 | 5 |
| Dcm_Publish | effect.kind 为 Dcm_PublicationKindType enum | 6 |

Ecu_BswMAction 由 core/src/integration/ecu_config.rs 生成，进入 Ecu_HostBusSM_ApplyMode。FreeRTOS tasks.c 的三个 switch 完整区间为 7451–7478、
8032–8074、8175–8216，八份 patchedLines 均无项目行，原件沿采用组；co-routine 宏无项目调用。

## 第九批的当前结算

当前 master `2f42a9b9` 的双目标四 profile 输入由正常
[run 44046359697](https://github.com/g1331/autsaro/actions/runs/44046359697)
的实际检出树 `f386fd2e` 提供，树字节相同；最近成功 master
[run 44021064087](https://github.com/g1331/autsaro/actions/runs/44021064087)
为 `0f8cd603`，用于核对到当前 master 的增量。核查 16 个程序的 888 个
C99 TU，GCC origins 共有 452 份。物理扫描先拼接续行并保留每个字符的原行，
清除注释时保留换行。展开按 GCC 行标记回查交付源、宏使用点与 patchedLines，
采用原行和主机 provider 按 ADR 0025／0030 归属。最终使用本批 PR 的实际
producer 和封存输入重算[当前绑定](declarations.md#当前输入重评)。

本批结算 R15.4、R15.6、R15.7、R17.4、R17.5、R17.8 和 R18.8 七项。
11 条转 #556，其中 R14.3 修复了 Os_Backend 的一个具体违反，R18.4、R18.6
有既有违反和处置。全组为 92 satisfied、3 not_applicable、49 not_assessed。
R17.2 主机端口的既有偏离沿 ADR 0030 转历史；当前自有回调与再入边仍归 #556。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R14.1 | Required | 浮点循环计数及控制 | satisfied | 主文 §8.14／117 的 counter 须为标量、单调变化并参与退出决定；同号入口仅识别 for 及有限 while 模式。独立核对当前物理文件与实际展开，浮点仅 Platform_Types 的 float32／float64 typedef、portFLOAT／portDOUBLE 定义和 Linux Os_StackLinux.c 的 altstacks[].storage 的 long double 数组／sizeof；没有浮点值运算、标量或成员计数。备用栈由 sigaltstack 按字节使用，所有实际计数均无 essentially floating 来源。 |
| R14.2 | Required | for 的三个子句与循环体 | not_assessed | 主文 118–119、TC1 与 AMD2.66–67。逐实际 for 体提取直接写、取址及调用候选。146 种体形未见所属计数器的直接改写或取址；ComM_NotifyUsers 的生成 notify 与 Os_Time 的定时 action 尚缺传递写集合。#556 承接，归 #443，逐 call 按实参别名核实 condition／update 读取的对象，区分单标识符 Boolean flag 与三个空子句。 |
| R14.3 | Required | 控制表达式的 invariant 性质 | not_assessed | 主文 119–121＋TC2.27–28。native_task_state 的 configASSERT(FALSE) 展开为恒真 if；改为直接调用同一 noreturn Os_BackendAssert，外层 valid 仍由 FreeRTOS eTaskGetState 分派。xTaskIncrementTick 回绕与恒假 do-while 的例外分别保持。#556 承接，归 #443，须验证配置、回调写入、task_waiting 重读及标准 assert 展开以外的各控制值域。 |
| R15.1 | Advisory | 实际预处理后的 goto | satisfied | 主文 122，AMD2.27 只追加 C11；同号入口逐 semantic goto 报告。物理自有／生成／应用文件、补丁最终项目行及实际 GCC 展开均无 goto 关键字，另区分注释、字符串和宏 replacement；主机清理使用显式分支和返回。独立词法枚举补足工具模型，当前没有需保留的项目跳转。 |
| R15.2 | Required | goto 目标的前向关系 | satisfied | 主文 122–123 要求同一函数中向后声明的 label，同号入口 findGotoLabel 仅向前查找。当前全部自有／生成／应用及项目 patch 的原 token 和实际展开没有 goto，故没有须比较的源序／目标标签对；与 R15.1 共用经来源核验的构造全集，宏展开也未产生跳转。 |
| R15.3 | Required | goto 与 enclosing／switch clause | satisfied | 主文 123–124＋AMD2.68；switch-clause 按 block 处理。同号入口检查 enclosing scope，但其 switch-clause 状态机不能单独证明符合。当前项目原 token 及实际展开没有 goto，故没有跨块或跨 clause 的跳转；该构造缺席证据同时覆盖宏、生成消费者与项目 patch。 |
| R15.4 | Advisory | 循环的 break／goto 终止点数量 | satisfied | 主文 125–126。337 个自有或内核展开循环形态按最近 loop／switch 归属配对 break，实际无 goto。唯一双 break 为 xTaskIncrementTick 的 4874／4896 行，分别保护空链和未到期队首。完整循环 4864–4952 与八份 patchedLines 的交集为空；该采用实例的 Advisory 违反沿 OS 篇保留。自有源码和项目补丁的循环满足。 |
| R15.5 | Advisory | 自有 C 的函数退出路径 | satisfied | [ADR 0025 GRP](../../../adr/0025-misra-layered-scope.md#grp指南重分类计划)已将本条对自有目标 C（含生成 C 及项目补丁；主机层已由 [ADR 0030](../../../adr/0030-host-simulation-layer-outside-misra-scope.md) 移出范围）Disapplied；本行消费既有重分类决定，固定 addon 的非顶层 return 告警照常保留。资源回收、输出和返回语义继续由各自规则与实际消费者核查，采用原件沿供应方责任。 |
| R15.6 | Required | iteration／selection 的复合语句体 | satisfied | 主文 127–129＋TC1。配对双目标四 profile 的实际 if／else、switch、for、while、do 体。范围内没有独立非复合体；else 后直跟 if 按例外，do-while 尾部只为条件。FreeRTOSConfig.h 中 configASSERT 的嵌套 if 和 do 展开均有复合体，源宏和使用点分别核对。 |
| R15.7 | Required | if／else-if 链的最终 else | satisfied | 主文 129＋TC1／TC2.29。整链物理区间与 patchedLines 求交，77 个范围内展开链变体均有最终 else，其中 44 个空体回查物理源注释。八份输入回查采用原行起始的三条链：kernel/queue.c:1007 的 1032 为注释，kernel/queue.c:2367 的 2401／2407 属复制与队列指针写入分支，kernel/tasks.c:4322 的 4336 为注释。生成 Ecu_ComMMode_UM_HostUser 回接 ecu_config.rs。普通单 if 无最终 else 义务。 |
| R16.1 | Required | switch 的规范结构 | satisfied | 主文 130–131 文法已看原页，TC1 修 opt 字体、AMD2.69 更新语言措辞；固定入口按 break 近似。独立配对 184 次展开、10 个逻辑 switch：主体从 case 开始，clause 为语句列表或单个复合体并以 break 结束，无文法外标签。Ecu_BswMAction 回接 ecu_config.rs producer 和 HostBusSM，其他消费者见全集。 |
| R16.2 | Required | case／default 的所属 switch | satisfied | 主文 131 同时约束 case 和 default，固定入口只查 case.scope。独立维护所有复合体／switch 体关系，184 次展开每个标签的最近复合体均为所属 switch 主体，内层 if／loop 无标签；原 kernel switch 无项目补丁行。 |
| R16.3 | Required | 各 switch clause 的终止 | satisfied | 主文 131–132 约束非空 clause 末尾的无条件 break，空标签可共享；固定 raw 入口额外接受 return／throw／fallthrough。独立按直接标签分组，184 次展开的每个 clause 均在直接层以 break 结束，单个复合体以其内部末尾 break 结束，无隐式贯穿或条件退出替代。 |
| R16.4 | Required | 每个 switch 的 default | satisfied | 主文 132–133 要求 default，且 break 前有语句或注释；固定入口只查标签。10 个逻辑 switch 均有 default：赋状态／错误、清 owner 或更新中断；唯一只有 break 的 Os_ArtiHook，逐展开回接物理源 AlarmCallback and NONE 注释，明确该类没有 Hook 事件。 |
| R16.5 | Required | default 的首尾位置 | satisfied | 主文 134 约束 default 为首个或最后一个 switch label；入口按花括号／case 近似。独立提取直接标签序列，184 次展开的 default 全部为最后标签；内层复合体另配对。 |
| R16.6 | Required | switch clause 的最少数量 | satisfied | 主文 135 至少两个 clause，共享一条执行路径只算一个；固定入口以 break／return／throw／noreturn 数量近似。独立按直接标签及非空内容分组，10 个逻辑 switch 各有 4–42 个 clause，184 次展开逐一满足；未用共享标签或内部退出凑数。 |
| R16.7 | Required | switch 控制表达式的 essential 类型 | satisfied | 主文 136＋D.6；isBoolExpression 的数值模型由完整类型清单补齐。控制式均为下表五类命名 enum 或 uint8／uint32，无 Boolean 转换／条件式；两个 Backend switch 的实际类型均为 eTaskState。ComM_ModeType／OSServiceIdType 回接 uint8，Hook／ARTI／Dcm 回接命名 enum 与真实成员；逐声明连接全部 10 个消费者。 |
| R17.1 | Required | 变参设施的声明和使用 | satisfied | 主文 136–137＋AMD2.70、AMD3.65–66；当前义务同时禁止 #include stdarg.h 和 va_list／va_arg／va_start／va_end／va_copy。固定入口遗漏仅包含头及内建拼写，另独立查物理 include／宏和实际展开的 __builtin_va_*／__gnuc_va_list；项目原件、生成／应用及 patch 无此包含或设施用途。HostBatch 的 snprintf／sscanf 是普通库调用，未自行遍历变参；stdio 原头内部声明按 §6.7，非 ISO provider 内部沿采用责任。 |
| R17.2 | Required | 直接／间接递归的完整调用图 | not_assessed | 主文 137。四 profile 的生成 det_config 未注册开发错误 hook，唯一 runtime callout 为 Ecu_DetRuntimeCallout。ADR 0025 的同步嵌套 ISR 类别只有两份端口补丁，ADR 0030 后转历史；当前 Os_ErrorResult 到 ErrorHook 和 Os_PortDispatchNested、Backend 到生成 ISR 的回调边仍须评估。#556 承接，归 #443，按实际注册、回调、runner／Task 和外层 guard 闭合间接图。 |
| R17.4 | Mandatory | 非 void 函数的返回路径 | satisfied | 主文 138–139＋AMD2.71–72／TC2.30。实际两个程序的 TU 以 C99、-Wall、-Werror 完整编译链接。Linux 228 个真实展开另以 -Werror=return-type 编译；Det_ReportError 的无限循环保护 halt 返回边界，Os_BackendAssert／Os_BackendShutdown 沿 noreturn 声明到关闭 provider。逐展开区分非 void 的显式返回值、无正常出口与 C99 main 例外。 |
| R17.5 | Required | 数组形参的实际可访问范围 | satisfied | 主文 139–140＋TC2.31–32／41／43 为 Required。实际数组形参为 CAN 八字节合同；Can_InternalHostFlush 的 payload[8] 经过 Ecu_ExecutionTransmit 到 Ecu_TargetEnqueueTransmit，所选 provider 不调用 Can_TxSinkType。接收经 Ecu_Input.data[8] 或 Ecu_BatchFrame.data[8]、Can_Inject、Can_InternalInject 到 Can_ReceiveHost，均从首元素传入。未调用的外部入口拥有独立容量合同。 |
| R17.6 | Mandatory | 数组参数声明的 static 形式 | satisfied | 主文 140–141＋AMD2.73–74，C99 Mandatory；禁止数组形参方括号内任意位置的 static。同号入口只匹配 [ static；独立平衡全部实际方括号并检查 const／volatile／restrict 等顺序，同时查物理声明、宏和 typedef，当前项目／生成／应用及 patch 无 static 数组形参。Can_TxSinkType 的 data[8] 与所有实际原型不含该容量承诺关键字；原 SDK 声明沿采用责任。 |
| R17.7 | Required | 非 void 返回值的使用或显式丢弃 | not_assessed | 主文 141–142。为实际调用提取表达式父节点；Ecu_ExecutionTransmit 状态赋给 result，Can_TxSinkType 由所选 provider 消除，CAN 和 DET 显式丢弃按语义核对。#556 承接，归 #443，须闭合全部跨 TU 及间接声明返回类型、嵌套表达式、内建与 SDK 宏。D4.7 的错误处理继续独立。 |
| R17.8 | Advisory | 函数形参对象的修改 | satisfied | 主文 142。按展开的形参声明回接赋值／自增的左对象及取址。0023-preserve-parameter-values.patch 的内核推进使用局部量。当前形参对象无直接写，三处对按值形参 data 的取址均为生成 Rte_Write 向 Com_SendSignal 传只读源，由 memcpy 读入 COM 的 signal_values。指针所指对象写入另沿服务输出合同。 |
| R18.1 | Required | 指针算术结果所在数组范围 | not_assessed | 主文 143–144＋AMD2.75–76／TC2.33–34。0020-bounded-queue-front.patch 先判队列头再执行减步；Can_ReceiveHost 按 DLC 遍历且当前输入容量为八。#556 承接，归 #443，须逐偏移／下标及复合赋值证明基对象、最大范围和 one-past 不解引用，含队列步长、Dcm 请求／响应及 OS 索引。 |
| R18.2 | Required | 指针相减的对象归属 | not_assessed | 主文 145。同数组或其末尾后一位置的指针可相减。旧 record-stacks 例已随两个栈 provider 归主机范围外。#556 承接，归 #443，须按当前实际操作数类型核对减法全集，将每个指针对回接共同对象和 ptrdiff_t 可表示范围，区分整数地址差。 |
| R18.3 | Required | 指针关系比较的对象归属 | not_assessed | 主文 145–146＋AMD4.52 按 pointer 表达式判断。主机 record 范围比较已随平台栈迁移归范围外。#556 承接，归 #443，须逐当前关系式确认指针两端共同对象、one-past 或同结构成员关系，含 SDK 及项目补丁展开。 |
| R18.4 | Advisory | 指针的 +／-／+=／-= | not_assessed | 主文 147–148。满足 R18.2 的指针相减属于例外，自增减另行判断。当前原始 R18.4 五个位置均是采用 queue.c 的环形存储运算，保留已记录的 Advisory 违反和表示理由。#556 承接，归 #443，须独立核对项目补丁和当前作用域的所有四类运算符与消费者。 |
| R18.5 | Advisory | 对象声明的指针层数 | not_assessed | 主文 148–149。按 typedef 替换及数组形参调整后计算连续 pointer declarator；函数与数组边界分段处理。#556 承接，归 #443，逐声明解析 RTE 回调、Can_TxSinkType 和 OS native handle 的真实类型层次。 |
| R18.6 | Required | 自动对象地址的存续范围 | not_assessed | 主文 150＋AMD4.24。开启 ErrorHook 参数访问时，Os_ErrorResult 复制含自动输出地址的参数到 TLS，Os_ErrorContextSave 再转存至静态 ISR frame。沿既有 ErrorHook 风险处置保留 Required 违反及清空证明，正式义务开放。#556 承接，归 #443，补全部地址副本、owner 和线程退出证据。 |
| R18.7 | Required | flexible array member 声明 | satisfied | 主文 151，AMD2.27 追加 C11；同号入口的 Struct／[] 模型另以实际声明独立补齐。当前物理及展开的结构成员均无 flexible array member；空 [] 为 tasks／counters／configured_alarms、通信路由／PDU 配置、task_name 和 fatal 文本等由初始化器推导容量的独立数组，数组形参另按声明角色区分。Os_ErrorParametersType 当前为有界结构，Dcm／CAN 缓冲为固定成员容量。 |
| R18.8 | Required | variable length array 类型 | satisfied | 主文 152–153＋AMD4.25–30 区分 VLA 与 R18.10。四 profile 双目标的数组声明各维只见字面量、整数常量组合、枚举常量或推导容量。Arti_Types.h 的 char[条件?1:-1] 使用 sizeof 固定类型，实际 LP64／LLP64 选正界；Linux 全部 228 个展开另以 -Werror=vla 编译。 |
| R19.1 | Mandatory | 对象赋值／复制的重叠 | not_assessed | 主文 153–154＋TC1。当前 memcpy 源落在 Com_SendSignal／Com_ReceiveSignal 与 Os_TargetPostInput；含指针的 ErrorHook 结构复制及 Dcm 各缓冲另属赋值／复制全集。#556 承接，归 #443，须按别名、起点、长度证明不重叠或同型精确重叠／memmove 例外，Mandatory 保持阻塞。 |
| R19.2 | Advisory | union 的声明和实际用途 | satisfied | 主文 154–155 约束 union 关键字，同号入口逐 semantic union 报告。独立核对物理 token、宏和全部实际展开，自有／生成／应用与项目 patch 均无 union 声明或关键字用途；0015-typed-port-state.patch 已移除项目引入的 union，当前 Os_ErrorParametersType 按服务分开的结构成员由 Save／Restore 和 getter 消费。SDK 的 LARGE_INTEGER／CONTEXT 等原定义沿采用组，字段使用没有在项目位置生成 union 关键字。 |
<!-- misra-checker-obligations-end -->
