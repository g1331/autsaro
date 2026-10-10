---
title: '在现有工作台安全编辑多组件组合与调度'
type: 'feature'
created: '2026-10-09'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '3da36070473aa6bf7451e3ec407c92010dc544f6'
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/CONTRIBUTING.md'
  - '{project-root}/DESIGN.md'
  - '{project-root}/_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-8-context.md'
  - '{project-root}/_bmad-output/specs/spec-multi-component-scheduling/application-contract.md'
  - '{project-root}/_bmad-output/specs/spec-multi-component-scheduling/acceptance.md'
  - '{project-root}/_bmad-output/specs/spec-multi-component-scheduling/compliance-references.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**问题：** Story 8.3 的全组件源码保护已完成，但标准工程仍混入旧 host CAN 诊断，SWC／组合／事件和实例引用缺少安全编辑描述。

**方案：** 完成 Story 8.4、候选 R6 的现有工程树→检查器／引用选择→批次→保存重开→初始化／再生成流程，并经真实原生 IPC 验收。

## Boundaries & Constraints

**始终：** 固定 R24-11 与已选首批配置；ARXML 原字节、共享模型、可信计划为唯一依据。补齐类型／实例、端口／接口／类型映射、assembly 连接、runnable／event／OS 映射的有界安全编辑。保留源定位、事务预览、外部变化拒绝和未知内容保护；生成不支持不得成为全局保存门。已初始化成员身份改变须明确拒绝可能遗失源码的配置操作；不自动迁移用户 C。正式分支 `feat/multi-component-scheduling`，实施期间只做本地改动与验证。

**禁止：** 新增另一套组合／调度界面、清空诊断、前端按 host 错误码过滤、移除全部 IREF 安全检查、扩大 R11、混入 Epic 7 发行收尾。修改 C／生成模板前读取 MISRA 技能，实际受影响 profile 运行 c-check。

## I/O & Edge-Case Matrix

| 场景 | Given／When | Then／错误处理 |
| --- | --- | --- |
| 合法编辑 | 真实多组件工程，修改字段、引用及连接／调度批次并保存重开 | 正确 profile、无伪 host 诊断；计划和实际生成成立，未改源字节保持 |
| 非法关系 | 非法连接、类型不匹配或调度位置／任务关系 | 真实对象诊断可定位；生成拒绝，安全配置仍可保存，无半批次 |
| 实例引用 | 成对修改 connector 的实例与其类型端口 | 合法组合接纳；不配对、悬空、未知 IREF 拒绝，原内容保持 |
| 安全边界 | 外部变化、stale 预览、未知／变体对象 | 拒绝危险应用，指向真实源；revision、输入与用户 C 保持 |
| 全槽源码 | 正常菜单初始化所有组件，修改 C 后保存／重开／再生成 | 全 slots 及归属正确，逐字节保护；已有文件与成员身份破坏拒绝 |
| 交互 | 真实长名称、窄窗、键盘、主题、中英反馈 | 身份可辨、关键操作可达，预览取消保持 draft 与焦点 |
| 回归 | 旧 host CAN、标准单组件及安装模式 | 正常诊断／编辑／源码行为保留；安装隔离断言不减弱 |

</frozen-after-approval>

## Code Map

