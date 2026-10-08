---
title: '免费 MISRA C 检查与双平台 CI'
type: 'feature'
created: '2026-10-08'
status: 'in-progress'
route: 'dispatch'
baseline_commit: 'de64a1ece8ad6529607ac48d81d7a06d2137c75e'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="用户已批准对话中的完整方案并要求实施">

## Intent

**Problem:** 当前正式质量入口未启用 MISRA 扫描，现有 Cppcheck 只检查少量生成 RTE 文件，绿色开发检查不能证明完整交付 C 的编码质量。

**Approach:** 复用正常生成、封存和构建参数，增加免费的 Cppcheck 2.21.0 MISRA／项目规则检查及 Windows、Linux CI。先修复真实违规，再启用严格门禁；人工义务与工具覆盖分别记录。

## Boundaries & Constraints

**Always:** C99；C:2012 AMD1–AMD4／TC1–TC2 为核查基线；完整源码清单、目标模型、宏和头文件；生成源整改与实际行为回归；分析不完整非零；原始诊断保留；输出与源码分离；正式偏离须批准。

**Never:** 修改 AUTOSAR 公开 ABI；把主机结果写成 MCU／认证通过；批量 suppression 或吸收历史问题制造通过；更改固定内核；购买工具；推送或修改远端保护规则；首期接入 sanitizer；新增独立 assurance 状态体系。

## I/O & Edge-Case Matrix

| 场景 | 输入／状态 | 预期 | 失败处理 |
| --- | --- | --- | --- |
| 合法工程 | 对应目标的封存源码、有效构建清单 | 完整扫描；日志及工具身份可读；源码不变 | 返回真实检查状态 |
| 真实违规 | MISRA 或项目规则反例 | 文件／位置／规则可定位 | 非零，保留其他样本诊断 |
| 工具失败 | 缺工具、版本不符、addon 崩溃、超时 | 不冒充零诊断 | 非零并保存原因 |
| 不完整 | 空扫描、漏源、缺头文件或解析失败 | 明确未检查范围 | 非零 |
| 输出冲突 | 源目录内输出、非空目录、符号链接 | 不写、不覆盖 | 拒绝 |

</frozen-after-approval>

## Code Map

- `tools/python/src/ecu_tools/build.py` — 共享只读目标源码、编译参数和 include 提取；保持严格封存／目标检查。
- `tools/python/src/autosar_tooling/cli.py` — 增加 c-check；检查器位于同包，不创建独立编排工具。
- `core/tests/` — 独立集成测试调用内置 can-signals-v1／standard-ecu-v1 和 prepare/generate 公共 API；不依赖官方资源。
- `.github/workflows/checks.yml` — 现有 PR／push／manual 入口增加双平台分析。
- `.agents/skills/misra-c2012/` — 221 项指导；不是完整规范副本或自动通过证明。

## Tasks & Acceptance

**Execution:**
- [x] 构建参数共享与生成样本：源码／宏／include／应用入口来自真实目标；标准示例、用户应用、诊断及容量／布局边界。
- [x] c-check：工具身份、分离输出、封存完整性、kernel 补丁、MISRA／项目 addon、原始 XML／日志与覆盖。
- [x] 项目 addon：动态分配直接调用与普通 BSW 创建线程；独立项目编号，不伪称完整 MISRA 检查。
- [x] 正反及边界测试：真实扫描违规、失败、路径拒绝、不完整、源码无污染。
- [ ] 完整扫描与整改：自有通用运行时／OS／host／生成 C，采用代码单列，必要偏离保留批准状态。
- [x] CI／开发文档：固定工具 2.21.0、双平台、每样本继续收集、always artifact 14 天；首期不路径跳过。

**Acceptance Criteria:**
- Given 合法生成工程，when c-check，then 使用真实目标配置分析完整声明源码，源摘要不变。
- Given 任一违规、工具错误或不完整分析，when 本地／CI，then 返回非零并保存诊断，不被后续报告覆盖。
- Given 每个项目规则，when 违规／合法／宏与 typedef 边界测试，then 诊断准确且不误判声明或注释。
- Given 当前实际代码，when 收尾，then 修复已证实问题，适用义务／未检查／必要偏离分别记录；未批准不写通过。

## Implementation Notes

