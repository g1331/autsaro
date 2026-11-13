# 日常修改的最小有效反馈

先准备锁定依赖与[原生工具](../environment.md)，核对 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON`；检查共享类型／段符号还需 `AUTOSAR_NM`。缺工具先运行 `uv run --locked python -m autosar_tooling doctor --role native --target linux-x64-controlled-v1`，再检查所选测试的额外工具。下表是 Linux、GCC 13、已准备依赖的同机墙钟实测；首次／重复指该命令第一次及紧接着运行，未清操作系统页缓存。测试 profile 优化依赖与 core（见[基础检查](README.md#基础检查)），首次编译比不优化时慢：`CARGO_BUILD_JOBS=2` 下无 Cargo 构建产物时 `it` 编译约 252 秒，随后首次启用 native feature 另需约 135 秒，不能把准备成本算成行为检查耗时。

## 按改动选择常规检查

```sh
uv run --locked python -m autosar_tooling verify --scope changed --base origin/master
```

加 `--list` 只查看命令，不创建产物。入口比较给定分支与 HEAD 的 merge-base，并读取当前工作区的提交、暂存、未暂存及未跟踪文件。已识别改名保留原路径；未暂存删除加未跟踪文件可能是改名，按未知输入处理。空清单、未知路径和改名选择完整检查。PR 与本地入口调用同一 `ci_selection.select_changes`，CI 的 push 仍保留完整非 GUI 矩阵。

| 改动 | 常规本地检查 |
| --- | --- |
| 普通读者文档 | diff、全范围 quality、条款清单 |
| 仅 UI 与普通文档 | 上述检查、Python 测试与 lint、UI lint／test／build、桌面 IPC／test／build／Clippy |
| 仅独立分析驱动或标准评估记录 | 上述源码检查、Python 测试与 lint、生成四样本、样本 C 语法和 clang-tidy |
| core／runtime／共享工具／混合／未知输入 | 完整常规组，含全部 core 与 native 测试、Clippy、应用检查和四样本质量检查 |

workflow 改动追加 actionlint／zizmor；依赖清单改动追加原依赖检查。安全检查的完整矩阵继续在 CI 的 quality job 中运行。本地只运行当前主机的命令，双平台、解释器和 GUI 矩阵由必需 CI 验证。

changed 模式使用一个 `native-tests` feature 组合，正常 Cargo 入口枚举全部库、it、native 与 doctests，保留 `--no-fail-fast`。现有 feature 仅增加测试辅助及额外用例；原手选 scope 保持。每次调用新建 `build/local-verification-*`，it 先生成四样本，native 与 clang-tidy 消费同一批字节；跨调用重新生成。成功或空失败时清理目录，有实际失败输入时打印并保留目录，查阅完成后删除。未缓存测试或分析通过结果。

这些是常规检查。C 源码／生成器仍按[生成 C 检查](generated-c.md)执行受影响 profile 的 c-check；OS／进程／独占区按[原生测试](native.md)追加相关 OS、边界与检测器入口；GUI／IPC 按[桌面场景](desktop.md)验收。保持各专题的工具身份、期限、断言及原始失败。

## 模块内的第一轮反馈

以下命令均以 `cargo test --locked -p autosar-config-core` 开头，再接表中参数；测试输出须确认实际运行目标用例，不能把零匹配当作通过。

| 真实修改与落点 | 最快有意义的参数 | 首次／重复 | 升级条件 |
| --- | --- | --- | --- |
| RTE 生成配置：`core/src/integration/rte_configuration.rs`、`contracts.rs`、ECUC 模板（#213） | `--test it -- integration::rte::rte_generation_options_are_explicit_and_unsupported_values_are_refused` | 2.285／1.705 秒 | 输出改变：受影响生成／原生消费者及 c-check；共同计划、命名或类型改变：对应完整模块 |
| Det 状态：`runtime/bsw/src/Det.c`、`runtime/host/src/Det_Host.c`（#205） | `--features native-tests --test native -- runtime_contracts::det_standard_consumer_executes_runtime_and_halt_contracts` | 1.023／0.803 秒 | 并发／callout 改变：Det 并发用例与[检测器](native.md#检测器构建)；配置接缝改变：生成消费者；C 改动仍须受影响样本 c-check |
| 共享类型：`runtime/bsw/include/` 的 Platform／Std／ComStack 类型、`core/src/integration/rte_types.rs`（#213） | `--features native-tests --test native -- integration::generated_c::generated_headers_compile_and_link_independent_scalar_and_void_server_signatures` | 5.201／5.145 秒 | ABI、宽度、include 改变：全部直接消费者、全部受影响样本 c-check；双目标完整矩阵交必需 CI |

它们分别提供配置对象的接受／拒绝、独立 C 状态向量，以及共享头的编译／链接／执行与改名变体反馈。按实际模块替换 filter；它们是修改过程中的第一轮反馈，交付前仍完成该改动要求的受影响检查。UI 使用现有 Vitest filter／watch、lint 和 build；涉及原生异步操作时执行[隔离桌面场景](desktop.md)，浏览器检查不能替代 IPC。

本地只运行受影响套件，完整双平台矩阵交给必需 CI；改变共享测试辅助或进程 owner 时，扩大到它的全部直接消费者。互不依赖的检查可在资源允许时并行运行；同一 Cargo target 的构建会等待锁，应先完成共同构建，检查之间也不能同时改写相同输入或输出。失败先读源对象、阶段、命令、原始 exit 和输出末尾；需要现场时使用既有 scratch／artifact，不通过重试或放宽期限隐藏问题。

实测与后续可比结果保存在 [#53](https://github.com/g1331/autsaro/issues/53) 对应 PR；这里的数值用于选择反馈层，不是耗时上限。没有证明风险覆盖相同的测试／步骤不删除。
