# 项目源码验收与冻结证据

[ADR 0026](../../adr/0026-project-source-acceptance-preserves-raw-analysis.md) 将原始分析与项目验收分开。`c-check` 在原 seal 和语义输入复核后附加 `projectAcceptance`，不修改 `passed`、`error`、程序退出码、诊断、原始 221 项状态、`completeCompliance` 或比较结果。

## 结果与严格入口

- `accepted`：当前输入下，所有诊断位置都有有效处置，必要人工与正式义务均已完成。
- `blocked`：分析可用，但有未知诊断、未绑定或过期上下文、未评估或未完成的义务。
- `error`：工具、输入、解析或判定不能完整执行。正常 CI 的完整性检查也拒绝此状态。

`objects` 给出具体对象和阻塞理由，`occurrences` 回接原诊断索引、位置序号、规则、程序及处置来源。`checkersReport` 是工具信息，不为它制造偏离。SDK 来源完整只证明责任可识别，不证明其集成义务已完成；采用代码同样不自动放行。

严格消费者使用含摘要、已分析 `project/` 及程序目录的完整结果，不接受仅有摘要或缺失、重复、额外身份：

```bash
uv run --locked python -m autosar_tooling c-acceptance \
  --analysis build/c-analysis-results/standard-ecu \
  --expected linux-x64-controlled-v1=standard-ecu \
  --upstream prepare=success --upstream analysis=success
```

完整矩阵在各自目标主机分别给出四个 `--expected` 与完整结果；Windows 工件不能交给 Linux 验证其实际 SDK 路径。上游值取实际任务结果，不能在失败时写死为 success。消费者重新读取当前 SDK、runner、分析驱动和完整源码，重算输入身份与验收，并精确核对存储回执；修改旧摘要的 SDK、seal、参数或当前 producer，不能重放旧 `accepted`。原始 `passed=false` 与诊断退出码 1 可以和有效项目验收并存。

跨平台离线核对封存输入时，从同一 head 按目标 Git 换行规则导出独立副本，例如 Windows 使用 `git -c core.autocrlf=true checkout-index --all --prefix=<新目录>/`，同时从该副本的 `tools/python/src` 加载解析器。逐项核对 producer、解析器与评估正文摘要后重算身份；Windows 的 SDK 和实际运行证据仍由目标主机正常验收入口提供。

PR 工作流的 `head_sha` 标识 PR 分支，实际检出可能是测试合并提交。消费工件前先从
checkout 日志核对实际提交及父提交，按该树导出目标换行副本；本地分支缺少其中的目标
分支增量时，先同步并核对完整 tree，再核验 producer。扫描来源与当前评估身份分别记录。

手动 workflow 的严格步骤直接调用同一 CLI；自动 PR 仍只验证完整执行，直到 #55 的剩余义务及真实保护验证完成。测试里的完整合成正例验证可执行契约，不是实际工程的合法绿色出口。

`docs/standards/` 的评估 Markdown 与固定指南索引是实际分析输入，单独修改时选择 Python、两平台准备和全部 C 分析，保持原始状态及严格汇总。`rte/generated-retention/` 还由 Rust native 消费，继续选择完整原生检查；adoption 上下文块由分析消费，沿分析入口验证。五个独立分析驱动使用相同选择；共享 lexer／retention、混合改动、改名或未知输入仍完整运行。标准记录不单独选择桌面场景，普通读者指南沿文档快路径。

固定 221 项的类别独立锁定，不能靠同步修改索引和摘要重新定义；已有处置不授权改变规范类别。

## 当前输入身份

`inputIdentity` 绑定已分析副本，而非目录标签：完整 plan 与 ARXML 来源、独立应用快照字节、producer 当前字节及原来源映射、交付文件集合与字节、补丁和最终 kernel、target、entry、各程序 TU，以及实际编译器／SDK／宏／工具语义输入。标准表正文和解析器也参与绑定。

