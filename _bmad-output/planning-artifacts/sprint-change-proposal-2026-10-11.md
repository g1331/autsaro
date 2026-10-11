# Epic 9 收敛为单一主线并建立新工程基线

日期：2026-10-11。分析基线：`fdc05f1`（已获取远端 master）。工作模式：批量；本文中的决定均已由用户在本轮讨论中逐项确认。本次只修改规划与规格，不修改代码、CI 或分支保护，不批准任何标准偏离。

## 1. 问题与依据

Epic 9 原定在三个生成族上分别完成标准整改：legacy host、单组件和多组件。核对后发现三个问题。这些问题不先处理，后续每个模块 Story 都要在多套实现和历史命名上重复投入。

**同一能力有多套实现。**

| 链路 | 入口 | profile | 主要代码 |
| --- | --- | --- | --- |
| 帧／信号 host | 模板 `can-empty-v1`、`can-signals-v1`；“主机运行”页 | `host-can-v1` | `prepare_host_project`→`generator/render.rs`、`core/src/host/`、`arxml/host_profile.rs`、`package_host_reference` |
| 单组件 ECU | 模板 `standard-ecu-v1`（默认内置模板） | `epic4-win64-sr-cs-v1` | `integration/ecu.rs`、`contracts.rs`、`runtime/src`、`generate_epic4_ecu` |
| 多组件 ECU | 工程中有多于一种 `APPLICATION-SW-COMPONENT-TYPE` 时 | `singlecore-multi-swc-v1` | `integration/multi*.rs`、`runtime/multi` |

- 选择条件在 `core/src/integration/multi.rs:182`（`multi::selected()`，SWC 类型数 `> 1`）。所以从默认模板新建的工程不走多组件主线。
- 主线仍借用 legacy 的 Can、Dem、NvM、Security。Dcm 的 producer 身份记录为 `runtime/src/Dcm.c`（`multi.rs:1786`），实际使用的却是 `runtime/multi/src/Dcm.c`。
- “主机运行”页（`ui/src/pages/VirtualPage.tsx` → `host::run`／`run_diagnostic`）只读取 `render.rs` 输出的 `profile.txt`，只服务于帧／信号 host 链路。
- 校验也有两套：用户在设置里配置 XSD／MOD 档案后（`Workspace::uses_legacy_validation`），创建、打开、编辑、计划、保存和生成都会切换到 9 组 `*_legacy` 平行实现。

**开发计划编号成了产品身份。** 除 `_bmad*`、`.agents`、`.claude` 外，共有 271 行引用。

- 交付身份 `epic4-single-application-v1`、`epic4-win64-sr-cs-v1`；
- `generate_epic4_ecu` bin；
- 产品代码 `include_str!` 引用 `core/tests/fixtures/epic4/`；
- `core/tests/support/epic4_*`、`epic7_*` 共 16 个文件，以及多个 fixture 目录。

另外，`runtime/multi`、`integration/multi_*.rs`、`multi_*_contracts.rs` 用“multi”区分新旧实现，没有表达职责。

**工程基础本身的问题。**

- **CLI 子命令形式的测试。** 一些测试和预期做成了 CLI 子命令，绕开了测试框架：`os_suites.py`（1979 行）、`input_oracles`、`protocol_oracles`、`bsw_catalog`、`probe`。
- **Windows 桌面隔离验收自建了 Win32 设施。** 包括 WindowStation、token、elevate、WFP 等约十个模块。
- **lint 门槛很低。** clippy 只拦 `correctness`／`suspicious`；`core/src` 中约 630 处 `unwrap`／`expect`／`panic!`（粗略统计，含少量内联测试）。
- **生成器回读并修补自己的输出。** 生成 C 后再回读修补，例如 `multi_ecu.rs:93-141` 的 `get_mut(..).unwrap()` 加 `.replace("#endif", ..)`。
- **超大文件。** `multi_component_contracts.rs` 7087 行，`desktop_builtin_scenario.mjs` 2632 行。
- **两个独立 Cargo manifest。** `core/` 和 `src-tauri/` 各自构建、各自测试。
- **版本后缀没有意义。** 很多标识带 `-v1`／`-v2`，但没有任何兼容承诺。

