# 指针转换

[#544](https://github.com/g1331/autsaro/issues/544) 的前两批证据取自
[master run 44835837239](https://github.com/g1331/autsaro/actions/runs/44835837239)
（`f265ded8691fdc55c5286763439d000f89203d98`）。Linux 和 Windows 各核对四个 C99
profile（standard-ecu、user-application、component-pipeline、no-diagnostic），覆盖
HostBatch 和 probe 的 888 个程序 TU，按 GCC 依赖复用其中 452 份独立展开。
以带行标记的 GCC 展开作输入，用 Clang 的 LP64／LLP64 C99 语法树枚举转换及消费者，
同时回查物理源码和原始 Cppcheck XML。Windows 的语法树输入省略第三方 GCC x86
intrinsic 头的内容，保留目标源码、使用点展开及 Windows SDK 的接口声明。
按 ADR 0030 的 GCC 包含闭包并集判定集成头，按 `patchedLines` 区分 FreeRTOS 项目补丁
和采用原件；SDK 原件和主机平台实现按各自范围记录。

## 函数指针的原型与调用

2013 主文 R11.1 印刷 93–95／PDF 101–103 的三项例外分别适用于空指针常量、
转成 `void` 以及函数设计符转为同类型指针。AMD2.27 补 C11 适用性，AMD2.58
修订第三项的说明，其他修订对本批 C99 判据无改动。逐个检查转换、取址与间接调用的
返回类型、参数／限定符和适用的调用约定。`Det_Callouts` 的 `Ecu_DetRuntimeCallout`
以 `Std_ReturnType (uint16, uint8, uint8, uint8)` 进入 `Det_ErrorHookType` 表，由
`Det_ReportRuntimeError` 按相同原型调用。RTE 的 `Process_Transform`／
`Ingress_ReadData` 通过生成头中的原型直接调用。BSW 的 CanIf、Com、ComM、BswM、
Dcm、PduR、LSduR 通过生成配置中的具类型字段调用对应函数。

OS 配置 task／ISR 和生命周期 hook 使用各自声明的返回类型与参数，向量表保存
`uint32 (void)`，启动记录保存 `void (void *)`。Windows 的线程入口使用
`DWORD (void *)` 及 SDK 的 `LPTHREAD_START_ROUTINE`，Linux 的启动入口继续沿其
声明匹配 provider。项目内核补丁传递 `TaskFunction_t` 时沿 `void (void *)` 接缝，
采用内核和 ADR 0030 主机实现按实际归属另记。除三项例外，当前目标范围内的
函数指针保持兼容原型，没有经对象指针或整数转接的调用。

八份原始报告均在 `generated/src/Ecu_Execution.c:7:5` 报 `(void)sink`，各覆盖两个程序。
`Can_TxSinkType` 只在此转为 `void`，使用既有 [Exception 2 裁决](README.md#函数指针的显式丢弃)。
发送继续经 `Ecu_TargetEnqueueTransmit`，这处丢弃无后续指针消费者。

## 空指针常量的使用点

2013 主文 R11.9 印刷 101–102／PDF 109–110 要求整数空指针常量在指针赋值、比较及
条件分支中来自 `NULL` 的完整展开，并允许 `(void *)0`。主文另以实参例说明；本批
同时核对指针返回。TC1 印刷 7／PDF 13 补充含指针聚合体或联合体的 `{0}` 例外；
AMD2.27 追加 C11，其他修订对本批 C99 判据无改动。

按实际目标类型分开指针和数值零，回查 `NULL_PTR` 与 `NULL` 的完整 `((void *)0)`
展开。覆盖对象／函数指针的初始化、赋值、两个比较方向、实参、返回及 `?:` 两臂
（包括未求值分支）。`Os_Error` 在进入 hook 路径前比较 `Os_StackCurrent()` 返回的
栈指针，CanTp 的 `PduInfoType` 元数据成员用空指针初始化。`CanTp_Perform` 的空重试
指针交给 PduR，项目 `tasks.c`／`queue.c`
补丁的空值沿真实 guard 消费。目标范围内唯一未以 `void *` 形式出现的显式整数空指针
常量是 `Os_InterruptVectorTable` 的 `{0}`，应用 TC1 聚合初始化例外。八份原始报告
没有 R11.9 诊断；判定来自指针消费者和展开。

原始 `passed=false`、`completeCompliance=false` 和项目验收 `blocked` 保留。

## 对象指针的转换与限定符

第二批用同一提交的 452 份 GCC 展开（Linux 228、Windows 224），按 C99 的 LP64／
LLP64 模型读取显式 cast 及初始化、赋值、实参、返回、比较与条件式中的隐式转换。
按使用点回查两个程序的 16 份原始 XML 和实际文件。Windows 的辅助解析保留 SDK
接口声明，从输入移除 GCC 私有 x86 intrinsic 头的正文，全部 452 份解析完成。
`patchedLines` 保留项目补丁的评估责任，FreeRTOS 未修改的行和 SDK 原件沿各自的
来源记录，主机仿真用点按 ADR 0030 和八个配置的 GCC 包含闭包判定。

**R11.2。** 主文印刷 95–96／PDF 103–104，TC1 按指向类型去除外层限定符判断；
检查转换点可见的 struct／union 定义和数组边界，`void *` 作为独立类型处理。
`TaskHandle_t` 指向的 `tskTaskControlBlock` 在范围内转换处已经由 `task.h` 完整定义，
`StaticTask_t` 与它沿同一 tag。Linux 的 `Os_HostHandleType` 仍可作为不完整类型，
范围内涉及转换的只有 `Os_Backend.c` 七处和 `Os_TimeClose` 一处空指针初始化或比较。
GCC 展开为 `((void *)0)`，按 Exception 1。`Os_HostSetEvent` 等接缝继续传递同型句柄，
`winnt.h` 内部的不完整 `_TEB` 与 `_ACTIVATION_CONTEXT` 转换归 SDK 原件。

**R11.3。** 主文印刷 96–97／PDF 104–105，AMD2.59、TC2.21、AMD4.7–9；
按去掉被指向类型的外层限定符后的类型比较，保留内层限定；字符访问例外要求源对象
无 `_Atomic` 限定。八个 `os/FreeRTOSConfig.h` 均将 `configUSE_MINI_LIST_ITEM` 设为 `0`。
`prvOsInsertReadyTask` 中的 `listGET_END_MARKER` 把 `&xListEnd` 转成 `const ListItem_t *`，
实际 `MiniListItem_t` 和 `ListItem_t` 同为 `struct xLIST_ITEM`；循环比较哨兵，再由
`listGET_LIST_ITEM_OWNER` 读取有效节点。自有代码两处不同对象类型的转换均在
`Os_Backend.c`。`report_and_exit` 以 `const char *` 读取 `os_backend_trace` 的字节前缀，
`Os_TargetTrace` 保留零终止位置。`xTaskCreateStatic` 的名称参数以 `const char *` 接收
配置中的 `uint8` 名称，由内核按字符复制到任务名缓冲。这两处均适用非原子字符访问例外。

**R11.8。** 主文印刷 100–101／PDF 108–109，AMD2.61、AMD4.10–14，逐指针层检查
const、volatile 和 `_Atomic`，`restrict` 沿原文说明。自有代码和项目 hunk 中需复判的
三处仅是相等关系。`task_slot` 比较 `StaticTask_t *` 与 `const StaticTask_t *`；
`prvOsInsertReadyTask` 比较 `ListItem_t *` 与 `const ListItem_t *`。两者按
[C99 §6.5.9](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1256.pdf#page=98) 比较
兼容类型的指针。`Os_BackendBeforeSelect` 按同节 §6.5.9p5 将任务对象指针转为
`const void *` 与 `current` 比较。这三处按 C99 原文判定转换方向，Clang 插入的
`BitCast` 作为定位线索。实际实参、返回和赋值逐层保留被指向类型的限定，
包括 `pxTaskOsPortStack` 返回的 `volatile StackType_t *`。两处 R11.3 字节读取使用
`const char *`，C99 范围内无 `_Atomic` 限定丢弃。

**R11.5 待处置。** 主文印刷 98／PDF 106 的空指针常量例外只覆盖空值。
`Os_Backend.c:1535,1666` 将 `native_task_start`／`task_entry` 的 `void *` 参数送进
`task_slot`。`xTaskCreateStatic` 以 `&task_memory[i]` 提供同一数组中的静态对象，
`task_slot` 用恢复的类型查找槽位。`kernel/tasks.c:8943` 的项目 FIFO hunk 从
`pvOwner` 恢复 `const TCB_t *`，内核初始化列表项时写入原 `pxNewTCB`，随后读取
`ullOsReadySequence`。这三处在当前配置有对应的存活对象、类型和对齐，仍属于 R11.5
实际转换。`Os_BackendBeforeSelect` 的比较按上节语义处理。

16 份原始 XML 无 R11.2／R11.3／R11.8 诊断。R11.5 的现有原始诊断落在采用原件
`kernel/tasks.c:4474` 和 Linux 主机实现 `Os_HostLinux.c:359`，三个自有或补丁实例均漏报。
AMD2.27 补充语言适用性，其余修订未改变本节所用 C99 判据。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R11.1 | Required | 函数指针的显式与隐式转换 | satisfied | 双平台四 profile 的兼容原型和调用约定逐项回查；`(void)sink` 按 Exception 2，空指针常量和自身函数设计符分别按 Exception 1／3。 |
| R11.2 | Required | 不完整类型指针的显式与隐式转换 | satisfied | `TaskHandle_t` 在转换处完整化；`Os_HostHandleType` 的空指针使用按 TC1 的无限定类型判据及原文 Exception 1。 |
| R11.3 | Required | 不同对象类型的指针转换 | satisfied | AMD4.7–9；`configUSE_MINI_LIST_ITEM=0` 使 `listGET_END_MARKER` 保持指向同一类型，两处 `const char *` 访问适用非原子字符例外。 |
| R11.8 | Required | 指向类型的限定符保留 | satisfied | AMD4.10–14；按 C99 §6.5.9 复判三处比较，赋值、实参及返回逐层保留限定符。`pxTaskOsPortStack` 保留 `volatile`。 |
| R11.9 | Required | 整数空指针常量的形式 | satisfied | 双平台四 profile 的类型化指针消费者使用 `((void *)0)`；`Os_InterruptVectorTable` 的 `{0}` 按 TC1 聚合例外。 |
<!-- misra-checker-obligations-end -->
