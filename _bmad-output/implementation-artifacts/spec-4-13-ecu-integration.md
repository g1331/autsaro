---
title: '4.13 从唯一计划生成可链接 ECU 集成工程'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'dbd82d4a9a286a8279bed24b32d4d4793204f30e'
story_key: '4-13-从唯一计划生成可链接-ecu-集成工程'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

从同一ValidatedIntegrationPlan生成独立Windows x64/GCC工程，闭合标准RTE、SchM、BSW配置、OS元数据和启动任务，使完整文件集合可迁移后编译、链接及独立验证启动，无需手补符号。

## Boundaries & Constraints

4.3–4.9和4.10–4.12实际门已通过；固定FreeRTOS、补丁、编译器和单Task_Ecu所有者，不重选OS或引入第二调度器。只消费已验证计划和固定源码清单，不重新猜XML关联。复用4.11契约、安全预览/安装和来源身份；离线工程包含原始输入、生成/复用源码、内核与补丁来源、许可及确定性清单，不携带官方XSD/MOD或工作区绝对路径。

AUTOSTART任务与两个Alarm从ECUC生成，Ready前关闭tick/通信。StartupHook按驱动→CAN/CanIf→路由/CanTp→Com→Dcm→RTE/应用→控制器STARTED的依赖建立；不调用禁止激活/事件/Alarm服务。每阶段失败发布Failed并ShutdownOS，真实栈/端口故障仍使用W1控制面。Task_Ecu按已固定输入/确认、驱动、CanTp、Com deadline、应用、Com Tx、Dcm顺序执行；只能在已证明单所有者的BSW区域消除SchM锁，mailbox/OS保留原子协议。

不链接旧host-v1 Os/Rte入口，不用空壳或手补桩冒充运行闭包。必要BSW适配明确区分标准接口和复用内部主机实现；保持旧目标回归。参考应用使用真实读/写/快照/同步服务实现，4.15仍负责完整应用行为验收；4.14负责HostBatchV1文本控制和输出确认闭环，4.16负责独立通信向量，当前不提前声称这些门通过。不扩展Epic2/5/6、不推送，不升级完整SC1或Epic状态。

## I/O & Edge-Case Matrix

| 输入／状态 | 预期 |
| --- | --- |
| 七文件正例、合法名称/ID/周期变体 | 同计划生成，标准函数签名和单生产者闭合；重复生成字节集合一致 |
| 工程移到新目录 | 固定工具链从工程内来源编译链接，独立控制harness验证启动及周期顺序 |
| 任一初始化阶段真实拒绝/测试失败注入 | 只发布Failed，通信/tick保持闭锁并关闭，无Ready或继续运行 |
| 缺/重复周期入口、服务责任冲突、跨任务共享或时间来源冲突 | 生成前拒绝，旧工程字节不变 |
| 来源改动、过期预览、输出被编辑/含用户文件 | 拒绝安装，旧输出完整保留 |

</frozen-after-approval>

## Code Map

- core/src/integration/ecu.rs及mod.rs：同一计划的工程文件消费者，复用contracts.rs和generator.rs的seal/preview_prepared/generate_prepared。
- runtime/ecu/：新目标启动、BSW签名/时间适配、单所有者SchM与任务循环；与runtime/os/实际backend相接。
- runtime/include、runtime/src及runtime/contracts/bsw-v1.json：复用编码/传输/诊断来源；适配异步确认、显式目标时间和计划诊断计时，保留host-v1契约及清单身份。
- third_party/freertos、runtime/os、scripts/epic4_os.py：固定原始源码/补丁/许可、实际目标配置和独立控制验证参照；不修改原始vendor源码。
- core/tests/support/epic4_ecu.rs及end_to_end.rs：正式epic4_ecu_integration_generation入口，新目录独立编译/链接/启动与拒绝测试。

## Tasks & Acceptance

- [x] 确定性生成工程、全部来源/许可/配置及标准符号闭包；同计划变体和拒绝路径。
- [x] 单Task_Ecu、依赖初始化、AUTOSTART/Ready与故障关闭；实际BSW/RTE/SchM边界。
- [x] 离线构建入口与独立控制harness；重复/迁移/每阶段失败/旧输出保护。
- [x] 适用MISRA人工及部分工具核查、完整增量门、三路独立复核，同步sprint及本地提交。

## Implementation Notes

4.12提交dbd82d4，最终29 Python/3单元/76集成、UI、双clippy、桌面构建及实际隔离原生IPC全部通过，当前树干净。W1最新controlled-time源码摘要全部匹配；本轮所有W1正式入口再通过。两路只读探查确认没有现成W3实现；复用生成器安全安装和W1内核打包，但不能直接调用旧Ecu_Init/Os_Advance。旧Com_TriggerTransmit要求同步确认，CanIf默认取Os_Now，Dcm会话响应计时固定为50/500ms，须在新目标适配中落实计划/单所有者契约，不能把这些旧行为当成新目标正确性证据。

已实现同计划工程消费者、嵌入固定内核/OS/BSW来源、单Task_Ecu与StartupHook、原子输出/确认环、真实RTE/应用模板、独立build.ps1及20tick控制probe。原始FreeRTOS的StackMacros.h/stack_macros.h大小写别名分别保存在kernel-compat与活动kernel路径，原始两份字节及路径映射保留，避免Windows目标冲突。构建只修改独立输出目录的内核副本，按序检查/应用七补丁，并核对固定GCC和PE TLS。旧目标由明确ECU_TARGET_EPIC4编译变体隔离；新目标发送句柄来自计划并在PduR映射回内部帧域。

