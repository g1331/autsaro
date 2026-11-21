# 指针相减、关系比较与声明层数

本批接续 [#556](https://github.com/g1331/autsaro/issues/556) 和
[控制流篇](control-flow.md)。固定 MISRA C:2012 Third Edition、AMD1–4、TC1–2
七份原件均与授权资料库的 SHA-256 清单一致。R18.2、R18.3、R18.5 分别见主文
印刷 145、146、148–149 页（PDF 153、154、156–157）。AMD2.27 增补 C11
适用性和相关未定义行为索引。AMD4.52（印刷 34／PDF 41）把 R18.3 的对象改为
表达式。其余修订不改变这三条的 C99 判定。

## 分析输入与来源

以 master `656ebb2b744435835c7c43a08b4e34e61c7b8312` 的
[Development checks](https://github.com/g1331/autsaro/actions/runs/44851605791)
为证据。Linux 和 Windows 各选四个 C99 profile，包含 HostBatch／probe
的 888 个程序 TU。452 份独立 GCC 展开由 Clang 18.1.3 按 LP64／LLP64 辅助解析。
逐 AST 取实际操作数类型和完整声明类型，再回查 GCC 行标记、封存源码与
`patchedLines`。Windows 辅助解析保留项目和 SDK 接缝声明，省略 GCC 专用
x86 intrinsic 头的正文。ADR 0030 归属按两个程序、八份 GCC 依赖回执的并集，
FreeRTOS 项目 hunk 与原件行分开。16 份原始 XML 中三条规则均无诊断。

## 相减与关系比较

- **R18.2。** 按减号两侧经数组转换、typedef 展开后的类型筛选。
  同一数组元素与末尾后一位置可参与相减，差值须可由 `ptrdiff_t` 表示。
  八个实际配置均无指针与指针相减。其中 `kernel/queue.c:2396` 是
  `pcTail - uxItemSize`，右侧是无符号的单项字节数。两平台的栈 provider
  现用显式记录下标查找，旧 `record - stacks` 留在历史记录。
  栈地址经 `uintptr_t` 转换后产生的差值是整数运算。
- **R18.3。** 按 `<`、`<=`、`>` 和 `>=` 枚举指针表达式。
  主文允许同一数组（含 one-past）及规定的同一结构体成员关系。
  项目 0015 补丁在 `kernel/tasks.c:1945–1946` 展开两个
  `configASSERT`，比较 `pxPortInitialiseStack` 的返回值与原栈底和
  初始栈顶。常规任务、bootstrap 及 idle 分别由 `Os_Backend.c` 的
  `xTaskCreateStatic`、`vApplicationGetIdleTaskMemory` 提供独立的
  `task_buffers` 行，每行 512 个 `StackType_t`。静态创建把实参直接
  存到 TCB 的 `pxStack`。两端口的 8 字节元素已满足 8 字节对齐，
  向下生长的初始顶位于下标 511。两端口分别为 `PortTask` 和
  `ThreadState_t` 留出两个栈字并返回同一行的下标 509。
  因而两次比较分别使用同一行的 509／0 和 509／511。
  动态任务和 timer 服务任务在当前配置关闭。
- FreeRTOS 采用原件另外有 `kernel/queue.c:2382,2438` 两处关系比较，
  写游标和读游标每次各进一个 `uxItemSize`，与同一个队列尾
  `pcTail` 比较。初始化把尾设为 `pcHead + uxLength * uxItemSize`；
  达到 one-past 时先回绕到 `pcHead`。已选程序无数据队列的项目创建方，
  这些原件实例沿采用代码记录。0020 的退步判断是指针相等比较。

## 指针声明

R18.5 展开 typedef 和调整数组形参后，逐段计算连续指针 declarator。
数组和函数 declarator 各自截断连续层，函数的每个形参与返回类型独立检查。
实际自有及项目补丁声明最多两层。DET 的 `error_hooks`、
`runtime_callouts` 和 `Det_ValidHooks` 的 `hooks` 是指向回调指针的
指针，`Os_Target.h` 的 `entries` 同理。idle 输出 `StaticTask_t **`、
`StackType_t **` 各为两层。FreeRTOS 的 `TaskHandle_t *const` 输出展开
为 `struct tskTaskControlBlock **const`。RTE 和 CAN 回调类型也逐一检查；
函数外层指针、参数指针与返回类型各走自己的声明链。
Windows SDK 原头里的 `SNB *`、`RPC_EP_INQ_HANDLE *` 和
`PCCERT_CHAIN_CONTEXT **` 等达到三层。其声明属于 SDK，
实际项目和补丁展开未引用这些类型或关联调用。

## 项目源码验收

当前 [checker-obligations 绑定](declarations.md#当前输入重评) 消费本篇。
原始 XML、诊断指纹和 R8.2 的批准实体保留。原始 `passed=false`、
`completeCompliance=false` 和项目验收 `blocked` 保持。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R18.2 | Required | 指针相减的对象归属 | satisfied | 主文 145。同数组及 one-past 的差须可由 ptrdiff_t 表示。八个实际配置无指针与指针相减；pcTail - uxItemSize 是指针与整数的运算。 |
| R18.3 | Required | 指针关系比较的对象归属 | satisfied | 主文 146＋AMD4.52。0015 的两次比较共用 task_buffers 的同一行。采用 queue.c 的两次比较沿同一队列的尾界回绕。 |
| R18.5 | Advisory | 对象声明的连续指针层数 | satisfied | 主文 148–149。typedef 展开及数组形参调整后，项目声明至多两层。三层声明来自未被项目使用的 Windows SDK 原头。 |
<!-- misra-checker-obligations-end -->
