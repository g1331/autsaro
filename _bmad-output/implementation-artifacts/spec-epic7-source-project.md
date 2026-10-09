---
title: 'Epic 7 稳定源投影、原子配置变化与可搬移工程'
type: feature
created: '2026-10-03'
status: done
route: dispatch
baseline_commit: '52367974cab5f73ce5a336936d5c0f352290dae1'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/spec-epic-7-configurator.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
  - '{project-root}/_bmad-output/planning-artifacts/epics.md'
  - '{project-root}/_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md'
---

<frozen-after-approval reason="derived from approved Epic 7 intent">
## Intent
实现 7.1 源投影与 7.3–7.6 全部事务/结构/模板/成员能力：不依赖官方档案的默认配置路径，真实 stable IDs、类型/引用/批次/创建/改名/移除、原字节安全保存和搬移重开。不以目标生成错误封死所有安全修复。
## Boundaries & Constraints
同一 Workspace 原字节权威；50 MiB/路径/链接/DTD/UTF-8 防护；后台 opaque IDs，改名保留、删除不复用、重开换 epoch。prepare 不改内存/磁盘，apply 完整 prospective graph 一次发布，所有冲突全拒绝。默认不隐式写值，未改 lexeme/未知内容保留，无法确定引用/变体影响拒绝。Schema 实际错误阻断保存，未变旧 definition witness 可保留、新增/恶化拒绝。不得提交或推送。
</frozen-after-approval>

## Code Map
- `core/src/arxml.rs` 及 `arxml/{xml,host_profile,persistence,integration_editor}.rs`：当前原字节、补丁、目标视图、PreparedSave；扩大同一实现。
- `core/src/project_model.rs`：协调者已定义 ProjectProjection/FieldDescriptor/ChangeSet/内部 op tag 与各种 Reference/Value/Object/FieldRef。不得改 wire 或建立第二套业务身份。
- 原生 worker API：`rules::rule_set_identity()/coverage()/validate_native(&[(&Path,&str)])`。
- 定义 worker API：`DefinitionCatalog::builtin/get/children/definitions/validate_documents/fingerprint/accepted_extensions/required_extensions/accept_extension/remove_extension/restore_extensions`。实现源码会并行到达，接口问题向协调者提问，不写 mock/fallback。

## Tasks & Acceptance
- [x] 独占 `core/src/arxml.rs` 和整个 `core/src/arxml/`，新增 `changes.rs`、`project.rs`/必要子文件、必要产品原创模板文件；新增 `core/tests/support/epic7_workspace.rs`。只为 API 迁移可修改现有 `core/tests/support/` 非 epic7_* 测试及 `core/src/bin/`；不碰 generator/prepared/integration、模型、导出、Tauri/UI、测试入口。
- [x] 正常 `Workspace::open(paths)`、`create(directory,name)` 默认 builtin；旧资源参数入口改为明确 `open_legacy(paths,schema)`/`create_legacy(directory,name,schema)`，只用于真实旧 v1/oracle。迁移你权限内所有调用方，协调者迁移其他生成/Tauri 调用；不以空 path 或隐式环境 fallback 判断模式。
- [x] `project_projection(&self,input_fingerprint:&str)->Result<ProjectProjection,String>` 使用快照索引，不为筛选重复扫描；全部源/模块/container/字段/引用实际身份/归属；未知可见只读；绑定 ruleSetIdentity/definitionFingerprint 和分域结果。`source_text(&self,source_id:&str)->Result<String,String>` 仅当前 owned snapshot。
- [x] `prepare_change(&self,changes:&ChangeSet)->Result<ChangePreview,String>`；`apply_change(&mut self,changes:&ChangeSet,revision:&str)->Result<ChangeOutcome,String>`：原值/类型/默认/引用/完整规则身份/外部状态/规范化批次核对，prospective graph 真正原子提交；createdIds/createdFields 只成功分配，epoch/stable identities/selection 不丢。
- [x] 全面 set-value/set-reference/create-instance/rename-instance/remove-instance，父子/choice/基数/必需 new 字段和入站闭包；大整数/非有限 float/函数名/冲突/同名/有引用删除明确拒绝，不使用全局文本替换或循环 IPC。
- [x] PreparedSave 继续 staging/backup/recovery/外部摘要；fingerprint/规则/定义/成员接纳绑定一次事务，未改文件不写，失败不会假 clean。新增或恶化 definition witness 不能因同 code 放行；未知生成能力不是保存总门。
- [x] workbench-project.json v1 安全相对 inputs/applicationInputs/acceptedExtensionDefinitions，hint 不可信；`open_project_manifest(path,cache_root)` 精确核验恢复，缺扩展仅消费者诊断；`accept_definition_catalog(path,cache_root)`、`remove_definition_catalog(id)` 改会话接纳和 dirty，不隐式重启恢复。
- [x] 模板 can-empty-v1/can-signals-v1/standard-ecu-v1：真实文件预览/确认到新空目录，完整原创源集合/成员，失败不留下假工程，无绝对个人路径/官方档案；提供明确 `preview_project_creation`/`create_project_previewed` 及 `preview_save_as_project`/`save_as_project_previewed` API，具体请求/返回类型向协调者登记。
- [x] 标准模板 7.6 applicationInputs 默认空，原纯源码参考应用仍可用；不提前冒称 7.10 v2 或 live 初始化。列出允许应用槽真实 descriptor 接入需要，交接实现后由协调者接线。
- [x] 更新真实成功/拒绝行为测试与不变字节断言；现有 oracle/legacy 测试仍以明确资源模式执行，不弱化合法断言、不用源码文字测试。

