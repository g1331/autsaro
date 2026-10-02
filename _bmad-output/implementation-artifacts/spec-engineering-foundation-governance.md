---
title: '全生命周期工程基础治理'
type: 'refactor'
created: '2026-10-01'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '834b9b996e6e7592ab73a6c788f5a38287796aea'
context:
  - '_bmad-output/planning-artifacts/architecture.md'
  - 'README.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 现有工作台开发入口、Windows原生进程与FreeRTOS端口、目标资源、交付构建及状态归属彼此缠绕，不能可靠地在Windows/Linux交付并从安装包搬移复验；未来模块缺少一致的工程链。

**Approach:** 按用户批准的完整阶段0→8实施，每阶段实测正反路径及下一阶段准入，顺序为环境、POSIX owner、Linux受控OS、目标/资源/纯源码、双目标生产ECU、显式语义策略、开发工具、统一工作台状态、正式分发。用户在本会话批准的内联计划是完整实施与验收契约；本spec仅承载过程与结果，不缩减其中任何要求。

## Boundaries & Constraints

**Always:** Windows/Linux原生验证分别完成；26个OS独立消费者实际运行；旧host-v1与Epic4可观察合同保持；R24-11外部档案不打包；Windows/Linux每阶段出口当场验证；`BASE=834b9b996e6e7592ab73a6c788f5a38287796aea`；使用受控后台/隔离桌面，保留用户已有文件。

**Never:** 用Linux交叉编译或Windows模拟代替原生运行；用旧绿色测试替代受影响路径回归；更换FreeRTOS核、降低watchdog或悄悄扩合法工具白名单；触及用户输入桌面；声明Mac原生已验证、MCU/ASIL/标准认证；恢复独立assurance状态体系。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 新克隆开发首跑 | Windows/Ubuntu合法档案及锁定依赖 | doctor、CLI、核心与桌面构建成功 | 缺件、版本或hash不匹配非零，原文件不动 |
| 受控原生执行 | 父子孙、nested scope、故障退出 | 已登记合作组完整关闭且日志保留 | supervisor失败/未证实清理不得报告成功 |
| 双目标交付 | 同一validated plan分别输出Win/Linux | 各自在本机独立build/verify，搬移闭包逐字节相等 | 多余/篡改/旧格式/不支持目标拒绝且旧目录不变 |
| 工作台并发与分发 | 项目或目标在运行中切换、无checkout安装 | 过期操作不提交；真实IPC与捆绑产物能完成主路径 | 取消保留旧状态并关闭受管进程 |

</frozen-after-approval>

## Code Map

- `scripts/autosar_tooling/{cli,verify,quality,protocol_oracles,input_oracles,bsw_catalog,os_suites}.py` — 开发单入口与独立 oracle；原维护顶层脚本已删除，不留代理。26个OS suite在all内仅经Cargo注册执行一次。
- `core/src/execution/{mod,unix,windows_job}.rs`, `core/src/integration/handoff.rs` — 同一 root owner/Windows Job 与 argv/单调截止时间；阶段4离线工程和legacy实际运行已迁入，旧link_check和PowerShell runner删除。
- `runtime/os/`, `runtime/ecu/`, `third_party/freertos/portable/ThirdParty/GCC/Posix/` — 固定内核与受控 Win/Linux host adapter；Linux生产ECU已接入并运行有界协议，完整工作台/分发验收仍按阶段7/8。
- `core/src/integration/ecu.rs`, `core/src/generator.rs`, `core/build.rs` — 计划、静态资源、纯生成及封存入口。
- `src-tauri/src/{lib,workbench}.rs`, `ui/src/workbench/useWorkbench.ts` — 单一后台Session与前台reducer owner、typed fingerprint IPC、immutable snapshot及序列化staging提交。
- `README.md`, `runtime/README.md`, `docs/project/OWNER_GUIDE.md`, `_bmad-output/planning-artifacts/architecture.md` — 能力/安装/职责文字须同步实际门结果。

## Tasks & Acceptance

**Execution:**
- [x] 阶段0：锁环境、doctor、官方Tauri CLI与新克隆Windows/Ubuntu阶段出口。
- [x] 阶段1：root owner、Windows Job统一及Python/Rust正反probe阶段出口。
- [x] 阶段2：Linux受控OS port及两平台26 suite独立阶段出口。
- [x] 阶段3：目标/资源/schema/PreparedProject纯源码双平台阶段出口。
- [x] 阶段4：双目标ECU、离线包build/verify及搬移阶段出口。
- [x] 阶段5：语义策略替代Epic宏及legacy/integration回归阶段出口。
- [x] 阶段6：工具单入口及职责拆分，双平台all阶段出口。
- [x] 阶段7：统一状态与隔离Windows/Linux真实IPC阶段出口。
- [ ] 阶段8：本机bundle与无checkout真实交付，双平台最终全量门及文档。

**Acceptance Criteria:**
- Given each stage's declared prerequisites, when its real positive and negative entry points run, then its stated exit gate passes before the next begins; failure stays attributed to the relevant stage and does not lower the final scope.
- Given installed Windows/Linux applications and sealed sources outside checkout, when legitimate inputs are opened and edited then generated, built and verified, then both target's independent CAN/DID/N_Cr contracts and exact closure hold.
- Given no isolated macOS native session, when results are reported, then its code/config/entry point exist but native build/IPC remain explicitly unverified.

## Implementation Notes

