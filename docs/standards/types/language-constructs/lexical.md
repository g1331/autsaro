# 语言、词法与名称的检查边界

本篇承接[语言构造](README.md)。固定工具为 Cppcheck 2.21.0／e73bf44，
`misra.py` 的 SHA256 为 `d1f7947b6c2c6f3980314155d6a89f0e6a9435456fb3a02ac1ded25a99af3f44`。
表中同号入口均指该文件的 `misra_N_M`，核心入口指 `getCppcheckRules` 的准确诊断映射。
入口审阅和必要人工责任分别记录；`not_assessed` 行须由 #443 完成所列实际核查后重评。

所选输入为双目标四 profile 的 C99 HostBatch／probe，自有 runtime、生成／应用输出
与有序补丁后的项目行。完整扫描采用 #459 的 run 42448921398，实际 checkout
`a6529d192757b570429e8a2676ce77054561f55a`。本篇的能力判断须与该扫描的完整
程序／TU、配置及 addon 收据共同使用；SDK 实例沿 #444，采用内核原件沿采用篇。
本次记录保留具体人工缺口，尚未给本组绑定 approved 上下文。

原文为 March 2013 Third Edition＋AMD1–4／TC1–2，七份原件已逐一核验摘要。
本篇已阅读主文 R1.2／R1.3（38–39）、R2（39–44）、R3.1（45）、R4（46–47）、
R5.2／R5.4／R5.6／R5.8／R5.9（49–57）、R6.2／R7.1（59–60）的正文。
以下未评估项还要求把适用修订和实际源码／直接消费者核查一起完成；表中的能力
定位支持当前缺口判断，保留原始工具与 221 项 assessment 的含义。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R1.2 | Advisory | C99 扩展及实际消费者 | not_assessed | 同号入口只识别省略中项的条件表达式和 statement expression。实际 GNU 属性、TLS、原子内建和平台 ABI 扩展须连接 [OS 平台](../../os/platform.md) 的选择理由与编译／运行约束；#443 尚缺按当前全部 TU 的扩展用途核对及适用修订合成。 |
| R1.3 | Required | 完整程序的未定义及关键未指定行为 | not_assessed | 核心映射为大部分 error 类诊断；值流能发现具体故障，尚不足以覆盖主文 Appendix H 的全部行为。#443 须把每个适用行为接到自有／patch 的对象生命周期、范围、求值顺序和实际消费者，核对主文及修订后的 Appendix H；检测器通过仅覆盖实际执行路径。 |
| R2.1 | Required | 各程序的可达路径 | not_assessed | 核心 alwaysFalse／duplicateBreak 检查局部不可达结构。回调、状态机和配置决定的入口仍需实际调用闭包；#443 尚缺按 HostBatch／probe 分开的配置可达性核查和适用修订合成，避免把示例没调用的公开接口当作不可达。 |
| R2.2 | Required | 可观察输出和持久状态的生产路径 | not_assessed | addon 识别部分加减零、乘零／一，主动跳过宏展开；核心补充冗余赋值／条件。#443 须核对实际宏、副作用、volatile、时序和直接消费者；删除状态写入后无可观察效果的路径仍须沿 producer 修复。 |
| R2.3 | Advisory | 全程序 typedef 定义与使用 | not_assessed | 同号入口保存 typedef CTU 摘要，使用证据依赖完整 TU 与类型识别。既有公共类型处置见[标准类型](../public-types.md)；#443 尚缺当前 CTU 的逐实体用途和新生成类型对照，具体保留诊断由原 owned／generated 组处理。 |
| R2.4 | Advisory | 全程序 tag 定义与实际引用 | not_assessed | 同号入口保存 tag CTU 摘要。关联 typedef 的使用仍须与 tag 本身的使用区分；#443 须核对匿名类型、重复头包含及补丁删除的 xSTATIC_TIMER，接续[声明用途](../declarations.md)，并合成适用修订。 |
| R2.5 | Advisory | define／undef 历史与实际宏使用 | not_assessed | 同号入口保存 MisraMacro 摘要，并将遇到 undef 的名字记作已用；这会留下对真实使用的额外人工责任。#443 须逐实体核对接口宏、段关键词及生成宏的当前消费者，接续[生成接口](../../rte/generated-retention/README.md)；仅撤销宏的路径须单独判断。 |
| R2.6 | Advisory | 函数内 label 与跳转 | not_assessed | 核心 unusedLabel 定位未用标签。#443 尚缺固定核心诊断的完整性审阅、宏展开后标签与实际 goto 的关联，以及当前配置消费者证据；主机重入／清理标签沿实际函数核查。 |
| R2.7 | Advisory | 函数定义的形参与调用契约 | not_assessed | 同号入口从函数体变量引用中扣除已用形参，不逐项评价仅显式丢弃的用途。#443 须核对标准回调角色、实际丢弃与固定签名必要性，接续[公共接口](../../bsw-general/public-api-os.md)，并合成适用修订。 |
| R3.1 | Required | 原物理文件的注释 token | not_assessed | 同号入口检查注释内 /* 和 //，对 URI 形式另有启发式处理；主文的 // 注释内 // 例外须分别核定。#443 尚缺所有自有／patch 注释、行拼接与适用修订的实际核查，不能只依赖启发式零报告。 |
| R4.1 | Required | 字符／字符串中的数值转义 | not_assessed | 同号入口拆分 raw token 中的反斜杠并调用数值／简单转义辅助函数。#443 尚缺带前缀字面量、转义边界和宏定义中的原 token 核查及适用修订；源码与运行字符值须同时保持。 |
| R4.2 | Advisory | 预处理之前的全部物理源码 | not_assessed | 主文 47 页覆盖任意位置的三字符组；同号入口只检查双引号字符串中的九种序列，字符常量、注释和其他位置留下人工缺口。#443 须检查实际自有／patch 物理字节及拼接前条件，SDK 原件及其集成用途沿 #444。 |
| R5.2 | Required | 同作用域／namespace 的有效名称 | not_assessed | 同号入口比较超出固定 significant-chars 长度的变量／命名 scope 前缀，跳过两者皆为形参等分支。#443 须按实际编译器有效字符、名称种类和原文例外核查同 scope 实体，补齐适用修订。 |
| R5.3 | Required | 嵌套 scope 的名称遮蔽 | not_assessed | 核心 shadowVariable 提供变量遮蔽候选；其他 namespace、typedef／枚举常量及宏展开后的名字仍需核查。#443 须连接当前声明 owner 和实际消费者，并合成修订例外。 |
| R5.4 | Required | 活动宏名与各自参数名 | not_assessed | 同号入口用 define 正则和前缀表比较名字／参数，未按 undef 清除表，参数正则也有语法范围。#443 须按 define／undef 实际时间顺序、有效字符和参数集合核查；固定 11 项 R5.5 保持独立。 |
| R5.6 | Required | 跨 namespace／TU 的 typedef 名称 | not_assessed | 同号入口保存 typedef CTU 摘要；主文要求同名其他实体也参与唯一性判断，关联 tag 和同头重复定义各有条件。#443 须核对完整摘要的 namespace 闭包、实际头身份及修订例外。 |
| R5.8 | Required | 外部链接名称与全部其他实体 | not_assessed | 同号入口保存 identifier CTU 摘要。#443 尚缺跨 namespace、局部无链接对象、条件定义及两个程序各自的完整集合复核，结合主文唯一性义务与适用修订逐实体判断。 |
| R5.9 | Advisory | 内部链接名称与全部其他实体 | not_assessed | 同号入口保存 identifier CTU 摘要；同头 static inline 的原文例外须按真实声明来源识别。#443 尚缺当前内部函数／对象与局部实体的完整比较及修订合成，不能仅按路径或名字前缀排除。 |
| R6.1 | Required | 项目定义的位域及实际声明类型 | not_assessed | 同号入口检查 Struct 中已解析 bits／variable 的显式 signed／unsigned，C99 另接受 bool。Union、typedef 及实现允许类型的边界仍须核查；#443 尚缺当前实际位域集合、修订原文和直接读写消费者证据。 |
| R6.2 | Required | 有名的一位位域 | not_assessed | 同号入口检查 Struct 中 bits=1 且 signed 的 token；主文明确无名位域的范围。#443 须补 Union／typedef 展开的实际位域及使用点核查，确认有名对象的符号属性并合成修订。 |
| R7.1 | Required | 原 token 和实际展开的整数常量 | not_assessed | 主文 59–60 页允许单独的 0，数值后缀保留八进制属性；同号正则 ^0[0-7]+$ 只识别无后缀形式，077U／077L 留下缺口。#443 须对当前原 token、条件表达式和宏展开作完整词法核查，数值转义沿 R4.1 单独判断。 |
| R7.4 | Required | 字符串流入对象、返回和函数实参 | not_assessed | 同号入口使用赋值目标 constness、已解析实参和简单 const char * 返回声明匹配。typedef、条件表达式、间接调用及跨 TU 别名仍需实际类型追踪；#443 尚缺当前字符串到直接消费者的闭包和适用修订合成。 |
<!-- misra-checker-obligations-end -->
