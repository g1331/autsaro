---
title: '离线 ECU 包的工具前置与失败重试说明'
type: 'bugfix'
created: '2026-10-11'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '1ed2f1d673cb767e96c5e82a019a2ae548b48059'
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/CONTRIBUTING.md'
---

<frozen-after-approval reason="delegated approval by dev-user/dev-pm under maintainer authorization 2026-10-10">

## Intent

问题：多组件生成包遗漏三项必需工具的绝对路径，误称 Git 身份受 target 锁定，并未说明失败后目录已有日志而不能原地重试；接收者无法仅凭包内说明接手。
方案：修复新生成包 README，校对直接相关 single/legacy 和公共所有权说明，补正常生成回归与独立复制构建证据。来源为 t_b6a1f960 的已冻结客户裁决、PM t_45a30c72 和实施卡 t_3123e6cf；本规格复用该裁决，不冒充真人批准。

## Boundaries & Constraints

始终：Python >=3.11；包在所选 Windows/Linux x64 原生平台执行；AUTOSAR_CC、AUTOSAR_OBJDUMP、AUTOSAR_GIT 指向真实绝对可执行路径，给对应 PowerShell/POSIX 示例。GCC 版本、triple、摘要与 target.json.toolchain 一致；objdump 仅在配置摘要时锁摘要，两平台都实际查询版本；Git 只查询版本，不宣称固定摘要。说明工具取得后匹配包内记录，不将示例路径当保证。输出与封存源码互不嵌套，新建或空目录均可；失败现场可能含 logs/kernel，保留诊断，修正工具后换另一新或空外部目录。live/sealed 所有权、旧封存及配置/源码生成无需编译器或官方档案保持。

禁止：修改 build 策略、runtime C、ABI、wire/profile/schema、旧 seal/历史包；新增 AUTOSAR_NM 或 multi 通用 verify 命令；改根 README 或开始 Story9.2；将构建等同于行为、AUTOSAR/MISRA/MCU 符合。保留 native 候选分支，不提交/推送由实施子代理执行；父 Engineer 在内部 review 后按本卡授权提交、明确 refspec 推送至产品名分支、创建 PR，独立 Tester/必需 exact-head CI 后由下游整合卡 merge。

</frozen-after-approval>

## Code Map

- `core/src/integration/multi_ecu.rs:682`：multi README producer；仅改说明，不触碰 C 字符串。
- `core/src/integration/ecu.rs:515`：single README producer，已有工具变量及模式说明，保持 verify 支持范围。
- `runtime/generated-README.md` 与 `core/src/generator/render.rs::handoff_readme`：legacy 模板及插值入口。
- `core/src/generator/delivery.rs::append_native_readme`：公共 native v2 所有权，原有 live/sealed 描述保留。
- `tools/python/src/ecu_tools/build.py::pinned_tools/build`：只读契约依据，GCC 必锁，objdump 摘要可选，Git --version；输出失败保留现场。
- `runtime/os/toolchain{,-linux}.json`：实际锁；Windows GCC16.1.0 MSYS2 Rev5，Linux GCC13.3.0/binutils2.42；不得笼统声称 Windows objdump hash 已锁。
- `core/tests/multi_component_contracts.rs`、`core/tests/support/generation.rs`、`core/tests/support/epic7_delivery.rs`：扩展现有生成/所有权测试，复用双 target 正常入口；native-tests 下有真实 sealed standalone build。
- `docs/development/testing.md`、`pyproject.toml`：正式测试/资产摘要入口。模板资产改动显式 assets check/update，仅接受自有 README 摘要变化，核对 .gitattributes 换行。

## Tasks & Acceptance

执行：
- [x] 修改上述三个 README 来源与公共相关说明，使用现有来源复用方式，不新增运行时或通用工具框架。
- [x] 扩展正常生成测试覆盖两 target 的 multi/single/legacy README 完整前置、平台示例、真实工具锁、输出和失败重试，原有所有权断言保留。
- [x] 运行相关 Cargo/正常质量与资产检查；实际生成新包、移到独立目录，仅声明工具构建，确认源字节不变；记录 Linux 本地和 Windows CI 证据边界。
- [x] 完成 Build 隔离 review、修复真实问题并记录同候选证据，准备移交同卡 dev-tester；保留未完成独立 QA/CI/整合出口。

