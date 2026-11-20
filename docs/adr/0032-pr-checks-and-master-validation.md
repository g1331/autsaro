# 0032. PR 必需检查与 master 合并后验证

## 决定

采用维护者在 [#461](https://github.com/g1331/autsaro/issues/461#issuecomment-7030805068) 批准的方案 B。
`CI required` 继续作为唯一的 PR 必需检查。master 的每次 push 运行完整 Development 矩阵，
再按输入运行 Extended。项目使用公开仓库的免费标准 GitHub runner。仓库归个人账户所有，
[GitHub merge queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue)
仅适用于组织仓库。master 合并后的矩阵负责发现需要回滚或修复的回归，发布另走发行验证。

| 检查 | PR（按 #578 的改动选择） | master push |
| --- | --- | --- |
| 改动检测、quality／lint、必需汇总 | 执行 | 执行 |
| Python 单测、进程测试、linter、打包 | Linux 3.12／3.14 与 Windows 3.14 | 同左 |
| core、UI 与桌面构建、Linux native | 相关输入触发，保留现有构建、单测、Clippy、语料重放与原生测试 | 全量 |
| Linux／Windows 原生交付 | 相关输入触发，验证生成样本的封存构建、CMake 和离线接收 | 全量 |
| C 分析 | 按 [#578](https://github.com/g1331/autsaro/pull/578) 选择，执行双平台八样本回执与比较门 | 全量 |
| Windows native、Linux／Windows native-os、Linux sanitizers | 转到 master | 所有现有分片、断言、期限和取证 |
| 原生桌面 GUI／IPC 场景 | 转到 master | 双平台构建与全部场景分片 |
| Extended | 自动任务转到 master，手动全量入口可在功能分支触发 | 保留 master 的按输入选择及原有矩阵和运行时长 |

当 #578 选择 C 分析时，保留当前树与第一父树的生成、原始扫描、逐样本 gate、行为检查、
SARIF、八份摘要与八份 gate 回执，继续在本 PR 的运行中上传重绑工件。维护者仍从分析输入
所在 PR 的实际 checkout 和对应目标树取工件，按[源码验收流程](../development/testing/source-acceptance.md)
核对及更新记录。普通文档仍走快速路径。合并后的工件依照原有身份匹配规则用于基准复用。

## 时间依据

[#461 第四轮实测](https://github.com/g1331/autsaro/issues/461#issuecomment-7026664805)的
[#602 首轮](https://github.com/g1331/autsaro/actions/runs/44640398218/attempts/1)
成功全量运行，墙钟 27:39，消耗 275.0 runner·分钟，含 Extended 尾段。下表按实际作业
`completed_at − started_at` 计算，逐分片以分钟计，按显示精度取整。L 为 Linux，W 为 Windows；
C gate 的 L／W 表示工件平台。

| 作业 | 分片执行时间 | 位置 |
| --- | --- | --- |
| Detect changed areas、quality、CI required | 0.10／0.82／0.05 | PR 与 master |
| Python L 3.12／L 3.14／W 3.14 | 2.12／2.22／4.72 | PR 与 master |
| core、桌面构建 L／W、native L | 7.15；6.87／7.32；6.57 | PR 与 master |
| native-delivery L、W builds／receiver | 2.48；10.33／5.83 | PR 与 master |
| C prepare L／W、behavior L／W | 1.73／2.92；5.28／7.32 | PR 与 master |
| C current generate L／W、baseline generate L／W | 4.12／4.65；3.07／5.75 | PR 与 master |
| C current 配对作业 L、W | 2.85／2.88；10.17／10.98 | PR 与 master |
| C baseline 配对作业 L、W | 3.05／3.63；7.62／10.15 | PR 与 master |
| C gate L 四片、W 四片、c-analysis | 1.13／0.88／2.27／1.53；2.37／2.28／2.45／1.55；0.28 | PR 与 master |
| Windows native protocol／projects／integration／rest | 10.53／9.42／8.30／9.80 | master |
| native-os L、W 0/2／1/2 | 7.40；10.17／7.45 | master |
| sanitizers ASan+UBSan、TSan programs／OS | 8.40；4.70／9.33 | master |
| 桌面场景构建 L／W、L 两片、W 两片 | 4.97／3.47；3.25／4.57；7.95／7.48 | master |
| Extended 选择／汇总 | 0.23／0.08，本例未选择测试矩阵 | master |

Extended 按改动路径使用不同矩阵。[master 44644161394](https://github.com/g1331/autsaro/actions/runs/44644161394/attempts/1)
的接收端 L／W 分别为 5.23／25.07 分钟，Python L 3.13、W 3.12／3.13、macOS
3.12／3.13／3.14 分别为 2.25、2.95／5.48、3.00／2.53／3.28 分钟，macOS UI 为
0.63 分钟。[PR 44625907914](https://github.com/g1331/autsaro/actions/runs/44625907914/attempts/1)
的 official L／W 分别为 4.22／5.68 分钟。fuzz 每目标各用一个 runner，master 运行预算
为每目标 900 秒，另计构建与收尾。手动 Desktop 与 Development 执行相同场景矩阵。

按同一批作业移除 117.5 runner·分钟，估算全量 PR 约 **157.5 runner·分钟**。把同样的选择
套到[较新的 master 运行](https://github.com/g1331/autsaro/actions/runs/44671547709/attempts/1)
得 **126.8 runner·分钟**，表示不同缓存、输入与 runner 下的波动。以 #602 的单作业时间
重放依赖，最长无排队链约 **18.8 分钟**（Windows baseline generate → 扫描 → gate → 两级汇总）。
如果保留作业沿用这次的实际启动和完成时间，C 分析仍在 **27:39** 决定 PR 完成。
据此以 **约 20–28 分钟**作为 #602 类全量代码 PR 在低到中等负载下的估算，拥挤时可能更长。
master 每次 push 也使用共享 runner，PR 需求下降后的净排队收益须实测。

## 延后发现的代价

- Windows native／OS 容易暴露平台专有的进程、栈和时钟行为。Windows OS 在
  [#463](https://github.com/g1331/autsaro/issues/463) 留下过栈故障现场。合并后才出现的
  Windows 行为回归会进入 master，可能影响后续 Windows 验证。本轮保留 Windows 应用与交付构建、
  Python 进程测试，并在 master 每次运行全部受影响套件和原有取证。
- Sanitizer 覆盖常规测试难以发现的交错。TSan 作业先观察到 [#396](https://github.com/g1331/autsaro/issues/396)
  的 HostBatch 输入与 tick 错序，后续用受控交错修复。类似缺陷在合并后首次出现时需尽快恢复
  master 的运行契约。常规原生测试和既有复现器依各自原入口执行。
- GUI／IPC 能揭示真实窗口和加载顺序问题。例如 [#305](https://github.com/g1331/autsaro/issues/305)
  的 reload 场景曾在 master 桌面场景报告失败。移动后的风险集中在原生 transport、
  WebDriver、窗口和进程生命周期，PR 仍先做两平台桌面构建、UI 测试与 IPC 契约检查。
- Extended 的平台、解释器、官方对照和持续 fuzz 信号延至合并后。master 的模糊测试
  保留原 900 秒预算以及覆盖直接输入的路径；已提交的复现语料仍由 core 测试重放。
  公开接口、交付包和多解释器回归可能在其专有环境首次出现，保留功能分支手动全量入口。

## master 失败处理

每次 master push 使用该提交自己的运行。Post-merge checks result 汇总必需、重矩阵及
Extended 结果，失败、取消或应执行而跳过时保存作业摘要并创建 `area:quality-ci` Issue。
同一 run 的后续失败尝试追加记录，摘要和 Issue 均链接提交及具体 attempt。整个运行被取消
或 Issue 写入失败时，从 Actions 的红色／取消运行核对，维护者补记关联 Issue。

维护者从首次失败的 SHA 和日志区分新回归、已跟踪失败与 runner 故障。已知失败把运行链接
归到原 Issue，处理重复通知。新回归由引入它的 PR 作者提交修复，维护者负责追踪到同套件
验证通过。影响交付、破坏主线关键用例或会污染后续验证时，维护者在确定引入提交后立即回滚。
其他确定回归若当天无法验证修复且仍影响 master，则回滚。原始失败与取证留在 Actions。
开 PR 或合并前核对最近完成的 master 汇总，无关失败已有 Issue 跟踪时照常推进，相关或
归属未明的失败先查清。

## 连续运行计数与验收

[ADR 0031](0031-intermittent-failure-exit-policy.md) 的 30 次计数仍覆盖实际运行的 master
合并后作业，包括 #412、#414 所需的 Windows 取证。PR 只统计受影响测试实际执行的运行，
移走的作业在 PR 中缺席不计数。既有合格运行保留，完整取消或未执行的作业不计数；核对
同一 run 的全部 attempts，以同一失败或已纳入范围的取证夹具失败作为清零条件。

本 PR 用选择到全量的首轮补记实际 `CI required` 墙钟和 runner·分钟，附 run 与 head。
从合并起按合并顺序取连续 10 个代码 PR，每个 PR 只取其最终合并 head 的首个普通
Development checks run 的 attempt 1。墙钟为 run 创建至 `CI required` 完成，含排队；
runner·分钟汇总该轮实际执行的必需依赖和汇总作业，分片各计一次。记录选择模式、并发背景、
失败、取消和重跑；取消且无汇总完成时单列为缺测并保留该 PR 的位置，缺测或失败需补证后
才判断出口。另列全量、分析专用及其他快速路径的组成，普通文档继续以 5 分钟为目标。

代码 PR 的既定中位数目标为 **15 分钟以内**，仍按 #461／#519 的 10 次实测判定。
由于全量 C 分析无排队链已有 18.8 分钟的实例，暂以 **25 分钟**作为全量代码 PR 的运行预算。
若 10 次样本的实际组成和 C 分析链显示 15 分钟目标仍不可达，建议以该批数据讨论将全量
目标定为 25 分钟，并保留整体 15 分钟作为进一步缩短 C 分析关键路径的目标。
