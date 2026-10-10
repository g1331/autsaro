---
title: '保护全部组件用户源码再生成'
type: 'feature'
created: '2026-10-09'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 1eb81af87ea2248a33824800e8fa9a71eb056df9
context:
  - /root/.t3/worktrees/autsaro/t3code-b540700d/AGENTS.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/DESIGN.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/implementation-artifacts/epic-8-context.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/application-contract.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/_bmad-output/specs/spec-multi-component-scheduling/acceptance.md
  - /root/.t3/worktrees/autsaro/t3code-b540700d/.agents/skills/misra-c2012/SKILL.md
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

8.2真实闭环已通过；正常工作区仍只允许一个应用源。扩展既有初始化、manifest重开和生成入口到全部可信组件槽，完成AC-9。

## Boundaries & Constraints

始终由真实计划核定producer、full componentPath、live→sealed路径与全部入口签名；初始化只创建不存在文件、完整预览确认、全部成员与manifest一次接纳。用户编辑后只快照真实字节，任何stale确认拒绝。保留旧单槽行为、无关配置通用重开与未改ARXML字节；R24-11／C99／MISRA及原事务平台边界不变。

不生成参考业务算法，不覆盖用户源，不依赖编译器或官方档案完成初始化，不扩展R11；8.4完整编辑体验与8.5独立交接仍由后续故事验收。结果仅回到相关既有BMad规格。

## I/O & Edge-Case Matrix

| 情景 | 输入 | 预期 |
| --- | --- | --- |
| 初始化／重开 | 原名及变名合法工程 | 全部APPLICATION槽和源码一次接纳，SERVICE桥仍归生成器 |
| 再生成 | 编辑每份live源 | sealed逐字节等于用户输入；live不变 |
| stale | 改preview、manifest或任一源 | 拒绝旧确认；已有输入／输出不变 |
| 非法成员 | 缺失、未知、重复、错槽／路径、symlink | 来源定位拒绝；不留下部分工程 |
| 发布竞态 | 中途占位、manifest或源被外部替换 | 保留外部字节；回滚本次文件，冲突恢复位置准确 |
| 回归 | 旧单槽及通用配置 | 原初始化、读取、保存和生成规则保持 |

</frozen-after-approval>

## Code Map

- `core/src/integration/ecu.rs`：plural slots已存在；seed仍legacy-only。复用真实SymbolContract和owner，不重读XML。
- `core/src/arxml/{application,project,persistence}.rs`：扩展preview与严格成员重开；复用master的capture／verify／no-clobber发布与恢复，关闭read-then-unlink竞态，不复制平台实现。
- `core/src/prepared.rs`、`generator/delivery`：plural准备及字节guard已交付，验证正常workspace接线。
- `ui/src/workbench/projectTypes.ts`、`PreviewDialogs.tsx`与既有消费者tests：同步slots数组，旧profile返回一个槽，不靠前端拼身份。
- `core/tests/{multi_component_contracts.rs,support/epic7_workspace.rs}`：正常Workspace、真实文件事务、再生成与单槽回归；复用已有private closure故障注入。

## Tasks & Acceptance

- [x] `integration/ecu.rs`：按可信符号派生全部create-only源码骨架，保持准确void／参数／owner及旧seed字节。
- [x] `arxml/application.rs`、`project.rs`、必要`persistence.rs`：全槽预览、成员授权、原子发布／回滚及重开。
- [x] 核心、DTO、既有dialog和消费者tests共同迁移；完整源码集合绑定revision。
- [x] 正常回归与真实multi源码／构建／c-check；结果写本规格。

Given全部可信槽，when初始化并编辑各源后重开、预览和生成，then AC-9的身份、字节、拒绝与原子性成立。Given发布中真实竞争路径，when事务拒绝或回滚，then外部文件不被删除，原manifest与内存基线准确，必要恢复证据可定位。

## Design Notes

