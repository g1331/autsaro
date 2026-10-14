# 测试

测试按实际前置条件选择，不需要恢复代理会话。

## 基础检查

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked -p autosar-config-core
cargo test --locked -p autosar-config-desktop
cargo clippy --locked -p autosar-config-core --all-targets --all-features -- -D warnings
cargo clippy --locked -p autosar-config-desktop --all-targets -- -D warnings
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
```

仓库根目录是 Cargo workspace（成员 `core` 与 `src-tauri`），共用根目录的 `Cargo.lock` 与 `target/`；`cargo test --locked --workspace` 一次运行两个 crate 的默认测试，需先准备桌面平台依赖。前端使用 Vitest；`npm run test:watch --prefix ui` 用于开发。Python 普通测试位于 `tests/python`，进程和平台测试独立选择。默认 Rust 测试不需要外部 Python 或官方档案。

core 的集成测试分为三个二进制，按能力模块组织，共用 `core/tests/support/`：

| 二进制 | 运行方式 | 前置条件 |
| --- | --- | --- |
| `it` | `cargo test -p autosar-config-core --test it` | 无 |
| `native` | `cargo test -p autosar-config-core --features native-tests --test native` | 目标 GCC／binutils、Git 与 `AUTOSAR_PYTHON` |
| `official` | `cargo test -p autosar-config-core --features official-oracles --test official` | 官方档案（见[官方对照](#官方对照)）与 `AUTOSAR_PYTHON` |

测试名带模块路径，可按名称筛选，例如 `cargo test -p autosar-config-core --test it -- integration::diagnostics::`、`cargo test -p autosar-config-core --features native-tests --test native -- integration::os::`。协议与 OS 独立对照向量（`core/tests/fixtures/oracles/`）由 `it` 的 `protocol_oracles::` 测试绑定到参考 ECUC 配置，设置或固定字面值漂移时失败。

Tauri 后端单测覆盖 Session、设置与请求归属，需先准备[桌面平台依赖](environment.md)并运行 `npm run build --prefix ui`。本地 `verify --scope desktop`／`all` 及 PR 的 desktop job 均执行锁定 Cargo 单测、build 与 Clippy；PR 只在 Linux 运行 Clippy。这些检查不启动原生窗口，不能替代 GUI／IPC 行为验收。

Clippy 以默认规则集加 `-D warnings` 检查两个 crate，本地 `verify` 与 PR 使用相同参数。生成与集成模块（`core/src/generator.rs`、`core/src/integration/mod.rs`、`core/src/prepared.rs` 及其子模块）在模块属性中另外拒绝 `unwrap`、`expect`、`panic!` 以及 `core/clippy.toml` 列出的文本修补与文件嵌入：计划无法保证的前提返回 `PlanDiagnostic`，每个输出只渲染一次，受信模板经 `outputs::render` 编辑。确有理由的例外使用带 `reason` 的局部 `#[allow]`。启用门禁时其余模块遗留的告警以 `#[expect(.., reason = "clippy baseline #37: ...")]` 标记，`git grep -n "clippy baseline"` 列出剩余基线；修好告警后该 `#[expect]` 本身会报 `unfulfilled_lint_expectations`，须一并删除标记。CI 只在 Linux 运行 Clippy，Windows 专属的 `cfg` 代码不在门禁内。

## 生成 C 的静态检查

安装 Cppcheck **2.21.0** 和同一版本的 addons。检查入口校验程序版本及三个 MISRA addon 文件的 SHA-256；CI 从官方 `2.21.0` 标签（提交 `e73bf44c3e49686b7495fab352d03a6c6075516b`）构建开放版，不需要商业许可证。本地按同样的官方 CMake 方式构建即可，构建出的程序旁带有 `cfg` 与 `addons`，只需让 `CPPCHECK` 指向它：