- 用户明确选择首期 MISRA、整改后启用门禁；沿批准计划直接实施。用户 AGENTS.md 要求主代理承担代码设计与实现，因此不将整个实现交给子代理；只读生成入口探查已完成。

## Verification

- `uv run --locked python -B -m unittest discover -s tests/python`。
- `uv run --locked ruff check tools tests/python scripts`。
- 新 C 分析样本集成测试与 c-check 完整实际执行；受影响 C 回归、资产检查及增量 quality。
- 无官方资源、输出冲突、分析前后 files.sha256、缺工具／解析失败和真实违规验证。

## Review Triage Log

| 来源 | 判定／处置 | 证据与结果 |
| --- | --- | --- |
| Blind reviewer | 无确认缺陷 | GCC 验证系统头文件、Cppcheck 使用库模型是有依据的不同职责，未将标准库未直接解析列为缺陷。 |
| Verification：真实违规返回码 | medium／patch | 原真实工具测试丢弃 check 返回码，存在摘要失败但调用方返回 0 未被捕获的验证缺口。现在每次实际调用断言 code 与 passed 一致，MISRA 反例走正式 CLI 并验证非零。 |
| Verification：完整 ECU 分析集成 | medium／patch | 单文件 host 反例没有覆盖 ECU 补丁与补充源码。CI 现在每次对五类实际样本完整运行 c-check，拒绝 error、不一致返回码及漏扫，保存真实源码失败结果；严格 source passed 门禁仍按整改前不启用的决定只在手动运行执行。 |
| Final Edge：第二程序回执结构 | medium／patch | 有效 JSON 的错误结构可能抛出 TypeError 并遗留首程序绿色摘要。现在显式验证 units／configurations，并在全部程序与封存核对结束后才设置总体 passed；新增真实 helper 路径的第二程序错误回执回归。 |
| Linux Edge：动态宏未知展开 | medium／patch | 不识别的 SIGRTMIN／PTHREAD_STACK_MIN 展开明确失败，新增实际编译器结果注入反例。 |
| Linux Verification：必需容器回归 | false-positive | 实际删除 DcmDspSecurity 修复后，现有 c_analysis 测试以 MULTIPLICITY 失败；恢复后通过。现有公共 API 测试已经捕获原错误。 |
| Edge：线程宏／typedef | medium／patch | 动态分配已有宏与 typedef 反例，线程创建没有同类用例。增加 typedef 返回类型与 SPAWN 宏别名的实际线程反例，保持声明、注释、host adapter 和 SWC 非 BSW 对照。 |


## Spec Change Log

## 当前证据与剩余验收

- 新入口已完成 Windows 实际执行，Cppcheck／三个 addon 身份固定。GCC 逐 TU 验证真实头文件及语法，提取目标宏及编译器展开的系统数值宏；按 Cppcheck 官方建议使用标准库模型，不解析 GCC 标准库实现。工具发现诊断或配置不完整时保持非零。
- 全部五类工程从公共 Workspace／prepare／generate API 生成。无安全诊断配置漏必需 DcmDspSecurity 空容器的问题已修复；不改变安全功能与 C ABI。容量样本以稳定名称顺序覆盖 32 帧、64 信号和最高位／32 位布局。用户应用样本实际修改并重新打开 SWC 文件。
- 已修复 CanIf 末尾分支说明、容量零值 essential type、主机十六进制字符／数值混用、安全状态字节标记与比较、内存操作返回值用途及 BCrypt 哈希输入 const 丢弃。所有公开签名、持久化 SEC1 字节和密钥域保持一致；资产更新仅吸收经过审阅的自有源码变更。
- 普通 Python 测试 41 项通过；真实检查器测试 Windows 10 项通过、2 项 Linux 专用跳过，Linux 12 项通过。默认 Cargo 测试通过（17 lib、69 builtin、1 样本）；CAN 接口 2 项、SecurityAccess 2 项、DTC 3 项、诊断 roundtrip 1 项真实 native 测试通过；批处理 codec、受控 counter/alarm、嵌套中断回归各 1 项通过。第一次 SecurityAccess 因未设置 AUTOSAR_CC 在启动前失败，设置进程级绝对工具路径后通过。增量 quality、ruff、assets check 通过。工作流 actionlint 1.7.12 检查通过。
- 上一轮完整扫描：Windows 五类样本为 283／289／283／1842／1842 条诊断，Linux 为 225／231／225／1826／1826 条，均 error=null。前三类各 21 TU，后两类各 42 TU。计数包含 checker 提示、advisory 和需核对的模型报告，不能当成已证实违规数。原始结果位于 build/c-analysis-complete-results 与 build/c-analysis-linux-complete；最新分链接分析双平台五类均完成。
- CI 代码已加入 Windows／Ubuntu 矩阵，固定源提交构建，timeout 30 分钟，逐样本继续收集，always 上传结果／样本 14 天。按用户“整改后启用门禁”的决定，自动执行检查器反例、生成与完整分析链路，拒绝漏扫／配置／解析不完整；源码诊断仍保留非零与 passed=false，严格源码门禁暂为 workflow_dispatch。尚未推送、未运行 GitHub CI；Linux 已在现有 Ubuntu WSL 上实际生成与分析。尚未完成所有历史诊断整改，不能把本任务标记 done，也不能声称 MISRA 完整符合。

