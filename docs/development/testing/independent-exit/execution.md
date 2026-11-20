# 双平台执行命令

按[验收入口](README.md)冻结版本，Linux 与 Windows 均由与实现无关的独立会话执行。每个平台准备同一个提交的工作台、参考工程输入和本平台封存包。发布工具及主机依赖见[开发环境](../../environment.md)
及[交付说明](../../../architecture/targets-and-delivery.md)。工具路径用实际安装位置。Windows 使用 PowerShell 7、MSVC／WebView2，
并按[必需 CI 的编译器安装定义](../../../../.github/actions/native-analysis-compiler/action.yml)安装同版 MinGW GCC／binutils。
Linux 使用受控版本 GCC／binutils。目标清单要求的身份由包内 build 自行核对。接收者 Python 使用交付支持的 3.11–3.14。Linux 本地验证命令以 `nice -n 10` 串行执行。

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
export AUTOSAR_RECEIVER_PYTHON=/absolute/path/to/receiver/python3
uv run --locked python -B -m unittest discover -v -s tests/python/native -p test_offline_delivery.py \
  2>&1 | tee "$ROOT/results/B1-receiver-matrix.log"
cargo test --locked -p autosar-config-core --test it -- integration:: \
  2>&1 | tee "$ROOT/results/A3-config.log"
cargo test --locked -p autosar-config-core --test it -- delivery:: \
  2>&1 | tee "$ROOT/results/A3-delivery.log"
```

## Windows 发送端

在 GitHub 托管的 Windows runner（`windows-2025-vs2026`）或隔离专用桌面会话中，由与实现无关的独立会话运行。先把受控 MinGW 和配套 WebDriver 放到 PATH，并从 Visual Studio 开发环境启动。

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
  继续执行[编辑后交付复验](edited-handoff.md)，把单／多 SWC 保存重开后的实际导出交给两个生产 main。固定样本另做回归。
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
  `single_component_meets_independent_protocol_and_timing_vectors`，核对 SID `0x27` 返回 NRC `0x11`。独立验收人将原正向 BCrypt 向量按该移交裁决登记，历史备注与当前支持范围的最终判定由协调者在 #56 出口判定中确认。
- **A3 保护。** 在 `A3-delivery` 与 `A2-native` 核对旧 profile／清单／包身份拒绝、篡改与重新封签伪造、旧有效产物保留；
  在 B3 实际跑过外部改动使预览失效、用户源 create-only 与快照导入。把失败对象、错误码、拒绝前后用户文件及封存摘要留在同一条记录。
  旧格式原生用例缺失或跳过时回到交付／工作台 owner 补证。构建模式中的 test 入口属夹具，生产交付的两个 main 分别验收。

## B1 搬移包与隔离复验

发送端用下列命令，从冻结的四份回归样本生成 SHA-256 清单。包、脚本、`identity.txt` 随交付工件或介质移交（在 GitHub 托管 Windows runner 上，发送端作业将其作为工件上传，接收端作业只下载该工件、不检出源码）。
通过可信交接渠道单独发送 `sender-seals.json`，并注明冻结 SHA 和目标。收到同名的其他有效封存树也须在任何包内代码运行前拒绝。

Linux 发送端：

```bash
"$AUTOSAR_PYTHON" -I -S scripts/rehearse_ecu_handoff.py seals "$AUTOSAR_C_ANALYSIS_SAMPLES" \
  > "$ROOT/results/sender-seals.json"
```

Windows 发送端：

```powershell
& $env:AUTOSAR_PYTHON -I -S scripts/rehearse_ecu_handoff.py seals $env:AUTOSAR_C_ANALYSIS_SAMPLES |
    Set-Content -Encoding utf8 "$Root/results/sender-seals.json"
if ($LASTEXITCODE -ne 0) { throw "Could not record the sender seals" }
```

双平台接收端均由与实现无关的独立会话在只含四个样本、脚本、可信清单、已安装 Python、受控 GCC／binutils 与 Git 的空目录中执行，不得读取或挂载源码 checkout：

- **Linux 接收端**：保持原有离线与无源码核验标准不放松——在独立主机或隔离环境中**断开网卡、移除共享 checkout／挂载**，保留隔离记录。
- **Windows 接收端（#56 方案 1）**：在 GitHub 托管的 `windows-2025-vs2026` runner 上执行——**不检出源码仓库**（仅下载搬移包与清单工件），通过 Windows 防火墙出站阻断规则禁止被测程序及接收构建／运行入口联网，并保存防火墙规则记录。在 `#56` 验收记录中须如实写明与“整机断网”原要求的唯一差距：GitHub 托管 runner 自身仍需与 GitHub 通信，仅做到被测程序断网。

接收端解析四个固定相对路径并逐个比较发送端摘要，缺项或不符即停止。
预检通过后，即使构建或运行失败也再比较同样的四条摘要。`seal-after.json` 保留四个固定路径的实际摘要或读取错误，
某个路径缺失时仍记录其他路径。保留前后摘要、比较错误和各阶段退出码。
正常 `test_offline_delivery.py` 的矩阵补充 Python 兼容性和发送端路径检查。

Linux 在独立 Bash 会话中进入接收目录。把 `RECEIVER_PYTHON` 改成预装 CPython 的绝对路径：

