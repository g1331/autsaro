# 规范基线与来源

基线指导原文复核日期：2026-10-23；公开编号目录仍保留 2026-09-28 的快照身份。以下是版本选择和定位依据，不是完整规范副本。再次遇到“最新修订”、迁移或工具覆盖判断时，核实 MISRA 官方发布与实际工具文档；无法核实时注明沿用此快照。

## 使用哪个版本

| 项目要求或手头材料 | 处理方式 |
| --- | --- |
| MISRA C:2012 Third Edition | 连同 AMD1、AMD2、AMD3、AMD4、TC1、TC2 使用，核对适用的更新与例外。 |
| MISRA C:2012 Third Edition, First Revision | 已合入 AMD1/TC1；再核对 AMD2–AMD4、TC2，避免按已被修订的旧条文判断。 |
| MISRA C:2023 | C:2012 规则、TC1/TC2 及 AMD1–AMD4 的合订版本；核实条文对应关系后作为参考，不把未核对的工具结果改标为 C:2012。 |
| MISRA C:2025 | 已有后续出版物；不属于名为 C:2012 的 amendment。需要明确迁移需求与规则差异核对。 |
| 本仓库 C99 | 仍使用 C99 编译与接口契约；C11/C18 专有功能按语言与代码实际判断不适用，AMD 中针对已有功能的修订仍需核对。 |

AMD 是 amendment（修订），TC 是 technical corrigendum（勘误）；ADD 是 addendum（补充资料），不能把规则映射或覆盖说明当成修订。C:2012 更新链与“整个 MISRA C 系列最新版本”是两个问题。

## 原文定位与身份

先沿仓库 [官方资料入口](../../../../docs/official/README.md) 与 [#57 资料记录](https://github.com/g1331/autsaro/issues/57) 找到当前环境获准访问的仓库外资料库，再定位其 `MISRA/C-2012/`。资料库示例位置不是自动发现路径；没有该目录时检查任务提供的位置或受控资料入口，不把另一台机器的可用性当作当前事实。

固定组合的文件名为 `MISRA-C-2012.pdf`、`MISRA-C-2012-AMD1.pdf` 至 `MISRA-C-2012-AMD4.pdf`、`MISRA-C-2012-TC1.pdf` 和 `MISRA-C-2012-TC2.pdf`，同目录 `SHA256SUMS` 固定收到字节。核对清单、实际 SHA-256、封面和版权页；收到字节匹配不自动证明出版方摘要或许可来源。主文应辨明 March 2013 第三版（PDF ISBN 978-1-906400-11-8）与 February 2019 First Revision（PDF ISBN 978-1-906400-22-4）；下载／授权日期和 PDF 元数据不能替代版次。

| 原文 | 封面日期 | 本组合的定位方式 |
| --- | --- | --- |
| 2013 主文 | March 2013 | §6 类别／语言／角色，§7 Directives，§8 Rules，相关附录 |
| AMD1 | April 2016 | §1/§2 新增项，§3 R21.8 移除 getenv |
| TC1 | June 2017 | §1/§2/§3 的指令、规则和附录勘误；无逐项 TC 编号时引用条目标题 |
| AMD2 | February 2020 | §2 的 AMD2.n 修订项，含语言适用性表与 R21.8 整项替换 |
| TC2 | March 2022 | §1–§4 的 TC2.n 勘误项，含类别与允许形式变化 |
| AMD3 | October 2022 | §2 新增／更新指导及 §3 后续修订，不能只读新增项 |
| AMD4 | March 2023 | §2 新指导、§3 勘误、§4 后续修订，不能只读并发条目 |

逐项从主文或首次新增条目出发，合成全部相关修订，包含后续修订对既有构造的改变；旧文本中的替换／删除指令不能脱离后续整项替换单独使用。精确引用写文件名、节／修订项和印刷页，同时注明 PDF 页（从1计数）。当前七份文件中主文正文印刷页加8为PDF页，AMD1加8、TC1/TC2加6、AMD2/AMD4加7、AMD3加8；换文件必须重新确认。图、类型／运算符表或版面影响判断时渲染并查看原页。提取文本和图片只放临时目录，原件和全文不入产品仓库。

## 辅助来源

- [MISRA 官方 C:2023 编号与分类目录](https://gitlab.com/MISRA/MISRA-C/MISRA-C-2012/tools/-/raw/main/misra_c_2023__headlines_for_cppcheck.txt)：交叉检查 200 Rules、21 Directives 的身份和有效分类，原文优先；许可说明与全文核对边界见 [修订与例外核对](revisions-and-exceptions.md)。
- [C:2012 逐项公开技术入口](https://www.mathworks.com/help/bugfinder/misra-c-2012-reference.html) 与 [未检查项入口](https://www.mathworks.com/help/bugfinder/ug/misra-c2012-guidelines-not-checked.html)：逐项指导各自链接到具体参考页，不从“工具支持列表”推导完整性。

- [MISRA 官方 AMD4，2023-03](https://www.misra.org.uk/app/uploads/2023/03/MISRA-C-2012-AMD4.pdf)：§1.1 给出 Third Edition 与 First Revision 的配套更新链；§1.2 说明 C11/C18 相关增补。只引用所需位置，不复制整份规范。
- [MathWorks 官方 MISRA C:2023 说明](https://www.mathworks.com/help/ecoder/ug/misra-c2023-compliance-information-summary-tables.html)：开头说明合并了 C:2012、AMD1–AMD4、TC1–TC2。该厂商的自动生成代码适用性和分类表仅适用于其说明范围，不能原样作为本仓库规则分类或完整覆盖证明。
- [MISRA 官方 C:2025 ADD6](https://misra.org.uk/app/uploads/2025/03/MISRA-C-2025-ADD6.pdf)：References [1] 列出 2025-03 的 C:2025 主规范。这里只用来核实后续版次存在，不据此推导具体规则变化。
- [MISRA Compliance:2020 官方文档](https://www.misra.org.uk/app/uploads/2021/06/MISRA-Compliance-2020.pdf)：§5 核对规则类别与重分类限制，相关章节核对偏离、采用代码及符合性总结流程。它是流程指导，不是 C:2012 规则全文。
- [MISRA 官方论坛](https://forum.misra.org.uk/)：按明确的规则/指令和版本查询澄清；区分官方答复、普通用户提问与特定案例，不能把案例解释扩展成普遍豁免。

## 材料与工具缺口

优先使用用户合法提供、且许可允许本次访问的本地原文或受控规范库。记录实际文档版次/标识及相关章节，不搜索盗版或将原文嵌入技能。实际能读取固定原文时直接核对，不沿用“未取得主文”的历史前提。

原文不可用或只取得部分修订时，明确缺哪份文件、哪项判断依赖它；已有实现提示和工具诊断仍可指导局部修改、编译与行为验证。缺失部分涉及的适用性、有效类别、例外与规范裁决保留未核查，依赖该裁决的整改不靠摘要猜定；不把整个任务一概停住。不得以模型回忆、网络规则摘要或厂商的产品覆盖表代替项目的完整规范核对。

分析器应记录版本、规则包/配置、语言标准、目标数据模型、预定义宏、include 路径、翻译单元与所支持的 amendment/TC。工具未覆盖的 directive、不可判定规则及配置变体需其他证据；完整性缺口与实际违规分开。使用工具时按当前任务核对实际工具版本、配置和检查范围。