- `core/src/arxml/host_profile.rs::Workspace::refresh/view`：旧 parser issues 污染标准 workspace；从实际 profile 统一来源，保留真实 schema／definition 错误。
- `core/src/arxml/projection.rs`：ECU_EXTRACT 被标作旧 profile；`unsafe_object` 排除所有 IREF，writable 依赖 descriptor。按首批对象及实例上下文有界支持，复用字段／引用投影。
- `core/src/definitions/{standard,validation}.rs`、`core/src/arxml/changes.rs`：复用 catalogue、ChangeSet 和 transition 检查；标准字段／child containers 按真实 XML 语法处理，不套 ECUC 参数形状。
- `ui/src` 的 `useWorkbench`、`ObjectEditor`、`StructureEditor`、`ReferenceView`、`ToolWindows`：沿既有 DTO、引用候选、问题焦点、批次和保存入口接线；已有全 slots 初始化 dialog 复用。
- `core/tests/multi_component_contracts.rs`：既有真实 source fixtures、共享模型与生成成功／拒绝测试入口。
- `tools/python/src/autosar_tooling/{cli.py,acceptance/native_profile.py,acceptance/native_builtin.py}`：既有 `desktop --builtin-only` 错误限定 installed；允许 development，准确记录 installed=false。保留应用无工具／档案环境，外层仍用绝对工具路径。
- `tests/desktop/desktop_builtin_{inputs,scenario}.mjs`：真实 Tauri／WebDriver builtin 场景。development 验证 `http://127.0.0.1:1420`；仅 installed 要求 packaged URL、无 checkout、普通用户、网络 namespace 及 receipts。开发模式不声称安装验收通过。

## Tasks & Acceptance

**执行：**
- [x] `core/src/arxml/{host_profile,projection,changes}.rs`、`core/src/definitions/{standard,validation}.rs` — 完成共享 profile、诊断及有界标准对象／引用编辑；核对所改语法的固定官方契约。
- [x] `ui/src` — 补齐必要共享 DTO／检查器／引用／源码入口，保持 DESIGN 与已有组件。
- [x] `core/tests/multi_component_contracts.rs`、现有 UI tests — 覆盖矩阵的真实修改、拒绝、保存重开、再生成与源码字节保护。
- [x] `tools/python/src/autosar_tooling`、`tests/python/test_development.py` — 既有 builtin-only runner 支持开发模式且准确区分安装证据。
- [x] `tests/desktop/desktop_builtin_{inputs,scenario}.mjs`、`docs/development/testing.md` — 用正常命令实际走通多组件编辑／全槽初始化／生成、长名称及交互矩阵，保留旧场景；只补必要命令说明。

**验收：**
- Given 8.3 已通过，when 正常共享编辑与真实 native IPC 执行矩阵，then CAP-1／4、UX8-1–3、AC-5–9 成立，未执行层级不写通过。
- Given 合法标准／非法标准／旧 host 工程，when 打开并编辑，then profile 与诊断一致；真实错误保留，目标不支持与全局保存安全分开。

## Implementation Notes

- 大工程验收的现有完整 IPC JSONL 记录逐条写入，不能在内存中永久保留全部历史完整回复；保持原文件格式、全部请求／回复／错误内容、16,000 个对象规模与既有快照／计数／响应／取消断言。此修改仅解决已确认的测试驱动证据重复持有，不削减产品或验收范围。

- 标准 source workspace 以共享 graph selector 区分多组件与旧 host profile；旧 parser 的 CAN 视图／issues 不再成为标准 workspace 的诊断来源。原 schema／definition 诊断保留，目标生成关系检查进入 `target-generation` scope，安全编辑与保存仍使用事务／schema／definition gate。
- 标准编辑词汇实际位于 `core/src/definitions/standard.rs`，通过已有 catalogue、字段投影、引用候选、ChangeSet 与检查器接入。标准 child／leaf 使用真实 XML tag 和顺序；支持的 IREF 语法逐项限定，重复 element 身份及未知 IREF 仍为只读；跨源 AR-PACKAGE 开放集合允许通过显式 source／object 身份创建子项，包改名／删除仍拒绝路径歧义。assembly 实例／其类型端口按最终批次成对检查，允许完全未改的既有生成关系错误随无关安全编辑保留；新增或改动为非法端点拒绝。已有成员源码时拒绝 SWC／实例／runnable 源码身份变化。
- 缺失标准 leaf 的插入顺序直接复用既有有界 grammar 顺序，包含 identification header、CATEGORY、SW-DATA-DEF-PROPS 等位置，避免可编辑字段无法修复缺失值。成员加载先注入真实 manifest／catalog 再构建唯一 snapshot；profile 判型只读取同一内存 source snapshot，执行／修改／保存和最终打开的磁盘外部变化校验保留。
- builtin-only 开发验收沿原有 desktop runner 进入真实 Tauri／WebDriver，receipt 明确 `installed=false`；应用仍使用无开发工具／官方档案的私有环境。安装模式的 packaged URL、不可用 checkout、普通用户和离线 namespace 要求保留。开发入口说明放在已有 testing 文档。
- 新增多组件原生场景沿工程树、检查器、引用选择及批次执行：长事件名称与周期、键盘取消保留 draft、窄窗保存重开、connector 成对重指向、菜单全槽初始化、用户 C 字节保护及 source delivery。原有 host／标准单组件及布局主题场景继续执行。

