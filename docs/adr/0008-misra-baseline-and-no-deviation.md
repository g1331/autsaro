# 0008. MISRA C:2012 修订基线与无偏离政策

## 背景

交付的 C 运行时与生成代码需要一个固定的编码规范基线。自动分析工具（固定 Cppcheck 2.21.0 及 MISRA addon）只能覆盖部分规则，且有已知模型限制（例如 R21.8 仍报告 `getenv`、终止函数清单缺 `_Exit`／`quick_exit`）。当前完整分析已经可以运行，但源码结果仍为 `passed=false`，221 项人工评估未完成。若允许批量 suppression 或以“扫描完整”代替符合性，会掩盖真实违反。

## 决策

1. 基线为 **MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2**，共 200 Rules＋21 Directives＝221 项；语言目标仍为 C99。
2. 工具可用、扫描完整、源码诊断通过、人工规范评估与实际行为分别证明，不互相替代。原始诊断完整保留；固定内核等采用代码标记为 `adopted`，但不自动排除，纳入实际责任链。
3. 诊断逐项分为真实违反、误报、不适用与未核查；工具限制只是核查线索，不授权 suppression 或批量判误报。
4. **当前没有任何已批准的偏离。** Mandatory 规则不允许偏离；无法消除的真实违反保持阻塞，并提交具体替代或改造方案。不做规则降级、批量 suppression 或缩小既有支持范围。
5. 申请偏离前须核对适用的 amendment／TC 与实际调用路径，不能把旧 checker 的规则编号直接当作当前规范依据；偏离只能由维护者批准，代理或工具不能批准。
6. 修改 C、头文件或生成 C 的模板时，对实际生成的受影响 profile 运行 `c-check`（命令见 `docs/development/testing/generated-c.md`）。

## 后果

- 在严格源码门禁接入必需检查之前，CI 只验证分析链路完整，不声明源码合规。
- 完整 MISRA 结论需要授权正文、需求／设计追踪和人工评估，不能由自动检查关闭。
- 采用代码（内核、端口、补丁）的诊断同样需要裁决，可能促成补丁或替代方案。

## 状态

已接受（2026-10）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-5）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/BASELINE.md`（AUTOSAR 与 MISRA：已确认和未核查）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/SPEC.md`（Constraints、CAP-3）、`planning-archive-2026-10:_bmad-output/specs/spec-multi-component-scheduling/compliance-references.md`。
