# 0030. 主机仿真层移出 MISRA 合规范围

## 状态与背景

已接受（2026-11-14，项目负责人决定），执行 [#501](https://github.com/g1331/autsaro/issues/501)。本决定部分取代 [ADR 0025](0025-misra-layered-scope.md) 对“自有主机平台适配”层的处置：该层不再是正常整改层，也不再引用主机类别记录。[ADR 0024](0024-bounded-linux-platform-retention.md) 改为历史记录。ADR 0023、0026 和 0027 的其余决定不变，不回退门（ADR 0027）对范围外位置的处理由本篇补充。

`SWS_BSW_00115`（R24-11 SWS BSW General，PDF 第 37 页，已对照原文）要求 C 写的 BSW 模块实现符合 MISRA C:2012，违反只限技术上合理的例外，并在源码中就近注释。项目负责人决定：Linux／Windows 主机适配是仿真设施，只在宿主机运行，不进入实机 ECU，不纳入 MISRA 合规范围。本篇把这个决定落成可机械判定的边界和处置，不再论证取舍。

#501 记录的 master 最新分析共 22322 处诊断：路径落在 `integration/`（来自 `runtime/host`、`runtime/ecu`）、`os/src/host/`、`kernel/portable/` 的有 1900 处（8.5%）。下文的判定规则划出的范围与这个路径统计不完全相同，差异在“已知边界”一节列明。

## 边界

规则的一句话形式：**凡是也会编译进目标的文件或头文件，都留在合规范围内；只在宿主机构建里出现的文件在范围外。** 判定只看文件和包含关系，不看符号名或注释。

### 路径与交付位置

| 源码位置 | 交付位置 | 范围 | 判定 |
| --- | --- | --- | --- |
| `runtime/os/src/host/linux/`、`runtime/os/src/host/windows/` | `os/src/host/` | 范围外 | 规则 1 |
| FreeRTOS `portable/MSVC-MingW/`、`portable/ThirdParty/GCC/Posix/` 及补丁中落在这些目录的改动 | `kernel/portable/` | 范围外 | 规则 1、3 |
| `runtime/host/`（`Can_Host*`、`Det_Host*`、`SchM_*_Host.c`、`Ecu_HostBusSM*`、`Ecu_Status.h`） | `integration/` | 逐文件 | 规则 2 |
| `runtime/ecu/src`、`runtime/ecu/include`（HostBatch、`Ecu_Target`、`ecu_probe.c` 等） | `integration/` | 逐文件 | 规则 2 |
| `runtime/ecu/templates/` 渲染出的文件 | `generated/` | 范围内 | 生成 C 是自有目标代码 |
| `runtime/bsw/`、`runtime/os/` 其余文件（含 `os/include/`、`os/src/` 顶层） | `bsw/`、`os/` | 范围内 | 见“已知边界” |
| FreeRTOS 内核其余文件及补丁中落在这些文件的改动 | `kernel/` | 范围内 | 规则 3 |
| R10 的 MCU port 与驱动 | 待定 | 范围内 | 目标代码 |
| 工具链／SDK 原头、固定 FreeRTOS 原件 | — | 沿 ADR 0025 | 采用代码层不变 |

### 判定规则

1. **平台选择目录整体范围外。** `os/src/host/<平台>/` 和 `kernel/portable/<port>/` 由构建按平台选择，目标构建换成 MCU port 的对应目录。目标代码经 `#include "portmacro.h"`、`Os_HostTypes.h` 里的 `#if defined(__linux__)` 分支包含其中的头文件，属于平台选择，不改变归属。
2. **集成目录按包含闭包判定。** `runtime/host`、`runtime/ecu/src`、`runtime/ecu/include` 里的文件，凡是出现在任一目标翻译单元的包含闭包里，就留在范围内；其余范围外。目标翻译单元是规则 1 目录和这两个集成目录之外的全部 `.c`（含生成 C 和内核文件）。闭包取 GCC `-MD` 回执（`scope.projectTranslationUnits`），覆盖两个目标、四个 profile、两个程序的并集，由检查器生成，不手写清单。`.c` 文件不会被包含，因此这一目录里的 `.c` 一律范围外；被目标代码调用的声明在头文件里，头文件按闭包留在范围内。
3. **项目补丁逐 hunk 判定。** 一个补丁落在规则 1 目录的 hunk 范围外，落在其余内核文件的 hunk 是目标代码，留在范围内。同一个补丁可以两边都有。
4. 新增文件按路径适用上述规则；文件移动使归属变化时，PR 说明新归属。

规则 2 的闭包是静态事实，所以“被目标代码包含的主机头文件”会留在范围内。这些声明和宏随包含它的翻译单元一起分析，实现体在范围外。

### 三个真实例子

**共用头文件：`Det_Host.h`。** `runtime/bsw/src/Det.c` 包含 `Det_Host.h`（位于 `runtime/host/src`）并调用其中声明的 `Det_HostHalt`。`Det.c` 是目标翻译单元，所以 `Det_Host.h` 在闭包里，留在范围内（规则 2）；`Det_Host.c` 是退出码 134 的主机实现，没有人包含它，范围外。目标构建需要给同一个声明提供自己的实现，这个接缝在范围内。同类的还有 `Can_Host.h`（经 `bsw/src/Can_Internal.h`、`ecu/src/Ecu_Execution.h` 被 `Can.c` 包含）和 `Ecu_Status.h`（被 `bsw/include/Can_CfgTypes.h` 包含）。

**host 专用的 SchM 实现：`SchM_CanIf_Host.c`。** 它实现 `SchM_Enter_*`／`SchM_Exit_*`，声明在 `bsw/include/SchM_Can.h` 等范围内头文件里，实现用主机互斥锁 `Can_HostLock`。它是 `runtime/host/src` 里的 `.c`，没有被任何文件包含，范围外；目标构建换成中断锁实现。`Can_HostLock.h` 则被生成的 `SchM_Main.c.in` 包含，留在范围内，`Can_HostLock.c` 范围外。`SchM_Diagnostic_Host.c`、`SchM_Mode_Host.c` 同理。

**项目 FreeRTOS port 补丁：`linux/0001` 与 `0003`。** `0001-controlled-posix-port.patch` 只改 `portable/ThirdParty/GCC/Posix/port.c` 和 `portmacro.h`。内核的 `tasks.c` 通过 include 路径包含 `portmacro.h`，但它位于平台选择目录，整体范围外（规则 1）。`0003-activation-ready-policy.patch` 同时改 `tasks.c`（请求 FIFO 就绪策略，进入目标）和 Windows `port.c`（主机）：前者留在范围内，后者范围外（规则 3）。

### 已知边界：仍在范围内的主机相关文件

下列文件按规则留在范围内，尽管内容带有明显的原生实现。本决定不搬动代码，逐个裁定由 [#502](https://github.com/g1331/autsaro/issues/502) 承接：

- `runtime/os/src/` 顶层的 `Os_Backend.c`（含 Windows 的 `Os_PortThread`）、`Os_Stack.c`（Windows 栈记录）、`Os_PortRestart.c`、`Os_HostIo.c`、`Os_HostServices.c`、`Os_Windows.h`，以及 `os/include/Os_Host.h`、`Os_HostIo.h`、`Os_HostTypes.h`。Windows 主机适配大部分在这些顶层文件里，只有 `Os_HostWindows.c` 在 `os/src/host/windows/`；Linux 适配则整体在平台选择目录里。两个平台因此不对称，Windows 留在范围内的部分比 Linux 多。
- 目标代码里直接引用主机接缝的位置：`Can.c` 对 `Ecu_Execution.h` 的包含，生成的 `Ecu_Config.c` 对 `Ecu_Target.h`、`Ecu_HostBusSM.h`、`Can_Host.h` 的包含。这些引用本身在范围内。

裁定的标准是：R10 的目标构建是否还需要该文件的这一部分。只服务主机的部分迁入 `os/src/host/<平台>/`，迁入后自动按规则 1 移出范围；目标仍需要的部分留在范围内。在裁定完成前，这些文件上的既有类别记录继续有效（见“影响处置”）。

## 范围外保留的检查

- **Cppcheck** 对范围外文件照常扫描，原始诊断照常保留在 `summary.json` 和 `diagnostics.xml`，增加“范围外”标注。`passed`、`error`、退出码和 221 项 assessment 保持原义。
- **不回退门**（ADR 0027）对范围外位置的新增诊断只报告、不阻断；基准树与 PR 树仍然完整扫描。范围内位置的规则不变。
- **项目验收**（ADR 0026）不要求范围外位置有处置：它们不进入 `blocked` 判定和 `objects`，报告里单列条数。范围内的处置规则不变。
- **clang-tidy**、sanitizer、原生测试和 `test_host_boundaries.py` 照旧；`-Wall -Wextra -Werror` 和项目规则检查（直接动态内存调用等）照旧。
- 范围外不放宽 AUTOSAR 契约：对外使用标准名称的接口仍须满足标准签名和语义，主机行为的验证仍按[测试说明](../development/testing/README.md)执行。
- 已完成的整改和已有的 `MR12` 注释保留，不批量回头删除，也不再为范围外位置新增类别记录或偏离注释。

## 对外表述

- 主机仿真包（`windows-x64-controlled-v1`、`linux-x64-controlled-v1` 的生成工程、HostBatch 和 probe）不声明 MISRA 结论，README、发行说明和工作台不写“符合”。主机验证的结论仍限定为主机能力。
- 今后对外的 MISRA 说明只针对目标代码集合，并写明本篇的范围规则。R10 的 MCU port 和驱动是目标代码，进入合规范围，`SWS_BSW_00115` 的义务对它们完整适用。
- ADR 0025 对 SDK／FreeRTOS 采用代码的责任划分不变；“来源分类不是供应方符合性证明”的限定继续有效。

## 影响处置

### 类别记录

| 记录 | 处置 |
| --- | --- |
| [主机动态设施 D4.12／R21.3／R22.1](../standards/os/native-facilities/storage-interrupts.md#主机动态设施类别偏离) | `Os_HostLinux.c::Os_HostCreateActor` 范围外，转历史。`Os_HostServices.c` 的两个 Spawn 和 `Os_Backend.c::Os_PortThread` 在待定文件内，记录对这些位置继续有效，随 #502 裁定 |
| [同步嵌套 ISR R17.2](../standards/os/native-facilities/storage-interrupts.md#同步嵌套-isr-类别偏离) | 范围只有两个端口补丁（Linux `0001`、Windows `0012`），全部落在 `kernel/portable/`，整体转历史 |
| [主机地址表示 R11.4／R11.5／R11.6](../standards/os/native-facilities/addresses.md#主机地址表示类别处置) | Linux 三处实例在 `os/src/host/linux/`，转历史。Windows 五处实例（`Os_Stack.c` 的 `Os_StackRecordBuffer`、`Os_StackRegister`、`Os_StackCheck`、`Os_StackObserve`，R11.4／R11.6）在待定文件内，记录继续有效，随 #502 裁定 |
| [ADR 0024 三处 Linux 标准头](0024-bounded-linux-platform-retention.md)（R21.5／R21.10） | 三处都在 `os/src/host/linux/`，转历史 |
| [SDK 类别](../standards/os/sdk.md#当前集成义务与类别处置)（#444，R5.5／R5.7／R7.1／R7.2／R7.3／R10.1） | 按位置：落在 `os/src/host/linux/`、`kernel/portable/`、`ecu_host_batch.c` 的实例转历史；`Os_Backend.c`、`Os_Stack.c`、`Os_Time.c` 里的实例继续有效，随 #502 裁定 |
| R15.5 的 [GRP](0025-misra-layered-scope.md#grp指南重分类计划) | 适用对象改为自有目标代码（含生成 C 和项目内核补丁）；范围外的主机层不再需要它 |
| MemMap R20.1／R20.5 | 与主机无关，不变 |

“转历史”的含义：记录和证据原样保留，写明当时的依据与范围，不再被验收消费，也不要求随源码变化重评。

### 合规线的义务

| Issue | 只因主机代码而存在的部分 | 处置 |
| --- | --- | --- |
| #439（自有源码精确裁决消费） | 同步嵌套 ISR 记录的消费；主机动态设施、主机地址表示记录中范围外实例的消费；`os/src/host/`、`integration/` 位置的精确裁决 | 按位置移除；`Os_HostServices.c`、`Os_Backend.c`、`Os_Stack.c` 内的实例继续消费，随 #502 裁定。其余（既有 124 键里范围内的位置、ErrorHook R18.6、MemMap）不变 |
| #443（checker 覆盖项人工证据） | 七处 Windows 小写 `l`、44 次 SDK 缺 U 暴露、#470 的 signal 宏对 R7.2 证据的影响、R7.1 `O_RDONLY` 使用点 | `os/src/host/linux/`、`kernel/portable/`、`ecu_host_batch.c` 里的位置移除；`Os_Backend.c`、`Os_Stack.c`、`Os_Time.c` 里的位置保留，随 #502 裁定 |
| #445（66 项人工记录） | 7 项 retained 中的 D4.12，以及 D4.5／D4.6 里只涉及原生 `HANDLE`、`BOOL`、`CONTEXT`、pthread 错误值的实例 | 范围外实例移除；待定文件里的实例随 #502 裁定。D4.8、D4.9、D4.10、R2.8 与主机无关，不变 |
| #482（合规线出口） | 完整验收的“同一 head 全部位置已处置”条件 | 条件限于范围内位置；范围外位置不参与 `accepted` 判定 |
| #58（父项） | 上述各项对应的主机部分 | 随子项调整；父项出口不变 |

已关闭的 #381（自有源码诊断分类）、#60（主机偏离草案）、#444（SDK 集成）保持关闭，结论按“转历史”处理，不重开。

## FreeRTOS 采用代码与项目补丁

### 固定原件

固定的 V11.3.1（`054e14f3397023aa83813a65aa065fc4597d481b`）原件沿 ADR 0025 和 [adoption.md](../standards/os/adoption.md) 的做法，整体分类为采用代码，引用该版本上游 MISRA.md，不逐项评估。新增一条：原件位置上的诊断在不回退门中只报告、不阻断，处理方式与主机层相同；项目验收同样不要求这些位置有处置。

### 补丁分类

`runtime/os/patches` 有 27 个补丁，`linux/` 有 3 个。Windows 链用前 27 个，Linux 用其中 14 个公共补丁加 `linux/` 三个。下面按各补丁的实际 diff 分三类；范围按规则 3 逐 hunk 判定，同一补丁可以两边都有。

**(a) 主机专用行为，13 个。** diff 落在 `portable/MSVC-MingW/` 或 `portable/ThirdParty/GCC/Posix/`，MCU port 会自己实现。范围外。

| 补丁 | diff 内容 |
| --- | --- |
| 0001 controlled-host-lifecycle | Windows port 的受控启动与关闭 |
| 0002 native-stack | Windows port 接入原生栈 |
| 0004 windows-event-namespace | `SetEvent` 改调 `Os_HostSetEvent` |
| 0005 category2-resource-gate | Windows port 在 ISR 区间内挂起应用线程，并按中断源使能状态门控分发 |
| 0006 input-handler-ownership | 注册 handler 前的所有权检查 |
| 0007 controlled-tick | port 的受控 tick；`tasks.c` 里的 `vTaskOsTestTick` 在 `OS_TIME_TESTS` 守卫内，交付构建不编译，按规则 3 仍属范围内 hunk |
| 0009 interrupt-source-control | port 的中断源屏蔽状态与控制函数 |
| 0010 native-interrupt-identities | 中断身份检查 |
| 0011 isr-phase-before-mutex-release | ISR 阶段在释放互斥量前结束 |
| 0012 nested-interrupt-dispatch | 嵌套 ISR 分发 |
| 0013 interrupt-source-repetition | 拒绝重复使能或屏蔽同一中断源 |
| 0014 relocatable-interrupt-vectors | 分发表改用可重定位段 |
| linux/0001 controlled-posix-port | 受控 POSIX port（`port.c`、`portmacro.h`） |

0005 的名字带 OS 语义（Category 2 资源与中断门控），但 diff 全在 Windows `port.c` 里，端口换成 MCU 版本时由新端口实现，因此归 (a)。

**(b) 随内核进入目标的功能改动，3 个。** 改动落在内核文件，目标构建会编译。留在合规范围内。

| 补丁 | diff 内容 |
| --- | --- |
| 0003 activation-ready-policy | `tasks.c`、`task.h`、`FreeRTOS.h` 加 `configOS_REQUEST_FIFO` 请求 FIFO 就绪选择、就绪序号字段和访问函数；同补丁里 10 行 Windows port 改动范围外 |
| 0008 task-hook-boundaries | `tasks.c` 的任务选择宏加 `traceOS_TASK_SELECTED` 观察点 |
| 0020 bounded-queue-front | `queue.c` 的队列前端游标先判断是否在头部再回绕。原实现先执行 `pcReadFrom -= uxItemSize` 再比较，合法的单元素队列在读游标位于头部时会形成数组之前的指针，这是进入目标的队列正确性修复，同时消除 R18.1 诊断；补丁保留自有代码评估责任 |

**(c) 只为消除 MISRA 诊断的清理，14 个。**

| 补丁 | 落点 | diff 内容 | 范围 |
| --- | --- | --- | --- |
| 0018 windows-thread-setup-errors | port | 检查 Win32 线程设置调用的返回值，失败停止启动（D4.7／R17.7） | 范围外 |
| 0019 windows-native-handoff-errors | port | 检查 mutex、event、yield 交接调用的返回值 | 范围外 |
| linux/0002 sequence-persistent-side-effects | port | 持久副作用拆成独立语句（R13.5） | 范围外 |
| linux/0003 port-diagnostic-types | port | 端口类型与诊断整改 | 范围外 |
| 0017 portable-header-paths | 内核 | `deprecated_definitions.h` 的反斜杠路径改正斜杠 | 范围内 |
| 0021 c99-comment-examples | 内核 | `task.h`、`queue.h`、`timers.h` 的注释示例改写，补丁约 295 KB | 范围内 |
| 0023 preserve-parameter-values | 内核 | `tasks.c` 参数改用局部副本（R17.8） | 范围内 |
| 0024 conditional-private-definitions | 内核 | `tasks.c`、`queue.c` 的私有宏加配置条件（R2.5） | 范围内 |
| 0025 configured-kernel-declarations | 内核 | `portable.h`、`task.h`、`queue.h` 的声明按配置条件编译 | 范围内 |
| 0026 explicit-control-flow-boundaries | 内核 | 补注释，并删除 `configASSERT` 之后的 `NULL` 早返回 | 范围内 |
| 0015 typed-port-state | 内核＋port | R19.2：union 改独立字段，`StaticTask_t`、`StaticQueue_t` 换成真实对象类型 | 内核部分范围内，port 部分范围外 |
| 0016 essential-boolean-controls | 内核＋port | `while(0)`→`while(false)`，`configASSERT(x)`→`x != NULL` | 同上 |
| 0022 explicit-function-addresses | 内核＋port | 显式取函数地址（R17.12） | 同上 |
| 0027 port-diagnostic-types | 内核＋port | MR12 注释、栈游标与队列游标改写 | 同上 |

### 政策与回退建议

补丁改过的行是自有代码，评估责任从上游转到项目。因此：**今后不再为消除 MISRA 诊断去修改 FreeRTOS 原件。** 内核补丁只收 (b) 类功能改动，每个都要有 AUTOSAR 语义依据；主机 port 的改动不受此限，它们在范围外。

对已有 (c) 类补丁的建议，实际回退由 [#503](https://github.com/g1331/autsaro/issues/503) 承接，本 PR 不改补丁：

- **port 目录里的清理（0018、0019、linux/0002、linux/0003 及混合补丁的 port 部分）：保留。** 它们在范围外，回退没有合规收益，0018、0019 的返回值检查还有实际的失败处理价值。
- **内核文件里的等价改写：建议回退。** 包括 0016、0022 的内核 hunk，以及 0017、0021、0023、0024、0025。回退不改变布局、ABI 和运行语义。
- **涉及布局、指针运算或控制流的：逐个核对再定。** 0015 改变对象大小，0027 改写栈游标和队列游标，0026 删除的早返回在关闭 `configASSERT` 的构建里会改变行为。
- **0020 属于 (b)，保留。** 它修的是真实的越界指针构造，不在回退范围内。

回退的代价：

- 内核字节、`kernelSha256`、交付资产与封存身份都会变，`assets check` 清单和 `adoption.md` 的补丁表要同步更新。
- c-check 的行归属变化：`scope.patchedLines` 里被回退的行消失，这些行上的诊断从 `patched`（自有代码）回到 `adopted`（采用代码，只报告不阻断）；FreeRTOS 的集成证据身份（原件清单、有序补丁、最终内核）随之重算，已绑定的评估要重绑。
- 补丁链有序叠加，0024–0027 的上下文会变，需要重新生成补丁链。
- 收益是内核只留功能补丁，不再有需要自有代码评估的清理行，今后升级内核的合并成本也低得多。

回退在 #55 合并后单独一个 PR 完成，避免与不回退门的基准比较叠加。

## 检查器配置

范围判定写进检查器的范围配置（`c_check`／`c_acceptance`）：按规则 1–3 标注每条诊断是否在范围内，并让不回退门和项目验收按上文消费。这项改动与 #55 的不回退门实现修改同一批文件，留给 #55 合并后的后续 PR，由 [#502](https://github.com/g1331/autsaro/issues/502) 承接。在此之前不回退门保持 ADR 0027 的现行行为（范围外位置照常按范围内处理），不会漏检。

## 同步修改与授权边界

同步修改：`docs/standards/README.md` 第 7 条、`source-acceptance.md`、`generated-c.md`、`docs/architecture/README.md` 的 ADR 索引、`AGENTS.md` 与 C 编码规范中描述 ADR 0025 分层的句子，以及各类别记录篇的状态说明；ADR 0024、0025、0027 只改状态行。

本决定不批准新偏离、SDK 修改或维护分叉、供应方投稿和对外的符合性承诺。R10 的 MCU port 与驱动进入合规范围后，按 ADR 0023、0025 的权限链处置。
