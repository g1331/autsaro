---
title: 'Epic 7 自有完整模块定义与隔离扩展实现'
type: feature
created: '2026-10-03'
status: done
route: dispatch
baseline_commit: '52367974cab5f73ce5a336936d5c0f352290dae1'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/spec-epic-7-configurator.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-7-context.md'
  - '{project-root}/_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md'
---

<frozen-after-approval reason="derived from approved Epic 7 intent">
## Intent
完成 7.2 全部已声明 CAN/标准 ECU 配置定义的产品原创元数据、类型/默认/基数/引用校验，支持显式第三方 catalog 与摘要键完整缓存。不只移植现有表单字段，不把定义能力视为生成支持，不复制/转换官方 XML。
## Boundaries & Constraints
定义覆盖 Can/CanIf/CanTp/Com/Dcm/EcuC/Os/PduR/Rte 以及 CAN 的 Mcu/Dem/NvM。保留 unknown/variation/expression/instance-reference 的真实只读原因；kind/lexeme 不经 JS number，默认不自动落盘。第三方不得覆盖内置、错版次/摘要/冲突/危险路径拒绝；缓存≠工程接纳。官方资料仅本地 oracle。不得提交或推送。
</frozen-after-approval>

## Code Map
- `core/src/integration/configuration.rs`：9 模块目标 allowlist、已有跨模块规则；不能放宽。
- `core/src/integration/graph.rs`：当前 Graph::new(source,definitions XML) 及 path/parent/reference/DEST 解析；复用而非第二持久图。
- `core/src/integration/catalog.rs`：RuntimeCatalog 是 ABI/source 库，不是定义目录。
- `core/src/arxml_render.rs`、`core/tests/support/arxml_package.rs`：真实 CAN/Dcm/Dem/NvM 输出与现有官方 MOD oracle。
- `core/src/project_model.rs`：唯一 DefinitionDescriptor、TypedValue/ValueKind、ScopeValidation/diagnostic/Witness、ExtensionDefinitionIdentity；直接使用，不自创 wire。
- 本地参考 `docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`。

## Tasks & Acceptance
- [x] 独占新增 `core/src/definitions.rs`/`core/src/definitions/`；仅可修改 `core/src/integration/graph.rs` 接入 native metadata；新增测试 `core/tests/support/epic7_definitions.rs`。不编辑 Workspace、integration/mod.rs、模型、Session、生成器或其他测试文件。
- [x] `DefinitionCatalog::builtin() -> Result<Self,String>`；`get(&str)->Option<&DefinitionDescriptor>`；`children(&str)->Vec<&DefinitionDescriptor>`；`definitions()` 返回内置+接纳定义迭代器。完整参数/container/module 类型、范围、基数、枚举、单位、默认来源及引用允许目标；元数据独立原创实现。
- [x] `validate_documents(&self, files:&[(&Path,&str)]) -> Result<ScopeValidation,String>` 实际 definition 校验：参数/引用 def kind、必需/重复条目、DESTINATION 类型、值域/默认来源、结构、旧目标跨约束。诊断 file/path 供 Workspace 绑定稳定身份；未知覆盖与实际错误分开。
- [x] `fingerprint(&self, identity:&RuleSetIdentity)->String` 按实际 metadata 与稳定排序的已接纳身份派生，不绑定路径或其他工程；`accepted_extensions()->Vec<ExtensionDefinitionIdentity>`；`required_extensions(definition_ids:&[String])->Vec<ExtensionDefinitionIdentity>` 只返回真实消费者闭包。
- [x] `accept_extension(&mut self,catalog_path:&Path,cache_root:&Path)->Result<ExtensionDefinitionIdentity,String>`：完整 catalog.json v1 原字节/两层摘要、完整唯一成员、无链接/越界/额外 payload；安全 native 定义解析后 owned staging 发布缓存，复用仍核验。失败保持旧目录；不执行代码。
- [x] `remove_extension(&mut self,catalog_id:&str)->Result<(),String>` 不删除源/共享缓存；`restore_extensions(&mut self,identities:&[ExtensionDefinitionIdentity],cache_root:&Path)->Result<Vec<ConfigurationDiagnostic>,String>` 从精确核验缓存恢复，缺失保持需求身份及消费者诊断，不全局锁内置；直接会话不隐式恢复。
- [x] `Graph::from_catalog(sources:&[InputSource],catalog:&DefinitionCatalog)` 与既有 source graph 共用构建代码，用元数据建立 external definition-kind 映射，不把元数据重新序列化为官方 XML。原 Graph::new 保留严格 legacy/oracle 用途。
- [x] 编写多模块与整数精度、默认/显式、引用目标、扩展冲突/错版次/摘要/损坏的实际行为测试和固定官方 oracle 对照，不只测常量/非空。

