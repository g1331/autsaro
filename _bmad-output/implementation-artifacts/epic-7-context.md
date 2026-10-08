# Epic 7 Context: 可独立使用的统一 ECU 工程配置工作台

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

把既有 CAN 和标准 ECU 配置、生成、构建与主机验证迁入同一个真实桌面工作台。普通使用者使用工作台时，不需要官方 XSD/MOD、源码 checkout、联网或预先安装编译器，即可建立和打开工程、查看真实对象及原文、按定义编辑和批量修复、安全保存重开、校验并交接受支持的源码。生成、构建及行为验证分别报告，不扩大已有运行时、硬件或标准符合性声明。

当前实施状态以中央实施规格、sprint 与交接记录为准：7.1–7.10 开发完成，7.11 为 review，Epic 为 in-progress。规划中“尚未实施”的文字属于旧规划阶段，不能覆盖较新的实施结果。2026-10-09 用户已授权完成开发收尾并推进 PR 合并；这更新了此前仅本地提交的交付边界，不使历史未完成发行验收自动通过。真实安装不再是本 Epic 强制验收前置，后续 CI 原生行为可使用当前构建产物或解包产物；安装器未验不得宣称已验证。

## Stories

- Story 7.1: 以内置规则安全打开并检查真实工程
- Story 7.2: 使用内置定义检查参数约束与真实引用
- Story 7.3: 按定义应用参数与引用并安全保存
- Story 7.4: 预览并原子应用多对象参数与引用修改
- Story 7.5: 安全创建改名和移除配置实例
- Story 7.6: 从原创模板建立可搬移的工程
- Story 7.7: 在统一工程壳层中使用全部真实操作
- Story 7.8: 保护草稿并完成键盘与问题焦点接力
- Story 7.9: 保存分层设置并迁入暂定身份
- Story 7.10: 独立预览源码并保护用户代码再生成
- Story 7.11: 独立复核发行配置链与完整工程交接

## Requirements & Constraints

版次为 CP R24-11；支持既有 CAN 信号输入、标准 ECU Extract 多文件输入及可安全解释的 ECUC 配置。保留合法未知内容，未知条件、变体、表达式和 instance-reference 明确只读或阻断相关消费者。模块可配置不等于可生成；不放宽既有有限目标 allowlist。内置自有定义覆盖全部产品已声明支持的模块与配置剖面，不得缩成几个表单字段。

单输入保持 50 MiB、UTF-8、路径及链接安全、良构 XML 和 DTD/实体拒绝边界。解析、校验、索引与扫描不占 GUI 主线程；筛选和展开复用快照，日志摘要有界且完整 owned 详情可取。外部改源、旧确认、恢复备份、目录冲突、取消与晚回调均不能破坏当前输入或旧输出。

必须分别展示 source-safety、schema、definition、target-generation 实际执行结果。schema 是明确覆盖内的原生结构、顺序、基数及类型检查，不是完整官方 XSD 认证；unsupported 与 not_run 不冒充 passed。编辑拒绝新增或恶化违规，允许保留完全未变的旧 definition witness；实际 native schema error 仍阻断保存。目标生成错误不自动阻断安全浏览、修复或保存。

退出覆盖两个不同原创 CAN 工程、七文件标准 ECU 及不同 ID/周期变体、多模块通用类型/结构/引用编辑、关键全拒绝及数据保留。开发侧使用固定合法官方 XSD/MOD 与独立正反例对照；用户侧 Windows/Linux 使用当前构建产物或解包产物的隔离原生环境无官方资源、无 checkout、无开发语言工具/编译器、离线操作并生成源码。独立消费者另行构建并验证真实 CAN/DID/N_Cr/恢复及旧目标行为。macOS 无隔离宿主如实未验证，不能以未运行替代 Windows/Linux 必需证据。

发行出口仍未闭合：历史完整 deb、AppRun、Windows MSI 场景为 0/3，四个跨故事出口保留未完成；0/3 仅描述旧包场景，不是修订后必须逐包安装补齐的计数。历史分段 GUI、v2 独立消费及开发测试按原输入与身份复用；保存基线和 UI 批次状态修复后的原生验收须使用当前提交的构建产物或解包产物，旧包仅保留历史身份。剩余验收包括完整 owned 长日志复制、取消／晚结果及原字节保持、真实进程关闭、旧诊断／有序 DID 和两类 v1 GUI 交接与独立消费。旧 cleanup_unconfirmed 保持 RED／UNKNOWN，不用后来进程不存在追认成功。Windows 曾遇隔离 provider 拒绝，必须使用允许的隔离环境，不绕过拒绝、不占用户桌面；不存在已确认的 ECU-v1 种子时先核定原入口与输入契约，不伪造历史封包。

## Technical Decisions

原 ARXML saved/current 字节是配置权威，源索引和对象图都是可重建投影；成员文件不保存参数或通过状态。扩大同一 Workspace、Session 和前端 reducer，不建立第二配置库/store，不升级锁定框架或重选 OS。