```powershell
git clone -c core.autocrlf=false -c core.eol=lf --depth 1 --branch 2.21.0 https://github.com/cppcheck-opensource/cppcheck.git build/cppcheck-source
cmake -S build/cppcheck-source -B build/cppcheck-build -DCMAKE_BUILD_TYPE=Release -DUSE_MATCHCOMPILER=OFF -DBUILD_GUI=OFF -DBUILD_TESTING=OFF
cmake --build build/cppcheck-build --config Release --target cppcheck --parallel
$env:CPPCHECK = "$PWD/build/cppcheck-build/bin/cppcheck"
```

克隆时保留 LF：该仓库按 `text=auto` 检出，Windows 默认会把 addon 转成 CRLF，摘要校验随之失败。Visual Studio 等多配置生成器把程序放在 `bin/Release/`。未安装 CMake 时可用 `uvx --from cmake cmake` 运行 Kitware 发布的 CMake。`CPPCHECK_ADDON_DIR` 可单独指定 addon 目录；`AUTOSAR_CC` 指定目标 GCC。

```powershell
$env:AUTOSAR_C_ANALYSIS_SAMPLES = "$PWD/build/c-analysis-samples"
cargo test --locked -p autosar-config-core --test it -- c_analysis::
uv run --locked python -B -m unittest discover -s tests/python/native -p test_c_analysis.py
uv run --locked python -m autosar_tooling c-check --target windows-x64-controlled-v1 --project build/c-analysis-samples/standard-ecu --output-directory build/c-analysis-results/standard-ecu
```

Linux 使用 `linux-x64-controlled-v1` 和对应主机 GCC。WSL 的结果目录应放在 Linux 文件系统中；未启用 Unix 元数据的 Windows 挂载目录无法满足受管进程日志的私有目录权限要求。生成目录必须不存在，结果目录必须新建或为空，并与封存工程分离。分析在副本上应用原有内核补丁，检查构建清单、主机入口和补充 C 文件，原始工程不会被修改。

