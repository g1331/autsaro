# Epic 9 Context: 运行时、生成 C 与开发工程体系整顿

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

先把三个生成族收敛为一条按职责组织、不含开发计划编号的生成主线，建立新的工程基线；再让维护者沿明确的模块契约、状态归属与生成来源维护主线，获得快速可信的开发反馈；让接收者取得完成适用标准整改、独立行为回归并受严格 PR 源码门禁保护的工程，为后续 CAN 扩展提供基础。继承已交付成果，不以规划完成替代实施或符合性证据。

## Stories

执行顺序：9.1、9.2（done）→ 9.21–9.28（新基线）→ 9.3–9.15 → 9.16–9.20。编号不代表执行先后。

- Story 9.1: 固定共享契约与逐模块进入依据（done）
- Story 9.2: 分离 CAN 标准接口与主机适配（done）
- Story 9.21: 让主线接管单组件工程
- Story 9.22: 让主线拥有全部 BSW 来源
- Story 9.23: 删除帧／信号 host 链路、单组件链路与外部档案模式
- Story 9.24: 让测试与开发工具回到正常入口
- Story 9.25: 用标准驱动方式替换桌面隔离验收设施
- Story 9.26: 去除开发编号并按职责命名
- Story 9.27: 用结构和静态检查守住生成规范
- Story 9.28: 按所有权组织生成工程
- Story 9.3: 明确 ECU 批次执行与收据责任
- Story 9.4: 闭合 CanIf 配置、路由与异步接口
- Story 9.5: 整理 LSduR 与 PduR 路由和缓冲契约
- Story 9.6: 整理 COM 状态与模块生成配置
- Story 9.7: 整理 CanTp 传输与可信计时
- Story 9.8: 整理 Dcm 会话与诊断生成链
- Story 9.9: 整理 ComM 与 BswM 模式衔接
- Story 9.10: 分离 Det 标准报告与主机控制
- Story 9.11: 整理 NvM 与主机持久原语
- Story 9.12: 整理 Dem 状态与持久消费者
- Story 9.13: 整理 Security 会话与 Windows 密钥边界
- Story 9.14: 整理 OS、SchM 与固定内核责任链
- Story 9.15: 整理 RTE、共享类型与模块生成阶段
- Story 9.16: 验证主线交付与离线工具
- Story 9.17: 让日常修改获得快速有效反馈
- Story 9.18: 优化实际 CI 等待与重复工作
- Story 9.19: 让严格源码结果真正阻止错误 PR 合并
- Story 9.20: 完成非实现者独立接手与 Epic 出口

## Requirements & Constraints

- 只保留一条生成主线：SWC 数为 1 与多于 1 走同一计划与 emitter。帧／信号 host 链路（含“主机运行”页）、单组件链路和外部 XSD／MOD 校验模式全部删除；`schema.rs` 只作为测试和 acceptance 中的官方对照基准。
- 稳定版本发布前不保留旧 profile／旧格式工程的兼容；旧工程和旧交付包明确拒绝，不静默误解析、不改写。契约变化仍审阅 ABI／第三方身份并写入 CHANGELOG，资产摘要不能追认。
- 产品、测试、fixture、工具、bin、目录名、测试名和读者文档不含 Epic／Story 编号，由 `quality` 检查强制；不用 multi、legacy 这类相对命名。历史 BMad 记录不改。
- 仓库工具只分三类：交付工具、统一开发 CLI（`doctor`、`verify`、`quality`、`c-check`、`assets`）、正常运行器中的测试。桌面端到端优先用 Tauri 官方 WebDriver 与 runner 级隔离。
- 主线不依赖官方档案的原生行为全部进入必需 CI；手动 acceptance 只保留确需授权档案或真实环境的测试。
- 固定 CP/FO R24-11、C99、MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2。逐模块核对公开接口、配置、状态、错误、回调、上下层及适用描述／MemMap。自动分析、诊断裁决与 221 项适用性／必要人工评估分开，采用代码不豁免；真实违反未关闭即阻塞，本轮不批准偏离或规则降级。
- 模块开工前补齐合法原文、实际差距、独立成功／拒绝／恢复预期、工具与 ABI 审阅；缺依据保持未核查，历史绿色 CI 不等于源码通过。
- 每次 PR 源码门要求选集完整、error=null、passed=true、上游全部 success；违规、漏扫、重复身份、工具失败及 skip/cancel 均阻断实际合并，不缓存扫描通过结果。

