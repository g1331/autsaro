---
title: '从多组件输入生成一致的应用契约'
type: feature
created: '2026-10-09'
status: done
baseline_commit: '74128a430616695a2d0b09732198a0ede1320ef1'
route: dispatch
review_loop_iteration: 2
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

- [x] `core/src/integration`：增加真实多组件有类型契约与严格profile分派，闭合所有当前组件/连接/类型/ECU/runnable/event/task输入，保持旧消费者兼容。
- [x] `core/src/integration/contracts.rs`：从同一计划生成每组件header、共享类型及唯一实现声明，记录准确符号生产者/消费者。
- [x] `core/src/rules`及相关定义/消息：扩展实际必要正常校验与定位，遵循现有i18n及资产身份入口。
- [x] `core/tests`及原创fixture：覆盖矩阵每行，实际C编译检查typed声明和void服务ABI，回归旧profile。
- [x] 本规格：记录实际命令、结果和未验证边界；本故事实际生成 header 运行独立 C99 编译/链接及固定 Cppcheck/MISRA addon 分析，旧可构建 standard-ecu profile 运行正常 c-check。完整新 multi ECU 的正常 c-check 为 8.2 实际工程生成后的必需退出条件，不得用 contract-only 目录或补充分析冒充通过。

**Acceptance Criteria:**

- Given 首批三组件多文件输入，when 正常内置规则建立计划并生成契约，then AC-1/5/6相关身份、类型、连接及签名全部闭合，纯local组件可用。
- Given 任一非法关系或unsupported配置，when 计划构建，then 输出前明确拒绝并定位真实对象，不改变输入或已有输出。
- Given 原有单组件输入，when 全部相关旧测试运行，then 历史公开行为保持。

## Implementation Notes

实现采用同一个私有 `ValidatedIntegrationPlan`。新 `multi` 字段保存类型、实例、端点、连接、真实访问/call point、完整调用图、组件 header 及符号来源；旧 `component`/`diagnostic` 仅在旧 profile 序列化为原字段/值，新 profile 不伪造旧 Echo 契约。现有 ECU 生成、编辑与交接消费者明确拒绝尚未支持的新 profile，8.2–8.5 不因此被标记完成。阶段提交由主代理负责。

原创 fixture 包含 Ingress、Process、Observe 及已有 Dcm 服务客户端实例；Process/Observe 没有专属 CAN/DID。接收方独立初值、完整类型 mapping、所有同步 call point、实际 runnable 调用环、真实 ECU/RTE 实例与任务位置均由输入闭合；冲突的 unsigned base encoding 按 R24-11 Software Component Template constr_10383（NONE 表示 unsigned integer）明确拒绝，仅收窄新 profile，旧 profile 保持原行为。

当前计划复用旧 BSW 调度/目录检查，仍包含 `Com_AdvanceTime`/旧主动 `Com_TriggerTransmit` 形状。这不是新 profile 最终标准 BSW 契约：8.2 在消费该计划生成真实 ECU 前，必须将 COM 主函数对齐 `Com_MainFunctionRx`/`Com_MainFunctionTx`，将标准 `Com_TriggerTransmit` 对齐 pull callback 签名/语义，并同步输入、目录、计划、生成与公开接口验证；不得通过伪造 ABI 目录身份掩盖差距。网络 deadline monitoring 的可支持配置范围由 8.2 决策及官方闭包核定，本故事未实施或验证网络 freshness runtime。


### 审查后必须闭合的实施契约

