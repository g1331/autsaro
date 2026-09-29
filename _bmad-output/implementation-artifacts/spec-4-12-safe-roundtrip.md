---
title: '4.12 标准输入的安全编辑、保存与重开'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '54f13b1b6c3d57c822c1871e68b54d5213eb3998'
story_key: '4-12-接入标准输入的安全编辑-保存与重开'
context: []
---

<frozen-after-approval reason="Epic4 approved intent and autonomous story execution">

## Intent

让工作台检查、编辑、保存及重开标准Epic4多文件输入，显示实际来源角色、唯一核心计划和诊断位置。支持同一S/R通信链的CAN ID，以及应用/ComTx/对应Alarm共同周期；保留所有未修改原字节和有效无关对象，不通过UI补映射。

## Boundaries & Constraints

依赖4.11已完成，保持固定目标及旧host-v1。PlanDescription/PlanDiagnostic来自4.10唯一构造器；编辑根据计划中的完整路径定位原XML文本范围，不重新猜关联。支持受验证剖面的Rx/Tx CAN ID及应用周期；诊断ID、类型/端口/引用及其他不支持字段保持只读。周期原子修改应用TimingEvent、同Alarm的BSW TimingEvent、ComTxMode及Alarm start/cycle，修改后重新校验同一计划，不接受部分成立状态。

预览显示所有受影响来源；保存前重新检查计划/来源/revision，复用已成立的暂存、备份、故障回滚机制。拒绝外部编辑、断引用、混合XML值、影响目标变体、未知必要参数及不安全关联。新路径不得以legacy校验器代替标准计划。编辑失败恢复内存原文，保存失败保留原文或可恢复备份，不宣称运行工程已经生成。

界面沿用现有布局、导入与保存预览，标准目标状态明确，与旧目标不混用生成/编辑入口。官方XSD/MOD为外部提供，不复制。原生Tauri/IPC仅在实际隔离桌面验证；无法隔离时not_run且保持相关门未通过，不启动交互桌面窗口或抢焦点。W3须在全部适用关键门通过后才进入。不扩展其他Epic、不推送。

## I/O & Edge-Case Matrix

| 输入／状态 | 预期 |
| --- | --- |
| 七文件正例、合法CAN ID/周期修改 | 原子跨文件更新，计划和角色可见，预览不落盘，保存/重开参数与计划一致 |
| 同文件保留对象/属性/注释与未改文件 | 仅受支持值范围改变，其余字节保留；无关有效内容不参与生成映射 |
| 外部编辑、预览后新编辑、保存中断/备份冲突 | 定位拒绝，外部字节保留；原文或恢复备份完整 |
| 重复CAN ID、非法周期、错类型/断链、变体、未知编辑对象/字段 | 诊断带类别/文件/对象/补救，编辑或保存拒绝，不保留部分改动 |
| 缺角色/XSD/MOD | 区分必需/可选/外部提供及不可生成状态，不把未运行当成功 |
| UI操作/原生IPC | UI实际渲染和关键编辑/预览/重开流程；原生IPC须有隔离位置证据 |

</frozen-after-approval>

## Code Map

- core/src/arxml/integration_editor.rs：作为arxml子模块使用既有SourceFile/Patch/patch_child/patch_param及save_revision；新增标准计划报告、受支持编辑、专用保存预览/保存。
- core/src/arxml.rs：挂载子模块，将已有save中验证后的文件安装提为共享私有save_sources，旧save保留验证和返回行为。
- core/src/integration/editor.rs、mod.rs：强类型edit请求、计划报告与从计划建立精确字段修改列表；不允许未验证计划构造。
- src-tauri/src/lib.rs：标准计划/编辑/保存预览/保存IPC，使用同一AppState工作区及runtime/MOD来源。
- ui/src/IntegrationPanel.tsx、types.ts、App.tsx：现有编辑页内显示角色/参数/诊断及标准保存预览，标准目标使用受限入口；旧目标保留原有使用方式。
- core/tests/support/epic4_roundtrip.rs、end_to_end.rs及无头交互脚本：正式往返、拒绝/原字节/保存故障与桌面IPC验证。

## Tasks & Acceptance

- [x] 核心同一计划检查和多文件受支持编辑，失败回滚及原字节保持。
- [x] 复用安全来源/revision/保存安装，正式往返及所有关键拒绝路径验证。
- [x] IPC与现有UI接入角色、诊断、编辑和预览/保存/重开；不补隐含映射。
- [x] 无头UI及适用隔离原生IPC，完整增量门、三路独立复核通过。
- [x] 同步sprint、本地提交并核对W3进入门；未过出口不标done。

Given完整合法输入，When标准检查/编辑/保存/重开，Then来源角色、参数和同一核心计划一致，未改字节保持。

Given危险输入/外部修改或失败保存，When操作，Then拒绝并提供定位/补救，原始或可恢复内容完整。

## Implementation Notes

4.11最终75集成/29Python/3单元、UI lint/build、双clippy和桌面构建全部通过并提交54f13b1b6c3d57c822c1871e68b54d5213eb3998。只读定位已确认旧update_frame周期仅改Com，并拒绝标准CAN trigger的ID修改；新入口须完整处理关联。现有save包含跨文件暂存/备份回滚和外部改动二次检查，复用其文件安装而不复制。Microsoft官方桌面接口允许通过CreateDesktop与STARTUPINFO.lpDesktop分配独立桌面；后续须验证实际桌面归属和输入桌面未改变，不能凭启动另一个进程声称隔离。

