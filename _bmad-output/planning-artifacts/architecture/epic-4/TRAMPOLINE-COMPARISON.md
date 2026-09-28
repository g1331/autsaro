# Epic 4：Trampoline 与 FreeRTOS 自有语义路线比较

核查日期：2026-09-28。范围是方案比较，不是依赖替换或功能开发。本轮下载固定源码并只读核对机制、测试、端口、生成器与许可；没有构建或运行 Trampoline，不把其测试源码或历史 CI 视为本项目验收。

## 判断

**Trampoline 在汽车 OS 语义复用方面优于 FreeRTOS＋从头实现汽车语义，已完成主要对照核查，定案后保留为参考和备选。它不是当前原生 Windows 离线交付的现成替代品。**其主要成本集中在 GPL/模板交付组合、原生 Windows 端口、R24-11 差距和 Rust→Goil 集成；FreeRTOS 路线则集中在自有语义与内核策略扩展。

若接受 GPL 内核，并允许 Linux/POSIX 验证，推荐优先验证 Trampoline。若原生 Windows 和宽松许可同时是硬条件，FreeRTOS 更贴合这两个边界，但完整汽车语义的工作量更大。若接受 GPL 且仍要求原生 Windows，必须比较“Trampoline 新端口＋标准差距”与“FreeRTOS 策略扩展＋完整语义实现”的实测工作量；本轮不能保证前者更便宜。

两条路线都能通过合法固定源码、fork 和自有维护获得源码控制，不要求独家供应商服务。许可选择自由与源码可维护性是不同维度，不能因 GPL 就判定被厂商卡住，也不能因公开源码就推定交付权利闭合。

## 固定来源与证据范围

