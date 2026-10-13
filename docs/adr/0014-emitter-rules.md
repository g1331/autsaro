# 0014. Emitter 规则

## 背景

生成来源混合了 `.c.in` 模板、Rust `format!`／字符串拼接和对已生成 C 文本的二次替换。例如多组件 ECU 组装先生成整个文件，再 `get_mut(..).unwrap()` 加 `.replace("#endif", ..)` 插入声明。这样配置语义在多个模板和替换中各写一次，跨模块写同一文件的内容没有唯一负责人；`core/src` 中约 630 处 `unwrap`／`expect`／`panic!`（粗略统计）使计划无法保证的前提以崩溃而不是诊断表现。clippy 当时只拦 `correctness`／`suspicious`。

## 决策

1. **每个输出文件只有一个生成方。** 生成前登记文件路径与 emitter；两个 emitter 写同一路径在生成前拒绝。跨模块需要写入同一文件的内容（MemMap 段、SchM 声明、类型补充）在计划阶段汇总后交给该文件的生成方。资产来源到交付路径显式映射，拒绝碰撞、遗漏或多个 producer。
2. **不回读、不修补已生成的 C。** emitter 只消费计划，不从其他 emitter 的输出中查找、替换或追加文本；决定配置或接口的最终文本替换移入结构化计划字段与模块 emitter，一次渲染。纯占位的模板渲染可以保留。
3. **不 panic。** 计划已保证的前提用类型或构造方式表达；计划无法保证的前提（缺少映射、计数不符、名称冲突）返回 `PlanDiagnostic` 并定位到源对象。
4. **静态检查强制。** clippy 启用默认集；生成与集成模块 deny `unwrap_used`、`expect_used`、`panic`，CI 拦截新增违例；其余模块按记录的基线只减不增，不放宽门槛。
5. **结构调整不改产物。** 重组 emitter 时重新生成主线样本并逐字节比较；有意差异逐项审阅并记录在对应 PR。
6. emitter 按模块组织（如 COM、路由／TP、mode、RTE、OS、交付），共享类型来自已核定计划；COM→RTE 等跨模块 callback、类型与配置 glue 的唯一 producer、符号、声明与消费路径由对应契约一次核定，其他 emitter 只引用。
7. 不引入通用 pass manager、第二种持久中间模型或插件框架。

## 后果

- 修改某模块的配置规则时，有明确的校验落点和 emitter，不再需要在多个模板里各改一次。
- 现有回读修补代码需要先迁入计划字段，产物逐字节比较作为护栏；全量 clippy 可能带出大量既有告警，按基线逐步收紧。
- 局部测试断言契约相关的符号、类型、次序或拒绝；只有字节本身属于 seal 或确定性契约时才比较完整字节，不批量更新快照吸收错误。

## 状态

已接受（2026-10-11），实施中：决策 1–5 已实施（#37），决策 6 随各模块整理（#40–#51）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/sprint-change-proposal-2026-10-11.md`（第 1 节、决定 6、8）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-3、AD-9）、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/generator-practices.md`、`planning-archive-2026-10:_bmad-output/specs/spec-runtime-codegen-quality/work-packages.md`（B6）。
