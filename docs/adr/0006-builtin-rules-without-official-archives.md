# 0006. 内置规则与产品模块定义，日常使用不需官方档案

## 背景

早期实现要求使用者自行提供 R24-11 官方 XSD／MOD 档案才能打开、校验、保存和生成。官方档案不能随产品再分发，使普通用户在安装后无法工作；同时也不能把官方档案加密或转换后打包规避许可。

## 决策

1. 产品随软件交付**产品原创**的版本化校验代码与不可变模块定义（BuiltinRuleSet、DefinitionCatalog），不是官方 XSD／MOD 的原样、加密或自动转换副本。
2. `RuleSetIdentity={release,rulesVersion,sha256}`：`release` 为规范版次，`rulesVersion` 是独立的产品规则版号，`sha256` 对确定性打包 inventory 求摘要（含实现来源、显式覆盖条目与全部内置 metadata 摘要）。规则、覆盖或 metadata 改变必须发布新身份；构建与启动核对可信身份，用户设置或环境变量不能替换。
3. 内置规则只声明其实际覆盖范围：结构检查不称为“完整官方 XSD 通过”，未覆盖语义为 `unsupported`。
4. 新建、导入、编辑、保存、校验和源码生成不需要官方档案、编译器或网络；构建与运行另需目标工具链。
5. 第三方定义是显式可选扩展：按 `{catalogId,release,version,sha256}` 核验并经用户明确接纳后进入工程隔离 catalog（本机不可变缓存 `app_config_dir/definition-catalogs/<sha256>/`），只约束实际消费者，不覆盖内置定义，不自动联网或跟随 `schemaLocation`。
6. 官方档案只用于研发核对：作为测试中的独立对照（见 ADR 0012）。

## 后果

- 普通用户无需准备官方资料即可在声明覆盖内完成配置与源码生成。
- 内置规则的正确性须以合法持有的官方 XSD／MOD 作为开发 oracle 对照正例与定向反例证明；ECUC metadata 的数值范围通过不等于配置语义合法（如 controller／HOH 连续编号）。
- 校验、缓存、预览和交接按完整规则／定义／输入身份隔离；规则或扩展变化使相关确认与下游结果失效。

## 状态

已接受（2026-10-03）。外部档案模式的删除见 ADR 0012。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md`（AD-2、AD-4、AD-10、Catalog 身份与扩展接纳、Validation scopes）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-7/reviews/`、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture.md`（下一产品增量的架构规划边界）。
