---
title: '经真实 OS 调度运行多组件通信'
type: 'feature'
created: '2026-10-09'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 319d39f9c688c59b6a19ff8c2b7c399874534cba
context:
  - /root/.t3/worktrees/autsaro/t3code-b540700d/AGENTS.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/implementation-artifacts/epic-8-context.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/application-contract.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/acceptance.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/compliance-references.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/.agents/skills/misra-c2012/SKILL.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

8.1 只有声明，完整 multi 工程仍拒绝生成。实现候选 R6 的实际 RTE／SchM／OS／通信闭环，先通过 AC-1–AC-8，再进入源码编辑及交接。

## Boundaries & Constraints

**Always:** 用户已选择最小真实 Rx I-PDU group、标准 DM、COM→RTE 通知，保留正数网络 aliveTimeout。固定 R24-11／C99；同一可信计划生成全部配置、接口和任务调用。按所选配置关闭 COM→PduR→CanIf 及 Dcm→PduR→CanTp 直接依赖的标准接口、状态与回调差距，独立核验官方契约。正常 sealed 工程运行真实用户测试应用和实际 OS；各组件 caller-provided 源码的最小可信准备／快照／归属接纳是本故事必要切片，8.3 承接 live 初始化与再生成流程。旧 profile 精确回归。

**Never:** 第二调度器、主机计时模拟新 DM、函数桩、生成业务算法、替换 sealed 源后重封、空壳标准包装、默认放行不支持配置。R11、Epic 7 收尾及 merge 不在范围。

## I/O & Edge-Case Matrix

| 场景 | 输入 | 预期 |
| --- | --- | --- |
| 通信／任务 | acceptance AC-1–AC-3 | 固定 CAN／DID／local／CS 值及 owner 顺序；重复 epoch 无重放 |
| 初值／隔离 | AC-4、AC-8 | 各 R 独立 init／fanout；CAN 停止不等于 COM_STOPPED；空参数拒绝且输出保持 |
| Rx DM | acceptance 末段独立向量 | 真实 group／周期 DM／接收与超时回调；NEVER_RECEIVED／MAX_AGE／恢复／启停／NONE 保值 |
| 非法配置 | AC-5–AC-7 及 group／callback／timebase | 来源明确，生成前拒绝，无部分产物 |
| 公开接口 | 所选 R24-11 COM、PduR、CanIf、CanTp、Dcm 类型化消费者 | 独立签名编译链接及真实成功／拒绝／buffer 生命周期，不以 inventory 生成唯一预期 |
| 来源／回归 | 全槽真实字节；旧 profiles | 精确来源闭包和 sealed 校验；已有行为不变 |

</frozen-after-approval>

## Code Map

- `core/src/integration/{multi,plan,configuration,schedule,ecu,artifacts,handoff}.rs`：8.1 私有模型已有完整身份；legacy guards 不可直接删除复用单组件模板。
- `core/src/{prepared.rs,generator/delivery.rs,generator/delivery/ownership.rs}`：NativeInputs 已有 application 数组，准备仍 first()／单槽；复用来源 guard、资源身份与正常 source-only prepare。
- `runtime/{include,src,ecu}`：旧 COM 主机签名／计时及单入口保留；新 profile 选择真实标准实现，Ecu_Target 的 mailbox／owner／tick 是唯一调度源。
- `core/tests/{multi_component_contracts.rs,support/epic4_ecu.rs,support/tooling.rs}`：复用正常生成／离线构建／probe／HostBatch；实际 C 行为测试置 native-tests。
- `runtime/contracts`、`tools/python/src/ecu_tools`：资产身份、包内工具与归属同步；审阅 ABI 后处理固定 CRLF 摘要。
- `/tmp/autsaro-r6-official/*.txt`：十六份官方 PDF 已匹配官方 SHA；COM／RTE handle-bearing 通知属于 R24-11 draft 条款，不能套旧零参回调。

## Tasks & Acceptance

- [ ] 模型／fixture／定义：真实 Rx group、timeout、通知 handle、主函数 timebase 和完整调度共同核定，所有非法关系有拒绝测试。
- [ ] 模板／运行时／调用方：生成多组件 RTE、标准选定链路配置、回调及真实任务调用；最小可信应用准备入口接纳所有计划槽。
- [ ] 正常工程入口：用户应用真实执行矩阵及 AC-1–AC-8；变名、fanout、DM 和下层拒绝验证全部运行，不跳过。
- [ ] 交付资源／BMad：审阅 ABI／第三方／换行；实际 multi 与受影响 single c-check，记录完整性与诊断，必要回归通过。

