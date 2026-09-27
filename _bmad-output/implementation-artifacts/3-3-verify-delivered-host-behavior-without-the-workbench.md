---
title: Verify delivered host behavior without the workbench
type: feature
created: '2026-09-28'
status: done
baseline_commit: 14efd7abdf979354737a1d655d19fd4d795d7683
context:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
---

## Intent

**使用者成果：**工程接收者用一组随包的合法双 ECU R24-11 主机参考输入和离线复验入口，在没有工作台仓库的环境下构建两个虚拟 ECU，观察 CAN 信号闭环、声明的物理诊断结果及至少一条拒绝或恢复路径，并得到可保存的通过/失败记录。

**依据与范围：**PRD R2–R4、R7、R5，Epic 3 的主机交接退出条件。测试仅覆盖固定参考输入及已实现的受限 CAN/DoCAN 行为；参考向量通过不自动将任意使用者配置或能力档案升级为“内部已支持”。

## Boundaries & Constraints

- 使用 Story 3.1/3.2 的可重建包；固定预期 CAN 帧与诊断响应由独立参考数据给出，不用被测生成代码计算“预期”。
- 参考包提供 `verify.ps1` 作为离线入口，只要求已声明的 Windows、PowerShell 和 MinGW GCC；随包提供测试器及参考配置，不访问仓库源码、网络或用户桌面 GUI。
- 在新的临时构建目录中运行，不覆盖用户已有 `ecu_host.exe`、输入、密钥或状态文件；按声明的超时结束子进程。
- 逐步记录“输入校验、构建、运行向量、拒绝/恢复”的实际结果和工具版本；未运行与失败有不同状态。对不适用的诊断向量给出明确边界，不将主机时序推断为实机能力。

## Acceptance Criteria

1. 非实现者将参考包移至新路径，仅按交接说明和包内入口完成双 ECU 构建及 CAN 双向信号的独立预期比对，结果可复现且不需要本仓库。
2. 对声明的物理诊断子集，独立测试器至少核对一次合法请求/响应及一条拒绝或故障恢复；预期请求和响应明确写入随包参考数据，不由被测实现即时推导。
3. 删除或篡改包内文件、缺少 GCC、构建失败、错误报文、异常退出或超时均使入口非零退出并保留可定位记录；绝不输出“通过”或修改原包以掩盖失败。
4. 交接结果明确标为固定 Windows 主机参考配置，列出未验证的 ECU Extract/SWC/RTE/AUTOSAR OS、硬件实时性、真实存储、第三方互操作和完整规范义务；不自动修改能力支持等级。

## Verification

以干净临时目录执行离线复验，并用缺文件、错误帧、超时和缺编译器反例检查失败关闭。运行相关 `core/tests/end_to_end.rs` 集成测试及增量质量门；不在用户交互桌面弹窗或抢焦点。

## Code Map

- `core/src/bin/package_host_reference.rs`：从固定的 Alpha/Beta 合法主机配置生成双 ECU 参考交付包。
- `runtime/reference-verify.ps1`、`runtime/reference-vectors.json`、`runtime/reference-README.md`：离线完整性、构建、独立向量和报告入口。
- `core/tests/end_to_end.rs`：搬迁后通过、错误预期与缺输入失败的端到端验证。

## Tasks & Acceptance

- [x] 在 `core/src/bin/package_host_reference.rs` 产出带输入快照的双 ECU 参考包；Given 合法 R24-11 XSD，When 生成并搬迁，Then 包内包含独立构建所需源码及参考资料。
- [x] 在 `runtime/reference-verify.ps1` 与 `runtime/reference-vectors.json` 核对 CAN 双向和物理诊断拒绝/恢复；Given 预期与实际不一致，When 离线复验，Then 非零退出并保存失败报告。
- [x] 在 `runtime/reference-verify.ps1` 防止修改原包，校验缺文件/篡改、GCC 缺失、构建失败、异常退出与超时；Given 任一失败，Then 不报告通过。
- [x] 在 `core/tests/end_to_end.rs` 验证新路径离线通过与负例；在说明中明确固定主机剖面及未验范围。