```bash
set -e
RECEIVER_PYTHON=/absolute/path/to/receiver/python3
export AUTOSAR_CC=/usr/bin/gcc-13 AUTOSAR_OBJDUMP=/usr/bin/objdump AUTOSAR_GIT=/usr/bin/git
mkdir receipts
"$RECEIVER_PYTHON" -I -S -c 'import pathlib, sys; print(pathlib.Path(sys.executable).resolve()); print(sys.version)' \
  > receipts/python.txt
"$RECEIVER_PYTHON" -I -S rehearse_ecu_handoff.py seals . --expected sender-seals.json \
  > receipts/seal-before.json
run_profiles() {
  local sample
  "$RECEIVER_PYTHON" -I -S rehearse_ecu_handoff.py run standard-ecu receipts/standard-ecu --verify || return $?
  "$RECEIVER_PYTHON" -I -S rehearse_ecu_handoff.py run component-pipeline receipts/component-pipeline --pipeline || return $?
  for sample in user-application no-diagnostic; do
    "$RECEIVER_PYTHON" -I -S rehearse_ecu_handoff.py run "$sample" "receipts/$sample" || return $?
  done
}
run_code=0
run_profiles || run_code=$?
seal_code=0
"$RECEIVER_PYTHON" -I -S rehearse_ecu_handoff.py seals . --expected receipts/seal-before.json \
  > receipts/seal-after.json 2> receipts/seal-after.stderr.log || seal_code=$?
printf 'run %s\nseal-after %s\n' "$run_code" "$seal_code" > receipts/exit-codes.txt
if [ "$run_code" -ne 0 ]; then exit "$run_code"; fi
exit "$seal_code"
```

Windows 按 [#56 方案 1](https://github.com/g1331/autsaro/issues/56#issuecomment-7031215781) 在不检出源码的 GitHub 托管 Windows runner（`windows-2025-vs2026`）独立会话中执行（历史 D-24 义务按此方式落实，并在记录中写明仅被测程序断网的差距；不设维护者人工签字）。
进入仅含搬移包工件的接收目录，把 `$Python` 改成预装 CPython 的绝对路径，先配置并记录禁止被测程序出站联网的 Windows 防火墙规则再运行：

```powershell
$env:AUTOSAR_CC = (Get-Command gcc.exe).Source
$env:AUTOSAR_OBJDUMP = (Get-Command objdump.exe).Source
$env:AUTOSAR_GIT = (Get-Command git.exe).Source
$Python = 'C:/absolute/path/to/receiver/python.exe'
$PSNativeCommandUseErrorActionPreference = $false
New-Item -ItemType Directory receipts | Out-Null
$RuleGroup = 'Autsaro-Independent-Exit-B1'
$BlockedPrograms = @(
    $Python, $env:AUTOSAR_CC, $env:AUTOSAR_OBJDUMP, $env:AUTOSAR_GIT,
    (Join-Path $PWD 'receipts' 'standard-ecu' 'verify' 'ecu_host_batch.exe')
) + (
    'standard-ecu', 'component-pipeline', 'user-application', 'no-diagnostic' | ForEach-Object {
        (Join-Path $PWD 'receipts' $_ 'probe' 'ecu_probe.exe')
        (Join-Path $PWD 'receipts' $_ 'host-batch' 'ecu_host_batch.exe')
    }
) | ForEach-Object { [System.IO.Path]::GetFullPath($_) }
foreach ($Program in $BlockedPrograms) {
    New-NetFirewallRule -DisplayName "Block-$($Program)" -Group $RuleGroup `
        -Direction Outbound -Action Block -Program $Program -Profile Any | Out-Null
}
Get-NetFirewallRule -Group $RuleGroup | Format-List DisplayName,Enabled,Direction,Action |
    Out-File -Encoding utf8 receipts/firewall-rules.txt
& $Python -I -S -c 'import pathlib, sys; print(pathlib.Path(sys.executable).resolve()); print(sys.version)' |
    Set-Content -Encoding utf8 receipts/python.txt
if ($LASTEXITCODE -ne 0) { throw "Receiver Python is unavailable" }
& $Python -I -S rehearse_ecu_handoff.py seals . --expected sender-seals.json |
    Set-Content -Encoding utf8 receipts/seal-before.json
if ($LASTEXITCODE -ne 0) { throw "Received seals differ from the trusted sender inventory" }
$RunCode = 1
try {
    foreach ($Sample in 'standard-ecu', 'component-pipeline', 'user-application', 'no-diagnostic') {
        $Options = @()
        if ($Sample -eq 'standard-ecu') { $Options += '--verify' }
        if ($Sample -eq 'component-pipeline') { $Options += '--pipeline' }
        & $Python -I -S rehearse_ecu_handoff.py run $Sample "receipts/$Sample" @Options
        $RunCode = $LASTEXITCODE
        if ($RunCode -ne 0) { throw "$Sample handoff failed with exit $RunCode" }
    }
} finally {
    & $Python -I -S rehearse_ecu_handoff.py seals . --expected receipts/seal-before.json `
        2> receipts/seal-after.stderr.log | Set-Content -Encoding utf8 receipts/seal-after.json
    $SealCode = $LASTEXITCODE
    @("run $RunCode", "seal-after $SealCode") | Set-Content -Encoding utf8 receipts/exit-codes.txt
    Remove-NetFirewallRule -Group $RuleGroup -ErrorAction SilentlyContinue
}
if ($SealCode -ne 0) { throw "End-of-run seal check failed with exit $SealCode" }
```

保留发送端清单、解释器身份、Linux 断网隔离记录或 Windows 防火墙规则记录以及前后匹配的摘要。每个样本须有 PASS、`identity.json`（含实际解释器、版本、包摘要和受控 PATH）、
两个 `build-*.log` 以及 `host-batch.log`、`probe.log`。脚本给包内子进程使用受控工具路径和平台必要环境变量。
standard-ecu 另有 `verify.log` 和 `verify/` 下的协议记录。component-pipeline 核对五次批次确认及五帧精确输出。
可把同样的 standard-ecu 命令作为 CI 冒烟，输入取对应目标的 `c-analysis-inputs-*` 工件。

## B2–B3 工作台与真实 IPC

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

安装与卸载另见 [B4 安装包实测](installation.md)。