`standard-ecu` 与 `user-application` 的 ARXML 可以相同，应用字节仍分别绑定。重命名目录不改变身份；应用、配置、producer、程序／TU、SDK 或评估正文变化都会使旧上下文失效。原分析副本中的 kernel 已应用补丁，因此复用现有最终 kernel 身份检查，不以补丁前 seal 再验证补丁后字节。

修改 C 分析准备或身份拒绝路径时，除 `tests/python` 开发测试外，还须设置固定版本
`CPPCHECK` 并运行以下命令。根目录发现测试不包含这组 `ActualCAnalysisTests`；单独
记录其实际执行数量、跳过项与原因。

```bash
uv run --locked python -B -m unittest discover -s tests/python/native -p test_c_analysis.py
```

这些身份是证据适用范围，不是诊断 baseline 或缓存。不得采集当前 hash 后自动追认成旧批准。初次绑定引用原冻结工件与收据；旧材料不完整时保持未绑定，或对当前范围完成明确的重新评估。

动态构造带摘要的原件、补丁或封存夹具时，用同一份编码后的字节写文件并计算身份，
采用 `write_bytes` 或显式指定换行；默认文本写入在 Windows 会转换 LF 为 CRLF。
仓库的 `.gitattributes` 只约束检出文件，临时夹具须自行固定字节，并在 Linux 上模拟
Windows 默认 CRLF 写入验证身份及正常消费路径。

## 原标准篇中的上下文

上下文写在其原标准篇的 JSON 注释块中，起止行分别为 `<!-- source-assessment-context-begin -->` 和 `<!-- source-assessment-context-end -->`。不另建批准文件，也不复制原 159 键或人工表。格式示意：

```json
{
  "id": "reviewed-context-id",
  "evidence": {"head": "<40 位提交摘要>", "receipt": "<冻结评估收据 URL>"},
  "cases": [{"caseId": "standard-ecu", "target": "linux-x64-controlled-v1", "inputIdentity": "<64 位输入摘要>"}],
  "records": {"kind": "generated-retention", "cohortSha256": "<原表固定 cohort>"},
  "formalStatus": "pending"
}
```

基本记录组为 `generated-retention`、`manual-assessment`、`checker-limitations` 和
`checker-obligations`。后三组的 `records` 只包含 `kind`。每组必须有自己的适用上下文；
一个组的批准不授权其他组。同组不能重复绑定同一输入，跨组的样本标签必须一致。
`approved` 只能在对应实际评估与正式义务完成后记录；`pending` 永不放行。

自有源码的局部消费者另接受 `records={"kind":"owned-source"}`。它复用原标准篇与
`c_standard` 的 MR12 双向关联，核对原实体、已登记 shape、当前原件及 ownership 字节，
并保留表中的目标、程序和 ND 样本限制。主机地址使用原有八个表达式实例；MemMap
R20.1 逐包含验证既有 START／STOP 协议，R20.5 只消费原表精确登记的 MR12 位置。
未登记的内部属性／状态撤销保留阻塞，完整类别实例解析由自有源码消费任务接续。
C99 下的 R17.3 适用性回接 OS 原篇。匹配结果逐位置返回 `owned_record_matched`，
缺少本组当前绑定时返回 `owned_source_context_unbound_or_stale`。
共用 MR12 解析器返回的原 159 键目录记录始终交给 `generated-retention` 消费；
`owned-source` 的批准也保持该组的 pending、过期、shape 或 owner 拒绝。
消费来源取当前可追溯的 runtime 原件及明确归属的生成／配置产物；应用快照与内核实例
由原任务继续核定。允许的程序为 host-batch／probe，样本为现有四个 profile。

