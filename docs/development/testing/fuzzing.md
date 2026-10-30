# 模糊测试

[`fuzz/`](../../../fuzz/) 是标准的 cargo-fuzz 工程（libFuzzer），为直接处理不可信输入的两类代码提供 fuzz target：用户提供的 ARXML 与工程，以及来自总线的 CanTp／Dcm 接收路径。它是独立的 Cargo workspace，有自己的 `Cargo.lock`，不进入产品依赖。

| 目标 | 入口 | 输入 |
| --- | --- | --- |
| `project_import` | `Workspace::open` 或有 `workbench-project.json` 时的 `Workspace::open_project_manifest`，随后执行工作台打开后的投影、集成检查（`build_plan_native` 与 `integration/graph.rs`）和校验 | 多个文件：`名称\0内容\0名称\0内容…`，名称只取可移植的相对路径 |
| `definition_extension` | `DefinitionCatalog::accept_extension`，清单带输入的实际摘要，使定义文件本身被解析 | 一个 ECUC 定义 ARXML |
| `cantp_receive` | 交付的 `CanTp.c` 的公开接口：接收 N-PDU、Tx 确认、main function、发送与取消请求、`CanTp_Init`／`CanTp_Shutdown`，以及 PduR／LSduR 下一步的应答 | 配置前缀（合法域内的定时器、N-SDU 上限、WFTmax、填充字节、周期）加操作序列 |
| `dcm_request` | 交付的 `Dcm.c` 的公开接口：任意切分的接收、指示、main function、带各种重试状态的响应复制、确认、通信模式、会话复位，以及 PduR 与 DID 读取的应答 | 配置前缀（缓冲、P2、S3、DID）加操作序列 |

两个 C 驱动（[`fuzz/c/`](../../../fuzz/c/)）由 `fuzz/build.rs` 用 clang 编译，与 Rust 代码一样带 libFuzzer 覆盖插桩及 AddressSanitizer、UndefinedBehaviorSanitizer（`-fno-sanitize-recover=all`）；两种检测器的运行时来自 cargo-fuzz 链接的 Rust ASan 运行时，不需要 compiler-rt 包。Dcm 使用的 `Rte_Dcm_Type.h`、`Rte_Type.h` 取自内置模板的实际生成结果，与原生 Dcm 测试相同。

## 断言

每个输入先恢复默认配置再读取配置前缀，单独重放与在 fuzz 进程中运行的结果相同。

Rust 目标断言不 panic、不越界。C 驱动的桩按 R24-11 原文检查上下层可依赖的契约，违反时打印驱动源码行并中止：

- CanTp：SF／FF 才开始接收，每个被接受的接收恰有一次指示，`E_OK` 只用于完整的 N-SDU，进行中的接收先以 `E_NOT_OK` 结束再开始新的（SWS_CanTp_00079、00084、00081、00353、00057 表 1）；上层以 `BUFREQ_E_OVFL` 拒绝的 FF 必须得到 FC(OVFLW)，除非被新的 SF／FF 或重启取代，或 N_Ar 届满：已过 N_Ar 个周期且最后一个之前的每个周期 Tx N-PDU 都被占用（00318、00311）；复制不超过宣告长度和上层报告的空闲缓冲（00080、00222、00224）；每个被接受的发送恰有一次确认，忙时拒绝（00090、00204、00205、00123）；Tx N-PDU 确认前不再发送，帧为 8 字节，SF、末个 CF 与 FC 的未用字节等于配置的填充字节（00075、00248、00347、00348）；`CanTp_Init` 不通知地丢弃连接，`CanTp_Shutdown` 后不再调用任何模块（00273、00200）。
- Dcm：超出缓冲的请求不接受，请求从接受到响应确认期间阻塞连接，一个请求至多一个响应（SWS_Dcm_00444、00241、01678、00118）；复制按 SWS_COMTYPE_00027 的重试语义且不超出响应（00350）；0x40–0x7F、0xC0–0xFF 不响应，表外服务回 NRC 0x11，通信一直为 full 时须在 P2ServerMax 内发出，另容许指示所在的一个周期（00024），正响应 SID 为请求 SID+0x40，置位 suppressPosRspMsgIndicationBit 时只允许负响应（00084、00197、01622、00200）；ComM 通知使用连接的通道（01376、01377）。

