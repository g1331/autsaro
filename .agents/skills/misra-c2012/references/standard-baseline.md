# 规范基线与来源

核对日期：2026-09-28。以下是本技能的版本选择依据，不是完整规范副本。再次遇到“最新修订”、迁移或工具覆盖判断时，核实 MISRA 官方发布与实际工具文档；无法核实时注明沿用此快照。

## 使用哪个版本

| 项目要求或手头材料 | 处理方式 |
| --- | --- |
| MISRA C:2012 Third Edition | 连同 AMD1、AMD2、AMD3、AMD4、TC1、TC2 使用，核对适用的更新与例外。 |
| MISRA C:2012 Third Edition, First Revision | 已合入 AMD1/TC1；再核对 AMD2–AMD4、TC2，避免按已被修订的旧条文判断。 |
| MISRA C:2023 | C:2012 规则、TC1/TC2 及 AMD1–AMD4 的合订版本；核实条文对应关系后作为参考，不把未核对的工具结果改标为 C:2012。 |
| MISRA C:2025 | 已有后续出版物；不属于名为 C:2012 的 amendment。需要明确迁移需求与规则差异核对。 |
| 本仓库 C99 | 仍使用 C99 编译与接口契约；C11/C18 专有功能按语言与代码实际判断不适用，AMD 中针对已有功能的修订仍需核对。 |

AMD 是 amendment（修订），TC 是 technical corrigendum（勘误）；ADD 是 addendum（补充资料），不能把规则映射或覆盖说明当成修订。C:2012 更新链与“整个 MISRA C 系列最新版本”是两个问题。

## 可核验来源

- [MISRA 官方 C:2023 编号与分类目录](https://gitlab.com/MISRA/MISRA-C/MISRA-C-2012/tools/-/raw/main/misra_c_2023__headlines_for_cppcheck.txt)：锁定 200 Rules、21 Directives 的身份和有效分类；许可说明与全文核对边界见 [修订与例外核对](revisions-and-exceptions.md)。
- [C:2012 逐项公开技术入口](https://www.mathworks.com/help/bugfinder/misra-c-2012-reference.html) 与 [未检查项入口](https://www.mathworks.com/help/bugfinder/ug/misra-c2012-guidelines-not-checked.html)：逐项指导各自链接到具体参考页，不从“工具支持列表”推导完整性。

- [MISRA 官方 AMD4，2023-03](https://www.misra.org.uk/app/uploads/2023/03/MISRA-C-2012-AMD4.pdf)：§1.1 给出 Third Edition 与 First Revision 的配套更新链；§1.2 说明 C11/C18 相关增补。只引用所需位置，不复制整份规范。
- [MathWorks 官方 MISRA C:2023 说明](https://www.mathworks.com/help/ecoder/ug/misra-c2023-compliance-information-summary-tables.html)：开头说明合并了 C:2012、AMD1–AMD4、TC1–TC2。该厂商的自动生成代码适用性和分类表仅适用于其说明范围，不能原样作为本仓库规则分类或完整覆盖证明。
- [MISRA 官方 C:2025 ADD6](https://misra.org.uk/app/uploads/2025/03/MISRA-C-2025-ADD6.pdf)：References [1] 列出 2025-03 的 C:2025 主规范。这里只用来核实后续版次存在，不据此推导具体规则变化。
- [MISRA Compliance:2020 官方文档](https://www.misra.org.uk/app/uploads/2021/06/MISRA-Compliance-2020.pdf)：§5 核对规则类别与重分类限制，相关章节核对偏离、采用代码及符合性总结流程。它是流程指导，不是 C:2012 规则全文。
- [MISRA 官方论坛](https://forum.misra.org.uk/)：按明确的规则/指令和版本查询澄清；区分官方答复、普通用户提问与特定案例，不能把案例解释扩展成普遍豁免。

## 材料与工具缺口

优先使用用户合法提供、且许可允许本次访问的本地原文或受控规范库。记录实际文档版次/标识及相关章节，不搜索盗版或将原文嵌入技能。`docs/official/` 的忽略约定不等于自动存在 MISRA 文件。

正式规范不可用时，已有实现提示和工具诊断仍可指导局部修改；对规则编号、有效分类、例外和完整符合结论保留缺口。不得以模型回忆、网络规则摘要或厂商的产品覆盖表代替项目的完整规范核对。

分析器应记录版本、规则包/配置、语言标准、目标数据模型、预定义宏、include 路径、翻译单元与所支持的 amendment/TC。工具未覆盖的 directive、不可判定规则及配置变体需其他证据；完整性缺口与实际违规分开。当前仓库的 Cppcheck 版本和扫描范围以 `scripts/quality_baseline.py` 为准。