## Spec Change Log

- 2026-10-10：按用户更新的 AGENTS.md，后续实现、整合和审查修复由主代理执行；独立子代理仅作只读探查和审查，不继续委派实现，不派生其他代理。验收范围和冻结意图保持。

## Review Triage Log

| Layer／finding | Verdict／route | Evidence／resolution |
| --- | --- | --- |
| blind-hunter：新建嵌套引用直接写入对象 | medium／patch | 主代理核对 catalogue→NewField→standard_insert 与固定 grammar，确认四种已提供创建形状缺少 wrapper。保留现有字段身份与已有嵌套投影，descriptor 携带完整插入路径；共享插入函数一次构造全部缺失层级。公开 ChangeSet 新建 event、behavior、实现类型及 mapping，保存重开逐项核对实际引用父节点，测试通过。 |
| verification-gap：初始化后 SYMBOL 单独修改未覆盖 | medium／patch | 核对实际身份检查已比较 SYMBOL，但组件改名测试不能发现漏掉 SYMBOL 的回归。扩展原测试，用合法无冲突符号通过 prepare／apply 两个入口验证精确拒绝消息，完整 projection、dirty、全部 ARXML 和用户 C 字节保持。测试通过。 |

edge-case-hunter 返回空列表；三层全部收齐后再逐项判断与修复，没有按问题数补造发现。全部修复由主代理执行，无延期项。冻结意图、公开接口及验收范围保持。

## Verification

- 审查后 core all-targets correctness／suspicious Clippy 与增量 quality22 通过。首次 quality 未把受控 Rust 工具加入 PATH，明确报 rustfmt 不可用；修正本次命令环境后重跑正常入口，Requested source checks passed。没有修改全局配置或放宽检查。

- 独立审查后的受影响 workbench 8／8 passed（含四种嵌套创建与 SYMBOL 身份保护），17.38s；初次新增断言把 LocalizedText 当作 struct 导致编译错误，已按公开序列化消息形状修正，未改变产品代码拒绝语义。原完整 21 项 native 的其他实现保持；本轮共享 XML 插入修复由公开事务／保存重开测试覆盖。

- 2026-10-10 最终正常开发 native `/tmp/autosar-workbench-native-acceptance-20261010-complete-contract` 完整 21／21 passed、0 failed／skipped，原 1500s 总期限内完成（1,422,838ms），外层 exit 0；configurationStatus=passed、error=null、nativeTransportUnchanged=true、installed=false。原五项延迟、16,000 对象、全部旧 host／single、批次／源码安全、主题／键盘／中英、真实 exit23／20,001行完整日志／剪贴板及 cancel／late fence 断言保留。完整 IPC JSONL 为1,095,211,129字节。安装、网络namespace及独立消费者未运行；8.5 的完整多组件接收闭包继续本任务完成。README 中英加入实际多组件操作入口与原生截图，HTML本地链接／差异空白检查通过，最终quality22通过。冻结矩阵七行现有实际测试均通过；安装行仅核验边界拒绝与既有断言保持，不宣称安装运行通过。

- 最终受管执行聚焦验收 `/tmp/autosar-owned-focused-20261010-contract` 三项实际通过（development boundary、原完整 exit-23/log 场景、原完整 cancel/late-fence 场景），外层 exit 0、error=null、nativeTransportUnchanged=true；日志 20,001 行、原 12,000 字符 UI 尾部及完整剪贴板、5,000ms 延迟取消后 5,100ms fence、工程与全部源字节断言保留。末尾 stage 断言按真实 ProductMessage 的稳定 key 核验，不将其当作原始英文字符串。临时抽取探查不计完整 21 项；完整正常命令在新私有目录继续。最终 native-webdriver all-targets policy Clippy（15.74s）、quality22 与 rustfmt／diff whitespace 通过。

