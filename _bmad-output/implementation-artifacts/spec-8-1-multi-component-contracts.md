---
title: '从多组件输入生成一致的应用契约'
type: feature
created: '2026-10-09'
status: ready-for-dev
route: dispatch
review_loop_iteration: 0
context:
  - AGENTS.md
  - CONTRIBUTING.md
  - .agents/skills/misra-c2012/SKILL.md
  - _bmad-output/specs/spec-multi-component-scheduling/application-contract.md
  - _bmad-output/specs/spec-multi-component-scheduling/acceptance.md
  - _bmad-output/specs/spec-multi-component-scheduling/compliance-references.md
  - _bmad-output/planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md
---

<frozen-after-approval reason="用户授权依次规划、实施、验证；无无法核定的实质产品问题">

## Intent

现有集成计划只接受完整 EchoApplication，不能表达纯本地生产者、消费者或服务组件。本故事从正常原生规则入口验证平坦多组件配置，并生成准确、独立的组件接口，为下一故事真实通信及OS调度提供同一个不可变计划。

## Boundaries & Constraints

严格按关联的首批配置、架构和R24-11官方依据实施。新profile为singlecore-multi-swc-v1；旧profile保持精确行为。输入ARXML权威，先解析并验证全部关系，再构造私有计划；下游不能从XML重新猜测。一个类型一个实例，独立保留type、instance、port、element、operation、network/channel身份。纯local组件是必要支持形状，不能要求每个组件具有CAN/DID。无possibleErrors的server为void，client为Std_ReturnType；uint32 IN/OUT/INOUT及uint8[4] OUT的声明完整。组件标准API可通过宏映射唯一内部符号，文件名、guard和C符号均查碰撞。不得实施R11、重复Epic7收尾、更新无关历史状态或引入第二套模型。修改生成C前读取MISRA指导。当前只完成契约与生成前验证，8.2实现实际运行，8.3以后才扩展源编辑及交接。

## I/O & Edge-Case Matrix

| 场景 | 输入 | 预期 |
| --- | --- | --- |
| 多组件 | 原创多文件Ingress/Process/Observe，local与network端点及同步服务 | 正常入口得到新profile不可变计划、三个组件header、准确类型/实例/连接/映射/调度描述 |
| 纯local | 不具有专属CAN/DID的生产、消费和服务组件 | 正确生成自己的API，不套用旧Echo完整形状 |
| 非法关系 | dangling、方向错误、归属错误、重复生产者、network/local混接、重复实例、重复event或位置 | 生成前定位到真实对象并拒绝，原输入与旧输出保持 |
| 类型 | 接口不同、等宽不同类型、缺失type mapping、错误参数方向 | 明确拒绝，不能自动转换或猜测 |
| 命名 | 大小写文件冲突、normalized C符号冲突、保留类型名 | 拒绝而不产生覆盖或多生产者 |
| 旧profile | 原Echo及原有正反fixture | 原公开计划与header回归保持 |

</frozen-after-approval>

## Code Map

- `core/src/integration/{mod,graph,component,communication,schedule,plan,contracts}.rs`：当前单组件inspect链；复用Graph引用与定位、官方/原生双入口、私有ValidatedIntegrationPlan，保留旧分派。
- `core/src/integration/{configuration,native_configuration,catalog,diagnostic,routing,ecu}.rs`：下游依赖单component形状；新计划应为8.2消费准备有类型关系，不能伪造旧component充当所有组件。
- `core/src/rules.rs`、`core/src/rules/grammar.rs`及原生定义：正常原生结构/语义入口；只有所需支持项扩展，规则身份按现有流程更新。
- `core/tests/fixtures/epic4/positive`、`core/tests/{integration_plan,component_contracts,native_rules}.rs`：复用正常fixture框架；新原创多文件fixture与独立负向断言，实际测试文件名以仓库为准。
- `runtime/ecu/templates`与资源摘要：只有本故事实际受影响生成header来源才修改，禁止用摘要追认ABI。

## Tasks & Acceptance

**Execution:**

- [ ] `core/src/integration`：增加真实多组件有类型契约与严格profile分派，闭合所有当前组件/连接/类型/ECU/runnable/event/task输入，保持旧消费者兼容。
- [ ] `core/src/integration/contracts.rs`：从同一计划生成每组件header、共享类型及唯一实现声明，记录准确符号生产者/消费者。
- [ ] `core/src/rules`及相关定义/消息：扩展实际必要正常校验与定位，遵循现有i18n及资产身份入口。
- [ ] `core/tests`及原创fixture：覆盖矩阵每行，实际C编译检查typed声明和void服务ABI，回归旧profile。
- [ ] 本规格：记录实际命令、结果和未验证边界，实际生成受影响header/profile执行c-check；不得把尚未运行8.2写为通过。

**Acceptance Criteria:**

- Given 首批三组件多文件输入，when 正常内置规则建立计划并生成契约，then AC-1/5/6相关身份、类型、连接及签名全部闭合，纯local组件可用。
- Given 任一非法关系或unsupported配置，when 计划构建，then 输出前明确拒绝并定位真实对象，不改变输入或已有输出。
- Given 原有单组件输入，when 全部相关旧测试运行，then 历史公开行为保持。

## Implementation Notes

规划已完成：用户授权继续实施，无未解决产品问题；仅本地必要可逆修改。阶段提交由主代理负责。

## Spec Change Log

## Review Triage Log

## Verification

- `cargo test --manifest-path core/Cargo.toml`：新增契约和旧回归实际运行通过。
- 实际生成header的C99编译/链接类型检查及正常c-check：按测试说明准备固定工具，不以字符串快照代替签名验证。
- 记录精确运行范围；下一故事的真实OS运行未实施时明确保留。
