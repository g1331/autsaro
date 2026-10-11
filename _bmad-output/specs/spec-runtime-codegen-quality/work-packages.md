# 模块工作范围、验收与进入条件

本文件供后续 Story 拆分使用，P0–P6 沿用 Epic 9 基线的工作包代号，**不是 Story 编号**。具体缺口按模块核定后再分任务；不把原始诊断数当任务数量，也不把本规格定稿标成实施就绪。

## 共同边界

开始每项工作时记录实际提交、支持配置、profile、target、source／header／宏／patch／main 选集；最新共享版本须重新核对。[基线](../../planning-artifacts/architecture/epic-9/BASELINE.md)记录 `a09955c` 的现状和失败摘要，是历史观察，不保证后续仍相同；[架构契约](../../planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md) AD ID 保持不变。

源码路径以仓库根目录为准。下列范围是必须跟踪的实际责任链，不是授权全目录重写。每个模块先定位所有调用方，再选具体文件与受影响配置；只有真正一致的状态与契约才共享实现。

所有生成／ABI 包共同核对：

- `core/src/model.rs`、`arxml`／`arxml_render`、`definitions`／`rules` 与当前内置 metadata；源字节、对象／引用、定义身份和 plan 分开负责。
- `core/src/integration/{graph,plan,configuration,native_configuration,multi_bsw,contracts,artifacts,ecu,multi_ecu}.rs` 与该模块专用 emitter；原文到计划的合法性、唯一 producer、声明及配置同时闭合。
- `core/src/prepared.rs`、`resources.rs`、`target.rs`、`generator/delivery*` 与 ownership／reopen／tools；target 选集、交付路径、资产身份、live 快照及 seal 不漂移。
- `runtime/contracts/{bsw-v1,assets-v1}.json`、`tools/python/src/ecu_tools/workbench-v2-assets.json`、固定第三方 manifests；只有经过审阅的源码字节变更才更新摘要，ABI／第三方身份另行审阅，先核对固定换行。
- `tools/python/src/ecu_tools/{build,verify,reference,workbench_v2}.py`、包内 `runtime/ecu-tool.py` 和生成的 wrapper；接收者独立 Python／工具路径不依赖开发 checkout。

## P0：逐模块核定契约和独立预期

**成果：**为下面每个实际模块确定完整适用范围、具体缺口、正反预期和需要保护的旧行为。研究可以开始，不能要求待整改能力先通过才允许调查。

依据从基线的规范范围、[Epic 4 固定契约](../../planning-artifacts/architecture/epic-4/R24-11-CONTRACT.md)、[Epic 8 固定规范来源](../spec-multi-component-scheduling/compliance-references.md)定位，核对合法 R24-11 原文及摘要后判断；旧记录、模块名、内置定义和 ABI 清单不替代全文。核对模块 SWS、ECUC、BSW General，以及条件适用 BSWMD／SWCT、RTE、MemMap、状态、错误与上下层回调；OS 保持完整适用 SC1，而非仅参考对象。

MISRA 原文与修订基线单独核定，200 Rules＋21 Directives 自动／人工及采用代码责任分开；每条分类有源码、配置、目标和规范依据。当前 `assessment=not_assessed`、源码 `passed=false` 不能因整理状态改为通过；R21.8、errno、mmap 等已知工具限制只提供核查线索，不授权 suppression 或批量误报。

**验收：**对应模块范围和条款明确，独立 C consumer／输入向量可从正常测试入口执行，成功／拒绝／恢复及实际并发预期独立定义；缺资料、待审契约与未执行测试各自写清解除条件。结果进入该模块的正常 BMad 规格／Story，不新增全量检查矩阵或审批系统。

## P1：共用 Can、主机适配和 ECU 执行责任

**实际来源与接线：**`runtime/src/{Can,Can_HostLock}.c`、相关私有头与 `runtime/include/{Can,Can_GeneralTypes,Can_MemMap}.h`；`runtime/host`；`runtime/ecu/src/{Ecu_Target,Ecu_HostBridge,Ecu_HostBatch,Ecu_Execution,Ecu_OsHooks,Ecu_SchM}.c`。直接消费者是 legacy／multi CanIf、LSduR、HostBatch／probe、OS mailbox／integration hook；选择由 `target.rs`、`ecu::source_assets` 和资产库存决定。

