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

四样本同时选择实际支持的通信类型分支：`standard-ecu`／`user-application` 使用 EcuC 的 `PduIdTypeEnum=UINT16`、`PduLengthTypeEnum=UINT16`；`component-pipeline` 使用 `UINT8`／`UINT32`，保留移动交付包后的导入往返；`no-diagnostic` 使用 `UINT8`／`UINT8`，保留无应用 DID 和诊断路由的配置。`it` 的 `c_analysis::` 固定每个样本的配置预期，`native` 的同名模块用交付 include 路径编译并运行 `ComStack_Types.h` 消费者，按独立数值核对宽度、无符号值域和 `PduInfoType.SduLength`；设置 `AUTOSAR_C_ANALYSIS_SAMPLES` 时复用已生成样本，未设置时在临时目录重新生成。正常 `c-check` 继续扫描这些头经 CanIf／CanTp／PduR／Com／Dcm 等 TU 进入的真实程序；不向交付程序加入测试定义或模板骨架。这覆盖可构建配置的 ID 8／16 位与长度 8／16／32 位分支，不代表所有合法配置组合均已分析。Dcm 配置容量独立于通信长度类型，初始化检查可表示性；CanTp 先解码完整首帧长度再转换。无诊断样本仍编译交付中的 Dcm TU。样本参数改变会改变 plan／分析身份，旧配置的 CI 工件不能作为严格可比较基线。