- 受管入口的真实原生聚焦探查沿原 runner／私有桌面执行，临时抽取原末尾两个场景，未改变仓库完整 21 场景。首次工具 PATH 缺少 tauri-driver 的尝试未计通过；补齐临时工具后确认 0700 目录、真实 exit 23 和 20,001 行完整日志。随后暴露旧验收脚本仍匹配英文 `exit 23`／`failed` 与原始 stage 字符串，实际用户界面已本地化、stage 已为 ProductMessage；修正为场景固定中文反馈及稳定 message key，保留真实状态／日志／迟到 fence 断言，待聚焦和完整验收。

- 2026-10-10：去重后的完整开发 native `/tmp/autosar-workbench-native-acceptance-20261010-sealed` 前 19 项通过，第 20 项在真实子进程启动前被 ProcessSpec 拒绝：验收入口以默认权限创建日志目录，未满足既有 private directory 契约；第 21 项未执行，最终 exit 1。`src-tauri/src/verification.rs` 的 native-webdriver 专用入口改用既有 Unix DirBuilder 模式，在创建时设置 0700；保留 ProcessSpec 权限／链接拒绝，普通发布入口不变。release 构建（39.75s）与格式检查通过，仍待实际受管执行和完整 21 场景。

- 矩阵审阅补齐未知 IREF 的实际拒绝验证：既有实例重命名合同测试同时执行未知 connector 的 prepare／apply 拒绝，比较完整 projection／revision、dirty 状态和全部源文件原始字节；聚焦 1 passed、0 ignored（5.86s），修改文件 rustfmt 检查通过。此前只读断言保留；产品代码及当前原生验收二进制未改变。

- 严格总期限后的聚焦根因确认：`saved_integration_plan` 完整检查 saved sources 后立即经 `integration_plan→integration_sources` 重读同一 snapshot；初始化预览还先执行一次自身 saved-source 检查，成功路径原来封印三次。现只移除同一不可变调用中的重复 seal：saved-plan 保留最前 saved-source 检查，初始化保留原最前 raw error 及全部成员／catalog检查；普通 plan 的磁盘校验、初始化最终检查、Tauri发布及交付晚期 SourceGuard 全部保留。同一 release metrics-enabled公共API、原五项延迟实测：saved-plan 10,304ms／20 reads→5,304ms／10 reads，初始化预览15,322ms／30 reads→5,291ms／10 reads，SnapshotBuild／SourceScan均0→0。最终 workbench7／7（10.21s）、新增 saved-source／legacy拒绝优先级unit1／1（1.10s）、policy all-targets Clippy（3.28s）、normal quality21格式文件及release构建（1m01s）通过，仍须完整21场景证据。

- 2026-10-10：四项工具声明齐全后的 `/tmp/autosar-workbench-native-acceptance-20261010-final-complete` 原 1–19 场景全部通过；第 20 项执行中遭原 1500s 外层期限终止，ProcessResult 为 timeout／exit -15，最终外层 exit 1，无最终 JSON，第 21 项未执行。大型场景 19 真实完成（143.50s）；最大阶段为多组件 345.85s、旧单组件源码初始化／快照 285.81s、v2 导入 111.84s。保持全部场景、延迟及原期限，按实际重复封印路径与 metrics 计数定位正常入口重复工作，未以局部通过宣称完整 native 验收。