**必须解决与保护：**标准 driver 服务／配置与 sink、注入、时钟、输出及测试控制分离；控制器、pending 帧、复制缓冲及 lifetime token 唯一拥有；ECU batch admission／output／confirmation／receipt 端口由 P1 负责。与 P4 共同固定 OS 端口声明、SchM 锁协议和独立链接向量，不分别定义两个接口；不改变 backend 调度所有权。

**独立验收：**标准入口的初始化、重复调用、合法／非法 controller/HOH、STOP／BUS_OFF／恢复、BUSY、帧复制与迟到确认；sink write/flush 失败不能伪造确认或回滚；同 epoch 不重跑，COMMIT_OK 仅在真实 Waiting 且全部工作排空后发布。现有 batch 容量与 watchdog 边界保持，超限／并发 admission 拒绝；锁忙、锁错、退出与资源回收覆盖真实 Win/Linux 原语。

**正常入口：**`core/tests/multi_runtime_contracts.rs` 的 driver／CanIf fixtures，`core/tests/native.rs` 与 `tests/python/native/test_host_boundaries.py`、`test_ecu_build_modes.py`，封存 `ecu-tool build/verify`；实际 c-check 覆盖三族受影响目标／main。标准签名变化先完成 Q2，不能用改名／typedef 把旧 API 自动变成标准。

## P2：通信模块与对应生成来源

可按以下独立机制拆分，共享公共接口的 owner 先确定。全部包同步 P0 标准核定、MISRA 分类和人工审查，不等迁移末尾才扫描。

| 单元 | 来源、生成与直接消费者 | 独立验收与必须保留的边界 |
| --- | --- | --- |
| COM | legacy `runtime/src/Com.c`／include、multi `runtime/multi/src/Com.c`／Com_Internal 与公开／MemMap 头；`integration/{multi_com,multi_rte,multi_ecu,ecu}.rs`、单组件 Com 桥；RTE／PduR、legacy Dem | 真实发送／接收、初值、首次发布、last/freshness、DM 超时及恢复、group start/stop和下层失败；local 通信不受 CAN 停机阻断，multi COM_STOPPED 不与下层 TX 错误混用；legacy 同步确认／gate 保留 |
| CanIf | legacy／multi CanIf.*、上下层类型与 SchM／MemMap；`multi_bsw`、`communication`、`multi_ecu` 配置；Can driver／LSduR／mode consumer | 独立标准接口编译链接、路由过滤、长度、controller/PDU mode、RX/TX 回调、异步锁；命名空间和标准 controller/HOH 连续编号不由 metadata 范围代替；旧 frame-index ABI 必须审阅迁移 |
| LSduR／PduR | legacy／multi 模块与私有／模块专用头；`routing.rs`／`multi_ecu.rs`；Com、CanIf、CanTp、Dcm | Rx/Tx／上下层同值 handle 不合并；指针／长度／路由拒绝、复制及缓冲边界、确认恰一次、TP 成功／失败／恢复；生成函数指针与独立 consumer 兼容 |
| CanTp | legacy／multi CanTp.* 与 SchM/MemMap；配置、routing、multi_ecu；LSduR／PduR／Dcm | SF/FF/CF/FC、序号、长度、block/STmin、N_As/N_Bs/N_Cr、迟到确认、下一合法请求恢复；不同 profile 的 DLC、WAIT 与 payload 策略保留；计时来自唯一可信时基 |

COM→RTE notification、共享通信类型及配置 glue 的符号、声明、交付路径和唯一 producer 在 P2/P4 的共同契约中一次核定；其他 emitter 只引用。不能通过 `multi_ecu` 最终字符串替换另定语义，也不能未经必要回归直接合并旧／新源码。

**正常入口：**Cargo `multi_runtime_contracts`、`multi_component_contracts`，其 `fixtures/multi-runtime` 独立 C consumer；`c_analysis` 生成真实样本，再运行对应目标 c-check 和实际 `ecu-tool` 构建／行为。最小配置拒绝用普通 Rust tests；运行和分析需要目标工具，不能用无工具单测替代。

