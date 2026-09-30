---
title: '清除额外 assurance 流程，恢复安装版本 BMad 开发方式'
type: 'refactor'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'd8ae0c2b40fb9af47ffd2498ff4cea58ba471233'
context: []
---

<frozen-after-approval reason="用户明确调整当前工作方式并授权修改、验证和本地提交">

## Intent

仓库额外的 assurance 政策、报告目录、能力审批和矩阵机制使普通开发维护第二套流程。以已安装 BMad 的规格、实施、验证、复核和 sprint 为唯一开发流程，删除额外机制，继续 Epic 4 产品实现。

## Boundaries & Constraints

保留实际产品源码、接口契约、行为断言、拒绝测试、独立预期、必要规范输入、固定依赖与许可。测试中间文件使用临时目录；验证与复核结果进入本规格和对应 BMad story。删除已提交报告的当前版本，历史由 Git 保留；不批量迁移到 BMad 或新归档。后台和隔离桌面规则、工作区保护及远端权限不变，不推送。不以清理流程删掉 SC1 的实际服务、容量、栈、Hook、ARTI、MemMap 和工程交接功能。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 当前规则、BMad 规划或本地技能含额外 assurance 要求；完成清理 | 普通开发只遵循安装的 BMad，额外政策、六门、固定报告目录和审批状态不再驱动执行 |
| 历史报告是测试回放输入；提取最小夹具并运行 | 成功／故障／拒绝断言保留，地址、线程号、报告元数据不进入新夹具 |
| 运行普通构建和测试 | 验证实际行为，测试只写临时文件，不改写受版本管理的报告 |
| 重生成汽车技能并检查锁定信息 | 适配源、副本和锁同步，更新不恢复额外流程要求 |
| Epic 4 尚有产品功能未实现 | 保留在途成果和准确 sprint 状态，按 BMad 继续完成，不借清理标记完成 |

</frozen-after-approval>

## Code Map

- AGENTS.md、README.md、docs/project/OWNER_GUIDE.md：移除独立验收政策与命令，保留构建使用、后台测试和产品边界。
- _bmad-output/planning-artifacts/ 及 implementation-artifacts/：移除额外报告／审批／矩阵要求，保留功能、标准接口和真实验证记录；BMad 模板字段保留。
- scripts/verify.py：保留工程检查编排，去掉 assurance／baseline scope；删除专属 assurance.py、quality_baseline.py 与对应测试。
- scripts/epic4_obligations.py：删除专属矩阵审计；保留其独立预期的实际行为检查到测试位置。
- scripts/epic4_os.py、epic4_desktop.py、epic4_bsw_catalog.py 与 core/tests/support/：移除报告封装与默认写入，保留原生／隔离 UI／IPC／接口探针。
- scripts/test_epic4_os.py：从五份历史报告提取 scenario／exit／stdout／stderr 最小回放夹具；原生运行测试不改写。
- docs/assurance/：提取实际需要的编译器固定身份后删除政策、矩阵与报告；同步 .gitattributes 和引用。
- scripts/install_automotive_skills.py、本地汽车适配及 MISRA 技能：源先改，安装器重生成副本与锁；保留编码指导和明确委托文档任务。

## Tasks & Acceptance

- [x] 清理有效规则与 BMad 规划／实施工件中的额外流程。
- [x] 提取必要身份和最小确定性夹具，删除 assurance 文件及消费者。
- [x] 修改验证、测试和探针，删除报告写入与专属检查机制。
- [x] 同步技能适配源、已安装副本与锁定信息。
- [x] 完成相关工程验证及 BMad 三路复核、本地提交，再继续 Epic 4。

## Implementation Notes

当前 Epic4 已完成17条 story，4.18在途，4.19～4.22待办。起始工作区干净，已提交两个验证通过增量065dbfa和d8ae0c2。此清理是用户明确授权的独立工作增量，不改写已实现产品事实。

## Review Triage Log

BMad blind、edge-case、verification-gap 三路只读复核均已返回，无可定位发现和验证缺口；未递延本增量问题。主代理额外核对实际脚本、测试与有效规划，修正 4.2 的旧执行入口与追踪表描述，以及汽车技能使用说明残余的固定证据目录要求。

## Verification

使用既有 Python 单测、核心集成、UI lint/build、桌面build、Clippy与适用增量格式／C99检查验证变更。技能安装器按现有 --update／--check 更新与验证；检查当前源码引用和测试前后 Git 状态。结果写在本节，不新增清理报告或审计矩阵。

已完成15项Python、17个BSW接口探针、隔离桌面真实UI/IPC、24技能锁检查、MISRA指导目录200规则/21指令和增量格式/C99检查。完整检查的15 Python、3核心单元、102核心集成（819.78s）、UI lint/build、核心Clippy均通过。桌面build最初因C盘空间不足退出101；本任务4.27GB缓存保留迁移至忽略目录，并用本地junction保留工具缓存路径，随后桌面build和Clippy补跑均退出0。未重跑无变化的核心测试。

测试后核对工作树：没有报告被重新生成，新增文件仅为本 BMad 规格、五个最小回放夹具、编译器身份及实际预期检查代码／测试。固定原始内核、MIT许可、13个产品补丁和原协议／OS预期字节保留。CONTEXT.md不存在。清理完成不关闭Epic4，4.18仍在途；后续产品任务继续。