Cppcheck 使用目标平台模型及项目头文件。按[官方建议](https://cppcheck.sourceforge.io/manual.html)，不直接解析编译器的标准库头文件；GCC 按可用 CPU 数以最多四个并发任务逐个验证真实头文件与 C99 语法，并提取代码引用的系统宏和标准整数常量宏的实际后缀。Windows 的 `setvbuf` 参数模型使用该编译器头文件中的三个实际模式值，非法模式仍会报告。Linux 的递归互斥锁枚举及 glibc 动态信号／栈值由有限分析模型补齐；GCC 会校验枚举值和函数原型，动态值保留实际函数调用，模型不匹配即失败。其他 Windows／POSIX 接口仍取决于分析器的库模型，未知符号或解析失败保持失败。

ECU 的 HostBatch 与 probe 是两个独立可执行程序，跨翻译单元检查分别运行，两个程序的 Cppcheck 进程并发执行、互不共享构建目录；共同源码在两个链接范围中均检查。`summary.json` 按程序声明顺序保留各程序清单、命令、退出码和诊断所属程序，原始工件分别位于 `host-batch/` 与 `probe/`。

结果包含 `summary.json`、每个程序的原始 `diagnostics.xml`、进程日志、`compile_commands.json`、工具身份、逐项 checker 可用性和扫描范围。固定内核的诊断标记为 `adopted`，保留原始诊断。项目规则检查直接动态内存调用，以及普通 BSW 中的直接主机线程创建；宏展开与 typedef 可以检测，间接函数指针等路径尚未覆盖。项目规则不会冒充完整 MISRA 指令。

主机锁错误边界测试使用 `uv run --locked python -B -m unittest discover -s tests/python/native -p test_host_boundaries.py`，通过测试构建替换平台调用，验证锁忙／锁错误及必要终止点，不向交付源码增加故障 hook。OS 资源获取和栈保证故障注入仅编译到显式测试构建（`OS_HOST_FAILURE_TESTS`）；生产 HostBatch／probe 不读取 `AUTOSAR_OS_FAIL_RESOURCE` 或 `AUTOSAR_OS_BAD_GUARANTEE`。

生成上述样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_ecu_build_modes.py` 通过封存工程的离线 CLI 实际构建 HostBatch、probe 和 test 三种模式，分别运行正常、非法资源参数、首次资源失败及栈保证失败场景。构建输出位于临时 Git 仓库内，覆盖内核选择补丁不能受外层仓库路径影响的回归。正式构建应完成正常协议，test 模式应拒绝故障；需要目标 GCC、objdump、git，可用 `AUTOSAR_CC`／`AUTOSAR_OBJDUMP`／`AUTOSAR_GIT` 指定，未指定时使用 PATH。

`uv run --locked python -B -m unittest discover -s tests/python/native -p test_cmake_build.py` 用交付工程生成的 `CMakeLists.txt` 以 Ninja 构建 `user-application` 样本，核对每个编译单元、编译选项和 include 顺序与 `target.json` 一致，检查两个程序的受检段，并与 `ecu-tool` 构建的程序比较运行输出；构建后封存清单不变。CMake 与 Ninja 来自 uv 的 `build` 依赖组，也可用 `CMAKE`／`NINJA` 指定。

固定 addon 的 R21.8 仍报告 `getenv`，与修订基线不一致；返回摘要的 `knownCheckerLimitations` 标明此缺口，原始诊断保留。它也没有在该 checker 的终止函数清单中列出 `_Exit`／`quick_exit`，不能据此作完整规则覆盖声明。

真实诊断、工具缺失、版本错误、超时、编译／解析失败和扫描不完整均返回非零。无诊断只说明这些自动检查通过；221 项规则／指令中的人工评估仍需根据需求、设计、授权规范和偏离审批完成，不会声明“完整符合 MISRA”。

CI 每个平台先验证检查器、生成四类完整样本并发布本轮工具与样本工件，再把标准 ECU（`standard-ecu`）、用户应用 ECU（`user-application`）、多组件流水线 ECU（`component-pipeline`）及不含应用 DID 与诊断路由的 ECU（`no-diagnostic`）分成四个并行任务；汇总任务要求所有准备和分析任务成功，跳过或取消也不会通过（仅改文档的 PR 整体跳过，见下）。固定版本 Cppcheck、vcpkg 二进制包与 Cargo 构建使用缓存，每次仍重新生成样本并分析全部源码，缓存不包含扫描结果。PR 的新提交会取消同一 PR 的旧任务。双平台任务自动验证完整分析链路，拒绝配置／解析失败或漏扫；源码违规仍保留非零退出码及 `passed=false`，不会由链路验证改写为通过。严格源码门禁暂限手动 workflow_dispatch。现存诊断整改与必要偏离获批后，才能移除该条件，启用每次 PR 的严格源码门禁。手动扫描收集所有样本结果后返回真实失败状态，日志与样本保留 14 天。

`Development checks` 中的 `CI required` 是分支规则唯一要求的检查，阻塞合并的任务清单写在该任务的 `needs` 中，增删检查只改工作流。PR 只改根目录指南、`docs/`、代理说明或 PR 模板时，构建与测试任务整体跳过，`quality` 照常运行；其余 PR、推送到主干及手动运行都执行完整检查。分支无须先与主干同步即可合并，合并后主干推送会再完整运行一次。

`native` 任务在 Windows 与 Linux 上以 `--features native-tests --lib --test native --no-fail-fast` 运行全部原生 Rust 套件（Windows 按测试名把最慢的几个模块分到第二个 runner，同一名单在另一半用 `--skip` 排除，每个测试只运行一次），工具路径取自已安装的受控 GCC、binutils、Git 及项目 uv 环境；`native-os` 任务在两个平台运行 26 个 OS suites 和主机锁错误边界测试，Windows 用 `AUTOSAR_OS_SHARD=k/n` 把 suite 按序号分到两个 runner，每个 suite 仍单独串行运行。`native-delivery` 任务下载准备任务发布的样本，与样本分析并行运行封存工程构建模式和 CMake 构建，再把 `standard-ecu` 样本移到新目录，用 uv 管理的 Python 3.11–3.14 接收者解释器（`AUTOSAR_RECEIVER_PYTHON`）逐个直接运行包内工具完成构建与验证，不向接收者解释器安装开发工具；这只证明解释器兼容性，不代表网络隔离下的安装验收。`official` 二进制由不属于必需检查的 `acceptance.yml` 运行（见[官方对照](#官方对照)），桌面场景由不属于必需检查的 `desktop.yml` 运行（见[桌面场景](#桌面场景)）；PR 检查通过不代表这些检查通过。

## 进程、资源和原生行为

```powershell
uv run --locked python -B -m unittest discover -s tests/python/process
cargo test --locked -p autosar-config-core --lib --features native-tests execution::
cargo test --locked -p autosar-config-core --features native-tests --lib --test native --no-fail-fast
uv run --locked python -B -m unittest discover -s tests/python/native -p test_os_suites.py
cargo test --locked -p autosar-config-core --features official-oracles --test official --no-fail-fast
```

进程测试需 `AUTOSAR_PYTHON` 指向支持的绝对解释器路径；受管进程的探针是测试辅助脚本 `tests/python/process/probe.py`。官方对照的档案准备见下节；native 测试需对应平台的受控 GCC/binutils。OS suites 由 unittest 按 suite 生成测试，可用 `-k stack` 等筛选，只在与当前宿主一致的目标上运行；测试名对应 suite，进程失败信息包含阶段、命令和原始退出状态。

需要保留 OS 的编译产物和进程日志时，设置 `AUTOSAR_OS_OUTPUT_DIRECTORY` 为新的结果目录，unittest 按 suite 保存到该目录。已有目录不会被覆盖；未指定时仍只保留失败现场。

### 官方对照

`official` 二进制把内置规则、模块定义和生成的 OS 描述与 R24-11 官方 XSD、ECUC MOD 及建模示例档案比较。档案受 AUTOSAR 条款限制，不入库、不随产品分发；这三份档案与条款核算用的 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 的地址、默认位置、环境变量和 SHA-256 登记在 `core/resources/official.json`。本机用下面的命令下载到登记位置（已设置 `AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE`／`AUTOSAR_SAMPLE_ARCHIVE`／`AUTOSAR_SPECIFICATIONS_ARCHIVE` 时下载到它们指向的位置）：

```powershell
uv run --locked python -m autosar_tooling fetch-official
```

已存在的文件只校验、不覆盖，哈希不一致即失败；新下载的文件哈希不一致时丢弃。`www.autosar.org` 在 TLS 握手中不发送中间证书，命令从证书 AIA 地址取得 Starfield Secure Certificate Authority - G2，校验固定指纹后与系统信任库一起使用，不关闭证书校验。站点更换证书后指纹校验会失败，需审阅新证书链后再更新 `autosar_tooling/official.py` 中的固定值。

CI 中的 `Official archive comparison`（`acceptance.yml`）在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` runner 上运行 `fetch-official`、`cargo test --locked -p autosar-config-core --features official-oracles --test official --no-fail-fast` 和条款处置核算（见[标准基线 §5](../standards/README.md#5-条款处置与核算)）：`core/resources/requirements/` 中已提交清单的文档从档案做完整检查，并核对清单与档案一致；其余被引用的文档只检查失效引用，列表在工作流的 `Requirement dispositions` 步骤中。档案按 `official.json` 的哈希存入本仓库的 Actions 缓存，不作为 artifact 上传，日志只记录路径和哈希。触发方式：

- 手动运行（workflow_dispatch）；
- 每天对 master 定时运行一次，该提交已有成功运行时跳过；
- 修改 `core/src/definitions*`、`core/src/schema.rs`、`core/src/rules*`、`core/resources/official.json`、`core/resources/requirements/`、`core/tests/official/`、`docs/standards/`、下载或核算入口或该工作流的 PR。未登记文档的测试编号引用变化由每日运行发现，模块整改 PR 应在本地运行核算。

它不属于 `CI required`：外部站点不可用不阻塞合并，PR 上的失败需要自行查看。已整改模块的处置由必需检查保护：`quality` 任务运行 `uv run --locked python -m autosar_tooling requirements`，按提交的清单完整核算，不需要档案，只改 `docs/standards/` 的 PR 也会运行；`verify --scope all` 包含同一检查。清单过期由本工作流发现。下载、证书或哈希失败时，`Download and verify official archives` 步骤失败并标注 `Official archives unavailable`，测试不会运行；测试失败标注 `Official comparison failed`，核算失败标注 `Requirement dispositions failed`。

完整检查入口 `uv run --locked python -m autosar_tooling verify --scope all --base <基准提交>` 组合原有完整检查，不安装依赖，不编排另一套任务或写汇总报告。失败直接保留原输出和退出状态。

## 桌面场景

`tests/desktop/*.test.mjs` 通过 Tauri 官方 `tauri-driver` 与平台 WebDriver 驱动真实应用和 IPC，由 Node 内置的 `node:test` 运行。每个文件启动独立的应用会话，使用私有的配置、主目录和临时目录，文件之间没有顺序依赖；场景只使用内置规则，共用辅助位于 `tests/desktop/support/`。

先构建验证版应用（前端打包进程序，不需要开发服务器）并安装 `tauri-driver`：

```powershell
npm run tauri --prefix ui -- build --debug --no-bundle --features native-webdriver
cargo install tauri-driver --version 2.1.0 --locked
node --test --test-concurrency=1 "tests/desktop/*.test.mjs"
```

运行前把 `AUTOSAR_DESKTOP_APP` 设为构建出的 `target/debug/autosar-config-desktop`（Windows 带 `.exe`）的绝对路径。Node 24 的 `--test` 不接受目录，glob 加引号后由 Node 展开；也可只传单个文件，或用 `--test-name-pattern` 选择检查。`native-webdriver` 只用于验证构建：提供计数器、受管失败和延迟注入，并在 Windows 上通过 WebView2 API 转交 msedgedriver 的启动参数（WebView2 在提权进程中忽略 `WEBVIEW2_*` 环境变量，GitHub 托管 runner 以管理员身份运行）。发行构建不包含这些能力。

`AUTOSAR_DESKTOP_RESULTS` 指定结果目录，默认使用系统临时目录。每个文件在其中新建私有目录，保存各项检查的截图、`ipc.jsonl` 和会话关闭后的 `process-tree.json`；失败时另存 `failure.png` 与页面文本 `failure.txt`。`owned-execution.test.mjs` 还需要用 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 声明受控 GCC、binutils、Git 与受支持 Python 的绝对路径；应用环境仍剥除这些变量，由场景通过真实的“执行工具”设置填写，未声明时该文件失败。

- Linux：需要 `WebKitWebDriver`（Debian／Ubuntu 软件包 `webkit2gtk-driver`）、`Xvfb`、`dbus-daemon`、`xsettingsd`、`xdotool` 与 `xclip`。每个会话使用私有 Xvfb display 和不激活服务的私有 D-Bus，原生文件选择框也在该 display 中操作，不触及正在使用的桌面，可在开发机直接运行。
- Windows：需要与 Edge／WebView2 版本一致的 `msedgedriver` 位于 PATH 中。会话显示真实窗口、读取会话剪贴板并操作原生文件选择框，因此只在 GitHub Actions、Windows Sandbox 或设置了 `AUTOSAR_DESKTOP_DEDICATED_SESSION=1` 的专用会话中启动，否则直接拒绝；不得在日常桌面运行。各会话共用桌面与剪贴板，必须串行运行。
- macOS：`tauri-driver` 不支持 WKWebView，不运行桌面场景。

关闭会话后，测试断言应用进程树已全部退出，残留时列出 pid、名称和命令行；按 Ctrl-C 或取消 CI 也会结束私有 display 与 driver 进程组。

`desktop.yml` 在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` 上构建验证版并运行全部场景；改动应用、界面或场景的 PR 及手动触发都会运行，结果目录作为工件保留 14 天。它目前不在必需检查中。这些场景验证开发构建的界面与 IPC，不提供安装包、无源码 checkout 或断网环境的验收证据。

## 离线 ECU 包

离线 ECU 包可由 Python 3.11 或更新版本运行自带 `tools/ecu-tool.py`，不需要 uv、Rust、Node 或源 checkout。构建和验证仍检查封存内容与目标工具链。支持版本的 Windows/Linux 实际构建由受控验收验证，未运行的平台或版本不宣称通过。
