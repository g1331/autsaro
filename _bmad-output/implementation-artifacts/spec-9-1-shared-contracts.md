---
title: 'Story 9.1：共享契约与逐模块进入依据'
type: 'chore'
created: '2026-10-10'
status: 'done'
baseline_commit: '680570a55b7b78d3930a654824c572147f276377'
route: 'dispatch'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-9-context.md'
  - '{project-root}/_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md'
  - '{project-root}/_bmad-output/specs/spec-runtime-codegen-quality/generator-practices.md'
---

<frozen-after-approval reason="maintainer-owned intent — changes require authorized delegated approval or maintainer renegotiation">

## Intent

当前 Epic 9 尚缺精确交付选集、接缝责任及逐模块进入证据。本 Story 固定维护者可定位的共享契约并以独立 consumer／碰撞护栏保护后续整改，不承担模块迁移或完整标准审计。

## Boundaries & Constraints

保持基线 680570a55b7b78d3930a654824c572147f276377；三族 profile、双 target、各 main 独立。execution 拥有 batch/output/receipt，OS/backend 拥有调度及生成 SchM；保留异步锁。COM 与 RTE 状态分离，glue 唯一生成者。保留 ABI、wire、旧 seal、用户源及历史失败。仅修改本开发规格与正常测试 fixtures/suites；不改产品 emitter/runtime/target/资产摘要，不提交或发布，不实施9.2–9.20。固定R24-11及MISRA修订链；无合法正文不作规范裁决。Build CHECKPOINT 1 已由 dev-user/dev-pm 在维护者授权下代理批准，并选择 Approve and continue + Keep full spec；非直接人类回复或标准偏离批准。

## I/O & Edge-Case Matrix

| 输入／状态 | 预期 |
| --- | --- |
| 三族、目标、main及可选配置 | 从实际选集定位；互斥 producer 不混链，六样本不等于全覆盖 |
| 有效生成头与独立C consumer | 签名编译、链接及回调分派成立 |
| 参数遮蔽／共享event周期或phase冲突 | 生成前拒绝，原输入与既有输出保持 |
| 缺原文／授权或真实工具 | 分别记录未核查与缺件，只阻塞对应裁决／实施；不伪造pass |

</frozen-after-approval>

## Code Map

- `core/build.rs:139–268`、`resources.rs::AssetInventory::selected`、`prepared.rs::deliver_path/finish`：合并 `runtime/contracts/{assets-v1,bsw-v1}.json` 与两份FreeRTOS manifest；按target/profile筛选并形成交付闭包。
- `core/src/target.rs::BuildTarget::spec`、`tools/python/src/ecu_tools/build.py::build`：Windows/Linux port、patch、库及唯一main。host为`runtime/src/ecu_host_main.c`；ECU batch/probe分别为`runtime/ecu/src/{ecu_host_batch,ecu_probe}.c`，test用probe或外部control源并加测试宏。
- `core/src/integration/ecu.rs::source_assets`：multi保留Can/lock/status，替换其余legacy BSW及execution/SchM；不能混链原producer。`multi_ecu.rs`生成容量、polling宏及配置；`plan.rs:586–660`核定无DID/network-only transport；`generator/render.rs::render_host_profile`拒绝Linux Security。`runtime/{README.md,os/README.md}`说明支持边界。
- `integration/{plan,contracts,configuration,native_configuration,multi_bsw}.rs`：唯一验证计划、声明、配置及实际consumer闭包；Can controller/HOH语义和routing方向handle独立。
- `integration/plan.rs:633–650`及`multi_rte.rs:283–316`：`Rte_COMCbk/Rte_COMCbkRxTOut`声明`include/Rte_Com.h`，定义`src/Rte.c`；`multi_ecu.rs`接入Com配置，实际consumer为`runtime/multi/src/Com.c`。
- `runtime/ecu/include/Ecu_Target.h`及`Ecu_Target.c`：ticket/output/receipt与排空边界；`runtime/os/src/Os_Time.c`消费execution接口。`integration/{ecu,multi_ecu}.rs`及`runtime/ecu/src/Ecu_SchM.c`核定OS/SchM生成来源。
- `integration/multi.rs:1568–1722`及`schedule.rs:596–619,729–745`：名字及共享event护栏。`multi.rs:1786`仍记录legacy Dcm consumer，而multi实际选`runtime/multi/src/Dcm.c`：来源元数据缺口，不在本授权内改产品。
- `core/tests/{multi_component_contracts,multi_runtime_contracts}.rs`及`fixtures/multi-runtime/`：独立C签名／真实下层行为；前者保留参数shadowing与event回归；后者必须启用`native-tests`，否则零测试。
- 资料：`core/resources/official.json`、`docs/official/R24-11/README.md`及`spec-multi-component-scheduling/compliance-references.md:9–33`定位来源／期望摘要。主工作区三份ZIP可供显式配置，candidate无默认原件；模块PDF完整正文与MISRA授权主文未取得。历史十二摘要passed=false仍在`architecture/epic-9/BASELINE.md`，不代表本轮扫描。

## Tasks & Acceptance

- [x] 本规格补充选集与逐模块进入证据：依据上述真实manifest记录include顺序、宏、patch和可选配置；引用既有work-packages给9.2–9.16明确直接依赖、独立向量、工具、兼容授权和退出条件，不改kernel派生companions。
- [x] `core/tests/multi_component_contracts.rs`及正常fixture：增加生成`Rte_Com.h`独立translation unit，固定两个`void(CbkHandleIdType)`预期并验证实际glue链接／分派；复用已有标量／C/S与碰撞测试，不从ABI清单制造唯一预期。
- [x] `core/tests/multi_runtime_contracts.rs`：复用真实COM/CanIf/routing/TP/Dcm正反状态向量，记录实际执行及未覆盖配置；不重复新增代理测试系统。
- [x] 本规格记录公开契约缺口的所有消费者、旧包兼容／显式版本选项和授权条件；标准整改保持下游阻塞。本次发现Dcm来源身份不一致明确交9.15修复和独立证明。
- [x] 按既有Build snapshot完成隔离实现、开发检查、内置独立review及step05呈现，准备同卡移交dev-tester。依据PM裁决t_dabe832f有条件恢复9.1入口工作；spec done仅表示本Build入口工作完成，sprint=review及同卡Tester仍为必需出口，未作Story最终交付批准。

验收沿`epics.md:1139–1149`：
1. Given取得共享版本与实际选集，When核对固定资料，Then模块／直接依赖、支持配置、原文来源／摘要及confirmed/existing/unverified/missing可定位，历史失败保留。
2. Given定位声明和producer，When核对接缝，Thenexecution/OS/COM/RTE责任唯一，独立consumer及有效碰撞反例经正常入口成立。
3. Given发现公开整改需求，When分析全部消费者和旧包，Then迁移／授权条件与逐模块准备范围可执行，缺依据实施保持阻塞。

## Implementation Notes

### 2026-10-11 有限验收状态消费（t_cd90481e）