- 正常 DefinitionCatalog::validate_documents 与 build_plan_native 对同一 multi 输入使用同一完整关系验证，包括配置、调度、诊断、信号路由和 handle 身份；所有 multi diagnostics 必须传播到正常工作区，不沿用仅旧 profile 的错误白名单。测试必须通过正常定义入口断言成功与具体错误，不只测试 plan builder。
- 信号路由不依赖可选 DID。CanIf handle 在每个方向内唯一，诊断与应用信号共同检查；不同方向同 handle 可合法存在。
- DCM 无事件 bridge 仅接受唯一真实 Service SWC client，其实际接口明确 IS-SERVICE=true、SERVICE-KIND=DIAGNOSTIC-COMMUNICATION-MANAGER，并通过完整 instance/port/operation connector 关联 DID server。DCM required R-Port 和接口均使用 `DataServices_{DcmDspData.shortName}`（R24-11 DCM ECUC_Dcm_00713 的 USE_DATA_SYNCH_CLIENT_SERVER 定义及 SWCT 同步数据服务映射）；应用 provider 端口名称可独立变化，只通过完整 connector 确定，名称不替代 requester 的完整身份或唯一性。普通 Application client 可共享 server，不得被选为 Dcm bridge；任意其他无事件 Service runnable 拒绝。保留改名传播。
- 网络接收的首批组合严格 HANDLE-NEVER-RECEIVED=true、HANDLE-TIMEOUT-TYPE=NONE；本地保持 false/0/NONE。aliveTimeout/COM DM 的 8.2 范围决定仍独立待核定，本故事不增加 runtime freshness 状态。
- 首批执行约束明确 minimumStartInterval 缺省或零，非零拒绝；exclusive area 声明/访问不支持则拒绝，不扩展 SchM 锁机制。使用官方合法且引用完整的输入证明拒绝；正常 grammar 先拒绝尚不支持形状也算明确拒绝，不引入虚构合法标签。
- 独立 C consuming TUs 编译/链接并实际运行所有已交付 API 形状：各组件 Rte_Read/Rte_Write 宏与不同实例唯一实现、scalar client/server、byte-array client/server、Dcm bridge typed 声明。复用正常工具链与受控进程入口，支持 Linux/Windows；本地仅声明实际运行平台。CONTRIBUTING 明确默认 Cargo 测试不依赖受控工具，实际 C consuming test 应由已有 native-tests feature 启用，显式执行此 feature 的 targeted test，不给默认测试增加 compiler/Python 环境依赖。

KEEP：保留已通过的四组件原创官方 XSD 输入、完整路径身份、one-type-one-instance、receiver 独立初值、void server ABI、所有 call point/递归拒绝、unsigned NONE 约束、命名碰撞及旧 profile JSON/行为。可从 `/tmp/autsaro-r6-story81-keep-code.patch` 取回已验证实现作为重推导素材，必须修正以上缺陷后才交付；不得直接恢复已知错误当成完成。保留其他工作区内容，UI 暂仅安全只读展示，后续编辑与完整 ECU 运行不在本故事伪称支持。

- C/S 关系双向闭合：每个 OperationInvokedEvent 必须唯一关联本组件 provider port/interface operation，runnable/mapping 精确匹配；每个 client/server COM-SPEC 必须唯一属于对应接口的 operation 集合，拒绝 foreign/重复项；首批 C/S interface 至少一个 operation。无参数 operation 合法并生成 (void)，须实际消费 ABI。同接口多 operation 均需自身 spec、call point 或 invoked event，覆盖成功及缺失项拒绝。
- SERVER-ARGUMENT-IMPL-POLICY 首批仅缺省或 USE-ARGUMENT-TYPE，其他 policy 明确拒绝。应用及 BSW 周期 event 映射必须显式 isMappedToTask=true/1 并有 task ref；false/缺省带 task ref 拒绝（RTE CONSTR08936/08938），server 保持 false/无 task。更新原创 fixture 必需参数与正常 definitions，旧 profile 保持历史契约。
- grammar 接受官方合法 TimingEvent OFFSET=0；新 profile 缺省/零接受，非零拒绝，以官方 XSD 和双正常入口覆盖。multi 按至少两个不同 Application SWC 类型分派，单一 Echo 类型重复实例保留旧精确诊断。
- contract.json 记录计划已有完整 rule_set_identity、required_extension_definitions，并保留摘要。实际 C 消费改名 component/port/operation 和唯一符号；周期/server concurrent=true 双入口拒绝；实际调用 editing/handoff fail-closed 证明输入和已有输出不变。

