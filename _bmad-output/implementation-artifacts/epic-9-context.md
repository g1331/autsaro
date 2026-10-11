# Epic 9 Context: 运行时、生成 C 与开发工程体系整顿

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让运行时与生成器维护者沿明确的模块契约、状态归属和生成来源维护全部现有交付工程，获得可信、高效的开发反馈；让接收者获得完成适用标准整改、独立行为回归并受严格 PR 源码门禁保护的工程。继承 Epic 4／7／8 已交付成果，不重开历史任务，也不把规划完成视为实施或符合性通过。

## Stories

- Story 9.1: 固定共享契约与逐模块进入依据
- Story 9.2: 分离 CAN 标准接口与主机适配
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
- Story 9.16: 验证兼容交付、旧工程与离线工具
- Story 9.17: 让日常修改获得快速有效反馈
- Story 9.18: 优化实际 CI 等待与重复工作
- Story 9.19: 让严格源码结果真正阻止错误 PR 合并
- Story 9.20: 完成非实现者独立接手与 Epic 出口

## Requirements & Constraints

- 覆盖 legacy host、单组件、多组件工程，Windows／Linux 受控目标及各实际 main 的完整程序闭包；包括生成来源、配置、交付工具和固定内核／port／patch 的直接依赖。不能缩成演示样本；不新增 MCU、未选网络、全部可选变体或界面重设计。
- 固定 CP／FO R24-11、C99、MISRA C:2012 Third Edition＋AMD1–AMD4＋TC1–TC2。逐模块核对公开接口、配置、状态、错误、回调、直接上下层及适用 BSWMD／MemMap；自动分析、诊断裁决与 221 项适用性／必要人工评估分别取证，采用代码不自动豁免。真实违反未消除即阻塞，无规则降级、批量 suppression 或本轮偏离批准。
- 模块进入前须有合法固定原文、具体差距、独立成功／拒绝／恢复向量、工具及兼容审阅；缺件只阻塞对应裁决。整体就绪仍为 CONCERNS，规划 backlog 不自动成为 ready-for-dev。
- 保留历史失败：基线 CI 38038608981 的双平台六样本十二摘要均为 error=null、passed=false，CI 绿色不代表源码通过；严格步骤当时仅手动运行，必需保护不含 c-analysis。该记录不是本轮重扫；诊断数不是已裁决违反数。历史 PDF 打开／TLS 下载失败及合法正文缺失不能写成资料已齐，全文审计、人工评估、原生与 MCU 未验证结论继续保留。
- 从固定规范独立定义预期，使用正常 npm／Cargo／uv、autosar_tooling 和离线入口；实际重新生成受影响工程，按目标与 main 编译、链接、运行和 c-check。快速测试、模块、进程、原生、安装包与独立交接各自证明，缺工具记未执行；删除重复须先证明不丢独立风险覆盖。
- 严格 PR 必需门同时要求完整选集、error=null、passed=true 和上游全部成功；违规、缺失、重复、skip／cancel、工具／解析失败均不得通过，并以实际故障 PR 验证阻断。不得缓存源码通过结果；CI 优化须有同覆盖、同工具身份的完整运行实测，新增门禁成本与优化收益分开核定。

## Technical Decisions

- 保持分层编译与端口／适配器路线：ARXML／manifest 是持久配置权威，私有不可变验证计划核定语义，模块 emitter 只消费计划；不在最终 C 文本替换中重新决定身份、类型或配置。跨模块声明、类型与 callback glue 必须有唯一 producer 和独立 consumer。
- 每个模块拥有自身状态，RTE 拥有 local 初始／发布数据，OS/backend 拥有调度，ECU 执行层拥有 batch admission、输出及 receipt。标准公开、模块私有、host／测试控制分离；真实异步共享状态保留 SchM 锁协议，BSW 不直接新增平台线程或 FreeRTOS 依赖。
- 继承固定 FreeRTOS V11.3.1 与唯一 backend、单核 SC1、受控 tick／owner 边界，不重新选型或新增调度器。profile、target 与程序身份独立；互斥 producer 不混合链接，Windows 证据不替代 Linux。
- 迁移同步源码／模板、配置定义、调用方、ABI、资产及离线工具。公开签名、布局、句柄或 wire 变化先审兼容并取得授权；资产摘要不能追认契约变化。保留原字节、安全保存、live／sealed 分离、create-only 初始化、用户源码和旧 seal，不静默升级旧包；生成前继续拒绝符号碰撞与同 event 周期／phase 不一致。

## UX & Interaction Patterns

不新增界面设计。保留已有草稿、异步忙态、保存／生成预览及确认失效规则：预览不写入，生成不隐式构建，配置操作不依赖官方档案或编译器。实际影响保存／IPC 时使用既有隔离原生入口，等待对话框可操作、忙态解除及后台事务完成，不能以 mock 或固定延时替代。

## Cross-Story Dependencies

共享契约先固定，接口变更随 owner 同步全部直接消费者并保持工程可运行，不留待未来故事接线的空壳。驱动／执行接缝支撑通信，路由／传输支撑诊断，持久接口支撑故障与安全消费者；OS／SchM 与 RTE 消费已固定模块契约后完成兼容交付。开发反馈与 CI 调查可并行，实际迁移等待对应契约／热点证据；严格门依赖模块整改、兼容、人工及性能结果，独立出口依赖全部成果。9.14–9.16 进入前须核定粒度，必要时按已确认机制拆小且不削减验收。R11 消费者实施等待本 Epic 完整出口，研究可提前；Epic 10 承接后续累计验收，不接收本可在主机关闭的软件差距。
