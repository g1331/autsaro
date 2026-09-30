---
title: '4.18 映射标准OS入口声明及实际生成代码段'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '780ec8d341bd89ed6e8c54ae8399cd193fbf541f'
context: []
---

<frozen-after-approval reason="用户已授权完整Epic4及逐独立增量持续实施">

## Intent

实际生成OS的Task与Hook声明尚未按SWS_Os_00815使用Memory Mapping Allocation Keywords，标准ISR和AlarmCallback入口也缺少对应目标布局。提供可重复包含的Os_MemMap.h，并使标准入口声明及生成Task／Hook实际链接到目标代码段。

## Boundaries & Constraints

固定Win64 GCC16.1.0、C99、单核SC1及现有公开ABI不变。使用未配置SwAddrMethod时的默认CODE关键词；输入如指定本剖面未支持的代码位置仍明确拒绝，不伪造任意MCU布局支持。OS_START_SEC_CODE／OS_STOP_SEC_CODE成对包围相应声明，目标OS_CODE属性保持函数名和签名；重复包含可用于多对声明。已有32槽.os_vec、真实调度、Hook、ISR和Alarm语义保持。SC1没有受信函数／OS-Application，不扩展SC3。依据实际代码使用MISRA指导，BMad记录验证与复核，不创建额外文档或报告体系。不推送；此增量不关闭Counter服务和后续故事。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| Task、ISR、AlarmCallback与五种Hook的有效标记／声明；真实构建运行 | ABI及行为保持，实际函数位于只读可执行.os_code段，独立符号检查匹配 |
| 生成并搬移参考工程；构建和实际运行 | Task_Ecu及真实Error／Pre／Post／Startup／Shutdown Hook的声明标记、源码与链接位置一致；来源树保持不变 |
| 重复合法START／STOP对 | 每对独立消费，宏不泄漏活动状态，不影响头文件不同包含顺序 |
| 嵌套START、无START的STOP、同时START与STOP或未支持标记 | 编译明确拒绝，不静默消费错配 |
| 去掉实际入口段属性的编译副本 | 独立符号／段检查拒绝；包装文字不足以通过 |

</frozen-after-approval>

## Code Map

- runtime/os/include/Os_MemMap.h：重复包含标记、成对状态检查与固定目标代码属性。复用既有头文件重复包含惯例，实际添加段映射。
- runtime/os/include/Os.h、Os_Hooks.h：TASK／ISR／ALARMCALLBACK属性；五种标准Hook原型使用CODE标记；不改类型或参数。
- runtime/ecu/templates/Ecu_Config.c.in、src/Ecu_Target.c、Ecu_OsHooks.c：生成Task及实际Hook定义的标记，修改生成源。
- core/src/integration/offline.rs：交付新头文件；ecu.rs保持实际产品使用说明。
- runtime/os/tests/entry_bodies.c、scripts/epic4_os.py：实际Task／ISR／AlarmCallback／Hook符号段检查及编译负例；既有11入口／26嵌套／10清理回归复用。
- core/tests/support/epic4_ecu.rs：实际生成／搬移构建后独立读取Task和Hook段，不靠生成字符串代替链接结果。

## Tasks & Acceptance

- [x] 实现重复包含CODE标记与固定目标函数属性，包围标准入口声明。
- [x] 更新真实生成源及交付清单，保持ABI与向量段。
- [x] 完成实际链接检查、错配编译拒绝与无映射副本拒绝，运行受影响回归。
- [x] BMad三路复核与本地提交，父story保持准确在途状态。

## Implementation Notes

已读取本地R24-11 OS原件p393～394，关键词使用Os_MemMap.h，已配置时使用SwAddrMethod短名；本参考配置没有代码位置引用，采用CODE默认段。由主代理实施；独立代理只读复核。

## Spec Change Log

## Review Triage Log

- edge: 预定义OS_CODE可绕过固定段映射，medium／patch。实际以-DOS_CODE=编译原消费者成功且.os_code不存在，确认分支真实。首包含时拒绝已定义OS_CODE，私有属性初始化标记允许后续合法重复包含；增加精确诊断编译拒绝用例。blind无发现，verification-gap无缺口。

- 追加edge复核：同时预定义私有OS_MEMMAP_ATTRIBUTES_DEFINED与空OS_CODE会绕过初次初始化，low／reject。复制编译确认该人为私有状态覆盖可改变裸C消费者，但源码检索没有合法生产者／配置入口定义此私有标记；实际固定产品构建仍用最终符号／段检查拒绝未映射入口。该情形要求同时伪造内部初始化状态，正常用户路径不遇到；追加状态防护不作为本增量要求。建议片段的!defined(OS_CODE)也无法检测已定义的空宏，不采用。普通OS_CODE覆盖问题已修正且编译拒绝通过。

## Verification

正式测试新增Memory Mapping编译／符号／拒绝入口；运行既有11标准入口、26嵌套、10清理和真实生成／搬移构建测试，正负预期保持。运行Python单测、增量格式/C99和核心Clippy。编译副本及消费者均在临时目录，结果进入本BMad规格。

编码指导实际采用R8.4／8.6的唯一声明与定义、R20.1／20.5的Memory Mapping必要include／undef理由；GCC属性限定固定目标。没有全MISRA符合声明或新增偏离审批档案。

最终7个正式exact入口均恰选1项通过：MemoryMapping0.59s、entry8.29s、nested9.59s、cleanup9.46s、publicCompatibility7.91s、publicTypes2.87s、生成／搬移构建258.27s。10个真实链接入口执行成功；四种错误标记编译拒绝，去掉段属性的可编译副本在实际符号／段检查拒绝。生成Task与五Hook位于.os_code，32槽向量段仍通过独立检查。15 Python、核心Clippy及增量格式/C99已通过，后续说明文字更新不改变这些运行逻辑。

最终修正复验：MemoryMapping0.57s（10入口／6拒绝）、entry7.74s、publicCompatibility6.53s、publicTypes2.47s及实际生成／搬移构建240.41s通过，15 Python和增量格式/C99再次通过；嵌套与清理实现及正常宏展开未变化，复用已有效结果。没有待修正的本增量问题，无递延工作。父4.18的Counter服务与配置一致性仍继续实施。
