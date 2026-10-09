# 测试

测试按实际前置条件选择，不需要恢复代理会话。

## 基础检查

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked --manifest-path core/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
```

前端使用 Vitest；`npm run test:watch --prefix ui` 用于开发。Python 普通测试位于 `tests/python`，进程和平台测试独立选择。默认 Rust 测试不需要外部 Python 或官方档案。

Tauri 后端单测覆盖 Session、设置与请求归属，需先准备[桌面平台依赖](environment.md)并运行 `npm run build --prefix ui`。本地 `verify --scope desktop`／`all` 及 PR 的 desktop job 均执行锁定 Cargo 单测；本地聚合入口继续执行 build／Clippy，PR job 保留 build。这些检查不启动原生窗口，不能替代 GUI／IPC 行为验收。

## 生成 C 的静态检查

安装 Cppcheck **2.21.0** 和同一版本的 addons。检查入口校验三个 MISRA addon 文件的 SHA-256；CI 从固定提交 `e73bf44c3e49686b7495fab352d03a6c6075516b` 构建开放版。使用 `CPPCHECK`、`CPPCHECK_ADDON_DIR` 指定程序与 addon 目录，通过 `AUTOSAR_CC` 指定目标 GCC，不需要商业许可证。

```powershell
$env:AUTOSAR_C_ANALYSIS_SAMPLES = "$PWD/build/c-analysis-samples"
cargo test --locked --manifest-path core/Cargo.toml --test c_analysis
uv run --locked python -B -m unittest discover -s tests/python/native -p test_c_analysis.py
uv run --locked python -m autosar_tooling c-check --target windows-x64-controlled-v1 --project build/c-analysis-samples/signals --output-directory build/c-analysis-results/signals
```

Linux 使用 `linux-x64-controlled-v1` 和对应主机 GCC。WSL 的结果目录应放在 Linux 文件系统中；未启用 Unix 元数据的 Windows 挂载目录无法满足受管进程日志的私有目录权限要求。生成目录必须不存在，结果目录必须新建或为空，并与封存工程分离。分析在副本上应用原有内核补丁，检查构建清单、主机入口和补充 C 文件，原始工程不会被修改。

Cppcheck 使用目标平台模型及项目头文件。按[官方建议](https://cppcheck.sourceforge.io/manual.html)，不直接解析编译器的标准库头文件；GCC 按可用 CPU 数以最多四个并发任务逐个验证真实头文件与 C99 语法，并提取代码引用的系统宏和标准整数常量宏的实际后缀。Windows 的 `setvbuf` 参数模型使用该编译器头文件中的三个实际模式值，非法模式仍会报告。Linux 的递归互斥锁枚举及 glibc 动态信号／栈值由有限分析模型补齐；GCC 会校验枚举值和函数原型，动态值保留实际函数调用，模型不匹配即失败。其他 Windows／POSIX 接口仍取决于分析器的库模型，未知符号或解析失败保持失败。

ECU 的 HostBatch 与 legacy probe 是两个独立可执行程序，跨翻译单元检查分别运行；共同源码在两个链接范围中均检查。`summary.json` 保留各程序清单、命令、退出码和诊断所属程序，原始工件分别位于 `host-batch/` 与 `legacy-probe/`。主机 profile 使用一个分析目录。

结果包含 `summary.json`、每个程序的原始 `diagnostics.xml`、进程日志、`compile_commands.json`、工具身份、逐项 checker 可用性和扫描范围。固定内核的诊断标记为 `adopted`，保留原始诊断。项目规则检查直接动态内存调用，以及普通 BSW 中的直接主机线程创建；宏展开与 typedef 可以检测，间接函数指针等路径尚未覆盖。项目规则不会冒充完整 MISRA 指令。

主机锁和文件错误边界测试使用 `uv run --locked python -B -m unittest discover -s tests/python/native -p test_host_boundaries.py`，通过测试构建替换平台调用，验证锁忙／锁错误、必要终止点及文件关闭失败传播，不向交付源码增加故障 hook。OS 资源获取和栈保证故障注入仅编译到显式测试构建（`OS_HOST_FAILURE_TESTS`）；生产 HostBatch／probe 不读取 `AUTOSAR_OS_FAIL_RESOURCE` 或 `AUTOSAR_OS_BAD_GUARANTEE`。

生成上述样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_ecu_build_modes.py` 通过封存工程的离线 CLI 实际构建 HostBatch、probe 和 test 三种模式，分别运行正常、非法资源参数、首次资源失败及栈保证失败场景。构建输出位于临时 Git 仓库内，覆盖内核选择补丁不能受外层仓库路径影响的回归。正式构建应完成正常协议，test 模式应拒绝故障；需要目标 GCC、objdump、git，可用 `AUTOSAR_CC`／`AUTOSAR_OBJDUMP`／`AUTOSAR_GIT` 指定，未指定时使用 PATH。