## P3：诊断、mode 和持久链

| 单元 | 来源／接线 | 独立验收与兼容保护 |
| --- | --- | --- |
| Dcm | legacy／multi Dcm.*、Dcm_Internal／Rte_Dcm_Type、host Dcm_Execution；`integration/{diagnostic,multi_mode,multi_rte,multi_ecu}.rs` 和 ecu 模板；PduR/CanTp/RTE／ComM/Security/Dem | 会话提交、P2/P2*/S3、DID 顺序／容量／会话权限、收发拒绝和恢复；无应用 DID 的 F186、完整无诊断/network-only及半路由拒绝；旧 write/routine/DTC、安全配置不丢失；配置 callback 与独立签名兼容 |
| ComM／BswM／Det | `runtime/multi` 模块、ComM_Internal／Det_Host与专用声明/MemMap；`multi_mode`／`multi_ecu`；CanIf、Dcm、host CDD BusSM与SchM | 初始化/DeInit、合法／非法 user/channel、诊断请求、wakeup、模式通知及重入／并发；ECU lifetime classification 与 ComM lifetime 不混淆；有界 NM NONE 不冒充 CanSM/Nm/CanNm |
| Dem／NvM／Security | legacy 模块及 NvM_HostStorage／BCrypt；generator/render 的可选配置与回调；Com monitor／Dcm／RTE | DTC 状态变化、暂停／恢复、清除及失败返回，双槽写入／落盘、损坏/配置指纹/锁/关闭失败不降级成功；seed/key/session、失败次数与重启行为；密钥不打包，Security 的 Windows 范围保留 |

标准直接上下层义务随选择配置核定，host 存储与注入接口不冒充设备驱动；若发现范围内必需软件依赖缺失，纳入对应模块故事整改，不能仅贴“host 专属”延期。P3 消费 P2 路由／类型及 P1 执行端口，按真实接缝依赖；没有共享变更的子模块可独立研究。

**正常入口：**multi `diagnostic_contract`／`diagnostic_queued_contract`／`mode_contract`／`det_*` fixtures 的 Cargo suites，legacy Python protocol／diagnostic／native suites、封存工具真实验证及 c-check。已有六样本不覆盖全部 Security/routine/无诊断配置，受影响组合必须补到现有测试或样本入口，并分别取证。

## P4：RTE、OS、类型、调度与工程交付

**来源／调用方：**`integration/{contracts,multi_rte,multi_com,multi_mode,schedule,os_service,arti,configuration,ecu,multi_ecu}.rs`、`runtime/ecu/templates`，`runtime/os` 全部实际受影响公共／私有／host/backend、`third_party/freertos` 固定内核/port/patch；应用槽与 `generator/delivery`；build／reopen／offline consumers。

**责任与进入：**P4 拥有 OS/backend、生成 SchM 与 RTE；消费 P1 固定 ECU execution／receipt 声明、P2/P3 通信／诊断类型和 callback 契约。保持唯一 backend、SC1 全部适用义务、真实栈／ARTI、single/multi 应用身份与同 owner 调度；类型/句柄/周期/phase/顺序在同一计划核定。生成源负责模块配置和适用描述／MemMap，sealed 表达交付，不建立第二配置权威。

**独立验收：**公开 OS/RTE 接口编译链接、初始化/任务事件/资源/错误/Hook/ISR/时序与实际栈、multi S/R 初值／扇出／同步 C/S 及拒绝时输出保护；PR #18 参数符号碰撞与同 event 周期/phase 反例继续保护。各实际 main 不混链；真实 PE/ELF 段按目标证明，MCU 放置不推断。

交付验收覆盖未改配置原字节、保存重开、旧 v1 精确接纳／拒绝、v2 多槽搬移、create-only 初始化、用户源变动使确认失效、再生成逐字节保留 live 与快照；篡改/额外文件/链接或 reparse point/路径冲突明确拒绝。封存工具不用 checkout、Rust/Node/uv；支持 Python 和固定工具身份按已声明目标取证。ABI变化先完成兼容方案与授权，再同步来源／所有调用方／清单／说明；旧 seal 不升级。

