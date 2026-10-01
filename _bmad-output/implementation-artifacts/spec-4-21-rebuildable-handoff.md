---
title: '4.21 新目标可重建交付与工作台实际状态'
type: 'feature'
created: '2026-10-01'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '692a65530f41ebb4b58ff06a3fad1c5f7abd0909'
story_key: '4-21-交付新目标可重建包与准确工作台状态'
context: []
---

<frozen-after-approval reason="持续完成Epic4授权涵盖规格、实施、验证和本地提交">

## Intent

交付一个能够搬移、重导入、再生成、独立构建和复验的新目标工程，并在真实工作台接通预览、导出、构建与行为检查。接收者能依据固定输入和源码重现CAN/DID与失败行为，界面清楚显示各阶段已实际完成什么。

## Boundaries & Constraints

保持真实FreeRTOS／Win64、单核SC1接口与唯一计划，host-v1读取及旧离线入口继续有效。新格式显式版本化，输入逻辑路径和原始字节保持；校验合法外部XSD/MOD与固定源码／工具身份，包内许可和必要依赖闭合。脏输入、外部变更、篡改和用户输出受到保护；不打包密钥、运行状态、官方原件或源机器路径。BMad为唯一流程与状态来源，不恢复assurance、审批、证据目录或报告。原生UI／IPC仅在不会干扰用户的隔离桌面验证；不推送。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 已保存的合法新目标／预览确认导出、搬移并重导入 | 输入身份、版次、固定依赖和完整闭包可复核；重新建立唯一计划，生成源码稳定，不把JSON描述当成可信计划 |
| 新目录及声明工具／独立编译和离线检查 | 实际CAN/DID成功与故障向量成立；依赖按许可交付或明确外部取得，工具结果可复现 |
| 未保存／外部变化／缺文件／篡改或伪造清单／修改旧输出／不匹配依赖 | 定位拒绝，保留输入与旧包，不伪造生成、构建或行为通过 |
| 工作台执行、失败、编辑或重开 | 分开呈现保存、校验、生成、构建、主机行为、当前工程等级复验及实机状态；下游结果随相关变更失效，不由生成或构建推断完整SC1／实机通过 |

</frozen-after-approval>

## Code Map

- `core/src/integration/ecu.rs`／`handoff.rs`（新增）：复用封闭源码与seal_files／preview_prepared；新格式元数据、重导入、构建及离线入口。
- `core/src/generator.rs`：复用清单／路径／摘要保护，host-v1路径保持；`arxml.rs`／`arxml/integration_editor.rs`连接已保存输入及逻辑根。
- `runtime/ecu/build.ps1`及新verify／独立消费者：已有真实编译、段检查和固定工具；规范原件外置，产物使用相对来源身份。
- `src-tauri/src/lib.rs`：异步blocking线程中连接实际新目标命令和工作区锁。
- `ui/src/IntegrationDelivery.tsx`（新增）／`App.tsx`／`types.ts`：复用预览、路径输入与确认，接入新目标动作和实际阶段状态；沿用现有样式。
- `core/tests/support/epic4_handoff.rs`（新增）、正式入口及已有host-v1负例：跨目录复现和保护断言。
- `scripts/epic4_desktop.py`／CDP：已有独立Windows桌面、私有WebView2、真实IPC及输入桌面不变检查，扩展实际新目标路径。

## Tasks & Acceptance

- [x] `core/src/integration/handoff.rs`／`ecu.rs`／`generator.rs`／`arxml/integration_editor.rs`：显式格式、输入与固定源码身份、重建校验及保护旧输出。
- [x] `runtime/ecu/build.ps1`／`verify.ps1`：包内成功、诊断恢复和失败验证，编译与来源可在新目录复现。
- [x] `src-tauri/src/lib.rs`／`ui/src/IntegrationDelivery.tsx`／`App.tsx`／`types.ts`：真实预览、导出、重导入、构建和主机行为；准确阶段及失效反馈，等级／实机缺验证保持未验证。
- [x] `core/tests/support/epic4_handoff.rs`／`scripts/epic4_desktop*`／`README.md`：正式`epic4_independent_handoff`、关键拒绝、host-v1回归及隔离UI／IPC；结论进入本BMad工件。
- [x] 当前BMad spec及`sprint-status.yaml`：所有适用检查有实际结果，未完成保持未完成。

Given声明固定工具和原始合法输入，when接收者从新目录重开、生成、构建并执行实际独立检查，then完整源码稳定、主机向量与拒绝成立，真实界面各阶段结论与动作一致。

## Implementation Notes

`autosar-ecu-handoff-v1`从包内原始输入重建计划并比较完整产品字节；工作台构建／行为检查同样依据当前已保存计划核对闭包，不能通过重写摘要绕过。源码重建与编译预检共用同一生成来源，公开生成入口仍要求真实链接预检。核对构建前后来源，实际构建和HostBatch运行在后台；没有审批状态或报告写入。

交付面板保留跨页的实际结果；输出／构建路径变化按影响清除下游结果，输入变化或重导入清除全部下游。标准输入面板保留挂载，使未应用草稿跨页保留；导航忙状态与实际后台工作一致。沿用现有types与样式，不建立第二套开发状态。

## Spec Change Log

## Review Triage Log

