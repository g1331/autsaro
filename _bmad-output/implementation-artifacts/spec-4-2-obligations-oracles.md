---
title: '4.2 完整义务、依赖来源与独立预期'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'b01a4082e2389449c0662d3b6df13bad669705a1'
story_key: '4-2-固定标准义务-依赖来源及独立验收预期'
context: []
---

<frozen-after-approval reason="用户2026-09-30调整流程；保留固定依赖、许可和独立预期，移除额外assurance机制">

## Intent

固定 Epic 4 实际使用的内核、工具链、规范输入和独立 CAN／UDS／OS 预期，使后续开发验证有可复现的真实输入。开发与复核按安装的 BMad，不维护额外义务登记、能力审批或报告目录。

## Boundaries & Constraints

FreeRTOS固定commit、原件、许可证和生产补丁保持；AUTOSAR原件不随工程分发。独立预期不由生成器推导，配置、字节、FIFO、错误和计时边界相符。保留实际产品行为及构建身份检查。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 固定内核或编译器被篡改／替换，执行构建 | 明确拒绝，依赖身份与许可保留 |
| 独立CAN／UDS／OS输入与配置，执行测试 | 报文、状态、FIFO及计时符合独立预期 |
| 预期字节、期限或配置处理周期被错误修改 | 相应拒绝测试识别差异 |

</frozen-after-approval>

## Code Map

- third_party/freertos/source-manifest.json、内核原件／LICENSE及runtime/os/patches/：固定依赖。
- runtime/os/toolchain.json：原生测试使用的编译器身份。
- core/tests/fixtures/epic4_oracles/：独立协议／OS预期与最小原生检查回放数据。
- scripts/epic4_oracles.py、scripts/test_epic4_oracles.py、scripts/epic4_os.py：实际配置与预期检查及拒绝测试。

## Tasks & Acceptance

- [x] 固定实际依赖和许可信息。
- [x] 保留独立协议／OS输入及行为／拒绝断言。
- [x] 将验证与复核结论记录在BMad工件，移除额外assurance机制。

## Implementation Notes

本故事原有产品输入及独立预期已建立。2026-09-30按用户明确调整移除专属矩阵审计和报告体系；只保留测试真正使用的依赖身份、预期和拒绝逻辑。历史材料由Git保留，不新建归档。

## Review Triage Log

原实施的独立复核发现已在当时修正；本次流程清理由独立清理规格继续验证和复核。

## Verification

`epic4_independent_oracle_contracts`执行实际参考配置与独立预期检查；Python拒绝测试保护错误字节、期限和处理周期。生产原生行为由各对应story测试验证。
