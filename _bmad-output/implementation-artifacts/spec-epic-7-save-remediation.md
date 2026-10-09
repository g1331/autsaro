---
title: 'Epic 7 保存数据保护与后端提交围栏整改'
type: 'bugfix'
created: '2026-10-09'
status: 'done'
baseline_commit: '977e9100d37b48eb58fef41a23d2bc96acca4b3d'
route: 'dispatch'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

关闭 Epic 7 回顾中的四项 open 行动，修复外观／语言保存覆盖被拒绝设置，以及源保存最终发布覆盖外部新文件的窗口；同步保存清理失败契约，保护取消／失效 Operation 的后端提交拒绝。完成后以新证据重新评估同一份回顾，并交付至 master。

## Boundaries & Constraints

**Always:** 保留已有回顾及 sprint 内容，包括本地提交 `977e9100d37b48eb58fef41a23d2bc96acca4b3d`；沿当前独立分支 `t3code/fix-settings-source-save` 分阶段提交、推送、创建并关联 PR、审查、等待必需 CI、合并并核对目标 master。错误必须保留原字节、恢复资料和当前状态，不能伪造成功。沿用现有 Rust 测试、设置反馈和保存事务入口；跨平台结论以实际执行范围为准。

**Never:** 恢复已取消的原生／发行组合验收门槛；把旧失败、未执行或历史证据改成通过；借规格对齐将正确的发布后 clean 基线改回 dirty；引入设置自动重置、新配置框架或独立验收系统。

## I/O & Edge-Case Matrix

| 情形 | 输入／状态 | 预期行为 | 错误处理 |
| --- | --- | --- | --- |
| 设置被拒绝 | 未知字段或损坏 JSON，分别保存外观／语言 | 拒绝写盘，原设置字节、settings_error、会话和 fingerprint 保留 | 返回现有本地化设置错误 |
| 设置正常 | 有效设置、并发类别保存、外部改动或 stage 冲突 | 正常类别保存保留其他类别；现有并发／外部保护成立 | 保留已有拒绝和反馈 |
| 源发布冲突 | 旧源已备份，最终发布前外部创建原路径 | 发布拒绝，外部字节不被替换；恢复资料可定位 | 保存失败且保留需要人工恢复的备份 |
| 回滚发布冲突 | 回滚准备恢复原路径时出现外部文件 | 恢复拒绝覆盖，保留外部字节与备份 | 返回恢复问题，不冒称完整回滚 |
| 发布后清理失败 | 全部源及 manifest 安装成功，备份删除失败 | saved 基线与磁盘一致、dirty=false；保留备份并阻断后续保存 | 返回清理错误，明确恢复限制 |
| 旧 Operation | 已取消或工程输入已失效 | commit 拒绝，闭包不执行，当前 workspace/fingerprint 保留 | 保持取消／过期错误语义 |
| 有效 Operation | 当前归属有效，期间只保存外观／语言 | commit 正常，表面设置不使工程操作失效 | 保留现有正例 |

</frozen-after-approval>

## Code Map

- `src-tauri/src/workbench.rs`：AppState::new 保留 settings_error 和原 bytes；configure_appearance/configure_language 缺拒绝检查；configure/select_target 的显式工程设置路径另有职责。Operation::commit 已核对 fingerprint 和 active id，复用 Fixture 补回归。
- `src-tauri/src/configuration.rs`、`ui/src/workbench/settingsActions.ts`：两项设置调用与已有错误反馈，不需新增 IPC 或界面。
- `core/src/arxml/persistence.rs`：install_staged、restore_backup、save_sources_with_cleanup；源与 manifest 共享事务；已有 Scratch、外部修改与清理故障测试。
- `core/src/arxml/application.rs`：已有同目录 hard_link 拒绝覆盖发布先例，可复用标准文件系统原语。
- `.github/workflows/checks.yml`：普通 core/desktop 单测现为 Linux；已有双平台 C 分析准备任务编译 core，可复用 Windows 构建补执行保存单测，不恢复桌面发行门槛。
- `spec-epic7-source-project.md:49`：过时的清理失败 dirty 描述；中央 `spec-epic-7-configurator.md:141` 与实现／故障测试已保持正确基线。
- `epic-7-retro-2026-10-09.md`、`sprint-status.yaml`：四项行动及 rejected 判断；历史复现保留，追加整改证据并更新当前判定。

## Tasks & Acceptance

**Execution:**