验收：
- Given 正常配置与任一目标，when 新生成 multi/single/legacy 包，then README 包含完整前置/相应平台示例与安全重试，工具锁语义符合实际实现，生成不启动编译器。
- Given 新生成包的独立复制和 README 声明工具，when 包内 Python build 正常入口执行，then 不依赖 checkout/uv/Rust/Node/PYTHONPATH 即真实产出二进制，源字节不变。
- Given 缺失各必需工具或错误 GCC/已锁 objdump 身份，when 构建失败，then 非零、源与已有输出字节保持、诊断保留，同失败目录拒绝，修正工具并用另一空外部目录成功；此条件由同卡 Tester 独立补证，不以 Engineer 自测冒充。
- Given 旧包/用户 live 源码，when 新生成说明改变，then 不静默升级或重写旧包/源，配置与源码准备仍无编译器/档案依赖。Windows 原生行为仅由锁定工具链的原生/CI 支持，不从 Linux 外推。

## Implementation Notes

调查未发现未决用户意图、迁移或删除需求。直接来源跨数个文件且需要完整独立 review，选择 dispatch。冻结意图精确复用客户/PM 裁决：delegated approval by dev-user/dev-pm under maintainer authorization 2026-10-10。该缺陷非正式 Epic story，不写 sprint 状态；Build done 仅实施阶段，独立 QA、exact-head CI 和受保护整合仍属后续出口。

## Spec Change Log

## Review Triage Log

- Blind Hunter B1：medium → patch。查 `.github/actions/native-analysis-compiler/action.yml`，锁定 Windows 工具来自 MINGW64，不是 UCRT64。已改示例及断言为 mingw64；不改变工具锁。
- Blind Hunter B2：false（关于本次宣称 AC 完成）。失败／重试独立证据确实尚未执行，本规格和交接一直明确交同卡 Tester，不宣称该条件通过；Build 完成不替代整项验收。
- Blind Hunter B3：false（存在隐式 checkout/uv/Rust/Node 依赖的推断）。追踪包内 CLI、自带模块与 build.py，Python 使用 -I -S，GCC/objdump/Git 从三绝对路径启动；继承 PATH 支持 GCC 子工具，不会调用 checkout/uv/Rust/Node。没有可达的隐藏开发入口，独立 Tester 仍须按 README 补有限接收环境证据。
- Blind Hunter B4：medium → patch。原 payload 比较只覆盖 manifest+seal，不能发现额外文件／目录。新增测试专用完整递归目录/类型/字节 inventory，并在三个独立复制构建中比较，拒绝 link/特殊项；不改变正式 owner 实现。
- Blind Hunter B5：low，拒绝。正常生成测试自身不改全局环境是正确的并行测试行为；本轮完整默认 Cargo 确实在删除工具变量条件下执行，生产源码准备调用链不触发工具。增加恶意可执行探针或另一个子进程入口超出直接修正文案，不用此测试单独宣称探针级证明。
- Blind Hunter B6：low，拒绝。新测试沿用正式 CLI 的各阶段有界 ProcessOwner，未发现 bootstrap 阻塞路径；当前共享外层 helper 不支持 env_remove，修其环境契约会扩大无关范围。保留 CI job 上界，不把假设死锁当实测缺陷。
- Blind Hunter B7：low → patch。占位命令原本要求替换路径，但 Python alias 与 shell 占位语法解释不够明确；补 Python3/Linux、py -3/Windows 的解释及替换、引用空格路径提示，测试保护。不是要求默认系统拥有名为 python 的命令。
- Edge Case Hunter：返回 []，无可触发边界发现。
- Verification Gap Reviewer：No verification gaps found；读取测试并执行相关 Cargo 检查。该结论仅针对变更验证保护，不覆盖尚未发生的独立 Tester 验收或 Windows CI。

## Verification

- `cargo test --locked --manifest-path core/Cargo.toml --test multi_component_contracts`：相关生成与所有权回归实际运行。
- 按实际测试注册补 single/legacy 正常生成测试；`--features native-tests` 的独立 sealed build 须实际执行非零测试数量。
- `uv run --locked python -m autosar_tooling assets check`，必要时显式 `assets update` 后检查。
- `uv run --locked python -m autosar_tooling quality --base 1ed2f1d673cb767e96c5e82a019a2ae548b48059` 与 `git diff --check`。
- 临时复制包和构建输出放 TMPDIR，不新增持久报告/脚本；保留可定位候选与测试结果到本规格。

### 实施子代理验证记录