## Technical Decisions

- ARXML/manifest 是持久权威，私有不可变计划核定语义；emitter 只消费计划。每个输出文件只有一个生成方，跨模块内容在计划阶段汇总；emitter 不回读或修补已生成的 C，不 panic，计划无法保证的前提返回 `PlanDiagnostic`。生成与集成模块由 clippy deny `unwrap_used`、`expect_used`、`panic`。
- 模块拥有协议状态，RTE 拥有 local 初始／发布数据，OS/backend 拥有调度，ECU 执行适配拥有批次、输出与收据。标准公开、私有及 host/test 接口分离；真实异步共享状态保留 SchM 保护，故障控制仅在测试构建。
- 保持固定 FreeRTOS、唯一 backend、SC1 与受控 tick；target、main 分别核验，平台证据不互借。
- 建议目录（依据见 Epic 9 spine 的 Structural Seed）：根目录虚拟 manifest 的 Cargo workspace（成员 `core`、`src-tauri`，位置不变，不拆新 crate）；主线 BSW 合并为扁平的 `runtime/bsw/{include,src}`，标准文件名按 SWS BSW General 不变，`_Cfg`／`_PBcfg` 只在生成工程中；`integration/multi_*.rs` 去前缀按职责命名；内置模板移到 `core/resources/`；`core/tests` 按运行条件收成普通、native、official 三个测试二进制，内部按能力分模块。最终名称由 9.26 裁定。
- 生成工程按所有权分目录（9.28）：`bsw/`、`os/`、`kernel/` 只读；`generated/` 每次整体重写（含 MemMap 头与 SWC 骨架 `templates/`）；`app/<SWC>/` 是用户代码快照；`integration/` 放适配；`meta/` 放台账。从 `target.json` 生成 CMake 描述，与 `ecu-tool` 用同一清单；MCU 目标的 MemMap 可由集成层通过显式配置覆盖。只有持久化或交付格式带版本，并登记在格式版本表中。

## UX & Interaction Patterns

新建工程不再提供“空 CAN”“CAN 信号”模板（需要空白起点时提供主线最小模板）；“主机运行”页和设置中的 XSD／MOD 项删除；打开旧工程时给出明确拒绝提示。其余保留统一壳层、明暗主题、草稿保护、保存／生成预览及异步结果归属；界面改动按 DESIGN.md，帮助与 README 同步。实际影响 UI/IPC 时用隔离原生入口取证，不以 mock、浏览器或构建替代。

## Cross-Story Dependencies

9.21 风险最高，先做；9.22 依赖 9.21；9.23 依赖 9.21、9.22，删除前先把有价值的独立预期迁到主线；9.24、9.25 依赖 9.23，可并行；9.26 依赖 9.23、9.24；9.27 依赖 9.26；9.28 依赖 9.27。新基线达成后才进入 9.3–9.15 模块整改，模块按实际驱动、执行、路由、传输、诊断、持久与调度依赖推进。9.16 汇总主线交付，9.17/9.18 调查可提前，9.19 等待源码、人工及反馈／性能结果，9.20 由非实现者新目录复验全部成果。涉及原生 fixture、交付布局或共用路径辅助的 PR，合并前在本地 Linux 与 Windows 运行受影响原生套件并请维护者触发 acceptance。R11 消费者实施等待本 Epic 出口；Epic 10 复核后续累计成果。