本候选`/root/workspace/autsaro/.worktrees/t_cd90481e`、branch `autsaro/t_cd90481e-bmad-9.1`，HEAD=`680570a55b7b78d3930a654824c572147f276377`。原候选`/root/workspace/autsaro/.worktrees/t_40909b7c`保留且未写。复用Tester t_40909b7c的7项聚焦真实测试、Build三层内部review/完整开发回归以及PM t_17240366三AC/客户要求核对；Rust test、C fixture及epic-context字节与原证据相同，本次没有新产品实现或产品全套测试。

授权writer使用实际installed `bmad-sprint-planning/scripts/sprint_plan.py generate` merge及`--set '9-1-固定共享契约与逐模块进入依据=done'`，先dry-run再写，未使用`--fresh`。独立比较全部development_status/action_items及评论，仅该key由review变done和实际last_updated更新；脚本遗漏原Epic8两条内联评论后按原字节补回，未修改installer脚本/其他状态。c-check exit1/passed=false/2607原始诊断、合法模块正文/MISRA221人工缺口、Dcm来源身份与所有下游限制完整保留，不是标准符合结论或后续Story放行。

本轮只在五个supportedworkflow team TOML写阶段指令，不是可执行硬门禁引擎；真实resolver/Build冻结渲染与人工正负场景走查分开记录于原卡。按PM转达Boss最新授权，整体工程出口为本地提交、非破坏集成canonical master及master读回，无远程权限；本卡先配置/状态落地→同卡Tester，独立验收前不commit/merge，后续独立Engineer集成阶段承担Git写入及针对性独立复核。当前仅阶段完成/待授权本地集成，不将此未提交worktree称整体交付；不push/PR/deploy或启动新业务。

### 实际选集与共享身份（本轮核对，不是标准符合结论）

HEAD 实际为 `680570a55b7b78d3930a654824c572147f276377`，与保留的 baseline 一致。只新增正常测试和本节证据，不改 emitter、runtime、target、ABI 或资产摘要。派发时已存在的 `sprint-status.yaml` 改动和 `epic-9-context.md` 保持原状；本实现不负责推进 sprint。固定 CP／FO R24-11、C99、MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2，不能以编译成功裁决标准义务。

本轮读取并计算以下共享清单 SHA-256；逐文件身份仍取清单各项的摘要，不以清单摘要替代源码摘要：

- `runtime/contracts/assets-v1.json`：`8c4755d4aa95d67b69e0b3381f7aa4be1873ea18bf8407f8241c7f1efe69f79d`。
- `runtime/contracts/bsw-v1.json`：`6cf1b53133ced3764ed0c906ed92cb8946638b03126624b5ca59986dbfd1644e`；其 scope 明确只是当前 legacy host 声明／producer 身份，不是标准签名认证。
- `third_party/freertos/source-manifest.json`：`1394bc9b093d1d09f41992e34240d7b442e0cbbd459cb83b4f138e99feefbb8b`。
- `third_party/freertos/posix-source-manifest.json`：`09d76410dacb89ad9251bca01991046fb6a6f99fb87c0818c141e083b0bdc677`。
- 两份 kernel manifest 均固定 V11.3.1、commit `054e14f3397023aa83813a65aa065fc4597d481b`、archive SHA-256 `dd832f699e6ebee00366d7c737e259860d9d1fd5f2c2311c90dff16e8cfbcf61`。未修改派生 companions、原件或 patch。

**选集规则：**`core/build.rs` 合并上述来源后，`AssetInventory::selected(target, profile)` 按真实 target/profile 身份过滤，`prepared::deliver_path` 映射交付路径；最后 `finish` 从已组装文件的 `src/*.c`、`os/src/**/*.c` 和根 `Ecu_Config.c` 形成 `target.json.sources`，单独排除 batch/probe main。所有已选 `.h` 进入 `files.list`／`files.sha256`，不是只看 `target.json.sources`。同路径 generated 产物及实际 source owner 须沿 `source_assets`／ownership 核对，不能把同名源误认为同一 producer。生成头包括配置、RTE、SchM、MemMap、OS/ARTI；用户应用源是 create-only 初始化后冻结的 live 快照，不是 kernel 或 emitter 的可覆写工件。

本轮 `c_analysis` 实际在 Linux 重生成六个正常封存样本：`signals`、`diagnostic`、`capacity`、`standard-ecu`、`user-application`、`multi-component`。前三者都是 legacy host；后两种单组件样本使用同一 ECU 族，user 样本改变用户应用算法；multi 样本包含 Ingress／Process／Observe 和 DID 服务。下列选集从实际 `target.json` 读回，Windows 差异另从 `BuildTarget::spec`／`finish`／manifest 核对，**不是 Windows 运行证据**。

- **legacy host：**根 `Ecu_Config.c`；`src/` 下 `Can`、`CanIf`、`CanTp`、`Can_Execution`、`Can_HostLock`、`Com`、`Dcm`、`Dcm_Execution`、`Dem`、`Ecu_Clock`、`Ecu_Runtime`、`Ecu_Status`、`LSduR`、`NvM`、`NvM_HostStorage`、`Os`、`PduR`、`Rte`、`Security`、`ecu_host_main`（均 `.c`）。这些是已交付编译选集，不意味着每个可选模块在无诊断配置中活跃。主机 `Os.c` 不与 automotive OS 混链，host clock／execution 不与 ECU execution 混链。
- **单组件 ECU：**`src/` 下 `Application`、`Can`、`CanIf`、`CanTp`、`Com`、`Dcm`、`Dem`、`Ecu_Config`、`Ecu_Execution`、`Ecu_HostBatch`、`Ecu_HostBridge`、`Ecu_OsHooks`、`Ecu_SchM`、`Ecu_Status`、`Ecu_Target`、`LSduR`、`NvM`、`NvM_HostStorage`、`PduR`、`Rte`、`Rte_OsService`、`Security`；再加下面唯一 automotive OS／kernel 选集。单组件 `include/Com.h` 是生成桥，原 legacy 头在 `bsw-origin/include/Com.h`，不能凭同名替换。
- **多组件 ECU：**与单组件复用 automotive OS／kernel、ECU target/bridge/batch/hooks 和真实 Can／Can_HostLock／Ecu_Status；`source_assets` 排除其余 `runtime/src/` legacy producers，排除手写 ECU `Ecu_Execution.c`／`Ecu_SchM.c`。`src/` 实际选 `BswM`、`Can`、`CanIf`、`CanTp`、`Can_HostLock`、`Com`、`ComM`、`Dcm`、`Det`、`Det_Host`、`Ecu_Config`、`Ecu_Execution`、`Ecu_HostBatch`、`Ecu_HostBridge`、`Ecu_HostBusSM`、`Ecu_OsHooks`、`Ecu_RuntimeConfig`、`Ecu_Status`、`Ecu_Target`、`Ingress`、`LSduR`、`LSduR_PBcfg`、`Observe`、`PduR`、`PduR_PBcfg`、`Process`、`Rte`、`Rte_OsService`、`SchM`、`SchM_CanIf_Host`、`SchM_Diagnostic_Host`、`SchM_Mode_Host`（均 `.c`）。`multi_ecu` 唯一生成 execution／SchM／模块配置 glue，`multi_rte` 唯一生成 RTE 存储／访问／COM 通知实现。不得补链 legacy NvM/Dem/Security 或手写 Rte/Ecu_SchM 冒充同一配置。
- **ECU 公共 OS：**`os/src/` 下 `Arti`、`Os`、`Os_Arti`、`Os_ArtiServices`、`Os_Backend`、`Os_Error`、`Os_HostServices`、`Os_Interrupt`、`Os_Mailbox`、`Os_Schedule`、`Os_Time`、`Os_Vector`；Linux 另选 `os/src/host/linux/{Os_HostLinux,Os_StackLinux}.c`，Windows 另选 `os/src/Os_Stack.c` 和 `os/src/host/windows/Os_HostWindows.c`。kernel 独立编译 `tasks.c`、`list.c`、`queue.c` 和唯一目标 port（Windows `portable/MSVC-MingW/port.c`；Linux `portable/ThirdParty/GCC/Posix/port.c`）。POSIX utils 原件也在 manifest／交付闭包中，不能擅自增加为第二 port producer。

