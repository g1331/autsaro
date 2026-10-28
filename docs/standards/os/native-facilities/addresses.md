# 主机地址表示

## 主机地址表示类别处置

两平台分别记录不可消除的原生地址表示；只覆盖下列固定来源与风险控制。

## Linux x64

规则 R11.4、R11.5 均为 Advisory（主文印刷97–98／PDF105–106；
TC1 将 R11.4 限定到对象指针，AMD2.60 扩充 stdint 说明）。
uintptr_t 的可表示性没有构成 R11.4 例外，R11.5 同时覆盖显式 cast 与隐式赋值。
依 [ADR 0025](../../../adr/0025-misra-layered-scope.md) 的自有主机适配层及
[项目决定第1节](https://github.com/g1331/autsaro/issues/58#issuecomment-6437338057)，
以下记录限宿主接口决定的地址表示，不借用动态设施的 Required 类别授权。

范围仅 linux-x64-controlled-v1、四 profile 的原生 OS／HostBatch／probe 与相同实现的
native 消费者：`Os_HostCreateActor` 将 mmap 返回的 void 指针保存为 unsigned char 映射；
`Os_HostStackRange` 将该映射地址转为 uintptr_t；`Os_StackRecordBuffer` 将静态内核缓冲的
one-past 地址转为 uintptr_t。均由自有代码实施，不能整体移交 SDK 来源责任。

mmap 的固定 ABI 返回 void 指针；guard、ASan fiber 和实际栈入口需要同一映射内的字节偏移。
改成整数搬运、memcpy 指针表示或换一层 helper 不消除该转换。
栈保护比较的是来自不同存储对象的 native 地址区间（包括真实 RSP 和静态内核缓冲），
不能用 C 对同一数组内指针的大小比较替代。数值区间是既有 ARTI／原生栈合同的输入，
在受控 x64 上 uintptr_t 完整容纳地址；结果不转换回对象指针解引用。

风险是错误对齐、截短、溢出及跨对象计算错误。mmap 保证页对齐，映射长度与 guard
来自同一次成功创建，失败和回滚保持所有权；字节偏移限该映射，内核缓冲由实际静态
Task 存储提供。既有 native_stack／ARTI 消费者观察物理栈、注册区间和汽车缓冲不重叠，
host boundary 检查创建、errno、失败回收和句柄复用；检测器与目标 CI 另按实际运行留证。
可修复的 watch／Task／Linux actor 回调参数不在此类别：它们使用既有静态对象或按地址
匹配固定 typed 槽，消除反向 void 转换，并保留异常参数拒绝的反例。

重评条件为目标 ABI／地址宽度、provider、mmap flags／容量、栈布局／guard、ARTI
数值合同或缓冲所有权变化。下节独立说明 Windows 的接口与风险控制。
保留原始诊断和分析身份，类别处置不表示源码符合这两条规则。


## Windows x64

限 windows-x64-controlled-v1、四 profile 的原生 OS／HostBatch／probe 及相同实现的 native
消费者。R11.4 为 Advisory，R11.6 为 Required（主文印刷99／PDF107；TC2.22 修正指针整数
转换的实现定义措辞）。uintptr_t 的可表示性不是规则例外。下列均为自有转换，依上述原
项目决定的宿主接口原则逐来源处置，不归入 SDK 或主机动态设施授权。

| 文件与函数 | 规则 | 精确构造／用途 |
| --- | --- | --- |
| Os_Stack.c / Os_StackRecordBuffer | R11.4 | 静态内核缓冲 top 的 one-past 对象地址转为 kernel_buffer_high。 |
| Os_Stack.c / Os_StackRegister | R11.6 | AllocationBase 转为 reserve_low；VirtualQuery 的 cursor 整数转为 const void 指针。 |
| Os_Stack.c / Os_StackCheck | R11.6 | 真实 RSP 转为 VirtualQuery 地址参数。 |
| Os_Stack.c / Os_StackObserve | R11.6 | 被观察线程的 CONTEXT.Rsp 转为 VirtualQuery 地址参数。 |

[VirtualQuery](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualquery)
接受地址参数，返回所在分配、区域长度、提交状态与保护属性；该地址参数按页处理。
当前注册须跨原生栈页枚举，运行检查须查询真实 RSP；两者不能以任一 C 自动对象的地址
替换。AllocationBase 为 SDK 的 void 指针，现有数值区间还包含原生上下文 RSP 和不同对象
的内核栈缓冲；同数组内指针算术不能表达该跨对象区间。包装、整数搬运或复制指针表示
不消除转换；删除查询会丢失实际页保护与 guard 检查，改为本地对象只检查另一地址。

分配身份直接以首次查询保留的 AllocationBase 指针作相等比较，不纳入地址转换类别。
受控 x64 的 uintptr_t 容纳本机地址；VirtualQuery 返回大小必须等于结构大小，枚举要求
RegionSize 非零且不溢出 UINTPTR_MAX，运行检查先验证 RSP 在已注册区间，再检查 MEM_COMMIT
及 PAGE_GUARD／PAGE_NOACCESS。重构的指针只交给元数据查询，应用不经它解引用；页对齐由
原生 API 处理。静态内核缓冲来自现有 Task 存储。原子发布、21槽身份索引、故障 winner、
停止观察和进程期存储不变；native stack／ARTI 观察注册区间、实际 RSP 和独立内核缓冲，
foreign／one-past／wrong-slot 反例保持身份拒绝。实际平台行为与检测器结果集中 PR 留证。

风险为截短、错误地址、长度溢出和错误保护判断。以上目标宽度、区间及返回值检查约束
这些风险；ABI／地址宽度、provider、VirtualQuery 合同、栈布局、所有权或 ARTI 数值合同
改变时重评。此类别不覆盖任意 Windows cast、回调反向 void 转换、函数指针或经重构指针
解引用，也不表示源码符合 R11.4／R11.6。

## 类别实例检查

`c_standard.host_address_instances` 仅在上述命名函数体内匹配八个固定表达式：三个 Linux、
五个 Windows。同一函数内的不同表达式保持独立身份；文件、规则、函数或表达式改变、
缺失／重复实例均拒绝，MR12 与记录双向关联；类别文档缺失同样拒绝。
`test_host_address_category_requires_exact_owner_expression_and_annotation` 检查既有 Linux
正反例；`test_windows_address_category_keeps_each_expression_bounded` 验证 Windows 的全部五实例、同函数双实例、缺注释、错规则、越界表达式、
错误函数及重复表达式。项目检查识别来源模式，不替代类型／行为分析、调用合同或规范判断。