## Design Notes
协调者独占 core/model/导出、integration/mod.rs 模式分派、IPC/Session 和测试注册。改变 exported API 先查实际 references（Rust LSP 当前不可用，可用 Codegraph/精确符号搜索）。缓存需低拷贝、不另建持久配置；重任务用后台调用。不得启动子代理或自行运行 build/lint/tests/formatter/可见 UI/安装；重检查由协调者排队。文件冲突先通过 Orca question 沟通，禁止覆盖其他 worker。

## Implementation Notes

- 默认 `Workspace::open(Vec<PathBuf>)` / `create(&Path,&str)` 使用真实 builtin catalog；`open_legacy` / `create_legacy` 明确持有官方 schema。`set_legacy_validation_schema(PathBuf)->Result<(),String>` 不允许替换 builtin 规则；旧 support/bin oracle 调用迁为显式 legacy，原断言保留。
- 新增 `arxml/projection.rs` 保存源、对象、字段、引用、合法候选与实例定义选项的 `Arc<SourceSnapshot>`；相同显示路径不充当 ID，源归属和重复实例 occurrence 独立，改名保留实际对象/字段身份。`source_text(&str)` 只读取该 epoch 已拥有原文；投影不为筛选再读磁盘/重扫源。
- `prepare_change(&ChangeSet)` / `apply_change(&ChangeSet,&str)` 使用完整 private prospective Workspace；规范化 changeId 顺序，创建依赖拓扑排序、原值比较、精确原文补丁、入站闭包、实际 schema/definition witness 检查，再一次发布。prepare 不发布新 ID；新实例预览使用批次键/路径，成功 outcome 才返回 `createdIds` / `createdFields`。删除值条目不让后续重复条目继承被删 ID。
- 唯一 AppState capability token 由协调者验证两个请求 echo；core 的 `input_fingerprint()` 是原文/revision/rule/catalog 的内部摘要，不被误当为第二个 Session token。`changeRevision` 同时绑定请求批次 echo、实际原文与成员、规则/定义、完整影响及 prospective 保存身份。
- `PreparedSave` 的现有 staging/backup/rollback 扩为源与 manifest 同一写入事务；确认摘要绑定完整规则/定义/成员，未改文件不写。builtin 保存只关闭实际 source-safety/schema 错误，不使用 target-generation 错误作为总门；保留精确未变 definition witness，新增/改变反例拒绝。发布失败拒绝覆盖外部文件，回滚先隔离当前文件再核验，恢复发布拒绝覆盖；冲突时保留外部字节及可定位恢复资料。全部源与 manifest 发布成功后，即使备份清理失败仍同步 saved 基线并保持 dirty=false，返回清理错误、保留备份并阻断后续保存。暂存清理失败时保留的 stage 是已发布源的硬链接别名，修改 stage 同样会修改源；错误明确标出链接暂存及备份恢复路径，后续保存同时拒绝外部改动与未恢复资料。
- `arxml/project.rs` 导出 `ProjectManifest`、`ProjectInput`、`ApplicationInput`、`ProjectCreationPreview`、`ProjectFilePreview`。创建/另存预览字段为 `revision/templateId/directory/name/files[{path,contents}]/acceptedExtensionDefinitions`；确认重新派生可信模板/当前源字节并完整比较，不信任回传 payload。公开 `preview_project_creation(&Path,&str,&str)`、`create_project_previewed(&ProjectCreationPreview)`、`preview_save_as_project(&self,&Path,&str)`、`save_as_project_previewed(&self,&ProjectCreationPreview)`，均返回 `Result<...,String>`。
- 原创 CAN 模板复用产品 renderer；标准模板嵌入既有七份产品原创输入，以 XML 范围修改 ECU 名及真实引用，不附官方包。私有 sibling staging 校验后原子发布到新/空目录；复制另存不改原源。模板的 `applicationInputs` / `acceptedExtensionDefinitions` 默认空。
- `open_project_manifest(&Path,&Path)` 从安全相对成员重新构建，精确恢复已持久化 catalog 身份；缺失/损坏 cache 不默选版本，未知消费者只读，builtin 消费者不关闭。`accept_definition_catalog(&mut self,&Path,&Path)` / `remove_definition_catalog(&mut self,&str)` 只改变会话接纳及 dirty；`project_manifest()` / `definition_catalog()` 暴露真实只读接缝。扩展 view 的 `source` 未记录缓存路径时为 `None`，不虚构来源。
- 原标准集成方法改为不收 MOD 的 native catalog 路径；显式 `*_legacy` 保留原 v1/oracle。7.10 的真实应用槽 descriptor 应由 `ValidatedIntegrationPlan` / 实际 `component_contract_files` 产生：固定 producerSlot 为 `epic4-single-application-v1`，componentPath/sourcePaths/generatedHeaders/entrySymbols 由该计划与实际文件集合取得，不能从模板名或源 worker 的硬编码符号推导。7.6 不创建 live 应用或提前升级 v2。
- native inventory 失败时安全打开仍保留 owned 源；可信期望 identity 仅用于软件故障诊断，依赖检查为 `not_run` 并记录错误，编辑/保存能力关闭，没有官方 fallback 或伪通过。
- 已按真实共享 DTO 填充 `referenceOptions` / `recursiveDefinitions` 与实际 `projectPath`；既有目标包含实际 objectId/path/DEST 与可空的 ECUC definitionId，prospective created 目标只携真实 definitionId/DEST，不伪造对象 ID。显式非 4.10.0 ECUC edition 的模块与后代只读；实际 source read / snapshot build / full scan 使用 feature-guarded verification hooks。
- `Workspace::generation_snapshot()` 返回已保存且复核外部字节的 `GenerationSnapshot`：真实 manifest/raw manifest bytes/可空 manifestPath、projectRoot，以及 logicalPath/diskPath/bytes 的 inputs/applications；直接 ARXML 使用源名称唯一性核对后的派生成员表示。`open_project_with_catalog(&Path,&DefinitionCatalog)` 只保留调用方显式提供且与 manifest 完全相同的接纳 catalog，未声明的 catalog 本地剔除，没有全局 cache fallback。
- 7.10 源接点已根据实际 generator provider 接线：`preview_application_initialization(&self)->Result<ApplicationInitializationPreview,String>` 内部派生保存 native plan、真实 slot descriptor 与 `application_seed_files`。preview 字段为 revision/slot/files[{path,contents}]/manifestBefore/manifestAfter；`initialize_application_previewed(&mut self,&ApplicationInitializationPreview)->Result<ApplicationInitializationOutcome,String>` 完整重新派生核对，拒绝已有 live 路径，同一 reviewed transaction 创建用户源与 manifest membership。Outcome 为 projection/warnings/retainedRecoveryFiles；post-commit 清理故障不谎报回滚或删除已被 manifest 接纳的 live 源。只复制成员变化，不复制未变 ARXML；发布前预计算投影/摘要，source state 与 revision 只发布一次。此接点不改变 7.6 模板默认空 applicationInputs。
- `arxml/standard_template.rs` 是产品原创标准模板的显式源码 policy：补全真实必需字段、Mcu clock、EcuC Hardware/VirtualCore/CoreId=0 与被 Dcm 真实引用的 Dem client/memory/cycle/config/不可用事件及 monitor-internal debounce 闭包；不读取 catalog default/minimum/首个 enum 作为隐式值，不修改 legacy fixture。虚拟 target 的固定 metadata policy 保持禁用额外服务，完整源仍由 native schema/definition 校验，不宣称新增 Dem/Mcu 运行时。