KEEP 第二轮：保留 v2 全部正常验证闭包、四组件官方输入、15/18项验证、正确 DCM DataServices required port 与独立改名 provider、全部 S/R/scalar/array/DCM C 消费和受控跨平台测试入口。恢复素材 `/tmp/autsaro-r6-story81-keep-v2-code.patch`，修正已知缺陷才交付。保留 UI/其他规格修改，不冒称实际 ECU 调度/源交付完成。

## Spec Change Log

- 2026-10-09：第三轮审查补齐固定数组 element/base CATEGORY、flat signal context 唯一性、SWC-to-ECU 非空/无重复成员和零网络 timeout；声明文档明确局部计划及运行时未实现，符号 consumer 覆盖所有 runnable。新增语法的 legacy 非零 OFFSET/minimumStartInterval 在正常校验与 native/official 生成一致拒绝，窄范围传播两类诊断，其他历史行为保持。全部存活 patch 已关闭并重跑最终验证，冻结意图/矩阵不变。

- 2026-10-09：第二轮 bad_spec 重推导（B21–24/B28/E21–24），闭合双向 C/S 集合、参数 policy、显式 task mapping、零 offset 和来源身份，避免忽略行为配置或无绑定事件；第二轮 KEEP 如上，同时关闭真实旧分派回归及实际 C 消费缺口，冻结意图/矩阵不变。

- 2026-10-09：重新核对 R24-11 DCM／SWCT，修正原创 fixture 的 DCM required R-Port 为 `DataServices_ApplicationValue`，按完整 requester／operation／connector 定位 provider，取消 provider 短名等于 DcmDspData 名称的错误限制；改名和共享 application client 分别验证。

- 2026-10-09：三路审查触发第 1 次 bad_spec 重推导，补齐正常校验入口与错误传播、可选诊断下信号路由、同方向 handle、真实 DCM bridge 身份、freshness/执行约束及全部 C API 消费。避免生成前才发现工作区漏报、首个 client 误绑定和默默忽略配置。KEEP 指令如上；冻结意图和矩阵保持。

- 2026-10-09：按实际生成边界明确最后一个执行任务：8.1 交付组件声明契约及生成前关系验证，实际 header 编译/链接和固定 addon 分析已执行；旧可构建 profile 正常 c-check 已执行。8.2 才生成完整 multi ECU，必须在该工程执行正常 c-check，保留整体要求而不为 contract-only 输出伪造 build/seal。冻结矩阵与验收条件未变。
- 2026-10-09：记录直接依赖的旧 COM BSW 调度/目录接口差距及 8.2 退出条件，避免把局部应用 header 与关系检查作为完整 AUTOSAR 模块符合性证据。

## Review Triage Log

