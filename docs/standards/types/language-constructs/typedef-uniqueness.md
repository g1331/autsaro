# R5.6 typedef 名称唯一性

MISRA C:2012 第三版主文印刷第 54 页（PDF 第 62 页）将 R5.6 定为
Required／System。每个程序的 typedef 名须在所有名称空间和 TU 中唯一。同一头
中的同一定义可随包含进入多个 TU，typedef 可与其关联的 struct／union／enum tag
同名。AMD2 增列 C11，本项目使用 C99。AMD1–4／TC1–2 没有改变本批的例外条件。

## 实际程序

以 [master 0c8a9b94 的双平台原始工件](https://github.com/g1331/autsaro/actions/runs/44785660740)
为声明证据。核对到 `a3f8244e` 的 Git 差异，生成器、runtime、第三方源码与样本夹具
字节一致。四个 profile 各有 HostBatch／probe，合计 16 个程序、888 个程序 TU 实例、
452 份独立 GCC 展开。对每个程序，将同一头的定义去重，再按拼写连接其他 typedef、
tag、成员、枚举常量、函数、对象、形参及标签。用 GCC 行标记回接物理声明，
用实际底层类型检查关联 tag，按 GCC 依赖、`patchedLines` 和 ADR 0025／0030
区分项目、采用原件及主机专用位置。

| Profile | 每个程序的范围内项目 typedef | Linux 每个程序的全部 typedef | Windows 每个程序的全部 typedef |
| --- | ---: | ---: | ---: |
| standard-ecu | 223 | 498 | 6968 |
| user-application | 223 | 498 | 6968 |
| component-pipeline | 227 | 502 | 6972 |
| no-diagnostic | 224 | 499 | 6969 |

Linux 的辅助 Clang 解析完成所有 TU。Windows 的 GCC 向量 intrinsic 在 Clang 中
产生兼容性诊断，实扫来自 Windows GCC。错误均回接 GCC SDK 头，另用原始展开
核查这些头的 typedef 与项目标识符没有重名。四组 SDK 交叉实体回查
原始 GCC 展开和包含链。FreeRTOS 采用行按原件归属，项目补丁按物理行
纳入；本次未发现项目补丁的同名冲突。Windows SDK 三个 typedef 的展开列
随宏变化，回到相同原头、物理行和名称后各计一次。关联 tag 例外包括 `Os_NativeStackType`，
`Os_Arti.h` 的前置声明与 `Os_StackTypes.h` 的 typedef 定义关联同一 struct。
Linux 宿主的 `Os_HostHandleType` typedef 和 tag 均归 ADR 0030。
除以下实例外，项目范围内未发现其他重名。

## 两个多组件 profile 的 Rte_Instance

`Rte_Ingress.h`、`Rte_Process.h`、`Rte_Observe.h` 分别在
`app/Ingress/Ingress.c`、`app/Process/Process.c`、`app/Observe/Observe.c`
声明指向各自 const CDS 的 `Rte_Instance`。四个目标／样本组合的两个程序均含
三个独立定义。单组件两个 profile 的两个程序各有一个定义。核心 TU 的
`RTE_CORE` 排除该 typedef，应用 TU 各自还原真实定义。

AUTOSAR CP R24-11 SWS RTE 的 01007（PDF 第 597 页）要求应用头定义实例句柄类型，
02311（第 656 页）要求各组件 CDS 的具名 struct 和 typedef。91176、01148、01150
（第 696 页）合起来规定接口的头、底层指针类型和 `Rte_[Byps_]Instance` 名字。
当前配置为 `bypass=NONE`。加组件后缀会违反 01150，共用 CDS 类型会违反
01148／02311。应用头按组件隔离，双头相遇由生成头拒绝。技术处置与风险控制见
[RTE 实例类型](../../rte/README.md#组件实例类型的同名冲突)。

三个 MR12 位置均为生成应用头第 30 行第 32 列。每个位置在 HostBatch／probe
都有原始诊断，合计 12 个样本内报告位置、24 个逐程序出现记录。ADR 0023 的
既有授权覆盖其必需契约冲突。验收当前仍报 `formal_deviation_pending`。

## Windows SDK 集成

Windows 的四个 profile、两个程序均有下列交叉重名。项目侧位置与 SDK 原头位置
分别按实际展开核对。

| 名称 | 项目位置 | SDK 位置 | 实际声明 |
| --- | --- | --- | --- |
| `CounterType` | `generated/include/Rte_Type.h:15` | `winperf.h:155` | typedef 与性能计数器结构成员 |
| `ResourceType` | `os/include/Os_Types.h:38` | `rpcdcep.h:239` | typedef 与 `RDR_CALLOUT_STATE` 成员 |
| `boolean` | `bsw/include/Platform_Types.h:66` | `rpcndr.h:95` | `_Bool` typedef 与 SDK `unsigned char` typedef |
| `byte` | `bsw/src/Com.c:86` | `rpcndr.h:93` | `Com_Pack` 循环索引与 SDK typedef |

前两组是跨名称空间重名，后两组是跨 TU 的普通标识符重名。三个 `byte`
声明还出现在 HostBatch 专用源码。HostBatch 入口的形参和局部对象只进入该程序，
`Ecu_HostBatch.c` 的局部对象随共享源进入两个程序。这些位置归 ADR 0030。

SDK 的性能和 RPC 声明随 `windows.h` 导入。OS 的 `Os_Windows.h` 已把 SDK
`boolean` 展开成 `Os_Win32Boolean`，但 `Can_HostLock.c` 直接包含
`windows.h`，仍引入 SDK 原名。只调整单个 TU 的包含无法消除仍出现在其他 TU 的
重名。项目公共名分别来自 SWS_Os_00560 及 OS 计数器服务签名、OSEK OS 2.2.3
§13.4 和 SWS_Platform_00026。保留这些发布名，优先验证缩小 Windows SDK 导入集合
是否能覆盖两条导入路径，并核对原生编译、链接与实际启动。

固定 Cppcheck 未报告这四组交叉实体。当前规则级 `retained` 格式要求每个保留实体
关联原始诊断列号。Windows 集成整改或能核验漏报实体的正式记录完成前，整条 R5.6
保持 `not_assessed`。诊断级指纹和程序报告按
[ADR 0033](../../../adr/0033-c-diagnostic-fingerprints.md) 保留。

将 `Com_Pack` 的局部名改为 `byte_index` 后，用同一 GCC 和 Cppcheck 本地双扫
Linux standard-ecu。两侧均有 2724 条原始诊断，规则、位置和程序集合相同。
由于 [#609](https://github.com/g1331/autsaro/issues/609) 的指纹绑定整个相关 TU，
602 个范围内逐程序指纹替换，其中 490 个已有绑定失效。新指纹包括同一文件中
106 个尚无处置的 R10.3／R10.4 报告。即使只重绑有依据的位置，门禁仍会拒绝
未处置的新指纹。本次保留该循环的原始代码；改名需要先完成这些诊断的处置，
或由 #609 提供能验证局部改名并保留未变诊断身份的机制。