- 起始提交 `834b9b996e6e7592ab73a6c788f5a38287796aea`；`git status --short --branch` 显示干净的 `master`。当前已观测Windows rustc/cargo 1.98.1、Node 24.19.0、npm 11.17.0、Python 3.12.9、uv 0.11.21；WSL Ubuntu24.04 x86_64当前root环境只发现python3/git/curl/apt，未发现uv/cargo/gcc/node/pkg-config。尚未对任何阶段宣称通过。

- 阶段0进行中：Windows 用户级记录 vcpkg/libclang 环境变量（当前Agent进程不会自动继承，原生复验时显式传入）；固定 GCC 16.1.0 Rev5 SHA-256 `d38d4dd6bea387499487881383e644ab7c193ac8f8364dc4252d6e6cc09700e2` 与原合同一致。`uv lock`/`uv sync --locked --group quality`、npm官方registry锁迁移、`npm ci --prefix ui`、`npm run build --prefix ui`、`npm run tauri --prefix ui -- info` 已在无venv/node_modules的独立Windows源码副本通过。原有217个npm包的版本和integrity保持不变，镜像resolved URL清零，新增CLI锁到2.11.0；Windows doctor在指定合法XSD/MOD下ready，换入不符SHA的档案非零`version_mismatch`。新Python包的两个archive反例单测、Ruff与增量`quality.py --base`已通过。
- Ubuntu24.04 WSL2在ext4独立源码副本安装发行版签名GCC13.3.0及Tauri/Linux构建前提，安装Rust1.98.1/rustfmt/clippy、Node24.19.0/npm11.17.0、uv0.11.21/CPython3.12.9；`uv sync --locked --group quality`、`npm ci`、UI构建、Python单测、doctor与`tauri info`已通过。首次core `cargo test --locked --no-run`暴露现存Windows-only PowerShell协议综合helper无条件引用五个`cfg(windows)`的legacy测试，已对该Windows-only综合入口与模块一致施加同条件编译；随后Ubuntu core `--no-run`通过，但它不是Linux OS/ECU行为证明。Linux原生测试覆盖仍按阶段2/4迁移，不能把这个旧Windows runner算入适用26项。
- Windows本轮实际`cargo build --locked --manifest-path src-tauri/Cargo.toml`成功，原工作树`RUST_TEST_THREADS=2 cargo test --locked --manifest-path core/Cargo.toml`返回117通过；独立新源码副本的完整core门及原生GUI隔离启动尚在执行/待执行。阶段0未标通过，阶段1尚未开始。
- 首次独立Windows源码副本运行`uv run --locked python scripts/verify.py --scope core`得到7个lib单测通过、end_to_end中109通过1失败：独立ARTI消费脚本导入`lxml`，原全局Python有5.4.0而锁定quality环境未声明。将`lxml==5.4.0`加入`quality`组并更新`uv.lock`，新副本`uv sync --locked --group quality`后精确重跑`epic4_arti_description_and_hooks -- --exact`通过，单独core Clippy通过；此前其余109项在同一代码/目标下已通过，不虚构第一次全绿。Ubuntu与Windows两份环境同步该依赖，两个doctor反例单测再次通过。独立桌面及CLI dev仍是阶段0剩余验证。
- 阶段0出口：独立Windows源码副本`cargo build --locked --manifest-path src-tauri/Cargo.toml`和`cargo build --locked --bin package_host_reference`通过，既有ECU生成/编译与legacy拒绝/回归由本轮core集成用例实际执行。`uv run --locked python scripts/epic4_desktop.py --binary <独立源码副本的真实桌面程序>`在Windows私有Desktop/Job中返回`epic4_isolated_native_ipc PASS`，用户输入桌面未切换。Ubuntu ext4副本core `cargo test --locked --no-run`、desktop `cargo build --locked`通过；私有`:188` Xvfb且强制`GDK_BACKEND=x11`实际执行`npm run tauri --prefix ui -- dev --no-watch`，真实WebKit显示起始页面；首次导入输入工具未保留换行，拼接来源路径后原生UI的`open_project` IPC返回后端`Not a directory (os error 20)`，没有伪造成功或修改旧Workspace。改为七行路径后尚未提交正例，Linux GUI正向编辑/保存/交付属于阶段7的全路径验收；当前Linux OS/ECU仍未验证。Windows正向IPC已通过；私有Linux Tauri、Xvfb和openbox退出后检查无残留，临时输入脚本已移除。阶段1准入满足。
- 阶段1出口：Python stdlib `ecu_tools.owner/process/launch/windows_job` 与 Rust `core::execution` 共用POSIX私有socket/capability、受控gate登记与绝对单调deadline；Win侧复用迁移后的Job，原integration旧runner删除，原`handoff`改用同一Job实现。Ubuntu ext4真实执行`.venv/bin/python -m unittest autosar_tooling.test_process`返回11通过（含guardian被杀、supervisor被杀、release失败/未启动、escape显式`cleanup_unconfirmed`且fixture清理自身PID）；`AUTOSAR_PYTHON=<锁定虚拟环境解释器> cargo test --locked --manifest-path core/Cargo.toml execution::tests -- --nocapture`返回6通过，真实检查PID及stderr、退出码、nested/sibling和guardian镜像。Windows锁定Python入口`uv run --locked python -m unittest autosar_tooling.test_process`返回5通过/6项POSIX专属跳过；同样Rust筛选`execution::`返回5通过（含已有Job assign失败在执行前关闭命令）。Windows受改动的`epic4_independent_handoff -- --exact`真实执行203.79s通过；`quality.py --base`及Ruff检查通过。`cargo clippy --lib`返回0但有11条原integration既存lint警告；附加`-D warnings`因此返回101，错误均指向未改的`artifacts/configuration/ecu/graph/plan/mod`，不能表述为零警告或降低规则。原`link_check`/产品工具尚未迁入owner，按阶段4执行，不据此宣称Linux ECU通过。
- 阶段2固定来源：仍用 FreeRTOS V11.3.1 commit `054e14f3397023aa83813a65aa065fc4597d481b` 与原归档 SHA-256 `dd832f699e6ebee00366d7c737e259860d9d1fd5f2c2311c90dff16e8cfbcf61`；四份未改 POSIX 原件由 `posix-source-manifest.json` 校验，Windows 十四补丁未改序列，Linux 依次从原件应用公共 0003/0007/0008 的适用文件与 `patches/linux/0001-controlled-posix-port.patch`。Ubuntu24.04 x86_64／WSL2 发行版签名 GCC13.3.0 `13.3.0-6ubuntu2~24.04.1`、binutils `2.42-4ubuntu2.10`：gcc SHA-256 `1b99826121ae6682a634e5efe09bd3e3df58ce58e0b28f849114ab5b89139c26`、objdump `325c4205a4c658a9d1e1ebc469ae55975a2b897a3d3c1e79d9b158612d37f745`、nm `3708df3d7ce16e2c1dcdcd722784ae382d4d582276a71ab7bbe2d542f5621202`；专属 `runtime/os/toolchain-linux.json` 已锁且执行时逐项校验。
- 阶段2机制：Linux Task、唯一 S dispatcher、主／备用健康控制及 host actor 各自使用实际 mmap+pthread 栈、低端 PROT_NONE guard 和预留 sigaltstack；静态代际 SIGRTMIN／futex 停车与独立 fault-stop，注册 guard 故障由健康控制报告；Task 的 FreeRTOS 元数据栈与原生执行栈分开。S 独占 Cat1/Cat2/嵌套 ISR、显式 tick 与单次 `vTaskSwitchContext`，host producer 只 publish/wake；port gate 与 mutex 持有期屏蔽普通 park，T/S/C/D/I/B 的真实 guard、双控制耗尽和未知故障都被独立 C99 消费者检查。Linux ARTI C 消费者额外检查 `ucontext`／真实 SP／guard／altstack／metadata 分离、ELF64 目标和四字节公开原子 cell，Windows 保留 PE/TLS/线程保证区。`runtime/ecu/build.ps1` 已纳入 `os/src/host/windows` 的真实 Windows event adapter，避免仅 OS harness 正常而生成 ECU 缺符号；早期受影响 SC1 回归曾暴露 `Os_HostSetEvent` 未链接，修复该根因后通过。
- 阶段2独立运行：Windows与Ubuntu24.04各执行 `uv run --locked python -m autosar_tooling os --target <各自目标> --suite <逐项26个>`，26/26 suite 的独立 C99 原生编译／运行和手写正反 oracle 均通过；Ubuntu stack 内额外 ARTI 9项Task/tool＋26项嵌套／拒绝与两项已编译故障拒绝。两平台 `cargo test --locked --manifest-path core/Cargo.toml --test end_to_end epic4_` 严格筛选 OS 注册项各为26通过、0失败，未把 Linux 产品 ECU 测试计入 OS 结果。原生 `lifecycle` Windows46／Linux47向量，Linux stack23、nested26、time44、sc1-timing43、resources26。`uv run --locked python -m unittest autosar_tooling.test_os_suites` 两平台各6通过，Windows Ruff 与 `scripts/quality.py --base` 通过。Windows受影响的 `epic4_sc1_timing_capacity -- --exact`（224.17s）、`epic4_arti_description_and_hooks -- --exact`（189.78s）、`epic4_generated_counter_service -- --exact`（192.49s）、`epic4_independent_handoff -- --exact`（205.62s）各通过，均为实际生成／编译／运行。
- 阶段2能力边界：Ubuntu 的 `epic4_sc1_timing_capacity` 只执行原生 OS 独立消费者；旧 `epic4_timing::generated_tables` 是 Windows 产品工程验证，显式 `cfg(windows)` 至阶段4 Linux ECU 工程接入，不把其失败掩盖成已支持 Linux ECU。Windows 旧 host-v1 与 Epic4 既有产品行为按当前受影响目标回归；其他 Linux 发行版、CAN/DID 完整 ECU、Mac 原生 GUI/IPC、实机和标准／MISRA 合规认证未因此获得验证。
- 阶段3纯源码出口候选：`BuildTarget` 固定 Win/Linux 两字面量、ABI/原生port/patch作用域/目标锁/段与产物约束；`core/build.rs` 按本地资产清单、既有BSW map与FreeRTOS来源manifest核查 digest/许可/路径，生成唯一静态 `include_bytes!` 索引。`RuntimeCatalog::embedded()`与显式 `AssetInventory::from_directory` 使用编译可信身份；XSD由私有RAII目录按固定SHA和成员白名单解包，验证结束后释放，不再复用PID/mtime缓存或共享锁。核心 `prepare_ecu_project`/`prepare_host_project` 仅借可信原字节与渲染生成内存预览，`target.json` 指出未安装/未预检且Linux ECU bridge仍未交付；legacy安全档案在Linux准备边界拒绝，不伪造安全backend。Tauri app-config `settings.json` 与进程环境覆盖进入后端，既有Windows产品仍用checkout runtime直至阶段4。
- 阶段3核心已实测：Windows与Ubuntu ext4副本分别以锁定合法XSD/MOD及绝对CPython运行 `cargo test --manifest-path core/Cargo.toml --test end_to_end source_generation_does_not_require_native_executor -- --exact --nocapture`，均1通过；两端相同输入同得34个BSW受控资产，Win fingerprint `4767a95bea5d0e6a89f2718a4e473b882a33264cd4dfcacf05b041e1b81e77cd`、Linux fingerprint `910e77c7b970e0c4ca6ada0f7d0350b6384a4ef3754788f3e37c30777877f4d6`（目标不同、平台运行相同字节），不存在compiler仍可生成预览且preflight=`not_run`。两端 `source_generation_rejects_modified_validation_and_asset_bytes -- --exact` 与 `linux_legacy_security_profile_is_rejected_during_preparation -- --exact` 各1通过，错路径、坏ZIP、同尺寸改动XSD/MOD、可信资产改动与Linux BCrypt安全档案均拒绝。Win/Linux `doctor --role native --target <本机目标>` 的Git、GCC triple/hash、binutils、CPython与外部档案均ready；Windows缺GCC及XSD反例非零。Windows `cargo build --manifest-path src-tauri/Cargo.toml`、Ruff与 `scripts/quality.py --base 4e71799`通过。
- 阶段3产品兼容实测：首次私有Desktop真实IPC暴露Tauri默认资源路径残留 `..` 而被显式路径契约拒绝；修为从package父目录取规范路径、重建后隔离桌面实际完成标准输入正反校验、ECU源码预览/生成、`ecu_host_batch.exe` 构建、真实 CAN/DID/N_Cr 恢复及非法批拒绝。截图 `ecu-post-save-verified.png` 显示保存、校验、生成、构建及主机行为已通过；该整套GUI脚本在后续legacy部分被外层360秒截止中断，不报告整套通过。独立旧目标 `generated_handoff_builds_and_runs_after_moving_without_the_workbench -- --exact` 1通过。此阶段仅已有Windows产品和双目标纯源码选集，不将 Linux OS 与未封装源码说成 Linux 生产ECU。

