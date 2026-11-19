# 双平台执行命令

按[验收入口](README.md)冻结版本。每个平台准备同一个提交的工作台、参考工程输入和本平台封存包。发布工具及主机依赖见[开发环境](../../environment.md)
及[交付说明](../../../architecture/targets-and-delivery.md)。工具路径用实际安装位置。Windows 使用 PowerShell 7、MSVC／WebView2，
并按[必需 CI 的编译器安装定义](../../../../.github/actions/native-analysis-compiler/action.yml)安装同版 MinGW GCC／binutils。
Linux 使用受控版本 GCC／binutils。目标清单要求的身份由包内 build 自行核对。接收者 Python 使用交付支持的 3.11–3.14。

以下以空临时目录为起点。把 `FULL_MASTER_SHA` 替换为已冻结的提交。保留每个命令的退出码，失败时先保存现场再继续独立步骤。发送端的签名和链接验收还需要受控 binutils 的 `nm`。

## Linux 发送端

```bash
set -o pipefail
REV=FULL_MASTER_SHA
ROOT=$(mktemp -d)
mkdir "$ROOT/results" "$ROOT/tmp"
git clone https://github.com/g1331/autsaro.git "$ROOT/source"
cd "$ROOT/source"
git -c core.autocrlf=false checkout --detach "$REV"
export TMPDIR="$ROOT/tmp" UV_CACHE_DIR="$ROOT/uv-cache"
export AUTOSAR_CC=$(command -v gcc-13)
export AUTOSAR_OBJDUMP=$(command -v objdump)
export AUTOSAR_NM=$(command -v nm)
export AUTOSAR_GIT=$(command -v git)
uv sync --locked
export AUTOSAR_PYTHON=$(uv run --locked python -c 'import sys; print(sys.executable)')
export AUTOSAR_C_ANALYSIS_SAMPLES="$ROOT/samples"
{
  git rev-parse HEAD
  uname -a
  "$AUTOSAR_CC" --version
  "$AUTOSAR_OBJDUMP" --version
  "$AUTOSAR_NM" --version
  "$AUTOSAR_PYTHON" --version
  "$AUTOSAR_GIT" --version
  sha256sum "$AUTOSAR_CC" "$AUTOSAR_OBJDUMP" "$AUTOSAR_NM"
} > "$ROOT/results/identity.txt"
cargo test --locked -p autosar-config-core --test it -- c_analysis:: \
  2>&1 | tee "$ROOT/results/A1-generate.log"
```

串行运行原生套件。保持原期限。若出现 `supervisor shutdown timed out`，先保存日志、原二进制和 scratch，
再用同一测试名按 `--exact --test-threads=1` 单独运行并另存记录。

```bash
cargo test --locked -p autosar-config-core --features native-tests --lib --test native \
  --no-fail-fast -- --test-threads=1 2>&1 | tee "$ROOT/results/A2-native.log"
AUTOSAR_OS_SHARD=0/1 AUTOSAR_OS_OUTPUT_DIRECTORY="$ROOT/results/A2-os" \
  uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_os_suites.py \
  2>&1 | tee "$ROOT/results/A2-os.log"
uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_host_boundaries.py \
  2>&1 | tee "$ROOT/results/A2-boundaries.log"
uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_ecu_build_modes.py \
  2>&1 | tee "$ROOT/results/A3-build-modes.log"
uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_cmake_build.py \
  2>&1 | tee "$ROOT/results/A2-cmake.log"
AUTOSAR_RECEIVER_PYTHON="$(command -v python3)" \
  uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_offline_delivery.py \
  2>&1 | tee "$ROOT/results/B1-receiver-matrix.log"
cargo test --locked -p autosar-config-core --test it -- integration:: \
  2>&1 | tee "$ROOT/results/A3-config.log"
cargo test --locked -p autosar-config-core --test it -- delivery:: \
  2>&1 | tee "$ROOT/results/A3-delivery.log"
```

## Windows 发送端

在专用桌面会话中运行。先把受控 MinGW 和配套 WebDriver 放到 PATH，并从 Visual Studio 开发环境启动。

