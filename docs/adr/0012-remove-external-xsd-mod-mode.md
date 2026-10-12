# 0012. 删除外部 XSD／MOD 校验模式，XSD 只作测试对照

## 背景

内置规则（ADR 0006）上线后，设置中仍保留“配置官方 XSD／MOD 档案”的模式（`Workspace::uses_legacy_validation`）：一旦配置，创建、打开、编辑、计划、保存和生成都切换到九组 `*_legacy` 平行实现。两套校验并存，结果可能互相矛盾，且每次修改都要维护两份。

## 决策

1. 删除外部档案校验模式：设置项、`schema_zip` 状态与全部 `*_legacy` 链路。
2. 保留 `schema.rs` 的 XSD 校验，**只**用于测试与 acceptance 中的官方对照基准，检验内置规则是否符合 R24-11 schema；`official-oracles` 测试仍可运行。
3. 旧设置文件中的档案字段不导致启动失败。

## 后果

- 运行时只有一套校验语义；普通使用者的界面中不再出现官方资源设置。
- 内置规则与官方 Schema 的一致性证据完全来自开发期对照测试，因此对照测试必须保持可运行并覆盖正例与定向反例。
- 依赖外部档案的旧交接格式（v1）随 ADR 0011 一并拒绝。

## 状态

已接受（2026-10-11），已实施（#33）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（决定 4）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（B3）。