BuiltinRuleSet 为产品原创实现和不可变自有模块元数据；不嵌入官方原文、加密 XML 或机械转换副本。RuleSetIdentity={release,rulesVersion,sha256} 绑定确定性 inventory、规则实现来源、覆盖及全部 metadata 摘要，产品构建/启动核验，用户设置或环境不能替换。缺失/损坏为软件错误，不能降级假通过。definitionFingerprint 由后台绑定内置 metadata 与工程显式接纳扩展。

workspaceEpoch/sourceId/objectId/fieldId/changeId/token 为后台不透明字符串。改名保留对象身份，删除不复用，重开换 epoch；显示路径不是业务 ID。下一次请求使用返回 capabilities 的当前 fingerprint，Reply 请求 echo 不当作当前 token。wire 数值用 kind/lexeme，区分 absent、默认来源和 explicit。

唯一 ChangeSet 使用 op 标签、ObjectRef/FieldRef 与 expected 值，prepare 计算整个 prospective graph 及实际入站影响，不改内存/磁盘；apply 精确核对 changeRevision、输入/定义身份、批次及外部状态，一次发布一次 revision。创建必需字段/子实例同批完成；移除和改名无法核定的引用影响拒绝。

PreparedSave 的 preview/confirm 继续核对原字节、路径、库存/定义身份、staging、备份及恢复。workbench-project.json v1 只记录安全相对成员、hint、applicationInputs 和 acceptedExtensionDefinitions；hint 不可信。can-empty-v1、can-signals-v1、standard-ecu-v1 为原创模板，空目标目录显式预览创建；直接 ARXML 可继续使用，也可显式保存为工程。

扩展输入为 catalog.json v1 和 inventory 列出的原始定义文件，核对版次、两层摘要、路径、完整成员及冲突，不覆盖内置项、不执行代码。app_config_dir/definition-catalogs/<sha256>/ 为完整核验后发布的不可变缓存；缓存存在不等于工程接纳。工程接纳集合随源保存同一事务持久化，直接 ARXML 会话不隐式恢复；缺精确扩展仅限制实际消费者。

源码 preview 不编译，confirm 只安装预览字节。workbench-ownership.json v1 区分配置、用户应用、产品、生成、目标、构建拥有权；live 用户应用不进入输出可写集，显式初始化仅创建不存在的源，封存 snapshot 不可改。初始应用槽仅 epic4-single-application-v1，descriptor 从真实组件契约产生。ledger/payload/application snapshot 进入原严格 seal，未知 owner/version、生成文件改动及未经核定移除拒绝。

autosar-workbench-handoff-v2 精确记录规则身份、必需接纳扩展、成员/应用输入和拥有权。导入先核对 seal 与本机可信规则三字段完全一致，再按实际 source/catalog 重建、校验、重渲染和逐字节比较；不信任复制元数据为规则。原 host/ecu v1 精确分派，原官方资源摘要及闭包不弱化，不自动升级。源码、独立输出、构建和私有日志分离。

## UX & Interaction Patterns

根 DESIGN 和当前 EXPERIENCE 是权威；冻结原型只作历史参考，旧官方资源设置门禁已被取代。统一菜单/操作条/工具轨、一棵对象或文件树、复用文档标签、属性/真实引用检查器、底部问题/生成/构建/主机验证/日志和状态栏，迁入所有既有真实操作，不留假按钮。

遵循紧凑中性明暗 tokens、13px 正文、系统/等宽字体和透明珊瑚 A 身份。1480×920 与1080×720 主链可用；窄屏先折检查器/树，表格局部滚动。外观 light/dark/system、规则与模块定义、执行工具类别独立草稿和保存；内置版本/覆盖只读，扩展显式导入/移除，执行工具环境优先。主题改变不重建工程或丢草稿。

对象上下文替换先处理“应用并继续/放弃草稿/留在原处”；工程替换、重开、两类交接及退出先处理“返回并保存/明确放弃/取消切换”，全部应用和保存确认成功才继续。非破坏面板、筛选、主题和设置保留草稿。问题依真实身份接力树/文档/字段焦点，未知目标保留可复制详情。公共命令含 Ctrl/Cmd+K/S/O、Alt+1/4/9、Escape；焦点圈定归还、树键盘、label/aria-invalid/describedby、非颜色状态、温和播报和 reduced motion 必需。

## Cross-Story Dependencies

继承已完成 Epic 1/3/4，正式 Story 验收依赖此前编号；不等待 R6/R7、旧 Epic 5/6 或 MCU。独立实现模块可在固定接口/单一文件负责人的前提下并行，但不提前标记纵向 Story 完成。7.6 提供模板、manifest、接纳持久化和纯源码生成，applicationInputs 默认空；7.10 才提供 live 应用初始化及 v2 交接。7.11 由非实现者复验完整组合出口，单项 build/test 或 mock 不能替代。