**include／宏／链接：**host include 顺序严格为 `.`、`include`，flags 为 `-std=c99 -O1 -g -Wall -Wextra -Werror -pedantic`，无 kernel/patch；Windows host 链 `bcrypt`，Linux host 无附加 library。ECU 顺序为 `.`、`include`、`os`、`os/include`、`os/src`、`kernel/include`，Linux 再接 `os/src/host/linux`、`kernel/portable/ThirdParty/GCC/Posix`，Windows 再接 `kernel/portable/MSVC-MingW`。公共 ECU flags 为 `-std=c99 -O1 -g -Wall -Wextra -Werror`，Linux 追加 `-D_GNU_SOURCE -pthread`／链 `pthread`；Windows 链 `winmm`、`bcrypt`。构建还显式设置 source/build 的 `-ffile-prefix-map`；kernel include 指向外部 build 私有副本，不指向可变原件。

Windows 按 `target.rs::WINDOWS_PATCHES` 的明确顺序应用 `0001-controlled-host-lifecycle`、`0002-native-stack`、`0003-activation-ready-policy`、`0004-windows-event-namespace`、`0005-category2-resource-gate`、`0006-input-handler-ownership`、`0007-controlled-tick`、`0008-task-hook-boundaries`、`0009-interrupt-source-control`、`0010-native-interrupt-identities`、`0011-isr-phase-before-mutex-release`、`0012-nested-interrupt-dispatch`、`0013-interrupt-source-repetition`、`0014-relocatable-interrupt-vectors`（均 `runtime/os/patches/*.patch`）。Linux 顺序为公共 `0003`（仅 `tasks.c`、`include/task.h`、`include/FreeRTOS.h`）、`0007`（仅 `tasks.c`）、`0008`（仅 `tasks.c`）、`linux/0001-controlled-posix-port.patch`（全补丁）。`ecu_tools.build` 在私有 kernel 目录隔离外层 Git，先 check 再 apply，封存源前后核对；不修改选择或取消 include 过滤。

host 容量头固定 `ECU_MAX_PDU_PAYLOAD=256u`；单组件 ECU 为 `64u`。multi 的 `Ecu_ProfileConfig.h` 由计划生成 `ECU_MULTI_COMPONENT=1`、`CAN_HOST_QUEUED_COMPLETION=1`、`CAN_ZERO_LENGTH_SUPPORTED=1`、`CAN_RX_POLLING=1`、`CAN_TX_POLLING=1`、`CAN_BUSOFF_POLLING=1` 及 source-derived `CAN_CONTROLLER_ID`／`CAN_CANIF_CONTROLLER_ID`／`CAN_TX_HOH`，本样本分别为 `0u/0u/1u`。fixture 的非等 ID `7u/9u/17u` 仅用于机制隔离，不证明单控制器 source profile 的连续编号是否合法。multi 的 capacity／event／Rx deadline 宏由 `Ecu_TargetConfig.h` 唯一生成，types 由真实 EcuC 最大值决定；不可固定成全配置常量。

**main 和可选配置：**host 仅链接 `src/ecu_host_main.c`；两种 ECU 族各自的 source map 加唯一 `src/ecu_host_batch.c`（production）或 `src/ecu_probe.c`（probe）；`test` 使用 probe 或外部控制源，并只在该模式加 `ECU_TARGET_TESTS`／`OS_HOST_FAILURE_TESTS`。production 拒绝外部控制源及测试宏，batch/probe 是不同完整程序，不能合并 CTU。六样本不覆盖 Windows Security、reset routine、所有 DTC 错误及配置组合；本轮 native multi suite 另外实际覆盖无应用 DID／F186 与完整 network-only／无诊断的封存工程，以及拒绝半路由。既有 legacy write/routine/DTC/Security 语义保留，Linux 明确拒绝 BCrypt Security。multi 首批保持单控制器 Classical CAN、normal physical TP、无 metadata/FD/dynamic ID、无 CanIf Tx buffering、单 CDD NM NONE channel、无 PNC／动态 user／NvM inhibition；不将 CDD 声称为 CanSM/Nm。group/DM、首个 timeout=0、manual trigger、polling、可选 DID 的其他支持组合须由对应 Story 增补正常入口，不把本轮 fixture 视为全部配置证明。

### 原文与结论类别

