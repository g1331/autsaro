# 用 BMad 开发 Autsaro

在仓库中打开 Codex，说明开发目标，例如“按 BMad 完成 Epic 2”或“继续当前 story”。Agent 根据现有规划、任务状态和实际证据开展规划、开发与复核；需求或验收范围不明确时，先列出缺口和建议。

安装步骤、锁定工具版本和工作台开发命令见[README 的本地开发环境](../../README.md#本地开发环境)。

| 任务 | 示例提示词 | 预期结果 |
| --- | --- | --- |
| 看进度 | “使用 `bmad-sprint-planning` 显示当前状态，并告诉我实际能运行什么。” | 只读的在途、待办、风险和推荐动作；能力声明另行核对。 |
| 推进 story | “按 BMad 完成 Story【编号】。” | 该项开发、验证与复核的实际结果，或明确阻断。 |
| 完成 epic | “按 BMad 完成 Epic【编号】，包括跨 story 集成验收。” | 逐项推进和最终的 epic 验收结论；若规划尚未就绪，先说明缺口与建议。 |
| 不指定任务 | “按 BMad 判断当前最值得推进的工作并执行。” | 基于现有规划和证据选择工作，不把状态文件的下一条机械当成产品优先级。 |
| 改目标 | “我希望用户能【操作】并得到【结果】。请用 BMad 判断现有计划需怎样调整。” | 影响范围、推荐方案和需要决策的取舍。 |
| 看成果 | “使用 `bmad-walkthrough` 带我看这次交付。” | 用途、值得检查的地方及复现方式。 |
| 阶段复盘 | “使用 `bmad-retrospective` 复盘 Epic【编号】。” | 基于 story、提交和运行证据的结论与行动项。 |

`bmad-help` 用于查找下一步工作，`bmad-build` 处理明确的开发目标或 story。委托整个 epic 时，按 BMad 规划逐项推进，并核对组合结果。需求不明确时使用相应规划技能；`bmad-code-review` 用于额外审查，`bmad-retrospective` 用于阶段复盘。委托可覆盖一个或多个工作单元，会话切分按任务需要安排。

长期方向与阶段需求记录在 `_bmad-output/planning-artifacts/` 的 product brief 和 PRD，近期工作拆分为 epics/stories，任务状态记录在 `_bmad-output/implementation-artifacts/sprint-status.yaml`。学习层仍是 product brief 中的待规划方向，旧议题不直接进入开发队列。早期产品地图、决策和研究归档于 `docs/project/archive/` 与 `docs/research/autosar-platform/`；旧任务卡和反馈保存在 Git 历史中，均用于追溯。规划、开发、验证、复核与状态管理使用仓库安装的 BMad，结果记录在对应 story/spec 中。Story 完成不代表相应 AUTOSAR 模块已完整实现。

规范研究应服务于具体功能任务。任务选择同时依据开发目标、BMad 规划、状态和实际证据；以下示例不代表当前优先级。

截至 2026-10-06，Epic 7 的本轮开发验证范围包括单元／集成测试、静态检查、构建和代码复核。真机、原生桌面及完整发行验收不作为本轮开发的前置条件，也不在本轮扩建相关设施。7.1 至 7.10 已完成开发；7.11 保留后续 CI 发行验收，当前为 review。最新开发结果见[中央实施规格](../../_bmad-output/implementation-artifacts/spec-epic-7-configurator.md)，原发行失败与未验范围见[交接文件](../../_bmad-output/implementation-artifacts/epic7-handoff.md)。

开发任务通过 Codex 会话发起，不会在会话委托之外自动继续开发或发送通知。Agent 可作日常技术判断；新规范版次、硬件目标、重大兼容性变化、费用及远端推送与发布须另行确认，并说明建议与影响。

## 汽车工程技能怎么用

仓库提供 12 组、共 24 个汽车工程规格与审阅技能，清单、依赖、来源和限制见[技能说明](automotive-skills.md)。可通过“按 BMad 完成 Epic 3”或“为 Epic 4 设计验证方案”发起任务，Agent 按任务需要选择技能。规格交付需要工作簿时再生成 Excel；普通开发使用 BMad。

| 目标 | 示例提示词 |
| --- | --- |
| Epic 验证计划 | “使用 `$verification-plan-builder`，依据当前 Epic 3 和 story 验收条件编制验证计划，再使用 `$verification-plan-checklist-reviewer` 审阅。区分文档检查、实际测试和未验证项，不改变任务状态。” |
| 需求与测试的关联 | “使用 `$traceability-matrix-builder`，把当前明确纳入范围的需求、规范依据、设计和测试结果整理成追踪矩阵，再用 `$traceability-matrix-checklist-reviewer` 找出遗漏。注明统计范围，不推算整个 AUTOSAR 的完成率。” |
| 诊断规格 | “使用 `$uds-services-builder` 整理当前主机诊断的实际服务、DID、会话和拒绝契约，再用 `$uds-services-checklist-reviewer` 审阅。依据源码和测试区分已实现、待核实和未支持，不用模板补齐支持声明。” |
| Classic 配置设计 | “依据 Epic 4 当前架构，使用 `$autosar-swc-builder` 和 `$autosar-rte-mapping-builder` 整理参考应用及映射规格，并用对应 checklist-reviewer 审阅；对照 R24-11 标注版次差异和待核实项。” |

这些技能生成规格工作簿和文档审阅报告，报告可能包含上游固定模板、建议阈值和待确认判断。代码正确性及完整 AUTOSAR/MISRA 要求需要另外的验证证据。`$技能名` 可用于显式指定技能，也可由 Agent 按任务选择。Codex 自动发现技能变化，列表未更新时可重启 Codex。

## C 编码技能如何使用

功能或修复任务可直接描述，例如“修复 CanTp 的长度校验”“完成当前 OS story”或“修改生成的 C 回调”。修改 C 源码、头文件或 Rust 中的 C 生成模板时，Agent 在设计和编辑前读取 [misra-c2012](../../.agents/skills/misra-c2012/SKILL.md)，并与 BMad 开发流程配合使用；也可用 `$misra-c2012` 显式指定。纯 Rust/TypeScript 改动及只读状态查询不触发该技能。

该技能已允许自动调用。MISRA 完整性与符合性结论仍须依据实际核查范围和规范要求；调用技能本身不构成验证通过。

## 标准 ECU 输入的检查与保存

日常配置使用内置原创 R24-11 规则与受支持模块定义，无需 XSD/MOD 或编译器。可从 `can-empty-v1`、`can-signals-v1`、`standard-ecu-v1` 原创模板创建工程，打开 `workbench-project.json` v1，或一次选择同一输入集合的 ARXML。工程树、文档、检查器和工具窗口共用 Workspace；保存以原 ARXML 字节为准。

标准输入显示来源角色、原字节摘要和目标诊断。有限目标的 CAN ID／共同周期编辑，与按定义的通用字段／引用／结构编辑分别检查。参数 wire 保留 `kind/lexeme`，默认值不隐式落盘；批次先预览 prospective graph 与入站影响，再原子应用。未知条件、变体、表达式或 instance-reference 保持只读或限制相应消费者，切换剖面不会解除这些限制。

保存先检查原文差异，再确认当前预览；外部编辑、过期确认及待恢复备份会阻止覆盖。未改文件保持原字节。直接 ARXML 显式另存为新工程才记录成员与扩展接纳集合；本机缓存存在不等于工程已接纳。`source-safety`、原生有限 `schema`、`definition`、`target-generation` 各自报告，不以结构检查通过推定完整标准或生成／运行成功。

后台原生复验统一用 `uv run --locked python -m autosar_tooling desktop --platform windows|linux|macos --binary <本次桌面构建路径>`。正式包追加 `--installed --source-checkout <已搬离的本次 owned 构建副本>`；必须用 checkout 外的实际解包应用，不得搬动用户工作树。`--builtin-only` 验证无官方档案、无 checkout、无开发工具／编译器的默认配置链，独立工程消费者阶段再提供声明工具；默认兼容／oracle 分支仍使用合法固定规范和旧 v1 拒绝条件。

Windows／Linux 验收不使用用户当前桌面或剪贴板：采用私有 WindowStation/Desktop 或 Xvfb、受管进程树、私有配置／日志与明确的离线隔离，清理只作用于本次 owned 资源。Windows 产品进程保持普通权限；如私有 station／进程专属防火墙需要提升，仅在用户授权后提升验收控制器，不提升被测产品、不关闭整台机器网络。外部 driver 不注入产品 PATH。截图和完整 IPC／清理回执留在私有 scratch，实际结论写对应 BMad 工件。

此前 Windows MSI、Linux deb／解包 AppImage 的隔离复验是历史证据，不自动适用于 Epic 7 当前产物。当前双平台、不同真实输入、原字节／拒绝、性能／取消及独立 CAN/DID/N_Cr／恢复出口须按本次 spec 重新完成，不能由 helper smoke 或 build/test 推定。系统安装／升级、签名、公证、WebView2 首装及 macOS 无宿主范围如实另报。

macOS 入口要求独立的非 console GUI 登录会话与 `cargo build --manifest-path src-tauri/Cargo.toml --features native-webdriver` 测试构建；只验收编辑、校验、保存与纯源码预览，同时检查本机预检／构建／运行按钮不可用。专用 capability 和内嵌 loopback WebDriver 只在此 feature/macOS 组合中启用，生产构建不启用。没有 macOS 隔离宿主时该入口标为未原生验证，不得改在用户桌面补测。

“工作台设置”分别提交外观、规则与模块定义、执行工具类别；内置库存只读，扩展显式接纳并经工程预览保存持久化。外观／工具沿 Tauri `app_config_dir` 原子保存，执行工具环境覆盖优先且不写回；`AUTOSAR_CONFIG_DIR` 指定独立绝对目录，验收不能只覆盖 `APPDATA`。官方 XSD/MOD 属于兼容／开发路径，不替换默认规则。目标／工具／接纳身份变更使对应旧预检与确认失效，主题不重建工程或丢草稿；取消与晚结果按实际 ProcessOwner 归属围栏处理。

## 独立 ECU 集成工程

核心 `generate_epic4_ecu` 命令消费同一个已验证标准计划：`cargo run --manifest-path core/Cargo.toml --bin generate_epic4_ecu -- --target <windows-x64-controlled-v1|linux-x64-controlled-v1> --xsd-archive <合法XSD绝对路径> --mod-archive <合法MOD绝对路径> --output <工程目录> --input <文件1.arxml> --input <文件2.arxml> ...`。先读取 JSON 预览，再附加 `--write --revision <完整revision>` 安装工程；`--handoff` 包含原输入和重建元数据。每份输入分别传 `--input`，不需要 `--repository`。过期预览、来源变化或用户修改会拒绝安装并保留已有内容。

上述 CLI 使用固定官方资源，适用于兼容与开发路径，不是 R5 普通配置的依赖。R5 内置流程从已保存的 Workspace 准备源码，默认交接为 `autosar-workbench-handoff-v2`；重导入在新空目录重建成员，按本机可信规则身份、所需接纳扩展、source／rerender／seal 逐字节核对。原两类 v1 按各自格式分派，官方资源摘要不能用作内置规则摘要。

标准 ECU 可明确预览并初始化实际 `epic4-single-application-v1` 槽，只创建尚不存在的 live 源码和对应 manifest 成员。后续生成从当前用户字节形成 immutable application snapshot，不写回 live 树；拥有权清单、snapshot 与严格 seal 同时核验。修改生成文件、未知 owner／版本、过期输入或目的地冲突拒绝安装，保留旧有效输出。

准备、预览和重导入只渲染源码并核对身份，不启动编译器。需要本机编译证据时显式运行 `--preflight`，报告为 `not_run|passed|failed`；不适用的本机/目标组合为 `not_run`，不升级为通过。实际预检和构建要求 CPython 3.12.9 与绝对 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON`；Windows 使用固定 MSYS2 GCC 16.1.0 Rev5，Linux 使用固定 Ubuntu GCC 13.3.0/binutils，版本、目标和二进制摘要须符合包内工具链。失败报告实际来源和 owned 日志，不安装旧目标目录。

工程包含原始输入、生成配置、RTE/应用、实际 BSW/OS、固定 FreeRTOS 来源、所选目标补丁、来源映射、许可和 stdlib-only 工具。搬移后运行 `<CPython3.12.9> tools/ecu-tool.py build --project <封存源码目录> --output <工程之外的新空目录> --mode probe`，再运行 `ecu_probe.exe`（Windows）或 `ecu_probe`（Linux）。内核补丁只作用于私有构建副本。`--control-source <工程外的消费者.c>` 只适用于 `probe|test`，替换入口而不替换 BSW/应用；`test` 额外启用有用的私有探针。

同一工程使用 `tools/ecu-tool.py build --project <封存源码目录> --output <工程之外的新空目录> --mode host-batch` 构建生产 `ecu_host_batch.exe`（Windows）或 `ecu_host_batch`（Linux）。

入口逐行读取 `BEGIN <epoch>`、零到 256 行 `RX <CAN ID> <DLC> <hex>` 和 `COMMIT`，例如 `BEGIN 10`、`RX 0x320 4 78563412`、`COMMIT`。epoch 为非递减毫秒整数，单批最多跨 1000 ms；载荷须包含 DLC 对应的准确字节数。BEGIN/RX 只暂存，COMMIT 执行完整批次。目标 epoch 前每个 tick 逐一完成，目标输入先于该 epoch 的周期处理；同一 epoch 不重复执行周期。

`OUT` 携带输出的 epoch、全局 sequence、ticket/PDU 及 CAN 数据，实际 write/flush 成功后才确认。`COMMIT_OK` 表示批输入、tick、输出与确认完成，且进入等待点；`COMMIT_ERROR` 保留已执行前缀及 BSW 拒绝，`REJECT` 表示接纳失败。

每个 COMMIT 的宿主 watchdog 固定为 5000 ms。写失败、阻塞超时或 256 项输出队列溢出会关闭 ECU，已执行前缀不会回滚。生命周期诊断保留有界前缀，`trace_dropped` 记录省略数量；诊断容量与汽车输出容量分别限制。生产 `host-batch` 拒绝私有 test flags 和 `--control-source`，不会自动切换为测试模式。

入口交付 Windows/Linux 主机源码，两个本机生产目标的有界协议已分别实测。历史 4.22 的完整 SC1 结论仍限定原 Windows 目标与配置；编译或本轮有界检查不能扩大到任意用户工程。官方 XSD/MOD/PDF、编译器及许可受限规范不随包分发。

参考应用的 S/R Read 在未接收时返回初值 0／`RTE_E_NEVER_RECEIVED`，有效接收时返回实际值／`E_OK`，过期时保留最后接收值并返回 `RTE_E_MAX_AGE_EXCEEDED`。应用对非成功读取使用配置初值；只有 `Rte_Write` 成功后才提交值与逻辑 epoch，失败时记录标准状态并保留旧提交。

DID 0x1234 在默认／扩展会话中，通过同一 Task 的同步服务器读取提交值，并编码为四字节大端数据。epoch 30 的新输入先于 deadline 处理；同一 epoch 的批次不重复执行应用周期。

应用集成代码可在 owner 上调用 `Ecu_ApplicationInspect`，读取提交值、epoch 及最近读写状态；原生线程调用和空输出被拒绝，输出存储保持不变。独立消费者的 `--mode test` 阶段 9／10 分别位于周期调用前后，使用 CAN controller 状态验证写拒绝与恢复；有界观测由 owner 发布，并由原生线程输出。正式后台入口为 `cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_application_sr_cs_loop -- --exact`。

新目标只开放诊断服务 `0x10`、`0x3E`、`0x22`，单次读取最多 2 个 DID；应用 DID 为 `0x1234`，`F186` 返回实际会话。FC WAIT 按所选 WFTmax=0 终止交换。N_Bs／N_Cr 超时终止对应连接，后续合法请求可恢复。

HostBatch 错误回执的 `transport_status`、`transport_epoch` 和 `transport_count` 分别记录首个实际失败、其逻辑 epoch 及失败数量。已执行 tick／输入不会回滚，即使批目标晚于失败 epoch。失败记录由 owner 有界发布，原生桥接复制后消耗全局 sequence；实际输出失败、输出队列溢出及宿主 watchdog 会关闭 ECU。

独立协议及旧目标回归入口为 `cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_behavior_and_legacy_regression -- --exact`；旧 host-v1 按其独立配置保留原服务。

在同一 Windows 主机执行整套集成测试时，先设置 PowerShell 的 `$env:RUST_TEST_THREADS='2'`，再执行验证命令；单独的 `--exact` 协议测试只运行一个实例。多个 FreeRTOS 宿主实例会竞争 CPU，大量并发可能触发固定 5000 ms 的 COMMIT watchdog。限制测试并发用于提供可复验的运行环境，watchdog、单批 1000 ms 逻辑跨度与验收断言保持原值；故障关闭记录仍须保留。

RTE 周期组可显式引用 `RteUsedOsSchTblExpiryPointRef`／`RteBswUsedOsSchTblExpiryPointRef`。所选参考 ECU 使用一个重复 ExpiryPoint、`NONE` 同步策略和同一 owner／软件 SystemCounter；表 duration 等于 TimingEvent 周期，启动值加初始 offset 等于首个周期。周期编辑同时更新相关事件、Com 周期、表 duration 和启动值。未知或未绑定的表、混用 Alarm 与 ExpiryPoint、错误 Task/Event 及不匹配周期会在生成前被拒绝。

独立计时验收入口为 `cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_sc1_timing_capacity -- --exact`。它覆盖八个独立软件 Counter、八表配置、双表封存轨迹，以及单次／重复／绝对／链接／停止／错误前态和生成工程消费者。

`Os_CounterConfig.software=0` 的宿主定时器 Counter 由受控内核 tick ISR 推进，不能通过标准 `IncrementCounter` 写入。读取及 elapsed 值按该 Counter 的模数调整，内核 32 位 tick 回绕不改变已推进的 Counter 值。该结果限定 Win64 主机；ARTI、工件及完整交接由对应 BMad story 记录。

## 新目标交接与工作台状态

标准输入的生成与运行工具窗口使用同一新目标交付流程。选择工程之外的 ECU 输出目录，预览实际文件及拥有权，再确认生成；默认 R5 包是 `autosar-workbench-handoff-v2`，显式兼容路径保留原 `autosar-ecu-handoff-v1`。构建另用新空目录与声明工具链；行为检查独立重新构建生产 HostBatch，检查 CAN/DID、真实 N_Cr 超时恢复和非法批次，不能从生成结果推定运行通过。

交接包可整体搬移；v2 重导入显式选择新的空工作区目录，核对本机真实内置规则三字段、必需接纳扩展、输入／应用 snapshot／拥有权与重渲染闭包。v1 仍核对原外部 XSD/MOD 身份与完整生成源码，旧 host-v1 的读取和离线运行入口保留。不能用包内 JSON 自授权可信规则或计划；SHA-256 是完整性检查，不是发布者签名认证。

保存、分域校验、显式预检、生成、构建与本次主机行为各自报告；输入／live 应用／目标或依赖身份变化使对应下游结果失效。离线接收者执行 `<CPython3.12.9> tools/ecu-tool.py verify --project <封存源码目录> --build-directory <工程外的新空目录>`，只需声明的 CPython/GCC/binutils/Git，不需要 checkout、uv、Rust 或 Node。v2 重导入／再生成需要兼容的同版工作台及精确所需扩展，不需要官方档案；v1 才保留合法匹配 XSD/MOD 要求。包内许可与既有有限主机能力声明不变，不增加实机、完整 SC1 或认证声明。

最终独立交接入口为 `cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_handoff -- --exact`。它重新建立临时输入、搬移包、重导入、再生成、编译，并核对完整 CAN／DID／拒绝／恢复输出。完整主机等级还需执行既有 OS／ARTI 正式行为测试。BMad 4.22 记录非实现者的独立复验结果；一次包内行为检查不代表当前工程已经完成完整 SC1 复验。

## 开发工具与规划入口

仓库内的 `.agents/skills/` 和 `_bmad/` 可直接供 Codex 使用。重新安装或更新时固定 `bmad-method@6.12.0`、BMM 和 `codex`，先核对安装器差异，保留团队定制。`_bmad/config.user.toml` 是被 Git 忽略的个人安装答案；团队共用语言和配置放在 `_bmad/custom/config.toml`。从仓库打开 Codex 会话可调用 `bmad-help` 查看当前阶段，也可委托一个 story、epic 或规划目标。

| 工件 | 用途 |
| --- | --- |
| [产品简述](../../_bmad-output/planning-artifacts/product-brief.md) | 产品方向与候选路线 |
| [PRD](../../_bmad-output/planning-artifacts/prd.md) | 当前需求与完成契约 |
| [架构](../../_bmad-output/planning-artifacts/architecture.md) | 跨层设计与集成约束 |
| [epics](../../_bmad-output/planning-artifacts/epics.md) | 任务分解与依赖 |
| [Epic 7 实施规格](../../_bmad-output/implementation-artifacts/spec-epic-7-configurator.md) | 当前开发结果与未验发行范围 |

任务状态由 `_bmad-output/implementation-artifacts/sprint-status.yaml` 记录，验证结果进入对应 story/spec。生成工件的格式检查以开发规格记录的起始提交为基准；主机能力结论以相应规格中的原生运行记录为准。学习层目前仅在产品简述中列为待规划方向。

历史 Epic 4 的 22 条 story 已完成，固定 `epic4-win64-sr-cs-v1` 目标的适用主机 SC1 行为、跨 story 集成与非实现者交接已通过验证／复核，记录在 BMad 4.22 及 sprint 中。结论限定 CP/FO R24-11、Windows x64、GCC 16.1.0、固定 FreeRTOS 及声明配置，不扩展到任意用户工程、MCU、硬实时、ASIL、完整 MISRA 或官方符合性认证。

## 工程与验证参考

以下开发与验证命令均从仓库根目录运行。

### 当前支持

| 能力 | 当前范围 | 主要限制 |
| --- | --- | --- |
| 多文件 ARXML | 跨文件引用、差异预览、安全保存与重新导入 | 外部修改使旧预览失效；未支持的有效内容保留 |
| 定义驱动编辑 | 对象树、字段与引用检查、批量及实例结构编辑 | 未知条件、变体与表达式按覆盖范围保持只读或限制操作 |
| C99 源码交付 | 配置、BSW、OS、RTE／应用接口与离线构建工具 | 只生成声明支持的目标；源码与构建目录分离 |
| CAN／DoCAN 与诊断 | 11 位 Classical CAN、受限诊断服务与可选故障记忆 | 信号类型、容量、服务与平台范围见下方细节 |
| 主机行为验证 | 受控 Windows／Linux 目标、信号与诊断独立消费者 | 主机验证不代表真实硬件、硬实时或标准认证 |

当前生成目标覆盖有限的 CAN、DoCAN、诊断与标准 ECU 配置，尚未覆盖全部 AUTOSAR 运行模块，也未完成完整官方 XSD 符合性、真实硬件或功能安全认证。macOS 原生构建、bundle 与 IPC 尚未验证，不提供本机虚拟 ECU。

<details>
<summary>ARXML 编辑、校验与保存约束</summary>

- 一个配置项目可包含多份 ARXML；同一 `AR-PACKAGE` 的内容可以分布在不同文件。未支持的有效内容作为保留项保留，未修改的文件不重写。校验、保存和生成都会复核所选的全部来源文件；任一文件被外部修改时拒绝继续使用旧跨文件配置，并提示重新导入。保存时暂存新内容，在替换前再次复核待写文件；无法安全回滚时保留原备份。
- 编辑配置后，“查看并保存 ARXML”会先列出每份来源文件将修改或保持不变，并显示修改文件的前后差异及完整原文。预览不写磁盘；用户确认后才保存。若预览后配置或来源文件变化，旧预览不能用于保存。
- 同一工程使用文件／对象树、文档标签、属性与引用检查器及问题／生成／构建／运行／日志工具窗口。参数保持 `kind/lexeme`，未落盘默认值与 explicit 值分开；未知条件、变体、表达式和 instance-reference 保持只读，并按受影响消费者限制相应操作。批次与实例结构变化先预览整批，再以当前输入／定义身份原子应用。
- 工程树优先显示对象名称，以图标区分包、模块、容器和数据类型；悬停可查看完整名称、类型与路径。软件“帮助”菜单内置操作步骤、工具依赖和支持范围，不要求用户取得源码仓库；生成工程附带的构建说明仍属于交付内容。
- `workbench-project.json` v1 只记录安全相对成员、应用输入及工程明确接纳的扩展身份；原 ARXML 是配置权威。`can-empty-v1`、`can-signals-v1`、`standard-ecu-v1` 是产品内置的工程模板，创建与另存为先预览，只写新空目录。第三方定义通过 `catalog.json` 显式接纳；不可变本机缓存不等于工程已经选择该定义。
- 分别报告 `source-safety`、`schema`、`definition`、`target-generation`。原生 `schema` 只覆盖产品声明的结构、顺序、基数及类型，不等于完整官方 XSD；`unsupported`、`not_run` 不记为 `passed`。目标不支持不自动阻止安全浏览或修复；实际原生结构错误仍阻断保存。
- `files.sha256` 保存生成文件与 `files.list` 的逐项内容摘要，供再次生成前核验；该记录不等于对第三方修改的签名认证。

桌面标题与应用菜单合并为同一条浅／深主题栏。顶部空白与标题区域可拖动，双击切换最大化；右侧提供最小化、最大化／还原和关闭，关闭仍先处理未应用草稿及未保存修改。工具窗口、工程树和检查器使用方向图标收起，悬停提示说明操作；底部工具标签及侧栏可重新展开。

工程树右边缘、检查器左边缘和底部工具窗口上边缘提供拖动分隔条，分别调整左右侧栏宽度及底栏高度；其他方向随工作区伸缩。聚焦分隔条后用方向键微调，Shift＋方向键调整更大步幅，Home／End 调到当前允许边界；双击或 Enter 恢复默认。折叠、切换工具标签及缩小后再放大窗口保留本次运行的首选尺寸，重启后恢复默认布局；尺寸边界会为编辑区保留空间。

</details>

<details>
<summary>CAN、诊断与故障记忆范围</summary>

- 标准 11 位 Classical CAN，DLC 1 至 8；每 ECU 最多 32 帧、64 信号。每帧可映射多个不重叠的 1 至 32 位无符号 LSB0 小端信号。导入时拒绝与 Com 位段或位序冲突的 I-PDU 映射、重复的 CanIf PDU 映射，以及与 CanIf 不一致的关联 CAN 网络帧 ID 或布局；未解析的配置变体会阻止生成。Tx 帧按虚拟时钟周期发送；Rx 帧按最后有效接收时间判定超时。
- 生成工程包含独立的 Com、PduR、LSduR、CanIf、CanTp、Dcm、可选 Dem/NvM、虚拟 Can、Os 周期调度、Rte 接口及 ECU 配置；目录内的 `README.md` 和 stdlib-only `tools/ecu-tool.py` 给出锁定工具链的离线构建及按配置启动方法，`profile.txt`、`target.json` 和 `files.list` 描述生成结果，`Dcm_Externals.h` 随工程交付诊断回调声明。未配置诊断时目标仍是纯信号 ECU。legacy 目标产出 `ecu_host.exe`（Windows）或 `ecu_host`（Linux）；源码、ARXML 和构建目录必须分开。Windows BCrypt 安全档案不适用于 Linux，源码准备时明确拒绝，不生成缺少安全后端的工程。
- 可选诊断配置：一对不与信号帧冲突的 11 位物理 CAN ID；S3 至少 5000 ms，N_As/N_Bs/N_Cr 为正毫秒；一个扩展会话专属 DID，按配置顺序绑定 1 至 8 个实时 32-bit Tx Com 信号。支持 0x10 默认/扩展会话、0x3E TesterPresent（含抑制正响应）、0x22 读取 DID；同一 0x22 请求可列出多个 DID，按请求顺序返回当前可读的值。配置 DID 在默认会话不可读；保留 DID `0xF186` 在默认/扩展会话返回当前活动会话，S3 回退后返回默认会话，且不可配置为普通 DID。仅请求不可读 DID 时返回 NRC 0x31；请求格式错误返回 NRC 0x13，响应超出 256 字节上限返回 NRC 0x14。可单独启用 0x2E，在扩展会话将完整的大端数据记录写入同一 DID 所绑定的 Tx Com 信号；0x22 与周期 CAN 立即反映新值，ECU 进程重启恢复配置初值。默认会话或 DID 不匹配返回 NRC 0x31，记录长度不符返回 NRC 0x13；未启用时 0x2E 返回 NRC 0x11。该写入仅为易失的主机虚拟应用状态，不写入 NvM/Flash。CanTp 支持单帧、多帧、流控、序号校验、块大小/STmin、N_As/N_Bs/N_Cr 超时与错误后恢复，N-SDU 上限 256 字节。配置可修改或移除；重新导入后仍能识别，超出该确定性子集的诊断配置会阻止生成。
- 可选 0x31/0x01 StartRoutine：先启用上述 0x2E，再设置单个 16-bit RID；仅在扩展会话中将该 DID 绑定的 Tx Com 信号恢复为配置初值。未配置时返回 NRC 0x11，默认会话或 RID 不匹配返回 NRC 0x31，StopRoutine/RequestRoutineResults 返回 NRC 0x12，请求长度错误返回 NRC 0x13；不支持选项或状态记录。该例程只修改易失状态，不执行安全解锁或持久化操作。
- 可选故障记忆：诊断连接下配置一个 UDS DTC（`0x000100–0xFFFFFE`），绑定一条已有信号且超时为正的 Rx 帧。首次有效接收后超时才报告故障；Dem 状态跨 ECU 进程重启保存在主机文件。0x19/0x01 按状态掩码读取匹配 DTC 数量（当前单 DTC 配置因此返回 0 或 1）；0x19/0x02 按状态掩码在默认或扩展会话读取该 DTC；0x19/0x0A 无需状态掩码，列出配置的 DTC 及当前状态，包括状态为零的监测项；默认/扩展会话均可读，启用安全档案也无需解锁。0x0A 多带参数返回 NRC 0x13，未配置 DTC 返回 NRC 0x11，其他 0x19 子功能（含 0x8A 抑制位）不支持。0x14 仅扩展会话、仅 `0xFFFFFF` 全部清除。NvM 使用两个固定 CRC32 保护槽位、配置指纹并在状态更改确认前同步落盘；已有文件的任一槽位损坏均拒绝启动（`E NVM`），不存在的文件初始化为空状态。每次主机 ECU 进程启动代表新的操作周期，不等价于完整车辆生命周期模型。
- 配置该单 DTC 时，扩展会话还支持 `0x85/0x02` 暂停 DTC 设置、`0x85/0x01` 恢复；暂停期间监测 Rx 帧仍更新信号有效性，但不修改或持久化故障状态，已有 DTC 仍可读取及清除。切回默认会话（含 S3 超时）或 ECU 进程重启会自动恢复记录；不支持选项记录、抑制正响应位或其他子功能。未配置 DTC 时返回 NRC `0x11`，默认会话拒绝此服务。
- 可选主机安全访问：单级 `0x27/0x01` 请求 16 字节随机 seed、`0x27/0x02` 提交 16 字节 key；扩展会话内解锁。启用后，`0x2E`、`0x31/0x01`、`0x14`、`0x85` 在已满足原有会话与参数条件时，未解锁返回 NRC `0x33`。三次错误 key 后延迟 5 秒；失败计数跨主机进程重启保存在独立状态文件，状态损坏则拒绝启动。密钥是运行时提供的独立 32 字节文件，不进入 ARXML 或生成工程；该文件与状态文件不等于硬件安全存储，也不提供量产级认证保证。详见 [`runtime/README.md`](../../runtime/README.md)。
- 主机信号闭环检查两个 ECU 的独立信号值与位向量、CAN ID 优先顺序、丢帧后的接收超时、BUS_OFF/STARTED 恢复以及错误 DLC 拒绝。独立诊断测试器对生成的 ECU 注入物理 CAN 报文，检查会话、`0xF186` 活动会话 DID、实时 DID 多帧载荷、流控、N_Bs/N_Cr 超时、错误序号、后续请求恢复与 S3 回退；启用写入时还检查 0x2E 多帧请求、0x22/CAN 信号更新、会话限制与重启后恢复初值；配置例程时检查 0x31/0x01 恢复后 0x22 和 Com 信号的初值以及 RID、子功能、长度和 S3 限制；配置 DTC 时另用隔离存储验证 Rx 超时后 0x19/0x01 的匹配/不匹配/清零数量及 0x19/0x02 单 DTC 报告、0x19/0x0A 在无故障/暂停记录/超时/重启/清除后的支持列表和错误请求拒绝、0x85 禁用/恢复记录、跨进程保持、会话权限清除与损坏拒绝。两种运行结果分别呈现。虚拟总线不模拟电气层或位级仲裁。

</details>

<details>
<summary>ECUC 映射与第三方集成边界</summary>

新建工程的 EcuC PDU 配置按 R24-11 MOD 建立一个虚拟核心与全局 `EcucPduCollection/Pdu`；配置 Com 帧时还提供必需的 `ComGeneral`。`ComPduIdRef` 与 CanIf Tx/Rx PDU 引用指向独立的全局 Pdu 容器；系统 `I-SIGNAL-I-PDU`、`N-PDU`、`DCM-I-PDU` 保留各自语义，全局 Pdu 到系统 PDU 的关系由版本化工具专属 SDG 记录。固定主机剖面现在还生成虚拟 Mcu 时钟、Can 控制器及 Rx/Tx 硬件对象、CanIf 驱动/HOH/缓冲和必需的 PDU 参数与引用；导入时按同一剖面检查，不符合者只读阻断。这些虚拟时钟、基地址和位时序值不表示真实 MCU 配置，运行时代码也不是第三方 CanIf/Can 标准 ABI；尚无第三方协议栈或硬件互操作证据。诊断的 Dcm/CanTp/N-PDU 路径仍仅对本主机目标验证；可选写入使用 DcmDspDidWrite 与逐数据回调。主机专属的 0x31/0x01 例程把 RID 和固定扩展会话记录在 DID 工具 SDG 中，不输出 DcmDsd 0x31 服务或 DcmDspRoutine/StartRoutine/CommonAuthorization；第三方 Dcm 不能依据该 ARXML 获得此例程。DcmDspData 与 Com 信号、Dem 事件与监测 Rx 帧的绑定也由工具 SDG 记录。可选 0x27 的 ECUC 行仅描述主机目标的固定单级档案；所列主机回调并非第三方 Dcm ABI 的互操作证明。主机 NvM 不建模真实 Ea/Fee/MemIf 物理目标或分区引用，也未建模 ComM/EcuM/BswM/CanSM；未实现其他例程/写入 DID/持久写入、完整诊断子功能、功能寻址或完整 ISO 14229/15765 一致性。无真实硬件或标准符合性证明；不支持 29 位 CAN、签名/大端信号、真实芯片驱动与未解析配置变体。界面“通过”仅对应所运行的主机路径，详见 [`runtime/README.md`](../../runtime/README.md)。

诊断配置中的六项 CanTp N-SDU、N-PDU/FC N-PDU 引用及两项 `DcmDslProtocolRx/TxPduRef` 也按 MOD 指向相应 EcuC 全局 Pdu，`VALUE-REF DEST="ECUC-CONTAINER-VALUE"`；系统 N-PDU/DCM-I-PDU 仅保留系统描述与工具绑定。按 System Template `[constr_3448]`，这些系统 PDU 对应的全局容器和 Com 的 I-SIGNAL-I-PDU 全局容器均不写 `DynamicLength`，系统 N-PDU 也不写 `HAS-DYNAMIC-LENGTH`：N-PDU 长度由 TP 处理，DcmIPdu 的动态长度由系统模板语义决定，不从 EcuC 布尔值推断。含此不适用字段的导入配置只读阻断，不自动改写。

生成的主机工程对每个 DID 数据提供可外部链接的 `Ecu_DcmRead_<index>`，启用 0x2E 时还提供 `Ecu_DcmWrite_<index>`；主机 Dcm 实际通过这些回调读写 Com 信号，这些回调既写入 ARXML，也由运行时代码调用。`Dcm_Externals.h` 声明这些回调，`include/Ecu_DcmCallbackTypes.h` 仅定义本主机剖面所需的 `Std_ReturnType` 等类型，不是完整 AUTOSAR `Std_Types.h` 或生成的 RTE 类型头，也不证明第三方 Dcm 可直接接入。0x31 仍由生成工程的内部 `Ecu_HostRestoreDid` 实现；其 R24-11 ECUC 例程函数签名参数属草案，本产品不声明该例程的标准 ECUC 集成。

既有主机生成目标仍要求当前 R24-11 闭包：Com/CanIf/CanTp/Dcm 的 ECUC PDU 引用指向 EcuC 全局 Pdu，固定主机 CAN 配置包含 Mcu/Can/CanIf 关系，主机专属 0x31 只使用 DID 工具 SDG；单 DTC 还要求 DcmDsd 0x85、`DcmDspControlDTCSetting` 禁用选项记录与真实 `DcmDemClientRef`。缺少或不能解释这些消费者时拒绝相应生成，不自动改源、不扩大运行时 allowlist。默认原生配置按分域规则允许安全浏览及未恶化违规的定义编辑；兼容 v1／开发 oracle 路径仍执行原来的固定官方资源和严格拒绝。

既有信号生成剖面的 ComIPdu/ComSignal 必须直接归属本工程唯一的 `/{项目名}/ComCfg/ComConfig`，模块定义及 ComGeneral 必须正确；重命名模块、把子容器挂到其他模块或改动父级 `DEFINITION-REF` 会阻断该目标生成，按模块及父级定义确定所有者。通用配置编辑另按真实定义与分域违规规则判断，生成剖面的限制仅适用于相应生成目标。

</details>

### 使用顺序

1. 从工程入口选择内置模板新建工程、打开成员 manifest 或一次选择同一 ECU 的全部 ARXML。直接 ARXML 可继续使用，显式另存为工程才持久化成员与扩展接纳选择；不能信任 manifest 的 profile hint 或旧通过状态。
2. 在工程树／对象表选择真实对象，按定义检查与编辑字段、引用或结构；跨对象修改先查看整个批次的实际旧／新值与入站影响，再一次应用。既有 CAN、诊断／DTC 与标准参数编辑仍使用同一 Workspace。未应用草稿不写源；切对象先处理草稿，替换工程另处理 dirty 与保存确认。
3. 保存先查看各文件差异并确认；外部改源、过期预览或未恢复备份拒绝覆盖。源码生成另选择工程之外的输出目录，预览真实文件与拥有权再确认，不启动编译。构建目录继续独立，编译／运行只在对应工具与本机目标可用时执行。
4. 在“虚拟运行”页，对已构建、已配置诊断的当前工程点击“验证诊断连接”，由独立测试器检查本次二进制；配置故障记忆时使用隔离 NvM 文件，启用 Windows 0x27 时另用隔离密钥与安全状态文件。不修改真实 ECU 状态，不需要对端 ECU。信号总线验证另选对端 ECU 的封存源码目录与已构建的实际二进制，运行双 ECU 闭环。

<details>
<summary>源码交接与重导入</summary>

默认源码交接使用 `autosar-workbench-handoff-v2`，精确封存规则三字段身份、必需接纳扩展、输入、应用 snapshot 与拥有权；同版工作台在新空目录重建原始成员，重新校验、重渲染并逐字节核对。包内 JSON 不作为可执行规则；旧官方资源摘要不能转换成内置规则摘要。原 `autosar-host-handoff-v1`／`autosar-ecu-handoff-v1` 继续精确分派，并保留各自的合法 XSD/MOD 与完整性要求；不会自动升级旧包。

标准 ECU 的 live 应用初始化先展示真实 `epic4-single-application-v1` 槽描述符、源码与 manifest 变化，明确确认后只创建不存在的文件。声明的 `applicationInputs` 后续从当前用户字节形成不可改的封存 snapshot；生成器不写回 live 源，不覆盖改过的生成文件或未知 owner／清单版本。应用改动也会使旧生成确认和下游结果失效。

固定旧目标双 ECU 离线参考包仍可用 `package_host_reference` 与合法固定 XSD 显式生成，详见 [`runtime/reference-README.md`](../../runtime/reference-README.md)。生成、预检、构建和实际行为复验分别报告；配置内置定义可读不等于目标可生成。

</details>

<details>
<summary>生成目录、备份与构建安全</summary>

重复生成前，工具核对 `files.list` 的准确文件名、`files.sha256` 中每份生成文件及清单自身的 SHA-256、缺失/额外文件与目录；不匹配即拒绝替换并保留原目录。旧版没有完整性记录的生成目录不能直接覆盖，请选新空目录并保留旧目录。二进制和编译日志只进入工程之外的构建目录；源码包中出现未列二进制、文件或链接时拒绝重建和重导入，不自动清理。

再次生成成功时，原目录不会被删除：它会移到同一父目录下独占的新备份位置；返回结果和“生成与构建”页面显示其完整路径。多次生成会积累多份旧工程，由文件所有者确认无用后自行归档或清理，工具不自动回收。生成失败时，未移动的原目录保持原位；若已经移走，则报错中列出可恢复的旧目录路径。失败的临时生成目录也保留并在报错中给出路径，供所有者检查后自行处理。勿将这些备份目录误当成本次构建工程。

构建要求工程之外的新空目录，拒绝覆盖已有二进制、日志或其他所有者文件。编译先写入构建目录内独占的私有目录，再以不覆盖目标的硬链接安装；文件系统不支持硬链接或并发出现同名二进制时直接报错，不降级为覆盖。失败保留实际输出和受管日志；成功只清理工具自己的临时编译产物。

工作台与离线 `tools/ecu-tool.py build` 都在编译前和安装二进制前核对源码清单、SHA-256 与缺失/额外文件，拒绝链接/reparse point；内核补丁只应用到外部构建目录中的私有副本。生成后源码、目标或清单被改动时，工作台拒绝把该目录标为本次配置的结果，请重新生成到空目录。摘要用于完整性检查，不是发布者签名认证。所有实际命令以 argv、绝对工具路径和单调截止时间在 owned scope 中执行，不使用 PowerShell 拼接或 taskkill 清理。

</details>

<details>
<summary>受控主机验证范围</summary>

“本机预检”独立实际编译并返回绑定源码身份的 `not_run|passed|failed`；非本机目标为 `not_run`，不记为成功。“构建 ECU”使用独立空目录和包内目标工具链，Windows 得到 `ecu_host_batch.exe`，Linux 得到 `ecu_host_batch`。“验证 ECU 主机行为”独立重建并逐字节核对 CAN/DID、至多两个 DID、真实 N_Cr 超时恢复及非法批次。离线入口为 `<CPython3.12.9> tools/ecu-tool.py verify --project <封存源码目录> --build-directory <工程之外的新空目录>`。两平台生产协议已通过原生核心入口实测；Linux 桌面 IPC 与正式 bundle 不由该结果推定。当前工程完整 SC1 复验及实机状态不从这些有界向量推断。

项目切换或程序重启后可重新导入已保存的 ARXML；页面阶段状态与生成目录选择是本次工作会话状态，不能代替磁盘上的源文件或构建产物。

最终独立交接测试入口为 `cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_handoff -- --exact`；全部主机等级与旧目标回归使用完整核心测试。任意用户工程的一次生成、构建或包内 CAN／DID 检查，不代表已完成完整 SC1 或实机验证。

固定 `epic4-win64-sr-cs-v1` 目标已验证其全部适用主机 SC1 行为、集成与独立交接。结论限定 CP/FO R24-11、Windows x64、GCC 16.1.0、固定 FreeRTOS 及声明配置，不涵盖任意用户工程、MCU、硬实时、ASIL、完整 MISRA 或官方符合性认证。

</details>

### 本地开发环境

| 用途 | 工具 |
| --- | --- |
| 核心与桌面后端 | Rust 1.98.1、rustfmt、Clippy；平台原生依赖 |
| 前端 | React／TypeScript、Node 24.19.0、npm 11.17.0 |
| 开发与验证工具 | CPython 3.12.9、uv、Git |
| 可选主机 C99 执行 | 目标锁声明的 GCC、objdump、Git 与 CPython |

工具版本由根目录 `rust-toolchain.toml`（Rust 1.98.1）、`.node-version`（Node 24.19.0）、`.python-version`（CPython 3.12.9）、`pyproject.toml`/`uv.lock`（Python）、`ui/package.json` 的 npm 11.17.0 engine 约束及 `ui/package-lock.json` 的包完整性记录约束。安装 Rust/rustfmt/clippy、Node/npm、uv、Git 和目标所需的 C99 GCC；在新的终端检查安装结果。Cargo 构建产物保留在 `core/target/` 与 `src-tauri/target/`，不改设 `CARGO_TARGET_DIR`。

从新克隆的源码根目录运行以下命令，须预先准备平台原生依赖与合法官方档案。官方档案用于源码开发、测试及兼容路径，不是普通应用配置与源码准备的前置条件：

```sh
uv sync --locked --group quality
npm ci --prefix ui
uv run --locked python -m autosar_tooling doctor --role workbench
npm run tauri --prefix ui -- info
uv run --locked python -m autosar_tooling verify --scope all --base <本轮起始提交>
```

macOS 原生构建、bundle 与 IPC 尚未验证；当前不提供本机虚拟 ECU。

<details>
<summary>Windows 开发依赖</summary>

使用 Rust MSVC、Visual Studio C++ Build Tools 和 WebView2。安装 vcpkg 的 `libxml2[iconv,zlib]:x64-windows-static-md`；`VCPKG_ROOT` 指 vcpkg 根目录，`VCPKGRS_TRIPLET=x64-windows-static-md`，`LIBCLANG_PATH` 指含 `libclang.dll` 的目录。可在用户环境中设置这些变量，重新打开终端和 Agent 宿主后再检查；工程中不应包含个人安装路径。原生执行另需 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 指向目标锁声明的 GCC、objdump、Git 和 CPython 绝对路径；不回退 PATH 工具。

</details>

<details>
<summary>Ubuntu 24.04 开发依赖</summary>

安装 `build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev libxml2-dev libclang-dev clang pkg-config patchelf`，并按锁文件安装 Rust、Node、CPython 与 uv。WSL 构建副本、Cargo target 和 uv 缓存放在 ext4 文件系统；开发 oracle 从 Windows 卷读取官方档案时显式传路径。此前固定 GCC13.3.0 的受控 OS 26 项 suite、生产 ECU 独立协议及 deb／解包 AppImage 隔离桌面路径已有历史实测；新产物须重新验证，不能继承历史结论。

</details>

<details>
<summary>macOS 开发依赖与验证边界</summary>

安装 Xcode Command Line Tools、pkg-config/libxml2 和上述版本管理工具。源码工作台代码路径、隔离 IPC 测试入口与 app/dmg 配置已实现；macOS 原生构建、bundle 和 IPC 未验证，不提供本机虚拟 ECU。

</details>

<details>
<summary>官方参考材料与兼容路径</summary>

本地官方材料须由使用者自行合法放置，不随源码或安装包分发：

- XSD：`docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`（包含 `AUTOSAR_00053.xsd` 与 `xml.xsd`）。
- MOD：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`。开发 oracle 和兼容 v1 路径使用显式 XSD/MOD 并核对固定 R24-11 SHA-256；`AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE` 不替换默认内置规则权威。外观、规则与模块定义、执行工具是独立设置类别；外观／工具原子保存到 Tauri `app_config_dir/settings.json`，工具环境覆盖优先且不写回。`AUTOSAR_CONFIG_DIR` 可指定独立绝对配置目录，原生验收必须显式设置，不能仅修改 `APPDATA`。
- 集成样例：`docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip`。核心测试另读取上述两项合法规范档案。

</details>

<details>
<summary>开发启动与目标预检</summary>

`doctor` 只读报告 `ready`、`missing`、`version_mismatch` 或 `not_applicable`，缺少必需项返回非零并提示安装/配置；不下载规范或更改系统。`npm run tauri --prefix ui -- dev` 使用现有 Tauri/Vite hooks，在独立桌面会话交互调试；自动化不得在当前用户桌面弹窗或抢焦点。开发流程不执行 `create-tauri-app` 或 `tauri init --force` 重建已有工作台。开发构建使用 Vite；发行包使用内嵌界面及可信运行资源，真实安装包路径必须另行原生验收，不能从开发构建成功推定。

源码准备固定 `windows-x64-controlled-v1` 与 `linux-x64-controlled-v1` 输出目标；`AssetInventory` 在构建时核对 BSW、OS、FreeRTOS 原件、补丁、目标锁与交付素材的可信摘要。显式目录加载以编译进工作台的清单核对，不使用目录自报的散列作为可信依据。`prepare_ecu_project`/`prepare_host_project` 返回纯内存源码、`autosar-build-target-v1` 元数据与输入/目标/资源 fingerprint，预检为 `not_run`；不调用 Git/GCC/PowerShell，也不安装源码包。原生预检是显式独立操作；非本机目标可渲染与重导入，但不能在当前宿主执行。生成后的交付闭包包含 `tools/ecu-tool.py` 和全部 stdlib 工具，接收者无需 checkout、uv、Rust 或 Node。

明确检查本机能否执行目标时另运行 `uv run --locked python -m autosar_tooling doctor --role native --target windows-x64-controlled-v1`，Ubuntu24.04 原生环境将 target 改为 `linux-x64-controlled-v1`。此命令才有界启动 Git、固定 GCC、binutils 与 CPython 身份查询并核对合法 XSD/MOD；缺失或身份不符返回非零。本机不能执行某目标不妨碍上述纯源码预览。

</details>

### 代码质量检查

开发验证覆盖单元／集成测试、静态检查、UI 构建和桌面后端编译／Clippy。原生桌面、安装包环境及主要使用链需要单独验收，不能从构建或测试通过推定。当前发行范围仍有未验项。

<details>
<summary>检查命令与执行契约</summary>

Windows GBK 终端可使用 `uv run --locked python -X utf8 -m autosar_tooling verify --scope all --base <起始提交>`，避免打印构建工具的 Unicode 日志时因终端编码中断；无需修改系统编码。

质量工具及独立 ARTI 消费者使用的 `lxml` 由 `uv.lock` 中的 `quality` 组固定；UI Prettier/ESLint 由 npm 锁文件固定。定向检查使用 `uv run --locked python -m autosar_tooling quality --base <本轮起始提交>`，检查 UTF-8、末尾换行、空白、Python 语法与增量 rustfmt/clang-format/Prettier、主机 C99 语法。唯一聚合入口为 `uv run --locked python -m autosar_tooling verify --scope core|ui|desktop|all --base <起始提交>`：`all` 顺序组合 Git diff/check、Python unittest、npm ci、质量、Ruff、UI lint/build、core test/clippy 和 desktop build/clippy。每个命令使用独立的绝对单调 deadline、进程所有权与日志；失败报告具体子阶段、argv、观测到的退出码和日志位置，不重试或吞错。26 项原生 OS suite 只经注册的 Cargo 测试执行一次，不追加第二次 OS CLI。此构建／测试关口明确报告平台适用范围，不验证真实 GUI/IPC 或安装包；这些使用独立 native desktop 和 bundle 关口。

受控外部命令已有统一的 `core::execution` 与 `ecu_tools.process` 入口。开发聚合器将当前锁定的 `sys.executable` 作为 `AUTOSAR_PYTHON` 传给 Cargo；直接运行 Cargo 时，须把该变量设为 CPython 的绝对路径（开发环境 Windows 为 `.venv/Scripts/python.exe`，POSIX 为 `.venv/bin/python`），不得依赖 PATH 猜测解释器。可运行 `uv run --locked python -m unittest autosar_tooling.test_process` 与 `cargo test --locked --manifest-path core/Cargo.toml execution::tests` 检查真实父/子/孙进程的退出、超时、取消及故障清理。Windows 使用先登记后恢复的 Job；POSIX 保证已登记的合作进程组及未逃逸后代在绝对单调期限内关闭。未登记的 `setsid`/daemon 逃逸不视为成功清理，Linux 反例返回 `cleanup_unconfirmed`。产品构建、离线验证与 legacy 主机运行均已使用这一受控执行入口。

命令完成与进程回收分开判断：默认 `require_tree_exit` 用于 ECU、协议及生命周期检查，主进程退出后遗留活后代仍失败；开发聚合器仅对 Cargo test/build/Clippy 显式选择 `close_tree_on_exit`，主命令结束后 owner 有界关闭其私有 Job/已登记 scope，确认无残留后保留原退出码。发生实际回收时输出 `cleanup=confirmed descendants=reclaimed`；非零、超时、取消、逃逸或无法确认清理均不能转成成功。不按进程名加白名单，不影响同级任务或用户已有进程，不要求关闭机器级编译器遥测。内部测试与产品命令仍维持各自严格契约。

生成工件测试使用 PATH 中的 Cppcheck 2.21.0 检查实际 RTE 翻译单元，不包含完整 MISRA 扫描。格式检查应指定 `--base`；局部修改无需整体重排。能力结论以对应目标的实际运行记录为准。

</details>

<details>
<summary>原生桌面与发行验证</summary>

真实 GUI/IPC 使用独立入口 `uv run --locked python -m autosar_tooling desktop --platform windows|linux|macos --binary <本次桌面程序>`，不由上述 build/test PASS 推定。Windows/Linux 运行前构建同版 `core/target/debug/package_host_reference` 和桌面程序；Windows 用独立 Desktop/Job/CDP，Linux 用私有 Xvfb 与固定 `tauri-driver 2.1.0`/原生 WebKit driver。Linux 另安装 `xvfb x11-utils xdotool webkit2gtk-driver`，并运行 `cargo install tauri-driver --version 2.1.0 --locked`。macOS 的 `native-webdriver` feature/capability 仅用于独立登录会话中的源码工作台测试，目前无原生验证；生产构建不包含它。截图、driver 日志与真实 IPC 记录留在入口报告的私有证据目录，Linux/macOS 使用用户缓存目录保留跨会话结果；每次验收单独记录运行范围与结果。

当前 OS 独立消费者入口为 `uv run --locked python -m autosar_tooling os --target windows-x64-controlled-v1 --suite all`；在 Ubuntu24.04 原生环境把 target 改为 `linux-x64-controlled-v1`，不能跨宿主运行。26 项适用 suite 已分别接入 core 集成测试，ARTI 原生消费者随 stack suite 执行一次。固定内核、补丁、原生执行栈及 Windows PE／Linux ELF 差异见 [`runtime/os/README.md`](../../runtime/os/README.md)。生产 ECU 的两个目标、stdlib-only 离线 build/verify、Windows MSI 与 Linux deb/AppImage 的无 checkout 隔离桌面真实 IPC 均已实际运行；这些是受控原生主机能力，不包含实机验证或标准认证。

</details>

### 原生分发与图标

Windows 配置中文 MSI，Linux 配置 deb／AppImage，macOS 配置 app／dmg。发行构建须在各自原生宿主完成；macOS 尚无原生验证，签名、公证与远端发布也未声明。

<details>
<summary>发行构建与运行依赖</summary>

在各自原生宿主、已准备上述开发依赖的源码根目录执行 `npm run tauri --prefix ui -- build`。Tauri 自动合并 `src-tauri/tauri.<platform>.conf.json`：Windows 生成中文 MSI（`zh-CN`，保留中文产品名）；Linux 生成 deb/AppImage，包名使用 `Classic CAN Workbench`；macOS 配置 app/dmg，但尚无原生验证。产物位于 `src-tauri/target/release/bundle/`；尚未声明签名、公证或远端发布。

运行已提取或安装的应用不需要 npm、uv、Rust 或 checkout；Windows 需要 WebView2，Linux 需要 WebKitGTK 4.1、Ayatana AppIndicator 与 libxml2。普通配置、保存、原生检查及源码交付不依赖官方档案或外部编译器；开发 oracle／兼容 v1 重导入才需要合法匹配的 XSD/MOD。原生 ECU 预检、构建和行为验证另需声明的 CPython 3.12.9、GCC、objdump 与 Git。安装包不携带官方档案或编译器。无 FUSE 时可解包 AppImage 后运行 `squashfs-root/AppRun`。

</details>

<details>
<summary>发行包复验与历史结果</summary>

发行包复验使用现有独立入口，附加 `--installed --source-checkout <发行构建时的原始源码路径>`；只能搬离本次 owned 构建副本，不得搬动用户工作树。`--binary` 指 checkout 外的真实解包应用；`--builtin-only` 分支使用无官方资源、无开发工具／编译器的配置环境，独立消费者阶段再提供目标工具链。默认 oracle／v1 分支保留合法参考资源及原拒绝。应用从私有空 cwd/config 与最小 PATH 启动，不使用 Vite；外部自动化 driver 不进入产品环境。各次验收单独记录结果，build/test 不替代原生发行验收。

此前验收采用 Windows MSI 行政解包（`msiexec /a <MSI> /qn TARGETDIR=<私有目录>`）及 Linux deb 解包（`dpkg-deb -x <deb> <私有目录>`）／AppImage `AppRun`。历史结果不自动适用于当前变更；双平台能力以当前产物的实际运行记录为准。系统级安装／卸载、升级、签名、公证、远端发布及 WebView2 首装下载不由行政解包推定。

</details>

### 界面设计基线

正式工作台使用以下视觉与交互规格。冻结原型用于设计评审，项目数据与操作结果在内存中模拟，不代表真实运行结果。

- [视觉系统](../../DESIGN.md)：浅深主题、语义角色色、字体、布局及组件规范；根目录为后续实施的视觉入口，日期工作区为冻结评审快照。
- [交互规格](../../_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md)：当前操作映射、两种输入剖面、草稿／保存／结果生命周期、关键流程及扩展接入规则。
- [交互原型](../../_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/index.html)与[汽车电子图标提案](../../_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/logo.html)：设计评审材料，全部项目数据与操作结果在内存中模拟，刷新重置。

<details>
<summary>冻结原型的静态预览</summary>

从仓库根目录启动独立静态预览：

```powershell
uv run --locked python -m http.server 1421 --bind 127.0.0.1 --directory _bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups
```

访问 `http://127.0.0.1:1421/` 与 `http://127.0.0.1:1421/logo.html`。原型不读取官方档案、不写 ARXML、不生成真实源码、不编译或运行 ECU；它不替代正式 UI／IPC 的实际验收。

</details>