**正常入口：**Cargo `multi_component_contracts`、`native-tests`（含 26 OS suites）、`official-oracles` 与 existing delivery/ownership/reopen tests，Python `test_offline_delivery.py` 和包内 `ecu-tool build/verify`；保存／IPC 行为实际受影响时执行隔离 desktop 场景。合法规范与工具未就绪只阻塞相应验证，不能把缺前置或未运行记成通过。

## P5：正常开发反馈与 CI 成本

**来源／入口：**`core/tests`、`tests/python`、`tests/desktop`、`tools/python/src/autosar_tooling/{c_check,c_addon,quality,verify,acceptance}`、`ecu_tools`；`.github/workflows/checks.yml`、`.github/actions/native-analysis-compiler/action.yml`及实际受影响已有 workflows；团队说明放 `docs/development/testing.md`／environment，不另造启动器或任务系统。

**先做调查／设计：**依 [generator-practices](generator-practices.md) 用三类实际改动检查修改落点、最快有意义反馈、失败定位和完整检查升级条件；区分冷／暖依赖、Rust构建、样本生成、compiler validation／macro probes、各 main CTU、原生行为与队列耗时。不能仅按文件名删除不同层的测试，不让开发优化沦为只能在 CI 完成的工作。

**实施验收：**按既有 Cargo filter／测试目标、Python suite 和正式命令提供明确改动到检查对应；失败保留原始 stderr/stdout、输入、stage 和退出状态，不增加隐含代理环境。去重前证明独立风险覆盖不丢，修改同一生成规则无需在多个来源手工修补。性能优化解释可比前后耗时／缓存命中／资源及不稳定失败，门禁新增成本和优化收益分别报告；不承诺固定降幅。

CI 最低保留双目标六样本、8 分组、12 摘要及所有实际 main；profile 可选配置按影响补充，test 宏与 production 分开。分析前后 seal、工具版本／摘要、compiler真实头／宏、补丁私有副本与CTU闭包仍严格。缓存不存源码扫描结果；producer等待/重复工具准备/构建中间物复用只有实际证据和失效边界成立才改。单元/局部检查快反馈不替代整套源码门。

P5 可与 P0及独立模块研究并行；涉及某模块的测试迁移跟随其契约，不改变标准源码门启用前的整改依赖。

## P6：严格门禁和非实现者出口

**前置：**CAP-1–CAP-4 对应整改、所有受影响 profile 和目标验证、必要人工评估、兼容审阅及 P5 结果完成。无法消除真实违反时提交具体替代／改造方案，保留阻塞；不能由 Agent 批准偏离。

**持续门：**每个自动 PR 都执行严格源码结果检查，聚合要求完整身份、`error=null`、`passed=true`、所有上游 success；同名／重复／遗漏、skip/cancel和工具失败都不能通过。`c-analysis` 接入真实必需分支状态；用实际 PR 的违规输入、缺样本、工具失败、必需任务跳过／取消证明拒绝合并，再恢复合法完整输入，不能只检查 workflow 文本。远端修改和这些实验需届时对应授权，本轮不执行。

**独立接手：**非实现者从新目录沿普通入口复验配置往返、源码生成、接口及全部受影响程序构建、CAN/诊断/调度成功与拒绝恢复、源码保护和完整性；移动 sealed 包，按支持的平台与工具离线接收。实际受影响 Tauri/IPC 在隔离环境复验，不能占用用户桌面。浏览器、native、安装包、离线构建、硬件证据分别记，未运行保持未运行；本 Epic 不自动扩大为安装发行验收。

只有 Epic 9 全部退出符合证据才能关闭并放行 R11 消费者实施。一般软件缺口不移交 Epic 10／R10；真实硬件剩余项按规范、目标条件、责任和可执行判据交相应阶段。结果留正常 Story／规格、CI 工件与 sprint，不新增状态或报告系统。

## 正常命令与前置条件

这是选入口的索引，团队操作细则仍由 [testing.md](../../../docs/development/testing.md)维护。以下为后续实施验收入口，**不是本轮已执行清单**；具体 Story 需据实际支持配置补齐过滤器和路径，不把一个示例命令当全部验收。