- 阶段4实现：ECU生产桥接以阶段2 host adapter 提供事件、锁/原子和宿主时钟，保留owner、ring、真实write/flush确认和5000ms COMMIT watchdog。`tools/ecu-tool.py build|verify`为交付的stdlib-only入口；目标/工具链/源选集来自封存metadata，四种显式mode区分legacy、生产HostBatch及独立probe/test，私有patch副本、独立空输出、PE/ELF/TLS/sections和源闭包在实际工具执行中检查。legacy generator、重导入、Rust/Tauri和固定双ECU参考包已切换；`build.ps1`、`verify.ps1`、`process-tree.cs`及旧legacy构建模板删除。源码准备/预览/重导入不编译，预检独立记录，不把跨目标not_run当成功。
- 阶段4已运行：最终OS等待边观察修复后，Windows `windows_and_linux_ecu_targets_execute_production_protocol -- --exact --nocapture`实际138.26s通过，Ubuntu ext4同一入口实际28.52s通过；各自正例经handoff、搬移、逐字节重导入、显式native preflight、外部空目录build和独立verify，真实echo五批、N_Cr恢复四批、非法接纳与至多两DID边界通过。Windows `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults`、`security_access_roundtrips_and_gates_host_writes`、迁移后的`epic4_independent_handoff`（102.17s）、`generated_handoff_builds_and_runs_after_moving_without_the_workbench`（11.36s）、固定参考包`moved_reference_bundle_verifies_offline_and_rejects_wrong_vector`（43.43s）均各1通过；参考包真实拒绝错误向量、1ms绝对deadline、缺编译器、linked report及缺原输入，并保留失败报告/日志。Windows原生ECU完整集成入口327.34s通过。两平台UI TypeScript/Vite与desktop `cargo build --locked`均已通过；本阶段入口和生产关口通过，不把此结果替代后续全量／正式bundle验收。
- 阶段4过程根因与回归：Win32 verbatim `\\\\?\\`路径传入GNU工具导致源文件不存在，包内统一转换为DOS/UNC工具参数后实际legacy编译/闭环通过。新增真实失败assignment回归先观察到`Assign suspended command`错误被5秒`TimeoutExpired`覆盖且悬挂子进程未退出；Python Job现记录assignment，在分配失败时通过自有句柄结束未加入Job的suspended子进程。锁定Python的assignment失败与三代timeout两测试合计1.040s通过；PID检查申请SYNCHRONIZE并拒绝WAIT_FAILED，不能把查询失败当已关闭。Rust `parent_first_and_timeout_close_the_registered_tree`精确回归8.95s通过。
- 阶段4CLI拒绝回归：`requested_native_preflight_failure_never_installs_source -- --exact`先实测失败（native preflight failed却以成功退出并安装源码）；显式failed报告现在先输出JSON、非零退出且不安装源码。修后该真实CLI入口9.07s通过，原输入逐字节不变。非本机目标仍可纯源码准备，preflight=not_run且不要求构造本机执行工具。
- 阶段4原生ARTI根因：Linux host actor可先占用动态栈槽，消费者已改为按唯一角色S查找并保持真实SP范围检查，不把slot0当契约。真实20个ECU epoch完成后ARTI仅有Start=2/Wait=1/Release=1，暴露延迟native切换前已发布完成收据、下一epoch可使WAITING边消失；共同backend现在在挂起事务持锁期、收据发布前记录已提交的等待谓词，不改协议时序/5000ms watchdog。保持原Start>20/Wait≥20/Release≥20、原生PC/SP、getter、两次错误/拒绝和ISR成对断言，`epic4_arti_description_and_hooks -- --exact`最终Ubuntu36.44s／Windows118.81s均1通过。
- 阶段4OS基线去重：独立ARTI的9项Task/tool、26项嵌套/拒绝及两项编译故障拒绝随stack suite在两平台各执行一次；从ECU ARTI产品用例移除重复的OS-only调用，三个真实ECU配置消费者保留。最终 `autosar_tooling os --target <native> --suite stack` Windows127.45s／Ubuntu与生产入口组合49.34s均通过，stack仍为23个原生向量。增量hygiene/rustfmt/clang-format/Prettier、Python/C99 syntax及现有两个Python包Ruff全部通过；没有以OS suite代替生产ECU证据。
- 阶段4真实桌面：现有Windows私有Desktop/Job/CDP完整消费者首次364.47s通过；加入“显式编译预检”的实际按钮/IPC后最终421.60s通过。覆盖导入、合法编辑/保存/重开、源码预览、独立预检、取消与确认生成、真实build/协议验证、已有binary保护/恢复、篡改源拒绝、搬移/重导入/精确再生，以及host-v1/未知格式保持旧workspace。实际截图 `ecu-source-preview.png` 与 `ecu-post-save-verified.png`已读取，显示显式target/外部build目录、预检按钮与本次保存/校验/生成/构建/主机行为；程序窗口仅在私有Desktop，输入桌面未切换。Linux真实GUI仍按阶段7验证。
- 阶段4fresh checkout根因：实际 `git checkout-index`产生新Windows副本，92项封存资产中11项工具/嵌入README因autocrlf转写而摘要错误；`.gitattributes`现固定stdlib工具与交付文本的LF，不改原有BSW/内核字节规则。新独立checkout的92项实际原始SHA-256全部匹配，0差异；临时审计脚本及两份副本已删除，不留共享文件或新增验收账本。
- 阶段4独立本地提交：`9eac2a633b9e58afb276ef4c43ffb45f6d85ec0c`。阶段5采用每工程唯一生成的const `Ecu_Policy`及const Rx/Tx route表；发送、宿主时钟与DCM admission/pending/process由所链接execution adapter负责，DCM核心保留服务业务与静态策略。payload容量由legacy明确256及集成validated plan分别提供，不以目标宏猜默认值；OS waiting hook在原持锁quiescence/收据前边界链接。LSP对C无可用server，引用范围已通过当前源码/CodeGraph及已知符号搜索核对；正式双profile行为入口与两平台生产关口仍待本阶段实现后运行。
- 阶段5双profile行为已实测：`semantic_profiles_preserve_host_and_integrated_protocol -- --exact --nocapture` Windows44.15s／Ubuntu12.90s各1通过。独立C99消费者编译交付的真实legacy与native执行adapter；同步实际写/flush成功、回调内撤销CanIf导致未确认`ECU_ERR_IO`、恢复后再发成功，native sink真实flush前当前epoch收据仍`E_OS_NOFUNC`，flush后由原mailbox提交。覆盖实际/8字节DLC、WAIT-zero重置/abort及恢复、N_Cr同截止时刻time-before/RX-before差别、默认/扩展DID、0xf186与unsupported跳过、两DID请求顺序、legacy三DID/native第三DID拒绝、P2=50及P2*=500/5000ms、SID0x31拒绝，以及legacy真实256字节response overflow NRC0x14与后续恢复。原生消费者完成214个epoch，无增大5000ms watchdog或更换已验证port。
- 阶段5回归根因：const route迁移最初未保留CanIf当前配置的Rx方向/连接守卫；新增公开C消费者在Tx-only重初始化后实际失败，修回活动配置守卫后同一入口1.29s通过。Windows十项选定legacy CAN/诊断/故障/安全/写/例程/CanTp回归首轮9通过，DTC安全用例被已失效的README命令字串断言阻挡；删除该措辞断言而不重钉文字，保留实际构建/诊断断言后该用例11.34s通过。所有四个直接Can/CanIf/CanTp编译消费者显式选择profile容量与所需adapter，公共接口不留旧target别名。最终增量质量27.71s通过；两平台新waiting hook的独立stack/ARTI 23向量＋9项Task/tool＋26项嵌套/拒绝及两项已编译故障拒绝通过，Windows123.19s。不以此替代生产ECU。
- 阶段5完整出口：最终Windows/Ubuntu `windows_and_linux_ecu_targets_execute_production_protocol -- --exact`各140.31s／34.15s通过，包括实际生产object私有probe符号拒绝检查及混合HostBatch/control构建非零、无输出安装。CanIf活动守卫修复后的两平台生产echo/N_Cr/两DID/畸形输入不退化。最后将不再属于BSW直接依赖的legacy `Os.h`归入显式host-clock资产，BSW catalog实际C99 link/address/type消费17项producer通过；双平台纯生成与最终semantic入口仍通过，Windowssemantic44.63s、Ubuntu两入口合计13.01s。源码中旧总宏/include guards/target Dcm接口及旧payload别名零匹配；Ruff通过。实际staged fresh checkout核查127个资产原始SHA-256，0差异。阶段5验收通过，进入阶段6；正式GUI/bundle与最终all仍按原范围执行。
- 阶段6实现：剩余维护脚本及单测迁入`autosar_tooling`，所有原普通fixture reader保留独立argv消费；聚合CLI注入自身绝对CPython，动态`-c`及bare-python切换到命名probe。`generator/{render,output}`、`host/{profile,process,protocol,scenarios}`按责任拆出，`end_to_end`保留原精确测试名并移动完整行为到support。真实子进程统一OwnedProcess及绝对单调deadline；虚拟环境/rustup可执行路径保留symlink身份，不canonicalize到错误解释器/多调用程序；显式新POSIX owner不继承无关外层scope。嵌套supervisor关闭后不可执行zombie与活成员明确区分，不把仍运行后代视作已清理。
- 阶段6回归：原非空build目录、旧包再生、公开OS服务符号竞争和Cppcheck工具定位分别以实际编译/拒绝路径修复；删除失效README措辞断言，保留真实运行与文件归属断言。公共生成`OsService`/`Rte_Call_OsService`符号及OS客户端header竞争在生成前拒绝，不用编译失败掩盖模型冲突。Windows真实编译期间修改封存源，构建非零且不安装binary；已有修改binary继续保留。命名`protocol-oracles`与`input-oracles`实际执行通过。
- 阶段6Ubuntu最终完整门：`uv run --locked python -m autosar_tooling verify --scope all --base 5c4164b`在Ubuntu24.04 ext4原生副本返回PASS，323.30s；Python29项（1项Windows专属跳过）、Rust lib11项、core集成81项全部适用通过，26个OS suite仅在该Cargo测试中一遍；包括Linux生产CAN/DID/N_Cr协议、源码/档案篡改拒绝、搬移精确再生与实际ARXML保存。npm依赖、增量质量、Ruff、UI lint/build、core Clippy、desktop build/Clippy全部通过。owned日志`/tmp/autosar-verify-rn5hqndj`；原生GUI/IPC和bundle未由此执行，仍属阶段7/8。
- 阶段6Windows最终门尚未通过：同一all命令先通过Python29项（7项POSIX专属跳过）、Rust lib10项及core集成116项（967.42s，包含单次26个OS suite和真实生产协议），UI lint/build、增量质量及Ruff均通过；core Clippy退出0后owner返回`orphaned_members`，故整体非零，未宣称全绿。owned日志`C:\Users\admin\AppData\Local\Temp\autosar-verify-iucqovmr`。独立冷编译的Job成员列表实际定位仍运行的`D:\VS\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\vctip.exe`，不是Job accounting延迟；不放宽成员检查、增加grace或忽略退出状态。
- 阶段6Windows剩余门定向检查：`uv run --locked python -m autosar_tooling verify --scope desktop --base 5c4164b`实际desktop编译退出0（23.59s），同样因root退出后仍有成员而返回`orphaned_members`，owned日志`C:\Users\admin\AppData\Local\Temp\autosar-verify-jvwife3s`；desktop Clippy未执行，不把缓存编译通过替代owner要求。机器级权限是当前阶段准入阻塞，不是Linux运行证据或Windows测试断言失败。
- 阶段6环境权限阻塞：仅对子进程设置`VSCMD_SKIP_SENDTELEMETRY=1`、`VCTIP_DISABLE=1`以及安装编译器中可见SQM开关分别冷编译，均仍是`orphaned_members`，未写入项目配置。用户已明确批准按Microsoft官方VSCEIP注册表`OptIn=0`关闭Build Tools可选遥测；已读取`HKLM\SOFTWARE\Wow6432Node\Microsoft\VSCommon\17.0\SQM`原`OptIn=1`，首次写入即因“注册表访问权”拒绝，未改原值，尚未触及policy键。当前Agent令牌无该写入权限；需管理员在上述官方键及`HKLM\Software\Policies\Microsoft\VisualStudio\SQM`执行授权设置后先冷编译验证，失败则恢复原值。官方依据：https://learn.microsoft.com/en-us/visualstudio/ide/visual-studio-experience-improvement-program?view=visualstudio 。阶段6保持未完成、未提交，阶段7/8没有提前开始。
- 阶段6权限原状态补记：只读查询确认`HKLM\Software\Policies\Microsoft\VisualStudio\SQM`整个key原不存在，当前principal `ElevatedAdministrator=False`。因此恢复授权变更时首项`OptIn`应恢复为1，新policy项应回到原不存在；不静默删除其他注册表内容或在用户桌面请求UAC。
- 阶段6当前决策更新（取代上述权限阻塞路径）：用户明确要求采用工程上最佳处理并继续执行，授权将“业务完成”和“受管树回收”分开建模。Python/Rust `CompletionPolicy`默认`require_tree_exit`；仅开发聚合器的Cargo test/build/Clippy使用显式`close_tree_on_exit`。root正常结束后先有界关闭所拥有的Job/已登记scope，再确认零残留并保留root原退出码；实际回收通过`descendants_reclaimed`结果及阶段日志公开。非零、超时、取消、逃逸、清理无法确认仍失败；协议及生命周期用例继续严格，不按`vctip`名称加白名单、不增加grace或5000ms watchdog、不更改机器遥测。不再依赖管理员设置；此前失败尝试与原值仅保留为历史证据。
- 阶段6完成策略初步运行证据：Windows真实Python owner16项（7项POSIX专属跳过）通过；随后Ubuntu真实owner17项（1项Windows专属跳过）通过，新增正常/非零root退出后关闭父子孙、同级活任务不受影响、超时仍失败以及两种策略下未完成注册嵌套scope的回收。Windows Rust `execution::tests`7项通过，实际核对被回收PID和仍运行的sibling；原严格parent-first反例仍`orphaned_members`。一个独立冷Cargo/Clippy编译实际返回`exited`/0，但该次无残留，`descendants_reclaimed=false`；额外要求它必然产生后台服务的临时断言失败，因此不把该次说成vctip回收证明。最终双平台all及真实工具回收观察继续执行。
- 阶段6最终出口：Windows及Ubuntu分别运行`uv run --locked python -m autosar_tooling verify --scope all --base 5c4164b`，均实际PASS。Windows1182.09s：Python33项（8项POSIX专属跳过）、Rust lib11项、集成116项全部通过（1057.67s），UI lint/build、增量质量、Ruff、core Clippy及desktop build/Clippy均通过；desktop build与desktop Clippy真实出现残留后代，分别报告`exit=0 cleanup=confirmed descendants=reclaimed`，不是依赖缓存或跳过owner检查。owned日志`C:\Users\admin\AppData\Local\Temp\autosar-verify-4vjaoxt1`。Ubuntu300.54s：Python33项（1项Windows专属跳过）、Rust lib12项、集成81项全部通过（237.18s），同一完整构建/质量链通过，owned日志`/tmp/autosar-verify-m0rbm7ga`。两端各26个OS suite仅在该次Cargo测试内运行一遍；严格parent-first、工具回收、非零保留、同级隔离、超时及两策略逃逸拒绝均有真实运行结果。先前错误测试循环换行已按实际rustfmt输出修正，不重排历史文件。
- 阶段6验收通过：无机器级遥测更改、无进程名白名单、无watchdog增大，所有原工具入口完成切换并删除维护顶层旧脚本；编译可信资产清单已同步owner/process/WindowsJob实际源码摘要。GUI/IPC、正式bundle和无checkout全链未由阶段6声称完成，严格继续原阶段7/8。阶段6按用户要求独立本地提交，不push。
- 阶段7实现：`Workspace`门面保留，完整XML工具／host profile／Patch与保存事务分别迁入`arxml/{xml,host_profile,persistence}`；保存备份恢复及根owner取消回归已实际通过。Tauri单一`workbench::AppState`拥有Workspace、合法规范、显式工具、target/revision/active operation；耗时worker读取Arc snapshot，最终唯一operation锁内核对fingerprint后提交私有生成／构建staging。TS所有调用方一次迁入`useWorkbench` reducer owner，页面／表单／交付面板为受控消费者；`useDelivery`仅内部纯helper。
- 阶段7Windows完整门：`uv run --locked python -m autosar_tooling verify --scope all --base 71db38a`实际PASS，1399.53s，owned日志`C:\Users\admin\AppData\Local\Temp\autosar-verify-axwfqxvp`。core集成116项通过（1245.07s），26个OS suite只在该Cargo测试内执行一遍；Python、增量质量、Ruff、UI lint/build、core及desktop build/Clippy均通过，开发Cargo的真实后代回收保留原退出码。随后针对复核修正后的UI，lint/build再次通过；完整原生Windows私有Desktop/Job入口229.17s实际PASS，临时证据`C:\Users\admin\AppData\Local\Temp\autosar-native-windows-glhmepmw`，含真实导入／编辑／保存／预览／预检／确认／build／CAN-DID-N_Cr、旧binary保护、移动重导入精确再生、未知host格式保留旧workspace及target/input变化关闭owner并拒绝旧build发布，原生IPC记录为`native-ipc.jsonl`。
- 阶段7环境失败保持：跨系统同步命令的变量转义错误误复制编译缓存，D盘耗尽导致WSL只读回退；未修改用户源码，先移走可重建缓存。用户明确要求只重启WSL，实际`wsl --shutdown`后`df`及`findmnt`恢复ext4 `rw`，未离线修复、未注销／重装。改用显式`--exec`与仅指定源码／runtime目录的同步，删除隔离镜像中的混入build缓存后Linux桌面与同版参考包编译通过。Linux native driver目前仍在验收，不用这些构建结果推定GUI通过；已定位WSLg只读X11目录不能以socket文件判断Xvfb就绪，以及`xdotool search`无窗口的正常exit=1须用实际ProcessResult判定。macOS测试feature/capability已实现、无原生宿主，未声称构建或IPC通过。
- 阶段7Linux真实IPC首次全路径通过：`uv run --locked python -m autosar_tooling desktop --platform linux --binary src-tauri/target/debug/autosar-config-desktop`实际105.91s PASS，私有Xvfb证据`/tmp/autosar-native-linux-vz647js1`。与Windows共用场景实际完成标准输入导入／编辑／保存、取消与确认生成、生产build／CAN-DID-N_Cr、已有binary保护及恢复、篡改拒绝、搬移／重导入／逐字节再生、host-v1拒绝保持旧workspace，以及并发target/input变更取消旧build且目的地无旧工件。实际GTK截图显示Cancel左／OK右，布局会调整源码添加顺序；确认动作改为私有display内按实际窗口尺寸点击右侧按钮，不mock invoke或隐藏拒绝。
- 阶段7追加消费者回归：新`build_creates_missing_parent_directories_without_replacing_owner_output -- --exact`在Windows实际1通过（6.75s），从缺失父级的新路径构建并运行真实CAN输出，随后非空目的地拒绝重建且owner文件和binary原字节保留。新桌面场景从无规范设置首跑，核对自动设置页、合法固定hash保存、取消草稿及`DEPENDENCY_IDENTITY`拒绝保持workspace。Windows追加场景在目标切换后实际暴露旧预检成功通知仍可见：结构化report已清除，但通知保留旧日志；target/resource/tools成功失效现在同步清除旧notice，正在按同一实际消费者复验，未提前标阶段7完成。
- 阶段7Linux追加场景实际109.52s PASS，证据已改存`/root/.cache/autosar-tooling/native/autosar-native-linux-jseuwejw`并实际读取`ecu-post-save-verified.png`及`native-ipc.jsonl`；五阶段已通过，完整SC1工程和实机仍明确未验证。日志记录target切换在release前取消旧build、输入变更关闭实际pid8099／SIGTERM并拒绝最终发布，workspace保留新编辑。此前仅从env override map删除规范键不能取消owner继承值；native server改用真实`env -u`执行，故无规范首跑与UI合法配置路径实际成立，不修改owner契约。旧`/tmp`证据目录已不可读取，未推断原因，后续使用持久用户cache。
- 阶段7Windows配置隔离修正：追加首跑场景第二次发现普通用户规范设置可见，读取实际`Roaming/dev.autosar.classic-can-workbench/settings.json`证实`APPDATA`覆盖未隔离Tauri Known Folder。首跑截图证实原规范草稿为空，实际Windows预检证明原target为Windows；已回退本次测试写入的XSD/MOD及Linux target字段，不触及其他字段。新增显式绝对`AUTOSAR_CONFIG_DIR`，默认仍为Tauri app_config_dir，所有native driver指定私有目录；正在重建并复验此实际隔离路径。UI删除无人消费的preflightReport冗余缓存，只保留当前预检展示通知并在target/resource/tools成功失效时清除；全局声明dark color-scheme使原生表单控件跟随既有深色界面。
- 阶段7配置隔离修正后的最终native场景：Windows私有Desktop/Job实际269.26s PASS，证据`C:\Users\admin\AppData\Local\Temp\autosar-native-windows-53fxy14r`；Linux私有Xvfb实际113.32s PASS，证据`/root/.cache/autosar-tooling/native/autosar-native-linux-6n7nb2dn`。均实际读取五阶段通过截图与私有`app-config/settings.json`，Windows普通用户配置仍为回退后的原字段值；Linux目标select已按dark color-scheme正确显示。缺规范首跑、合法设置、取消／错误身份保持workspace、target切换撤销旧预检、正反交付及owner并发闭树均在同版真实Tauri中通过，Mac仍未原生验证。
- 阶段7最终all过程：Linux镜像缺少`71db38a`基线对象，质量门明确非零且未执行core；仅从本地Windows仓库fetch对象补齐，不覆盖工作树。随后两平台增量格式门共同发现新配置初始化表达式的换行不符rustfmt；按实际formatter输出修正两行并同步，没有降低规则或重排其他源码。两平台最终all正在运行，阶段7尚待该完整门结果。
- 阶段7Linux最终完整门：`uv run --locked python -m autosar_tooling verify --scope all --base 71db38a`实际301.51s PASS，owned日志`/tmp/autosar-verify-m5csj2jd`。Python33项（1项Windows专属跳过）、Rust lib13项、core集成82项全部适用通过（229.88s），含新增缺失build父级／非空输出保护、保存事务和root取消回归，以及单次26个OS suite和真实生产协议；增量质量、Ruff、UI lint/build、core Clippy、desktop build/Clippy全部通过。Linux真实UI/native另由上述私有Xvfb门证明；Windows最终all仍在运行，尚未提交或开始阶段8。
- 阶段7Windows最终完整门：同一`verify --scope all --base 71db38a`实际1333.77s PASS，owned日志`C:\Users\admin\AppData\Local\Temp\autosar-verify-mmztsp7a`。Python33项（8项POSIX专属跳过）、Rust lib12项、集成117项全部通过（1220.48s），26个OS suite只在该Cargo测试内执行一次；增量质量、Ruff、UI lint/build、core Clippy、desktop build/Clippy均通过，desktop build真实回收后代并保持exit0。最终UI资源hash与Linux一致；两平台完整门与真实native门共同满足阶段7出口，按用户授权独立本地提交，不push。阶段8正式bundle／无checkout路径尚未由这些结果声称完成。


