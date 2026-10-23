# C 编码规范

适用于 `runtime/` 的 C 源码与头文件，以及 emitter 生成的 C；FreeRTOS 内核（`third_party/`）和用户应用（生成工程的 `app/`）保持各自风格。本篇只写项目约定，硬约束引用原文，不复述。

## 引用的硬约束

| 约束 | 依据 | 要点 |
| --- | --- | --- |
| MISRA C:2012 修订基线 | [ADR 0008](../adr/0008-misra-baseline-and-no-deviation.md)／[ADR 0023](../adr/0023-autosar-required-conflict-disposition.md)、[misra-c2012 指导](../../.agents/skills/misra-c2012/SKILL.md) | C99；偏离见下文 [MISRA 偏离注释](#misra-偏离注释)；`c-check` 的用法见[生成 C 的静态检查](testing/generated-c.md) |
| BSW 命名 | SWS BSWGeneral R24-11 | API `<Mip>_<Sn>`（`SRS_BSW_00310`）；主函数 `<Mip>_MainFunction[<Sd>]`（`SWS_BSW_00153`）；类型 `<Ma>_<Tn>Type`，平台基本类型除外（`SWS_BSW_00146`）；宏、枚举字面值与状态值 `<MIP>_<SN>`，全大写，`E_OK`／`E_NOT_OK` 除外（`SWS_BSW_00124`）；错误值 `<MIP>_E_<EN>`（`SWS_BSW_00125`）；全局变量 `<Mip>_<Vn>`（`SWS_BSW_00130`），只读的加 `const`（`SWS_BSW_00131`） |
| BSW 文件 | SWS BSWGeneral R24-11 | 文件名 `<Mip>[_<Ie>]*.*`（`SWS_BSW_00103`）；区分大小写，且不能只靠大小写区分（`SWS_BSW_00170`／`00171`）；实现源包含 `<Mip>_MemMap.h`（`SWS_BSW_00006`）；只引入需要的头、只导出其他模块需要的内容（`SWS_BSW_00104`／`00105`）；全局变量定义在实现源中（`SWS_BSW_00129`） |
| 数据类型 | `SWS_BSW_00120`、MISRA Dir 4.6 | BSW 模块（`runtime/bsw`）不用 C 原生类型（`int`、`unsigned`、`char` 等），用 `Platform_Types.h` 的 AUTOSAR 类型（`uint8`、`sint16`、`boolean` 等），经 `Std_Types.h` 引入；`Platform_Types.h` 自身的定义除外。OS 的模块代码同样使用 AUTOSAR 类型，配置计数使用有界 AUTOSAR 整数，文本字节使用 `uint8`，仅在原生 API 调用边界转换字符指针；主机适配（`runtime/host`、`runtime/ecu` 及 OS 主机后端）对接平台 API 时按其签名 |
| 编译开关 | `SWS_BSW_00123` | 与定义值比较（`#if (X == STD_ON)`），不用 `#ifdef` 判断开关 |
| 独占区 | [ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md) | 模块只进入自己的区；持区期间不调用其他模块、回调或平台 API；区不嵌套 |
| 格式 | `.clang-format`，规则见 [#129](https://github.com/g1331/autsaro/issues/129#issuecomment-6160383270) | 生成 C 由 emitter 直接写成同一排版（[ADR 0014](../adr/0014-emitter-rules.md)） |

## 项目约定

### 命名

标准接口、类型、宏与配置参数一律用规范原名。规范没有规定的部分按下表：

| 对象 | 写法 | 示例 |
| --- | --- | --- |
| 文件内 `static` 函数 | `<Mip>_<Name>` 或 `lower_case` | `CanIf_Reset`、`owner_state` |
| 局部变量、`static` 变量、内部结构成员 | `lower_case` | `pending_lower_id` |
| 参数与配置结构成员 | `lower_case`；对应标准签名或配置时用规范原名 | `TxPduId`、`PduInfoPtr` |
| 宏、枚举常量 | `UPPER_CASE`，以本模块 `<MIP>_` 开头 | `CAN_TX_HOH`、`CANIF_ONLINE` |
| 类型 | 以 `Type` 结尾；模块类型用 `<Ma>_<Tn>Type` | `CanIf_PduModeType` |
| 主机适配接口 | `<Mip>_Host<Name>`，不占用标准接口名 | `Can_HostTransmitToken` |

名称表达职责，不使用开发计划编号或 multi、legacy、new 等相对词（[ADR 0015](../adr/0015-repository-code-organization.md)）。

### 文件头与公开 API 注释

每个文件第一行是文件说明，写本文件负责什么、范围或主要限制，不写变更历史：

```c
/** @file Standard CanIf for one Classical CAN controller and immutable routes. */
```

说明较长时用块形式，首行只写 `/** @file`，下一行写 `@brief`。项目不生成 Doxygen 文档；单行形式与生成 C 的文件头一致。

公开头中的每个函数在声明前说明契约。标准接口只写本实现的选择与限制，不抄规范原文；四项中不适用的写“无”，不省略。注释使用英文，与现有代码一致：

```c
/** <What the call does; cite the SWS item where it helps>.
 * @param[in] <Name> <Accepted range; who owns a pointed-to object>.
 * @param[out] <Name> <What is written, and when>.
 * @return <Meaning of each result>.
 * Preconditions: <State that must hold; result when it does not>.
 * Context: <Task, callback or interrupt; whether callable inside an exclusive area>.
 * Reentrancy: <Concurrent calls for the same and for different instances>.
 * Ownership: <Whether pointers are kept after return; when outputs are written>.
 */
```

- 参数方向：`@param[in]`、`@param[out]`、`@param[in,out]` 按 SWS 接口表的 Parameters (in)／(out)／(inout) 标注，非标准接口按实际读写标注；无参数时省略 `@param`。
- `void` 函数用 `@par Return` 单独起一段，下一行写 `None.`；不使用会触发 clang `-Wdocumentation` 的 `@return`。非 `void` 函数继续用 `@return` 说明结果。
- AUTOSAR 要求无名参数时保留签名：`SWS_Rte_06703` 规定存在 `RunnableEntityArgument.symbol` 时的参数名，`SWS_Rte_06705` 规定既无该对象又无 BSW 映射时生成无名 role 参数。无名参数用 `@par Parameters` 段，按声明顺序逐项写 `1. [in] …`、`2. [out] …` 或 `3. [in,out] …`；编号从 1 开始，数量、顺序与方向逐项由同一检查器校验。有名参数仍用 `@param`，不以增加参数名规避规范。

```c
/** Run the selected server operation.
 * @par Parameters
 * 1. [in] Input value.
 * 2. [out] Non-NULL caller-owned result storage; borrowed until return.
 * @par Return
 * None.
 * Preconditions: ECU initialized and the component's owner task active.
 * Context: component task, outside exclusive areas.
 * Reentrancy: not reentrant for the same component.
 * Ownership: no pointer is retained.
 */
void Server_Run(uint8, uint32 *);
```

规范规定的签名和语义优先于工具的默认注释写法；遇到冲突时核对条款，在本篇记录可校验的等价注释形式，并由检查器适配。例行选择由实现者在 PR 写明依据与验证，不改变签名或降低诊断要求。

- 前置条件：调用前必须成立的状态，以及不成立时的结果。
- 调用上下文：任务、回调、中断，以及是否允许在独占区内调用。
- 可重入性：同一实例与不同实例能否并发调用。
- 所有权：指针参数由谁持有、是否在返回后保留、输出参数何时写入。

公开头中的类型、枚举与宏按语义来源注释：

- 实现自定义的类型（如 `<Mip>_ConfigType`，其内容由实现决定）及项目自有枚举：类型前说明用途；成员用行尾 `/**< … */` 写明含义、单位、取值范围、特殊值（如 0 的含义）及对应的 ECUC 参数。例如 CanTp 的 `n_as` 是主函数周期数而不是毫秒，只能从这里得知。
- SWS 定义的类型、枚举值与常量：类型前注明条款编号（如 `/** SWS_CANIF_00137 */`），成员和枚举值用行尾 `/**< … */` 写一句用途说明，读代码时不必查规范；用自己的话简述，不抄规范原文，规范定义之外的实现限制（如主机控制器不报告某个状态）写在类型说明中。
- `.c` 内部的类型不强制，按下节在有不变式或约束时说明。
- 公开头中的宏按用途注释：模块身份与版本信息（`SWS_BSW_00059` 的 Published information）写一条组注释，说明来源与缺项；配置开关写明对应的 ECUC 参数及由谁定义；SWS 定义的常量（如错误码）注条款编号并写一句用途说明；实现自定的常量用行尾 `/**< … */` 写含义与单位；带参数的宏按函数写契约注释（MISRA Dir 4.9 建议优先用函数）。include guard 与 MemMap 段宏不注释。`.c` 内部的宏按[魔数](#魔数)要求命名。
- 文件作用域的状态变量（`static` 及模块全局变量）用行尾 `/**< … */` 说明含义，并写明由哪个独占区保护（或“仅初始化期间写入”）、`Init` 是否重置；同组标志之间的不变式（如某标志为真时另一标志必为真）写在该组前。函数内局部变量不强制。
- 行尾 `/**< … */` 保持简短，使同组声明能在 100 列内对齐：`.clang-format` 在对齐会超出列宽时从该行另起一组，结果参差不齐。较长的说明（含同组不变式）写在组前注释中。

### 注释

交付的 C 源码注释直接写出约束，不引用仓库内部的 ADR、Issue 编号或文档路径；出处放在模块标准篇或 ADR 中。

注释写原因与约束：为什么这样做、依据哪条规范、不变式是什么、哪种看似更简单的做法不成立。不复述代码，不写会过时的过程记录。刻意为空的分支写出为什么不处理：

```c
} else {
    /* Late confirmation after shutdown/timeout cannot replay callbacks. */
}
```

### 文件内顺序

`#include` → 宏 → 类型 → 数据（含 MemMap 段）→ `static` 函数原型 → 实现。不强制分隔横幅；MemMap 段按 `<MIP>_START_SEC_…`／`<MIP>_STOP_SEC_…` 成对出现。

### 魔数

除 0、1 及其在位运算和计数中的自然用法外，数值用有名字的宏、枚举或配置项，名字说明含义与单位，例如 `COM_MAX_PDUS`。来自规范的值在定义处注明条款。生成 C 中的数值来自配置，由生成器命名。一般魔数由本规范和审阅约束；`c_standard` 另外检查 BSW `.c` 中直接 `Det_ReportError`、`Det_ReportRuntimeError`、`Det_ReportTransientFault` 调用的模块 ID（第 1 实参）与服务 ID（第 3 实参），拒绝其中的数字 token，包括括号或转换表达式。命名宏的取值、宏展开和间接调用仍须按规范及独立行为预期核对。

### 函数长度

函数不超过 60 行（含空行与注释）。超过时按职责拆出 `static` 函数，不为凑行数拆出无意义的片段。存量长函数和不合约定的名字记录在 clang-tidy 基线中，随所在模块改写收敛：BSW 模块对应 #41–#45，OS、ECU 与主机适配在修改到相应函数时处理。

### 错误与返回值

- 标准接口按规范返回 `Std_ReturnType` 或规定的类型；发展错误按 `SWS_BSW_00045` 报告 Det，先做规定顺序的检查（`SWS_BSW_00255`）。
- 内部函数用返回值报告失败，不用全局错误变量；调用方检查每个非 `void` 返回值，确实忽略时写 `(void)` 并说明原因（MISRA 17.7）。
- 输出参数只在成功时写入，或在说明中写明失败时的值。
- 不吞错、不伪造成功；无法恢复的主机错误走既有的终止路径，例如 OS 的 `Os_BackendShutdown`。

## AUTOSAR 契约与 MISRA 诊断的冲突

固定版次的 AUTOSAR 必需契约优先。核对原文后，若接口、callout 专用内存段等标准义务与 MISRA 规则或工具诊断无法同时满足，保留正确契约和原始分析结果。对应 PR 逐条列出新增诊断的规则、文件与源码行，以及要求该行的条款编号和必要性（条目多时放折叠块或 PR 评论）；对不上必需条款的诊断不属于冲突，应修改代码消除。Issue 范围内的这类决定由实现者依据上述证据作出，源码诊断裁决的 Issue 只记录结论与链接。不能靠删除标准段标记、停止 include、缩小扫描范围或增加无依据 suppression 来消除诊断。

按 [ADR 0023](../adr/0023-autosar-required-conflict-disposition.md) 执行 [2026-10-17 用户决定](https://github.com/g1331/autsaro/issues/58#issuecomment-6232975641)：独立核实必需且无法同时满足的冲突，允许逐项依证据记为偏离，无需用户再次逐条批准。项目授权与 MISRA 规范效力分开记录；合法原件（#57）、适用版次／amendment／TC、类别（包括 Mandatory）、实际调用／包含闭包、风险控制及修改后源码人工评估（#58）与独立复核均是实际处置前提。缺任何依据时保持未核查／待评估，不进入批准表或添加违反注释；不能从用户意图、工具编号或技能摘要推定允许偏离。Mandatory 不允许偏离的限制仍保留，原文确认不能偏离的真实违反保持阻塞。保留必需行本身不构成偏离或符合性结论，`passed=false`、`error` 与人工 `not_assessed` 原样保留。

## MISRA 偏离注释

`SWS_BSW_00115` 要求偏离在源码中就近注释说明。格式按该条款的示例，`12` 为 MISRA C:2012：

```c
/* MR12 RULE 11.5 VIOLATION: <为什么不能遵守，以及如何限定风险> */
/* MR12 DIR 4.9 VIOLATION: <理由> */
```

注释写在引起违反的语句同一行或上一行，一处违反一条注释。注释只标出位置和理由，不构成批准：

- 具体记录位于对应模块的标准篇（`docs/standards/`），沿用现有记录路径；[ADR 0023 的逐项格式](../adr/0023-autosar-required-conflict-disposition.md#逐项记录与复核)连接固定提交／profile／target／program／TU、规则、文件／源码行和符号、实际调用闭包、SWS 全文必要性及替代方案、合法 MISRA 版次／amendment／TC、风险控制与验证、评估和独立复核。原授权来源、评估者及日期、复核 head／发现／处置分开写，不把实现者写成新增批准人；范围外新例外先完成具体文本再由用户决定。
- 只有具体记录满足原授权范围、规范允许条件与上述全部评估／复核前提后才在源码中加注释，注释中的规则编号与记录一致；Cppcheck 的原始诊断照常保留，注释不是 suppression。本次政策同步不批准具体诊断，不修改既有批准表或源码注释。
- 已批准的偏离：[PduR](../standards/pdur.md#misra-偏离) 的 Rule 8.7，[Com](../standards/com.md#misra-偏离) 的 Rule 8.7 与 2.5，[CanTp](../standards/cantp.md#misra-偏离) 的 Rule 8.7，[ComM](../standards/comm/README.md#misra-偏离) 的 Rule 8.7 与 2.5，[BswM](../standards/bswm/README.md#misra-偏离) 的 Rule 8.7 与 2.3。其余模块整改时补齐所需记录与注释。
- ADR0023原授权下的具体有界冲突处置见[RTE无名role参数](../standards/rte/contracts.md#无名-role-参数的逐处核查)的Rule 8.2记录；它不改变其他源码点或221项人工评估的状态。

## 自动检查

| 检查 | 范围 | 入口 |
| --- | --- | --- |
| 文件头 | `runtime/` 的 `.c`／`.h` 第一行为 `/** @file` | `quality` |
| 公开头自包含 | `runtime/{bsw,host,ecu,os}/include` 的全部头文件单独以 C99 编译；MemMap 头按成对段使用 | `quality`：C 文件有改动、`--scope runtime` 或 `--all-format` 时运行 |
| 契约与可判定源码规则 | runtime 公开头的函数、typedef、成员、枚举值与宏注释，文件状态注释，BSW 原生类型、开关写法及偏离记录双向对应 | 每次 `quality`；`--c-samples` 同时检查实际生成的公开头 |
| clang documentation | 生成样本的实际编译闭包，检查参数名称及方向语法 | 第二编译器 `-Wdocumentation` |
| clang-tidy | 生成样本中除 `kernel/`、`app/` 外的全部编译单元及其头文件 | `quality --c-samples <样本目录>` |

clang-tidy 的配置在仓库根目录 `.clang-tidy`，启用 `clang-analyzer-*`、`bugprone-*`、`readability-function-size`（60 行）、`readability-braces-around-statements` 与 `readability-identifier-naming`。停用的检查及原因写在配置文件中：`easily-swappable-parameters` 与固定的标准签名冲突；`branch-clone` 把 MISRA 15.7 要求的结尾 `else` 和其他刻意为空的分支视为重复；`optin.performance.Padding` 要求按大小重排字段。命名检查只用于 `runtime/` 的源码，生成 C 的名字来自 RTE／BSW 规则与配置短名。工具只能检查名字的形式，模块前缀是否正确仍由审阅确认。

运行时源码依赖生成的配置头，所以 clang-tidy 分析生成样本，结果按实际文件映射回 `runtime/`。生成样本的命令见[生成 C 的静态检查](testing/generated-c.md)，然后运行：

```powershell
uv run --locked python -m autosar_tooling quality --scope runtime --c-samples build/c-analysis-samples
```

结果按“检查＋文件＋源码行文本”与基线 `tools/python/src/autosar_tooling/clang_tidy_baseline.txt` 比较，同一行文本出现多次时按次数计。比较时只忽略词法确认的完整行尾 `/**< … */` 注释，补齐说明不产生新的代码诊断；代码、字符串、普通注释及出现次数仍逐项比较，原始输出保留完整源码行。代码移动不影响比较；新出现的结果和已经修好、但仍留在基线中的条目都会失败。修好后删除对应条目；新增代码诊断应修复源码，不通过增加基线条目消除失败。基线只覆盖 Linux 样本，必需 CI 在 Linux 的 `c-analysis-prepare` 中运行；Windows 样本会被拒绝。

仅补注释或排版导致既有诊断换到另一源码行时，先核对同一文件、符号、规则及出现次数，并证明改动前后的 C 与预处理 token 一致；满足这些条件后只迁移该存量条目的源码行文本，在 PR 保留原始诊断和前后对应证据。诊断消失时删除条目，实际新增或代码行为变化不能按此迁移。

clang-tidy 与 clang-format 由 uv 固定为同一版本 **22.1.8**（`pyproject.toml` 的 `quality` 组），两者一起升级：`--fix` 产生的排版与格式检查一致。PyPI 的 `clang-tidy` 包还没有更新的主版本前不单独升级 clang-format。升级后对运行时源码和生成样本重做格式检查，重新生成样本并核对基线变化：分析器结论会随版本改变。

源码规则的存量基线为 `tools/python/src/autosar_tooling/c_standard_baseline.txt`，同样按“规则＋相对文件＋源码行文本”计数，新增违规与已修复但未删的条目均失败。`emitted/` 标识实际生成头文件，只有提供 `--c-samples` 时比较这部分；共享源码从 `integration.json` 的来源映射识别，避免重复记录。检查器验证注释结构和参数覆盖；范围、单位、生命周期和 SWS 语义仍须人工按调用链核对。偏离检查读取现有模块篇的批准表，按 RULE／DIR、规则号、文件与具体符号双向比较；注释与记录匹配不构成批准，也不改变原始 MISRA 结果。