项目尚无稳定版本，没有需要保护的已交付用户工程。

## 2. 已确认的决定

1. **单一主线。** 新生成的工程全部走现在的多组件主线，单组件就是 SWC 数为 1 的特例。主线按职责命名，不再叫“multi”。
2. **不保留向后兼容。** 稳定版本发布前，以最新主线为准。旧 profile 和旧格式的工程、交付包不识别、不迁移，打开或复验时**明确拒绝**，给出可理解的错误，不静默误解析。原计划的“legacy 识别、重开、迁移 Story”取消。不兼容的对象是工作台自己的工程与交付元数据；用户的 R24-11 ARXML 源文件仍是合法输入，可以导入新工程。
3. **删除帧／信号 host 链路。** 包括两个 UI 模板、“主机运行”页、`core/src/host/`、`render.rs`、`host_profile.rs`、`package_host_reference`。需要空白起点时，提供主线上的最小模板。“在工作台内运行生成的 ECU”这一能力保留为产品目标，记为已知差距：Epic 9 之后基于 9.3 稳定的 host-batch 契约在主线上重做；在此之前用 `ecu-tool verify` 和原生测试验证。
4. **删除外部 XSD／MOD 校验模式。** 包括设置项、`schema_zip` 状态和全部 `*_legacy` 链路。`schema.rs` 的 XSD 校验保留，只作为测试和 acceptance 中的官方对照基准，用于检验内置规则。
5. **彻底去编号。** 产品代码、测试、fixture、工具、bin、目录名、测试名、读者文档中都不出现 Epic／Story 编号。历史规格（`_bmad-output/` 中已完成的记录）保持原样。
6. **生成规范。** emitter 不 panic、不回读自己生成的 C 文本，计划无法保证的前提返回 `PlanDiagnostic`；每个输出文件只有一个生成方。
7. **脚本治理纳入 Epic 9。** 只保留三类：交付工具（`ecu_tools`）、统一开发 CLI（`doctor`、`verify`、`quality`、`c-check`、`assets`）、放在 cargo test／unittest／vitest／node:test 中的测试。桌面隔离设施在 Epic 9 内评估并替换。
8. **工程实践补充。**
   - 生成管线有结构保证；
   - 用 lint 和 `quality` 检查强制规范；
   - 测试分层，扩大必需 CI 的覆盖面；
   - 根目录 Cargo workspace；
   - 版本与格式标识策略；
   - 架构说明跟上新结构；
   - 基线退出标准与分阶段 PR。

## 3. 影响分析

- **Epic 9 范围：**
  - 由“三族分别整改”改为“先收敛出一条主线，再在主线上完成标准整改”。
  - 9.1、9.2 的完成状态和历史证据保持不变。9.2 修改的是 `runtime/src/Can.c`，这些成果随 9.22 进入主线。
  - 新增 8 个基线 Story（9.21–9.28），在 9.3–9.20 之前执行。
- **9.3–9.20：** 范围只针对主线。删除“legacy／single 重新生成”和“旧包契约”等兼容要求；保留独立预期、成功／拒绝／恢复、用户源码保护和完整性拒绝。9.16 改为验证主线交付与离线工具。
- **PRD：** QLT-1、QLT-2 不再要求覆盖三族、回归旧工程，改为单一主线，并说明稳定版本前的兼容策略。
- **架构：**
  - Epic 9 spine 中：AD-1 改为单一主线；AD-3 补充文件唯一生成方和禁止回读；AD-4 改为稳定版前不兼容、明确拒绝；AD-6 要求测试放入测试框架；AD-7 的样本集随主线重定。
  - 新增 AD-9：工程基线（去编号、按职责命名、工具三分类、lint 门槛、workspace、格式版本）。
  - Structural Seed 改为新目录建议。
