# R7.2 普通表达式整数的有界证据

本篇接续 [条件控制证据](conditional-integers.md)，只评估相同正常 TU 预处理后实际存在的整数常量，包括声明、初始化、宏展开与未求值操作数。起点为 `e8d47edfb9d97d66fa3e22daa6b4a9fd4b8d30c8`；[进入结论](https://github.com/g1331/autsaro/issues/58#issuecomment-6622766176)先于交付文件修改。四 profile、两个正常程序 HostBatch／probe、PDU ID／长度及 no-diagnostic 仍含 Dcm 的选择均沿用 #422／#423。

## 判据与实际输入

MISRA C:2012 Third Edition R7.2（Required，印刷 60–61／PDF 68–69）同时覆盖条件控制和预处理后整数；本次重新校验七份原件 SHA256SUMS、阅读正文与原表，并合成 AMD2.46 及其余 AMD／TC。普通表达式按 [WG14 N1256](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1256.pdf) §6.4.4.1 的候选表选择第一个能表示该值的类型。原件身份见 #422，不取赋值目标、essential type 或后续转换作为 literal 类型。

两目标 int 均为 32 位，long long 为 64 位；Linux LP64 的 long 为 64 位，Windows LLP64 的 long 为 32 位。无后缀十进制只依次考虑 signed int／long／long long；十六／八进制在各 rank 交错考虑 signed／unsigned。L 从 long 开始，LL 从 long long 开始，U 只考虑 unsigned。不能由“不带 U”推定 signed，也不能把 #if 的 intmax_t 模型套到这里。负号是单独运算符。

直接读取 #422 Linux 228 份 `compiler-origins-N.stdout.log` 与 [#423 Windows 成功运行 41771007441](https://github.com/g1331/autsaro/actions/runs/41771007441)的 224 份 `normal.i.gz`。它们是正常 producer 的 `-E -dD` 输出；Windows 另含 `-MD -MF … -MT autsaro-tu`。保留原 C99／O1／g／Wall／Wextra／Werror／fexceptions、Linux pthread 和 include 顺序。没有插桩项目 TU、重新生成或改写历史输出。Windows 实际 headers／CRT 为 r598，历史 r569 仍单列，不能声称二者输入等价。

行标记连接实际补丁后 project 文件和原行号；每个数字保存 TU、文件、行、预处理输出列、拼写、类型与责任层。输出列用于重现 token，不冒充宏定义的源码列。词法扫描完整普通输出，排除指令（含宏定义）、字符串、字符和注释，完整识别 C99 进制／后缀；浮点单列，Windows SDK 的 f16／f32／f64／f128 扩展浮点也明确分开，未知数字拒绝分类。未实例化宏定义不计入。

沿 `originalBswSources` 核对当前 C／头／patch 与历史交付映射，共 1,340 项（Windows 历史 CRLF 按 LF 归一化比较，非重写 seal）；生成来源沿现有 plan、`core/tests/support/analysis_samples.rs` 和生成器闭包。相对 #421 基线 core／runtime 没有代码差异，复用其已核实的生成／采用及补丁后身份，不重做 seal 或来源审计。应用来自正常初始化及多组件 application 输入，不将 SDK 声明算入 application。`scope.patchedLines` 区分项目 patch 和采用原行；采用行的实际整数同时作保守扫描，以防项目宏经采用使用点进入，结果不外推供应方内部符合。

## 结果与来源责任

| 目标／profile | TU | 项目使用点整数次数 | application | patch |
| --- | --- | --- | --- | --- |
| linux／standard-ecu | 56 | 13605 | 10 | 400 |
| linux／user-application | 56 | 13605 | 11 | 400 |
| linux／component-pipeline | 58 | 13666 | 21 | 400 |
| linux／no-diagnostic | 58 | 13611 | 21 | 400 |
| windows／standard-ecu | 55 | 12619 | 10 | 382 |
| windows／user-application | 55 | 12619 | 11 | 382 |
| windows／component-pipeline | 57 | 12680 | 21 | 382 |
| windows／no-diagnostic | 57 | 12625 | 21 | 382 |


次数保留跨 TU／重复包含，每行多个数字分别计数；“项目使用点”含自有 runtime／generated，可能消费 SDK 宏，不能直接等同数字定义归属。application 及 patch 单列。采用原行数字的保守检查未另发现缺 U 候选，SDK 内部数字保留边界，不作本项销项。

Linux 所选项目、application 和 patch 使用点未发现本项违反。Windows 每 profile 有 11 个使用位置出现 unsigned 且缺 U 的 literal，三种数字均追到实际所选 MinGW 原头：

| 使用位置（每 profile） | 实际 token／类型 | 定义闭包 |
| --- | --- | --- |
| Os_Backend.c:1263、1266、1353、1483、1521、1537；Os_Stack.c:411；patch 后 port.c:54 | `0xffffffff`／unsigned int | winbase.h:518 `INFINITE` |
| Os_Stack.c:167、172 | `0xC0000005`／unsigned int | minwinbase.h:334 `EXCEPTION_ACCESS_VIOLATION` → winnt.h:1440 `STATUS_ACCESS_VIOLATION` |
| Os_Stack.c:171 | `0xC00000FD`／unsigned int | minwinbase.h:352 `EXCEPTION_STACK_OVERFLOW` → winnt.h:1459 `STATUS_STACK_OVERFLOW` |

这些是普通表达式中的真实 Required 缺口，四 profile 共 44 次，不能用 DWORD cast、无穷等待语义或 SDK 身份将它们判为满足。port.c:54 属项目补丁使用点，数字定义仍来自 SDK；不得按内核路径忽略它。当前所选项目自行定义／生成／应用的 literal 未另发现缺 U 违反，因此未修改源码、生成器或 patch。非 ISO SDK 宏的既有来源／采用责任仍由 [#58](https://github.com/g1331/autsaro/issues/58)承接，不扩大标准库内部边界，也不靠改名、复制数字或重定义保留宏规避；本次不开展供应方修复调查。

## 编译器证伪与未评估范围

Linux GCC 13.3.0 与 Clang 18.1.3 对 443 个 token／控制形式的类型断言成功，并拒绝故意将 `0xffffffffL` 判为 unsigned long 的反例。[真实 Windows 控制运行 41775278505](https://github.com/g1331/autsaro/actions/runs/41775278505)成功：相同 GCC 16.1.0 Rev5 driver（摘要匹配 #423）对 446 个形式和 LLP64 宽度断言成功，六个错误类型断言全部失败。工件保留完整 fixture、身份、正负输出。Linux 实际项目使用点有 296 种拼写，Windows 有 304 种；Windows 其中 303 种直接进入类型断言，另一个 `4294967295u` 按明确 U 后缀及已验证 32 位 unsigned int 值域判定，没有声称对该拼写另作编译断言。

类型断言使用 GCC／Clang 的 `__typeof__` 与 `__builtin_types_compatible_p` 在独立 C99 fixture 中观察标准 literal 类型；不加入产品或修改正常宏配置。所编译的实际 token、后缀去 U 变体和边界集与独立候选表一致；去 U 后超出 C99 有符号候选范围的十进制明确拒绝，不作为正常 signed 值。正反控制包括 `0xffffffff` 的 unsigned int、`2147483648` 的 signed decimal、LP64／LLP64 不同的 `0xffffffffL`／等值八进制、LL 与 U／UL／ULL，以及 `0xffffffffffffffffLL` 缺 U。故能发现普通表达式中缺 U 的 unsigned literal，也不会把 unsigned 变量的 signed 初始化常量误报。独立 fixture 的错误预期必须编译失败。

本项只闭合所列正常 TU 的普通整数取证缺口；上述 SDK 违反、其他配置／TU、未实例化宏的其他使用点和 SDK 内部继续未完成。未宣布整条 R7.2 或总体 MISRA 符合。原始 passed=false／error=null、程序 exit1、221 not_assessed、completeCompliance=false、66／155、retention 与 strict refusal 不变；本目录不在五篇人工机制输入选集中。实际本地验证及独立审查、精确 head CI 见交付 PR，不把文档 CI 未选择的运行时矩阵记作执行通过。

## 当前普通表达式的复核

#443 从 [#448 最终 head 的完整工件](https://github.com/g1331/autsaro/actions/runs/42391343618)
接续上述方法，逐份检查实际 `compiler-origins-N` 输出，Linux 228 个 TU、Windows 224 个 TU。
仍按 LP64／LLP64 的 C99 候选类型表选择 literal 类型，保留原行标记、实际后缀及项目 patch 归属。
只统计自有 runtime、生成／应用及项目修改行的使用点；采用内核原行继续沿其集成责任核对。

| profile | Linux 整数 token 次数 | Windows 整数 token 次数 | Windows 缺 U 使用位置 |
| --- | --- | --- | --- |
| standard-ecu | 14015 | 13011 | 11 |
| user-application | 14016 | 13012 | 11 |
| component-pipeline | 14087 | 13083 | 11 |
| no-diagnostic | 14032 | 13028 | 11 |

次数保留重复 TU 和重复包含；浮点、指令、字符串及字符常量另行区分。
Linux 所选使用点没有缺 U 候选；Windows 仍为上表三种 SDK 数字，四 profile 共 44 次。
MR12 注释插入后 Backend 最后两处等待位置移动到 1523／1539，其余具体位置沿前表。
这次复核确认当前暴露仍存在，项目侧集成处置由 #444 承接；原头和宏定义保持。
当前生成输出、完整 seal 与 producer 差异的适用范围见[语言构造](../language-constructs.md#当前输入与三项人工证据的接续)。

正常 `c-check` 的控制输入进一步区分两个入口：普通表达式 `0xffffffff` 能触发固定 R7.2，
相同 64 位目标下的 `#if 0xffffffffffffffff > 0` 则没有该报告且 raw `passed=true`。
后者的 unsigned 常量仍由人工条件证据承担，见[条件控制](conditional-integers.md)。
本篇正文的验收消费继续归 #443，完整扫描和具体诊断消费分别保留其证据范围。
