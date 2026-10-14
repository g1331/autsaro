# 状态与所有权不变量

本篇属于[架构](README.md)：标注“目标（实施中）”的内容尚未在 `master` 落地，其余描述当前代码；决定的背景与取舍见 [ADR](../adr/)。

以下约束当前有效，修改相关代码时不得削弱。

## 配置与身份

1. **原字节是唯一配置权威**（[ADR 0003](../adr/0003-original-bytes-and-single-validated-plan.md)）。R24-11 多文件 ARXML 原字节与工程 manifest 是唯一持久配置；对象图、定义索引、表格和投影都可重建，不序列化回 XML，不建第二个持久配置库。未修改的文件不重写，未支持但有效的元素、属性、值和引用原样保留。
2. **稳定身份由后台分配。** `workspaceEpoch`、`sourceId`、`objectId`、`fieldId` 是不透明字符串；`objectId` 是实例身份，不是 SHORT-NAME 或路径。改名保留 ID，删除的 ID 不复用，重开或切换工程更换 epoch。前台只回显，不生成业务 ID。`ChangeSet.inputFingerprint` 取当前 `capabilities.fingerprint`；`Reply.inputFingerprint` 只是请求回显。
3. **原子 ChangeSet 是唯一配置变化入口。** `prepareChange` 不改 Workspace 或磁盘，返回完整影响与 `changeRevision`；`applyChange` 在串行 gate 内重核全部身份后一次发布。改名同步更新全部可核定入站引用；无法确定影响、新增或恶化约束即拒绝，旧 Workspace 不变。
4. **安全保存。** 保存经 PreparedSave：原文摘要、`create_new` staging、备份与恢复；manifest 成员变化与源集合保存同一事务。外部修改、规则／定义变化或未恢复备份使旧确认失效；不能恢复时保留备份与具体状态，不报告 clean。
5. **规则身份。** `RuleSetIdentity={release,rulesVersion,sha256}` 绑定产品内置规则实现、覆盖清单和内置模块 metadata（[ADR 0006](../adr/0006-builtin-rules-without-official-archives.md)）。规则或覆盖变化必须发布新身份；构建和启动核对可信身份，用户设置或环境变量不能替换。扩展定义按 `{catalogId,release,version,sha256}` 显式接纳，只约束实际消费者，不覆盖内置定义。
6. **诊断分范围、分状态。** 每条诊断属于 `source-safety`、`schema`、`definition`、`target-generation` 之一，携带 ruleId 与稳定 subject；结果区分 passed、failed、unsupported、not_run。编辑允许保留完全未变的旧 definition 违规，新增或恶化的拒绝；保存拒绝实际 schema 错误；生成要求目标消费闭包内全部检查通过。

## 计划与生成

7. **唯一验证计划。** 所有生成器只消费同一份私有、不可变的 `ValidatedIntegrationPlan`，不重新解析 XML、不补默认关系。对象以完整 AUTOSAR 路径加实例身份关联，数值句柄由计划确定分配；重复短名不是同一对象。每个外部符号和产物恰有一个生产者，所有消费者、类型与目标必须闭合。
8. **身份分层。** 模板 ID、集成 profile、交接格式和 target 是不同身份，不能从一个推断另一个。组件身份为（完整 type 路径，完整 composition prototype 路径），端点为（实例，端口，数据元素／操作）。本地 S/R 存储与网络传输分开保留，网络映射另带 network／channel／SystemSignal／ComSignal／PDU／frame 身份；应用类型、实现类型与总线位宽／字节序分别核对。
9. **调度关系在计划内核定。** 每个周期实体恰有一个调度调用者；周期、相位、任务位置与先后在计划中固定。同一 OS event 分派多个实体时，核定整个调度关系的周期与相位一致，不以单条 mapping 合法代替。生成 C 的名字检查覆盖参数与函数体实际引用的运行时函数、宏和状态名。
10. **生成是确定性的纯函数。** 相同输入两次生成字节一致；普通生成不运行编译器、不读官方档案、不联网。目标选择与执行结果不反写为配置参数。

## 应用源码、预览与交付

11. **live 与 sealed 分离**（[ADR 0007](../adr/0007-live-and-sealed-application-source.md)）。用户 live 应用源属于用户，永远不在生成器可写集内；封存工程中的应用文件是不可变快照，`snapshotOf` 指向已声明源成员。再生成从 live 源重新快照；被改动的封存快照一律拒绝。
12. **create-only 初始化。** 初始化只创建尚不存在的 live 成员，并与 manifest 接纳同一事务；目标已存在不覆盖，失败不留下部分成员。
13. **预览确认失效。** 预览绑定全部 ARXML、manifest、live C、规则与定义身份；任一变化使旧确认失效，必须重读、重预览。生成改变使已有 build／run 结果失效，旧结果不冒充新配置的验证。
14. **完整性拒绝。** 封存 payload、所有权台账和应用快照全部进入 `meta/files.list`／`meta/files.sha256` 闭包；台账不记录自身和 seal 元数据，避免自摘要循环。构建与离线验证消费同一不可变 seal；版本、owner、producer 或路径未知、篡改、额外文件、链接或 reparse point 一律拒绝。交接元数据不写源机器绝对路径，不打包密钥、运行状态或官方规范。
15. **ABI 与第三方身份单独审阅。** 资产摘要（`python -m autosar_tooling assets check|update`）只证明字节来源；公开签名、布局、句柄语义、wire 或交付格式变化须审阅并写入 CHANGELOG，不能用摘要更新吸收契约变化。更新摘要前核对 `.gitattributes` 规定的固定换行。

## 工作台状态

16. **两个状态所有者。** 后台 `Session` 与前台 reducer 各自唯一拥有自己的状态；immutable snapshot 与序列化 staging 阻止过期发布。草稿按 epoch／objectId／文档类型归属，表面组件不持配置副本；菜单、工具条、树和快捷键共用动作与能力原因。
17. **长任务受管。** 长任务走既有 Operation 归属与串行 mutating gate；取消先使结果失效，再等待 `ProcessOwner` 实际关闭进程树。解析、校验和扫描不在 GUI 线程，单输入上限 50 MiB，路径与 XML 防护不因新界面放宽。
18. **状态分层呈现。** 已保存、已校验、已生成、已构建、主机已运行、目标已验证分别呈现，后者不由前者推断。