## Spec Change Log

## Review Triage Log

三路default/fork-none只读独立复核全部收齐后处理：blind与gap均complete且无发现；edge提出一项命名空间遮蔽意见。

| 来源／定位 | 判定与依据 | 处置 |
| --- | --- | --- |
| edge／core/src/arxml/integration_editor.rs:62–64：外部命名空间同名子元素位于AUTOSAR值前，编辑可能只改扩展值并成功 | false：edit_integration第一步integration_plan强制固定摘要的R24-11 XSD校验，Workspace::open也先校验；IDENTIFIER、PERIOD和ECUC VALUE处均不能插入这种外部命名空间元素。新增三个实际XML输入使用正式schema::validate_files验证，全部命中R24_XSD；最终epic4_safe_input_roundtrip在76项完整集成测试中通过。该输入无法到达patch_child，未发生所称成功编辑。 | 保留有效入口守卫，不修改产品编辑语义；拒绝反例作为稳定契约纳入正式测试，无延期项。 |

## Verification

epic4_safe_input_roundtrip exact；UI严格类型/lint/build、实际无头交互和截图；隔离原生Tauri/IPC证据；最终python scripts/verify.py --scope all --base 54f13b1b6c3d57c822c1871e68b54d5213eb3998。存在未验证关口则如实记录并保持W3关闭。

核心epic4_safe_input_roundtrip exact最终恰选1项通过（其余75项未运行）：独立literal原文比较证明四文件仅目标数值范围改变，跨文件共同周期、未改字节、保留注释、重复ID/越界/未知对象、失效预览、已有备份和外部编辑拒绝。增加缺MOD依赖类别、不完整输入及合法带数字注释的混合内容拒绝；混合内容编辑整体回滚。一次新增依赖断言误用已被外部编辑的旧Workspace，先命中SOURCE_CHANGED而非MOD，已改为重新打开当前来源的独立Workspace后复验通过，保留正确守卫而未改断言要求。

真实隔离桌面初次GetThreadDesktop跨进程读取拒绝访问，改用持有独立Desktop句柄的EnumDesktopWindows及窗口PID归属核验，窗口仅在新Desktop且输入桌面未改变。Tauri原生invoke不可写，测试替换未生效；放弃替换，通过产品实际来源路径输入完成真实IPC。最终之前两次真实UI流程通过；原生截图已直接检查，修正参数标签/输入的暗色布局、多行路径草稿、来源角色字面名和误导的旧诊断徽标。正在最终源码重新构建后复验，进一步覆盖取消未应用草稿丢弃、不完整来源检查，并保存规范化证据。尚未完成独立复核和完整增量门，不标故事done。

最终原生出口新增实际缺陷并修复：本机tauri-plugin-dialog 2.7.3的init脚本将window.confirm覆写为async，旧UI同步条件把Promise当true。新增ui/src/confirmation.ts通过正式SDK await确认；项目切换、对象切换/新建、诊断配置移除及标准重开使用同一契约，失败显示原因且不丢草稿，等待时锁定相关操作。独立Desktop中的真实原生确认框由仅针对本次Tauri PID、独立Desktop枚举的Cancel按钮取消；实际脚本确认取消一次、21ms未应用草稿仍留在原工作区，然后还原后切换成功。缺Extract的单类型文件导入也能进入标准检查并显示TARGET_NOT_UNIQUE，不依赖candidate自动识别才可检查。实测原生确认与实际来源路径输入均未替换Tauri transport。

最终原生脚本退出0，共享证据standard-input-roundtrip.json的13个产品/验证源码摘要与当前工作树一致，原生取消确认一次、输入桌面未改变、job子进程终止和Desktop句柄关闭均核实。最终原始记录/截图在.scratch/epic4/desktop-737b7fa200be4004b3e29bf015ef825e。核心exact、UI lint/build、增量质量门通过；三路独立复核和最终完整增量门待完成，W3仍关闭。

复核后的完整增量门已通过29 Python、3 Rust单元和76集成测试（243.02秒），新增命名空间反例通过；UI lint/build及增量质量检查通过。产品/原生测试脚本未因本次新增核心测试改变，13个原生证据源码摘要再次全部匹配。双clippy及桌面构建正在最终门继续执行，全部退出0前保持in-review及W3关闭。

最终完整增量门退出0：上述全部测试、core/desktop clippy及desktop build全部通过。针对最终构建二进制再次执行真实隔离原生IPC退出0，记录位于.scratch/epic4/desktop-33b73b1ec22f4bec84042fa9662cb2f4；共享证据已更新为该二进制与13个源码摘要，三张实际截图已检查。取消未应用草稿、不完整输入、保存与重开均通过，输入桌面未改变，子进程和Desktop句柄已关闭。三路复核无未处理或延期发现，4.12完成。

W3进入审计：4.3–4.9与4.10–4.12规格和sprint均已done且有本地独立增量提交；4.8 controlled-time证据全部当前源码摘要匹配，本轮76项集成门再次运行所有W1正式入口，包括真实栈关闭、FIFO、Finish/Chain、资源、事件和受控tick。W2原始输入/唯一计划/独立契约编译及实际隔离UI/IPC均通过。因此只放行4.13开发，不宣称W3工程已实现、完整SC1已通过或Epic4完成；4.17–4.22仍保留等级及交接出口。