- **UX：**
  - 新建工程的模板选项减少；
  - “主机运行”页和设置中的 XSD／MOD 项删除；
  - 打开旧工程时显示明确拒绝。

  修改界面时按 DESIGN.md 保持一致，用户帮助和 README 同步修改。不新增设计。
- **代码与交付：**
  - **大量删除和改名：** `runtime/src`、`integration/ecu.rs` 等单组件和 host 链路代码、`*_legacy`、两个 Epic 命名的 bin。
  - **交付契约改变：** profile ID、交付身份、`runtime/contracts`、`assets-v1.json` 改变，属于破坏性变更，写入 CHANGELOG。ABI 与第三方身份仍然逐项审阅，资源摘要变化不能代替审阅。
- **AUTOSAR 依据：**上述决定属于产品与工程决策，AUTOSAR 不规定生成工具的内部链路、工程格式或开发工具。与规范相关的约束有三处，已写入对应 Story：9.21 按 RTE SWS 核对单 SWC 时的 RTE 义务；9.23 保留官方 XSD 对照测试，作为内置规则符合 R24-11 schema 的证据；9.26 改名时标准模块文件、头文件和内存段名称按 SWS BSW General 与 RTE 保持不变。
- **CI 与文档：**
  - `checks.yml` 中两个必需原生套件改名；
  - `docs/development/testing.md`、AGENTS.md、README 按新结构同步；
  - 本提案先修订 AGENTS.md 中与新兼容策略冲突的规则。
- **已知差距：**
  - 9.3 的 Windows 并行 `E_OS_STATE`：如果根因在共用的 `runtime/os`，不能因为删除 legacy 测试就当作不存在，由 9.24 迁移时复现并归入 9.3 或 9.14。
  - 9.15 的 Dcm 来源身份不一致由 9.22 关闭。
  - 9.2 遗留的 MISRA 人工评估和 c-check 清零仍由各模块 Story 承担。

## 4. 推荐方案、代价与风险

采用**直接调整**：Epic 9 内新增基线 Story，改写后续 Story 范围。不回滚已完成工作，也不缩减产品目标。删除 legacy 不是缩减支持，而是在稳定版本前收敛实现。

**执行顺序**（每一步一个或多个可独立验证的 PR，合并后 master 保持绿色）：

1. 9.21 主线接管单 SWC 工程。风险最高，先做。
2. 9.22 主线拥有全部 BSW 来源。
3. 9.23 删除帧／信号 host 链路、单组件链路和外部档案模式。
4. 9.24 测试与开发工具归位；9.25 桌面验收设施替换。两者可并行。
5. 9.26 去编号、按职责命名与目录结构。
6. 9.27 生成管线结构与静态检查门槛。
7. 9.28 按所有权组织生成工程（静态 BSW、生成物、用户快照、适配层分目录，并从同一清单生成 CMake 构建描述）。
8. 9.3–9.15 模块整改 → 9.16 交付 → 9.17、9.18 反馈与 CI → 9.19 严格门 → 9.20 出口。

代价与风险：

- **代价：** 规划修改投入低。基线 Story 投入高，但会减少 9.3–9.20 中的重复实现和重复验证。目前给不出可靠工期。
- **9.21：** 主线处理单 SWC 时，可能暴露多组件假设（比如“至少两个组件”的连接、调度和 RTE 生成）。先用最小单 SWC 正反例和内置模板端到端验证。
- **9.23：** 删除 legacy 时可能连带删掉有价值的独立预期，比如协议、OS 时序、输入预期。删除前必须先把这类预期迁到主线，并在主线上通过。只验证 legacy 内部行为的测试才能删除。
- **9.24、9.26：** 迁移和改名会动到原生 fixture、交付布局和测试共用的路径辅助。按 AGENTS.md，合并前要在本地 Linux 和 Windows 运行受影响的原生套件，并请维护者触发 acceptance。
- **9.27：** 全量 clippy 默认集可能带出大量既有告警。生成和集成模块必须清零，其他模块先固定基线，逐步收紧，不放宽门槛。

## 5. 具体规划修改