## Verification
报告真实变更、公开方法/请求返回类型、测试函数、拒绝路径与未运行项。模块完成不等于各纵向 Story 完成。

- Worker 遵守集中核查约束，未运行 build/lint/tests/formatter、可见 UI 或安装；已向协调者登记 `core/tests/support/epic7_workspace.rs` 的真实 API/文件场景并请求集中编译、针对性运行及源码烟测。
- 测试函数：`builtin_source_projection_owns_bytes_and_changes_epoch_on_reopen`、`builtin_batch_is_atomic_stale_safe_and_preserves_untouched_bytes`、`builtin_rename_updates_real_incoming_edges_without_rewriting_unrelated_source`、`builtin_create_requires_complete_batch_and_deleted_ids_are_never_reused`、`builtin_remove_rejects_incoming_references_and_batch_reference_can_target_created_instance`、`builtin_templates_are_previewed_portable_and_confirmation_rejects_tampering`、`builtin_external_changes_and_manifest_escape_refuse_without_losing_owned_bytes`、`builtin_schema_failure_blocks_save_but_unknown_generation_does_not`、`builtin_workspace_types_defaults_and_exact_catalog_acceptance_roundtrip`、`builtin_save_as_preserves_original_sources_and_rejects_name_collisions`。
- 中央 bg70 实际运行 `cargo test --locked --manifest-path core/Cargo.toml --test end_to_end epic7_ --features verification-metrics -- --nocapture`：54 个场景中 51 个通过，source-owned `epic7_workspace` 全部 15 个通过；其余三个当次失败分别属于规范 oracle 和目标 consumer closure，不能把该次命令记为全绿。源测试使用真实 29-bit CAN ID 上界 536870911；2048 是合法 ECUC 配置而非通用保存拒绝边界。edition 测试明确写入不支持的 edition，再核对 owning module/后代只读、其他模块仍可编辑及原字节保持。
- 中央 bg77 返回固定 MOD golden、全部 20 个 definition 场景及 7 个 rule 场景通过。此前 bg76 暴露标准模板缺少 EcucHardware 必需闭包，源切片按真实 metadata 增加 Hardware/VirtualCore/CoreId=0 三条显式原创 policy 后重新冻结；没有修改 legacy fixture 或放宽规则。
- 中央 bg78 在该最终 policy 上运行真实 API 烟测，EXIT 0，20.82 秒：原创 CAN 单源与标准 ECU 七源创建、reviewed create-only live 初始化、确认 sealed 生成、独立 v2 重建及源/应用字节相等均成立，warnings 为空；实际 catalog 接受 SC2 与 float 500.0，不依赖外部档案或编译器，临时烟测源码已由协调者移除。
- 新增场景函数：`builtin_application_initialization_is_reviewed_create_only_and_owns_real_live_bytes`、`builtin_application_initialization_refuses_a_late_user_path_without_touching_membership`、`builtin_unsupported_ecuc_edition_keeps_owned_bytes_and_only_that_module_readonly`；另有跨文件批次与重复路径身份场景，均包含在中央 bg70 的 15 个 source-owned 通过项。
- 中央 bg66 的 `cargo check --locked --manifest-path src-tauri/Cargo.toml --tests --features native-webdriver` EXIT 0、无警告，证明源接点已由真实桌面后端编译接入；不等同于窗口/IPC 场景通过。最终全套回归、源码质量门及 7.11 隔离桌面场景仍由协调者汇总，本源记录不宣称这些尚未返回的检查通过，也不修改纵向 Story/Epic 状态。
- 源切片的上述实现与验收已完成并保持冻结，协调者已收到 merge-ready；本工件暂保留 `in-progress`，等待协调者的最终全套/质量门与纵向状态收口。源 worker 清除了其创建的三个本地和三个共享临时 staging 文件；没有提交、推送或启动可见桌面窗口。

## 2026-10-06 开发收尾

用户明确要求本轮只确保开发与单元／集成测试完整，不执行真机或原生发行场景，不扩建隔离／验收设施。本规格的开发部分已随[中央实施规格](spec-epic-7-configurator.md)完成开发检查与独立复核；这里的 done 限定为该开发范围。原未完成的发行／GUI验收事实保留，后续交给CI，不写成已通过，也不自动恢复旧场景。
