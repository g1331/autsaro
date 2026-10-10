---
title: '独立交接并复验完整多组件工程'
type: 'feature'
created: '2026-10-10'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '660c0217e126ebd5208e052f0c2c59d9f23d9675'
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/CONTRIBUTING.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-8-context.md'
  - '{project-root}/_bmad-output/specs/spec-multi-component-scheduling/acceptance.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**问题：** 多组件生成、调度、源码保护和编辑已有验证，完整封存包搬移、移除原工作区后重新导入、再生成和实际运行尚未闭环；交付 README 仍写成单组件源码。

**方案：** 完成候选 R6／Story 8.5 的独立接收闭包，让接手者按包内正常入口恢复全部输入、离线构建并复验三组件真实行为。

## Boundaries & Constraints

**始终：** 固定 R24-11、既定首批 profile 与真实用户应用。复用严格 v2 importer、封存验证、可信计划、Python -I -S 和受控工具。全部 slot／owner／producer／逻辑与封存路径精确对应，拒绝不能发布部分接收工程。旧 host／single、v1／v2 入口与源字节保持。用户已授权整个 Epic 至 PR／CI／合并；8.4 阶段合并后继续实施8.5。

**禁止：** 修改哈希掩盖篡改、放宽身份或重新生成字节比对、编辑封存源码、引入第二交接器、额外报告／台账、扩展 R11 或 Epic7 发行收尾。未运行的平台、安装、硬件和规范人工评估不记通过。

## I/O & Edge-Case Matrix

| 场景 | 输入／动作 | 预期与拒绝 |
| --- | --- | --- |
| 独立恢复 | 真实三应用封存包搬到接收目录，原包与 live 不存在；Rust导入并重新生成 | 全部输入／应用原字节与完整槽映射保持；所有恢复磁盘路径在新 live；新封存 payload 完全相同 |
| 独立运行 | 新封存包内 Python离线构建生产 owner 和已有工程外 probe | AC-2／3 固定 CAN／DID、重复epoch、S/R加1、同步C/S值／上下文与真实任务顺序成立 |
| 拒绝 | 封存C篡改、重封的未知owner／producer、路径逃逸、规则身份不符 | Rust与包内Python实际入口拒绝对应类别；无新receiver／build、包和外部sentinel不变 |
| 兼容与分析 | 旧host／single、v1／v2正常入口；实际multi与受影响single c-check | 既有成功／拒绝保持，工具身份、翻译单元、完整性、exit与源码诊断真实记录 |

</frozen-after-approval>

## Code Map

- `core/src/generator/delivery.rs::append_native_readme` 与唯一 caller `core/src/prepared.rs::NativePreparation::new`：使用已经验证的 metadata与slots 写真实多组件源码归属，host／single说明字节保持。
- `core/src/generator/delivery/{reopen,ownership}.rs`：复用严格重建／全payload比对、路径与身份边界；不改守卫。
- `core/tests/multi_component_contracts.rs`：复用真实live初始化、fixture算法、生产owner固定向量，新增或扩展完整搬移／导入／再生成及拒绝矩阵。
- `core/tests/c_analysis.rs`：通过真正live初始化、用户源码、搬移／导入／再生成输出分析样本，原live／原包移除，全payload相等；复用正常样本导出环境。
- `core/tests/support/{tooling,epic7_delivery}.rs`：复用包内 -I -S 构建、payload／reseal及旧host／single回归；复用现有helpers时保持路径安全。
- `core/tests/fixtures/multi-application/owner_probe.c`：canonical真实owner检查四runnable计数、顺序／上下文违例为0、三个观测值0x12345679与E_OK；原源码复用。

## Tasks & Acceptance

- [x] `core/src/generator/delivery.rs`、`core/src/prepared.rs` — 多组件 README从完整slot和metadata说明live→sealed、type、instance、header与入口归属，保留旧说明。
- [x] `core/tests/multi_component_contracts.rs`、`core/tests/c_analysis.rs`、必要既有support — 真实搬移、移除原live、导入、新生成、逐字节比较、精确三槽及离线实际运行；拒绝矩阵同时走Rust和Python正常入口。
- [x] 现有 BMad规格 — 记录旧v1／v2／host／single回归、实际生成multi／single c-check；按真实阶段更新sprint。独立审查、远端CI和merge是实现后交付门，未完成前Epic保持进行中。

**验收：** Given 8.4完整工程与真实用户源码，when正常Rust／Python接收闭包执行，then AC-10–12及全部拒绝成立、没有原机器路径或官方档案运行依赖，审查和必要CI通过后按授权提交、PR和合并。

## Implementation Notes

- README仅multi分支按已验证metadata和slots输出完整类型、实例producer、live→sealed、headers与entry symbols，唯一caller同步传入真实数据；host／single段落字节保持。没有C／模板／公开ABI改动，也没有放宽importer或Python封存守卫。
- 真实live初始化后拷贝已有三份用户算法，搬移完整包并移除自身原目录，再Rust导入与正常再生成；精确核对三槽producer／logical／compiled、owner和snapshotOf、全部源字节、接收磁盘归属、无原目录绝对路径泄漏与全payload相等。正常Python -I -S在consumer目录离线构建生产host-batch与已有canonical probe，工程外控制源不参与生产构建。
- 六类拒绝同时经过Rust和实际包内Python入口；重封不能授予修改生成物的权利。escaped ledger path保留合法排序且只重算外层seal，不沿恶意路径读取；逐类匹配拒绝类别，接收者旧文件、外部sentinel、live输入和封存包保持，无新build。共用既有payload／reseal帮助函数移到test support，原single测试复用；生产向量断言整体移成共享测试函数，原四种案例继续通过。
- 实际C分析样本也走完整搬移／恢复／重新生成再输出，源包必需真实handoff=true，不改source-only拒绝。原single与host实际README逐字节核对保持；旧v1真实host与ECU协议正常入口另行运行。

