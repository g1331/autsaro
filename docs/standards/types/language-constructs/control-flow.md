# 控制流、调用与对象生命周期的检查边界

输入、固定工具和责任沿[语言与词法](lexical.md)。同号入口为固定 addon 的
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
新增构造、宏／typedef、producer、目标或 SDK 集成变化时须重新评估。

本批结算 8 项 satisfied，其余 25 项保留具体恢复条件；全组为
28 satisfied、3 not_applicable、113 not_assessed。本篇加上既有 R15.5 共 9 项满足。
本组当前输入绑定沿[声明篇](declarations.md#当前输入重评)，未评估项继续由 #443 承接。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R14.1 | Required | 浮点循环计数及控制 | satisfied | 主文 §8.14／117 的 counter 须为标量、单调变化并参与退出决定；同号入口仅识别 for 及有限 while 模式。独立核对当前物理文件与实际展开，浮点仅 Platform_Types 的 float32／float64 typedef、portFLOAT／portDOUBLE 定义和 Linux Os_StackLinux.c 的 altstacks[].storage 的 long double 数组／sizeof；没有浮点值运算、标量或成员计数。备用栈由 sigaltstack 按字节使用，所有实际计数均无 essentially floating 来源；SDK 内部沿 §6.7／采用责任。 |
| R14.2 | Required | for 的三个子句与循环体 | not_assessed | 主文 118–119＋TC1 第一子句修正／AMD2.66–67；函数调用可用于设定唯一 counter，但循环体所调用函数也受约束。同号入口 TODO 漏函数修改。当前 BSW 数组遍历、Os_Stack 的 stacks 枚举及 patch 链表循环可定位；#443 恢复须逐 for 收齐三个子句、Boolean flag、地址逃逸及被调用函数的写集合，证明 counter 在体内无直接／别名更新，第二／三子句没有其他被体内修改的对象。 |
| R14.3 | Required | 控制表达式的 invariant 性质 | not_assessed | 主文 119–121＋TC2.27–28；无限循环例外和 essentially Boolean 的整数常量假 do-while 分开，运行时矛盾式不享后者。核心 alwaysTrue／alwaysFalse／compareValueOutOfTypeRangeError 依赖值流。xTaskIncrementTick 的无符号 tick 回绕沿 essential-types 原裁决，OS 的永久服务循环与宏 do-while 各有用途；#443 恢复须逐 if／循环／switch／?: 证明可变输入、完整取值域或精确例外，覆盖配置常量和回调写入。 |
| R15.1 | Advisory | 实际预处理后的 goto | satisfied | 主文 122，AMD2.27 只追加 C11；同号入口逐 semantic goto 报告。物理自有／生成／应用文件、补丁最终项目行及实际 GCC 展开均无 goto 关键字，另区分注释、字符串和宏 replacement；主机清理使用显式分支和返回，FreeRTOS 原行沿采用组。独立词法枚举补足工具模型，当前没有需保留的项目跳转。 |
| R15.2 | Required | goto 目标的前向关系 | satisfied | 主文 122–123 要求同一函数中向后声明的 label，同号入口 findGotoLabel 仅向前查找。当前全部自有／生成／应用及项目 patch 的原 token 和实际展开没有 goto，故没有须比较的源序／目标标签对；与 R15.1 共用经来源核验的构造全集，宏展开也未产生跳转。 |
| R15.3 | Required | goto 与 enclosing／switch clause | satisfied | 主文 123–124＋AMD2.68；switch-clause 按 block 处理。同号入口检查 enclosing scope，但其 switch-clause 状态机不能单独证明符合。当前项目原 token 及实际展开没有 goto，故没有跨块或跨 clause 的跳转；该构造缺席证据同时覆盖宏、生成消费者与项目 patch，保留原 checker 结果。 |
| R15.4 | Advisory | 循环的 break／goto 终止点数量 | not_assessed | 主文 125–126；同号入口递归数 break／goto，未按 goto 的真实目标排除不退出循环者。当前无项目 goto；xTaskIncrementTick 延迟链循环两个 break 分别保护空链与未到期队首，沿 OS 原篇保留真实 Advisory 违反和理由。#443 恢复须逐当前循环区分内层 switch／loop 的 break，核对已有实例的当前位置及所有多出口，再按本组可消费状态整改或接续处置；R15.5 GRP 不覆盖本条。 |
| R15.5 | Advisory | 自有 C 的函数退出路径 | satisfied | [ADR 0025 GRP](../../../adr/0025-misra-layered-scope.md#grp指南重分类计划)已将本条对自有 C（含生成 C、主机层及项目补丁）Disapplied；本行消费既有重分类决定，固定 addon 的非顶层 return 告警照常保留。资源回收、输出和返回语义继续由各自规则与实际消费者核查，采用原件沿供应方责任。 |
| R15.6 | Required | iteration／selection 的复合语句体 | not_assessed | 主文 127–129＋TC1 示例；else 后直接 if 有原文例外，do-while 的 while 尾部不是另一个循环体。同号 raw 状态机没有完整展开语法。Os_Atomic 软件屏障曾因开放式 while 宏修复，当前为复合体；#443 恢复须逐实际 if／else／for／while／do／switch 配对完整体，覆盖 SDK 宏的自有使用点和 patch 宏，核对原 token 与展开结构并处理所有例外。 |
| R15.7 | Required | if／else-if 链的最终 else | not_assessed | 主文 129＋TC1 明确函数调用算副作用／TC2.29 仅改 R16.4 引用。固定入口只找部分规范化 Else scope。CanTp 的接收／流控链、Os_HookServiceAllowed 的 phase 分派和生成模式链可定位；#443 恢复须独立配对所有 else-if 链，逐最终 else 核对副作用或原注释；实际预处理消去注释，须回到源宏／分支核实，普通单 if 无本条最终 else 义务。 |
| R16.1 | Required | switch 的规范结构 | not_assessed | 主文 130–131 的完整 switch 文法已看原页，TC1 改 opt 字体、AMD2.69 扩语言措辞。入口按 break 状态近似。当前 Ecu_StatusName、Os_Arti 的 event／hook／service、Dcm_PublicationType 与 Os_Time alarm action 都可追溯；#443 恢复须对全部实际 switch 按 case 标签组、clause 与最后 break 独立解析，含 patch／宏嵌套，不能仅借 R16.2–R16.7 局部性质结算。 |
| R16.2 | Required | case／default 的所属 switch | not_assessed | 主文 131 同时覆盖 case 和 default；固定入口仅检查 case.scope。Ecu_StatusName、Os_Interrupt operation 及 Dcm publication 的标签在 switch 主体；#443 恢复须完整枚举当前 case／default 的最近 compound-statement，核对嵌套 switch、宏和项目 patch，明确每个标签的直接所属体。 |
| R16.3 | Required | 各 switch clause 的终止 | not_assessed | 主文 131–132 要求每个非空 switch-clause 最后一条为无条件 break；相邻空标签可共享 clause。固定 raw 状态机还接受 return、throw、fallthrough 注释／属性，这些接受形式没有本条通用例外。当前 Ecu_StatusName 和 Dcm publication 均显式 break；#443 恢复须逐全部 clause（含复合体和 patch）证明末尾 break 及其无条件性，发现真实遗漏沿 producer 修正。 |
| R16.4 | Required | 每个 switch 的 default | not_assessed | 主文 132–133 除 default 标签，还要求其终止 break 之前有语句或注释；固定入口只找 default。Ecu_StatusName 的 INTERNAL 赋值与 Dcm publication 的终止分支可定位；#443 恢复须逐全部实际 switch 同时核对 default 与分支内容，空操作理由从物理源／宏取得，补齐展开后丢失的注释和 patch。 |
| R16.5 | Required | default 的首尾位置 | not_assessed | 主文 134 约束 default 为首个或最后一个 switch label；固定入口以花括号／case 近似查找。Os_Arti 及 Ecu_StatusName 当前 default 位于末端；#443 恢复须按每个实际 switch 的直接标签序列核对首尾，剔除内层 switch 标签并覆盖宏生成标签、共享空标签及 patch。 |
| R16.6 | Required | switch clause 的最少数量 | not_assessed | 主文 135 至少两个 switch-clause；连续 case／default 共享一条执行路径只算一个。固定入口按 break／return／throw／noreturn 数量计数。Ecu_StatusName 的各 name 赋值与 Dcm publication 的各 effect 是独立路径；#443 恢复须逐全部实际 switch 独立按语法分组、计数并连接消费者，避免把嵌套退出或共享标签重复计数。 |
| R16.7 | Required | switch 控制表达式的 essential 类型 | not_assessed | 主文 136 约束 essential Boolean；同号入口 isBoolExpression 的数值宏模型仍有局限。EcuStatus、Os_ArtiTaskEventType、Dcm_PublicationType.kind、alarm->action 的配置类型 及 eTaskState 可定位；#443 恢复须列齐所有控制表达式，递归展开 typedef／宏并按 D.6 核 essential 类型，含命名 enum、条件式和 patch，普通整数的 C 类型不足以证明。 |
| R17.1 | Required | 变参设施的声明和使用 | satisfied | 主文 136–137＋AMD2.70、AMD3.65–66；当前义务同时禁止 #include stdarg.h 和 va_list／va_arg／va_start／va_end／va_copy。固定入口遗漏仅包含头及内建拼写，另独立查物理 include／宏和实际展开的 __builtin_va_*／__gnuc_va_list；项目原件、生成／应用及 patch 无此包含或设施用途。HostBatch 的 snprintf／sscanf 是普通库调用，未自行遍历变参；stdio 原头内部声明按 §6.7，非 ISO provider 内部沿采用责任。 |
| R17.2 | Required | 直接／间接递归的完整调用图 | not_assessed | 主文 137 禁直接及间接递归；固定入口仅串联同配置已解析函数，遗漏未知函数指针／跨 TU。Det_Invoke→hook、Os_ErrorResult→ErrorHook、RTE Runnable／服务和同步 ISR 重入均有真实调用边；#443 恢复须逐 HostBatch／probe 建完整目标绑定图，核对动态注册、错误回调和所有可达环路及已有重入屏障，资源有界本身不提供递归例外。 |
| R17.4 | Mandatory | 非 void 函数的返回路径 | not_assessed | 主文 138–139＋AMD2.71–72／TC2.30：C99 main 可省末尾 return，其他非 void 退出须带表达式。核心 missingReturn 依赖局部 CFG／noreturn 模型。Ecu_StatusName 有最终 name 返回，Os_HostCreateActor 有成功／失败路径，Shutdown／GuardService 连接平台终止；#443 恢复须逐当前非 void 定义核对全部正常出口和真实不可返回路径，含宏、间接调用和 patch；Mandatory 继续阻塞。 |
| R17.5 | Required | 数组形参的实际可访问范围 | not_assessed | 主文 139–140＋TC2.31–32／41／43 为 Required；指定数组容量是每次调用从实参位置起的最小承诺。核心 argumentSize 只处理已知尺寸。Can_TxSinkType 的 data[8]、CAN payload、应用 DID 缓冲及间接 sink 各有消费者；#443 恢复须按全部数组形参声明／定义／typedef 连接每次调用的起点和剩余元素数，未指定尺寸另核长度／sentinel，局部实际访问较少不减少声明容量。 |
| R17.6 | Mandatory | 数组参数声明的 static 形式 | satisfied | 主文 140–141＋AMD2.73–74，C99 Mandatory；禁止数组形参方括号内任意位置的 static。同号入口只匹配 [ static；独立平衡全部实际方括号并检查 const／volatile／restrict 等顺序，同时查物理声明、宏和 typedef，当前项目／生成／应用及 patch 无 static 数组形参。Can_TxSinkType 的 data[8] 与所有实际原型不含该容量承诺关键字；原 SDK 声明沿采用责任。 |
| R17.7 | Required | 非 void 返回值的使用或显式丢弃 | not_assessed | 主文 141–142 允许显式 cast 到 void 使用返回值；错误处理仍由 D4.7 约束。同号入口依赖 executable scope、AST parent 和已知返回类型。当前 SchM_Switch、pthread／Win32 调用及 Can_TxSinkType 的返回有各自用途；#443 恢复须列齐直接／间接调用返回类型和 parent，覆盖宏、显式丢弃及未知模型，逐调用连接赋值、比较、返回或 void cast，另保留错误处理责任。 |
| R17.8 | Advisory | 函数形参对象的修改 | not_assessed | 主文 142 约束形参对象本身；*p／p[i] 写所指对象与 p=／++p 分开。同号入口只识别赋值／增减左侧的 isArgument。0023-preserve-parameter-values.patch 已将原内核参数推进改为局部量；#443 恢复须逐当前形参追地址别名、被调用写方和复杂左值，核对全部直接及间接修改并连接调用消费者，已有 patch 不覆盖其余函数。 |
| R18.1 | Required | 指针算术结果所在数组范围 | not_assessed | 主文 143–144＋AMD2.75–76／TC2.33–34；包括下标、增减和复合赋值，允许 one-past 构造但禁止解引用，字符表示按 sizeof(T) 定界。核心 pointerOutOfBounds 依赖已知对象和值流。0020-bounded-queue-front.patch 已先选择头部回绕再减 itemSize；#443 恢复须逐当前数组／子数组、序列化缓冲、映射栈与宏算术证明基对象、偏移和最终访问，不能用修复单个队列概括全部。 |
| R18.2 | Required | 指针相减的对象归属 | not_assessed | 主文 145 只允许同一数组（含 one-past）的两指针相减；核心 comparePointers 不能完成别名来源证明。Os_StackSavedContext／publish_fault 的 record-stacks 沿 OS 原篇连接 Register／Observe 的私有记录来源和 21 元素数组；#443 恢复须列齐其余实际减法，逐个证明两端基对象、存续和 ptrdiff_t 可表示范围，包含 patch／宏及复杂表达式。 |
| R18.3 | Required | 指针关系比较的对象归属 | not_assessed | 主文 145–146＋AMD4.52 将对象改为表达式，仍约束 >／>=／<／<= 的同对象关系；结构／union 成员按原标准关系核对。核心 comparePointers 只提供候选。Linux 的 record 范围比较依赖 Register／Observe 已保证 stacks 来源，任意外部地址比较本身无验证作用；#443 恢复须追齐全部实际比较两端来源、成员顺序及最后消费者，不能借用 R18.2 的减法证明。 |
| R18.4 | Advisory | 指针的 +／-／+=／-= | not_assessed | 主文 147–148 明确允许满足 R18.2 的指针相减，++／-- 不属于本条四个运算符。同号入口仅查有 valueType 的 +／-／+=／-=。prvCopyDataToQueue 的 itemSize 步长沿平台原篇保留真实 Advisory 违反；#443 恢复须按当前实例核对全部指针算术与宏／typedef，区分允许减法、下标和建议性保留，并沿实际缓冲消费者整改或接续处置。 |
| R18.5 | Advisory | 对象声明的指针层数 | not_assessed | 主文 148–149 按 typedef 替换后的连续 pointer declarator 计数；数组形参衰减增加一层，数组／函数边界会中断连续层数。同号入口向前数星号遗漏这些类型关系。Can_TxSinkType、Os_HostHandleType 和 RTE 回调／指针数组可定位；#443 恢复须递归展开全部声明类型并按层级分段计数，逐个连接对象和消费者，不能将声明中总星号数当规范层数。 |
| R18.6 | Required | 自动对象地址的存续范围 | not_assessed | 主文 150＋AMD4.24 同时约束 automatic 和 thread-local 地址，赋值／初始化／内存复制均纳入。核心 danglingLifetime 依赖逃逸与库模型。Os_ErrorContextSave／Restore、Os_ArtiCallerSave、同步 ISR 输出借用及 TLS 快照沿原类别保留；#443 恢复须按每个地址复制追到最后消费者、退休清零、actor／线程退出和所有异步保存，重核当前生命周期及 Required 正式义务。 |
| R18.7 | Required | flexible array member 声明 | satisfied | 主文 151，AMD2.27 追加 C11；同号入口的 Struct／[] 模型另以实际声明独立补齐。当前物理及展开的结构成员均无 flexible array member；空 [] 为 tasks／counters／configured_alarms、通信路由／PDU 配置、task_name 和 fatal 文本等由初始化器推导容量的独立数组，数组形参另按声明角色区分。Os_ErrorParametersType 当前为有界结构，Dcm／CAN 缓冲为固定成员容量；未把独立不完全数组误认柔性成员。 |
| R18.8 | Required | variable length array 类型 | not_assessed | 主文 152–153＋AMD4.25–30 将 VLA 与另列 R18.10 的 variably-modified pointer 分开。同号入口只查 local array 首维，未知常量跳过。altstacks 的 sizeof(long double) 容量、Os_Atomic 的 char[布局谓词?1:-1] 和生成任务数组均为编译期量；#443 恢复须按当前全部维度、typedef／参数和 sizeof 类型名证明整数常量表达式及正界，含宏展开，不能只借 checker 零告警结算。 |
| R19.1 | Mandatory | 对象赋值／复制的重叠 | not_assessed | 主文 153–154＋TC1 只更正例子；允许完全重叠且兼容类型的赋值，以及 memmove 的部分／完全重叠。核心 overlappingWriteUnion／overlappingWriteFunction 仅覆盖有限对象。Os_ErrorContextSave／Restore 的结构复制、CanTp payload 和 Dcm 请求／响应复制可定位；#443 恢复须逐实际赋值／库复制按别名、起点、长度证明区间关系和精确例外，包含回调输入、宏和 patch，Mandatory 继续阻塞。 |
| R19.2 | Advisory | union 的声明和实际用途 | satisfied | 主文 154–155 约束 union 关键字，同号入口逐 semantic union 报告。独立核对物理 token、宏和全部实际展开，自有／生成／应用与项目 patch 均无 union 声明或关键字用途；0015-typed-port-state.patch 已移除项目引入的 union，当前 Os_ErrorParametersType 按服务分开的结构成员由 Save／Restore 和 getter 消费。SDK 的 LARGE_INTEGER／CONTEXT 等原定义沿采用组，字段使用没有在项目位置生成 union 关键字。 |
<!-- misra-checker-obligations-end -->
