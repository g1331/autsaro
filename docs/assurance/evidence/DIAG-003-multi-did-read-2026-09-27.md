# DIAG-003：多 DID 读取主机证据（2026-09-27）

## 组合、来源与输入

基线 `3410e7cbbd19`，分支 `feature/DIAG-003-multi-did-read`。CP/FO R24-11、无变体、Windows 主机虚拟 ECU；Rust/Cargo 1.98.1 MSVC、MSYS2 MinGW GCC 16.1.0、Node 24.19.0/npm 11.17.0、Python 3.12.9。依据 [官方 R24-11 Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) §7.4.2.6，`SWS_Dcm_00253`、`SWS_Dcm_00438`、`SWS_Dcm_00434`；`DcmDspMaxDidToRead` `[ECUC_Dcm_00638]` 为可选参数，本主机固定配置未设置。响应 256 字节上限及超长返回 NRC 0x14 是本主机传输容量处理，不据此证明完整第三方 Dcm 接口。项目质量门见 `docs/assurance/acceptance-policy.md`。

`core/tests/end_to_end.rs::multiple_dids_keep_request_order_and_skip_unavailable_values` 新建 Tx 帧与一个 32-bit 初值为 42 的信号，配置 DID `0x1234`、请求/响应 CAN ID `0x700/0x708`、S3=5000 ms；保存、重开、XSD/语义验证、生成独立工程并编译，确认源 ARXML 字节未变。生成产物沿用现有 `files.list`/`files.sha256` 闭包；本卡未新增生成文件或 ECUC 项。独立预期从上述固定输入和 UDS/CanTp 报文结构写出，没有从生成器输出提取期望字节。

## 直接运行生成 ECU 的正反向向量

| 场景 | 预期与实际诊断响应（CAN ID 1800，十六进制） |
| --- | --- |
| 默认会话 `22 1234 F186` | `0462F18601`，配置 DID 被会话条件过滤 |
| 扩展会话 `22 1234 F186` | FF `100A621234000000`，FC 后 CF `212AF18603` |
| 扩展会话 `22 F186 1234` | FF `100A62F186031234`，FC 后 CF `210000002A` |
| 扩展会话 `22 F187 F186` | `0462F18603`，不支持 DID 被过滤 |
| 全部不支持 `22 F187 F188` | `037F2231` |
| 字节数不成对 `22 F186 12` | `037F2213` |
| 重复 `22 F186 F186` | `0762F18603F18603` |
| 86 次 `F186`，响应将为 259 字节 | FF 请求获 FC，完整请求后 `037F2214`；下一条 `22 F186` 仍返回 `0462F18603` |

工作台后端 `host::run_diagnostic` 在现有所有诊断配置组合下额外验证默认会话过滤、两个 DID 顺序及完整多帧流控、未知 DID、错误长度和错误后继续处理。`0xF187` 是合法用户 DID 时，测试器改用 `0xF188` 作未知探针；原有该配置的回归测试仍通过。传输错误测试原先用格式合法的四 DID 请求来预期 NRC 0x13，本卡将其负例改为真正不成对的 DID 字节，继续覆盖错误序号、N_Cr 超时及恢复。

## 执行记录与限制

- 定向用例 `cargo test --manifest-path core/Cargo.toml multiple_dids_keep_request_order_and_skip_unavailable_values -- --nocapture`：1/1 通过。
- 既有传输用例的负例修正后，`cargo test --manifest-path core/Cargo.toml diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers -- --nocapture`：1/1 通过。
- 最终 `python scripts/workflow.py verify --scope all`：`git diff --check`、工作流测试 8/8、`npm ci`、UI TypeScript/Vite 构建、核心单元 2/2、端到端 49/49、Tauri 构建通过；`python scripts/workflow.py check` 通过。Cargo 链接仍报告既有 `LNK4098` 默认库冲突警告，未导致构建失败。
- 第一次完整门禁因沙箱无权读取用户目录中的 npm 全局缓存而停在 `npm ci`；将 `npm_config_cache` 指向仓库已忽略的 `core/target/npm-cache` 后继续。第二次发现上述旧传输用例预期与新语义冲突，修正负例输入后第三次完整门禁通过。没有放宽检查或吞掉失败。
- 原生 Tauri 桌面窗口/IPC 未在隔离桌面操作；本轮 UI 仅更新测试器说明文字，TypeScript/Tauri 构建与共用后端测试器已验证。完整 AUTOSAR/ISO 符合性、第三方互操作、BSW 静态义务、目标 MCU 与实机未验证；能力六门及支持等级保持原状态。

## 独立复核

2026-09-27，独立只读 Agent `diag003_review` 检查 `runtime/src/Dcm.c`、端到端用例、任务与证据，并通过官方 R24-11 Dcm SWS 的检索索引核对 `SWS_Dcm_00253`、`SWS_Dcm_00438`、`SWS_Dcm_00434` 及 `ECUC_Dcm_00638`；官方 PDF 页面抓取超时。审查未发现阻断：筛选、顺序、重复、错误响应和缓冲上限与本卡限定行为一致。审查者没有重跑测试，也未操作原生窗口；完整门禁由开发侧在最终代码修改后运行。审查指出本卡没有新增恰好 256 字节的响应向量与多 DID 回调失败向量，这两项保持未覆盖，不升级六道证据门或标准支持声明。