新scaffold仅定义真实APPLICATION入口，不调用业务Read／Write／Call。void周期入口空体；C/S保持实际void签名，OUT给确定零值，INOUT保留输入，避免未初始化输出；不把noErrors服务器改成Std_ReturnType。生成Dcm桥不成为用户槽。旧seed保持原行为。初始化公共DTO统一为slots，最小dialog迁移属于本故事的必要调用方同步；完整交互留8.4。无未定产品问题，无外部副作用；仅按用户已授权范围创建本地工程。

## Implementation Notes

- 真实 `SymbolContract.definition_owner` 与组件 full type identity 共同派生全部 APPLICATION create-only 骨架；周期入口为空，noErrors server 保持 void，uint32／固定四字节 OUT 初始化为零，INOUT 不写入。SERVICE Dcm 桥保持生成器归属；旧 single seed 返回原有模板字节。
- 初始化 DTO 统一为 `slots` 数组，revision 覆盖全部槽、文件、manifest 与规则／定义身份。既有 dialog 逐槽展示真实身份、头文件与入口并可检查每份 seed；Rust、前端与桌面 single 场景消费者已同步，single 明确返回一个槽。
- manifest 重开严格核对全部真实 producer／path，拒绝缺失、未知、重复、错槽及 symlink。多组件源码归属只核对应用结构与类型、实例、入口关系；目标 transport 资格仍在正常 prepare／generate 核定。合法 DEFERRED 配置可以重开，生成保持拒绝。
- manifest 发布复用既有 persistence capture／verify／no-clobber 原语；回滚用户源先捕获再核对，不对已读取的 live 路径直接删除。发布前后核对全部配置与已安装源；失败保留外部替换字节及原内存基线。成功后的恢复清理冲突返回实际仍存在的 backup／captured 路径并保持已发布基线。
- 正常 Workspace 回归覆盖原名与变名初始化、全部用户源 CRLF／UTF-8 字节、重开、sealed 再生成、逐份 live／manifest／配置 stale guard、非法成员及 symlink。真实生产 OS／CAN／DID 向量改从 Workspace 初始化、用户编辑、重开及 prepare／generate 入口运行。

## Spec Change Log

## Review Triage Log

| 发现 | verdict | 核验依据与处置 |
| --- | --- | --- |
| B1 manifest恢复失败后仍删除源 | high | persistence::restore_backup在已存在rollback目录时直接失败，application失败分支无条件清源确会留下引用缺失源的新manifest。与E1同根因，patch：记录真实manifest发布，仅在旧manifest已恢复时清理新源，否则保留源并提示恢复位置；增加实际阻塞恢复目录向量。 |
| B2缺少manifest恢复故障注入 | false | 既有私有初始化publish回调可在manifest发布后创建backup.with_extension("rollback")，直接使真实restore_backup失败；无需新增共享恢复接口。B1的实际向量覆盖这一机制。 |
| B3长producer／入口换行 | medium | CSS mono只定义字体；新full实例producer及长入口无换行，确会撑宽preview。patch：复用path-text；真实窄窗进一步由8.4验收。 |
| B4确认测试未触达控制器IPC | medium | 新组件测试只验证dialog回调次数；实际projectActions完整传递preview，但缺少该全槽边界的直接断言。patch：既有控制器测试核对完整preview／revision经正常call传递。 |
| B5缺少实际多槽桌面IPC | false | 未宣称原生GUI通过；冻结意图明确完整编辑／IPC体验由8.4接续。当前configuration接收核心DTO并直接传给Operation.initialize_application，projectActions传完整对象；后端正常9项单测与构建通过。整体任务仍须8.4真实原生入口验收，不能以本故事局部证据收尾R6。 |
| B6实际源码缺失／目录替换未直接测试 | medium | manifest缺失成员与symlink测试不能代替完整成员清单下的磁盘缺失／目录替换。patch：正常Workspace与已准备生成拒绝向量，核对其他源码／既有输出不变。 |
| B7原地写入硬链接源未直接测试 | medium | 替换文件与写入同inode不同；当前capture核对能够保留外部字节，但需直接证明stage共享inode时的行为。patch：发布前后真实原地写入向量，核对外部新字节与事务回滚。 |
| B8未来类型扩展造成四次写入 | false | 当前真实multi类型检查仅uint32与固定uint8[4]，IN/OUT/INOUT方向亦由检查闭合；可信计划不能产生第三种OUT native type。未来类型扩展不是当前已示明坏状态，不为未授权变体新增guard。 |
| B9缺少独立官方签名预期 | false | 既有application-contract及compliance-references已按R24-11 RTE§5.7.5.6/SWS_Rte_08913、参数policy与MemMap00020/28/29/32核对；新C消费者独立硬编码void函数指针与scalar／array签名，OUT零／INOUT保持为明确scaffold设计，不从SymbolContract自动生成唯一预期。本build规格编辑亦不作为修复。 |
| B10spec／sprint状态和旧待验证描述 | false | 当前Step4要求spec=in-review，sprint到Step5才review；不是状态矛盾。实施侧与整合侧证据分时保留，历史部分结果未被改写为通过；最终验证在Step5统一。本build规格编辑不作为修复。 |
| E1发布后失败且恢复目录被占 | high | 与B1相同真实早返回路径；单独保留本行，归同根因patch，不能删除新manifest仍引用的源码。 |
| V1ownership-only有效异属data member缺少拒绝测试 | medium | 采纳verification-gap预核验证据；完整transport拒绝不能替代新ownership-only分支。patch：正常已初始化manifest重开，使用可解析且DEST正确的异属variable-data-prototype，独立断言SIGNAL_MAPPING并保留原始输入。 |

