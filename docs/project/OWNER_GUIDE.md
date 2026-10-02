# 怎样用 BMad 推进 Autosar

在仓库中打开 Codex，直接说明要达成的结果，例如“按 BMad 完成 Epic 2”或“继续当前 story”。Agent 应核对现有规划、状态与实际证据，并按委托范围使用 BMad 的规划、开发和复核技能。你无需逐项分派函数或重复上次聊天；如果需求或验收范围还不够明确，先得到具体的缺口和建议。

首次安装、锁定工具版本和工作台开发命令请看仓库[README 的本地开发环境](../../README.md#本地开发环境)；本说明只描述产品使用与BMad委托，不维护第二份安装清单。

| 你想做什么 | 直接说 | 你会得到什么 |
| --- | --- | --- |
| 看进度 | “使用 `bmad-sprint-planning` 显示当前状态，并告诉我实际能运行什么。” | 只读的在途、待办、风险和推荐动作；能力声明另行核对。 |
| 推进 story | “按 BMad 完成 Story【编号】。” | 该项开发、验证与复核的实际结果，或明确阻断。 |
| 完成 epic | “按 BMad 完成 Epic【编号】，包括跨 story 集成验收。” | 逐项推进和最终的 epic 验收结论；若规划尚未就绪，先说明缺口与建议。 |
| 不指定任务 | “按 BMad 判断当前最值得推进的工作并执行。” | 基于现有规划和证据选择工作，不把状态文件的下一条机械当成产品优先级。 |
| 改目标 | “我希望用户能【操作】并得到【结果】。请用 BMad 判断现有计划需怎样调整。” | 影响范围、推荐方案和确需你决定的取舍。 |
| 看成果 | “使用 `bmad-walkthrough` 带我看这次交付。” | 用途、值得检查的地方及复现方式。 |
| 阶段复盘 | “使用 `bmad-retrospective` 复盘 Epic【编号】。” | 基于 story、提交和运行证据的结论与行动项。 |

不知道下一步时直接调用 `bmad-help`。`bmad-build` 处理一个明确的开发目标或 story；委托整个 epic 时，Agent 应按 BMad 规划逐项推进，并核对组合结果，而不是把 epic 当成单次 Build。关键意图不清时使用适用的 BMad 规划技能；`bmad-code-review` 可作额外审查，`bmad-retrospective` 用于阶段复盘。这些技能各有工作范围，但仓库不再规定每次委托只能推进一个工作单元或必须在下一项前开新会话。

当前长期方向与阶段需求在 `_bmad-output/planning-artifacts/` 的 product brief 和 PRD，近期工作拆在 epics/stories，任务状态在 `_bmad-output/implementation-artifacts/sprint-status.yaml`。学习层目前是 product brief 中的待规划方向，不会仅因旧议题存在就进入开发队列。早期产品地图、决策和研究已归档到 `docs/project/archive/` 与 `docs/research/autosar-platform/`，不再需要你维护第二份产品计划。旧任务卡和反馈保存在 Git 历史，只作历史输入。开发、验证、复核、状态和流程工件完全按安装的 BMad，结果进入对应 story/spec。Story 完成不等于某个 AUTOSAR 模块已完整实现。

Agent 应把规范研究放进具体功能任务，不用一轮轮独立审计替代产品开发。当前做什么须同时核对委托目标、BMad 规划与状态及实际证据，不以本说明中的静态示例为准。

项目目前由你主动唤起 Agent，不会在你离开后后台自动开发或发送通知。Agent 可作日常技术判断；新标准版次、真实芯片、重大兼容性或费用、对外推送与发布，应带着推荐和影响请你决定。

## 汽车工程技能怎么用

仓库提供 12 组、共 24 个汽车工程规格与审阅技能，具体清单、依赖、来源和限制见[技能说明](automotive-skills.md)。你仍然可以直接说“按 BMad 完成 Epic 3”或“为 Epic 4 设计验证方案”，Agent 根据实际需要选技能；无需你维护 Excel 或逐个调用技能。没有规格交付需求时，普通开发继续使用 BMad。

| 你想得到的结果 | 可以直接发送的提示词 |
| --- | --- |
| Epic 验证计划 | “使用 `$verification-plan-builder`，依据当前 Epic 3 和 story 验收条件编制验证计划，再使用 `$verification-plan-checklist-reviewer` 审阅。区分文档检查、实际测试和未验证项，不改变任务状态。” |
| 需求与测试的关联 | “使用 `$traceability-matrix-builder`，把当前明确纳入范围的需求、规范依据、设计和测试结果整理成追踪矩阵，再用 `$traceability-matrix-checklist-reviewer` 找出遗漏。注明统计范围，不推算整个 AUTOSAR 的完成率。” |
| 诊断规格 | “使用 `$uds-services-builder` 整理当前主机诊断的实际服务、DID、会话和拒绝契约，再用 `$uds-services-checklist-reviewer` 审阅。依据源码和测试区分已实现、待核实和未支持，不用模板补齐支持声明。” |
| Classic 配置设计 | “依据 Epic 4 当前架构，使用 `$autosar-swc-builder` 和 `$autosar-rte-mapping-builder` 整理参考应用及映射规格，并用对应 checklist-reviewer 审阅；对照 R24-11 标注版次差异和待核实项。” |

这些技能生成规格工作簿和文档审阅报告；报告中可能包含上游固定模板、建议阈值和待确认判断。它们不能单独证明生成代码正确或满足完整 AUTOSAR/MISRA 要求，Agent 应说明实际证据。Agent 根据任务选择适用技能；`$技能名` 是可选的显式指定方式。Codex 自动发现技能变化，若列表未更新可重启 Codex。

## C 编码技能如何使用

直接提出功能或修复需求即可，例如“修复 CanTp 的长度校验”“完成当前 OS story”或“修改生成的 C 回调”。Agent 定位到 C 源码、头文件或 Rust 中的 C 生成模板后，应在设计和编辑前主动读取 [misra-c2012](../../.agents/skills/misra-c2012/SKILL.md)，无需你每次点名或输入 `$misra-c2012`。它与 BMad 开发流程配合使用，纯 Rust/TypeScript 改动及只读状态查询不会因此触发。

该技能已显式允许自动调用；完整性与符合性仍按技能的实际核查范围及规范依据判断，不能把自动使用技能当作 MISRA 验证通过。

## 标准 ECU 输入的检查与保存

在“导入 ARXML”中选择同一输入集合的多份文件，也可展开“直接填写来源路径”，每行填写一份 ARXML 的完整路径。含 ECU_EXTRACT 的集合自动进入“标准输入”；不完整集合也可打开此页查看缺失关系的诊断。原有主机目标仍使用原来的配置、生成和运行入口。

“标准输入”显示同一核心计划实际识别的来源角色和原字节摘要。当前可编辑所选 S/R 接收/发送 CAN ID，以及应用、Com 发送和对应 Alarm 的共同周期；诊断参数、类型、端口及引用保持只读。应用修改后会重新校验整个计划，冲突时恢复原配置。

点击“预览保存”查看受影响文件及保存前后原文，再确认保存；“重开来源”重新读取并检查磁盘输入。未修改文件保持原字节，外部编辑、失效预览及待恢复备份会阻止保存。输入已校验或已保存，都不表示运行工程已经生成。XSD/MOD 仍须在本机合法提供，不属于交付输入包。

后台原生复验统一使用 `uv run --locked python -m autosar_tooling desktop --platform windows|linux|macos --binary <本次桌面构建路径>`；Windows/Linux 先构建同版 `package_host_reference` 与桌面程序，设置合法规范和声明的原生工具绝对路径。测试程序从私有空配置启动，实际操作缺规范设置页与固定摘要校验。Windows 创建独立 Desktop，通过 `STARTUPINFO.lpDesktop` 指定子进程归属，核对窗口不在输入桌面，不调用 SwitchDesktop；WebView 使用私有数据目录，专属 Job 最终关闭测试树。Linux 使用私有 Xvfb display、`tauri-driver 2.1.0` 与 Ubuntu `WebKitWebDriver`，不连接用户的 DISPLAY/Wayland 会话；另需 `xvfb x11-utils xdotool webkit2gtk-driver`，通过 `cargo install tauri-driver --version 2.1.0 --locked` 安装固定 driver。两种 transport 共用真实 UI/原生 IPC 场景，不替换 Tauri invoke；截图、driver 日志和 `native-ipc.jsonl` 保留在入口报告的私有证据目录，Linux/macOS 使用用户缓存目录以保留跨会话证据。实际验证结论写对应 BMad 工件。

正式发行包使用相同入口追加 `--installed --source-checkout <发行构建时原始源码路径>`，`--binary` 指向 checkout 外实际解包的应用。先搬离仅属本次构建的源码副本，原始路径必须不存在；不得搬动用户工作树。应用不启动 Vite，从私有 cwd/config、复制的合法档案及外部 base CPython 开始，最小 PATH 不包含 Node/npm/uv/Cargo/rustc。Windows driver 与应用均在恢复首线程前登记同一测试 Job；清理回执须显示零受管进程。外部测试器依赖保留在测试环境，不当作安装应用依赖。

Windows MSI、Ubuntu24.04 deb／解包 AppImage 已从不存在的原构建 checkout 完成真实隔离复验；Windows 不切换输入桌面，Linux 用受管 Xvfb 高位 display 和独占文件锁，等待本次 Xvfb 的 `-displayfd` 回执后连接。Linux 只启用抽象本地 X11 传输（关闭 TCP／文件路径 Unix listener），不修改 WSLg 的只读 `/tmp/.X11-unix`。源级 `verify` 和安装路径证据分开记录；没有验证系统级安装／升级或 WebView2 首装下载。

macOS 入口要求独立的非 console GUI 登录会话与 `cargo build --manifest-path src-tauri/Cargo.toml --features native-webdriver` 测试构建；只验收编辑、校验、保存与纯源码预览，同时检查本机预检／构建／运行按钮不可用。专用 capability 和内嵌 loopback WebDriver 只在此 feature/macOS 组合中启用，生产构建不启用。没有 macOS 隔离宿主时该入口标为未原生验证，不得改在用户桌面补测。

“工作台设置”统一配置合法 XSD/MOD 与绝对 CPython/GCC/objdump/Git 路径；环境覆盖仍优先且不写回。配置默认存放于 Tauri `app_config_dir`；显式 `AUTOSAR_CONFIG_DIR` 指定独立绝对目录，后台验收必须设置，单独覆盖 Windows `APPDATA` 不保证隔离 Known Folder。切换目标、修改规范或工具会清除相应预检/交付成功态；取消操作须等原生 owner 关闭受管树，过期结果不会安装到最终目的地。设置失败保留旧配置及当前项目，官方档案不随包交付。

## 独立 ECU 集成工程

核心 `generate_epic4_ecu` 命令消费同一个已验证标准计划：`cargo run --manifest-path core/Cargo.toml --bin generate_epic4_ecu -- --target <windows-x64-controlled-v1|linux-x64-controlled-v1> --xsd-archive <合法XSD绝对路径> --mod-archive <合法MOD绝对路径> --output <工程目录> --input <文件1.arxml> --input <文件2.arxml> ...`。先读取 JSON 预览，再附加 `--write --revision <完整revision>` 安装工程；`--handoff` 包含原输入和重建元数据。每份输入分别传 `--input`，不需要 `--repository`。过期预览、来源变化或用户修改会拒绝安装并保留已有内容。

准备、预览和重导入只渲染源码并核对身份，不启动编译器。需要本机编译证据时显式运行 `--preflight`，报告为 `not_run|passed|failed`；不适用的本机/目标组合为 `not_run`，不升级为通过。实际预检和构建要求 CPython 3.12.9 与绝对 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON`；Windows 使用固定 MSYS2 GCC 16.1.0 Rev5，Linux 使用固定 Ubuntu GCC 13.3.0/binutils，版本、目标和二进制摘要须符合包内工具链。失败报告实际来源和 owned 日志，不安装旧目标目录。

工程包含原始输入、生成配置、RTE/应用、实际 BSW/OS、固定 FreeRTOS 来源、所选目标补丁、来源映射、许可和 stdlib-only 工具。搬移后运行 `<CPython3.12.9> tools/ecu-tool.py build --project <封存源码目录> --output <工程之外的新空目录> --mode probe`，再运行 `ecu_probe.exe`（Windows）或 `ecu_probe`（Linux）。内核补丁只作用于私有构建副本。`--control-source <工程外的消费者.c>` 只适用于 `probe|test`，替换入口而不替换 BSW/应用；`test` 额外启用有用的私有探针。

同一工程使用 `tools/ecu-tool.py build --project <封存源码目录> --output <工程之外的新空目录> --mode host-batch` 构建生产 `ecu_host_batch.exe`（Windows）或 `ecu_host_batch`（Linux）。入口逐行读取 `BEGIN <epoch>`、零到256行 `RX <CAN ID> <DLC> <hex>` 和 `COMMIT`；例如 `BEGIN 10`、`RX 0x320 4 78563412`、`COMMIT`。epoch为非递减毫秒整数，单批最多跨1000ms；载荷恰好包含DLC所需字节。BEGIN/RX只暂存，COMMIT执行完整批；目标epoch前每个tick逐一完成，目标输入先于该epoch周期处理，同epoch不重跑周期。

`OUT`携带真实输出的epoch、全局sequence、ticket/PDU及CAN数据；实际write/flush成功后才确认。`COMMIT_OK`表示批输入、tick、输出与确认完成且进入真实等待点，`COMMIT_ERROR`保留已执行前缀及BSW拒绝，`REJECT`表示接纳失败。每个COMMIT固定5000ms宿主watchdog；写失败、阻塞超时或256项输出队列溢出关闭ECU，不声明前缀回滚。生命周期诊断保留有界前缀，`trace_dropped`说明省略数量；诊断容量不同于汽车输出容量。生产 `host-batch` 拒绝私有 test flags 和 `--control-source`，不会自动切换为测试模式。

入口交付显式 Windows/Linux 主机源码，两个本机生产目标的有界协议已分别实测；历史4.22的完整SC1结论仍限定其原Windows目标和配置，不能从编译或本轮有界检查升级任意用户工程的能力。官方 XSD/MOD/PDF、编译器及许可受限规范不随包分发。

参考应用的S/R Read在未接收时返回初值0／`RTE_E_NEVER_RECEIVED`，有效接收返回实际值／`E_OK`，过期保留最后接收值并返回`RTE_E_MAX_AGE_EXCEEDED`。应用对非成功读取采用配置初值；只有`Rte_Write`成功才提交值和逻辑epoch，失败记录标准状态并保留旧提交。DID 0x1234在默认／扩展会话中，经同Task的同步服务器读取这一提交值并编码为四字节大端；epoch30的新输入先于deadline处理。同一epoch的批次不重复应用周期。

应用集成代码可在owner上调用`Ecu_ApplicationInspect`读取提交值、epoch及最近读写状态；原生线程和空输出拒绝，输出存储保持。独立消费者的`--mode test`阶段9／10分别位于周期调用前后，使用真实CAN controller状态验证写拒绝与恢复；有界观测由owner发布，原生线程输出。正式后台入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_application_sr_cs_loop -- --exact`。

新目标只开放诊断服务`0x10`、`0x3E`、`0x22`，单次读取最多2个DID；应用DID为`0x1234`，`F186`返回实际会话。FC WAIT按所选WFTmax=0终止交换。N_Bs／N_Cr超时终止对应连接并允许后续合法请求恢复；HostBatch错误回执的`transport_status`、`transport_epoch`和`transport_count`分别记录首个实际失败、其逻辑epoch及失败数量。已执行tick／输入不会回滚，即使批目标比失败epoch更晚。失败记录由owner有界发布，原生桥接复制后消耗全局sequence。实际输出失败、输出队列溢出及宿主watchdog仍关闭ECU。独立协议及旧目标回归入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_behavior_and_legacy_regression -- --exact`；旧host-v1按其独立配置保留原服务。

在同一Windows主机执行整套集成测试时，先设置PowerShell的`$env:RUST_TEST_THREADS='2'`，再执行验证命令；单独的`--exact`协议测试只运行一个实例。真实FreeRTOS宿主实例同时运行会竞争CPU，默认大量并发可能触发固定5000ms的COMMIT watchdog。限制测试并发用于提供可复验的运行环境，watchdog、单批1000ms逻辑跨度及所有验收断言保持原值；故障关闭记录仍须保留。

RTE周期组也可显式引用`RteUsedOsSchTblExpiryPointRef`／`RteBswUsedOsSchTblExpiryPointRef`。所选参考ECU使用一个重复ExpiryPoint、`NONE`同步策略和同一owner／软件SystemCounter；表duration等于TimingEvent周期，启动值加初始offset等于首个周期。周期编辑会同时更新相关事件、Com周期、表duration和启动值。未知或未绑定的表、混用Alarm与ExpiryPoint、错误Task/Event及不匹配周期会在生成前拒绝。

独立计时验收入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_sc1_timing_capacity -- --exact`。它运行八个独立软件Counter、八表实际配置、双表封存轨迹、单次／重复／绝对／链接／停止／错误前态和真实生成工程消费者。`Os_CounterConfig.software=0`的宿主定时器Counter由实际受控内核tick ISR推进，不能通过标准`IncrementCounter`写入；读取及elapsed值按该Counter模数调整，内核32位tick回绕不改变已推进的Counter值。该结果限定Win64主机；ARTI、工件及完整交接由其BMad故事记录。

## 新目标交接与工作台状态

标准输入工程的“生成与构建”和“虚拟运行”页使用同一新目标交付面板。填写独立 ECU 输出目录，预览实际文件后确认生成；默认包含 `autosar-ecu-handoff-v1` 元数据。再填写工程之外的新空构建目录，执行“构建 ECU”和“验证 ECU 主机行为”。行为检查会在临时目录重新构建生产 HostBatch 入口，执行 CAN/DID、真实 N_Cr 超时恢复和非法批次拒绝；日志留在界面，过程不会写入仓库报告。

交接包可整体搬移，在面板填写“重导入 ECU 目录”重新打开，再生成到另一目录。原字节输入、固定运行时、许可、外部 XSD/MOD 身份及每份生成源码会重新核对，不能用包内 JSON 直接恢复一个可信计划。旧 host-v1 保持原读取和离线运行入口。SHA-256 用于完整性检查，不提供发布者签名认证。

保存、校验、显式预检、生成、构建和本次主机行为分别呈现实际结果；输入修改或重新打开使下游结果失效。完整SC1当前工程复验与实机状态不从有界向量推断。离线接收者执行 `<CPython3.12.9> tools/ecu-tool.py verify --project <封存源码目录> --build-directory <工程外的新空目录>`，只需要声明的 CPython/GCC/binutils/Git，不需要 checkout、uv、Rust 或 Node；仅重导入和再生成需要同版工作台以及合法、匹配的 XSD/MOD。包内保留所选 FreeRTOS 来源、补丁与 MIT 许可；产品代码只用于所有者授权的内部用途，不增加公开发布许可。

最后的独立交接入口为`cargo test --manifest-path core/Cargo.toml --test end_to_end epic4_independent_handoff -- --exact`。该入口重新建立临时输入、搬移包、重导入、再生成、编译并核对完整CAN／DID／拒绝／恢复输出；完整主机等级还需执行既有OS／ARTI正式行为测试。BMad 4.22记录非实现者的实际复验结果，工作台不会把一次包内行为检查升级为当前工程完整SC1复验。