## Design Notes
协调者独占模型/core 导出/IPC、integration/mod.rs 的模式切换及测试注册；源事务 worker 使用本 API。必要接口细节向协调者发 Orca question。不得启动子代理或自行运行 build/lint/tests/formatter/可见 UI/安装；集中核查后再声明能力。

## Implementation Notes

- 自有内置目录覆盖当前 CAN／标准 ECU 产品剖面已声明的 Can、CanIf、CanTp、Com、Dcm、EcuC、Os、PduR、Rte、Mcu、Dem、NvM 定义及父容器链；不是完整官方 MOD、全模块生成器或官方符合性认证。初始七类值以 kind／lexeme 保存，无符号 64 位整数边界不经 JS number。默认及来源只用于元数据展示，不替代必填显式条目。
- `builtin.rs` 为手工原创描述；官方档案只在固定摘要 oracle 测试中读取。实际全目录对照修正了 Dcm、Dem、EcuC 等基数／枚举及缺失默认，未知 `PduTriggeredByRef` 未冒充受支持定义。
- native／legacy 共享同一原文派生 Graph 构建器。`from_catalog_target_scope` 延后全局引用审计，由实际消费者闭包调用 `reference_diagnostics_for`；旧 `Graph::new` 仍严格。补充跨模块定义规则只用于正向识别的目标结构，不把 SC1、固定主机策略或其他 target Unsupported 提升为通用定义错误。
- XML simple text 合并所有直接 text 节点，避免 comment／CDATA 截断 VALUE、SHORT-NAME、引用、边界及默认；正常单段保持借用。问题保留原 entry-range／完整对象路径，由源事务层绑定稳定身份，不创建第二持久图、不重写原始 XML。
- 扩展导入隔离接纳身份与摘要缓存，拒绝内置命名空间覆盖、错版次、摘要／成员／路径／链接问题；损坏缓存恢复保留精确需求身份。`verification-metrics` 只在真实有界原文件读取前计数，包含 catalog.json 与 ARXML／缓存核验文件；无 ZIP 解压或缓存查询伪计数。


## Verification
报告真实修改、目录实际覆盖、测试函数、必要命令及未运行项；保留有意义旧测试，不声称完整官方符合性。

- 协调者集中核查最终冻结产品及新增 enclosing-document 测试，bg86 实际 exit 0：57 passed、0 failed、0 ignored、117 filtered；20 个 Windows 适用定义测试、8 个规则测试、15 个 workspace 测试、14 个 delivery 测试，无编译警告。复现命令：`cargo test --locked --manifest-path core/Cargo.toml --features verification-metrics epic7_`。Unix symlink 专项未在 Windows 执行。
- 全部原创描述与固定 R24-11 MOD member SHA-256 `55ad8924b9aaccf600effc06e779c2d9c3300d182749e1bc7c079a55e7a2c68f` 比较 kind、min／max、lower／upper、default、完整枚举，最终通过；没有将官方 XML 转换为运行时源码。
- `standard_documents_accept_definition_valid_sc2_and_float_baud_without_target_policy_leak` 使用真实 `standard-ecu-v1` 七份原始 ARXML，分别修改实际 SC2 枚举及 `500.0` 浮点 baud 后验证完整文档：Definition Passed；不宣称对应有限主机生成剖面支持 SC2。其他已执行行为包括整数精度／边界、必填／重复／默认、引用目标／DEST、多源及大小端重叠、扩展完整性／错版次／冲突／损坏恢复、split literal 与源 witness 范围。
- 协调者实际 API smoke bg78 exit 0（20.82 秒）：原创 CAN 单源、标准 ECU 七源、显式真实 application 初始化、preview／confirm 封存生成、两个独立 v2 workspace 重建；source／application 字节相同，不配置外部 MOD／工具／编译器，warnings 为空。临时 smoke 源已由协调者移除。
- Worker 未运行本地 build／lint／tests／formatter、未安装工具、未启动可见界面、未提交或推送。此定义切片已完成上述验证；产品状态保留 `in-progress`，集中 quality／完整门禁／隔离 native 运行与非实现者复核仍由协调者完成，不将子任务结果写成整项 Epic 完成。

## 2026-10-06 开发收尾

用户明确要求本轮只确保开发与单元／集成测试完整，不执行真机或原生发行场景，不扩建隔离／验收设施。本规格的开发部分已随[中央实施规格](spec-epic-7-configurator.md)完成开发检查与独立复核；这里的 done 限定为该开发范围。原未完成的发行／GUI验收事实保留，后续交给CI，不写成已通过，也不自动恢复旧场景。
