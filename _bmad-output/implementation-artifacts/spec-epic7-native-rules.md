---
title: 'Epic 7 原生结构规则与可信库存实现'
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
完成 7.1 的产品原创原生结构、顺序、基数、类型校验与确定性规则库存，为整个工作台替换默认官方 XSD 依赖。不宣称完整 XSD 认证，不删有效旧拒绝，不复制或转换官方 XML。
## Boundaries & Constraints
原始输入在 50 MiB、UTF-8、namespace/R24-11、良构、无 DTD/实体及安全路径边界内解释。覆盖所有现有 CAN、诊断/DTC、标准七文件剖面实际使用的结构；相关未知内容准确 unsupported，合法未知原字节可保留。规则安装错误不降级通过。官方资料仅本地 oracle。不提交或推送。
</frozen-after-approval>

## Code Map
- `core/src/schema.rs`：现有 pinned XSD/libxml 验证，必须保留为开发 oracle/严格旧包兼容，不伪造其原 hash。
- `core/build.rs`、`core/src/resources.rs`：参考已有确定性编译库存/可信内容核验模式。
- `core/src/project_model.rs`：协调者已建立唯一 RuleSetIdentity、ValidationScope/Status、RuleCoverage、ScopeValidation、ConfigurationDiagnostic/Witness wire 类型。
- `core/src/arxml_render.rs`、`core/tests/fixtures/epic4/positive`：真实产品输入/输出语法，不只几个表单字段。
- 开发参考：`docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`。

## Tasks & Acceptance
- [ ] 独占修改 `core/build.rs`、`core/src/schema.rs`、新增 `core/src/rules.rs` 及必要的 `core/src/rules/` 子模块；新增行为测试 `core/tests/support/epic7_rules.rs`。不编辑其他 worker 或协调者文件。
- [ ] `rules::rule_set_identity() -> Result<RuleSetIdentity,String>` 核对产品真实可信 inventory/实现来源/元数据绑定，初始独立规则版号 `1.0.0`。所有库存内容 deterministic；构建及启动核验，错误拒绝依赖能力。
- [ ] `rules::validate_native(files: &[(&Path,&str)]) -> Result<ScopeValidation,String>` 实际执行 native schema 覆盖，输出 path/file/ruleId/witness 与已支持/未知条目，不联网。Workspace 拥有者将路径绑定 source/object/field IDs。
- [ ] `rules::coverage() -> Result<Vec<RuleCoverage>,String>` 用与实际执行相同库存返回声明，不另写互不相干的能力清单。
- [ ] 元数据来自定义 worker 的产品原创 `core/src/definitions.rs`/子模块：inventory 纳入完整定义来源内容和必要规则实现哈希，不读取/打包官方档案；以安全 deterministic 构建逻辑支持该文件稍后出现的并行工作。
- [ ] 保留现有 `schema::validate_files` 真正官方 oracle 与 legacy 身份行为；添加 native 调用，不用空路径/环境 fallback 隐藏模式。
- [ ] 起草具体正反例 oracle 测试，覆盖结构/顺序/基数/类型、不支持、安全拒绝与库存损坏；保持先前有意义的拒绝，测试须捕获消费者可见行为，不测试源码字符串/转发。

**Acceptance:** Given 合法原创 CAN/标准输入且无官方档案，when 原生检查，then 支持范围执行真实规则；Given 已覆盖结构错误/损坏库存，when 使用，then 明确失败；Given 合法未知保留语义，then unsupported 不伪通过。

## Design Notes
协调者独占 `core/src/lib.rs`、`project_model.rs`、IPC/Session 和集成测试注册；源事务 worker 独占 Workspace。只实现上述规则/库存，不创建第二 Workspace。不得自行启动子代理、build/lint/tests/formatter/可见界面/安装工具；检查由协调者集中执行。遇到接口问题通过 Orca coordinator inbox 询问，不等待用户。本任务完成只代表模块交付，不代表纵向 Story 验收。

## Implementation Notes

## Verification
向协调者报告真实修改文件、必要测试函数/命令、覆盖与尚未执行项；不得声称未运行的检查通过。

## 2026-10-06 开发收尾

用户明确要求本轮只确保开发与单元／集成测试完整，不执行真机或原生发行场景，不扩建隔离／验收设施。本规格的开发部分已随[中央实施规格](spec-epic-7-configurator.md)完成开发检查与独立复核；这里的 done 限定为该开发范围。原未完成的发行／GUI验收事实保留，后续交给CI，不写成已通过，也不自动恢复旧场景。
