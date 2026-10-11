# 0003. ARXML 原字节为配置权威，唯一私有验证计划

## 背景

工作台要编辑、保存和再生成用户的多文件 R24-11 ARXML，其中可能包含工具尚未理解的有效内容。若界面、内部模型或各生成器分别保存或推断配置，就会出现多个配置权威：保存时丢失未知内容，生成器各自猜测 ECU、信号、接口和任务所有者。

## 决策

1. 外部 ARXML 原字节（及工程 manifest）是唯一持久配置。Workspace 保留每个源的 saved／current 原字节、逻辑来源和文件身份；对象图、定义索引与投影是可重建的派生物，不序列化回 XML，也不另建持久配置库。
2. 未修改的文件不重写；未支持但有效的内容原样保留。无法安全解释或无法确定编辑影响时只读或拒绝，不猜测。
3. 校验通过后形成一份私有、不可变、全有或全无的 `ValidatedIntegrationPlan`。它包含输入文件角色／逻辑相对路径／原始字节 SHA-256、目标、完整 AUTOSAR 对象路径与实例、类型映射、通信链、服务绑定、调度关系、外部符号生产者与源码／工具／许可闭包。该类型没有未经检查的构造器、可变访问器或反序列化实现。
4. 所有生成器、预览、交接入口只消费该计划，不重新解析 XML、不补隐含映射。对象以完整路径＋实例身份关联，数值句柄由计划确定分配，符号确定性命名，归一化碰撞是错误。
5. 每个外部符号和产物恰有一个生产者；消费者、类型和目标必须闭合，否则整体拒绝，不输出半套工程。

## 后果

- 生成器的修复只能回到计划构造或对应 emitter；在模板里补判断会被视为第二套语义。
- 安全编辑需要原文补丁能力和完整入站引用分析，代价较高，但换来未知内容不丢失。
- 计划失败不产生输出、不改变会话，用户看到的是定位到源对象的诊断。

## 状态

已接受（2026-09），持续有效。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/ARCHITECTURE-SPINE.md`（AD-1、AD-2）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/INTEGRATION-CONTRACT.md`（§1）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md`（AD-1、AD-3、AD-5、AD-6）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md`（AD-1）。
