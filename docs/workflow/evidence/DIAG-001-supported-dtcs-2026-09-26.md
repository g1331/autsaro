# DIAG-001：主机受支持 DTC 列表证据（2026-09-26）

## 组合与规范边界

基线 `143cfe158be8`，分支 `feature/DIAG-001-supported-dtcs`。CP/FO R24-11、无变体、Windows 主机虚拟 ECU、MinGW GCC。现有单 DTC 配置下增加 `19 0A`，用于发现已配置而当前未故障的监测项。标准参考是 [R24-11 Dcm SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) §7.4.2.5.2、`SWS_Dcm_01645`（0x0A 禁用状态过滤）和 `SWS_Dcm_00828`（非分页响应不添加虚构零记录）。查询官方文档索引取得对应表格和条款；直接下载端本次返回 HTTP 502，本地 PDF 路径见任务卡，独立审查可核对原文。

主机 `Dem_GetSupportedDtc` 返回唯一配置编号及当前状态；Dcm 依据子功能分别执行无状态过滤与原有掩码过滤。因此状态为零仍是支持项，但 `19 01 00`/`19 02 00` 保持零匹配。接口是主机内部 API，不声称标准 Dem_SetDTCFilter/GetNextFilteredDTC ABI。无需新增 ECUC 选项；既有 SID 25 / DcmDspReadDTCInformation / DemClient 和单 DTC 配置控制此能力。标准 BSW 文件结构、MemMap、MISRA、BSWMD 及外部协议栈互操作仍未验证，不升级能力档案的六道门或内部支持等级。

## 可复现输入与独立预期

`core/tests/end_to_end.rs::supported_dtcs_include_zero_status_and_follow_configured_lifecycle` 创建 SupportedA/B 两个独立工程，DTC 分别 `0x123456` 与 `0xABCDEF`；诊断请求/响应 ID `0x700/0x708`，DID `0x1234` 绑定一个 32 位 Tx 信号，监测 Rx 帧 ID `0x456`、DLC 2、超时 50 ms。每个工程依次配置、保存、重开、XSD/语义验证、生成、GCC 编译链接，输入文件在生成后保持字节相同。测试不从生成器获取预期报文；DTC 字节串和状态演进由测试明确给定。

以 DTC `0x123456` 为例，主机逐行协议的实际断言为：

| 场景/输入 | 诊断 CAN 响应 |
| --- | --- |
| 新建状态：`R 1792 3 02190A` | `X 1800 8 07590A7F12345650` |
| 先 `R 1110 2 0100`，再查询 | `X 1800 8 07590A7F12345600` |
| 扩展会话 `85 02` 暂停，时间超过 Rx 超时，再查询 | 仍为 `X 1800 8 07590A7F12345600` |
| `85 01` 恢复，重新 Rx 后超时，再查询 | `X 1800 8 07590A7F1234562F` |
| 故障后退出、同一 NvM 文件重启，再查询 | `X 1800 8 07590A7F1234566D` |
| 重启周期重新收到有效 Rx，再查询 | `X 1800 8 07590A7F1234562C` |
| 扩展会话 `14 FF FF FF` 清除，再查询及再重启查询 | `X 1800 8 07590A7F12345650` |
| 错误长度：`R 1792 4 03190A00` 或缺子功能 `R 1792 2 0119` | `X 1800 4 037F1913` |
| 不支持：`R 1792 3 021903` 或 `R 1792 3 02198A` | `X 1800 4 037F1912` |
| 删除 DTC 配置、保存/重开/重新生成构建后查询 | `X 1800 4 037F1911` |

错误请求后正确请求恢复；初始 0x50 状态下启动前后比较 NvM 全部 64 字节，重复读取及拒绝请求均不改变内容。生命周期测试同时断言零掩码 0x01 返回零数量、0x02 返回空列表，以及状态为零时 `19 02 7F` 返回空列表。SupportedB 完整重复这些场景，响应编号为 `ABCDEF`。

工作台后端 `host::run_diagnostic` 调用的独立 Rust 测试器也加入支持列表、零状态、暂停记录、故障/重启/清除和错误请求检查；事件列表明确显示 0x19/0x0A 验证结果。既有安全 DTC 用例经同一测试器检查默认会话未解锁时仍可读；写操作继续走原权限路径。UI 只更新 DTC 配置说明与现有“验证诊断连接”入口说明，没有新增配置开关。

## 命令与结果

- Cargo/Rust 1.98.1 MSVC、MSYS2 GCC 16.1.0、Node 24.19.0、npm 11.17.0、Python 3.12.9。
- PowerShell 中先设置 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`；路径已核对存在，使用 crate 默认 target 目录。
- `cargo test --manifest-path core/Cargo.toml --test end_to_end supported_dtcs_include_zero_status_and_follow_configured_lifecycle -- --nocapture`：1/1 通过，11.55 秒，包含两套配置的生成 ECU 及移除配置后的运行拒绝。
- 完整 `python scripts/workflow.py verify --scope all`：退出码 0；工作流测试 8/8、npm ci、TypeScript/Vite 构建、核心单元 2/2、端到端 47/47（164.06 秒）、Tauri debug 构建均通过。既有安全 DTC 用例 `security_access_gates_dtc_mutations_without_a_writable_did` 也通过新增测试器检查。Rust 链接仍有既有 LNK4098 默认库冲突警告，未视为静态质量通过。
- `python scripts/workflow.py check`：通过。

## 未验证与审查

没有启动维护者桌面窗口。原生 Tauri 窗口与 IPC 交互、第三方诊断仪/协议栈、真实 CAN/MCU/电气与 ISR 时序、完整 BSW/MISRA 义务未验证。本主机剖面明确拒绝 0x8A，不声明完整 UDS 子功能抑制正响应行为。没有改变持久格式，没有新增多 DTC 或完成深 CAN 诊断阶段的声明。

Agent `diag001_review` 以不继承开发会话的只读新会话审查提交 `e8e49d9a21299a44fc7ed14ec51cfdaf814591af`（基线 `143cfe158be8`）。直接核对本地 R24-11 Dcm PDF 第 117–119 页，除任务中的条款外确认 `SWS_Dcm_01644` 要求响应含状态可用掩码及 DTC/status 记录；确认实现符合本任务所限定行为，主机自有 API 不冒充标准 Dem 接口。

独立重跑新增生命周期用例 1/1（11.24 秒）及 `security_access_gates_dtc_mutations_without_a_writable_did` 1/1（2.59 秒），检查 `git diff --check 143cfe158be8 c124c0abfc17`、`python scripts/workflow.py check` 均通过，审查前后工作区干净。核对配置保存/重开、两套 DTC 字节值、零状态、原掩码行为、读操作无 NvM 副作用、错误恢复与安全读取；未发现阻断问题，同意在既定主机单 DTC 范围内关闭任务并本地集成。

审查者未重复完整 47 项集成或 UI/Tauri 构建，未运行原生 GUI/IPC，也未将本次审查记为完整规范义务通过。主代理复核上述审查引用与实际源码、测试及门禁结果后收口。