| 工件／位置 | 旧 → 新 | 原因 |
| --- | --- | --- |
| `prd.md` QLT-1、QLT-2 | 三族范围与旧工程回归 → 单一主线范围；稳定版本前不兼容，明确拒绝旧工程 | 用户决定 1、2 |
| `epics.md` Epic 9 头部与退出条件 | 三族完整范围、旧工程兼容和显式迁移 → 先收敛主线再整改，新增“基线退出”条件 | 决定 1–8 |
| `epics.md` 9.3–9.20 | legacy／multi 范围与旧包契约 → 主线范围；9.16 改为交付与离线工具；依赖新增 9.21–9.28 | 同上 |
| `epics.md` 新增 9.21–9.28 | 无 → 8 个基线 Story，含验收标准 | 同上 |
| `architecture/epic-9/ARCHITECTURE-SPINE.md` | AD-1、AD-3、AD-4、AD-6、AD-7、Structural Seed 修订；新增 AD-9 | 架构约束随决定改变 |
| `specs/spec-runtime-codegen-quality/` | SPEC 范围、CAP-2、CAP-4 修订；`generator-practices.md` 新增 emitter 规范；`work-packages.md` 新增基线工作包 | 同上 |
| `sprint-status.yaml` | 新增 9.21–9.28 为 backlog；9.16 标题改名 | 跟踪新 Story |
| `epic-9-context.md` | 按上述内容重写 | 编译出的上下文需与规划一致 |
| `AGENTS.md` | “历史 profile 兼容按真实结构身份识别”→ 稳定版本前不兼容、明确拒绝 | 避免规则与决定冲突 |
| `.gitignore` | 删除 `.scratch/epic4/` 等规则由 9.26 处理 | 属于代码 Story，本次不改 |

## 6. 交接与完成判据

变更级别：**Moderate**，Epic 内 backlog 重组与范围收敛，不重做 PRD，也不重新选择技术栈或 OS。

1. 本提案和规格修改经过用户审阅，通过 PR 合并后生效。合并本身只完成规划调整。
2. 每个基线 Story 开工前，按 BMad 流程补齐具体方案和验证命令。按顺序推进，9.24 与 9.25 可并行。
3. **“新基线达成”的条件**是 9.21–9.28 全部 done，并满足：
   - 产品、测试、工具路径中 Epic／Story 编号检查为零，由 `quality` 强制；
   - 帧／信号 host 链路、单组件链路和外部档案模式代码已删除，打开旧工程会被明确拒绝；
   - 必需 CI 覆盖主线全部不依赖官方档案的原生行为；
   - 生成和集成模块通过 deny 级别的 panic 与回读检查；
   - 交付包按所有权分目录，CMake 与 `ecu-tool` 用同一清单构建通过；
   - Linux 和 Windows 各至少完成一次 acceptance；
   - ABI、第三方身份和资源摘要的变化经过审阅。

   达成后才开始 9.3–9.15。
4. Epic 9 的整体出口仍由 9.20 判定。完整 AUTOSAR／MISRA 符合性不因本次调整而成立。

### 变更检查

| Checklist 项 | 结果 |
| --- | --- |
| 1.1–1.3 触发与证据 | [x] 由用户在 Epic 9 推进中提出；依据见第 1 节，均来自 `fdc05f1` 实测 |
| 2.1–2.5 Epic 与顺序 | [x] Epic 9 保留并扩充，新增 9.21–9.28 并先行；Epic 10、R11 的依赖关系不变 |
| 3.1–3.4 PRD／架构／UX／工程影响 | [x] 见第 3、5 节；UX 只涉及删减和拒绝提示 |
| 4.1 直接调整 | [x] 采用 |
| 4.2 回滚、4.3 降低 MVP | [N/A] 不回滚已完成成果；删除 legacy 是收敛实现，不是降低目标 |
| 5.1–5.5 提案与交接 | [x] 已形成 |
| 6.1–6.5 审阅与跟踪 | [!] 等用户审阅；提交、推送、PR 和合并须另行授权 |
