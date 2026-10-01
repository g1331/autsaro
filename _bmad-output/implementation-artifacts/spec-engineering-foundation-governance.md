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

- `scripts/verify.py`, `scripts/quality.py`, `scripts/epic4_os.py` — 现有命令入口、独立OS消费者与硬编码工具路径；迁移不产生第二套oracle。
- `core/src/execution/{mod,unix,windows_job}.rs`, `core/src/integration/{handoff,link_check}.rs` — 阶段1统一owner入口；原Job已迁出integration，link_check待阶段4随产品切换迁移。
- `runtime/os/`, `runtime/ecu/` — 固定FreeRTOS、Win64 OS与ECU来源，Linux port须先OS后ECU。
- `core/src/integration/ecu.rs`, `core/src/generator.rs`, `core/build.rs` — 计划、静态资源、纯生成及封存入口。
- `src-tauri/src/lib.rs`, `ui/src/App.tsx` — IPC和多处交付状态归属。
- `README.md`, `runtime/README.md`, `docs/project/OWNER_GUIDE.md`, `_bmad-output/planning-artifacts/architecture.md` — 能力/安装/职责文字须同步实际门结果。

## Tasks & Acceptance

**Execution:**
- [x] 阶段0：锁环境、doctor、官方Tauri CLI与新克隆Windows/Ubuntu阶段出口。
- [x] 阶段1：root owner、Windows Job统一及Python/Rust正反probe阶段出口。
- [ ] 阶段2：Linux受控OS port及两平台26 suite独立阶段出口。
- [ ] 阶段3：目标/资源/schema/PreparedProject纯源码双平台阶段出口。
- [ ] 阶段4：双目标ECU、离线包build/verify及搬移阶段出口。
- [ ] 阶段5：语义策略替代Epic宏及legacy/integration回归阶段出口。
- [ ] 阶段6：工具单入口及职责拆分，双平台all阶段出口。
- [ ] 阶段7：统一状态与隔离Windows/Linux真实IPC阶段出口。
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

## Spec Change Log

## Review Triage Log

## Verification

**Commands:** 各阶段精确命令及真实输出追加在Implementation Notes；最终运行用户计划中的Windows/Ubuntu各自`uv run --locked python -m autosar_tooling verify --scope all --base 834b9b996e6e7592ab73a6c788f5a38287796aea`、两平台原生桌面/离线交付及bundled搬移路径。失败不标完成。