| Finding | Verdict | Route | Evidence |
| --- | --- | --- | --- |
| B1 | high | bad_spec | graph.rs 提前返回，正常 validate_documents 不调用 schedule/configuration/diagnostic/routing，生成与工作区结论分叉。 |
| B2 | high | bad_spec | plan.rs 仅按 provider 端口短名选 operation，再 find 首个 requester；没有验证 requester 的 DCM service-kind 与唯一关系，可能选到应用 client。 |
| B3 | medium | bad_spec | multi.rs 仅 local 分支检查 HANDLE-NEVER-RECEIVED/HANDLE-TIMEOUT-TYPE；network 配置变化不进入契约且未拒绝。 |
| B4 | high | bad_spec | 任意无事件 SERVICE-SW-COMPONENT-TYPE runnable 被指定为 runtime/src/Dcm.c 消费，未证明其 DCM 接口与调用关系。 |
| B5 | high | bad_spec | 无 DcmDspData 时 routes=Vec::new，即使存在 network endpoints 也跳过 PduR 路由验证。 |
| B6 | medium | bad_spec | 支持检查没有闭合有行为含义的 minimum start interval/exclusive area；需拒绝非零间隔及不支持的 exclusive area 使用，不能默默丢弃。 |
| B7 | medium | patch | 实际 consuming TUs 未消费新增 S/R 宏、byte-array client 和 Dcm bridge；其映射错误可逃过现有编译验证。 |
| B8 | medium | patch | 新 C ABI 测试 cfg(unix) 且直接 Command(gcc)，绕过已有受控 compiler/ProcessOwner，Windows 不执行；复用 support/tooling 正常入口。 |
| B9 | low | reject | 部分通用 malformed 向量以 dangling 拒绝，但另有保持全部引用有效的 collisions 专项测试覆盖同一碰撞矩阵；无需增加重复测试分支。 |
| B10 | low | reject | in-review 是审查过程状态，sprint in-progress 直到退出验收；最终验证记录属于本规格交接更新，按技能拒绝以改 spec 为修复的 finding。 |
| E1 | high | bad_spec | 多组件 CAN 循环只检查 CAN ID 唯一及 handle 宽度，没有同方向 CanIf handle 唯一检查，不同 ID 可共享分发 handle。 |
| E2 | high | bad_spec | 无诊断配置时遗漏信号 PduR 路由，和 B5 同根因。 |
| E3 | high | bad_spec | 正常 graph 分支未调用 schedule，server task=true 或 position 重复在工作区校验漏报，和 B1 同根因。 |
| V1 | medium | patch | 接受 verification-gap 的已核实证据：新 S/R aliases 没有实际调用或函数指针消费，旧 fixture 走不同生成分支。 |
| V2 | high | bad_spec | definitions/validation.rs registered 白名单过滤 MULTIPLE_PRODUCERS/TYPE_CONFLICT/ENDPOINT_BINDING/SERVICE_RECURSION 等新错误，正常 projection 可丢失拒绝结果。 |
| V3 | high | bad_spec | 普通 graph 分支提前返回，未核对服务器 mapping false；和 B1/E3 同根因。 |

| B21 | medium | bad_spec | 官方 XSD OFFSET 合法，grammar 缺失导致零 offset 被拒绝。 |
| B22 | high | bad_spec | 只从 operation 找 event，没有反向闭合全部 invoked events，可接受没有 operation 的事件。 |
| B23 | medium | bad_spec | COM-SPEC 过滤后检查，额外 foreign operation 的 spec 被忽略。 |
| B24 | medium | bad_spec | 空 C/S interface 被接受而没有通信 API/绑定。 |
| B25 | medium | patch | 无参数 (void) operation 分支没有实际 C 消费。 |
| B26 | medium | patch | 同接口多个 operation 缺失第二 spec/call/event 没有双入口验证。 |
| B27 | medium | patch | 改名只查 metadata，未实际 C 消费新宏与头文件。 |
| B28 | medium | bad_spec | contract.json 没有计划已有完整 rule identity 与 extension identities，只有摘要，交付消费者无法核对完整来源。 |
| B29 | false | reject | Issue 包含源文件、完整对象路径、具体错误代码与修复提示，正常 projection 可定位非法关系。 |
| B210 | medium | patch | 实际 GUI 明暗/窄窗口通过，长名称及英文尚未实测。 |
| B211 | medium | patch | 下游 fail-closed guard 存在但编辑/handoff 无副作用未实际调用验证。 |
| E21 | high | bad_spec | USE-VOID 被接受但 server 仍生成 uint8 *，违背 SWS_Rte_07027。 |
| E22 | high | bad_spec | task ref 存在而 isMappedToTask 缺省仍接受，违背 RTE CONSTR08936，BSW08938 同样需闭合。 |
| E23 | high | bad_spec | 真实 accepted USE-VOID 生成声明不符标准，同 E21。 |
| E24 | high | bad_spec | 真实缺省 mapped flag 配置被接受，同 E22。 |
| V21 | medium | patch | 接受预核实证据：multi periodic/server concurrent=true 未测试，旧 fixture 走另一分支。 |
| V22 | medium | patch | 接受预核实证据：改名宏和唯一符号未被实际 C TU 消费，同 B27。 |
| R21 | high | patch | 全量 native 实测41/42，单一 Echo 类型的重复实例误入 multi，原 MULTIPLE_INSTANCES 变 INSTANCE_MAPPING。 |