Cppcheck 使用目标平台模型及项目头文件。按[官方建议](https://cppcheck.sourceforge.io/manual.html)，不直接解析编译器的标准库头文件；GCC 按可用 CPU 数以最多四个并发任务逐个验证真实头文件与 C99 语法，并提取代码引用的系统宏和标准整数常量宏的实际后缀。Windows 的 `setvbuf` 参数模型使用该编译器头文件中的三个实际模式值，非法模式仍会报告。Linux 的 glibc 动态实时信号值由有限分析模型补齐；GCC 在同一 C99 参数下校验函数原型，动态值保留实际函数调用，模型不匹配即失败。其他 Windows／POSIX 接口仍取决于分析器的库模型，未知符号或解析失败保持失败。

ECU 的 HostBatch 与 probe 是两个独立可执行程序，跨翻译单元检查分别运行，两个程序的 Cppcheck 进程并发执行、互不共享构建目录；共同源码在两个链接范围中均检查。`summary.json` 按程序声明顺序保留各程序清单、命令、退出码和诊断所属程序，原始工件分别位于 `host-batch/` 与 `probe/`。

结果包含 `summary.json`、每个程序的原始 `diagnostics.xml`、进程日志、`compile_commands.json`、工具身份、逐项 checker 可用性和扫描范围。固定内核的诊断标记为 `adopted`，保留原始诊断；落在生产补丁新增或改动行上的诊断标记为 `patched`，属于本项目代码，不计为采用代码。准备阶段核对每个补丁实际改变了它选中的内核文件。项目规则检查直接动态内存调用，以及普通 BSW 中的直接主机线程创建；宏展开与 typedef 可以检测，间接函数指针等路径尚未覆盖。项目规则不会冒充完整 MISRA 指令。

主机锁错误边界测试使用 `uv run --locked python -B -m unittest discover -s tests/python/native -p test_host_boundaries.py`，通过测试构建替换平台调用，验证锁忙／锁错误及必要终止点，不向交付源码增加故障 hook。同一入口的 `HostIoTests` 链接实际 OS 与平台输出实现，检查有界字段、部分写、零进展、断管和终止报告回退；Linux 另检查 EINTR、既有 pending SIGPIPE 的保留，以及正常／终止路径的信号调用前 `errno` 清零；替换调用在成功后留下非零值，下一调用必须自行清零。HostBatch／probe 的确认顺序、字节输入及读错误由 core 原生 `integration::protocol::` 从生成可执行入口验证。OS 资源获取和栈保证故障注入仅编译到显式测试构建（`OS_HOST_FAILURE_TESTS`）；生产 HostBatch／probe 不读取 `AUTOSAR_OS_FAIL_RESOURCE` 或 `AUTOSAR_OS_BAD_GUARANTEE`。

生成的 C 与运行时源码使用同一 `.clang-format`：emitter 直接写出该排版（规则在 `core/src/integration/c_layout.rs`），生成时不调用格式化工具（ADR 0014）。生成样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_generated_format.py` 用 uv 固定的 clang-format 检查样本中除 `kernel/`（FreeRTOS）和 `app/`（用户源码）外的 C 与头文件，必需 CI 在两个平台的 `c-analysis-prepare` 中运行。过长函数头的断行由 clang-format 按惩罚值择优，emitter 只复现常见选择，超长的用户名称可能排版不同。

同一批样本还用 clang-tidy 按[C 编码规范](../c-coding-standard.md#自动检查)与基线比较：`uv run --locked python -m autosar_tooling quality --scope runtime --c-samples build/c-analysis-samples`，必需 CI 在 Linux 的 `c-analysis-prepare` 中运行。

clang 作为第二编译器检查同一批样本：`uv run --locked python -B -m unittest discover -s tests/python/native -p test_clang_build.py` 把样本内核按 `target.json` 打上补丁，再用样本自身的编译选项（含 `-Werror`）分别以正式与 test 模式（`-DECU_TARGET_TESTS -DOS_HOST_FAILURE_TESTS`）编译除 `kernel/`、`app/` 外的全部编译单元，另开启 `-Wshadow`、`-Wmissing-prototypes`、`-Wstrict-prototypes`、`-Wcast-qual`、`-Wextra-semi`、`-Wformat=2`、`-Wdocumentation`。与 MISRA 重复的诊断不加入（如 `-Wunused-macros`、`-Wvla`、`-Wsign-conversion`、`-Wcomma`、`-Wswitch-enum`）。`-Wdocumentation` 的存量警告按 C 规范源码基线逐项比较：只对这一组用 `-Wno-error=documentation` 保留警告输出，再拒绝新增或修好未移除的条目；其他编译错误仍直接失败。`void` 函数使用 `@par Return`／`None.`，AUTOSAR 强制无名的 role 参数使用按位置和方向说明的 `@par Parameters`；写法与校验见 C 编码规范。现存文档诊断仍随模块回填从基线删除，新增代码不能增加警告。clang 取自 `AUTOSAR_CLANG` 或 PATH，必需 CI 在 Linux 的 `c-analysis-prepare` 中运行；它只编译不链接，不替代受锁定的 GCC 构建。

生成上述样本后，`uv run --locked python -B -m unittest discover -s tests/python/native -p test_ecu_build_modes.py` 通过封存工程的离线 CLI 实际构建 HostBatch、probe 和 test 三种模式，分别运行正常、非法资源参数、首次资源失败及栈保证失败场景。构建输出位于临时 Git 仓库内，覆盖内核选择补丁不能受外层仓库路径影响的回归。正式构建应完成正常协议，test 模式应拒绝故障；运行前将 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT` 设置为目标 GCC、objdump、git 的绝对路径；直接调用构建 API 的内核身份反例也读取这组环境，不能只依赖 PATH。

`uv run --locked python -B -m unittest discover -s tests/python/native -p test_cmake_build.py` 用交付工程生成的 `CMakeLists.txt` 以 Ninja 构建 `user-application` 样本，核对每个编译单元、编译选项和 include 顺序与 `target.json` 一致，检查两个程序的受检段，并与 `ecu-tool` 构建的程序比较运行输出；构建后封存清单不变。CMake 与 Ninja 来自 uv 的 `build` 依赖组，也可用 `CMAKE`／`NINJA` 指定。

固定 addon 的 R21.8 仍报告 `getenv`，与修订基线不一致；返回摘要的 `knownCheckerLimitations` 标明此缺口，原始诊断保留。它也没有在该 checker 的终止函数清单中列出 `_Exit`／`quick_exit`，不能据此作完整规则覆盖声明。

真实诊断、工具缺失、版本错误、超时、编译／解析失败和扫描不完整均返回非零。无诊断只说明这些自动检查通过；221 项规则／指令中的人工评估仍需根据需求、设计、授权规范和偏离审批完成，不会声明“完整符合 MISRA”。

CI 每个平台先验证检查器、生成四类完整样本并发布本轮工具与样本工件，再把标准 ECU（`standard-ecu`）、用户应用 ECU（`user-application`）、多组件流水线 ECU（`component-pipeline`）及不含应用 DID 与诊断路由的 ECU（`no-diagnostic`）分成四个并行任务；汇总任务要求所有准备和分析任务成功，跳过或取消也不会通过（仅改文档的 PR 整体跳过，见 [CI](README.md#ci)）。固定版本 Cppcheck、vcpkg 二进制包与 Cargo 构建使用缓存，每次仍重新生成样本并分析全部源码，缓存不包含扫描结果。PR 的新提交会取消同一 PR 的旧任务。双平台任务自动验证完整分析链路，拒绝配置／解析失败或漏扫；源码违规仍保留非零退出码及 `passed=false`，不会由链路验证改写为通过。严格源码门禁暂限手动 workflow_dispatch。现存诊断整改与必要偏离获批后，才能移除该条件，启用每次 PR 的严格源码门禁。手动扫描收集所有样本结果后返回真实失败状态，日志与样本保留 14 天。

## 改动生成 C 时的比较

- 重组生成代码后按 ADR 0014 比较主线样本时，在改动前后分别用 `AUTOSAR_C_ANALYSIS_SAMPLES` 生成全部样本逐文件比较。规则集身份与 `integration.json` 的生成器摘要按 core 实现源码字节计算，任何源码改动都会改变各元数据文件和 `tools/ecu-tool.py` 中的哈希：元数据把 64 位十六进制哈希归一化后比较，生成的 C、头文件与 README 必须原样相同，其余差异逐项归因。
- 交付包新增不参与编译的 C 文件（如组件骨架）时，核对 `c-check` 的补充单元：它把封存清单中构建闭包之外的 `.c` 并入每个程序分析，与应用重复的定义会在应用源上产生 8.6 等跨单元误报。不属于任何程序的文件应明确排除并加用例，再按“规则＋文件＋行”与基线比较。
- 对复制源码应用带文件选择的 Git 补丁时，隔离外层仓库发现，并验证预期文件实际改变；零退出码不证明选中补丁已应用。

## 与 master CI 基线比较

每次仍完整运行本地分析。可用已完成的 master `Development checks` 工件省去在本机重算未修改基线的那次扫描；先用 `gh run list --branch master --workflow checks.yml` 选择对应提交的成功运行，再下载**完整**分析工件，例如 Linux 标准 ECU：

```bash
gh run download <run-id> --name c-analysis-linux-x64-controlled-v1-standard-ecu --dir build/c-baseline
uv run --locked python -m autosar_tooling c-check --target linux-x64-controlled-v1 --project build/c-analysis-samples/standard-ecu --output-directory build/c-analysis-results/standard-ecu --baseline build/c-baseline/standard-ecu
```

其他样本按同样方式下载 `c-analysis-<target>-<group>`，各次下载和分析使用新的目录。工件保留 14 天。`c-analysis-summaries-*` 只有摘要，没有诊断对应的源码文本，不能单独用于比较。

`--baseline` 指向含 `summary.json` 和 `project/` 的样本目录。完整本地扫描后，入口核对 Cppcheck 版本、源码 revision 与三个 addon 摘要、checker coverage、目标、样本的完整 `target.json` 与生成计划和全部 TU／程序范围，以及基线源清单摘要。`semanticIdentity.version=1` 另核对实际 GCC 目标与数值 release、完整预定义行、有效 builtin／逐 TU 宏定义、平台模型头／库内容、实际显式加载的 Cppcheck std／POSIX 或 Windows cfg、按正常查找顺序隐式加载的 std.cfg、实际 addon runner 及其 cppcheck／cppcheckdata 辅助文件，以及项目分析驱动、addon 和直接构建／进程辅助源码（含 POSIX launch 与 Windows Job）的内容摘要。文本摘要统一换行；预定义与宏定义排序，安装路径、结果目录、并发数及编译器／Cppcheck 二进制载体摘要仅作追溯，不决定等价。相同目标、版本、有效输入和分析代码可跨主机比较；版本字符串单独相同不足以证明等价，改变这些内容需要同配置重新扫描基线。驱动源码使用保守字节身份，注释变化也可能要求新基线。

缺失、旧版本或不完整语义身份明确拒绝，旧工件保持原样；结果身份版本不代表工具来源证明或 MISRA 评估状态。正常入口在扫描前捕获身份并在扫描后检查内容未变化。隐式路径按固定版本的实际规则及程序工作目录解析，不能由 CPPCHECK_ADDON_DIR 推定 runner；隐式 cppcheck.cfg 额外引入 addon、suppression 或 safety 覆盖时明确拒绝，纯展示元数据不影响等价。不同主机编译的 Cppcheck 二进制 SHA-256 可不同，版本和 addon 必须一致。任何身份不同、工件缺失或分析不完整都明确拒绝：在对应基线检出，用相同配置和工具通过正常 c-check 命令本地计算基线。配置本身改变时也需重新确定可比较的同配置基线，不能强行接受不相符的 CI 样本。

比较按每个诊断位置的“规则＋相对文件＋源码行文本”及程序出现次数计数，源码移动不算新增；新增和移除分别输出，不能用净差抵消，完整条目保存在现有 `summary.json` 的 `comparison.new`／`removed`。本地扫描与基线比较各自保留事实：零新增不改变 `passed=false`，现存违规仍返回非零；身份拒绝也返回非零。它不是扫描通过缓存，不减少样本、规则或比较粒度。严格源码门的启用与实际阻断证据由 [#55](https://github.com/g1331/autsaro/issues/55) 承接。
