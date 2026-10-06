# Epic 7 开发提交与后续发行验收交接

## 1. 接手结论

Epic 7 的 7.1–7.10 主体实现、Core／Tauri IPC／React 接线，以及开发测试和代码复核已完成。用户2026-10-06明确将当前收尾限定为开发与测试，真机、原生桌面和完整发行验收留给后续CI；本次继续整理开发成果的本地提交，不push、不重启原生编排或扩建隔离设施。

当前中央开发规格为 **done；Story 7.1–7.10 为 done，7.11 为 review，Epic整体仍为 in-progress**。完整发行场景0/3及四个跨故事出口保留未闭合，不当成当前开发提交的前置工作，也不改写为passed。原停机时0/11 Done属于历史状态，以当前中央规格和sprint为准。

接手开发成果以[中央实施规格](spec-epic-7-configurator.md)的最终验证与复核记录为准：全量177项集成通过，最终代码补跑13项库单元与19项Workspace回归，UI／质量／Core及桌面Clippy通过。原生安装链另按后续CI范围验证；以下历史包和旧场景记录不代表最终修复后的发行结果，不自动恢复长场景或重复未变检查。

## 2. 权威工件与工作区

- 原需求和故事验收：`_bmad-output/planning-artifacts/epics.md` 的 Epic 7，特别是 Story 7.11；PRD 的四个跨故事退出要求不由本文替代。
- 架构：`_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md`。
- 中央实施规格：`spec-epic-7-configurator.md`。
- 分项规格：`spec-epic7-native-rules.md`、`spec-epic7-definitions.md`、`spec-epic7-source-project.md`、`spec-epic7-workbench-ui.md`、`spec-epic7-native-delivery.md`、`spec-epic7-native-acceptance.md`；均位于本文同目录。
- 状态：同目录 `sprint-status.yaml`。继续开发、复核和状态同步仍使用仓库安装的 BMad；本文是用户要求的交接，不另建任务体系。
- 分析开始时 `git status --short` 报告 staged 0、unstaged 71、untracked 57 个条目；其中包含完整实现、资产、规格、测试及验收脚本。未提交、未 push，不能用 `reset`／`clean` 恢复整洁；untracked 不等于可删除缓存。本文及停机说明不属于原产品冻结清单。

### 实现定位

| 领域 | 接手入口 |
| --- | --- |
| 内置结构／安全打开／规则 | `core/src/rules.rs`、`rules/`、`schema.rs`、`arxml.rs` |
| 定义、默认、类型和引用投影 | `core/src/definitions.rs`、`definitions/`、`arxml/projection.rs` |
| 原子编辑、批次、结构和安全保存 | `core/src/arxml/changes.rs`、`persistence.rs`、`project.rs` |
| 原创工程和 live application | `core/src/arxml/standard_template.rs`、`application.rs` |
| 生成计划、用户代码归属、v2 封存／重开 | `core/src/prepared.rs`、`generator/delivery/`、`integration/handoff.rs`、`scripts/ecu_tools/workbench_v2.py` |
| Session／Operation／IPC／后台任务 | `src-tauri/src/lib.rs`、`workbench.rs`、`configuration.rs`、`verification.rs` |
| 统一状态、草稿保护、树表、设置和工具窗口 | `ui/src/workbench/useWorkbench.ts`、`PreviewDialogs.tsx`、`ToolWindows.tsx`、`ObjectEditor.tsx`、`ui/src/pages/` |
| 原生验收 | `scripts/autosar_tooling/desktop.py`、`native_offline.py`、`desktop_builtin_scenario.mjs`、`desktop_scenario.mjs` |

约束保持：源字节权威、唯一 Workspace／Session／reducer、四校验域分开、原子 prepare/apply/save、用户代码保护、严格 v1/v2 分派和 seal。不增加生成 allowlist，不声称实机支持或标准认证。修改 C、C 生成模板或运行时时先加载 `misra-c2012`；本次停机未修改产品代码。

## 3. 历史发行身份与可复用证据

2026-10-05停机时已构建产品的源码库存摘要：

`f5872804e41350ca55882c12acae553b78ba0b7dec0da105bacaaa0a22f1befc`

该摘要对应停机前已构建产品的冻结库存。2026-10-06又修正保存基线和UI批次提交状态，因此它及下列包不代表当前提交源码；原证据仅按对应历史身份和行为范围保留，不能冒充新版本发行结果。