- 候选：保留原分支 `autsaro/t_3123e6cf-ecu`，HEAD 为 `1ed2f1d673cb767e96c5e82a019a2ae548b48059`，未提交／推送。`git diff --binary -- core runtime | sha256sum` 的实现差异身份为 `0f4862906f6e0bd55237a63d68b16c95e799d373af059bb2a73cf693868a1656`（不含本规格）。
- 三个 README producer 复用 `generator::delivery::native_build_prerequisites`，legacy 通过原插值入口装入；仅改说明及正常测试，不改编译策略、C、ABI、schema 或历史包。公共所有权说明保留 live/sealed 和旧包边界，去掉把所有工具统称固定、把 multi 暗示为通用 verify 的措辞。
- 双 target 正常生成回归：`multi_handoff_readme_supports_both_native_targets_without_build_tools` 与 `epic7_delivery::builtin_host_and_ecu_deliver_project_license_and_preserve_kernel_notice` 覆盖 multi、single 及 legacy host README 来源，检查三变量、平台示例、GCC 版本／triple、objdump 可选摘要、Git 无锁、安全重试与原所有权说明。完整默认 Cargo 测试在移除四项 AUTOSAR 工具环境变量后运行通过；实际执行 lib 34、builtin 72、c_analysis 2、multi_component_contracts 70 项。0-test 容器不计覆盖。
- Linux 真实独立复制构建：`cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test multi_component_contracts copied_multi_package_builds_offline_without_changing_source_bytes -- --exact` 与 `cargo test --locked --manifest-path core/Cargo.toml --features native-tests --test builtin epic7_delivery::copied_host_and_single_packages_build_offline_without_changing_source_bytes -- --exact` 各实际执行 1 项并通过，合计构建 multi/single `ecu_host_batch` 和 host `ecu_host` 三个二进制。测试在 TMPDIR 生成、独立复制后删除原包及 live 输入；从接收包目录通过自带 Python CLI（`-I -S`、移除 PYTHONPATH/PYTHONHOME）仅声明 GCC/objdump/Git 路径，全部封存 payload 与两个 seal 文件构建前后逐字节相同。未将构建成功解释为行为或规范符合。
- 工具：Python 3.12.9；`AUTOSAR_CC=/usr/bin/gcc-13`、`AUTOSAR_OBJDUMP=/usr/bin/objdump`、`AUTOSAR_GIT=/usr/bin/git`。实际 GCC 13.3.0 与 GCC／objdump 摘要均匹配 Linux 锁。包执行不调用 uv/Rust/Node；Cargo/uv 仅属于开发测试入口。
- `assets check` 先发现唯一 README 摘要不符，经审阅及核对 `.gitattributes` 的 LF 要求后显式 `assets update`；最终 `assets check` 为 0 changes，仅 `runtime/generated-README.md` 摘要变化，无 ABI／第三方身份变化。
- 最终 `uv run --locked python -m autosar_tooling quality --base 1ed2f1d673cb767e96c5e82a019a2ae548b48059` 为 Requested source checks passed；`git diff --check` 通过。格式化仅作用于新增测试范围；未改 C，不声明 c-check/MISRA 结果。
- 已修正实施中发现的 Rust 模块引用错误及新测试格式问题；当前无已知实施阻塞。隔离内部 review 仍由父 Engineer 执行，第四项任务保持未完成；独立 Tester 的缺工具／错误身份／失败现场／拒绝原地重试证据、Windows 原生 exact-head CI 和下游整合均未在本子代理运行，不冒充通过。规格保持 in-progress，未修改冻结批准内容。

### 父 Engineer review 后复验

- 三个同能力隔离 reviewer 同批运行，未跳过层。修复 B1/B4/B7；其余逐条裁决如上，无延后生产代码缺陷。实施子代理无法续接，本人应用必要小 patch。
- `cargo test --locked --manifest-path core/Cargo.toml --test multi_component_contracts` 70项和 `--test builtin` 72项重新全测通过；日志位于本轮 TMPDIR 的 handoff-default-multi.log/handoff-default-builtin.log。
- 两项新独立复制 native-tests 重新各执行1项通过，实际构建 multi/single/host，新增完整递归 source tree 前后保持相同。命令与前述实施记录一致。
- 复验 `assets check` 为0 changes，`quality --base 1ed2f1d673cb767e96c5e82a019a2ae548b48059` 检查7个格式文件成功，C syntax not requested（无C变更）；git diff --check通过。
- Build 内部实现与review完成。独立 Tester 的缺工具／错误身份／失败重试检查、Windows锁定工具链 exact-head CI、PR正式 review 和受保护merge 尚待后续，不能由本规格状态推定成功。
