# 语法与原始诊断

## 原始 token 分类的具体误报

当前 master `3e93e436` 的 Linux standard-ecu 工件保留 20 条 R7.3：
Os_Backend 的 `OS_RESOURCE_CREATE(call)` 四处，Os_HostServices 的 `cell` 六处，
Os_HostLinux 的 `cell` 四处及 `all` 两处，Os_LinuxNative 的 `cell` 四处。
逐列回到原源码均为标识符；固定 2.21.0 的 `misra_7_3` 使用十六进制字符集合加小写 l
后缀的正则，未先确认 token 是数字，故这些名字恰好匹配。
R7.3（Required，主文印刷61／PDF69）约束数字字面量后缀，不约束这些标识符。
不能为了消除误报更名真实参数、改 addon 或删除原始记录。

同一工件的 12 条 R12.3 位于 Os_Stack.h 的七个成员声明分隔符、Os.c 的 i/j
声明、Os_Backend 的四个计数器声明和 registered/gate 成员声明。
它们在 C99 语法中是声明的逗号分隔符，不是 §6.5.17 的 comma operator；
R12.3（Advisory，主文印刷106／PDF114）禁止 comma operator；声明分隔符不属于该运算符。
类型头拆分只改变七个成员分隔符的位置，成员顺序与类型不变；其余声明不改写。

## 回调的 const 建议

通用 constParameterCallback 建议须按调用 ABI 复核。watch、control_actor、task_entry
以及两个 Linux signal handler 和 guard_signal 的参数类型分别由原生线程或 sigaction
回调原型约束；不能把参数类型改为 const void * 等不兼容函数类型。
Os_LinuxAltStack 的 previous 和 native_call 输入由汇编约束实际读写，固定分析器未建模，
其 constParameterPointer／R2.7 建议不作为删除或收窄这些参数的依据。
这些是限定位置的工具判断，不覆盖别的 const 建议或 SDK 实现。