## Verification

正常Cargo default／native相关源码与事务测试、旧builtin回归、npm lint／test／build、对应quality／Clippy／assets；全部SOURCE guard从正常prepare／generate入口检查。新骨架实际生成工程严格编译链接，运行真实用户源再生成向量；实际受影响multi profile执行c-check，分别记录完整性、诊断和人工未评估范围。整合基准为1eb81af87ea2248a33824800e8fa9a71eb056df9；整合后sealed_multi三项已通过（34.97s），不以文档或片段替代真实执行。

### 实施侧已执行检查

- `cargo test --locked --manifest-path core/Cargo.toml --lib arxml::`：16 项通过（4.32s），其中 5 项初始化事务测试覆盖后续源占位、manifest capture／publication 替换、源发布前后替换、目录替换及成功发布后的准确恢复路径；既有 11 项保存／恢复回归通过。
- `cargo test --locked --manifest-path core/Cargo.toml --test multi_component_contracts multi_workspace`：3 项通过（34.50s），覆盖原名／变名、全部 SOURCE guard、非法成员、通用配置重开与 symlink。
- `cargo test --locked --manifest-path core/Cargo.toml --test builtin builtin_application`：旧单槽初始化 2 项通过（5.39s）。
- 受控 Linux `native-tests` 的 `initialized_multi_scaffolds_compile_link_and_initialize_only_out_arguments`：通过（10.69s）。正常离线工程严格构建后，实际交付源码与头文件再次编译链接并运行 OUT=0、INOUT=17 保持、四字节 OUT 清零和 void server 类型向量。
- 受控 Linux `native-tests` 的 `sealed_multi_project_builds_and_runs_production_owner`：通过（49.11s），4 个原名／变名／不同 controller ID／诊断时序变体从正常 Workspace 完成真实用户源生成、构建及 OS／CAN／DID 执行。
- 前端 path-display／localization 消费者：2 文件、20 项通过（2.14s），包含简中／英文多槽身份、长名称、逐文件预览与完整数组复制。桌面消费者 `node --check` 与 `git diff --check` 通过；本记录不声明原生 GUI 已执行。
- 全量 Cargo／UI／quality／Clippy／assets 与实际 multi `c-check` 由整合验证补记；未把工具链通过或局部测试写成完整 MISRA 符合。

### 整合侧独立验证