- [x] `src-tauri/src/workbench.rs` — 阻断被拒绝设置的外观／语言隐式写入，补未知字段、损坏 JSON、原字节与状态保留回归；保留正常类别保存及并发保护。
- [x] `core/src/arxml/persistence.rs` — 最终安装及必要回滚采用拒绝覆盖发布；复核所有相关调用与失败清理，确定性制造发布窗口冲突，断言外部字节和备份。审查回滚的读取／移除边界，避免仅把覆盖窗口转移到恢复路径。
- [x] `src-tauri/src/workbench.rs` — 真实 AppState 取消及输入失效回归，断言旧提交闭包不执行与当前状态不变；保留有效操作及表面保存正例。
- [x] `spec-epic7-source-project.md`、中央规格 — 对齐发布失败与发布成功但清理失败的说明，记录本次实际验证范围。
- [x] `.github/workflows/checks.yml` — 在已有双平台构建入口执行保存回归，取得 Windows 与 Linux 的真实结果。
- [x] 原回顾、sprint、本规格 — 按独立完成条件更新四项行动及当前验收判定，并将远端交付关联到 [PR #13](https://github.com/g1331/autsaro/pull/13)；最终 CI、合并及 master 核对以该 PR 实际结果和交付核对为准。

**Acceptance Criteria:**

- Given 被拒绝设置，when 分别保存外观或语言，then 返回错误且原字节与会话错误保持。
- Given 最终发布或恢复目标出现外部文件，when 保存事务继续，then 不覆盖外部字节且恢复资料保留；Linux／Windows 普通测试均验证。
- Given 全部文件已发布，when 清理失败，then 规格与实际 clean 基线、错误及备份保护一致。
- Given Operation 已取消或输入失效，when 旧 Operation 提交，then 闭包未运行且当前状态保持。
- Given 四项条件有对应证据，when 重评原回顾，then 状态与判定准确且不扩大验收范围。

## Implementation Notes

- 用户于本轮批准完整实施及 PR 合并，无需重复确认常规步骤。独立分支原有回顾提交保留。
- 设置保存拒绝已有 settings_error；源发布使用 hard_link。回滚通过 create_dir 预留恢复目录，移动实际目标后核验，避免读取后删除的竞态；正常回滚仍可恢复。发布后 stage／backup 清理错误保持 clean 基线及恢复保护。
- 本地 Linux：Core 20 lib＋71 builtin integration＋1 c_analysis 全通过；Tauri 8 lib 全通过；UI 92 全通过、build 通过。Windows 保存回归由 PR 的既有双平台任务执行，尚待实际结果；回顾重评和远端交付项据此保留未完成。

- 评审补强后完整本地验证：Core 24 lib＋71 builtin＋1 c_analysis（96）；Tauri9；UI92；build、quality、diff检查均通过。新增双语stage别名错误、发布前备份预留、rollback残留保护、真实多文件事务及目标设置变化回归。7项评审问题全部关闭，定向复查无剩余发现。

- 首轮PR CI run `37882013957` 的17项必需检查通过；Windows保存回归9通过／1失败（外部目录原位置断言），Linux10通过。保留失败证据，补Windows文件句柄捕获后重跑。

- Windows平台修复提交 `97bb803c4c6ff04d348a2d178860788d782c7a23` 按已打开普通文件句柄捕获，应用到初始备份和回滚；定向ABI／路径／生命周期复查无发现。第二轮CI `37882612373` 已实际验证Linux11／Windows12项保存回归全部通过，Windows含长中文路径和打开后路径替换。最终本地Core97、Tauri9、UI92、build与quality通过。
- 四项行动全部done，原回顾复评accepted，11条故事无pending。此规格done为整改实现、审查、开发验证与复评完成；远端合并以PR13实际记录为准，提交本节时尚未执行合并。完整交付授权仍按冻结意图执行至master核对。

## Spec Change Log

## Review Triage Log

| 来源 | 判定／处置 | 核对依据 |
| --- | --- | --- |
| blind-1：不支持硬链接时先移走原文件 | medium／patch | 原实现移动 original 后才 hard_link，恢复依赖同一原语。先预留 backup 验证硬链接能力，拒绝时原路径不变。 |
| blind-2：遗留 stage 是正式源的别名 | medium／patch | hard_link 确实共享 inode；清理错误已有 clean 基线及备份保护，仍需明确别名恢复契约并验证后续变化拒绝。 |
| blind-3：缺少真实多文件冲突回归 | medium／patch | helper 不覆盖已安装兄弟文件的逆序回滚及事务清理；补源与 manifest 的 Workspace 故障用例。 |
| blind-4：缺少真实工程设置变化回归 | medium／patch | begin(Edit) 已覆盖 fingerprint 失效，但未覆盖新设置保留；补 select_target 真实入口。 |
| blind-5：stage 清理错误语义缺失 | medium／patch | 新错误只有裸路径，与产品本地化反馈不一致；补双语消息说明已发布及恢复限制。 |
| blind-6：恢复检测遗漏 rollback 目录 | medium／patch | ensure_no_recovery_backups 只扫描 bak，手工移除 bak 后遗留恢复目录可能进入下一保存；纳入现有守卫。 |
| edge-1：外部目录被捕获后无法硬链接恢复 | medium／patch | rename 能移动目录，后续 hard_link 拒绝目录导致原位置空缺；捕获前用硬链接预留文件目标，避免移动目录。 |
| CI-Windows：目录仍被移动 | medium／patch | 首轮 hosted Windows 的目录回归9/10通过、original.is_dir失败；Linux普通文件预留不能代表Windows目录移动语义。按已打开普通文件句柄捕获，并应用到初始备份及回滚，不弱化断言。 |
| verification-gap | 无发现 | 独立镜头未报告验证缺口；Windows 及最终交付证据仍待实际 CI，不作为已通过。 |

## Verification

- `cargo test --locked --manifest-path core/Cargo.toml`：保存故障用例及已有核心行为通过。
- `npm run build --prefix ui` 后执行 `cargo test --locked --manifest-path src-tauri/Cargo.toml`：真实设置与 Operation 单测通过。
- `npm run test --prefix ui`：已有设置调用与晚结果处理回归通过。
- `uv run --locked python -m autosar_tooling quality --base d3239c5ca5cbf8a1241c1ea9cd9e40ee3d7c49a9`、sprint 校验、`git diff --check`：正常质量入口与工件一致性通过。
- PR 上实际复核 Windows 保存回归、必需 CI 与审查；合并后获取远端引用并验证全部交付提交进入 master。单测与 CI 结论不替代原生 IPC、安装器、硬件或官方符合性验收。