| 修改／验证 | 正常入口 | 前置与不能证明的范围 |
| --- | --- | --- |
| 模型/计划/生成拒绝 | `cargo test --locked --manifest-path core/Cargo.toml`；按 `--test multi_component_contracts`／test名缩小 | 默认 tests 不需官方资源，局部通过不证明 C 运行 |
| 模块接口/行为 | `cargo test --locked --manifest-path core/Cargo.toml --test multi_runtime_contracts` | 依 suite 实际 compiler与宏；标准预期须独立，不能只看 ABI inventory |
| 真实分析样本 | `cargo test --locked --manifest-path core/Cargo.toml --test c_analysis` | 如需后续扫描先声明新的 AUTOSAR_C_ANALYSIS_SAMPLES 路径；样本生成无需 compiler，分析另需固定工具 |
| 受影响 C | `uv run --locked python -m autosar_tooling c-check --target <实际target> --project <封存工程> --output-directory <新目录>` | 对应宿主/compiler、固定 Cppcheck/addons；error/诊断/人工结论分开 |
| host错误/构建模式 | `uv run --locked python -B -m unittest discover -s tests/python/native -p test_host_boundaries.py` 或 `test_ecu_build_modes.py` | 必需目标工具预检；test宏故障入口不进入生产 |
| 真实集成/OS | Cargo `--features native-tests`、`--features official-oracles` | 目标身份与合法资源先核对；无工具默认 tests不替代 |
| 进程/工具 | `uv run --locked python -B -m unittest discover -s tests/python/process`；普通 `tests/python` | Python与平台原语按测试要求，单调 deadline和私有日志保持 |
| 独立构建/交接 | 包内 `python tools/ecu-tool.py build/verify`；`test_offline_delivery.py` | 匹配 CPython/GCC/binutils/Git、新外部build目录及不可用checkout场景 |
| 受影响桌面行为 | `autosar_tooling desktop --platform <目标> --binary <实际构建> --output-directory <新目录>` | 实际隔离桌面／driver／工具先满足，installed模式按其独立条件；不退回用户桌面 |
| 资产/开发说明 | `autosar_tooling assets check`、`quality --base <基准>`；文档链接／内容检查 | assets update须在源码/ABI/第三方身份审阅后，不能以更新掩盖契约变化 |

## 已知、待核查和阻塞的分界

| 当前情况 | 可先做 | 不能据此放行 | 解除条件 |
| --- | --- | --- | --- |
| 三族/双目标实际来源、固定依赖、历史分析完整但源码失败已盘点 | 核对范围、模块责任与独立风险，整理规格／故事设计 | 不关闭真实违反、不视为当前代码全部符合 | 对应模块规范核定、诊断裁决、实际重新生成与必要测试 |
| Q1合法全文／适用条款未满足 | 找固定官方／授权资料、核对已有摘要与接口证据 | 不做缺依据的标准判断／相关标准化实施，不关闭符合性出口 | 模块资料可读／摘要核定、条款与支持配置/角色对应 |
| Q2公开契约迁移尚未核定 | 调用方调查、兼容选项与迁移设计 | 不改公开ABI／wire或自动重写旧包 | 唯一契约与消费者同步方案、独立预期、审阅及授权 |
| Q3诊断与人工仍未评估 | 逐模块核查、最小反例、采用代码责任调查 | 不批量suppression/误报、不启用宣称可通过的最终严格出口 | 真实问题消除、适用性/误报证据、必要人工全部完成 |
| Q4性能效果与门禁尚未实测 | 沿既有日志找热点、设计可比局部和完整验证 | 不宣称已经提速或实际保护合并 | 同覆盖/工具前后证据、真实PR阻断和必需保护接线 |

Story 进入时逐项填写实际范围、原文、source/header/template/配置/调用方/清单、独立正反向量、测试和工具前置、兼容影响、共享接口负责人及依赖结果。P0/P5研究无需等全部整改；模块实施按具体依赖放行，不假设全并行或全串行。本规格不产生 ready-for-dev 状态。