| B31 | false | reject | rules.rs context_accepts 明确将 CLIENT-SERVER-OPERATION/ARGUMENTS 限为 ARGUMENT-DATA-PROTOTYPE，正常入口提前拒绝 SW-SERVICE-ARG，不会静默生成void。 |
| B32 | medium | patch | native_type 数组路径只检查固定长度和引用，不检查element category；已示出的形状变化可被接受，直接补首批TYPE_REFERENCE约束。 |
| B33 | medium | patch | DATA-ELEMENT-IREF允许多个context，referenced只取首个，平坦profile可能忽略额外context；改用现有唯一引用helper。 |
| B34 | medium | patch | SWC映射末尾循环无member数量/唯一性检查，重复iref或无COMPONENT-IREFS容器的mapping可被接纳；直接要求非空唯一member。 |
| B35 | medium | patch | unsigned_encoding只看encoding而不看base category，固定宽度支持约束不完整；直接要求FIXED_LENGTH并覆盖8/32位。 |
| B36 | medium | patch | multi可保存Some(0)，ComTimeout正数转换返回None导致零超时被拒绝；零禁用配置对两种待选DM范围均有意义，新profile专用允许零转换，旧profile保持。 |
| B37 | false | reject | 实际SymbolContract消费者来自完整data_accesses/callers，不消费代表性DataPort.runnable来丢弃成员；没有被指出的实际下游信息丢失。V31单独补验证集合。 |
| B38 | low | patch | 生成README的validated plan表述可误解为完整ECU计划，直接收窄为组件声明与所列关系，不增加输出字段。 |
| B39 | medium | patch | 通用return说明没有区分已核定的local初值与network freshness；直接补准确的声明阶段/API家族文档，不冒称runtime已交付。 |
| B310 | low | reject | 按技能拒绝修复只修改本build规格的finding。KEEP patch为可选当前重推导素材，最终可持久实现通过commit/正常测试入口交接，临时原始证据不冒称随Git交付。 |
| E31 | high | patch | 新增正常grammar允许非零OFFSET，旧component确实未检查，会忽略offset调度；直接拒绝旧profile非零并验证两入口。 |
| E32 | high | patch | 新增grammar允许非零MINIMUM_START_INTERVAL，旧component未检查，会忽略调用约束；直接拒绝所有相关legacy runnable非零并验证。 |
| V31 | medium | patch | 接受预核实缺口：exported symbols消费者集合未独立断言，多合法runnable访问同port/多client caller应核对完整源路径集合。 |

| R32 | high | patch | 新 grammar 放行 legacy OFFSET/minimum interval 后，component 的拒绝在 Graph Err=>None 及 legacy whitelist 被丢弃，正常校验与生成分叉；必须窄范围传播这些本次新增约束的错误，不按历史问题延期。 |

## Verification

