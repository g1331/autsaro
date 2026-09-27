# AUDIT-001：主机 CAN 证据审计收口（2026-09-26）

## 范围、基线与可重建输入

- 本次只审计 CP/FO R24-11、固定 Mcu/Can/CanIf 主机剖面、11 位 Classical CAN、DLC 1–8、无符号 LSB0 信号与 Windows/GCC 双 ECU。续作基线为 `e977cc385883`，分支 `audit/AUDIT-001-closure`；前两份证据分别对应 `afdfd77e0faf` 与 `c747bf91f365` 的源码和实际演练。本次不修改产品实现或扩大支持声明。
- R24-11 官方 FO XSD ZIP、CP Showcase ZIP、CP ECUC MOD ZIP 的 SHA-256 依次是 `9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766`、`dd55faad0bdc22181dd8fddf3927ad59b36c88ff9a7848f05f445c9b141fd701`、`df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb`。本地位置与可重建的官方 15 文件导入、工具生成后拆分的三文件主机 CAN 输入，见[行为与来源证据](AUDIT-001-host-can-2026-09-26.md)及[隔离原生界面证据](AUDIT-001-host-can-ui-2026-09-26.md)。官方 15 文件样例未提供可直接编辑的完整主机 CAN 项目。
- 当前工具链：Rust/Cargo 1.98.1 MSVC、MSYS2 GCC 16.1.0、Node 24.19.0、npm 11.17.0、Python 3.12.9。Windows 目标为本机主机虚拟 ECU，不是 MCU 或第三方 CAN 栈。未选择配置变体；影响生成的未决变体应拒绝。

## 工件和模块义务的实际核对

`core/src/generator.rs` 的 `source_files` 无条件复制 `runtime/include/*.h` 与 `runtime/src/*.c`；CAN-only 工程也包含全部 **16 个头文件、15 个 C 文件**，另写 `Ecu_Config.c`、`Dcm_Externals.h`、`profile.txt`，合计 `files.list` 中 **34 项**，另有 `files.list` 和 `files.sha256`。`build` 编译全部 15 个运行时 C 文件与 `Ecu_Config.c`，以 `-std=c99 -Wall -Wextra -Werror -pedantic` 链接。已有双 ECU 测试比较完整文件集合并运行独立位向量；这证明该主机配置的编译/链接与所测行为，不证明标准模块接口或全部工件义务。

| 交付物角色 | 当前可观察事实 | 对内部支持声明的结论 |
| --- | --- | --- |
| 主机运行时与固定配置 | `Can/CanIf/Com/PduR/Os` 等模块名的 `.c/.h` 与 `Ecu_Config.c` 一同交付；`Ecu_Config` 是主机聚合配置。诊断、Dem/NvM 等文件即使未配置诊断也复制并编译。 | 先按主机专属实现记录。文件名与可链接不构成标准 BSW API、配置或行为的证据；每个拟声明为标准 BSW 的模块须另立档案。 |
| `Rte.c` / `Rte.h` | 两个固定函数 `Rte_WriteSignal`、`Rte_ReadSignal` 只是转调 Com；源码来自 `runtime/`，不随 ECU/SWC 模型生成。 | 这是主机信号桥接层；不能仅因文件名是 `Rte.h` 就宣称 RTE 生成器或 SWC 接口已实现。 |
| 模型和描述文件 | ARXML 渲染器生成固定主机 ECUC 值与系统 PDU/信号；生成器仅输出上述三项专属文件，没有 BSW Module Description、RTE Basic Software Module Description 或其产物清单生成路径。 | 固定主机 ECUC 的局部 MOD 闭包有测试；标准 BSW/RTE 描述工件门未满足，不将主机 ECUC 值误当 BSWMD。 |

本地 R24-11 [BSW General PDF](../../../docs/official/R24-11/CP/BSWGeneral/AUTOSAR_CP_SWS_BSWGeneral.pdf) 原文页 18–37 与仓库[质量门研究](../../../.scratch/autosar-platform/research/generated-c-quality-gates.md)用于下表适用性核对。条款只施加在其实际对象上；下表的“缺口”不等于判定目前主机专属代码违反一个尚未声明适用的标准模块接口。