| 历史保留产物 | SHA-256 |
| --- | --- |
| MSI | `1b806e4349daeb5c779021ec101a7f2848ae15bc18cd81dbe26b2e69792bf0b0` |
| deb | `6f1924a16463b6a26f36243506b2b8376d1977cf4e11ed79f0b81bc350e45e86` |
| AppImage | `e62653c320ee99a630cd01c496bb9fa2cc416d33ed217cef4b3198a3010781ba` |
| 最终 Linux executable | `610fa46f7712db0ef27e6e6511521f6da922944a8ae520e082398ba221d84138` |
| 随包 Owner | `db6796c37ca9e4193c811d011e1a448ba5c2538f5b128d63743c9f911bcefa69` |
| CAN generated README | `2220161e5ac02ade489a0507bc505803c362ef95a4f25e60b4b579ef6c42ed63` |
| RuleSet | `7ac95f470079dcad8169d2f096f241f5d8b646dca0bca96593e3baef804f3a07` |

三个历史包在清理前另存并逐一核对上述摘要，不删除或重新seal。当前产品修复已改变构建输入；后续CI发行验收须从提交后的源码重新构建，不以这些旧包代替。

### 历史已证明行为（按原输入与身份复用）

- 最终纠正身份的 Windows Epic 7 Core 回归 60/60、UI 构建和增量 quality 通过。命令及范围见中央规格；不是本次停机重新运行的结果。
- 原 original-v1 独立消费者在 Windows 和 Linux 均通过；API-v2 六包的双平台消费者和 exact import/reproduction 通过。这些不是 GUI producer 或完整发行场景的替代。
- 最终真实 GUI 交付四包：NorthSensor CAN 74 文件、SouthActuator CAN 74 文件、GatewayReference ECU 169 文件、GatewayUserApplication 169 文件。原消费者真实 C99/native 编译运行、源包前后不变；matching Core 导入新 receiver 后全部 payload 和快照精确再现。
- 用户 C 包的低 31 位掩码算法：六个边界输入的 12 组完整 CAN／DID 输出一致，另有原 echo5、N_Cr recovery4、malformed0 和严格 Owner 关闭证明。无需重跑。
- 最终 deb 原检查 1–8、focused 9–16 及自然 17 已有真实分段证据，不能拼成完整 scene。覆盖创建、编辑、保存重开、结构／引用、搬移、v2 快照导入与关键拒绝、用户应用、七类值／跨文件批次、安全拒绝、扩展缺失／恢复、主题／尺寸／键盘和问题焦点。
- 大输入自然检查：16,000 preserved types／2,661,220 字节；取消 229.52ms；快照内读源／规则加载等计数未增加。四次 filter+工具切换实测约 2.29／8.01／2.13／6.51 秒，不能说全部小于 5 秒。

## 4. 后续CI未闭合项（本次开发提交不执行）

