---
id: SPEC-multi-component-scheduling
companions:
  - application-contract.md
  - acceptance.md
  - compliance-references.md
  - ../../planning-artifacts/architecture.md
  - ../../planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md
  - ../../planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md
sources: []
---

# 多组件应用与调度集成

## Why

候选 R6 将现有单应用参考 ECU 扩展为可交接的多组件应用工程。集成工程师需要从真实 SWC、连接与调度输入获得共同生成的接口与运行工程；应用开发者拥有算法源码，重新生成不能要求手改产品协议栈或覆盖用户代码。对应 PRD APP-1、APP-2、CFG-4 与 FR-1–FR-6、FR-12、FR-14、FR-15。

## Capabilities

- **CAP-1**
  - **intent:** 使用者描述并安全编辑 SWC 类型、实例、组合、端口、类型、连接与 ECU 映射。
  - **success:** 多文件输入保存重开保持原始来源和有效引用；悬空、方向、归属、类型及多生产者冲突在生成前定位拒绝。
- **CAP-2**
  - **intent:** 多个应用通过生成接口交换显式 S/R 数据和同步 C/S 请求。
  - **success:** 实际链接的应用按独立向量完成生产、消费、扇出和同步服务调用；初值、返回状态与拒绝时输出保护符合所选契约。
- **CAP-3**
  - **intent:** runnable 与 event 的配置经共同生成的 RTE／SchM 和 OS 调度运行。
  - **success:** 实际受控 tick 按映射位置执行每个到期 runnable 一次，同步服务器只在调用者上下文执行；非法任务、重复位置、周期和触发器矛盾明确拒绝。
- **CAP-4**
  - **intent:** 应用开发者维护各组件的 live 源码，重新生成并独立交接整个工程。
  - **success:** 初始化只创建新文件；源码改动使旧确认失效；再生成、异地重导入、构建和运行逐字节保留所有用户源码，封存篡改拒绝。
- **CAP-5**
  - **intent:** 使用者保留既有工程行为，并可在后续 CAN 扩展中明确连接应用。
  - **success:** 旧单组件与 host profile、交接格式和行为回归通过；接缝保留实例／端口／元素、网络／通道身份，不借本次证据声明 R11 已实现。

## Constraints

- CP／FO R24-11、C99、既定单核 SC1／固定 FreeRTOS 路线；配置权威为原始 ARXML，不增设持久模型或第二个调度器。
- 首批支持配置与独立预期见 companions；新 multi 选择 IMMEDIATE COM、唯一 partition／Core0；完整诊断路由可省略应用 DID，两条诊断路由均不选则为 network-only。未支持的生成语义明确拒绝，无关有效内容原样保留。
- 先验证真实多组件接口与 OS 调度闭环，之后才展开编辑和交接；独立预期从官方契约与产品向量定义，不从现有实现反向生成。
- Epic 7 的发行收尾由另一任务负责，不作为总门禁；只定位影响本任务的具体依赖缺陷。保留其他任务的修改，共享接口修改由当前协调者统一处理。
- 修改 C／生成模板前应用 misra-c2012；受影响实际生成 profile 必须运行 c-check，并区分扫描完整性、源码诊断和人工规范评估。
- 2026-10-10 用户撤回 8.5 延期安排，本次继续完成整个 Epic 8：先完成 8.1–8.4 阶段的提交、推送、PR、必需 CI 与合并，再继续 8.5 的独立多组件异地交接及其验收和交付。全部必需验收完成前 Epic 8 保持进行中。结果回到 BMad 规格和故事，不另建报告或台账。

## Non-goals

- 车辆算法、硬件验证、认证、重新选择 OS、Epic 7 发行收尾与 R11 实现。
- 同类型重复实例、嵌套组合／delegation、跨任务应用共享、队列、隐式通信、模式、异步 C/S、跨 ECU C/S、transformer、未选定变体；这些配置不得静默降级。

## Success signal

另一执行者从多组件原始工程得到确定的完整源码，在新目录独立构建并通过真实 OS tick 复现 S/R、同步 C/S、CAN／DID、拒绝与源码保护向量；已有工程行为保持，全部结果能定位到实际输入、产物、目标与执行命令。


## 实施与交付结果

2026-10-10，正式 Epic 8 的五条故事全部完成。首批组件／连接／调度、共同生成、用户源码保护、真实工作台编辑及独立交接已通过对应冻结验收与独立审查；[PR #15](https://github.com/g1331/autsaro/pull/15) 和 [PR #16](https://github.com/g1331/autsaro/pull/16) 各 28 项检查通过并已合并，master `e3b14a3c5e39386cc07ae06280d58b4f83f00943` 包含完整实现。结果落点为[验收规格](acceptance.md#2026-10-10-首批配置验收结果)和既有五份故事规格，sprint 的 Epic／故事均 done。历史延期安排与阶段失败保留，当前不再是派发依据。

实际 C 分析完整性已验证，源码 passed=false 和未完成的人工规范评估仍保留；完成不代表 Windows 交接运行、安装包、MCU、硬实时或认证通过。R11 仅核对既定信号类型、通道与应用映射接缝，运行扩展未实施。

2026-10-10 完成后的[复盘复评](../../implementation-artifacts/epic-8-context.md#epic-8-复盘2026-10-10)实际发现参数名遮蔽运行时函数和不同周期共享OS event两个缺陷，整改前机器验收判定为 `rejected`，该历史记录保留。按两项行动完成模型修复、独立审查和正常入口复验后，当前判定为 `accepted`，现有sprint两项行动均done；详细证据见[授权整改复评](../../implementation-artifacts/epic-8-context.md#2026-10-10-整改复评与验证结果)。此前交付／通过／失败保持历史事实，五条故事done不自动回退；R11调查可继续，这两个具体生成／调度缺陷已关闭，整改交付关联[PR #18](https://github.com/g1331/autsaro/pull/18)；R11调查可继续，实施仍须按自身范围检查就绪。