## 必要偏离草案（未批准）

以下草案不会产生 suppression，不会把当前失败写成通过，也不覆盖其他资源／错误处理规则。其批准只能由项目责任人作出。采用的第三方内核仍须独立评估，不能由这些自有代码草案豁免。

| 规则 | 范围 | 必要行为与理由 | 继续有效的约束 |
| --- | --- | --- | --- |
| R21.6 Required | runtime/src/NvM_HostStorage.c、runtime/src/Security.c 的持久化；runtime/src/ecu_host_main.c、runtime/ecu/src/ecu_host_batch.c、runtime/ecu/src/ecu_probe.c 的主机流协议；runtime/os/src/Os_Backend.c、runtime/os/src/Os_Stack.c、runtime/os/src/host/linux/Os_StackLinux.c 的主机关闭／故障输出 | 当前受控主机产品使用标准文件与流协议。全面禁止 stdio 会取消既有离线接收与持久化能力；将其留在受控主机边界。仅适用于 windows-x64-controlled-v1／linux-x64-controlled-v1，不能扩展到 MCU BSW。 | 调用边界、参数域、文件生命周期、实际失败传播与格式参数仍逐项核查；不批准丢弃 I/O 错误。安全 BCrypt 功能只适用 Windows。 |
| R21.8 Required | runtime/src/Can_HostLock.c 中三个剩余 abort 点：Windows Can_Lock 初始化失败；Linux Can_Lock 初始化／阻塞锁失败；Linux Can_Unlock 解锁失败 | 非阻塞 Can_TryLock 已按 1／0／-1 区分获得／忙／错误，Can_Write 返回 E_NOT_OK，主机发送返回 ECU_ERR_CONTROLLER；初始化 callback 清理资源后报告失败，不直接终止。剩余 void 锁边界返回后调用方会继续访问共享状态，或释放失败后锁所有权不可靠，因此保留失败终止策略。 | 仅限主机锁实现列出的三个位置；不得推广至业务错误或 MCU。单点失败注入验证选择终止端点且不返回，不冒充实际 CRT abort 的清理验证；终止可能丢失未持久化数据，禁止在锁健康状态未知时尝试重入式清理。正式批准仍待责任人确认。 |

其余真实 Required 问题继续按源代码整改；Mandatory 问题不能申请偏离。Windows API 原型相关 R17.3 报告必须结合真实编译头文件核查模型，不能统一忽略。Advisory 报告按意图与维护成本逐项处理，不能整体降级或静默丢弃。D1.1、D3.1、D4.1、D4.7 等过程和设计义务以及工具无 checker 项仍未完成全面评估，需核对授权规范、需求、设计及人工证据。

### Linux 平台模型补齐

Ubuntu WSL 实际测试发现 glibc 的 PTHREAD_MUTEX_RECURSIVE 枚举、SIGRTMIN 与 PTHREAD_STACK_MIN 动态宏不在开放版 Cppcheck 默认模型中。已补齐一个有限的分析前置头文件：GCC 对枚举值和两个函数指针原型进行独立编译核查；动态宏使用编译器实际展开的 __libc_current_sigrtmin／__sysconf 调用，不替换为虚构常量。模型、核查源码、日志和 SHA-256 保存在每次结果中。模型不兼容仍会失败；这不是规则 suppression。