| 剩余项 | 当前事实 | 接手动作／完成条件 |
| --- | --- | --- |
| deb 检查18：真实失败长日志与完整复制 | 历史 `n-aoser988` 的问题焦点通过；无后代探针要求reclaimed=true的失效断言已在2026-10-06修正为实际Exited／exit23／false，但新场景未执行到此项 | 后续CI核对20,001行完整日志首尾和错误、12,000字符有界UI尾部、真实完整复制及严格收尾，不伪造flag／新增子进程夹具 |
| deb 检查 19：取消和晚结果 | 原完整取消、延迟结果／epoch/source 保留尚未通过 | 保留原取消、5.1s late fence、输入身份和原字节断言，在同一真实产品路径完成，不用 mock 或只观察父进程退出替代 |
| 当前及历史 cleanup_unconfirmed | 最新外层 Node PID2667／scope `a019c059ab46429ea6b897707e32f6a1`、exit1 为真实 strict RED，原因 UNKNOWN；它与内层无后代 Exited23/false 是两件事 | 未来只对真实新失败当场获取足以区分根因的证据。旧记录保持 RED／UNKNOWN；后来 absence、boot 改变或插桩通过不能追认旧关闭成功 |
| 完整deb场景 | 历史场景未整场通过；2026-10-06新尝试1–7项passed、第8项STALE_DELIVERY失败 | 后续CI从当前源码重建后验证原端到端正常／拒绝／收尾；不把分段汇总标整场通过 |
| AppImage | 历史包已构建，完整AppRun未通过；当前源码又有产品修复 | 后续CI使用当前源码的新AppImage验证真实AppRun安装边界与组合链，不借用deb结论 |
| Windows MSI | 最终包已构建／quiet extraction；隔离激活收到 provider `cyber_policy` 拒绝 | 需要允许且不干扰用户桌面的隔离执行环境。不能重试原提升原语、换模型、绕过限制、修改全局网络策略或靠脚本补丁宣称通过 |
| 旧兼容／官方资源链 | 两份官方档案配置成功；旧七文件经普通 native 入口真实 MULTIPLICITY90 拒绝；原诊断／DTC／有序 DID、旧 v1 GUI 完整链未闭合 | 普通 `open_project` 不因配置档案自动变 legacy；显式 `open_handoff_project` 按 v1 格式选 legacy。原资源种子只有 Alpha／Beta host-v1，没有已确认 sealed ECU-v1 启动包。先核清原入口／输入契约，完成实际 GUI producer 和独立消费；不擅造新 producer/fixture，不能拿 v1 CLI 已通过冒充 GUI 闭环 |
| 最新父级 legacy GUI 尝试 | 父 PID60892 exited1；最早可读内层原因是 `Native chooser closed without an observed authorized result`，未取得完整 v1 GUI 成功证据 | 这不是已证明的产品校验失败或 Owner 回收缺陷；保存原 chooser／ACK 失败，不因外层 `OwnershipError` 类名误判。临时 focused 驱动已恢复，不再自动重试 |
| 最终复核／状态／说明 | 开发复核和测试已完成；7.1–7.10 done，7.11 review，Epic in-progress；完整发行0/3 | 提交开发成果时保留原PRD／story发行要求，后续CI结果再更新发行出口；不把未运行写成通过 |

### 已知验收前提，避免重新踩坑

- 原诊断 locator 的 code+message 可重复，曾匹配 24 行。应以真实复制出的 diagnostic 全身份选同一行，再验证 field／object 焦点；不是要求产品改变正确的模块焦点。
- 缺扩展缓存时，XML 中不存在的 Counter 不能凭空生成默认 descriptor。已经通过的检查使用实际 persisted Config 消费者只读，并保留内置消费者可写、原字节和恢复／移除边界；不要再物化新参数来满足脚本。
- 深 TMPDIR 引起真实 `AF_UNIX path too long`；临时 `/tmp` 版本又触发 private-log-directory 拒绝，并有后续实际 ENOENT，丢失 raw 不能宣称已归档。
- 最后已验证前提为 Linux 私有短路径 `~/.cache/an/n-XXXXXXXX`、leaf0700、launcher 启动首个 Owner 前 scoped umask077；同时保留真实 uid/mode/device/inode 和外层／namespace mask 回执。它不证明产品支持任意长 TEMP 或 permissive umask。
- 不按旧文案或按钮存在推断异步操作完成；核对实际 enabled、operation idle 和 requested input identity。保留原 bytes／拒绝／焦点断言，不用 sleep 伪造同步。

## 5. 停机事实与接手纪律

原编排 `run_fb2ce2ebdd0c` 的三个活动 Dispatch 均收到用户停止要求，发送真实 `worker_done outcome=failed`／user_cancelled 并结算；这表示未完成范围被取消，不是产品测试失败或 Epic 完成。

| 工作端 | Dispatch | 处理 |
| --- | --- | --- |
| Workbench UI／旧 Windows 跟踪端 | `ctx_cf7433b61d37` | 结算，精确终端关闭，`ptyKilled=true` |
| 独立交付消费者 | `ctx_da65dda1a630` | 结算，精确终端关闭，`ptyKilled=true` |
| 原生发行验收 | `ctx_93fedd45c4ec` | 结算，精确终端关闭，`ptyKilled=true` |

Orca 先因 `user_takeover` 保留生命周期资源，随后按本次用户明确停止授权关闭三个精确终端；未关闭当前协调终端或其它用户终端。收件已 ACK，reclaimable 列表为空。Run 和历史记录保留，不是运行中的调度器。