## Spec Change Log

- 2026-10-10：沿用用户最新“继续完成整个 Epic”及 PR／merge 授权，原 Story 8.5、AC-10–12 与首批契约不变，无新意图决策；不再重复请求阶段批准。调查由只读代理执行，代码与修复由主代理负责。规划已检查具体路径、正常消费者、成功／拒绝矩阵和旧行为，达到实施就绪；8.4 阶段合并后捕获基准并实施。

## Review Triage Log

| 审查层 | 返回发现 | 裁定与依据 |
| --- | --- | --- |
| Blind | `[]` | 只读追踪全 diff 与接收链，另跑非 native handoff 3 项，无可证实缺陷；没有继承未运行平台结论。 |
| Edge-case | `[]` | 对冻结成功／拒绝矩阵及外部副作用复核，无待处置发现。 |
| Verification | `[]` | 非实现者独立重跑正常 native handoff 命令，exit0、3 passed／0 failed／0 ignored，24.74s；检查真实生产向量、canonical probe 的四 runnable 次数／顺序／上下文以及六类 Rust／Python 拒绝和字节保护。 |

三层结果已完整收齐；没有 deferred 项。主代理核对测试源与实际运行证据，未将分析完整性或 Linux 行为扩大为源码符合、Windows 运行、安装或硬件验收。

## Design Notes

调查未发现需要用户决定的意图缺口。代码 footprint是已有README生成及测试，外部PR／merge已授权；临时测试仅删除自身Scratch。旧host／single的完整README保持。本次新生成multi包遵守全payload重建契约；历史未交付multi封存若生成规则字节不同仍明确拒绝，不通过跳过README比对追认。控制源仅用于已有test构建，不链接进生产owner。

## Verification

- 多组件实际搬移／重导入／重新生成后的分析：`/tmp/autosar-handoff85-multi-c-check/summary.json`，52个不同翻译单元，host-batch／legacy-probe各51，两个程序exit1，2607诊断（2573style、30warning、2error、1portability、1information），692 adopted，passed=false、error=null。固定Cppcheck／addons身份与单组件一致；source seal1439479603eb7219e63ae5f84942d521f521faab394b06ed8bf5816106004685，两个真实入口分析完整。非style与已有8.2分析一致，无suppression／关闭规则／篡改基线；221项规范人工assessment仍not_assessed，不声明完整符合。
- 冻结矩阵审计：独立恢复与独立运行由 moved_multi_handoff_reconstructs_every_user_source_and_exact_producer 的最终native执行覆盖；拒绝由 multi_handoff_rejects_resealed_authority_and_snapshot_forgery 的最终native执行覆盖；兼容由builtin72与semantic_profiles真实旧v1执行覆盖，分析由c_analysis2和两次实际c-check完整性覆盖。功能与记录均达到实现出口，独立三层审查、PR／CI／merge继续后续步骤，当前不标Epic完成。

- 当前native handoff 3／3 passed（24.79s），新成功测试执行Rust重建与包内生产／probe实际行为，拒绝测试涵盖六种Rust／Python拒绝及字节保护；原生产owner四组案例1／1 passed（45.98s）。builtin72／72 passed（20.80s），真实旧v1 host与集成ECU协议1／1 passed（12.45s），完整c_analysis2／2 passed（21.10s）。quality6、native-feature all-targets Clippy、assets0通过。
- 过程失败保留：初次新增测试误复用消费式PreparedProject导致编译错误，改为每次合法准备并在generate前取slot；逃逸path初次先被ledger排序守卫拦截，修正测试构造保持canonical顺序后实际portable_path_unsafe和Python Unsafe v2 portable path均通过。未调整产品守卫或放宽预期。
- GUI沿用8.4完整开发native21／21；本故事没有GUI交互改动。Windows交接行为／安装／网络namespace／硬件本地未运行；普通CI各平台范围与真实运行范围区分。

- 实施前正常 c_analysis 生成六种实际样本，1／1 passed（12.21s）。单组件标准ECU／用户源码初始化 profile：`/tmp/autosar-handoff85-single-c-check/summary.json`，42个不同翻译单元，host-batch／legacy-probe各41，完整分析error=null、两个程序exit1，1814诊断（1780style、30warning、2error、1portability、1information），passed=false。Cppcheck2.21.0，源码revision e73bf44c3e49686b7495fab352d03a6c6075516b，binary SHA434155fc2a092b4dd98042052fef93a501111bb18849e5b1db05e8854f8ba795；C99／受控linux，221项人工assessment仍not_assessed。受信source seal b9cf997b07f30a4de2ca0e6b72391f253ebd2ea12f4f438c65d900348af8da1e；本故事不改变single源码生成，后续核对README旧字节并复用此实际结果。

- 正常Cargo受影响multi交付／拒绝测试、builtin v2与既有v1／host／single回归；native-tests实际包内Python构建及生产／probe运行。
- `autosar_tooling quality`、Clippy、`assets check`及必要CI；无C模板变更时不扩大无关测试。
- 对实际重新生成multi及single封存工程运行 `c-check --target linux-x64-controlled-v1`，保留passed=false源码诊断与未评估人工项，分析完整不等于完整MISRA符合。

- 2026-10-10 13:32：实施及三层独立审查完成，规格状态done、sprint故事review；提交／PR／远端CI／合并继续按既有授权执行，Epic当前仍in-progress。