每个输入结束后检查收敛：上下层改为协作，CanTp 在有限 main function 内由完成或超时结束全部会话（00310、00311、00313、00316、00280），之后新的 SF 接收和 SF 发送都成功；Dcm 结束进行中的请求，S3Server 回到默认会话并转为 inactive（01670、01374、01375），之后 TesterPresent 被接受并得到 `7E 00`（00251）。

## 运行

前置条件为 Linux x86_64、[核心开发依赖](../environment.md)中的 clang、`fuzz/rust-toolchain.toml` 固定的 nightly（rustup 在 `fuzz/` 中首次运行时安装）和 `cargo install cargo-fuzz --version 0.13.2 --locked`。命令在 `fuzz/` 中执行：

```sh
cd fuzz
cargo fuzz list
mkdir -p ../build/fuzz-corpus/cantp_receive
cargo fuzz run cantp_receive ../build/fuzz-corpus/cantp_receive corpus/cantp_receive -- -max_total_time=600
```

第一个语料目录接收新输入，放在 `build/` 下可保持提交的语料不变；省略目录时 cargo-fuzz 直接写入 `corpus/<target>`。`project_import` 的种子是数百 KB 的完整工程，每秒执行几十次，需要更长时间才有意义。

## 崩溃转为回归

libFuzzer 把失败输入写到 `fuzz/artifacts/<target>/crash-<sha1>` 并打印复现命令。处理顺序：

1. `cargo fuzz run <target> artifacts/<target>/crash-…` 复现，`cargo fuzz tmin <target> artifacts/<target>/crash-…` 最小化。
2. 区分产品缺陷与驱动模型错误；产品缺陷按所在模块的契约修复，驱动错误按原文修正检查。
3. 把（最小化后的）输入放入 `fuzz/regressions/<target>/`，文件名保留 libFuzzer 的 `crash-<sha1>`。

提交的 `corpus/` 与 `regressions/` 由普通测试重放：Rust 目标为 `cargo test --locked --manifest-path fuzz/Cargo.toml`（稳定工具链即可），C 驱动为原生套件的 `runtime_contracts::cantp_fuzz_corpus_and_reproducers_replay` 和 `runtime_contracts::dcm_fuzz_corpus_and_reproducers_replay`，它们用 `fuzz/c/replay.c` 以 GCC 编译同一驱动，在 `AUTOSAR_SANITIZE=address,undefined` 的检测器任务中带 ASan／UBSan 运行（见[进程与原生行为](native.md#检测器构建)）。

## 语料维护

C 目标的语料是 fuzz 运行结果按覆盖特征取最小集合，Rust 目标的种子取自仓库自有夹具（`echo-application`、`standard-ecu` 模板工程、`desktop/unknown.arxml`、定义扩展测试中的 ECUC 定义）。不提交官方档案中的字节。更新 C 语料时把新输入与原有语料合并成最小集合再替换：

```sh
cargo fuzz run cantp_receive ../build/fuzz-min ../build/fuzz-corpus/cantp_receive corpus/cantp_receive -- -set_cover_merge=1
```

## CI

必需检查的 `core` 任务运行上述重放测试和 fuzz crate 的 Clippy；原生与检测器任务运行 C 驱动重放。`Extended checks` 的 `fuzz` 任务按路径选择，每个目标一个作业：PR 只在目标直接消费的源码改动时运行 180 秒（`fuzz/`、ARXML 解析、定义、XML 路径与工程模型、CanTp／Dcm 及驱动编译的主机源文件）；master 推送另按导入与规则校验、生成器、内置模板、共享头、主机区、core 构建与构建依赖选择，运行 900 秒。PR 不运行间接输入，避免几乎每个 PR 都占用四个 runner；这些改动合并后由 master 运行覆盖，失败按 [CI](README.md#ci) 的 Extended 规则处理。master 运行积累的语料按覆盖取最小集合后保存在 Actions 缓存，后续运行从中恢复；失败时上传 `fuzz/artifacts/<target>`。

## 限制

- 只在 Linux x86_64 运行，C 驱动使用主机互斥区实现，验证单线程调用序列；并发由原生并发测试与 TSan 任务承担，封存 ECU 的完整 TSan 见 #316。
- 驱动只覆盖所选配置的合法配置域；`Ecu_CanTpReceptionBeforeMain` 属于 ECU 所有者的排序协议，不作为任意输入。
- 未发现失败只表示在已运行的时长和语料内没有触发检查，不代表模块符合性。