WSL 的 Windows 挂载目录不能提供 Linux 私有日志目录权限，第一次完整分析因此在启动前拒绝。改用 Linux 文件系统结果目录后执行，并将实际结果副本保存到 build/ 下；未放宽 OwnedProcess 的权限检查，也未修改挂载或全局设置。Cppcheck addon 检出时的换行转换由 Git 原始 blob 恢复，严格哈希不变；CI 已采用同样处理。

### 最新整改与链接范围

GCC 标准整数宏实际后缀进入 Cppcheck；Windows setvbuf 模型使用实际三种模式，合法／非法模式回归通过。Dem／NvM 字节常量、Com 与 runtime/OS 位移操作数宽度已整改；Ecu_HostBatch 文本比较使用有界 strncmp，字符算术在计算前选定整数域；Os_Time 使用统一全零初始化。

两个独立 ECU main 原先被一个 CTU 集合误判为重复定义。现按 build.py 的 HostBatch／legacy entry 分别运行完整 CTU 分析，分别核对 TU 和 addon receipt，保留原始结果与退出码。摘要合并相同位置的重复诊断并标明所属程序。新增回归保证其中一个程序失败时总结果仍失败。

AGENTS.md 新增实际 profile 扫描及分析完整性／源码诊断／人工义务分别报告的要求，并要求资产摘要更新前核对 .gitattributes 的字节换行。公开 ABI 与内核身份未变。

### 需要继续核实的模型报告