- 2026-10-10：流式完整 IPC 证据后的 `/tmp/autosar-workbench-native-acceptance-20261010-stream` 前 19 项全部通过，包含 16,000 对象真实打开／过滤／重复投影计数／取消；第 20 项缺少外层显式 compiler／objdump／git 声明，在原严格工具声明断言处失败，第 21 项未执行。configurationStatus 为 passed、nativeTransportUnchanged 为 true；不是 OOM 或总期限失败。原 JSONL 的 362 条完整事件共 971,191,493 字节，最大完整 project_projection 事件 61,272,138 字节，全部可解析；没有删减历史回复、错误或对象规模。Node 在前 18 项 RSS 492,172kB、此前峰值 670,764kB，流式修改后不再永久持有全部历史回复。
- 外层 builtin prepare 现在先验证四项独立执行工具为现存绝对文件，避免完整配置验收结束后才发现缺少声明；应用配置阶段的工具环境剥除仍保留。现有 NativeBuiltinModeTests 覆盖 missing／relative／不存在文件的早拒绝和成功 prepare，14 tests 通过；最终 Python 全套 47 tests 与 Ruff 通过。补充 variant 实例真实 prepare／apply 拒绝、完整 projection／revision／fingerprint 及源字节不变到既有实例重命名合同测试，聚焦 1 passed（4.90s），rustfmt 检查通过。
- 固定 Linux 执行工具已独立核对：`/usr/bin/gcc` 的完整 identity 与 gcc／objdump／nm SHA-256 均匹配 `runtime/os/toolchain-linux.json`，binutils 2.42、Git 2.43.0、Python 3.12.3。初次工具补齐命令使用 gcc-13，完整名称与固定 identity 不同，在早期主动中止并完成清理（exit 130），不计通过；后续正常完整运行显式使用正确绝对工具路径，同一 release binary、开发 URL、五项延迟及原期限。
- 独立审查从 `origin/master` 对整个功能分支执行正常 quality 入口，151 个格式文件及 C 语法检查通过、exit 0；初次缺少 rustfmt PATH 的环境尝试未计通过，修正 Cargo PATH 和固定 gcc13 后通过。

- 2026-10-10：`/tmp/autosar-workbench-native-acceptance-20261010-verified` 前 18 项通过，包括真实中英文保存／Escape 和 package 创建；第 19 项大工程已打开，过滤阶段 Node driver 被内核 OOM 终止，退出 -9，无最终 JSON；第 19 项未完成，第 20–21 项未执行。内核 01:36:40 记录同一 PID 2463876 被杀、anon-rss 1235676kB；原场景 `ipc` 数组保留全部历史完整回复，大工程增加完整投影后耗尽内存。使用同一 JSONL 流式保存全部事件，修复后仍须完成原完整场景，不能把此前未执行部分记作通过。