首轮迁移构建发现Get-FileHash在当前Windows PowerShell环境不可用，改为直接.NET SHA256；随后补齐两个BSW私有头、使用W1已有Os_Windows避免SetEvent命名冲突，所有者检查复用实际Os_BackendTaskOwner而非调用尚未实现的GetTaskID。第二轮扩展拒绝测试发现共享生成器只认固定include/src/inputs目录，已改为仅允许清单文件的祖先目录；用户额外空目录仍拒绝，未放宽用户文件/重解析点/完整性守卫。

正式exact最近一轮退出0：恰选1项、其余76项未运行，67.61秒；重复字节集合、七文件原始来源、迁移GCC编译链接、实际20tick/两次周期输出/固定顺序、8初始化阶段失败、18封存反例、被编辑旧输出与额外用户目录/文件保护均通过。源质量门已通过一次；之后新增生成CLI、拆分真实应用/RTE模板、原生所有者DWORD发布与正常Closed状态及Com/RTE状态分层等变更，最近exact已覆盖这些行为。Com API语义直接核对本机R24-11 SWS_COM pp106–109，Com只返回自身接纳/不可用结果；首次接收与年龄结果由RTE独立映射，不把RTE状态冒作Com返回值。

用户另明确要求删除CONTEXT.md，已核对根目录文件、实际删除、确认相关活跃文档无引用，并单独本地提交5e1ecc12e5fa2006b776cbeaed7a57de0449c62c；本故事起始提交仍保留dbd82d4，不改写既有历史。

最新exact退出0：1项、76项未运行，255.44秒。新增合法ECU/Task/应用/端口/数组类型名称、反向信号排序、CAN ID、CanIf句柄、20ms周期和非连续位置编号变体均实际生成、编译、运行；四个外部符号与Os_TargetConfig类型竞争被完整GCC闭包拒绝，未占用的Ecu_应用名仍接受。256句柄拒绝CANIF_HANDLE_WIDTH，错误位置顺序拒绝SCHEDULE_ORDER；全部25条原BSW来源都有可验证交付映射，Com.h和旧Os.h原字节保留在bsw-origin，不进入旧调度器链接。

独立工程外C控制消费者通过真实生产模式公共头/库链接，固定预期验证两次Tx，以及默认会话DID在应用提交前/后的两组SF字节。该检查实际暴露LSduR遗漏初始化，已补入StartupHook路由阶段后通过。CLI实际预览/按revision安装通过；实际生成Ecu_Config、Ecu_TargetConfig、RTE、Application的clang-format检查通过。新控制夹具最初放入W0封存目录，完整门拒绝文件集合变化；已移到独立core/tests/fixtures/ecu_control.c，保持封存清单和29Python测试原契约。

部分Cppcheck2.21/C99/win64扫描覆盖生成src中18个真实文件，原始诊断和源码摘要保存在既有证据位置；未宣称符合或自行批准偏离。根据首轮诊断修正邮箱循环中赋值表达式、跨类型memcpy显式void边界、回调参数名与探针printf错误处理，C99邮箱容量编译约束现在用实际输入存储类型保留。首轮扫描代表修正前源码，最终来源与诊断尚需刷新。W1最新time重新通过40个native vectors并更新controlled-time的实际来源摘要。

完整门退出0：29Python、3单元、77集成（320.43秒）、UI lint/build、核心和桌面clippy/build全部通过。三路只读独立复核收齐后确认两路同一时间戳接纳缺陷，已在入队前用既有完成记录及单调生产者epoch拒绝，独立消费者覆盖拒绝后仍继续运行。最终exact退出0：1项、76项未运行、259.72秒；最终源码质量、双clippy和桌面构建复验退出0。CLI再次实际预览/安装、工程外生产消费者运行、缺失/过期revision拒绝、生成C格式、全部原来源映射和当前扫描身份核对完成。共享证据ecu-integration.json与ecu-integration-static-analysis.json保留限定通过与真实部分规范诊断。4.13完成；Epic4仍in-progress，4.14–22、完整SC1/MISRA及交接出口仍全部适用。

## Spec Change Log

## Review Triage Log

| 来源 | 判定／路由 | 依据与处理 |
| --- | --- | --- |
| blind：PostFrame时间戳先接纳后关闭 | medium / patch | 已沿入口、mailbox和consume验证可达；入口原只查ID/DLC，错误epoch入队后fail关闭。改用原OS完成记录限制当前/下一epoch，并由同一原生生产者保留单调接纳epoch，入队前拒绝，ticket不变。独立消费者新增过远、UINT64_MAX、倒退和未确认tick拒绝后仍继续通信的覆盖。 |
| edge：PostFrame时间戳越界 | medium / patch | 与blind同一根因，但逐项保留此结论；核对owner不能接受早于已处理或大于下一ms的输入。修正没有跨线程读汽车私有epoch，也没有引入第二时钟／新公共服务；Os_TargetTickCompletion绑定既有bridge生产者。 |
| verification-gap | 无发现 | 独立审阅返回complete、No verification gaps found；不替代最终代码运行和当前证据摘要核对。 |

## Verification

epic4_ecu_integration_generation exact正式入口；严格C99/GCC16.1编译链接及后台独立进程轨迹、全阶段失败、迁移/确定性和全部输入反例。最终python scripts/verify.py --scope all --base dbd82d4a9a286a8279bed24b32d4d4793204f30e；MISRA C:2012修订基线的适用人工核查及实际生成代码部分Cppcheck，不声称完整符合或实机支持。