验收：真实生成与 OS 运行关口全部通过，才推进 8.3–8.5；无授权 MISRA 主文不声明完整符合。

## Implementation Notes

最小准备入口按计划接受实例身份及真实磁盘文件，复用 read_source／refuse_links／NativeGuard 冻结来源；精确匹配全部槽，拒绝缺失／重复／额外成员及源文件复用。输出路径／owner／entry 由计划决定，无任意 file-map 替换；guard revision 必须覆盖全部输入。复用既有 v2 policy／seal／builder，多资产选择与 source owner 一致，OS build profile 仍 ecu。此入口不初始化 live／manifest，不承诺 handoff；8.3／8.5 状态保持。

2026-10-09 阶段 1（配置闭包，尚未完成运行关口）：新增 `integration/multi_com.rs`，可信计划的 `comRuntime` 记录唯一最小 Rx group 及其真实 PDU 成员、Rx／Tx 主函数实例名与 timebase、ComUserSignal 的系统映射和 uint16 通知 handle、first／regular timeout 及固定标准回调。原创 multi fixture 明确 group 0、Rx 1ms／Tx 10ms、handle 17、first 0／regular 30ms。只在 multi profile 接纳新增配置语法；旧 profile 不输出 `comRuntime`。R24-11 `ComUserModuleCnf` 的实际定义位于 `Rte/RteComUser` 下；通知声明头按 RTE SWS_Rte_91123／91127 核定为 `Rte_Com.h`。新增 builtin 定义的类型、范围、multiplicity 和枚举已通过固定官方 MOD oracle，不能把这项结果替代 C 行为证据。

本阶段保留全部 multi ECU 生成拒绝 guard，不借用旧单组件模板。当前 BSW 源描述仍以 `Com_AdvanceTime`／`Com_TriggerTransmit` 标记逻辑周期角色；`comRuntime` 仅核对其周期并保存真正的 `Com_MainFunctionRx_<shortName>`／`Com_MainFunctionTx_<shortName>` 名称，后续须共同迁移 runtime catalog、BSWMD、schedule 和生成调用到真实标准实例。尚未实现 ComM／BswM／CDD 配置闭包、标准 C 通信链、多槽可信准备、multi RTE／任务生成和真实 OS 运行；全部 Tasks 保持 unchecked，8.3–8.5 不推进。

## Spec Change Log

## Review Triage Log

## Verification

正常 Cargo default／native-tests／official-compat、Python、相关质量与 assets check；使用固定 GCC 13 的实际交付工程编译链接与运行。按 docs/development/testing.md 跑真实生成 profile 的 c-check，分析未完成须修正原因；诊断如实评估。矩阵逐行对应已执行测试。开发结果只写本规格及相关既有 BMad 工件。

2026-10-09 阶段 1 已执行：`cargo test --manifest-path core/Cargo.toml` 117 tests passed；`cargo test --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts` 29 tests passed，包括独立 scalar／void typed header 消费者的真实编译、链接与运行（GCC 13，正常工具链环境）；`--features official-oracles --test builtin fixed_r24_11_oracle_agrees_on_integer_precision_enum_and_default` passed。新增拒绝向量在 normal definition validation 与 plan construction 两入口验证 group 数量／handle／成员／重复引用／Tx 成员、主函数 timebase／PDU 绑定、callback 类型／符号／header／缺 handle／重复引用，并回归零 first timeout 的合法数值拼写及生成符号碰撞。正常项目 Clippy（`-A clippy::all -D clippy::correctness -D clippy::suspicious`）、`quality --scope core --base 319d39f9c688c59b6a19ff8c2b7c399874534cba`、`assets check`（0 changes）及 `git diff --check` passed。

历史失败保留：首次 native typed 检查因未设置 `AUTOSAR_CC` 拒绝；补齐明确 GCC 13／objdump／git／Python 的正常环境后上述 29 tests passed。额外 `clippy --all-targets -- -D warnings` 被既有 `core/build.rs:24` 的 `too_many_arguments` 阻断；未扩大范围修复旧 lint，正常项目 Clippy 门通过。阶段 1 未修改 C／模板，未生成可运行 multi ECU，故本阶段没有 multi／受影响 single 的 c-check 运行证据，不声明 DM 行为、标准通信链、真实 OS AC-1–AC-8 或完整 MISRA 符合。
