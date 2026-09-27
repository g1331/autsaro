# AUDIT-001 主机 CAN 证据切片（2026-09-26）

## 基线与可重建输入

- 源码基线：`0270add11a80`（`master` 干净）；被测源码提交：`afdfd77e0faf`（分支 `audit/AUDIT-001-host-can`）。目标是 Windows 虚拟双 ECU、11 位 Classical CAN、DLC 1–8、无符号 LSB0、固定 Mcu/Can/CanIf 主机剖面；未选择实机或配置变体。
- 本地官方资料（Git 忽略，需自行放在 README 指定位置）：FO `AUTOSAR_FO_MMOD_XMLSchema.zip` SHA-256 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`，内有 `AUTOSAR_00053.xsd`；CP `AUTOSAR_CP_EXP_ModelingShowCases.zip` SHA-256 `dd55faad0bdc22181dd8fddf3927ad59b36c88ff9a7848f05f445c9b141fd701`；CP `AUTOSAR_CP_MOD_ECUConfigurationParameters.zip` SHA-256 `df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`，内有同名 `.arxml`。
- 工具链：`rustc/cargo 1.98.1`（`stable-x86_64-pc-windows-msvc`）、MSYS2 `gcc 16.1.0`、Node `24.19.0`、npm `11.17.0`、Python `3.12.9`；vcpkg `2026-07-27-98d7cb0`，`x64-windows-static` 的 libxml2 `2.15.4`，以及由 `LIBCLANG_PATH` 指向的 libclang。安装位置因机器而异，不属于可移植基线；本次 `clang` 命令不在 PATH，未记录其 CLI 版本。
- 可重建输入：`core/tests/end_to_end.rs` 的 `official_r24_sample_imports_as_one_split_package_without_rewriting_sources` 从官方 Showcase ZIP 取 `30_MeasurementCalibration/10_Introductory/model/` 下 **15 份** ARXML；`create_pair` 新建 Alpha/Beta 两种不同主机 CAN 配置；`split_package_save_preserves_sources_and_rejects_stale_reference_file` 把 Alpha 的系统 `I-SIGNAL` 移到第二份同名 `AR-PACKAGE`，再编辑第一份。这后一组是由本工具生成、经 R24-11 XSD 校验的多文件主机配置，**不是**官方 Showcase 中现成的 CAN 项目。

## 独立预期、执行与实际

在 PowerShell 仓库根目录执行：

```powershell
$env:VCPKGRS_TRIPLET='x64-windows-static'
if (-not $env:VCPKG_ROOT -or -not $env:LIBCLANG_PATH) {
    throw '先按 README.md 为本机设置 VCPKG_ROOT 与 LIBCLANG_PATH'
}
cargo test --manifest-path core/Cargo.toml split_package_save_preserves_sources_and_rejects_stale_reference_file -- --nocapture
python scripts/workflow.py verify --scope all
python scripts/workflow.py check
```

首次未设置三个构建环境变量时，`verify --scope core` 在 `libxml v0.3.21` 构建脚本中以退出码 **101** 停止，输出 `vcpkg did not succeed in finding libxml2`；设置上述变量后继续。修复前，新增多文件测试的 `project.save().unwrap_err()` 收到 `Ok(WorkspaceView { ... dirty: false })`，证明未修改的第二份来源文件外部变更被漏检。修复后，该测试通过：合法 ID `801 → 802` 编辑、保存、重开仍为 `802`，第二文件逐字节未变；随后准备 `802 → 803` 编辑，把第二文件的系统信号长度从 8 改成 7，保存返回包含“外部修改”的错误，第一文件保持保存前字节，第二文件保持外部改动，工作区仍为 dirty。

`verify --scope all` 退出码 **0**：工作流 Python 测试 8/8、`npm ci`、`tsc -b`/Vite 构建、Rust 单元测试 2/2、端到端测试 **38/38**、Tauri debug 构建均通过。Rust/MSVC 链接给出 `LNK4098` 默认库冲突警告；本次没有将该警告解释为静态质量通过。

现有测试与本次增强的独立预期如下：

| 场景 | 独立预期 | 实际可复现断言 |
| --- | --- | --- |
| 官方 15 文件导入 | 同名跨文件包可合并，原始 15 文件无改动；其未配置主机 CAN，不应生成 ECU | `official_r24_sample_imports_as_one_split_package_without_rewriting_sources` 通过，逐文件字节相等，生成返回错误 |
| 主机双 ECU 报文 | Alpha 的 `SendCount=54=0x36` 放在 bit 3、长 8 bit，LSB0 得 `0x36 << 3 = 0x01B0`，两字节 `B0 01`；Beta 解出 54 且有效 | `generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults` 断言 `X 801 2 B001`、`V 0 54 1`；独立 `host::run` 检查两个不同配置、优先 ID、接收超时、BUS_OFF/STARTED 和错误 DLC，通过 |
| 构建前完整生成文件集合稳定性 | 同一配置两次生成的 `files.list` 中每个文件以及 `files.list`、`files.sha256` 字节一致 | 上述双 ECU 测试逐文件比较，通过；两份目录由 `generator::build` 使用 `gcc -std=c99 -Wall -Wextra -Werror -pedantic` 编译和链接 |
| 关联 CAN 配置拒绝 | R24-11 MOD 必需容器/参数/引用目标完整；断开时不得保存或生成 | `host_can_ecuc_closes_required_mod_fields_and_rejects_broken_links` 对照本地 MOD，检查 Mcu/Can/CanIf 重数、范围、引用目标；缺 CanIfPrivate、时钟引用、Tx buffer 引用、错误 HOH 均报 `PDU_UNSUPPORTED`，只读且不落生成目录 |
| 配置和运行拒绝 | 未决变体、位序/映射冲突、重复 CanIf 映射阻止生成；错误 DLC 应由 CanIf 拒绝 | `unresolved_r24_variant_is_preserved_but_blocks_generation`、`ipdu_mapping_disagreement_with_com_blocks_generation`、`conflicting_canif_entries_for_one_pdu_block_generation`、`host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange` 均通过；最后一项观察到 `FRAME_DLC` |

上述临时生成目录由测试自动清理；复核者按测试名和固定输入重建，不能把断言通过误写成已归档的发布工件或第三方互操作记录。

## 条款、门禁与剩余缺口

- **规范结构依据：**`AUTOSAR_00053.xsd` 仅检查 XML 结构；R24-11 ECUC MOD 的 Mcu/Can/CanIf 容器定义、必需重数、参数范围和 `DESTINATION-REF` 由上述测试另行检查。`07-assurance-gates.md` 中的 `SWS_BSW_00115`、`SWS_BSW_00234`、`SWS_BSW_00006`、`SWS_BSW_00001` 及 `SWS_Rte_05086/05090` 属生成工件适用性审查，不因本次构建通过而关闭。
- **项目质量门：**跨文件保存须基于仍与打开时一致的全部来源文件；生成前未知变体和语义不闭合必须拒绝。新增保存检查关闭了已复现的顺序外部修改缺口；未修改文件在全文件预检之后再次变化的并发窗口，以及多文件安装间的竞态，仍未做故障注入。
- **主机专属：**固定虚拟时钟、控制器和 CAN 行为只针对本机 GCC/Windows 虚拟 ECU。官方 Showcase 15 文件验证了真实输入导入和无改动保真，**没有**证明官方多文件 CAN 项目中的受支持编辑；编辑样例是工具生成后拆分的主机配置。
- **待审：**未在独立原生 Tauri 窗口执行导入、编辑、差异预览和离线交接；未形成每个 BSW/RTE 工件的 SWS/BSWMD/MemMap 对照、MISRA 报告、偏离记录或资源预算；未以真实外部多文件 CAN 项目验证编辑；未做第三方 CAN 栈、真实 MCU、电气层、时序或标准符合性验证。因此 `HOST-CAN-01` 保持 `documented_behavior`，不得升级为 `internal_supported`。

## 审查

独立审查者：Agent `audit001_review`，2026-09-26，只读复核。它核对了三份本地官方 ZIP 摘要、源码保存逻辑和测试断言，独立运行 `cargo test --manifest-path core/Cargo.toml --test end_to_end`（38/38，约 128 秒）及 `python scripts/workflow.py check`（通过）；未独立重跑 `verify --scope all`。结论：顺序外部修改拒绝、官方 15 文件无改动导入、完整生成清单比较、双 ECU 金向量及故障路径的记录有据；审查同时指出预检后的并发窗口，并要求将尚未逐工件执行的 `spec_obligations` 从 `pending_review` 改为 `not_run`。该状态已修正；`artifact_closure`、`build_static` 仅有局部证据，继续 `pending_review`，不能升级为通过。复核未发现这次新增文件有写死的机器安装路径。

任务尚未收口，继续使用 `audit/AUDIT-001-host-can`。下一位从真实外部多文件 CAN 输入与原生界面流程入手，随后逐模块补静态/工件义务证据；如要证明并发保存安全，还需针对预检后改动的故障注入和相应关闭措施。