该组当前只消费上述已实现的精确关联。Required 风险记录继续返回
`formal_deviation_pending`，即使上下文为 `approved` 也保留该义务；Mandatory 保留项
返回 `mandatory_deviation_forbidden`。其余模块误报、采用内核与 SDK 位置沿原有阻塞。
实际输入交集绑定在[原自有源码标准篇](../../standards/bsw-general/public-api-os.md#自有源码记录的当前输入消费)，
只接纳所列输入下已实现的 Advisory 记录与语言不适用位置；剩余模块消费与
Required 正式材料继续由自有源码消费任务接续。

固定 FreeRTOS 使用 `records={"kind":"freertos-integration"}`，原件集成义务和精确
补丁表见 [采用篇](../../standards/os/adoption.md#当前集成义务与消费)。本组沿用同一
上下文格式；其 case 的 `inputIdentity` 使用
`c_acceptance.freertos_identity(summary, c_acceptance.input_identity(summary, project))`，
额外绑定两份 manifest 和含 kernel 位置的完整诊断集合。重放有序补丁验证当前来源后，
原件按本组集成证据消费；补丁复用共同 MR12 匹配器及原有精确裁决，SDK 仍独立阻塞。
缺少当前绑定或 pending 时，返回本组明确的阻塞原因。R15.5 继续由 GRP 消费。

内核行归属按实际应用的有序补丁 hunk 传递：新增和替换行归项目，保留行携带原身份
随插删移动。hunk 的 offset 取 Git 实际应用回执，并核对重建文本与最终文件一致；
重复文本、删除后在别处加入以及恢复原文，均按这些位置身份判断。分析来源、集成验收
与内核 GRP 共用该结果。整条补丁链完成后先核验最终字节身份，再报告暂存的行归属
错误；追加空行或 LF／CRLF 变化仍按 `Kernel identity mismatch` 拒绝。

人工表及补充记录中的 `retained` 保留真实违反及理由，经本组当前绑定和有效批准参与项目验收。Required 保留项的批准材料须完成 AMD2.5 修订后 §6.2.2 引用的 MISRA Compliance:2020 第 4 节正式义务；AMD2.4 已撤换主文旧第 5 节。Advisory 使用 §6.2.3 的记录安排。`not_assessed` 和缺失记录持续阻塞，Mandatory 的 `retained` 在记录入口拒绝。

输入摘要排除上下文块自身，避免 hash 自引用；该篇其他正文仍逐字节绑定。原人工汇总的全文字节身份保持原义，更新人工篇后仍须正常重扫，不能改旧摘要追认。

## 检查器缺口的补充人工记录

原 66 项人工表及 155 项 checker-covered `not_assessed` 输出不变。现有标准篇可另用 `misra-checker-assessment-begin`／`misra-checker-assessment-end` 注释块记录固定 11 项已知检查器缺口，列与原人工表相同：编号、类别、评估范围、状态、结论。只接受已列缺口的准确编号和类别，缺项为 `not_assessed`，未知、重复或 Mandatory 的保留状态拒绝解析。

此输入格式不意味着 11 项已完成，原有论据仍须核对当前上下文。不得把有 checker、零告警、历史分类或风险接受写成当前人工义务满足。整体 MISRA 符合性继续独立评估。

固定 11 项以外的 144 项由 `checker-obligations` 独立消费。
记录仍放在原标准主题篇，用 `misra-checker-obligations-begin`／
`misra-checker-obligations-end` 注释块及相同五列（编号、类别、评估范围、状态、结论）。
只接受固定 155 项中扣除原 11 项后的准确编号、原类别，以及
`satisfied`／`not_applicable`／`not_assessed`；此组没有批准保留违反的入口。
未知编号、原 66 项或固定 11 项、重复、错误类别与非法状态在解析时拒绝。

缺少记录按编号返回 `checker-obligation_not_assessed`，责任为 #443；
已有记录的阻塞对象还包含实际文件、行号和结论。
满足或不适用记录须经本组当前输入绑定及 approved，pending、缺少／过期上下文继续阻塞。
选择的全部主题篇正文与解析器参加 `assessmentInputs.checkerObligations` 和普通输入身份；
上下文块自身仍按原算法排除。零告警输入同样逐项检查，其他三组的批准保持各自范围。

R7.2／R7.3／R12.3 的[局部证据和当前缺口](../../standards/types/language-constructs.md#三项局部证据的当前消费)
已进入本组，保留未评估状态；R1.4／R17.3 的 C99 语言适用性也独立记录，
其余项缺少能力／适用性和人工结算时逐项阻塞。
实际当前证据结算仍由 #443 承担，SDK 实例沿 #444、66 项沿 #445。
