# 生成 C 的静态检查

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

生成的 C 与运行时源码使用同一 `.clang-format`：emitter 直接写出该排版（规则在 `core/src/integration/c_layout.rs`），生成时不调用格式化工具（ADR 0014）。生成样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_generated_format.py` 用 uv 固定的 clang-format 检查样本中除 `kernel/`（FreeRTOS）和 `app/`（用户源码）外的 C 与头文件，必需 CI 在两个平台的 `c-analysis-prepare` 中运行。过长函数头的断行由 clang-format 按惩罚值择优，emitter 只复现常见选择，超长的用户名称可能排版不同。

clang 作为第二编译器检查同一批样本：`uv run --locked python -B -m unittest discover -s tests/python/native -p test_clang_build.py` 把样本内核按 `target.json` 打上补丁，再用样本自身的编译选项（含 `-Werror`）分别以正式与 test 模式（`-DECU_TARGET_TESTS -DOS_HOST_FAILURE_TESTS`）编译除 `kernel/`、`app/` 外的全部编译单元，另开启 `-Wshadow`、`-Wmissing-prototypes`、`-Wstrict-prototypes`、`-Wcast-qual`、`-Wextra-semi`、`-Wformat=2`。与 MISRA 重复的诊断不加入（如 `-Wunused-macros`、`-Wvla`、`-Wsign-conversion`、`-Wcomma`、`-Wswitch-enum`）。clang 取自 `AUTOSAR_CLANG` 或 PATH，必需 CI 在 Linux 的 `c-analysis-prepare` 中运行；它只编译不链接，不替代受锁定的 GCC 构建。

生成上述样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_ecu_build_modes.py` 通过封存工程的离线 CLI 实际构建 HostBatch、probe 和 test 三种模式，分别运行正常、非法资源参数、首次资源失败及栈保证失败场景。构建输出位于临时 Git 仓库内，覆盖内核选择补丁不能受外层仓库路径影响的回归。正式构建应完成正常协议，test 模式应拒绝故障；需要目标 GCC、objdump、git，可用 `AUTOSAR_CC`／`AUTOSAR_OBJDUMP`／`AUTOSAR_GIT` 指定，未指定时使用 PATH。

`uv run --locked python -B -m unittest discover -s tests/python/native -p test_cmake_build.py` 用交付工程生成的 `CMakeLists.txt` 以 Ninja 构建 `user-application` 样本，核对每个编译单元、编译选项和 include 顺序与 `target.json` 一致，检查两个程序的受检段，并与 `ecu-tool` 构建的程序比较运行输出；构建后封存清单不变。CMake 与 Ninja 来自 uv 的 `build` 依赖组，也可用 `CMAKE`／`NINJA` 指定。

固定 addon 的 R21.8 仍报告 `getenv`，与修订基线不一致；返回摘要的 `knownCheckerLimitations` 标明此缺口，原始诊断保留。它也没有在该 checker 的终止函数清单中列出 `_Exit`／`quick_exit`，不能据此作完整规则覆盖声明。

真实诊断、工具缺失、版本错误、超时、编译／解析失败和扫描不完整均返回非零。无诊断只说明这些自动检查通过；221 项规则／指令中的人工评估仍需根据需求、设计、授权规范和偏离审批完成，不会声明“完整符合 MISRA”。

CI 每个平台先验证检查器、生成四类完整样本并发布本轮工具与样本工件，再把标准 ECU（`standard-ecu`）、用户应用 ECU（`user-application`）、多组件流水线 ECU（`component-pipeline`）及不含应用 DID 与诊断路由的 ECU（`no-diagnostic`）分成四个并行任务；汇总任务要求所有准备和分析任务成功，跳过或取消也不会通过（仅改文档的 PR 整体跳过，见 [CI](README.md#ci)）。固定版本 Cppcheck、vcpkg 二进制包与 Cargo 构建使用缓存，每次仍重新生成样本并分析全部源码，缓存不包含扫描结果。PR 的新提交会取消同一 PR 的旧任务。双平台任务自动验证完整分析链路，拒绝配置／解析失败或漏扫；源码违规仍保留非零退出码及 `passed=false`，不会由链路验证改写为通过。严格源码门禁暂限手动 workflow_dispatch。现存诊断整改与必要偏离获批后，才能移除该条件，启用每次 PR 的严格源码门禁。手动扫描收集所有样本结果后返回真实失败状态，日志与样本保留 14 天。