| 条款/项目门 | 本次检查 | 状态与边界 |
| --- | --- | --- |
| `SWS_BSW_00001`、`SWS_BSW_00002`：标准 BSW 的描述 ARXML 和模块文档 | 34 项清单及 `generator.rs` 写入路径没有 BSWMD；`runtime/README.md` 是整体主机运行时说明，未逐标准模块提供要求与偏离、配置及集成文档。 | 若提升为标准 BSW 声明则缺工件；本主机行为档案不提升。 |
| `SWS_BSW_00004`、`SWS_BSW_00020`：实现源码和公开头文件 | 多个模块有相应命名的 `.c/.h`，但没有完成逐模块公共声明与 SWS API 对照。 | 文件存在仅为局部证据；接口义务待逐模块审查。 |
| `SWS_BSW_00006`：BSW 实现源码包含 `<Mip>_MemMap.h` | 在 `runtime/` 与生成器中检索 `MemMap`、`START_SEC`、`STOP_SEC` 无匹配；交付清单也没有相应头文件。 | 标准 BSW 声明的明确工件缺口；无目标段放置证据。 |
| `SWS_BSW_00013`：条件性的链接时 const 配置源 | `Ecu_Config.c` 含主机 `const EcuConfig`，但尚未把它分配为各标准 BSW 模块的链接时配置参数。 | 适用条件未逐模块判定；不要求生成空配置源，也不记为通过。 |
| `SWS_BSW_00036`：跨模块版本检查 | 未建立逐模块外部头文件版本宏、匹配检查及拒绝用例。 | 标准 BSW 集成待审。 |
| `SWS_BSW_00115`：BSW C 的 MISRA C:2012 与有理由的偏离 | 未运行合适的 MISRA 检查器，也无逐违规位置的偏离记录。当前 GCC 警告检查不是 MISRA 检查。 | `build_static` 只有构建部分的可复现证据，静态义务未通过。 |
| `SWS_BSW_00234`：BSW 外部接口 C99 绑定 | GCC C99 编译与链接通过；未对照逐模块标准外部函数签名、类型和头文件。 | 只证明当前主机接口可由 GCC 编译；互操作待审。 |
| `SWS_Rte_05086`、`SWS_Rte_05090`：生成 RTE 描述及产物记录 | 固定 `Rte.h/.c` 存在，但生成器没有 RTE 描述与产物记录路径。 | 真实 RTE 生成声明尚不成立；这两个要求不因主机桥接层而自动通过。 |

## 执行结果与拒绝路径

在仓库根目录以本机已安装的 vcpkg `x64-windows-static`、libclang 设置 `VCPKG_ROOT`、`VCPKGRS_TRIPLET`、`LIBCLANG_PATH`，执行：

```powershell
python scripts/workflow.py verify --scope all
python scripts/workflow.py check
```

本轮 `verify --scope all` **退出码 0**：工作流 Python 测试 8/8、`npm ci`、TypeScript/Vite 构建、Rust 单元测试 2/2、端到端测试 39/39（132.48 秒）、Tauri debug 构建均通过。Rust/MSVC 链接仍报 `LNK4098` 默认库冲突警告，本次未处理，也不将其解释为静态质量通过。前两份证据分别记录过双 ECU `B0 01` 金向量、错误 DLC、接收超时、BUS_OFF/恢复，以及磁盘来源变更时的校验/生成拒绝；本次不将那些观察扩大为并发文件修改安全。当前源码仍在 `Workspace::validate`、`save` 和 `generator::generate` 的顺序路径复核来源字节，`checked_profile` 在未决变体/阻断问题时拒绝生成。

## 逐门结论与后续任务

| HOST-CAN-01 门 | AUDIT-001 收口判定 |
| --- | --- |
| `input_roundtrip` | 官方真实 15 文件无改动保真、工具拆分三文件编辑与来源顺序变更拒绝有证据；独立外部 CAN 项目的可编辑往返仍缺。维持 `pending_review`。 |
| `artifact_closure` | 34 项完整清单、重复生成稳定、固定主机 ECUC 局部 MOD 核对及主机 C 符号闭包有证据；标准 BSW/RTE 描述与逐模块接口闭包未成立。维持 `pending_review`。 |
| `build_static` | 本机 GCC 编译链接有证据；MISRA、MemMap、模块版本与偏离及目标资源核对未运行。维持 `pending_review`。 |
| `independent_behavior` | 不同配置双 ECU 的独立位向量、故障和恢复已执行；对外部目标与更广配置不能外推。维持 `pending_review`。 |
| `user_workflow` | 隔离原生 Tauri/IPC 流程已演练；独立外部输入的差异预览、非实现者离线交接与重开复现未执行。维持 `pending_review`。 |
| `spec_obligations` | 本次只界定了适用对象及可定位的工件缺口；无逐模块完整 SWS、BSWMD、RTE/MISRA 对照。维持 `not_run`。 |

审计任务可以以**未升级支持声明**收口：主机已有行为保留 `documented_behavior`，六门均不标为 `passed`。下一个 CAN 质量任务应优先明确各交付 C 文件的标准 BSW/RTE/主机专属角色，再对拟声明标准的模块补 BSWMD、MemMap、版本/接口、MISRA 与逐模块 SWS 证据；外部多文件 CAN 输入及可见差异/离线交接另作有界用户流程任务。顺序来源变更已拒绝，但来源预检后的并发改动和多文件安装窗口尚无故障注入证据；不宣称原子多文件保存或并发安全。真实 MCU、第三方 CAN 栈和官方一致性均未验证。

## 独立复核

独立审查者：Agent `audit001_closure_review`，2026-09-26，只读。它直接核对 `runtime/include` **16** 个 `.h`、`runtime/src` **15** 个 `.c`、`generator.rs` 的无条件复制与编译路径、`end_to_end.rs` 对 34 项及两个清单的逐字节比较、`Rte.c` 的两个 Com 转调；直接读取本地 R24-11 BSW General PDF 原文页 18/19/21/22/23/26/30/37 和 RTE PDF 原文页 98/557，确认本记录对条款对象与条件的区分基本准确。独立运行 `python scripts/workflow.py check` 通过，核对本分支只改文档与状态；未独立重跑 132 秒完整构建，因此该结果仍以本轮执行记录为据。审查结论为 **AUDIT-001 可收口，HOST-CAN-01 保持 `documented_behavior`，六门不升级**；未发现阻断事实错误。审查要求的索引下一缺口与任务审查字段已同步。此复核不代替后续逐模块规范审查。