| 来源／发现 | 判定 | 核实与处理 |
| --- | --- | --- |
| edge：工作台验证无外层等待期限 | medium／patch | 原`.output()`确实无界；改为私有文件捕获、270s整调用watchdog及仅关闭本次进程树，真实停滞父子进程测试已通过。 |
| edge：完成验证后临时构建树保留 | low／patch | 每次唯一目录没有回收属实；改为独占临时目录，按原保留路径及直接临时父目录核对后回收。用户声明的构建目录保留。 |
| edge：失败验证后临时树保留 | low／patch | 与上一项同根因；失败／启动失败同样回收，清理失败在实际日志中说明，不伪造行为结论。 |
| edge：工作台构建无外层等待期限 | medium／patch | 原同步进程等待无界；使用同一受控执行函数，180s编译watchdog，不改变汽车逻辑时间或5000msCOMMIT要求。 |
| acceptance：离线build不受`TimeoutSeconds`控制 | medium／patch | 原计时器仅在build后启动属实；离线脚本增加独立180s构建期限并关闭本次树，60s行为期限保持；复验真实停滞工具失败及后代退出。 |
| blind：正确帧加错误额外帧仍通过 | high／patch | 原函数只统计目标匹配，确会接受额外OUT；改为完整epoch／ID／DLC／字节多重集合，保留独立配置的周期输出预期，增加真实原生生产入口的额外帧编译故障。 |
| blind：验证临时树持续积累 | low／patch | 同临时目录根因；成功／失败／无法启动的私有树回收测试已通过。 |
| gap：旧host-v1新Tauri分派无IPC回归 | medium／patch | 已有核心回归绕过改变的命令属实；隔离流程使用真实旧包经原生分派打开并核对工作区，未知格式拒绝保留旧工作区。 |

四路均完成全范围审查；blind初次标题`partial`经原代理澄清，已完整读取1941行并完成相关调用追踪，无未完成范围。上述修正已实际复验，未递延本story发现。

修正复验中的1ms合法输入暴露既有生成器将应用与工作循环按周期混为一组，导致生成拒绝。已按计划中的不同OS Event识别固定工作循环，保留其同alarm／event约束；应用／Com在相同1ms周期时仍属于独立组。原始包和输入保持，复制输入进行这一实际边界检查，不降低或跳过断言。

## Verification

运行正式入口`epic4_independent_handoff`及受影响生成／保护／旧host-v1回归，UI lint／严格构建、Rust Clippy与增量检查。以隔离桌面运行真实原生路径并检查实际画面及IPC结果；临时截图／中间文件仅临时目录，普通测试不写受版本管理报告。非实现者完整交接重复由4.22承担，不提前标Epic done。

核心已接入显式新格式、固定依赖、嵌套逻辑输入身份及重新生成字节比较。首次往返识别Workspace canonicalize路径与声明逻辑根的Windows前缀差异，已使验证后的包根规范化；不放宽输入根边界断言。

最终核心`epic4_rebuildable_handoff`退出0，345.13s：嵌套逻辑路径／搬移再生成字节一致、独立生产HostBatch CAN/DID／N_Cr恢复／非法批次、二进制无来源／构建机器路径、篡改及改摘要后的重导入／执行拒绝、旧输出保护、缺文件／额外文件／未知格式、外部输入修改、缺外部MOD及脏输入拒绝。旧host-v1搬移构建、往返、脏输入／旧包保护及junction回归均通过；5项handoff筛选合计357.04s，后续只修改新格式实现。

UI lint／严格build、核心与桌面Clippy、桌面实际build、15项Python及增量格式／C99检查均退出0；24技能安装锁检查通过。已核对当前有效规则／默认脚本：assurance目录、专属脚本与CONTEXT.md没有版本管理文件，旧固定路径仅见本次清理规格的删除对象记录。验证没有产生受版本管理报告。

隔离原生路径已通过预览／取消／导出／编译／行为／搬移重导入／稳定再生成。最终隔离路径退出0：真实生成取消、非空构建目录拒绝且旧二进制不变、源码修改后行为失败并恢复、草稿跨页保留／下游失效／还原，以及新目录搬移重导入／再生成字节一致全部通过。实际画面已检查，按钮、路径、阶段与失败原因可读；完整SC1／实机均保持未验证，输入桌面未改变。

受影响正式生成入口epic4_ecu_integration_generation最终退出0（464.88s）；包含实际生成／搬移构建与相关拒绝路径。以上验证对应当前代码，所有矩阵行已有运行通过的覆盖。

复核修正后的正式交接退出0（397.35s），完整输出及真实额外帧故障、离线构建超时及后代退出均通过。私有目录成功／失败／启动失败回收与真实父子进程watchdog两项单测退出0（2.59s）；当前1ms生成／原生完整输出／N_Cr恢复经独立正式入口epic4_one_ms_delivery退出0（101.03s）。最后隔离真实UI／IPC退出0，补充旧host-v1分派与未知格式拒绝保留工作区，全部成功、失败恢复、草稿和搬移再生成断言通过。

最终私有超时常量调整后的交接正式入口再次退出0（392.59s），源码增量检查退出0，桌面程序重新构建退出0（16.19s）。独立增量复核核对1ms事件组、完整输出集合、私有超时／后代退出与临时目录回收，未发现新问题；没有递延事项。

### Review Findings

- [x] [Review][Patch] 工作台构建及行为调用有界，并回收成功／失败私有目录；覆盖edge四项及blind临时树项。
- [x] [Review][Patch] 离线构建私有180s期限，真实停滞工具及后代退出已复验。
- [x] [Review][Patch] 完整输出集合拒绝额外帧，独立原生编译故障已复验。
- [x] [Review][Patch] 原生IPC复验旧格式分派和拒绝未知格式后保留工作区。

4.22最终审查指出两条story重复注册同一耗时消费者。当前正式入口统一为`epic4_independent_handoff`，原4.21入口更名合并；验证函数、全部行为与拒绝断言保持，历史通过记录仍对应同一消费者。