- **confirmed（本轮可验证事实）：**上述源码／清单／编译选集、唯一声明/producer 的位置、Linux 独立 consumer 实际执行、生成前名字及共享 event 护栏；这不是 confirmed 标准符合。
- **existing（历史依据）：**[Epic 8 规范来源](../specs/spec-multi-component-scheduling/compliance-references.md#r24-11-契约核定与直接依赖差距)表中二十一份 PDF 的 URL、历史 SHA-256、相关条款；[Epic 4 契约](../planning-artifacts/architecture/epic-4/R24-11-CONTRACT.md)及[当前进入范围](../specs/spec-runtime-codegen-quality/work-packages.md)。模块启动须取得相同正文并再次核对，不把历史“已取得”写成本 candidate 可读。
- **unverified：**各模块全部适用 SWS/ECUC、BSW General、BSWMD/SWCT、MemMap 义务及当前实现完整差距，MISRA 221 项适用性／人工裁决、真实 Windows 与 MCU；本 Story 不做缺原文的逐条标准判定。
- **missing：**本 candidate 没有官方模块 PDF 或授权 MISRA 主文。主工作区存在的 R24-11 XSD/MOD/samples ZIP 实际分别为 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`、`df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`、`dd55faad0bdc22181dd8fddf3927ad59b36c88ff9a7848f05f445c9b141fd701`。前两者匹配 `official.json`，samples 在该清单未固定摘要，不能升格为 pinned oracle；仅供未来显式设置档案路径，不复制原件或更改默认入口。初次缺Cppcheck 2.21.0是历史失败；本轮已取得固定工具并执行真实分析，当前源码失败见“本轮实际结果”，不再称工具缺件。

保留 [Epic 9 BASELINE](../planning-artifacts/architecture/epic-9/BASELINE.md)中 `a09955c`／CI `38038608981` 的双平台六样本十二摘要 `error=null, passed=false` 和严格门未自动必需的事实。历史 PDF/TLS 失败、人工未评估及源码失败不因本轮绿色 Cargo 改写；整体进入判断仍为 CONCERNS。

### 接缝责任、消费者及兼容授权

1. **execution／OS：**`Ecu_Target.h` 的 copied frame、ticket/epoch、output、confirmation 和 batch completion 由 ECU execution owner 持有；`Ecu_Target.c`、`Ecu_HostBatch.c`、`Ecu_HostBridge.c`、选中 `Ecu_Execution.c`、两种 main、Can execution 及 `Os_Time.c`／mailbox／Waiting hook 是同步迁移闭包。OS/backend/固定 port 决定任务、tick、Waiting 和 quiescence；execution 不引入第二调度器。COMMIT_OK 只在真实 owner Waiting 且 input/tick/output/confirmation 排空后发布，失败 output 不回滚或伪造确认。私有 test main 不替代生产批入口。
2. **COM／RTE：**COM 拥有 I-PDU、group、deadline、发送/接收缓冲；RTE 拥有实例 local 初值／发布／last/freshness 与运行生命周期。`include/Rte_Com.h` 声明两个 `void(CbkHandleIdType)` 通知，`multi_rte` 的 `src/Rte.c` 唯一定义；`multi_ecu` 生成 Com 配置函数指针，真实 `runtime/multi/src/Com.c` 调用，应用 `Rte_<component>.h` Read 消费结果。`multi_com`／plan 提供 source-derived handle，其他 emitter 不另造通知。COM group stop 与下层 CAN TX 失败不混同，local 数据不继承 CAN gate。
3. **共享声明／类型：**multi 的 `runtime/multi/include/{Platform_Types,Std_Types,ComStack_Types}.h` 为清单固定资产，交付到 `include/`；`ComStack_Cfg.h` 的 PduId/PduLength 选择从 EcuC 计划派生，`multi_ecu` 接入唯一 `Ecu_ProfileConfig.h`，不能以 fixture 固定 uint8/uint16 代替生成来源。RTE 的组件接口/类型由 contracts 与 multi emitter 从同一 validated plan 产生；`Rte_Com.h` 显式包含 `ComStack_Types.h`，不要求独立 consumer 自己补造 CbkHandleIdType。直接消费者覆盖所有所选通信模块、配置函数指针、RTE、应用源、公开 C 测试及离线交付，生成 glue 不重新决定类型或 handle。
4. **SchM／异步：**OS 服务/调度及 `multi_ecu` 生成 SchM lifecycle 是唯一来源；手写 `Ecu_SchM.c` 只属于单组件。Can/CanIf/diagnostic/mode 的真实 host SchM 锁继续保护异步 producer／confirmation，不能因 owner-adapter fixture 不建线程而删除锁。RTE glue 不拥有平台线程或 FreeRTOS 调度。
5. **标准名称与 legacy 差距：**当前 `bsw-v1` 中 legacy `CanIf_Init(const EcuConfig*)`／frame-index transmit、`Com_*` host 签名、`PduR_*` host routing、`CanTp_*` 时钟/admission、`Dcm_*` whole-message 与新 multi 公开类型/配置不是可互换 ABI。旧单组件 RTE server 返回值也与 multi 无 possibleErrors 的 `void` server 不同。9.2–9.8/9.15 启动须从合法正文裁定各适用标准差距，再同步两套头/实现、生成模板/plan/配置、Com/PduR/LSduR/CanIf/CanTp/Dcm/RTE 全部直接调用方、host adapters、应用初始化/live producer、普通 C/Python consumers 和 ABI/交付工具。NvM/Dem/Security 同样先审 host 合同，不能由绿色老测试宣称标准化。
6. **已确认来源缺口：**`integration/multi.rs` 对无 event 的服务把 consumer 记录为 `runtime/src/Dcm.c`，实际 multi 选集使用 `runtime/multi/src/Dcm.c`。这是 producer/consumer 元数据身份不一致，不能只更新文档或资产 hash 消除；交 9.15 在合法依赖确定后修来源，并以实际选集与独立 DID compile/link/run 证明 consumer 路径和符号唯一，未授权本 Story 修改产品。
7. **迁移选项／授权：**默认保留旧包精确字节与 ABI，给新版本/profile 明确标识的标准合同；若兼容适配确实可保持语义，可在明确私有 host 边界提供 adapter，不能用 typedef/改名冒充新语义。若必须 breaking change，先取得维护者针对签名、layout、handle、wire、用户源与旧包退出策略的明确授权，再同步上述闭包及 ABI/清单/工具/说明。旧 seal 不自动升级，reopen 以实际结构身份而非“能否通过新生成”识别历史 profile；用户修改不得隐式重写。旧 v1 与 v2 offline/reopen consumers、`ecu_tools.{build,verify,reference,workbench_v2}`、ownership/fingerprint/seal 及对应 Rust/Python tests 都在共享改动回归闭包。当前批准仅覆盖 9.1，本节不授予任何标准化、偏离、资产更新或后续 Story 实施授权。

### 9.2–9.16 可执行进入依据

下列入口继承 [work-packages 的 P0–P4](../specs/spec-runtime-codegen-quality/work-packages.md)；不是自动 ready-for-dev。每项先锁定支持配置、模块 SWS＋ECUC＋通用 BSW/条件适用描述与 MemMap 的合法正文及 SHA-256，逐条定位当前源码差距，再进行实施；来源欠缺只阻塞该项标准裁决。独立预期不从 ABI 或 emitter 自动生成。共用退出条件：同步直接消费者及配置/生成/交付来源，受影响三族/双目标/各 main 的正常 compile/link/run/c-check 与必要人工完成，Q2 兼容方案获授权；任何真实失败保持阻塞。下列状态向量是必需范围和本轮已有回归定位，不表示未执行部分已通过。

- **9.2 CAN（P1）：**来源 `runtime/src/Can.c`、Can_HostLock、Can/Can_GeneralTypes/Can_MemMap、host/ECU execution adapters；直接消费者 legacy/multi CanIf、LSduR、HostBatch/probe 与 OS mailbox。原文入口 CANDriver（历史 hash `d04d35b80d8f445b66f960fd9b5be4f02f324e73cc4be239887e8c1235b172aa`）及通用类型/BSW。需分离标准 config/services 与 sink/注入/clock。独立成功→BUSY、STOP/BUS_OFF→STARTED、复制、迟到/取消确认、合法/非法 controller/HOH 和 polling；本轮 `can_polling_contract`、`com_chain_contract`、`canif_parallel` 运行。工具为 native GCC/线程原语和正常 native/host-boundary suites；公开 config/sink 改动先 Q2。退出须两种确认策略都保持，不能以机制非等 ID 代替合法 source 编号。
- **9.3 ECU 执行（P1）：**来源 Ecu_Target/HostBatch/HostBridge/Execution/OsHooks，消费 9.2 driver 和 OS Waiting/mailbox 固定接缝；其声明/调用方见上节，不把自有 ticket/receipt API 称为标准 AUTOSAR 服务。独立 admission 超限/并发拒绝、同 epoch 不重跑、真实 output write/flush 失败、watchdog、排空后 COMMIT_OK；本轮 production owner sealed test 运行，完整 Win/Linux fault vectors 尚须 native Python build-mode/host-boundary 入口。batch/output wire 或 layout 变化先 Q2；退出真实生产入口可恢复/关闭且无伪确认。
- **9.4 CanIf（P2）：**legacy/multi CanIf 与 `communication`/`multi_bsw`/`multi_ecu`；依赖 9.2 typed Can、LSduR routing、ComM mode、Det runtime error 与 SchM。原文 CANInterface（历史 hash `84bf624c9e497d765653c8df4d62a6e3835efb64f76c742d727b177a64c10f53`）。独立模式 OFFLINE/TX_OFFLINE/ONLINE、length/filter/handle 拒绝、BUSY、异步 confirmation/stop、DevErrorDetect=false 时适用 runtime report；本轮 chain/parallel/polling 向量运行。工具 Cargo multi suites+真实线程、两目标 c-check；frame-index→PduId/config/callback 迁移须 Q2，全 controller/HOH 连续编号及数量前置，退出上下层真实 mode/callback 闭合。
- **9.5 LSduR／PduR（P2）：**两套 routing 模块、私有头、routing plan 及 PBcfg；依赖 CanIf、Com、CanTp、Dcm 与 ComStack 类型。原文 LSduRouter（DRAFT，历史 hash `2cd776bc9d54dd8d7f12fa300f24bbf4b790560f9fe7cc9702d20c76b24fdd3a`）和 PDURouter（`3f97e95f6b36c876daeddc1bd90db7d0b24ebd0aa652078349d323ad6664ad6f`）。独立方向同值 handle 隔离、NULL/length/route 拒绝、buffer copy/size、TP admission/恰一次成功与失败确认后复用；本轮 COM/diagnostic/queued chain 运行。工具 Cargo C consumers+sealed build/c-check；旧 route/frame-index/public buffer API 变化 Q2，退出 routing config/type/glue 与全部上下层一致，不合并命名空间。
- **9.6 COM（P2）：**两套 Com、multi_com/multi_rte/multi_ecu/单组件桥；依赖 PduR、RTE notification 与 legacy Dem monitor。原文 COM（历史 hash `8addd33ad1e625b4e9d01afc938131d424f7161f18075f44d2a5fcfa925c5190`）；handle-bearing 通知仍按历史 DRAFT 记录，未重审正文。独立 Send/Receive/first publish/group stop/DM first=0、disable/enable、重复 timeout、恢复保留 last、lower TX failure；本轮 com_contract、com_chain、新 rte_com_contract 运行，未覆盖全部 group/DM 可选组合。工具 Cargo native consumers及 c_analysis/c-check；legacy 签名与 gate/monitor 拆分 Q2。退出 COM/RTE 状态分离、唯一配置与通知，不能用 host epoch 代替 COM DM。
- **9.7 CanTp（P2）：**两套 CanTp、SchM、multi_ecu/routing；依赖可信 execution 时基、LSduR/PduR/CanIf、Dcm buffer。原文 CANTransportLayer（历史 hash `7334d633b02c443aacbe0ca25e20c319e977104eeb4bf8e67ac0358cfea34a22`）。独立 SF/FF/CF/FC、SN/length、WAIT/block/STmin、N_As/N_Bs/N_Cr 与 CopyTx BUSY/N_Cs、迟到确认/恢复、admission 拒绝无最终通知；本轮 diagnostic/queued chain 运行但非所有 timer/config vectors。工具 native Cargo+protocol/ecu-tool/c-check；保留 legacy DLC/WAIT/payload 策略，新增标准长度/回调/重试语义先 Q2。退出 accepted buffer 到真实 confirmation，唯一时基，不冒称支持 partial-buffer WAIT。
- **9.8 Dcm（P3）：**两套 Dcm、Dcm_Internal/Rte_Dcm_Type、host Dcm_Execution、diagnostic/multi_mode/multi_rte/multi_ecu；依赖 TP/PduR、RTE DID server、ComM、legacy Security/Dem。原文 DiagnosticCommunicationManager（历史 hash `cf5aeee78fda6e5a25f982f04cb146ce4a75586de7ba41913d46b0cea1cc2407`）。独立 session commit/P2/P2*/S3、DID 次序/容量/权限、Copy/Retry buffer 生命周期、无 DID F186、network-only/半路由拒绝、确认/超时后复用；本轮 diagnostic/queued、sealed optional DID/network-only 运行，旧 write/routine/DTC/Security 仍需 Python 独立回归。工具 Cargo native+legacy protocol/native/c-check；whole-message/CS callback 与无 possibleErrors 的 void server 迁移 Q2。退出实际 ComM 活动/gating 与 TP 完整链，9.15 消除来源身份缺口。
- **9.9 ComM／BswM（P3）：**multi 模块、Ecu_HostBusSM、SchM_Mode_Host 与 multi_mode/multi_ecu；依赖 CanIf/Can 的实际 mode、Dcm activity、execution admission/output。原文 COMManager/BSWModeManager/所选 BusSM 依据分别见历史表，不采用未实现 CanSM 代替 CDD。独立非法 user/channel/mode、Allowed pending、FULL minimum timer、普通/诊断需求合并、No/Silent/Full、failure 不发布 desired 为 current 与重入；本轮 mode_contract 运行，完整双平台并发仍须核定。工具 Cargo native/线程+c-check；channel/user/layout/callback 改动 Q2。退出 NM NONE/CDD 的所选义务真实闭合，COM group 与 lower mode 不耦合。
- **9.10 Det（P3）：**multi Det/Det_Host/专用头与静态 callouts；CanIf 等模块 runtime reporting 是直接消费者，host halt/control 不进入标准接口。原文 DefaultErrorTracer（历史 hash `f5caa8cb8f2b9ef2e2578a96d940abb795fbf7dfc09a1899b0b81797c2835e1f`）。独立 before-init、runtime 全 hooks 返回、development halt/recursive halt、invalid config、真实并发；本轮 det_contract/parallel 全运行。工具 native Cargo+目标 halt/线程+c-check；标准 ReportError 与 host hook 返回不能混淆，变更先 Q2。退出 halt/status/callouts 依正文判据而非“返回 E_OK”空桩。
- **9.11 NvM（P3）：**legacy NvM/NvM_HostStorage、可选 config/render；直接消费者 Dem 和 host diagnostic DTC、启动/关闭及离线验证器。需取得 NvM SWS/ECUC、所选存储层义务和通用 BSW，现历史表不含 NvM 正文摘要，保持 missing，不套用 host storage 为完整 NvM。独立双槽/flush/fsync、损坏/指纹/lock/close、重启、拒绝不降级成功；本轮仅生成 diagnostic 样本，未运行完整存储向量。工具 Python native/protocol/offline+c-check+实际 Win/Linux 文件原语；storage layout/wire 变化 Q2。退出正常失败与恢复保留旧持久字节，必需设备依赖按所选角色核定。
- **9.12 Dem（P3）：**legacy Dem、Com monitor、Dcm DTC consumers/config/render；依赖 9.11 持久接口与 9.6 明确 monitor。需取得 Dem SWS/ECUC、与 Dcm client/operation-cycle 的正文及摘要（本 candidate missing）。独立初次 Rx/timeout、pause/resume、clear 失败、重启/pending、query 无写入；本轮 diagnostic 生成不算运行证据。工具 legacy Python protocol/native+真实 NvM+c-check；DTC state/layout/公开函数与工具 SDG 迁移 Q2。退出全部所选状态/持久返回独立验证，不扩大到所有多事件变体。
- **9.13 Security（P3）：**legacy Security、BCrypt、Dcm session/config/render 与 host main/offline consumers；依赖可信时间和状态文件。Security 是自有 Windows host 档案，需从 Dcm 安全义务及所选 API/存储合同核定，不虚构名为 Security 的完整标准模块。独立 seed/key、顺序、失败次数、delay/restart、session/S3 relock、损坏和缺 key 拒绝，Linux 配置拒绝；本轮未运行 Windows crypto 向量。工具真实 pinned Windows GCC/BCrypt/Python protocol/native+c-check；key 不入包，state/wire/API 变更先 Q2。退出 Windows 正反/重启真实执行，Linux 拒绝与旧 host 支持边界保留。
- **9.14 OS／SchM（P4）：**实际 automotive OS/backend/host/ARTI 与固定 kernel/port/patch、os_service/schedule/ecu/multi_ecu；依赖 9.3 唯一 execution 声明，9.4–9.9 的 SchM/周期/mainfunction 合同。原文 OS（历史 hash `6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、RTE SchM、BSWMD/MemMap 和适用 ARTI。独立 tasks/events/resource/error/hooks/ISR/tick/stack、真实异步锁及共享 event 周期/phase 拒绝；本轮生成/production owner 和共享 event 回归通过，26 OS suites 未全跑。工具 autosar_tooling os/native-tests、PE/ELF、真实线程/c-check，改声明/栈/layout/patch 先 Q2+第三方身份审阅。退出完整适用 SC1 和双方真实段/栈，不增加 backend 或跨平台推论；进入前按机制核定粒度。
- **9.15 RTE／共享类型／生成（P4）：**contracts/plan/multi_rte/multi_com/multi_mode/multi_ecu、application slots、MemMap/BSWMD/ComStack 类型；依赖上述实际模块类型/callback、OS task/event/SchM 和 execution owner。原文 RTE、SWCT/System/CommunicationStackTypes/StandardTypes/PlatformTypes/BSWMD/MemMap 历史身份见来源表，正文本 candidate 未取得。独立 scalar S/R 初值/扇出、void C/S 输出保护、两个 handle 通知、owner/lifecycle/unknown handle、parameter shadowing 和 event 冲突；本轮相应 native/普通向量通过。工具 Cargo typed compile/link/run+sealed/c-check。全部 app/live slots/模板/模块消费者和 ownership/ABI 类型变化先 Q2；退出唯一 producer、类型来源/配置一致及上述 Dcm consumer 元数据修正，不能从 inventory 生成唯一预期。
- **9.16 兼容交付（P4）：**prepared/resources/target、delivery/ownership/reopen、包内 tools/ecu-tool 和 ecu_tools；消费 9.2–9.15 经授权固定合同，不能代替各 owner 迁移。官方资源/XSD/MOD 按旧 v1 正常入口显式提供，v2 不新增档案依赖。独立原字节/旧 seal、v1 精确接纳/拒绝、v2 多槽搬移、create-only、source edit/stale confirm、tamper/额外文件/link/reparse 拒绝；本轮普通 suite 和 moved/regenerated multi 样本、production owner 实际运行，完整旧 v1/offline 双平台接手尚需执行。工具 Rust 正常生成+外部 CPython/GCC/binutils/Git、test_offline_delivery；实际影响 IPC 才加隔离桌面。wire/seal/schema/用户源变化先 Q2；退出新目录不依赖 checkout 的真实接手，旧包不静默迁移，进入前按已确认风险拆小。

### 本轮独立 consumer 与回归范围

新增 `core/tests/fixtures/multi-runtime/rte_com_contract.c` 是独立 translation unit，不读取 ABI inventory；两个函数指针固定 `void(CbkHandleIdType)`。Cargo 测试分别生成 callback handle `17` 与 `43` 的有效配置，第二向量反转源顺序，链接生成 `Rte.c`、真实交付 `Com.c` 和三份应用源。仅 owner/lifecycle 是显式 test adapter；不冒充真实 OS lifecycle 或线程证明。向量保护未启动通知、初值 7/NEVER_RECEIVED、未知 handle 隔离、真实 RX 发布 21、30 次 COM main function 后 MAX_AGE/last 保留、非 owner receive/timeout 无效、真实 RX 恢复 42、group stop 的 COM_STOPPED 和 Rte_Stop 后 out-value 99 保留。这是现有固定接缝回归，非本轮未取得正文的标准审计。

复用并实际执行既有 `generated_headers_compile_and_link_independent_scalar_and_void_server_signatures`、`service_argument_names_cannot_shadow_selected_runtime_and_component_producers`、`shared_os_event_requires_one_effective_period_and_preserves_rejected_sources` 及相邻碰撞/共享 phase 反例；拒绝时有效引用、原始 source 与既有 sealed output 的保护仍由正常 suites 断言。真实 COM/CanIf/LSduR/PduR/CanTp/Dcm/mode/Det 的正反状态及并发全部走既有 `multi_runtime_contracts` 九个消费者，无新增编排或产品 stub。

## Spec Change Log

- 2026-10-11：本次仅消费已有有限验收，delegated approval by dev-user/dev-pm under maintainer authorization 2026-10-10；客户批准复用t_fe9c7807，范围批准t_9befde8f/t_dabe832f，状态消费授权及精确key由PM t_0c9ac2d8/t_cd90481e评论确认。原规格SHA256=`f9f0ec0abfc5a8b07533691ad55923b46ae487ef2671cdf6fed718135cd93dff`、原tracked diff SHA256=`1260941b398b7a336138300a13eeb6fc8f8f55eebf98972b87ae1f89974a24a4`；冻结意图、三条AC及所有既有失败/未核查记录不改。原有限Tester t_40909b7c及最终PM t_17240366已确认同一原候选的三AC及证据身份；本候选保真继承原产品/测试字节，所以复用而非重跑或假称fresh验收。Story9.1 review→done仅表示该共享入口/护栏有限验收已消费，Epic9仍in-progress，9.2–9.20仍backlog，源码/规范/硬件/认证/发布出口不升级。

- 2026-10-10：delegated approval by dev-user/dev-pm under maintainer authorization 2026-10-10。客户 t_fe9c7807 与 PM t_9befde8f 接受本单一目标及完整 Code Map；批准前 SHA256=bcccfd60a14c0dc4a700349c174777c3ecd17351fcaef1f15a9a0ce08d1e726f。仅修正审批来源与状态，冻结业务意图、三条 AC、架构及 local-only 权限不变。沿既有 Build snapshot 继续，不重渲染。

## Review Triage Log

- 实现者初次自检不是独立 review。随后父 Build 已同时运行 blind-hunter、edge-case-hunter、verification-gap 三个同能力、无上下文 reviewer；后两层分别返回 `[]` 与 `No verification gaps found`，blind-hunter 的八项逐项裁决如下。当前 `in-review` 表示 Build 内部审查，不是 Kanban tester review；sprint 仍 in-progress，第五项任务未完成。

| 发现 | verdict | 证据与路由 |
| --- | --- | --- |
| BH1 独立 fixture 手工 COM 配置未覆盖生成配置 | low | 该 fixture 确实手工配置，以独立签名和真实 COM/RTE glue 为目标；生成配置另由 `sealed_multi_owner_preserves_network_init_and_deadline_phase`（5360起）经实际初始化和正常 ECU 构建覆盖，epoch0/controls 明确断言通知、DM及恢复。将完整 bootstrap 重复嵌入此 fixture 是额外复杂度，不是日常集成缺口；拒绝该增补，既有完整生成入口不得删除。 |
| BH2 两个 handle 分两次运行不覆盖同时多个 receiver | low | 该 fixture 确实只覆盖单一所选 Rx；没有把两个独立运行声称为完整多接收者证明。当前选集及责任记录不允许据此放行全部配置；增加同时多个 endpoint 是更复杂的配置覆盖增补而非本选集缺陷，拒绝本次增补，未覆盖组合继续明确限制。 |
| BH3 只读第30次可能漏掉提前过期 | medium | 原 fixture 循环结束才读，虽然现有 sealed epoch0 已按1–30独立断言，新 consumer 本身缺边界断言。patch：逐次证明1–29正常，第30次过期，并在恢复后复验完整间隔。 |
| BH4 非owner首次publication未验证 | medium | 原 fixture 仅过期后测试非owner receive，确实不保护首次发布。patch：首次真实RX前非owner通知后保持7/NEVER_RECEIVED。 |
| BH5 adapter恒成功漏掉Start/Stop授权检查 | medium | 固定adapter确实不能捕获删除调用的回归；不涉及新增OS标准判定。patch：可拒绝adapter，Start/Stop分别失败且保留原生命周期/数据；真实OS生命周期仍由sealed入口证明。 |
| BH6 生命周期外通知的无作用不可观测 | medium | 原先Start重置或Stop后Read早退掩盖回调读COM。patch：Linux链接器wrap真实Com_ReceiveSignal，仅计数再调用真实函数；先初始化COM，断言Start前/Stop后未访问COM。不改产品源或引入第二COM实现。timeout的生命周期外内部状态不可观测，不声称完整副作用证明。 |
| BH7 Windows启用但helper生成Linux交付 | medium | `prepared_multi_files`确实固定Linux。patch：该新consumer明确仅Linux注册，链接器观测也仅该平台；不把它当作Windows证明，Windows验证保持未执行。未修改历史helper或其它suite。 |
| BH8 spec状态与自检说明不一致 | low | 自检说明为实现者返回时的历史记录，父Build开始后转in-review；产品无状态错误。按照Build拒绝仅编辑本spec的修复建议，本段将历史时点和当前Build内部阶段明确区分。 |

BH3–BH7按patch处理，无intent_gap/bad_spec/defer。实现child已结束且工具未提供可继续的ID，父会话应用最小补丁并复跑完整开发检查；独立review过程已完成，Build最终验证/呈现及同卡tester仍等待下述非零源码结果的范围裁决，不宣称Build完成。

## Verification

### 本轮实际结果

- 父 Build 补丁后实际复验：默认 Cargo exit0（lib34、builtin72、c_analysis2、普通multi69）；native multi_runtime **9 passed**；native multi_component **76 passed**（124.32s）；quality exit0，2 files/C syntax checked；git diff --check exit0。新consumer单独先跑 **1 passed/75 filtered**。旧实现者结果在下文保留，不混称为新运行。
- 固定工具缺件已通过正常HTTPS路径解除：下载固定提交 `e73bf44c3e49686b7495fab352d03a6c6075516b` 的源码到私有scratch，`make -j4 MATCHCOMPILER=yes` exit0；Cppcheck实测2.21.0、binary SHA256=`ac4b5d400b6db28f2ce16cf1a4c5fbd7d50d07d78c10aae4fc7e88b7f4abdf54`。三份addon摘要经正常入口校验；未修改系统信任、宿主配置或仓库工具身份要求。
- `CPPCHECK=<scratch>/cppcheck CPPCHECK_ADDON_DIR=<scratch>/addons uv run --locked python -m autosar_tooling c-check --target linux-x64-controlled-v1 --project <scratch>/story-9-1-final-samples-1791643904/multi-component --output-directory <scratch>/story-9-1-pinned-analysis`：**exit1**，52 units、2607原始diagnostics；两独立main `host-batch`/`legacy-probe` 均exit1，summary `error=null, passed=false, completeCompliance=false`。summary SHA256=`96538a045d8145b8d7160ed9cbdb746b4b07b4f4e1671d4793d13c1b0f88ebce`；sourceSeal=`c377331fc8003924338465b0ad098bfa93d537829636219048954520aec14556`。这不是工具缺失或完整规范判定，原始源码诊断保持失败；尚未裁决2607项为真实标准违反，也未批准偏离。Build step04要求无法修复的验证失败HALT，当前授权又禁止产品模块整改，因此交PM核定入口证据工作与源码整改的边界，未越权修runtime或弱化门禁。
- 补丁范围仅测试：新增deadline 29/30及恢复、首次非owner、Start/Stop拒绝、Linux真实COM访问计数；生成producer/清单未变，已扫描封存工程仍有效。fixture的GNU linker wrapper为主机测试观测，不作为产品接口或MISRA符合依据。资料分类中的“缺Cppcheck”是首次尝试历史事实，现已补齐工具但源码结果仍失败；法律正文/MISRA人工材料未补齐。

- `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts`：**9 passed, 0 failed, 0 ignored**，1.23s；真实 C COM、driver/CanIf/routing/TP/Dcm、queued token、mode、Det halt/parallel consumer 均执行，不使用默认 feature 下的零测试充数。
- `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts`：**76 passed, 0 failed, 0 ignored**，122.39s；新增两个 handle 的生成头 compile/link/run、已有 scalar/void-server、符号/parameter shadowing、共享 event period/phase、sealed optional DID/network-only、生产 owner 均实际运行。
- 默认 `cargo test --locked --manifest-path core/Cargo.toml`：最终全部通过；lib 34、builtin 72、c_analysis 2、multi_component_contracts 69。默认 multi_runtime 为 0，不作为本轮模块运行证据。`c_analysis` 生成六个 Linux 封存样本，multi 还执行搬移、删除原 live 工程、独立导入、再生成和完整 payload 字节一致断言。
- 复跑默认 suite 曾因持久环境里的 `AUTOSAR_C_ANALYSIS_SAMPLES` 指向已生成目录而拒绝覆盖（1 failed, 1 passed）。改为新的 `$TMPDIR/story-9-1-final-samples-<唯一时间值>` 后完整默认 suite 成功；原样本保留，未删除或弱化 create-only 检查。以后每次执行含 `c_analysis` 的 suite 必须设置新目录或取消该持久变量，不将已有输出目录复用为新样本。
- `uv run --locked python -m autosar_tooling c-check --target linux-x64-controlled-v1 --project "$AUTOSAR_C_ANALYSIS_SAMPLES/multi-component" --output-directory <新目录>`：实际执行，**exit 1**，`Cppcheck is missing; install the pinned 2.21.0 release`；正常 summary 为 `passed=false`、`error` 非空。受影响 multi profile 已重生成，但扫描/分析完整性、源码诊断和人工合规结论均未成立。解除条件是提供固定 2.21.0 及匹配 addons 后，用新分析目录重新运行完整各 main 分析；不能换未核定旧 checker 或借 Cargo 通过代替。
- `cargo clippy --locked --manifest-path core/Cargo.toml --all-targets --features native-tests -- -D clippy::correctness -D clippy::suspicious`：exit 0；既有 style/复杂度等 warnings 保留，未全仓改写，未称零 warning。
- `uv run --locked python -m autosar_tooling quality --base 680570a55b7b78d3930a654824c572147f276377`：exit 0，`formatted files checked=2; C syntax=checked`；包含新增 untracked C fixture 与 Rust 变更。`git diff --check` 通过。未改交付资产，不运行 assets update。
- native compiler 为 `/usr/bin/gcc`，实测 SHA-256 `1b99826121ae6682a634e5efe09bd3e3df58ce58e0b28f849114ab5b89139c26`；objdump 为 `/usr/bin/objdump`，SHA-256 `325c4205a4c658a9d1e1ebc469ae55975a2b897a3d3c1e79d9b158612d37f745`，匹配 Linux target 固定身份。native suites 使用明确 AUTOSAR_CC／AUTOSAR_OBJDUMP／AUTOSAR_GIT／AUTOSAR_PYTHON 路径，未放宽正常工具校验。

### 验收对应与保留范围

- AC1：本规格实际选集/身份、来源分类和 9.2–9.16 直接依赖/独立向量/工具/授权/退出条件可定位；历史失败不改写。六样本实际生成不表示 Windows、全可选配置或完整模块标准已核查。
- AC2：新增独立 COM→RTE consumer compile/link/运行成立；既有 scalar/void C/S、有效引用碰撞、parameter shadowing 与 period/phase 反例均跑过。责任唯一见本规格接缝节。owner/lifecycle adapter 的限定范围明确，真实异步锁有既有 CanIf/Det 并发 suite 证据，不由本 fixture 推断 OS lifecycle 正确。
- AC3：具体公开/legacy 差异、所有直接消费者责任链、版本化/兼容方案和授权门保持明确；Dcm 身份缺口及缺正文的模块实施继续阻塞。未实现 9.2–9.20，未提交、推送或发布。
- 未运行：Windows native/crypto/PE、全部 26 OS suites、官方原文 oracle、GUI/Tauri/IPC、安装包、MCU；纯测试/规格改动不触发无关桌面验收。MISRA 编码指导已加载，未获得授权规范全文或完整分析，不作完整 AUTOSAR/MISRA 符合声明。

### Change Log：PM范围裁决与Build入口收尾

- 2026-10-10 23:20，delegated approval by dev-pm under maintainer authorization 2026-10-10，裁决t_dabe832f=approve-and-continue；复用客户t_fe9c7807对原意图/三条AC的接受。裁决前spec SHA256=`6dab22e5127ed6de1bb338f40c9b6c9b4948ca7e5fdac6881aa8b4b944683bc7`，完整diff SHA256=`2afa26fb26ddc1f1004cf43a996dd54dccc03fc5aa24290f2111e3a1aa1feb8d`；父会话已核验完全一致，原卡仍running，直接按裁决续行，无重复unblock/新管线。
- 理由：epics.md:1135定义9.1调查/共享契约入口，AC1保留历史失败，AC3使欠依据模块实施继续阻塞；本次必需入口条件是实际执行受影响profile分析并如实保留结果，不是把全部既有模块整改至零诊断。Build step04 HALT已暴露并经PM核定出口归属，不修改该条，不豁免本次编译、行为、质量或独立验收失败。

| 入口证据检查满足（待独立Tester核验） | 源码/规范出口未满足（继续阻塞相关模块） |
| --- | --- |
| 共享身份/责任及三AC已形成可定位证据；本次consumer真实编译/链接/运行、回归和quality通过；固定工具及受影响生成profile分析已实际执行并保留失败结果 | c-check exit1、passed=false、completeCompliance=false及2607原始诊断完全保留；人工未评估、合法模块/MISRA正文缺失、完整符合性未证实；相同诊断数量不证明历史/本轮诊断逐项等价 |

- c-check扫描生成工程不含新增fixture，不声称该fixture的MISRA逐条通过；fixture仅有真实编译/链接/行为、增量quality及编码指导证据。未修改XML/summary/退出码、规则/工具要求或suppression；未批准偏离。Dcm来源缺口仍交9.15在获准范围闭合，9.2–9.20保持backlog。
- 既有snapshot step05完成：spec=done仅Build入口阶段；sprint Story9.1=review，Epic9仍in-progress，同卡Tester以及最终PM依赖不变，Story最终验收未完成。step05自动commit被本任务明确无commit授权覆盖，跳过且未push/PR/merge；不提出未经授权发布。无defer项，未覆盖平台/配置及源码失败不是被取消的验收义务。
- 本段收尾替代前文“第五项未完成/等待裁决”的历史时点说明；未重跑有效代码证据。实际candidate差异：tracked为sprint及multi_component_contracts；untracked为派发前已有epic-9-context、当前spec及新增rte_com_contract.c。multi_runtime_contracts/callback_contract.c无新差异，仅复用并运行原suite，不误列为本次改动。

### 后续正常复验入口（命令）

- `cargo test --locked --manifest-path core/Cargo.toml`：默认回归。
- `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_runtime_contracts`：实际独立C consumer，不接受零测试。
- `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts`：生成consumer、符号／时序回归；按真实工具条件区别未执行。
- 新增C fixture前加载项目misra-c2012；按实际受影响生成profile运行`c_analysis`及正常`c-check`。工具缺件单独阻塞该验证，不借green Cargo宣称MISRA符合。
- `git diff --check`及正常增量quality检查；记录tracked/untracked diff身份。不机械运行GUI、全OS或Windows行为验收，未跑层级明确保留。
