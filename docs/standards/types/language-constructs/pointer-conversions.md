# 函数指针与空指针常量

[#544](https://github.com/g1331/autsaro/issues/544) 的本批证据取自
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

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R11.1 | Required | 函数指针的显式与隐式转换 | satisfied | 双平台四 profile 的兼容原型和调用约定逐项回查；`(void)sink` 按 Exception 2，空指针常量和自身函数设计符分别按 Exception 1／3。 |
| R11.9 | Required | 整数空指针常量的形式 | satisfied | 双平台四 profile 的类型化指针消费者使用 `((void *)0)`；`Os_InterruptVectorTable` 的 `{0}` 按 TC1 聚合例外。 |
<!-- misra-checker-obligations-end -->