- 主代理最终默认 `cargo test --locked --manifest-path core/Cargo.toml`：17 unit、71 builtin integration、1 c_analysis、26 multi 全部通过。默认测试没有新增 compiler/Python 前置依赖。
- 主代理完整 `--features official-oracles`：17 unit、73 builtin、1 c_analysis、42 end_to_end、30 multi 全部通过；完整 `--features native-tests`：28 unit、71 builtin、1 c_analysis、27 multi、42 native 全部通过。旧 multiple-instances 的精确诊断回归已恢复，没有以更新旧预期吸收错误。原生层本机为 Linux，Windows 尚未实际运行。
- 主代理最终 `AUTOSAR_MULTI_CONTRACT_OUTPUT=/tmp/autsaro-multi-contract-final cargo test --locked --manifest-path core/Cargo.toml --features native-tests,official-oracles --test multi_component_contracts`：31 项通过。真实 GCC 13 C99 `-Wall -Wextra -Werror -pedantic` 编译/链接并运行全部 S/R、scalar/array、DCM及无参数 (void) API 独立 consuming TUs；另一组完整改名实际 include Compute header，消费 Rte_Read_InputValue_Value 和改名 C/S 宏/唯一符号。此为声明 ABI 验证，未实现 RTE dispatch。
- 固定 R24-11 XSD 原创九份输入和合法零 offset／多 operation／执行约束向量通过官方入口；原生/官方实际 header 字节一致。正常 DefinitionCatalog 与 native plan 同时覆盖非法关系和具体错误，包含缺省/false app/BSW mapped flag、policy、foreign/重复 spec、空接口、无绑定 invoked event和并发。XSD 不能替代模块符合性。
- UI 94 项／11 files、lint、TypeScript/Vite build 通过；最终 native-webdriver desktop 重编通过。主代理真实 Linux Tauri/WebKit IPC，私有 Xvfb/DBus：中文原始九文件与英文长名称九文件均实际导入、检查四组件计划、确认旧单组件编辑器不显示、1440/900窗口、light/dark，输入原字节不变。实际结果分别 `/tmp/autsaro-r6-story81-native-ui-v5`、`/tmp/autsaro-r6-story81-native-ui-en-long-v11`。PR 实际截图保存于 docs/images/workbench-multi-components-zh.png 与 workbench-multi-components-en-long.png。未占用用户桌面，未以 mock 或构建代替此证据；底栏旧 host diagnostics 与 native projection 的既有显示分歧已记录 Epic context，8.4 必须闭合，当前只读计划不声称完整编辑体验。
- 从最终实际契约目录对四组件 header TUs 再次 GCC C99 编译通过；Cppcheck 2.21.0、固定 MISRA addon、unix64/C99及实际 GCC/system include 路径分析结束，无 syntax/addon/missinginclude/配置失败。145 项诊断中118项位于系统 header、27项为TU/生成header/checker report，原始 `/tmp/autsaro-r6-contract-analysis-final`。先前缺失系统 include 的补充分析没有当作通过；修正真实 include 后重跑。未进行完整授权主规范逐条人工评估，不声明 MISRA 合规。
- 正常生成并封存旧 standard-ecu profile 的 c-check 已实际执行：42 units，两个分析程序各41完整扫描，error=null，1806项既有源码诊断，passed=false。仅为未受当前C模板修改影响的旧profile分析对照，不冒称源码合规。8.1只交付 declarations，没有完整 multi ECU；8.2实际完整工程的正常 c-check 是必需退出条件，不能用contract-only补充分析冒充。
- 最终 `autosar_tooling quality --base 74128a430616695a2d0b09732198a0ede1320ef1`、clippy correctness/suspicious、`assets check`、`git diff --check` 均通过。无运行时/交付模板及资产摘要改动。

冻结矩阵审计：多组件与纯 local 对应 source_derived_multi_contract_is_deterministic_and_keeps_local_identity；非法关系对应 malformed_multi_contracts_are_rejected_at_real_source_objects、normal_definition_validation_closes_multi_schedule_routes_types_and_handles、client_server_sets_close_in_both_directions_and_empty_interfaces_are_rejected；类型对应 unsigned_contract_rejects_conflicting_base_encodings_with_valid_references 及 malformed 向量；命名对应命名碰撞专项与 renamed_component_and_port_contracts_follow_full_source_identity；旧profile对应完整 official/native 回归及 normal_legacy_definition_validation_rejects_only_new_nonzero_execution_constraints、legacy_native_rejects_nonzero_execution_constraints_and_keeps_exact_zero、legacy_official_legal_inputs_reject_nonzero_execution_constraints。上述测试均实际运行通过。用户源码/输出保持在 unsupported_edit_and_handoff_preserve_live_inputs_and_existing_output 验证；此只证明未支持入口 fail-closed，不替代8.3再生成或8.5交接验收。

本故事全部矩阵和声明退出条件通过。8.2–8.5仍未实施完成，真实通信/OS顺序、源码槽保护、编辑及独立交接都不在本故事声称通过。网络 DM 范围仍待用户选择，不把 Epic7整体完成作为前置条件。
