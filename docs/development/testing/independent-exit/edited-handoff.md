# A1 到 A2 编辑后交付复验

先按[执行篇](execution.md)准备冻结版本、受控工具和 B3 验证版工作台。这里单独运行两个场景，使用新的结果目录。
两个场景均通过真实 Tauri／IPC 编辑、保存、重开并生成封存树。接下来的 HostBatch 和 probe 从该场景刚生成的目录独立构建并实际运行。
固定四个 profile 从 `c_analysis::` 生成，再按[执行篇的 B1](execution.md#b1-搬移包并实际断网)执行回归。

| 场景 | 通过判据 |
| --- | --- |
| 单 SWC `seven-source-standard-variant` | 从内置单 SWC 工程把 CAN ID 改成 1104／1105、周期改成 20 ms，保存并重开。断言实际重开值，导出 `GatewayReference-source` |
| 多 SWC `composition-shared-editing-and-source-protection` | 从三组件 Pipeline 输入编辑事件名称及链接，保存并重开。断言长名称对应的周期值 `0.0100` 和 `IngressObserve` 的提供者目标路径及 DEST，导出 `Pipeline-source`，核对封存的应用源码字节 |
| 同包运行 | 每个编辑收据保存原输入副本、重开值、按逻辑路径核对的重开与封存输入 SHA-256、生成目录和封存摘要。脚本核对收据目录和摘要，分别构建并运行两个 main。HostBatch 准备就绪、确认单笔合法空批次并正常关闭，probe 完成 20 ticks |

只筛选上述两个场景。标准交付文件随后还有一个会搬移该包的场景，单独运行 B3 全套时另起结果目录。

## Linux

在已完成 B3 构建准备的发送端源码目录执行。`AUTOSAR_RECEIVER_PYTHON` 使用[执行篇](execution.md)选定的绝对路径。

```bash
(
  set -euo pipefail
  export AUTOSAR_DESKTOP_RESULTS="$ROOT/results/A1-edited-desktop"
  test ! -e "$AUTOSAR_DESKTOP_RESULTS"
  pattern='^(seven-source-standard-variant|composition-shared-editing-and-source-protection)$'
  nice -n 10 node --test --test-concurrency=1 --test-name-pattern "$pattern" \
    tests/desktop/standard-delivery.test.mjs tests/desktop/composition-editing.test.mjs \
    2>&1 | tee "$ROOT/results/A1-edited-desktop.log"
  shopt -s nullglob
  for sample in GatewayReference-source Pipeline-source; do
    matches=("$AUTOSAR_DESKTOP_RESULTS"/*/deliveries/"$sample.edit.json")
    test "${#matches[@]}" -eq 1
    receipt="${matches[0]}"
    nice -n 10 "$AUTOSAR_RECEIVER_PYTHON" -I -S scripts/rehearse_ecu_handoff.py run \
      "$(dirname "$receipt")/$sample" "$ROOT/results/A2-edited-$sample" --edit-receipt "$receipt"
  done
)
```

## Windows

在执行篇的专用 Windows 会话中使用同一验证版工作台。先完成 B3 的构建及环境变量设置，再运行：

```powershell
$env:AUTOSAR_DESKTOP_RESULTS = "$Root/results/A1-edited-desktop"
if (Test-Path $env:AUTOSAR_DESKTOP_RESULTS) { throw "Choose a fresh desktop results directory" }
$Pattern = '^(seven-source-standard-variant|composition-shared-editing-and-source-protection)$'
Save-Check A1-edited-desktop node @('--test','--test-concurrency=1','--test-name-pattern',
    $Pattern,'tests/desktop/standard-delivery.test.mjs','tests/desktop/composition-editing.test.mjs')
if ($LASTEXITCODE -ne 0) { throw "Edited desktop scenarios failed" }
foreach ($Sample in 'GatewayReference-source', 'Pipeline-source') {
    $Receipts = @(Get-ChildItem $env:AUTOSAR_DESKTOP_RESULTS -Recurse -File -Filter "$Sample.edit.json")
    if ($Receipts.Count -ne 1) { throw "Expected one edit receipt for $Sample" }
    $Receipt = $Receipts[0]
    Save-Check "A2-edited-$Sample" $env:AUTOSAR_RECEIVER_PYTHON @('-I','-S',
        'scripts/rehearse_ecu_handoff.py','run',"$($Receipt.DirectoryName)/$Sample",
        "$Root/results/A2-edited-$Sample",'--edit-receipt',$Receipt.FullName)
    if ($LASTEXITCODE -ne 0) { throw "$Sample native handoff failed" }
}
```

两平台都应保留 `A1-edited-desktop` 内的真实 IPC 日志、截图、两个 `*.edit.json`、`*.edit-inputs/` 和同级的两个封存包。
`A2-edited-*` 含 `identity.json`、分别构建的 `build-host-batch.log` 和 `build-probe.log`，以及两个执行日志。
在归档证据前核对每组 `identity.json` 中的包路径和摘要与对应编辑收据一致。缺少任何一组收据，或编辑、导出、构建、运行中任一步失败，
均退回对应的工作台或交付责任人（#52）；报告时附具体阶段与日志。