- 主代理完整阅读基准以来包含未跟踪规格的统一 diff（81,247 bytes），逐项核对实施任务及冻结矩阵。正常完整 default Cargo：32 unit（5.57s）、71 builtin（21.09s）、2 c_analysis（13.28s）、53 multi（42.72s），全部通过，零失败／忽略／过滤。
- 正常公开核心 API 独立复现旧多槽初始化拒绝后，同一入口已完成初始化→manifest 重开→preview→generate，三份真实源码与 sealed 快照逐字节相等。实际初始化骨架的包内 `tools/ecu-tool.py build --mode host-batch` 严格 C99 编译与链接通过；没有把空骨架视为业务算法。
- npm lint／生产 build 通过；全量 UI 11 文件、96 项通过（3.39s），之后新增的多槽测试由实施侧再执行 2 文件、20 项通过，后续全量复验补记。生产 build 保留既有 bundle 大于 500kB 提示。core quality 检查 7 文件通过；项目既有 Clippy correctness／suspicious 门禁在 all-targets／native-tests 通过（6.72s），assets check 零变更，diff check 通过。
- 受影响实际初始化 multi profile 正常 c-check：固定 Cppcheck 2.21.0／提交 e73bf44c3e49686b7495fab352d03a6c6075516b、已核对同源 addons 与 GCC13，52 unique translation units，host-batch／legacy-probe 各 51 units 完成；summary.error=null，各 program exit1，源码 passed=false。2,620 diagnostics（2,586 style／30 warning／2 error／1 portability／1 information），692 adopted。与8.2 reviewed multi 按完整 id／severity／message／locations 比较新增非 style 诊断为零；实际三份骨架各有一条 R20.1 的 STOP MemMap include 提示，保留必要段切换及原始诊断，不自动申请偏离。221 项完整人工评估未完成，扫描完整不等于完整 MISRA 符合。

冻结矩阵实际运行覆盖：初始化／变名及真实字节再生成由 `multi_workspace_initialization_reopen_and_regeneration_preserve_every_user_source`；stale 覆盖全部 live 成员、manifest、配置及改动 preview；非法成员由 `multi_workspace_reopen_rejects_incomplete_unknown_and_wrong_slot_membership` 与 `multi_workspace_initialization_and_reopen_refuse_symlink_members`；发布竞态／恢复证据由5项 `arxml::application::tests`；旧 single／通用配置由71项 builtin 与 multi DEFERRED 重开向量。所有所引测试均已运行通过，没有用未运行测试代替覆盖。

整合补验：完整native multi 58项通过（78.81s），零failed／ignored／filtered；最终全量UI 11文件／96项通过（2.80s）；正常src-tauri Cargo 9项通过（0.54s，编译30.29s），不冒充原生GUI运行。

### 审查修复后的最终验证

三层审查共12条发现逐项裁决；B1/E1真实恢复失败同根因、B3/B4/B6/B7及V1均修复，未新增公共恢复API／业务算法或额外台账。主代理完整阅读原83,710-byte审查diff及29,796-byte补丁delta，核对最终99,937-byte变化集合。新增实际恢复阻塞向量保留全部新manifest引用源、外部原地写入字节、原内存基线与准确backup／recovery位置；源缺失／目录替换保留其他live与全部sealed字节，ownership-only异属member以SIGNAL_MAPPING拒绝；控制器将完整preview原样送入IPC，长身份复用path-text。

最终正常default Cargo：33 unit（8.17s）、71 builtin（19.04s）、2 c_analysis（13.11s）、55 multi（43.18s）；完整native multi 60项（80.81s），均零failed／ignored／filtered。最终npm lint／build通过，11 UI文件／97项通过（3.71s）；core quality7文件、UI quality4文件、项目all-targets/native Clippy通过（3.57s），assets零变更，diffcheck通过；src-tauri9项通过（0.56s，编译7.89s）。实际生成C模板及profile字节未被审查补丁改变，复用本故事52units完整c-check原始结果，仍passed=false，未将诊断吸收为通过。

本故事AC-9核心源码保护完成；真实原生编辑／IPC、窄窗口／主题操作由已冻结8.4承接，独立搬移／离线交接由8.5承接。Windows、MCU、完整MISRA人工评估未被本轮Linux结果替代，R11未实施；无本故事未处理的已确认修复项。
