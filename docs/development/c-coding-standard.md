# C 编码规范

适用于 `runtime/` 的 C 源码与头文件，以及 emitter 生成的 C；FreeRTOS 内核（`third_party/`）和用户应用（生成工程的 `app/`）保持各自风格。本篇只写项目约定，硬约束引用原文，不复述。

## 引用的硬约束

| 约束 | 依据 | 要点 |
| --- | --- | --- |
| MISRA C:2012 修订基线 | [ADR 0008](../adr/0008-misra-baseline-and-no-deviation.md)、[misra-c2012 指导](../../.agents/skills/misra-c2012/SKILL.md) | C99；偏离见下文 [MISRA 偏离注释](#misra-偏离注释)；`c-check` 的用法见[生成 C 的静态检查](testing/generated-c.md) |
| BSW 命名 | SWS BSWGeneral R24-11 | API `<Mip>_<Sn>`（`SRS_BSW_00310`）；主函数 `<Mip>_MainFunction[<Sd>]`（`SWS_BSW_00153`）；类型 `<Ma>_<Tn>Type`，平台基本类型除外（`SWS_BSW_00146`）；宏、枚举字面值与状态值 `<MIP>_<SN>`，全大写，`E_OK`／`E_NOT_OK` 除外（`SWS_BSW_00124`）；错误值 `<MIP>_E_<EN>`（`SWS_BSW_00125`）；全局变量 `<Mip>_<Vn>`（`SWS_BSW_00130`），只读的加 `const`（`SWS_BSW_00131`） |
| BSW 文件 | SWS BSWGeneral R24-11 | 文件名 `<Mip>[_<Ie>]*.*`（`SWS_BSW_00103`）；区分大小写，且不能只靠大小写区分（`SWS_BSW_00170`／`00171`）；实现源包含 `<Mip>_MemMap.h`（`SWS_BSW_00006`）；只引入需要的头、只导出其他模块需要的内容（`SWS_BSW_00104`／`00105`）；全局变量定义在实现源中（`SWS_BSW_00129`） |
| 数据类型 | `SWS_BSW_00120`、MISRA Dir 4.6 | BSW 模块（`runtime/bsw`）不用 C 原生类型（`int`、`unsigned`、`char` 等），用 `Platform_Types.h` 的 AUTOSAR 类型（`uint8`、`sint16`、`boolean` 等），经 `Std_Types.h` 引入；`Platform_Types.h` 自身的定义除外。OS 的标准接口随 #50 整改；主机适配（`runtime/host`、`runtime/ecu` 及 OS 主机后端）对接平台 API 时按其签名 |
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
- 前置条件：调用前必须成立的状态，以及不成立时的结果。
- 调用上下文：任务、回调、中断，以及是否允许在独占区内调用。
- 可重入性：同一实例与不同实例能否并发调用。
- 所有权：指针参数由谁持有、是否在返回后保留、输出参数何时写入。

公开头中的类型、枚举与宏按语义来源注释：

- 实现自定义的类型（如 `<Mip>_ConfigType`，其内容由实现决定）及项目自有枚举：类型前说明用途；成员用行尾 `/**< … */` 写明含义、单位、取值范围、特殊值（如 0 的含义）及对应的 ECUC 参数。例如 CanTp 的 `n_as` 是主函数周期数而不是毫秒，只能从这里得知。
- SWS 定义的类型、枚举值与常量：类型前注明条款编号（如 `/** SWS_CANIF_00137 */`），成员和枚举值用行尾 `/**< … */` 写一句用途说明，读代码时不必查规范；用自己的话简述，不抄规范原文，规范定义之外的实现限制（如主机控制器不报告某个状态）写在类型说明中。
- `.c` 内部的类型不强制，按下节在有不变式或约束时说明。
- 文件作用域的状态变量（`static` 及模块全局变量）用行尾 `/**< … */` 说明含义，并写明由哪个独占区保护（或“仅初始化期间写入”）、`Init` 是否重置；同组标志之间的不变式（如某标志为真时另一标志必为真）写在该组前。函数内局部变量不强制。

### 注释

注释写原因与约束：为什么这样做、依据哪条规范、不变式是什么、哪种看似更简单的做法不成立。不复述代码，不写会过时的过程记录。刻意为空的分支写出为什么不处理：

```c
} else {
    /* Late confirmation after shutdown/timeout cannot replay callbacks. */
}
```

### 文件内顺序

`#include` → 宏 → 类型 → 数据（含 MemMap 段）→ `static` 函数原型 → 实现。不强制分隔横幅；MemMap 段按 `<MIP>_START_SEC_…`／`<MIP>_STOP_SEC_…` 成对出现。

### 魔数

除 0、1 及其在位运算和计数中的自然用法外，数值用有名字的宏、枚举或配置项，名字说明含义与单位，例如 `COM_MAX_PDUS`。来自规范的值在定义处注明条款。生成 C 中的数值来自配置，由生成器命名。这一项只由本规范和审阅约束，不用工具检查。

### 函数长度

函数不超过 60 行（含空行与注释）。超过时按职责拆出 `static` 函数，不为凑行数拆出无意义的片段。存量长函数和不合约定的名字记录在 clang-tidy 基线中，随所在模块改写收敛：BSW 模块对应 #41–#45，OS、ECU 与主机适配在修改到相应函数时处理。

### 错误与返回值

- 标准接口按规范返回 `Std_ReturnType` 或规定的类型；发展错误按 `SWS_BSW_00045` 报告 Det，先做规定顺序的检查（`SWS_BSW_00255`）。
- 内部函数用返回值报告失败，不用全局错误变量；调用方检查每个非 `void` 返回值，确实忽略时写 `(void)` 并说明原因（MISRA 17.7）。
- 输出参数只在成功时写入，或在说明中写明失败时的值。
- 不吞错、不伪造成功；无法恢复的主机错误走既有的终止路径，例如 OS 的 `Os_BackendShutdown`。

## MISRA 偏离注释

`SWS_BSW_00115` 要求偏离在源码中就近注释说明。格式按该条款的示例，`12` 为 MISRA C:2012：

```c
/* MR12 RULE 11.5 VIOLATION: <为什么不能遵守，以及如何限定风险> */
/* MR12 DIR 4.9 VIOLATION: <理由> */
```

注释写在引起违反的语句同一行或上一行，一处违反一条注释。注释只标出位置和理由，不构成批准：

- 偏离记录写明规则、适用范围、依据、风险控制和批准人，位于对应模块的标准篇（`docs/standards/`）或 ADR；只能由维护者批准（ADR 0008）。
- 记录获批后才在源码中加注释，注释中的规则编号与记录一致；Cppcheck 的原始诊断照常保留，注释不是 suppression。
- 已批准的偏离：[PduR](../standards/pdur.md#misra-偏离) 的 Rule 8.7，[Com](../standards/com.md#misra-偏离) 的 Rule 8.7 与 2.5。其余模块随 #43–#45 改写时补齐所需记录与注释。

## 自动检查

| 检查 | 范围 | 入口 |
| --- | --- | --- |
| 文件头 | `runtime/` 的 `.c`／`.h` 第一行为 `/** @file` | `quality` |
| 公开头自包含 | `runtime/{bsw,host,ecu,os}/include` 的全部头文件单独以 C99 编译；MemMap 头按成对段使用 | `quality`：C 文件有改动、`--scope runtime` 或 `--all-format` 时运行 |
| clang-tidy | 生成样本中除 `kernel/`、`app/` 外的全部编译单元及其头文件 | `quality --c-samples <样本目录>` |

clang-tidy 的配置在仓库根目录 `.clang-tidy`，启用 `clang-analyzer-*`、`bugprone-*`、`readability-function-size`（60 行）、`readability-braces-around-statements` 与 `readability-identifier-naming`。停用的检查及原因写在配置文件中：`easily-swappable-parameters` 与固定的标准签名冲突；`branch-clone` 把 MISRA 15.7 要求的结尾 `else` 和其他刻意为空的分支视为重复；`optin.performance.Padding` 要求按大小重排字段。命名检查只用于 `runtime/` 的源码，生成 C 的名字来自 RTE／BSW 规则与配置短名。工具只能检查名字的形式，模块前缀是否正确仍由审阅确认。

运行时源码依赖生成的配置头，所以 clang-tidy 分析生成样本，结果按实际文件映射回 `runtime/`。生成样本的命令见[生成 C 的静态检查](testing/generated-c.md)，然后运行：

```powershell
uv run --locked python -m autosar_tooling quality --scope runtime --c-samples build/c-analysis-samples
```

结果按“检查＋文件＋源码行文本”与基线 `tools/python/src/autosar_tooling/clang_tidy_baseline.txt` 比较，同一行文本出现多次时按次数计。代码移动不影响比较；新出现的结果和已经修好、但仍留在基线中的条目都会失败。修好后删除对应条目；新增条目须在 PR 中说明理由并经审阅。基线只覆盖 Linux 样本，必需 CI 在 Linux 的 `c-analysis-prepare` 中运行；Windows 样本会被拒绝。

clang-tidy 与 clang-format 由 uv 固定为同一版本 **22.1.8**（`pyproject.toml` 的 `quality` 组），两者一起升级：`--fix` 产生的排版与格式检查一致。PyPI 的 `clang-tidy` 包还没有更新的主版本前不单独升级 clang-format。升级后对运行时源码和生成样本重做格式检查，重新生成样本并核对基线变化：分析器结论会随版本改变。