- `cargo test --locked --manifest-path core/Cargo.toml --features native-tests`；`npm test --prefix ui -- --run`、lint、build；Python 聚焦测试；质量与 assets check。
- 正常 native：构建 `src-tauri` 的 `native-webdriver`，`uv run --locked python -m autosar_tooling desktop --platform linux --binary <native-webdriver-binary> --builtin-only --output-directory <fresh-private-dir>`。既有 Xvfb／私有 D-Bus／WebKitWebDriver；不操作用户桌面。
- 当前本地可用工具在 `/tmp/autsaro-r6-desktop-tools/{root/usr/bin,cargo/bin}`；这是验收环境而非项目依赖。普通用户 `unshare --user --map-root-user --net` 已实际失败，安装离线验收不得伪造或弱化；本故事开发 native 路径独立验证。
- 2026-10-09：GUI 停止下先前完整 core native suite 通过，232 tests（44／71／2／64／9／42），0 failures；使用项目 venv Python 及固定 gcc／binutils。其后补充 connector 双 IREF 原子创建和缺失 CATEGORY 恢复；六个 workbench 合同测试现聚焦全部通过（8.02s），另覆盖成对修改／不配对拒绝、初始化身份保护、标准 child XML 创建与生成拒绝下保存、长实例名称重绑定及未知 IREF 保护。加载与判型优化后的最终完整 core suite 已全部通过：234 tests（44／71／2／66／9／42），0 failures，包含全部六个 workbench 合同测试；最终 core all-targets Clippy 使用 `-A clippy::all -D clippy::correctness -D clippy::suspicious` 通过。
- UI tests 11 files／97 tests、lint、build 均通过；build 保留既有 chunk size 提示。Python 全套 46 tests、Ruff、相对实现基线的 quality、assets check（0 changes）、diff whitespace 均通过。未改 C／生成 C 模板；本次不以这些检查宣称完整 AUTOSAR 或 MISRA 符合。
- 原生历史尝试保留：native1 缺少 `xdpyinfo`，native2 缺少其 `libXxf86dga` 依赖（仅补充临时工具目录）；native3 暴露键盘发送前 dialog 尚未就绪，native4 暴露关闭 dialog 后后台应用仍 active，均修正为实际就绪／idle 同步，未弱化键盘或并发拒绝。native5 长名称保存重开成功，但 untouched-source 断言误包含正确重绑定的 OS event reference；已改成该源只允许已知引用字节替换，其余源不变。
- 开发 native6 在 `/tmp/autsaro-editor84-native6` 的新增多组件检查已经通过：长名称／窄窗、真实字段与成对引用批次、键盘取消、保存重开、3 槽菜单初始化、用户 C 与实际 source delivery 字节保护、危险成员重命名拒绝。前 9 项通过；第 10 项既有单组件 v2 导入因真实工作重复执行而超过场景 60s，就此失败，后续场景未执行。
- 独立审查对 native6 的多组件工程与实际 delivery 中共 21 份 ARXML 使用固定官方 XSD 核验，全部通过。另经最终 core 公共 Workspace ChangeSet API 恢复缺失 uint8 CATEGORY 的实际输出也通过固定 `AUTOSAR_00053.xsd`。这些是实际 XML 语法证据，不等于完整标准符合。
- 导入性能修复按与 GUI 相同的 metrics-enabled Tauri rlib 和原五项 receipt delays 实测：成员打开 20.60s→10.62s，SourceRead 31→16、SnapshotBuild 2→1、SourceScan 14→7；同一正常 v2 导入 71.23s→41.17s，导入增量 SourceRead 109→64、SnapshotBuild 6→3、SourceScan 42→21。未扩大 60s 场景及 25min 总期限，未删除场景或晚期 seal／source 校验。正常完整开发 native 正在 `/tmp/autosar-workbench-native-acceptance-20261009` 刷新；安装／离线层级仍未通过。

- 后续正常开发 native `/tmp/autosar-workbench-native-acceptance-20261009` 前 9 项再次通过，第 10 项的 v2 导入、7源快照重建／字节检查与旧身份拒绝已成功，随后测试主动 `inspect_integration` 与界面自动 inspection 重叠，被正常 `operation_active` 拒绝；不是同一导入超时。项目 surface 就绪 helper 补充真实 backend operation idle，同步不进入通用 invoke，不掩盖故意并发拒绝场景。
- 独立审查发现既有非法 assembly 配对阻塞无关安全 PERIOD 编辑；transition gate 现允许保留完全未改的既有非法端点关系，新建或改动为非法关系仍拒绝。新增回归证明安全编辑／保存／重开保持非法源字节、生成仍拒绝，另证明修改已非法端点继续拒绝。最终七个 workbench 聚焦测试 7 passed／0 failed（10.03s），all-targets Clippy 通过（3.34s）；此前完整 234 tests 记录保留，不声称包含最后新增测试。

- `/tmp/autosar-workbench-native-acceptance-20261009-final` 的前 11 项通过（含 v2 导入及旧单组件源码初始化／异地快照恢复），第 12 项旧 unknown fixture 断言失效：原 IMPLEMENTATION-DATA-TYPE 已成为本故事支持对象。fixture 改为真实未支持的 SW-ADDR-METHOD，其完整包装 XML 经固定官方 XSD 验证合法；readonly／unsupported domain／逐字节保护断言不变。已检查余下场景，无同类现在已支持标准对象的旧 unknown 假设。