```powershell
$Rev = 'FULL_MASTER_SHA'
$Root = Join-Path ([IO.Path]::GetTempPath()) ("autsaro-acceptance-" + [guid]::NewGuid())
New-Item -ItemType Directory "$Root/results", "$Root/tmp" | Out-Null
git clone https://github.com/g1331/autsaro.git "$Root/source"
Set-Location "$Root/source"
git -c core.autocrlf=true checkout --detach $Rev
$env:TEMP = "$Root/tmp"
$env:TMP = "$Root/tmp"
$env:UV_CACHE_DIR = "$Root/uv-cache"
$env:AUTOSAR_CC = (Get-Command gcc.exe).Source
$env:AUTOSAR_OBJDUMP = (Get-Command objdump.exe).Source
$env:AUTOSAR_NM = (Get-Command nm.exe).Source
$env:AUTOSAR_GIT = (Get-Command git.exe).Source
uv sync --locked
$env:AUTOSAR_PYTHON = (uv run --locked python -c 'import sys; print(sys.executable)')
$env:AUTOSAR_C_ANALYSIS_SAMPLES = "$Root/samples"
& {
    git rev-parse HEAD
    Get-ComputerInfo -Property WindowsProductName,WindowsVersion,OsBuildNumber
    & $env:AUTOSAR_CC --version
    & $env:AUTOSAR_OBJDUMP --version
    & $env:AUTOSAR_NM --version
    & $env:AUTOSAR_PYTHON --version
    & $env:AUTOSAR_GIT --version
    Get-FileHash $env:AUTOSAR_CC, $env:AUTOSAR_OBJDUMP, $env:AUTOSAR_NM
} > "$Root/results/identity.txt"
cargo test --locked -p autosar-config-core --test it -- c_analysis:: 2>&1 |
    Tee-Object "$Root/results/A1-generate.log"
```

用以下函数保留每次 Cargo／uv 调用的退出状态。一次只运行一个原生套件。
`AUTOSAR_RECEIVER_PYTHON` 请填接收端支持的独立 CPython 的路径。

```powershell
function Save-Check($Name, $Program, [string[]]$Arguments) {
    & $Program @Arguments 2>&1 | Tee-Object "$Root/results/$Name.log"
    $Code = $LASTEXITCODE
    Add-Content "$Root/results/exit-codes.txt" "$Name $Code"
}
Save-Check A2-native cargo @('test','--locked','-p','autosar-config-core','--features','native-tests',
    '--lib','--test','native','--no-fail-fast','--','--test-threads=1')
$env:AUTOSAR_OS_SHARD = '0/1'
$env:AUTOSAR_OS_OUTPUT_DIRECTORY = "$Root/results/A2-os"
Save-Check A2-os uv @('run','--locked','python','-B','-m','unittest','discover','-v',
    '-s','tests/python/native','-p','test_os_suites.py')
@{
    'A2-boundaries' = 'test_host_boundaries.py'
    'A3-build-modes' = 'test_ecu_build_modes.py'
    'A2-cmake' = 'test_cmake_build.py'
}.GetEnumerator() | ForEach-Object {
    Save-Check $_.Key uv @('run','--locked','python','-B','-m','unittest','discover','-v',
        '-s','tests/python/native','-p',$_.Value)
}
$env:AUTOSAR_RECEIVER_PYTHON = 'C:/path/to/receiver/python.exe'
Save-Check B1-receiver-matrix uv @('run','--locked','python','-B','-m','unittest','discover','-v',
    '-s','tests/python/native','-p','test_offline_delivery.py')
Save-Check A3-config cargo @('test','--locked','-p','autosar-config-core','--test','it','--','integration::')
Save-Check A3-delivery cargo @('test','--locked','-p','autosar-config-core','--test','it','--','delivery::')
```

## A1–A3 的观察结果

- **A1 配置往返与生成。** `c_analysis::` 生成 `standard-ecu`、`user-application`、`component-pipeline` 和 `no-diagnostic`。
  保留四个 `meta/files.sha256`、`meta/integration.json`、`target.json`。再执行 B3 的配置编辑、保存重开和真实生成，
  核对重开值、原工程、用户 live 源码及封存应用，接收者须分别有单／多 SWC 的实际记录。
- **A2 独立行为。** 每个封存样本的 HostBatch 与 probe 分别从新目录构建。probe 完成 20 ticks，合法批次有 COMMIT 回执。
  `standard-ecu` 的 verify 检查 CAN/DID、分段接收超时后的恢复、非法批次及 probe。`component-pipeline` 的
  `test_offline_delivery.py` 按独立帧向量核对 epoch 0／1／10／10／20，包含重复 epoch 与调度后的精确帧／DID 内容。
  Cargo 原生套件提供各模块成功、拒绝、恢复及实际消费者编译／链接，定位到 `integration::protocol`、`sealed_projects`、
  `release_contracts`、`single_component` 和 `os`。按受影响模块从其已合并 PR 补齐对应的用例和原文编号。
- **A2 OS 与 PE／ELF。** 从 `tests/python/native/test_os_suites.py` 的完整发现结果核对每个 suite 的输出。当前基线为 27 个，
  覆盖 Issue 所列的 26 项及后来加入的套件。两个目标保留机器码与段、栈、TLS、异常及实际恢复断言。
  Windows 的 PE 段／向量／TLS 另看 `A2-os`、`A2-boundaries`、受控包 build 两个 main 的日志。
