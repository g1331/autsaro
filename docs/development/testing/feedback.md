# 日常修改的最小有效反馈

先准备锁定依赖与[原生工具](../environment.md)，核对 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON`；检查共享类型／段符号还需 `AUTOSAR_NM`。缺工具先运行 `uv run --locked python -m autosar_tooling doctor --role native --target linux-x64-controlled-v1`，再检查所选测试的额外工具。下表是 Linux、GCC 13、已准备依赖的同机墙钟实测；首次／重复指该命令第一次及紧接着运行，未清操作系统页缓存。测试 profile 优化依赖与 core（见[基础检查](README.md#基础检查)），首次编译比不优化时慢：`CARGO_BUILD_JOBS=2` 下无 Cargo 构建产物时 `it` 编译约 252 秒，随后首次启用 native feature 另需约 135 秒，不能把准备成本算成行为检查耗时。

以下命令均以 `cargo test --locked -p autosar-config-core` 开头，再接表中参数；测试输出须确认实际运行目标用例，不能把零匹配当作通过。

| 真实修改与落点 | 最快有意义的参数 | 首次／重复 | 升级条件 |
| --- | --- | --- | --- |
| RTE 生成配置：`core/src/integration/rte_configuration.rs`、`contracts.rs`、ECUC 模板（#213） | `--test it -- integration::rte::rte_generation_options_are_explicit_and_unsupported_values_are_refused` | 2.285／1.705 秒 | 输出改变：受影响生成／原生消费者及 c-check；共同计划、命名或类型改变：对应完整模块 |
| Det 状态：`runtime/bsw/src/Det.c`、`runtime/host/src/Det_Host.c`（#205） | `--features native-tests --test native -- runtime_contracts::det_standard_consumer_executes_runtime_and_halt_contracts` | 1.023／0.803 秒 | 并发／callout 改变：Det 并发用例与[检测器](native.md#检测器构建)；配置接缝改变：生成消费者；C 改动仍须受影响样本 c-check |
| 共享类型：`runtime/bsw/include/` 的 Platform／Std／ComStack 类型、`core/src/integration/rte_types.rs`（#213） | `--features native-tests --test native -- integration::generated_c::generated_headers_compile_and_link_independent_scalar_and_void_server_signatures` | 5.201／5.145 秒 | ABI、宽度、include 改变：全部直接消费者、全部受影响样本 c-check；双目标完整矩阵交必需 CI |

它们分别提供配置对象的接受／拒绝、独立 C 状态向量，以及共享头的编译／链接／执行与改名变体反馈。按实际模块替换 filter；它们是修改过程中的第一轮反馈，交付前仍完成该改动要求的受影响检查。UI 使用现有 Vitest filter／watch、lint 和 build；涉及原生异步操作时执行[隔离桌面场景](desktop.md)，浏览器检查不能替代 IPC。

本地只运行受影响套件，完整双平台矩阵交给必需 CI；改变共享测试辅助或进程 owner 时，扩大到它的全部直接消费者。互不依赖的检查可在资源允许时并行运行；同一 Cargo target 的构建会等待锁，应先完成共同构建，检查之间也不能同时改写相同输入或输出。失败先读源对象、阶段、命令、原始 exit 和输出末尾；需要现场时使用既有 scratch／artifact，不通过重试或放宽期限隐藏问题。

实测与后续可比结果保存在 [#53](https://github.com/g1331/autsaro/issues/53) 对应 PR；这里的数值用于选择反馈层，不是耗时上限。没有证明风险覆盖相同的测试／步骤不删除。