## Spec Change Log
- 阶段4：离线Windows/Linux工具和legacy/交接消费者一次切换，保留独立预期与真实拒绝；新增CLI失败不安装及suspended Job assignment关闭回归。每阶段出口仍以完整实际门为准，不修改批准范围。
- 阶段5：生成const业务策略/route和显式profile容量，链接选中发送/时钟/诊断adapter与OS waiting hook；独立C99双profile行为、混合构建拒绝和生产符号隔离进入维护用例，不改变公共BSW ABI及汽车能力声明。
- 阶段6：开发命令与完整行为函数一次迁移，保留绝对解释器、单次OS注册与严格owner关闭；原生Windows遥测子进程的环境权限失败明确留在阶段记录，不修改验收范围。
- 阶段6用户授权的契约修正：开发Cargo命令采用显式root完成后回收策略；默认业务严格退出与“结果发布前零受管残留”不变。此修正撤销机器级遥测配置作为准入前提，不删除历史失败、不改变阶段7/8范围。
- 阶段7：ARXML完整事务及工作台所有权一次迁移，新增真实缺规范／设置拒绝、非空输出归属及target/input并发过期发布回归；统一平台原生入口替代旧顶层脚本，Mac仅源码工作台测试路径，不改变原生能力边界。


## Review Triage Log

- 阶段7只读双切片复核：native snapshot/commit发现1项真实回归（新build目录父级缺失时被canonicalize提前拒绝）；UI owner发现4项真实回归（legacy未应用草稿／缓存inspection导航锁，重构建保留旧build/behavior成功，取消后running阶段未终止，关闭ECU项目残留共享生成预览）。均已按当前责任边界修正：规范化最近现存父级后安全创建新build parent、inspection最终释放锁且App不私设processing、编译前撤销旧结果、epoch取消终止running stage、关闭项目清空project-owned delivery/preview。新增真实native断言核对拒绝重建后禁用按钮及预览关闭，并保留原旧包／binary字节保护断言；Windows场景已通过，Linux场景继续验收。


## Verification

**Commands:** 各阶段精确命令及真实输出追加在Implementation Notes；最终运行用户计划中的Windows/Ubuntu各自`uv run --locked python -m autosar_tooling verify --scope all --base 834b9b996e6e7592ab73a6c788f5a38287796aea`、两平台原生桌面/离线交付及bundled搬移路径。失败不标完成。