- **A2 Windows crypto。** 历史要求针对 9.1 的 BCrypt／SecurityAccess 档案。当前主线按
  [#33](https://github.com/g1331/autsaro/issues/33) 与 [PR #124](https://github.com/g1331/autsaro/pull/124) 把该能力交给 R7。
  在 Windows 运行 `A3-config` 中的 SecurityAccess 配置拒绝和 `A2-native` 的
  `single_component_meets_independent_protocol_and_timing_vectors`，核对 SID `0x27` 返回 NRC `0x11`。独立验收人将原正向 BCrypt 向量按该移交裁决登记，历史备注与当前支持范围的最终判定由 #56 负责人确认。
- **A3 保护。** 在 `A3-delivery` 与 `A2-native` 核对旧 profile／清单／包身份拒绝、篡改与重新封签伪造、旧有效产物保留；
  在 B3 实际跑过外部改动使预览失效、用户源 create-only 与快照导入。把失败对象、错误码、拒绝前后用户文件及封存摘要留在同一条记录。
  旧格式原生用例缺失或跳过时回到交付／工作台 owner 补证。构建模式中的 test 入口属夹具，生产交付的两个 main 分别验收。

## B1 搬移包并实际断网

发送端用文件传输介质移交四份封存树和 [接收脚本](../../../../scripts/rehearse_ecu_handoff.py)，另附 `identity.txt`、
四份清单的独立摘要以及使用说明。发送端先核对摘要。接收端用空 VM 或空目录所在的独立主机环境，只带包、脚本、已安装的 Python、
受控 GCC/binutils 与 Git。断开网卡，移除共享 checkout／挂载后再开始。保存网卡隔离及 checkout 不可用的记录，工具用绝对路径。
用正常 `test_offline_delivery.py` 的矩阵补充 Python 兼容性与多组件帧向量。该 CI 入口还会检查封存字节及产物中的发送端路径。

Linux 在空接收目录放入四个样本目录和脚本，设置三个工具路径后在独立 Bash 会话执行：

```bash
set -e
export AUTOSAR_CC=/usr/bin/gcc-13 AUTOSAR_OBJDUMP=/usr/bin/objdump AUTOSAR_GIT=/usr/bin/git
mkdir receipts
sha256sum */meta/files.sha256 > receipts/seal-before.txt
/usr/bin/python3 -I -S rehearse_ecu_handoff.py --help
/usr/bin/python3 -I -S rehearse_ecu_handoff.py standard-ecu receipts/standard-ecu --verify
/usr/bin/python3 -I -S rehearse_ecu_handoff.py component-pipeline receipts/component-pipeline --pipeline
for sample in user-application no-diagnostic; do
  /usr/bin/python3 -I -S rehearse_ecu_handoff.py "$sample" "receipts/$sample"
done
sha256sum --check receipts/seal-before.txt
```

Windows 在真正断网、无宿主共享 checkout 的专用 Windows 主机／VM 中执行同一批交付，
应由接收者从移交的说明独立完成并签录（历史 D-24）。

```powershell
$env:AUTOSAR_CC = (Get-Command gcc.exe).Source
$env:AUTOSAR_OBJDUMP = (Get-Command objdump.exe).Source
$env:AUTOSAR_GIT = (Get-Command git.exe).Source
$Python = (Get-Command python.exe).Source
New-Item -ItemType Directory receipts | Out-Null
$Seals = Get-FileHash */meta/files.sha256
$Seals | Out-File receipts/seal-before.txt
& $Python -I -S rehearse_ecu_handoff.py --help
& $Python -I -S rehearse_ecu_handoff.py standard-ecu receipts/standard-ecu --verify
if ($LASTEXITCODE -ne 0) { throw "standard-ecu handoff failed" }
& $Python -I -S rehearse_ecu_handoff.py component-pipeline receipts/component-pipeline --pipeline
if ($LASTEXITCODE -ne 0) { throw "component-pipeline handoff failed" }
foreach ($Sample in 'user-application', 'no-diagnostic') {
    & $Python -I -S rehearse_ecu_handoff.py $Sample "receipts/$Sample"
    if ($LASTEXITCODE -ne 0) { throw "$Sample handoff failed" }
}
$After = Get-FileHash */meta/files.sha256
$After | Out-File receipts/seal-after.txt
if (Compare-Object $Seals $After -Property Path,Hash) { throw "received seals changed" }
```

接收端检查每个目录均有 `build-host-batch.log`、`build-probe.log`、`host-batch.log` 和 `probe.log`。
standard-ecu 另有 `verify.log` 及 `verify/` 下的原始协议记录。component-pipeline 在断网接收端直接核对五次批次确认及五帧输出，
包括 10／20 tick 调度后的 CAN 回显及 DID 值。每个样本都要有 PASS；两平台分别保留前后清单摘要和四份 receipts。
可把同样的 standard-ecu 命令作为 CI 冒烟，输入取对应目标的 `c-analysis-inputs-*` 工件。

## B2–B4 工作台、真实 IPC 与安装包

先运行 [桌面场景](../desktop.md) 中的隔离前置检查。Linux 使用测试框架的私有 Xvfb 与 D-Bus，Windows 使用专用交互会话，
驱动版本与 WebView2 匹配。B2 的组件测试用浏览器模拟环境，另记结果。B3 的 Tauri 与 WebDriver 启动真实应用。

Linux：

```bash
npm ci --prefix ui
npm test --prefix ui 2>&1 | tee "$ROOT/results/B2-ui.log"
npm run tauri --prefix ui -- build --debug --no-bundle --features native-webdriver -- --profile desktop-test
export AUTOSAR_DESKTOP_APP="$PWD/target/desktop-test/autosar-config-desktop"
export AUTOSAR_DESKTOP_RESULTS="$ROOT/results/B3-desktop"
node --test --test-concurrency=1 'tests/desktop/*.test.mjs' 2>&1 | tee "$ROOT/results/B3-ipc.log"
```

Windows：

```powershell
npm ci --prefix ui
Save-Check B2-ui npm @('test','--prefix','ui')
Save-Check B3-build npm @('run','tauri','--prefix','ui','--','build','--debug','--no-bundle',
    '--features','native-webdriver','--','--profile','desktop-test')
$env:AUTOSAR_DESKTOP_DEDICATED_SESSION = '1'
$env:AUTOSAR_DESKTOP_APP = "$PWD/target/desktop-test/autosar-config-desktop.exe"
$env:AUTOSAR_DESKTOP_RESULTS = "$Root/results/B3-desktop"
Save-Check B3-ipc node @('--test','--test-concurrency=1','tests/desktop/*.test.mjs')
```

看 `standard-delivery`、`composition-editing`、`application`、`projects`、`rejected-change` 与 `owned-execution` 的实际结果。
有编辑确认、重开值、生成与运行交付、预览失效或拒绝后的恢复证据，并保留 IPC 记录、截图、进程退出记录。
若正式交付约定了浏览器复核，另在隔离会话运行 `npm run dev --prefix ui`，从 UI 验证相应视图和控制可操作，
记录浏览器版本与截图。浏览器检视只对应前端能力。

B4 由 [#63](https://github.com/g1331/autsaro/issues/63) 承接安装验收。按[发布说明](../../../maintainers/releasing.md)取得同 SHA 构建的
实际 Linux／Windows 包，记录介质摘要及安装品种。在各自干净、无源码的隔离 VM 中使用操作系统正常安装入口。
实际启动、从可移交的 ARXML 创建工程与保存重开、生成一次、在重启应用后重开，核对安装／卸载和权限／命名空间记录。
`Build application bundles` 的上传记录作为介质来源。安装测试的证据包括安装器实际退出状态、启动后的工作台结果和卸载记录。

Linux 在预装依赖的测试 VM 中，选择同一 SHA 的 `.deb`。其余约定交付形式按发行说明另验。

```bash
DEB=/absolute/path/to/bundle.deb
sha256sum "$DEB"
sudo apt-get --no-download install "$DEB"
PACKAGE=$(dpkg-deb -f "$DEB" Package)
dpkg-query --show "$PACKAGE"
dpkg-query -L "$PACKAGE" | grep '/bin/'
# 从上一步列出的路径启动安装版并完成上面的工作台操作。
sudo apt-get --no-download remove "$PACKAGE"
```

Windows 在断网 VM 中将所选 `.msi` 放进结果目录，再执行：

```powershell
$Msi = (Resolve-Path './bundle.msi').Path
Get-FileHash $Msi
$Install = Start-Process msiexec.exe -Wait -PassThru -ArgumentList "/i `"$Msi`" /qn /L*v install.log"
$Install.ExitCode
# 从开始菜单启动安装版并完成上面的工作台操作。
$Remove = Start-Process msiexec.exe -Wait -PassThru -ArgumentList "/x `"$Msi`" /qn /L*v uninstall.log"
$Remove.ExitCode
```

核对退出状态和 MSI 日志。安装器要求重启时先完成重启，再核对实际启动和卸载。
