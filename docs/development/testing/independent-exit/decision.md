# 出口证据与判定

检查人先读 [#30 的当前正文与评论](https://github.com/g1331/autsaro/issues/30)、
[ADR 0027](../../../adr/0027-misra-compliance-track.md) 和 [ADR 0031](../../../adr/0031-intermittent-failure-exit-policy.md)。
归档背景从标签读取，现行决定及实际 Issue 状态以主线为准。

```bash
gh issue view 56 --comments
gh issue view 30 --comments
git fetch origin tag planning-archive-2026-10
git show planning-archive-2026-10:_bmad-output/planning-artifacts/epics.md
git show planning-archive-2026-10:_bmad-output/implementation-artifacts/spec-9-1-shared-contracts.md
git show planning-archive-2026-10:_bmad-output/specs/spec-multi-component-scheduling/acceptance.md
```

## 进入条件

从 #30 逐项复核关闭／被替代依据、合并 PR 和最终证据。重点区分以下类型，未关闭的出口义务退回当前 Issue 负责人。

| 条件 | 正式运行前必须查到 |
| --- | --- |
| 模块整改和交付 | #30 其他有效项都有结论，逐项对应主线改动、适用条款、迁移／ABI、两平台和独立消费者证据 |
| #55 与 PR 保护 | #55 的实际不回退门、故障与恢复 PR、必需检查和保护均有记录。完整项目验收的后续切换由 #482 承接 |
| 间歇失败 | 对 #412、#414 等仍在清单的项，查根因修复与实际验证，或查符合 ADR 0031 的取证、首次尝试计数、复发和正式移交 |
| 工程效率 | #461 报告真实可比的 PR 墙钟、覆盖和资源成本，且满足当时的 Issue 验收 |
| MISRA 合规线 | #439、#443、#445 是 #58 的遗留子项，ADR 0027 把它们与完整人工评估移到合规线。本次复核实际验收状态及剩余责任的报告 |
| 受理人和材料 | 接收人独立，参考输入、目标工具链、许可原文、适用的工具／兼容／人工结论可取，Linux 与 Windows 均有隔离环境 |

## C1 官方档案、原文和生成 C

在发送端源码目录执行。先按 [官方对照](../official.md)准备并校验四个固定档案。每个目标执行 oracle 二进制。
条款清单用 archive-backed 核算，`--stale-only` 对照 [acceptance.yml 当前清单](../../../../.github/workflows/acceptance.yml) 的选择。

Linux：

```bash
set -o pipefail
(
  status=0
  code=0
  uv run --locked python -m autosar_tooling fetch-official 2>&1 | tee "$ROOT/results/C1-fetch.log" \
    || code=$?
  printf 'C1-fetch %s\n' "$code" >> "$ROOT/results/exit-codes.txt"
  test "$code" -eq 0 || exit "$code"
  cargo test --locked -p autosar-config-core --features official-oracles --test official \
    --no-fail-fast 2>&1 | tee "$ROOT/results/C1-official.log" || code=$?
  printf 'C1-official %s\n' "$code" >> "$ROOT/results/exit-codes.txt"
  if [ "$code" -ne 0 ]; then status=1; fi
  check_requirement() {
    local label="$1" code=0
    shift
    uv run --locked python -m autosar_tooling requirements "$@" \
      2>&1 | tee "$ROOT/results/C1-$label.log" || code=$?
    printf 'C1-%s %s\n' "$label" "$code" >> "$ROOT/results/exit-codes.txt"
    if [ "$code" -ne 0 ]; then status=1; fi
  }
  for catalog in core/resources/requirements/*.json; do
    document=$(basename "$catalog" .json)
    check_requirement "$document" "$document"
  done
  for document in TR_Methodology TPS_SystemTemplate TPS_SoftwareComponentTemplate \
    TPS_BSWModuleDescriptionTemplate TPS_ECUConfiguration SWS_StandardTypes SWS_PlatformTypes \
    SWS_CommunicationStackTypes SWS_MemoryMapping SWS_CANStateManager; do
    check_requirement "stale-$document" --stale-only "$document"
  done
  exit "$status"
)
```

Windows：

```powershell
Save-Check C1-fetch uv @('run','--locked','python','-m','autosar_tooling','fetch-official')
if ($LASTEXITCODE -ne 0) { throw "Official archive verification failed" }
Save-Check C1-official cargo @('test','--locked','-p','autosar-config-core','--features',
    'official-oracles','--test','official','--no-fail-fast')
$Failed = $LASTEXITCODE -ne 0
$Requirements = @('run','--locked','python','-m','autosar_tooling','requirements')
foreach ($Catalog in Get-ChildItem core/resources/requirements -Filter '*.json' -File) {
    Save-Check "C1-$($Catalog.BaseName)" uv ($Requirements + @($Catalog.BaseName))
    if ($LASTEXITCODE -ne 0) { $Failed = $true }
}
$StaleOnly = @(
    'TR_Methodology','TPS_SystemTemplate','TPS_SoftwareComponentTemplate',
    'TPS_BSWModuleDescriptionTemplate','TPS_ECUConfiguration','SWS_StandardTypes','SWS_PlatformTypes',
    'SWS_CommunicationStackTypes','SWS_MemoryMapping','SWS_CANStateManager'
)
foreach ($Document in $StaleOnly) {
    Save-Check "C1-stale-$Document" uv ($Requirements + @('--stale-only', $Document))
    if ($LASTEXITCODE -ne 0) { $Failed = $true }
}
if ($Failed) { throw "Official requirement disposition checks failed" }
```

任一条款核算失败即判该环节失败。确认每个文档的退出码为 0，并保留官方档案版次和校验输出。按[标准基线](../../../standards/README.md)逐个核对受影响模块的独立预期、
义务及范围，实际读取引用的 SWS、ECUC 和通用 BSW 原文及涉及的 PDF 图，记录编号、页码、配置、实际消费者与结论。机械 oracle
验证 XSD、模块定义及官方样例等具体性质。人工条款核对单独记录。现有模块规范结论缺依据时退回对应整改 Issue；资料取得归 #57 相关 owner。

安装[固定 Cppcheck](../generated-c.md)后，在四个已生成项目上执行正常 `c-check`。以下输出目录此前未使用。

Linux：

```bash
export CPPCHECK=/absolute/path/to/cppcheck
for sample in standard-ecu user-application component-pipeline no-diagnostic; do
  uv run --locked python -m autosar_tooling c-check --target linux-x64-controlled-v1 \
    --project "$AUTOSAR_C_ANALYSIS_SAMPLES/$sample" --output-directory "$ROOT/results/C1-$sample" \
    2>&1 | tee "$ROOT/results/C1-$sample.log"
  printf '%s %s\n' "$sample" "$?" >> "$ROOT/results/exit-codes.txt"
done
```

Windows：

```powershell
$env:CPPCHECK = 'C:/path/to/cppcheck.exe'
foreach ($Sample in 'standard-ecu', 'user-application', 'component-pipeline', 'no-diagnostic') {
    Save-Check "C1-$Sample" uv @('run','--locked','python','-m','autosar_tooling','c-check',
        '--target','windows-x64-controlled-v1','--project',"$env:AUTOSAR_C_ANALYSIS_SAMPLES/$Sample",
        '--output-directory',"$Root/results/C1-$Sample")
}
```

保留每个样本的 `summary.json` 和两个程序的原始诊断。逐样本核实闭包、工具和解析无错误，检查项目验收报告与原始诊断是否相符。
已有范围内诊断和 `passed=false` 照实登记，新增未处置诊断、分析漏收、工具失败分别按其证据交给改动模块、#55 门禁 owner 或分析工具 owner。
MISRA 合规范围和剩余人工按 ADR 0027／0030 记入对应 Issue。

## C2 实际 PR 和 CI 保护

先看 [#55 的交付 PR](https://github.com/g1331/autsaro/pull/496) 及其真实 PR 故障／恢复记录；再核对冻结版本的正常代码 PR 记录。
先从选定的代码 PR 取 head SHA。把以下大写占位符换成实际值，两平台均可使用 GitHub CLI。

```text
gh pr view PR_NUMBER --repo g1331/autsaro --json headRefOid,headRefName,baseRefName,url
gh run list --repo g1331/autsaro --workflow checks.yml --event pull_request --commit PR_HEAD_SHA --limit 20
gh run view PR_RUN_ID --repo g1331/autsaro --json headSha,event,attempt,jobs,url
gh run view PR_RUN_ID --repo g1331/autsaro --log
gh run download PR_RUN_ID --repo g1331/autsaro --pattern "c-analysis-*" --dir ./pr-ci-evidence
gh run list --repo g1331/autsaro --workflow checks.yml --branch master --event push --commit FROZEN_MASTER_SHA
gh run view MASTER_RUN_ID --repo g1331/autsaro --json headSha,event,attempt,jobs,url
gh api repos/g1331/autsaro/branches/master/protection/required_status_checks
gh api repos/g1331/autsaro/rules/branches/master
```

1. 核对 PR run 的 `event=pull_request`、`headSha=PR_HEAD_SHA` 和所选 attempt。当前侧 `c-analysis-inputs-*`
   的 `c-analysis-generation.json` 中 `tree` 与 checkout 作业日志给出实际测试合并提交。读取该提交的两个父节点：

   ```text
   git fetch origin TESTED_MERGE_SHA
   git rev-list --parents -n 1 TESTED_MERGE_SHA
   ```

   第一父提交须等于门禁回执中的基准树，第二父提交须等于当前 PR head。当前侧分析取测试合并树。
   历史测试合并提交无法取回时，须从该 attempt 的 checkout 日志和工件确认这三个身份。当前 `refs/pull/*/merge`
   可能已更新，记录缺证时保留该缺口。冻结 master 的 push 运行独立记录和判定。
2. 双目标各四样本，逐一核对 PR 与基准侧每个程序的分析收据，以及八份 `gate.json`。保留真实的结论、比较范围、样本身份、工具与基准来源；
   已有缓存命中须满足[当前身份与完整性条件](../generated-c.md)。
3. 查看新增未处置诊断、坏工具／漏扫、必需任务取消或未执行的实际失败或相应独立反例。结合真实分支保护验收确认唯一必需
   `CI required` 阻断有缺陷的正常 PR，恢复后以实际成功作业重新取得资格。既有证据过期或接线变更时由 #55 owner 补受控演练。
4. 记录当前通过的正常代码运行以及失败的对应工件。纯文档 PR 的策略性跳过按工作流记录，不用来证明代码分析已经执行。

## C3 性能、缺口与责任

从 #461 和基准 PR 的 jobs／steps 取可比的首次尝试时长。比较同类代码与文档 PR 的完整墙钟、执行范围、取消或失败以及 runner 消耗。
覆盖、失败判据和平台一致时按 #461 的门槛判断。持续复发按 ADR 0031 返回原 Issue，后续一次成功仍保留原失败。

| 检查 | 首选归属 |
| --- | --- |
| A1/A3 编辑、保存、生成、兼容／完整性 | 工作台或交付责任人，查 #52 对应交付 PR；发现新缺陷时附精确输入与原始字节 |
| A2 模块成功／拒绝／恢复 | 具体 BSW／RTE 模块整改 Issue；OS／PE／ELF 查 #50、#189–#191；Windows kernel patch／Job 关闭对应 #412／#414 |
| A2 历史正向 BCrypt | #49／PR #124 的 R7 移交；当前 SID 0x27 拒绝回 Dcm 责任人 |
| B1 平台独立接收 | #56 独立接收者；包构建与兼容缺陷交实际交付 owner，查 #52，历史 Windows 义务见 D-24 |
| B2/B3 GUI 与真实 IPC | 工作台／Tauri 责任人，场景接线查 #91，已有时序失败按原 Issue |
| B4 原生安装与发行 | #63，由发布／工作台责任人按实际介质核对 |
| C1 原文和源码分析 | 对应模块和工具责任人；资料 #57、分析门 #55、剩余合规责任 #58／#482 |
| C2/C3 门禁与效率 | #55 门禁 owner、#461 效率 owner；其余按具体失败归属 |

## C4 状态结算与硬件事项

每行都应产生有平台、提交、执行人、输出证据和 owner 的结论。正式运行全数满足时，由独立接收者把验证发到 #56，
由项目负责人更新 #30、关联规格和当前计划状态。归档标签中的 BMad 文档是历史依据，现行事项用 GitHub Issue／Milestone
跟踪（见[路线图](../../../roadmap/README.md)）。#30 的实际出口完成后才解除 R11 消费者进入限制。

根据 [ADR 0022](../../../adr/0022-reference-ecu-before-mcu.md) 与[参考工程要求](../../../roadmap/reference-ecu.md)，
只将选定实机才能观察的属性交给 R10。下表在正式验收时附具体板卡／控制器、工具链、选用 MCAL／端口和责任 Issue。
门槛数值采用目标规格中已核定的值，缺失时由目标 owner 定义后再执行。

| 实机剩余属性 | 真实目标上的通过证据 |
| --- | --- |
| 物理 CAN、诊断及网络休眠／唤醒 | 指定板卡与真实对端，抓取应用信号和物理诊断帧，观察错误注入后的 bus-off／恢复与所选网络低功耗、唤醒状态 |
| 真实掉电后的目标 Flash/NvM | 板级保存后切断电源，恢复读取预期故障状态，再验证清除、损坏或写入中断后的行为 |
| 真实中断、调度、资源及复位 | 记录 MCU 的 IRQ 优先级、抖动、最坏延迟、栈／资源峰值；在目标异常和复位后核对恢复，逐项比对目标规格 |
| 板级构建与启动 | 按固定目标工具／驱动构建、链接、刷写；保存程序摘要、启动日志、段与布局及设备上的独立行为结果 |

主机可以验证的行为、原文、CI、Windows 独立接收和所选安装层分别留在各自检查项及责任 Issue。