本次只读 Linux 全进程表未见产品／私有 Xvfb／private Owner 进程；Windows 精确缓存可执行路径查询 count=0。工作端报告没有待执行 OwnedProcess／service handle。**这是当前停机与删除安全证据，不改写任何历史 cleanup_unconfirmed。** 没有运行新产品场景、测试、构建、UAC/WFP 激活、服务重启或 WSL shutdown。

新的接手不复用已结算 Dispatch IDs。先读本文，恢复必要的单个环境和目标路径；不要自动恢复旧无限工具迭代。常规开发决策由工程师承担，只有目标、重要兼容契约或高影响权限改变才交用户决定。

## 6. 证据保留与缓存清理边界

本机保留目录（仓库外，不属于待删除 cache）：

`C:/Users/admin/Documents/Autosar-handoffs/epic7-run_fb2ce2ebdd0c/`

- `bundles/`、`bundles.json`：最终三包与已复核身份。
- `windows-evidence.zip`、`windows-archive-manifest.json`：Windows raw receipts／logs／截图／临时消费者与再现树／必要移交输入；6,719 个归档文件逐一 SHA-256 验证。
- `linux-evidence.tar.gz`、`linux-archive-manifest.json`：run-scoped Linux raw evidence、43 个已核对 source-checkout-boundary 属于本 Run 的 native scratch、真实 GUI sealed originals／consumer copies、输入和私有状态；GNU tar 与原文件内容比较 exit0。
- `root-build-evidence.tar.gz`、`root-build-archive-manifest.json`：root-owned build receipts，不保留可重建的大型编译目录；GNU tar 比较 exit0。
- `orchestration-stopped.json`、`native-worker-final-output.json`：停机记录。另一终端的 local:// inventory 在协调端不可读，不依赖这个不可访问 URI；共享磁盘 raw evidence 和可读停机记录另存。
- `preservation-plan.json`、`cleanup-result.json`：精确根路径、归档映射、实际删除结果及未清理边界；清理已完成，无删除错误。

归档内部保留旧绝对路径字段、真实失败和原字节，不重新 seal／归一化／冒充新的生产者。移动后旧原路径将不再存在：先查各 archive manifest 的 `member`，按所需子树解包到新私有目录；必须重新获得运行所需的真实目录授权，旧 snapshot checkpoint 不是可直接继续运行的进程。

清理只覆盖本 Run 的 Windows `AutosarVerifier` 环境、已识别 Epic 7 临时根、ordinary/root Linux `autosar-acceptance/run_fb2ce2ebdd0c`、43 个已确认 scratch 和明确同任务的 media scratch／显示锁。可重建的 target、node_modules、私有 Python／WebView runtime、解包发行目录和重复 checkout 不放进证据归档；必要包／raw evidence 已先保留。

实际删除 Windows 44 个根（私有环境及临时文件／目录），其中 regular files 约 **3.22 GiB**；Linux ordinary 45 个根清理前占用约 **4.26 GiB**，另删除 root-owned 编译环境约 **3.16 GiB** 和 5 个零字节 owned 显示锁。ordinary 根逐一 lstat 得到 ENOENT，root 删除命令含 absence 检查并 exit0。以上两平台计量分别为文件字节和 Linux allocated blocks，不混称为宿主磁盘已释放量。

保留的三包及压缩证据约 **0.83 GiB**，不是运行环境。归档索引另核对真实原始 GUI 用户包 169 文件、Gateway 169、South／North 各 74 均保留；consumer copy 中额外构建日志不当成原始 seal payload。后续只按需要解包一个环境，不能直接复用已删除的旧 checkpoint 路径。

**不删除**仓库源码／untracked 实现、官方 XSD/MOD、`third_party`、仓库现有 `core/target`／`src-tauri/target`／`ui/node_modules`、共享 Cargo/npm/uv 缓存、vcpkg、安装工具、用户应用状态或 WSL 发行版。它们的历史归属不全是 Epic 7 私有环境。WSL 内文件删除不等于宿主 VHDX 自动压缩；本次不关闭／压缩／重建 WSL 来追求宿主磁盘数字。

## 7. 后续CI发行收口顺序（不在当前提交阶段执行）