实际 git ls-remote 的 HEAD/master 为 ff287023252bf73273067f00d2dbc8bbc9a20401；[固定源码](https://github.com/TrampolineRTOS/trampoline/tree/ff287023252bf73273067f00d2dbc8bbc9a20401)。归档 SHA-256 为 2ae36637b34e5988b85249e0a16bbe4a7a69b5fd1a6994a67f93b3722c6b9b40。完整源码仅在本机 TEMP，不进入项目依赖或产品包。

FreeRTOS 对照依据为已完成的 [隔离可行性调查](FREERTOS-FEASIBILITY.md)。双方证据深度不同：FreeRTOS 有本机研究实验，Trampoline 本轮只有源码/配置/测试阅读及上游 CI 核查；两者均未证明本项目完整 SC1。

## 工作量比较

| 维度 | Trampoline 固定源码事实 | FreeRTOS＋自有汽车 OS 语义 | 对 Epic 4 的含义 |
| --- | --- | --- | --- |
| 起点 | 声明 API 对齐 OSEK/VDX 和 AUTOSAR OS 4.2 | 通用 RTOS，汽车服务与策略自有实现 | Trampoline 减少语义开发；4.2 不是 R24-11 通过 |
| 激活/顺序 | ready job 带 priority/rank，每次激活有 job，有配置上限和 activation count | 原策略有同级抢占恢复反例；完整请求 FIFO 仍待实现 | 已有相符概念及实现，但需独立边界向量 |
| Chain/Terminate | 内核锁内检查，Chain 成功才终止，失败恢复源计数 | 合法转换有研究证据，生产错误回滚等未实现 | Trampoline 的收益是已有行为实现，不只 API 名称 |
| Event/Resource | task event mask、锁内 Wait 检查；保存/恢复资源优先级及释放顺序 | 需实现 mask/ceiling/LIFO/原子等待 | 可复用汽车协议，仍需核对和运行 |
| ScheduleTable | 已有状态/API 和 Counter/Alarm 关联 | 需自有实现，不能直接改名 timer | 可减少整族实现工作 |
| 测试 | functional 中有调度/状态/错误 assertions 和 expected 记录 | 需建立完整向量，当前只有局部研究 | 上游 fixture 是可用研究资产，本轮未运行 |
| 原生 Windows | 未找到 machines/ 下 Windows OS 目标；POSIX 面向 Linux/macOS | 已有 MSVC-MingW 端口及 x64/GCC 隔离实验 | Trampoline 需新建/取得端口，不静默替换成 WSL |
| 真实栈 | 所查 POSIX 检查无条件成功；Cortex-M footprint 有 TODO | Windows FreeRTOS buffer 不是真实线程栈 | 两者均未关闭主机 SC1 栈门，其他端口逐个判断 |
| 生成器 | Goil 3.1.16、OIL/旧 ARXML→OIL→目标生成 | Rust 自有 OS 元数据及配置生成 | Rust 仍为配置权威，Goil 可作固定 backend |
| 许可 | 所查内核 GPL V2，模板标记混合 | 内核 MIT，自有层许可由产品决定 | 必须核定具体生成/链接/分发组合 |

源码依据：[ready rank](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/os/tpl_os_kernel.c#L249-L285)、[Task/Chain](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/os/tpl_os_task_kernel.c#L153-L225)、[Event](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/os/tpl_os_event_kernel.c#L197-L260)、[Resource](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/os/tpl_os_resource_kernel.c)、[ScheduleTable](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/autosar/tpl_as_st_kernel.c)、[任务测试实例](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/tests/functional/tasks_s9_full/task1_instance.c)。

## 采用前关口

**Windows：**POSIX context/IRQ 使用 sigaction、sigaltstack、sigprocmask、kill、sigsuspend；ViPER 使用 shm_open/mmap、POSIX semaphore、fork/execve。它们涉及上下文、中断、计时和进程通信，不是只改 makefile 就能得到原生 Windows。Goil Windows 工程及 Cygwin CI 针对生成器，不能证明 Windows OS runtime。[POSIX context](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/machines/posix/tpl_posix_context.c)、[ViPER](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/machines/posix/tpl_viper_interface.c)、[Cygwin CI](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/.github/workflows/build-examples.yaml#L28-L46)。

**真实栈：**所查 [POSIX 检查](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/machines/posix/tpl_posix_autosar.c#L92-L104) 无条件返回 1；[Cortex-M footprint](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/machines/cortex-m/tpl_machine_cortex.c#L298-L304) 留 TODO。通用层有调用不代表目标检查完成，任何声明 SC1 的目标仍需真实栈故障证据；本轮不推断未检查的所有其他端口。

**R24-11 与配置权威：**上游仅声明 OS 4.2 对齐。Goil 自带 ARXML 元模型为 4.0.3、4.1.3、4.2.2；[解析路径](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/goil/galgas-sources/arxml_parser.galgas#L101-L143)按版本选元模型并转 OIL，不证明接受本项目的完整 ECU Extract/SWC/BSW/R24-11 ECUC 契约。本轮找到 ORTI，未找到 ARTI 实现；搜索未命中不证明所有版次均没有。采用前须建立完整差距表。可由 Rust 校验计划生成受限、可追溯 OIL/backend 输入，再调用固定 Goil，避免两份配置各自推导状态。

**许可：**[内核头](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/os/tpl_os_kernel.c#L9-L17)明示 GPL V2，[根许可](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/LICENSE-GPLv2.txt)为 GPLv2；本轮未发现内核链接例外。root.goilTemplate 标 LGPL，部分模板标 GPLv2，具体输出须按实际选用文件审查。GPL 覆盖组合的分发须满足相应许可及源码义务；独立工作台、目标固件和模板输出分别分析，不一律视作一个作品。仅使用 GPL 工具不自动决定输出许可，进入输出的模板/运行时代码另行判断。[GNU 输出说明](https://www.gnu.org/licenses/gpl-faq.en.html#WhatCaseIsOutputGPL)、[GPLv2 FAQ](https://www.gnu.org/licenses/old-licenses/gpl-2.0-faq.html)。当前产品本地研发、长期拟开源但未选许可，不能据此自动接受或拒绝 GPL。

**维护基线：**固定 HEAD 对应 [CI](https://github.com/TrampolineRTOS/trampoline/actions/runs/17733066387) 于 2025-09-15 触发，整体 Failure，页面列出 Cygwin 和部分示例失败。这不是全绿交付基线，也不证明内核逻辑错误或全部上游活动停止。本轮 Releases 页面无正式 GitHub release；README 的预编译 Goil 标 3.1.16/2023-12-04，模板要求 3.1.16。包、工具链、模板和子模块须固定复验。[Releases](https://github.com/TrampolineRTOS/trampoline/releases)、[根模板](https://github.com/TrampolineRTOS/trampoline/blob/ff287023252bf73273067f00d2dbc8bbc9a20401/goil/templates/root.goilTemplate)。

## 规划状态

维护者在比较后确认 FreeRTOS 主路线；Trampoline 未采用，保留参考和备选。本文技术事实与条件比较仍有效，不作为继续并列选型的待办。原生 Windows、FR-6/完整 SC1、真实栈、R24-11 及 Rust 配置权威保持；Linux/POSIX 不标作 Windows 交付。Epic 4 架构已 final，故事级就绪 NOT READY，实施状态 backlog，无功能 stories 或能力升级。

会改变选择的取舍是目标工程是否接受 GPL 覆盖组合的交付约束，以及原生 Windows 是否保持硬要求。当前定案保持原生 Windows 和现有交付边界，采用 FreeRTOS 主路线。上述条件只在实际触发主路线停止条件或维护者明确变更边界时重新比较，不继续扩展候选。