- 2026-10-10：`/tmp/autosar-workbench-native-acceptance-20261009-complete` 前 14 项通过，第 15 项 extension 场景在合法 split /Acceptance package 下显式指定 source-local parent 创建，被过宽 duplicate readonly 规则错误拒绝，属于实际回归。曾尝试让 unknown fixture 使用独立 /PreservedInputs package 并选该唯一 parent，增加全部重复 /Acceptance 只读断言；该错误前提的临时调整已撤回，原失败结果保留，按下条开放集合契约修复。

- 固定 R24-11 ARPackage 开放集合契约复核纠正上一项解释：多个源描述同一 package 是合法 fragment，不应等同重复 element 身份。先前唯一 package／全部重复 package 只读的临时 fixture 调整已撤回；native extension 场景使用原 split /Acceptance packages，显式选择 os.arxml 的 source-local parent。core 仅对不同源的 AR-PACKAGE fragments保留子项创建，包改名／删除独立拒绝路径歧义、source mismatch拒绝、重复非package继续只读。新增公开 API 合同测试证明局部创建／保存／重开及另一源字节不变；开始于10-10的旧premise原生run已主动停止（130退出），不计通过。

- 2026-10-10：split-package 修复后的完整 builtin suite 72／72（含重复非package身份保护和新的split-package局部创建／歧义身份拒绝），workbench 聚焦 7／7（9.78s）、all-targets Clippy（3.33s）、native-webdriver build 与 quality19均通过。此前完整234测试保留，最终增量由这些明确受影响suite覆盖。正常完整native在 `/tmp/autosar-workbench-native-acceptance-20261010-final` 刷新中。

- 2026-10-10：`/tmp/autosar-workbench-native-acceptance-20261010-final` 前 16 项通过，第 17 项 English settings 保存后 Escape未关闭；第18–21未执行，并非总期限。共享 Dialog只在section监听键盘，禁用focused button后body键盘失效；language capability更新也先于UI languageSaving解除。UI行为回归在旧代码下success/failure两项均失败，修复为最上层modal拥有document键盘并将body Tab引回modal；save期间close refusal保留。原生scene等待真实保存按钮enabled/aria-busy=false再发Escape，不强制焦点。完整UI97 tests／lint／build通过，正常release native-webdriver构建后全21原场景将继续验证。

- 2026-10-10：正常 release native第一次运行仅development boundary通过，随后外层窗口枚举创建OwnedProcess时supervisor启动reserve ECONNREFUSED并shutdown timeout，未产生完整scene结果。正常process框架确定性延迟bind到listen之间0.3s，旧代码复现同错误；Owner.start现等待token认证ready响应，保持原30s总启动预算（connect／send／recv逐次取剩余时间），仅注册前ConnectionRefused可继续等待，普通request30s／shutdown3s保持。process22 tests通过（Linux跳过1Windows），Python46／Ruff通过，owner.py固定LF受信asset仅更新1项，ABI及第三方身份不变。release native-webdriver重新嵌入最终资源后继续原21场景，同development URL、五项delays、60s场景及25min总期限，不宣称安装验收。

- 矩阵测试审计：合法编辑／实例配对由 `multi_workbench_edits_standard_fields_and_paired_instance_references` 与 connector 原子创建覆盖；非法关系由 `normal_definition_validation_closes_multi_schedule_routes_types_and_handles`、`equal_width_implementation_types_and_network_local_double_binding_are_rejected` 及 `multi_workbench_existing_invalid_connector_allows_unrelated_safe_edits` 覆盖，源诊断与生成拒绝仍保留。未知 IREF／变体 prepare＋apply 的最终聚焦补测通过；stale、外部变化和无半批次由既有 core／native batch 与 source-safety 场景覆盖。全槽保护由 `multi_workspace_initialization_reopen_and_regeneration_preserve_every_user_source`、初始化身份拒绝与原生多组件真实菜单场景覆盖。旧 host／single core 行为已实际通过，安装边界的 Python 拒绝测试通过；开发模式不提供安装执行证据。长名称／窄窗／键盘／主题／中英由本次正常完整 21 场景实际通过；安装执行证据保持未运行。
