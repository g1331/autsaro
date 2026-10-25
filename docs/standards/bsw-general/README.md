# BSW General

本篇属于[标准与规范基线](../README.md)：条款处置的标记与核算方法见 [README §5](../README.md#5-条款处置与核算)。`SWS_BSWGeneral` 的义务对每个 BSW 模块成立，按模块分篇处置，标记带模块限定。LSduR、PduR、Com、Can、CanIf、CanTp、Dcm、ComM、BswM、Os、Det 与 Rte 已逐条处置；其余模块下表列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_BSWGeneral.pdf`（`4709480090ff9a612d908e8bd46edd502e2e4fa332f7a3bf93801f17cc91872b`）。

| 条款 | 义务要点 |
| --- | --- |
| `SWS_BSW_00006`、`SWS_BSW_00115`、`[SWS_BSW_00103]`、`[SWS_BSW_00170]`／`[SWS_BSW_00171]`、`SWS_BSW_00036`；附录 A.2 模块 ID | 实现源包含 `<Mip>_MemMap.h`；文件名区分大小写且不能只靠大小写区分；跨模块头文件版本检查；模块 ID：Can 80、CanIf 60、CanTp 35、Com 50、ComM 12、Dcm 53、PduR 51、Rte 2、Os 1、BswM 42、Det 15、CDD 255，LSduR 132 |

`SWS_BSW_00115` 的具体偏离记录与就近 MR12 注释沿用 [C 编码规范](../../development/c-coding-standard.md#misra-偏离注释)；已有有界冲突授权、逐项证据和独立复核按 [ADR 0023](../../adr/0023-autosar-required-conflict-disposition.md)执行。项目授权不能替代该条款的 MISRA 符合义务，原文与必要人工评估仍由 #57／#58 承接，以下各模块差距不因此关闭。

## 禁用库设施的源码核查

本节核查固定双平台四样本及 `a00b8c03` 的四个 Linux 生成样本，包括交付 C／头文件、采用内核及受控补丁结果；逐文件身份与词法收据留在 PR #371。没有发现 `setjmp.h`、`jmp_buf`、setjmp 或 longjmp。R21.4 为 Required、适用于 C99（主文印刷页168／PDF176；AMD3.67，页43／PDF51 明确同时禁止包含头文件及使用其设施）。原生 restart 的汇编与 unwinder 接口另按 OS 平台扩展核查，不能由 setjmp 设施缺席推定安全。

所查源码也没有 atof／atoi／atol／atoll、bsearch／qsort 标识符或同名宏定义／展开入口。R21.7、R21.9 均为 Required、适用于 C99（主文页169–170／PDF177–178）；TC1 修改两条 headline 的标准库表述，AMD2.80 将 R21.7 的标识符约束扩展至 C99 and later，不改变这里的缺席判断。此结论不涵盖其他转换函数、排序实现或标准库内部实现。

R21.11 经 AMD3.40–43 改为 Advisory，约束 `tgmath.h` 的包含，不能沿用主文的 Required 分类，也不能把 math／complex 头中的同名普通函数判作泛型宏。R21.12 则经 AMD3.44–48 改为 Required，并扩展至 `fenv.h` 的全部设施（AMD3 页22–23／PDF30–31；主文页171–172／PDF179–180）。所查源码没有包含这两个头，也没有浮点环境类型、函数或 FE_ 宏；未援引例外。这些规则仍适用于 C99，不能因为没有使用而写成语言不适用。

上述证据只覆盖所列源码，系统头预处理闭包和链接库内部实现仍须各自核查。signal／time、动态分配、终止函数及 errno 的实际使用不属于这里的缺席结论，沿对应模块继续评估；原始诊断与 `not_assessed` 不变。

`1e13ef66` 四个 Linux 交付样本的补充核查包括当前生成 C／头文件与同一目标、补丁身份的已分析内核：唯一 `memcmp` 入口位于采用代码 `stack_macros.h` 的宏定义中，要求 `configCHECK_FOR_STACK_OVERFLOW > 1` 且 `portSTACK_GROWTH > 0`。实际配置分别为 0 和 -1，该分支未生效；其余所查源码、指令及非活动分支没有该标识符。不能把这条保留的上游定义写成词法缺席，也不评估未选的向上增长栈变体。

R21.14 禁止用 `memcmp` 比较零终止字符串，R21.16 则限制其指针实参指向的对象类别；二者均为 Required、适用于 C99（AMD1 页4–5／PDF12–13、页6–7／PDF14–15；AMD2增加C11适用性）。所选交付源码没有生效的 `memcmp` 入口，故这里没有待判定的字符串比较或对象表示比较；这不是语言不适用，也不替代 `memcpy`／`memmove` 的R21.15、长度边界、系统头展开或链接库内部义务。

## 字符分类与库内部存储

R21.13 为 Mandatory、适用于 C99，限定 `ctype.h` 函数实参的取值域（AMD1 印刷页4／PDF12；AMD2 增加 C11）。`4d28aeb8` 双平台八份完整工件及本轮零 DLC 修复后四个 Linux 工程的交付 C／头文件，包括采用和补丁后内核，未见十四个标准字符分类／转换函数的标识符。检查包含预处理指令和非活动分支；该缺席结论不覆盖系统头展开与链接库内部，也不是语言不适用。

R21.19／R21.20 均为 Mandatory、适用于 C99，分别保护库返回的内部数据及其在后续相关调用前的有效性（AMD1 页10–12／PDF18–20；TC2.36／37，页12／PDF18）。TC2 明确 localeconv 后的 setlocale、asctime／ctime 以及 gmtime／localtime 各组的相关调用，不能只检查同名函数；AMD2 的 C11 增补不改变当前义务。上述十二份源码中只见 getenv：共同 Backend 的 `read_resource_failure`，以及 Linux 的 `read_invalid_altstack`；其余所列内部存储函数未见入口。

两个 getenv 消费者均接为 `const char *`，不写内部字符串、不保存其地址。资源失败开关先经 strtoul 转成数值，再检查 errno、首字符、尾部与范围并保存整数；end 仅用于读取解析终点。备用栈开关在长度为一时只复制字符。每处最后一次读取都在 helper 返回前完成，后续启动调用不再使用其指针；核查也包含故障测试开关启用时的真实分支，不能因普通生产构建排除它们就跳过。const 写入反例仅证明编译期保护，调用次序结论来自实际源码闭包；两者均不外推系统库内部的存储或并发契约。

## 空载荷与复制长度

R21.18 为 Mandatory、适用于 C99，要求所列 `string.h` 函数的 `size_t` 实参为正，且不超过任何指针实参所指对象的容量（AMD1 印刷页9／PDF17；AMD2 增加 C11 适用性，未改变正值条件）。这比仅检查 C 库是否允许零字节操作更严格，不能以没有越界报告代替此项义务。

`Can_TransmitPdu → Can_InternalTransmitPdu → Can_InternalHostFlush → Ecu_ExecutionTransmit → Ecu_TargetEnqueueTransmit` 允许零 DLC。驱动向下传递的局部载荷与 Target 输出槽均为八字节，Target 先检查 DLC 不超过八并清零完整输出槽，随后仅在 DLC 非零时复制载荷；这样复制长度为 1–8，而零 DLC 仍产生原有 ticket、驱动 token、发布、确认和 tick 完成。没有通过拒绝空帧来缩窄接口。

真实生成 ECU 的永久测试发送 0／1／8 字节帧，检查载荷、剩余字节清零、输出确认与 tick 完成。复制观察器覆盖普通 `memcpy` 与 Linux 强化后的 `__memcpy_chk`，后者仍转交真实 checked 函数；独立越界控制要求该函数拒绝容量不足。源码旧行为和撤销 guard 的已编译变异都被零长度观察器拒绝，删除正长度复制的已编译变异被实际帧内容断言拒绝。执行收据归 PR #371；本项只处置这一输出路径，不表示其他长度参数、采用内核或系统库内部的 R21.18 义务已全部完成。

## 逻辑右侧的持续副作用

R13.5 为 Required、适用于 C99（主文印刷页113–114／PDF121–122）。Glossary 页224／PDF232 区分被调函数自己的临时对象与返回后仍存活的对象：写调用者输出参数、修改静态对象或文件、访问 volatile 对象均可能产生持续副作用。TC1 Appendix J（页14／PDF20）要求按函数本身判断，不因当前参数或状态避开某条路径就把函数当作 pure；AMD2 增列 C11 不改变当前 C99 义务。

`CanIf_SetPduMode` 先按原配置 guard 决定是否查询 `Can_GetControllerMode`，再检查查询结果、实际 controller mode 和请求值；CAN 查询有持区及输出参数效果，不能放在 guard 的逻辑右侧。重新取得 CANIF_STATE 后的 lifetime／decision 检查和重试保持，查询仍在模块区外。`Det_ReportError` 只在配置存在时进入开发报告递归边界，再按进入结果调用 hooks，随后走原有不可返回的 halt；不能把修改线程报告状态的 helper 当作普通配置查询。

主机 CDD 的发布者先确认 owner，再取得当前模式并在本模块区外通知 ComM；其 getter 的持区和输出效果按实际闭包核查。POSIX `Can_EnterLock` 完成 once／ready 检查后才获取 mutex，初始化失败不触及原生 lock；现有故障夹具以 `UNEXPECTED_LOCK` 区分提前获取与预期 fatal，不能仅凭相同退出码认定调用顺序正确。

HostBatch 的 `number` 成功时写调用者的 at／id／dlc，这些对象的存储期尚未结束。解析依次保留 token 数量、ID 成功、DLC 成功及后续值域／长度检查，局部 Boolean 只表示前面解析是否成功；缺失 token 和前项失败时不调用后续解析，返回状态、batch 状态和输出命令的更新位置保持。数字累加临时量本身不是这一判断的依据。

ECU state／owner、输出槽、等待发布及 tick 完成路径的原子查询、事件清除和完成调用均在原条件下独立求值；第二次事件刷新使用局部 event_status，不覆盖既有 tick 状态。平台线程身份读取也保留条件调用，不依赖未建立的 pure 模型。Backend 查询 Task 状态的 critical 效果与调用者输出分别核查；断言内的 kernel 查询保留在 `configASSERT` 参数中，条件运算符只在无排队 activation 时求值，断言不求值时不额外调用 kernel。

所查 `CanTp_Receiving`／`CanTp_Starts`／`CanTp_DataLength`、`Dcm_IsAvailability`、`Det_ValidHooks`、`Os_ErrorHookConfigured`、`Os_StackCurrent`、`Os_BackendCurrentInterrupt`、`Os_BackendInterruptMaySchedule`、`task_index`、`valid_cycle` 及生成端 `Ecu_SchMTimingActive`／`Ecu_TargetCheckLifecycleContext` 只读普通对象或修改自身临时量，不能把它们仅因出现在逻辑右侧而判为违反。该判断针对这些实际定义，不外推其他 volatile 表达式、采用内核调用或同名平台接口；完整源码人工评估仍由 #58 承接，原始结果不变。

在 `5243b3ef` 的 Linux standard-ecu 冻结源码中，两个实际入口的 56 个 TU 均取得所选宏展开 AST；采用／补丁后四个内核 TU 有 68 个逻辑表达式。所列 RHS 未见直接 volatile 读取、赋值或自增减；全部调用候选再按实际定义核查。端口剩余的 `Os_BackendInterruptPriority`、`Os_BackendHandlerAllowed` 与 `Os_MailboxHandlerAllowed` 只读普通配置，后者的闭包仍为前述 handler 判定，没有获取锁或改写状态。`Ecu_TargetCheckLifecycleContext` 读取普通 TLS phase，`Ecu_SchMTimingActive` 读取普通生命周期对象。源码与 TU 身份、AST 候选及被调函数核查收据见 PR #371；此结论不代替系统头展开、其他配置和 Windows 的实际平台调用评估。

## 按模块处置

| 模块 | 处置 |
| --- | --- |
| [LSduR 与 PduR](lsdur-pdur.md) | 全部 150 条，标记带 `[LSduR]`／`[PduR]` |
| [Com](com.md) | 全部 150 条，标记带 `[Com]` |
| [Can](can.md) | 全部 150 条，标记带 `[Can]` |
| [CanIf](canif.md) | 全部 150 条，标记带 `[CanIf]` |
| [CanTp](cantp.md) | 全部 150 条，标记带 `[CanTp]` |
| [Dcm](dcm.md) | 全部 150 条，标记带 `[Dcm]` |
| [ComM](comm.md) | 全部 150 条，标记带 `[ComM]` |
| [BswM](bswm.md) | 全部 150 条，标记带 `[BswM]` |
| [Os](os.md) | 全部 150 条，标记带 `[Os]` |
| [Det](det.md) | 全部 150 条，标记带 `[Det]` |
| [Rte](rte.md) | 全部 150 条，标记带 `[Rte]` |

## 已知差距

- **`SWS_BSW_00036` 跨模块头文件版本检查**：当前静态／生成公共头取得所属 owner 的 release，适用直接外部头逐边检查 major/minor；独立编译拒绝见 `core/tests/native/integration/release_contracts.rs`。正式身份与完整发布信息仍归 #59。
- **当前交付说明**：选定实现的 Host_Implementation.arxml 从实际交付公开头和不可变计划描述入口、独占区及调度；MODULES.md 导航逐模块手册、API 与同源配置说明。服务端口支持边界仍由 #187／#195 跟踪，版本身份仍由 #59 跟踪。
- **Symbolic Name Value**：提供方的配置头按 TPS_ECUC_02108 唯一导出所选句柄，生成配置包含提供方头并使用这些符号；LSduR 保持由路由计划合成的配置边界。

## Det 报告身份

`runtime/bsw/include/` 的各模块主头提供 `<MIP>_MODULE_ID`，取值按 R24-11 `SWS_BSW_00256` 与附录 A.2 独立核对。当前报告路径为 Can、CanIf 与 CanTp；服务 ID 在各自主头按原 API 表命名：

| 模块／路径 | 服务宏后缀和值 | 原文 API 表 |
| --- | --- | --- |
| Can 丢帧轮询 | `SID_MAIN_FUNCTION_READ=0x08` | `SWS_Can_00226` |
| CanIf 发送拒绝 | `SID_TRANSMIT=0x49` | `SWS_CANIF_00005` |
| CanIf 接收 DLC 拒绝 | `SID_RX_INDICATION=0x14` | `SWS_CANIF_00006` |
| CanTp 定时处理 | `SID_MAIN_FUNCTION=0x06` | `SWS_CanTp_00213` |
| CanTp 接收处理 | `SID_RX_INDICATION=0x42` | `SWS_CanTp_00214` |
| CanTp 取消发送／接收 | `SID_CANCEL_TRANSMIT=0x4a`／`SID_CANCEL_RECEIVE=0x4c` | `SWS_CanTp_00246`／`SWS_CanTp_00257` |

CanTp 的内部 `CanTp_Report` 保留服务参数与零错误过滤；Det 入口签名、通知与终止行为不变。`bsw-contracts` 消费者按原文数值断言头宏与实际通知，预期不引用生产宏。宏为增量公开信息，不改变函数 ABI、配置或类型布局；重新生成交付包以取得新头。完整 published information、正式 vendor 身份与 API 出口仍由 #59 承接。

## 发布版次与头文件 owner

R24-11 MOD 中 Arti、BswM、Can、CanIf、CanTp、Com、ComM、Dcm、Det、LSduR、MemMap、Os、PduR、Rte 的 `ECUC-MODULE-DEF/ADMIN-DATA/DOC-REVISIONS/DOC-REVISION/REVISION-LABEL` 均为 `4.10.0`。原件 ZIP 身份见标准基线；数字来自这些 revision，不能从 R24-11 的日期或 Cargo 软件版本换算。共享类型保留既有 `STD_TYPES`、`PLATFORM`、`COMSTACKTYPE` 前缀。

模块主头、callback、Cfg 与私有头从本模块 `_Version.h` 取得 release；RTE 生成的应用契约、`Rte_*` 与 `SchM_*` 属 RTE，`Rte_Dcm_Type.h` 的模式类型仍由 Dcm 提供。所有 `_MemMap.h` 由 Memory Mapping 产生，引用 `MemMap_Version.h`；该身份头的 guard 不包裹 START/STOP 处理。契约包与完整 ECU 使用同一组资产及 emitter，不复制另一组输出常量。host helper、ISO C／平台 API 与 FreeRTOS 保持实际角色，不伪造 AUTOSAR 身份。

00036 的拒绝只比较 major/minor；revision-only 与跨供应方 vendor／SW 差异可接受。Det、LSduR、Platform、Comtype 原文 NA 对 SRS_BSW_00004 的排除保留为消费者边界；其他适用模块仍须检查导入它们的头。独立 TU 首 include、边隔离毒化与真实源编译分别检验发布值、每条检查的诊断及编译闭包。ComM_00418 的源／头 release 与 software 三分量分别校验；源端保留独立快照，头单改／缺失拒绝，同步升级成功。正式身份仍由 #59 承接。

## 软件发行版本

自有 BSW、OS、共享类型、MemMap 与 RTE 生成器实现统一发行，初值 **0.1.0**；唯一数值来源是交付的 `runtime/bsw/include/Autsaro_Version.h`。各所属头通过既有 owner 头发布 `<MIP>_SW_MAJOR_VERSION`／`SW_MINOR_VERSION`／`SW_PATCH_VERSION`，Rust 构建读取同一头供 BSWMD 和模块手册使用。Cargo 包版本独立，不再隐式决定 BSW 版本。这个共享头不是另一个 AUTOSAR 模块，不声明 module/vendor ID，也不产生跨模块软件版本相等义务。

版本由维护者在发行变更中更新，按 `SWS_BSW_00162`：双向兼容修复递增 patch、向下兼容扩展递增 minor、不兼容接口变化递增 major；递增高位时低位归零。三个分量均在 uint8 范围内（`SWS_BSW_00256`），构建拒绝缺失、重复或越界的 owner 字段。初值保留已有 Can 与实现描述的声明；它不表示原型已符合或已经稳定。ComM 源内的软件快照随发行同步更新，不能替换为头宏别名，否则 `SWS_ComM_00418` 比较成为恒真。

ARTI 工具实现独立发行，初值 **1.0.0**，唯一 owner 为 `Arti_Version.h`；API、BSWMD 和手册消费该来源。RTE BSWMD／手册的配置版本仍按 `SWS_Rte_05184/05185/05186` 来自输入 DOC-REVISION，第四段只用于文档；头中生成器实现版本不是该配置版本。SchM build-version 的双输入语义见 [SchM](../schm.md)。

正常测试在 `descriptions.rs` 检查独立字面量的 BSWMD／manual，在 `release_contracts.rs` 编译所有实际公共头／重复 MemMap、逐分量破坏 ComM 一致性、接受同步升级，并运行 ARTI 哨兵消费者及逐赋值删除反例。版本 owner 的传递测试允许合法同步升级，不把固定初值当不可升级的协议。

这里只闭合软件发布来源；`SWS_BSW_00059/00256/00161` 的正式 vendor 部分仍欠缺，保留原有差距标记。项目尚未申请 vendor ID，不添加虚构 VENDOR_ID，不启用依赖真实身份的版本 API，不替代 RteCodeVendorId 的使用方身份。

## 源码语言边界

注释／字面量、初始化层级与实际求值见[源码语言边界](language.md)。限定的源码、配置与原始分析状态分别保留。

R21.21／R21.24均为Required（AMD2.83印刷18／PDF25；AMD3.50印刷26／PDF34），分别禁止system及rand／srand；当前四份callback-parameter样本与双目标补丁后内核无这些标识符或宏用途。R21.22为Mandatory、R21.23为Required（AMD3.49印刷24–25／PDF32–33），约束tgmath操作数类别及多参数类型；同一语料对固定glibc头的122个公开宏名无普通token或指令用途。四条在当前C99均适用；这里是调用构造缺席，保留SDK展开、provider内部及其他配置边界。