1. 从当前提交源码构建新的发行包，在允许的隔离环境恢复一个最小Linux执行环境，不恢复所有历史副本。
2. 第18项无子进程flag前提已修正；后续CI定点运行真实owned日志／完整复制、取消／晚结果和严格关闭。必要修复只针对最早契约违反，不另造通用验证设施。
3. 核清原 legacy 入口和既有种子，完成原诊断／DTC／有序 DID、两类 v1 GUI 与独立 consumer 的实际闭环。明确不存在现成 ECU-v1 seed 的当前前提，不默换普通 native 工程或虚构成功。
4. 验证当前源码的新deb、AppRun；Windows使用允许的隔离环境独立完成，不绕过现有provider拒绝，不能用Linux替代Windows。
5. 历史四份GUI v2消费按原输入与身份保留；产品输入变化时复验受影响行为。根据新发行证据关闭7.11剩余出口，再同步Epic和使用说明，不回退已完成的开发状态。

第5／6节记录原停机和缓存清理事实；当前开发收尾结果见第8节。本文及本地提交不宣称Epic 7完整发行验收已通过。

## 8. 2026-10-06 续接进展与范围讨论

用户授权依据本文收尾，随后明确不要为强求隔离环境扩建开发脚本，并最终要求本次仅确保开发和单元测试完整，真机／原生桌面／完整发行验收留给后续 CI。该范围已更新到中央规格；后续接手不自动恢复原发行长场景或隔离设施开发，也不将旧未验范围写成已通过。

本轮仅修正既有场景两处验收检查：第18项无后代探针要求实际 `Exited / exit23 / descendants_reclaimed=false`；第19项复用现有读取／比较函数核对取消及5.1秒晚结果后的磁盘原字节。两项独立只读复核无阻塞发现，Node语法和锁定Prettier检查通过。产品代码、Owner、冻结包及seal未改，原Core／UI和独立消费者证据继续按各自范围复用。

保留三包摘要已逐一核对，Linux工具与包在仓库外恢复：`/home/ubuntu/.cache/an/epic7-tools-20261006` 和 `/home/ubuntu/.cache/an/epic7-resume-20261006`。使用原完整deb驱动的新尝试在 `/home/ubuntu/.cache/an/n-e69tazav` 留下真实1–7项passed、第8项failed；最早错误为重开工程时投影请求 `STALE_DELIVERY`，当时截图仍显示正在读取投影，尚未区分脚本同步与产品缺陷。外层另有 chooser授权结果缺失，namespace／外层exit1，日志在 `/home/ubuntu/.cache/an/e-deb26`。本轮 `native-bus-closed.json` 记录五个scope关闭返回cancelled、私有bus socket消失；这只证明本次资源关闭，不追认历史RED。

第18／19项没有实际执行到，AppRun未启动；原完整发行0／3保持。开发收尾运行既有 Python／Core 单元与集成测试、质量／lint、UI 构建、桌面后端 build／Clippy，并完成独立代码复核。已修正备份清理失败后的保存基线与批次提交后刷新失败的 UI 状态，最终库单元13／13与Workspace回归19／19通过，两项新增Rust回归均实际执行；本轮产品源码已改变，原冻结包仅保留为历史身份，不代表这些修复后的产品版本，不在本轮重打包。开发结果及最终状态记录在 [中央实施规格](spec-epic-7-configurator.md)，原生历史结果仍见 [原生验收规格](spec-epic7-native-acceptance.md)。

2026-10-06开发提交前检查点：中央及五份实现规格为done，Story 7.1–7.10为done；7.11为review，原发行出口留待CI，Epic整体仍为in-progress。全量177项集成、最终13项库单元与19项Workspace回归、Python单元、质量／lint、UI构建及Core／桌面Clippy通过；当时尚未commit／push。未来发行包须从修复后的源码重建，原保留包不作为新版本发行证据。

## 9. 本地开发提交范围

用户随后要求接续完成收尾。本次只整理并记录已有开发成果的本地提交，更新本文开头的当前状态及历史包适用边界；不修改产品代码，不重跑未变的耗时集成测试，不启动原生场景、重建发行包或恢复编排，也不push。

提交包含Core／IPC／UI实现、源码交接工具、原有验收入口的改动、回归测试、图标、使用说明与BMad工件；本机缓存、归档、原始临时日志、官方档案不入库。源码完成依据仍是中央规格的最终测试和三份独立复核记录；Git提交前另外检查完整暂存差异的空白／冲突与文件归属，提交标识以本地Git历史为准。

开发提交不升级7.11或Epic的发行验收状态。后续CI从该提交源码重新构建和验证，旧证据、失败与策略阻断继续保留原身份。