固定 addon 的 R21.8 仍报告 `getenv`，与修订基线不一致；返回摘要的 `knownCheckerLimitations` 标明此缺口，原始诊断保留。它也没有在该 checker 的终止函数清单中列出 `_Exit`／`quick_exit`，不能据此作完整规则覆盖声明。

真实诊断、工具缺失、版本错误、超时、编译／解析失败和扫描不完整均返回非零。无诊断只说明这些自动检查通过；221 项规则／指令中的人工评估仍需根据需求、设计、授权规范和偏离审批完成，不会声明“完整符合 MISRA”。

CI 每个平台先验证检查器、生成六类完整样本并发布本轮工具与样本工件，再把三个主机样本、标准 ECU、用户应用 ECU 和多组件 ECU 分成四个并行任务；汇总任务要求所有准备和分析任务成功，跳过或取消也不会通过。固定版本 Cppcheck、vcpkg 二进制包与 Cargo 构建使用缓存，每次仍重新生成样本并分析全部源码，缓存不包含扫描结果。PR 的新提交会取消同一 PR 的旧任务。双平台任务自动验证完整分析链路，拒绝配置／解析失败或漏扫；源码违规仍保留非零退出码及 `passed=false`，不会由链路验证改写为通过。严格源码门禁暂限手动 workflow_dispatch。现存诊断整改与必要偏离获批后，才能移除该条件，启用每次 PR 的严格源码门禁。手动扫描收集所有样本结果后返回真实失败状态，日志与样本保留 14 天。

## 进程、资源和原生行为

```powershell
uv run --locked python -B -m unittest discover -s tests/python/process
cargo test --locked --manifest-path core/Cargo.toml --lib --features native-tests execution::
cargo test --locked --manifest-path core/Cargo.toml --features official-oracles
cargo test --locked --manifest-path core/Cargo.toml --features native-tests
```

进程测试需 `AUTOSAR_PYTHON` 指向支持的绝对解释器路径。官方对照需正式资源清单声明的合法档案；完整 native 测试需对应平台的受控 GCC/binutils，26 个 OS suites 通过 Cargo 注册运行，不重复执行。Windows 的历史交接对照会编译收到的包，因此 `official-oracles` 也需对应受控工具链。

需要保留 OS 的编译产物和进程日志时，设置 `AUTOSAR_OS_OUTPUT_DIRECTORY` 为新的结果目录，Cargo 测试按 suite 保存到该目录。单独调用 `autosar_tooling os` 可传 `--output-directory`。已有目录不会被覆盖；未指定时仍只保留失败现场。

BMad 的兼容入口 `uv run --locked python -m autosar_tooling verify --scope all --base <基准提交>` 组合原有完整检查，不安装依赖，不编排另一套任务或写汇总报告。失败直接保留原输出和退出状态。

## 隔离桌面与安装包

```powershell
uv run --locked python -m autosar_tooling desktop --platform windows --binary <应用绝对路径> --output-directory <新的空结果目录>
```

桌面驱动位于 `tests/desktop/`，平台控制和输入准备位于 `autosar_tooling.acceptance`。首次执行历史兼容场景前，用 `cargo build --locked --manifest-path core/Cargo.toml --bins` 构建测试所用的独立生成器。

Linux 和 macOS 使用对应 platform。`--installed --source-checkout <原构建目录>` 验证解包产物及原 checkout 不可用的接收场景。`--builtin-only` 用于安装包内置配置检查；计数器、受管错误和延迟测试使用 `native-webdriver` 验证构建，普通发行构建不会启用这些能力。Linux 驱动需要 Xvfb、WebKitWebDriver、tauri-driver、xdotool、xclip、xsettingsd 与 dump_xsettings；安装后离线验收还使用普通用户的私有 user/network namespace。平台隔离前置条件不满足时明确失败；不得在用户正在使用的桌面补测。

离线 ECU 包可由 Python 3.11 或更新版本运行自带 `tools/ecu-tool.py`，不需要 uv、Rust、Node 或源 checkout。构建和验证仍检查封存内容与目标工具链。支持版本的 Windows/Linux 实际构建由受控验收验证，未运行的平台或版本不宣称通过。