Cppcheck 2.21.0 addon 的 is_errno_setting_function 白名单不含 fopen，且遇到右花括号会清空最近调用。NvM_HostOpen 在 fopen 返回空指针时判断 ENOENT 的代码因而报告 R22.10。POSIX 的 [fopen 契约](https://pubs.opengroup.org/onlinepubs/9699919799/functions/fopen.html) 明确失败设置 errno；[Microsoft CRT 文档](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/fopen-wfopen?view=msvc-170) 也要求空返回时检查 errno。该处属于已定位的平台契约／checker 模型差异，不通过改掉真实错误判断制造通过；当前保留原始报告，无 suppression。其他 errno 位置仍须分别核对。

R8.6 的两个 main 在分程序 CTU 检查中应消失；R17.3 平台 API 原型模型、Required 平台偏离和其余 OS 类型／返回值／资源问题仍未全部验收。严格源码门禁与完整 MISRA 声明均未启用。检查器可用性目录中 155 项有固定版本 checker，66 项没有；这不是155项完整自动覆盖或通过证明。

### 分链接分析复核

Linux 最新五类样本完整分析均 error=null，诊断为 225／231／225／1817／1817；Windows 最新五类分别 283／289／283／1833／1833，均 error=null。两个 ECU 入口的 R8.6 重复 main 报告已经消失，原始诊断保留在各程序目录。新增回执结构缺陷经 edge reviewer 源码／回归复查确认关闭；blind 与 verification 最终轮没有新增确认缺陷。12 个修改的交付源码原始字节与 Git checkout filter 一致，assets check 通过。

### 编译器验证耗时与有界并行

Windows 42 TU 的串行预处理占据 CI 主要耗时；语法验证及宏提取现使用标准库 ThreadPoolExecutor，最多两个 worker。探针和日志以 TU 索引独立命名，worker 仅返回自己的宏与 stdio 模式，主线程按清单顺序收集；跨 TU 模式不一致明确失败，异常取消排队任务并等待活动受管进程收尾。edge／verification 只读复核未确认新缺陷。

新增实际双 TU 正反测试：两个单元各自保留真实标准整数宏；其中一个单元非法 C 时整体失败。Windows 10 项通过、2 项 Linux 专用跳过，Linux 12 项通过。Linux 完整 42 TU 的并行与串行扫描宏、源码诊断完全相同（1817 条），Windows 同范围对照亦完全相同（1833 条）；实际双 TU 正反测试双平台通过。保持 CI 30 分钟限制；尚无 GitHub hosted runner 的运行证据。

### 当前完成边界

检查入口、共享构建参数、真实五类样本、项目 addon、双平台 CI 配置、开发说明及本轮已证实热点整改均已落地并复核。默认 Cargo 17 lib／69 builtin／1 样本通过；普通 Python 41 项通过，真实检查器 Windows 10 项通过＋2 项 Linux 专用跳过，Linux 12 项通过。增量 quality、ruff、assets check、actionlint 与 git diff --check 通过。

当前任务仍 in-progress：全量历史诊断、采用内核与平台模型评估、人工义务及 Required 偏离批准尚未全部完成；自动 strict source gate 未启用。两项已写出的主机偏离草案仍未批准，不产生 suppression。没有提交、推送或 GitHub CI 运行证据。公开 ABI 和第三方身份未变，受信资产仅更新已审阅源码摘要。

### 偏离草案规则修订纠正

已核对 MISRA 官方 [AMD2 第 2.10.41 节／AMD2.81](https://misra.org.uk/app/uploads/2021/06/MISRA-C-2012-AMD2.pdf)：R21.8 已改为标准库终止函数，列举 abort／exit／_Exit／quick_exit。getenv 不应按本项目修订基线申请 R21.8 偏离；原草案将它列入是错误，已移除。

固定 Cppcheck 2.21.0 的 addons/misra.py 中 misra_21_8 仍检测 abort／exit／getenv，表明该项 checker 与修订基线不一致。原始诊断保留，不修改固定 addon 或添加 suppression。getenv 仍须核对 R21.19／R21.20 的只读与返回存储生命周期，以及适用并发和环境输入义务；调用本身不自动证明违规或合规。纠正草案时，故障注入环境读取仍位于正常主机构建的 Os_BackendStart 内；下述主机处置已将资源／栈两个入口限制到显式测试构建，并通过实际离线构建验证。

abort 草案先由“仅初始化失败”纠正为实际存在的初始化、互斥属性操作及部分加锁／解锁失败范围；下述主机处置进一步将可返回错误的路径改为返回，并把剩余范围收窄至三个 void 锁终止点。风险及失败路径证据已记录，草案仍未批准，不能一揽子豁免。

## 主机偏离处置（2026-10-08 用户授权按推荐方式处理）

- [x] NvM_HostClose 私有接口返回关闭状态；流在关闭调用前失效，关闭失败阻止重新打开和 NvM_Init 假成功，不修改公开 NvM API。
- [x] Security 持久化统一检查关闭结果，仅完整写出、刷新、同步及关闭成功后更新尝试计数；读取状态前清零 errno。主机入口检查 setvbuf 与协议／配置错误输出，OS 栈／关闭报告错误传播为非零退出，保留已有首要故障原因。
- [x] Can_TryLock 区分 busy 与同步错误并使用现有 CAN 返回通道；初始化回调负责属性资源清理，只在无法安全返回的三个 void 锁位置保留 abort。Windows 改用可返回失败的 InitializeCriticalSectionEx，参考 [API 契约](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-initializecriticalsectionex)。
- [x] 资源获取和栈保证两个环境故障入口均限制在 OS_HOST_FAILURE_TESTS；原有 native harness 和显式离线 test mode 开启，生产 HostBatch／probe 均不开启。生命周期 suite 验证正常生产构建忽略两个环境变量，测试构建仍逐个资源／栈故障拒绝，关闭 stdout 后退出非零。
- [x] 新增 tests/python/native/test_host_boundaries.py 与测试专用 C 调用替换 fixture，双平台各 4 项通过；CI 显式运行这些测试，交付源码不包含替换 hook。
- [x] 完成本轮重生成工件检查和独立审查，回写实际结果。

R21.6 保留限定主机 I/O 的申请，R21.8 仅申请上述三个终止点；两项仍是未批准的正式偏离草案，不产生 suppression。用户本次授权为按推荐方式整改／完善申请，未将其写成未限定范围的全部偏离批准。getenv 已从申请移除，检查摘要 knownCheckerLimitations 记录固定 addon 与修订的差异，原始诊断不改动。AGENTS.md 增加偏离前核对 amendment／TC 与实际调用路径的要求。

本轮 Windows／Linux 锁与文件错误边界各 4 项、OS 生命周期各 50 向量通过；Windows native CAN 2 项、SecurityAccess 2 项、DTC 3 项通过。双平台 native stack／ARTI 回归通过（Linux stack 23、ARTI 35 向量）；默认 Cargo 17 lib／69 builtin／1 样本、普通 Python 41 项、ruff／quality／assets check／actionlint 通过。Windows 重新生成诊断样本 21 TU 完整分析 error=null，298 条诊断、passed=false；其中仅一个当前 Windows 主机锁 R21.8 终止点。Linux standard-ecu 42 TU 完整分析 error=null，1806 条诊断、passed=false；实际结果副本位于 build/c-analysis-host-policy-linux-results/standard-ecu。计数不等于已确认违规数；此前五类全量结果属于本轮主机处置之前的源码快照，不能作为新源码全部 profile 的复验。

新增 tests/python/native/test_ecu_build_modes.py 通过封存包实际离线 CLI 构建 HostBatch／probe／test，各执行正常、非法资源参数、首次资源失败和栈保证失败四种场景。Windows 1 项／12 场景（54.774 秒）、Linux 1 项／12 场景（10.701 秒）通过；生产协议完成，test 故障退出分别为 E_OS_VALUE=8／E_OS_STATE=7，并有对应原因输出。CI 在生成样本后显式运行，文档给出前置工具与入口；没有用仅检查编译参数代替行为验证。

| 本轮审查 | 判定／处置 | 证据与结果 |
| --- | --- | --- |
| Blind：互斥属性销毁掩盖初始化失败 | false-positive／撤回 | 当前代码仅在 attr_destroy 失败时改写 result=-1，成功不写 result；原始 settype／mutex_init 失败保持，ready=0。Linux 两种失败注入均通过，reviewer 逐行复核后撤回。 |
| Edge：主机失败边界 | 无确认缺陷 | 复核初始化资源清理、CAN 错误映射、NvM 流生命周期、首要关闭原因及两个测试环境入口，未发现确认缺陷。 |
| Verification：正式 ECU 构建模式未直接验证 | medium／patch／关闭 | 原 native fixtures 与 os_suites 构建器不能保护 ecu_tools.build 的模式分支。新增封存离线 CLI 三模式真实构建与十二场景执行，双平台通过；reviewer 复核测试、CI 和文档后确认关闭。 |

本轮主机处置已完成；完整免费 MISRA 检查任务仍 in-progress，历史诊断、人工义务、第三方评估和正式偏离批准不因局部整改完成而标记完成。上述验证与审查为提交前本地证据；GitHub CI 结果须以远端实际运行记录为准。

### 首次 PR CI 接入修正

PR #9 首次实际运行暴露两项接入问题：macOS 的临时目录经过 /var 符号链接，新检查器测试在调用前未取真实临时路径；Cppcheck 固定源的 cmake/options.cmake 强制输出到 PROJECT_BINARY_DIR/bin，工作流却查找额外指定的目录。测试现使用已创建临时目录的 resolve(strict=True)，未改变交付入口拒绝符号链接的策略；工作流取消被上游覆盖的输出参数，并从实际 bin 目录查找单配置／多配置产物。

修正后检查器契约测试 Windows／Linux 各 10 项通过；Linux 额外以经过符号链接的临时根运行同组测试通过，同时显式链接仍被 refuse_links 拒绝。ruff、actionlint 与差异检查通过。CMake 输出路径由固定上游源码与失败 runner 的实际链接日志交叉确认；本地 WSL 没有 cmake，未声称已在本地重建该 CMake 产物。更新后的 GitHub CI 结果以 PR 检查为准。

### 实际双平台 C 分析 CI 失败整改

PR #9 的 46e6f36f3cd9d4a4ca9b0dbefce1cffc241cf341 双平台失败已定位：Linux 分析副本位于 checkout 内，git apply 对选择路径加仓库前缀，--include=tasks.c 等不匹配后跳过且返回 0，Os_Backend.c 因缺少四个补丁接口声明编译失败；独立 Git 实验复现原始路径零返回码、文件不变，隔离后实际修改。Windows 的滚动安装得到 GCC 16.2.0 Rev4，与交付目标 GCC 16.1.0 Rev5 的固定身份不符，真实构建三模式因此被拒绝。

- [x] 静态分析、封存离线构建和 OS native harness 共用 kernel_patch_environment，在复制的 kernel 父目录设置进程级 GIT_CEILING_DIRECTORIES，不改宿主配置、原始源码或补丁选取范围。
- [x] 普通 Python 增加真实 Git 仓库内／外的选择补丁测试，断言选中文件改变、排除文件不变；三模式离线构建回归输出亦放在临时 Git 仓库内。
- [x] Windows CI 安装官方固定 GCC／gcc-libs 16.1.0-5 包，校验两个包的 SHA-256 并保留包签名；已从官方归档提取 gcc.exe，确认与 runtime/os/toolchain.json 的 d38d4dd6bea387499487881383e644ab7c193ac8f8364dc4252d6e6cc09700e2 完全一致，不修改目标锁。
- [ ] 完成全量 Linux 工件分析、独立审查和修复提交的双平台 GitHub CI 复验。

普通 Python 42 项、ruff／actionlint／assets check 通过；真实仓库内构建三模式／12 场景 Windows 58.967 秒、Linux 11.094 秒通过。资产只更新交付 tools/python/src/ecu_tools/build.py 摘要，公开 C ABI 和第三方内核身份不变。AGENTS.md 补充选择补丁实际应用与 CI 固定工具链要求。五类 Linux 工件分析正在实际执行，最终结果及独立审查回写下文。

Linux 五类均分析完整、error=null，诊断为 234／240／234／1806／1806；两个 ECU 样本各 42 TU，源码仍 passed=false。OS native lifecycle 50 向量亦通过。Blind、edge、verification 三层独立只读复核均无确认缺陷或验证缺口。相对 46e6f36 的增量 quality 通过（本轮只改 Python／CI，无新增 C 语法验证），资产校验与差异检查通过。

99d45c821c44127b43e565875827fb9ce9f0964f 的实际 Windows runner 已确认固定包下载、SHA-256 校验成功，但先装最新 GCC 再降级时，其拆分 cc-libs 元包阻止替换库。固定旧 gcc-libs 元数据提供 cc-libs，故调整为干净 MSYS2 初始化后同步数据库，直接用 pacman -U 事务安装两个固定包及其依赖；不预装最新 GCC，不删除依赖或跳过依赖／签名检查。远端双平台完整结果继续以最终修复提交为准。

8193e996919fb52607b753d671f081e12c77b1e7 的 GitHub Linux 完整 C 分析通过，Windows 固定 GCC 安装也通过，但原生检查器测试发现另一真实跨盘问题。早期原始 GCC 日志复现：在 C 盘临时目录执行时，搜索列表包含 /mingw64/include；在 D 盘仓库执行则不包含它。检查器原先在自己的 D 盘工作目录解析这个无盘符根路径，因此报 WinError 3。路径解析现共用 compiler_include_paths，以 GCC 的实际 cwd 为基准；不存在路径及非目录仍拒绝，GCC 逐 TU 验证、addon 身份、诊断返回码和扫描清单义务不变。

普通 Python 增加相对路径、绝对路径、Windows 无盘符根路径，以及缺搜索清单／不存在路径／非目录的回归，43 项通过；Linux 检查器契约 11 项通过，ruff／actionlint 通过。Windows CI 使用正常 uv 入口，在 C 盘临时目录先运行真实 stdio 预处理与同一解析器，尽早定位工具链错误；原生完整分析测试及最终 CI 仍须复验。AGENTS.md 增加工具输出路径按实际执行目录解析并覆盖跨盘临时目录的要求。

修正后真实 checker 回归 Windows 10 项通过＋2 项 Linux 专用跳过（29.258 秒），Linux 12 项通过（42.085 秒）；三个复核层均无确认缺陷或验证缺口。原始 GCC 报告根路径被解析到实际执行盘，未把它转换为虚构的 MSYS2 安装路径，也未忽略不存在的目录。远端复验使用修复提交的 PR 检查记录，旧提交的重复运行已取消。

5ce28b4ef611d9aeb57510b0aab15c6cd294c82c 的 GitHub 实际运行中，17 项必需检查和 Linux 完整 C 分析通过；Windows 固定工具链、跨盘头文件预检、真实 checker、主机失败边界、样本生成和三模式实际构建亦通过。Windows 全量扫描前三类各 21 TU 完整，292／298／292 条诊断，standard-ecu 42 TU 完整、1821 条诊断；user-application 扫描被 30 分钟 job 总限时中断。单个标准 ECU 分析实际约 10 分 32 秒，含冷启动依赖的五类完整范围超过原先估计，故仅将 Windows job 总时限调整为 45 分钟，Linux 保持 30 分钟；未改变 GCC／Cppcheck 的逐命令超时、输入范围、清单核对或失败条件。此前“保持30分钟”的记录对应实测前的预算，最终 CI 验收须以全部五类完成为准。
