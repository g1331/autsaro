# AUDIT-002：基础主机 DoCAN 连接证据（2026-09-26）

## 被审组合、输入与基线

- 任务 `AUDIT-002`，能力 `HOST-DOCAN-01`。基线 `bdb9881`，本分支的测试与档案提交 `66ef3d9`。CP/FO R24-11，Windows 虚拟 ECU，MSYS2 GCC 16.1.0；Rust/Cargo 1.98.1、Node 24.19.0、npm 11.17.0、Python 3.12.9。未选配置变体。
- 可重建输入在 `core/tests/end_to_end.rs` 的 `configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did`：新建 `Diag.arxml`，一条 `0x321`、DLC 8、周期 1000 ms 的 Tx 帧，两个有序 32-bit LSB0 信号；诊断请求/响应 ID 为 `0x700/0x708`，S3=5000 ms，N_Bs=N_Cr=200 ms，DID=`0x1234`。未启用 DTC、`0x2E`、`0x31` 或 `0x27`。测试临时目录由 `Scratch` 清理，复核者以测试名重建输入。
- 本地官方 XSD 包 `docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip`，SHA-256 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`，含 `AUTOSAR_00053.xsd`；ECUC MOD 包 `docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip`，SHA-256 `df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`。官方资料被 Git 忽略，未入库。输入为工具新建的主机剖面，未使用外部可编辑诊断项目。

## 依据与适用边界

- **规范结构：**R24-11 ECUC MOD 中的 CanTp Rx/Tx N-SDU、N-PDU/FC 引用，以及 Dcm DSL/DSD/DSP DID 相关容器用于核对生成输入；`AUTOSAR_00053.xsd` 只校验 XML 结构。`core/src/arxml_render.rs` 输出全局 EcuC Pdu 引用，`diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu` 针对旧系统 PDU DEST 与 N-PDU 动态长度做拒绝。Com 信号到 DcmDspData 的关联使用工具 SDG，不作为标准第三方绑定。
- **规范行为的局部对照：**本地 `AUTOSAR_CP_SWS_CANTransportLayer.pdf` PDF 页 31 的 `SWS_CanTp_00312/00313/00314` 对应 N_Cr 和序号故障，页 34 的 `SWS_CanTp_00315/00316` 对应 N_Bs；`AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf` PDF 页 111 的 `SWS_Dcm_00250`、页 207 的 `SWS_Dcm_00251`、页 135–137 的 `SWS_Dcm_00253/00438/00433` 是 0x10、0x3E、0x22 的相关检查入口。这些条款涉及标准模块和更广服务行为，本审计只核对当前主机子集的报文观察，不判定完整满足。完整 Dcm/CanTp/PduR/CanIf SWS、BSWMD、MemMap 和 MISRA 适用性尚未逐项完成。
- **项目规则与主机行为：**不同的 11 位物理 ID、1–8 个 32-bit Tx 信号、S3 下限、正 N_Bs/N_Cr、固定 256 字节 N-SDU 和主机 1 ms 虚拟时钟是产品支持子集；其他诊断选项另立档案。无 DTC，因此 NvM 损坏、故障重启恢复不适用于本组合；主机进程重启、外部 CAN 电气时序、第三方诊断仪互操作本次未执行。

## 生成、构建与独立预期

测试保存并重开 ARXML，注入不改变模型的用户注释，核对诊断 ID/DID 保持，再生成工程。`files.list` 中每项文件与 `files.list`、`files.sha256` 的内容在再次生成后逐字节相等；`generator::build` 从生成目录编译所有 `src/*.c` 与 `Ecu_Config.c`，以 `gcc -std=c99 -Wall -Wextra -Werror -pedantic` 链接 `ecu_host.exe`。`Dcm_Externals.h` 声明 `Ecu_DcmRead_<index>`，`Ecu_Config.c` 导出实现，主机 Dcm 经配置读回调取信号；这只是主机符号闭包。测试临时生成目录不作为归档交付物；本轮未单独保留一份完整清单或 BSWMD。

独立按信号设置值 `287454020=0x11223344`、`1432778632=0x55667788` 推导 DID 响应载荷 `62 12 34 11 22 33 44 55 66 77 88`（11 字节）：首帧 `10 0B 62 12 34 11 22 33`，流控后连续帧 `21 44 55 66 77 88`。同一测试直接断言以下响应，不从 ECU 输出反推预期：

| 场景 | 独立预期与实测断言 |
| --- | --- |
| 默认会话读 DID | `X 1800 4 037F2231`，NRC 0x31 |
| 请求扩展会话 | `X 1800 7 06500300320032` |
| 扩展会话多帧 DID | `X 1800 8 100B621234112233`，FC 后 `X 1800 6 214455667788` |
| S3 过期后读 DID | 回到 `X 1800 4 037F2231` |

`diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response` 另检查 `3E 80` 无正响应、块大小 1 和 STmin=5 ms 下连续帧延后；`diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers` 检查错误序号 `E TP_SEQUENCE`、N_Cr 超时与随后正常请求恢复。`host::run_diagnostic` 另按生成配置和独立打包计算值检查多帧响应、N_Bs 超时、错误序号、N_Cr 超时及 S3 回退，但测试器读取生成的 `profile.txt`，不能单独证明 ARXML 配置正确。

## 运行记录、拒绝路径与六门判定

从仓库根目录设置本机 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`，执行：

```powershell
cargo test --manifest-path core/Cargo.toml --test end_to_end diagnostic_ -- --nocapture
cargo test --manifest-path core/Cargo.toml --test end_to_end configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did -- --nocapture
python scripts/workflow.py verify --scope all
python scripts/workflow.py check
```

首条命令在测试补充前退出码 0，6/6；第二条在补充完整再生成文件比较后退出码 0，1/1；全量 `verify --scope all` 退出码 0：工作流 Python 测试 8/8、`npm ci`、UI TypeScript/Vite 构建、Rust 单元 2/2、端到端 39/39、Tauri debug 构建通过。Rust/MSVC 链接仍显示 `LNK4098` 默认库冲突警告，不将警告或 GCC 零错误解释为静态质量通过。

冲突 CAN ID 在配置阶段返回错误且不建立诊断；导入 CanTp padding 打开时产生 `DIAG_UNSUPPORTED`，生成拒绝且目标目录不存在；旧系统 PDU DEST 和系统 N-PDU 的 `HAS-DYNAMIC-LENGTH` 导入后 `DIAG_UNSUPPORTED`，保存/生成拒绝，来源字节保留。错误传输序号给出 `E TP_SEQUENCE`，超时给出 `E TP_TIMEOUT`，后续完整请求可恢复。这些拒绝由上述测试及 `diagnostic_ecuc_refs_reject_old_system_destinations_and_dynamic_npdu`、`unsupported_imported_transport_padding_blocks_diagnostic_generation` 断言。

| `HOST-DOCAN-01` 门 | 本轮状态与理由 |
| --- | --- |
| `input_roundtrip` | `pending_review`：工具生成单文件重开、注释保留、XSD/引用与部分拒绝有证据；外部多文件诊断输入及编辑往返未执行。 |
| `artifact_closure` | `pending_review`：完整生成文件逐字节再生成比较和主机读回调链接有证据；标准 BSWMD/RTE、逐模块 API 与工件闭包未完成。 |
| `build_static` | `pending_review`：Windows GCC C99 编译链接通过；MISRA、MemMap、偏离记录与目标资源核对未执行。 |
| `independent_behavior` | `pending_review`：固定信号金向量、单/多帧、会话、错误与恢复有断言；诊断仪互操作和更广配置未执行。 |
| `user_workflow` | `not_run`：本轮未操作隔离的原生 Tauri 窗口、外部诊断项目或非实现者离线交接。 |
| `spec_obligations` | `not_run`：只定位相关规范条款与局部报文证据，未完成标准模块逐条适用性及静态/描述工件核对。 |

审计只收口基础连接证据边界，`claim_level` 保持 `documented_behavior`。`HOST-DOCAN-WRITE-01`、`HOST-DOCAN-ROUTINE-01`、`HOST-DOCAN-DTC-01`、`HOST-DOCAN-SEC-WRITE-01`、`HOST-DOCAN-SEC-DTC-01` 全部门继续 `not_run`；写入＋DTC 等其他组合也未由本轮覆盖。下一项建议单 DTC/NvM 档案，重点是落盘失败、状态损坏、重启与诊断报文的独立核对。

## 独立审查

独立审查者 Agent `audit002_review`，2026-09-26，只读核对任务、状态、源码、测试、本地官方资料包哈希与所引 SWS 位置；首次在新 PowerShell 会话未设置 vcpkg/libclang 环境变量时，Cargo 因找不到 `libxml2` 停止；按 README 设置后独立复跑主用例 1/1、`diagnostic_` 6/6、旧引用/动态长度拒绝 1/1、`workflow.py check`，均通过。它没有重跑全量验证，本记录的 `verify --scope all` 仍以本轮开发执行结果为据。审查结论为 **AUDIT-002 可作为有界证据审计收口，HOST-DOCAN-01 保持 `documented_behavior`，六门不得标为 `passed`**。

审查指出旧系统 PDU 引用和 N-PDU 动态长度用例断言生成失败、来源字节不变，却未直接断言输出目录不存在。`generator::generate` 在 `core/src/generator.rs` 的 `checked_profile` 检查通过后才预留/安装输出目录；结合测试中的配置拒绝，这一路径从源码可判定不会创建输出，本轮仍将其作为源码核对结论，不写成测试直接断言。CanTp padding 用例则直接断言目标目录不存在。此差别不影响审计收口，也不提升输入门状态。
