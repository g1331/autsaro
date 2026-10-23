# 修订与例外核对

221 项身份、有效类别、新增版次、语言适用性和现有指导的关键判断已按2013主文与 AMD1–AMD4/TC1–TC2 逐组复核，并与 MISRA 官方维护的 C:2023 合订目录交叉核对；`baseline-index.json` 只保存编号、分类与定位事实，不包含其受许可限制的 headline 原文。编码动作以固定原文为准，公开技术参考页仅作辅助。此复核不穷尽全部 amplification、例外、理由和示例，也不评估实际源码的221项符合性。

## 新增项的完整账目

| 层次 | Rules | Directives | 总数 |
| --- | --- | --- | --- |
| 2012 初版 | 143 | 16 | 159 |
| AMD1 新增 | 13 | 1 | 14 |
| AMD2 新增 | 2 | 0 | 2 |
| AMD3 新增 | 23 | 1 | 24 |
| AMD4 新增 | 19 | 3 | 22 |
| 当前合计 | 200 | 21 | 221 |

TC 与 AMD 对既有条文的修订不增加编号；各清单的“新增于”不是“最后修订于”。全部条目始终按当前基线理解，不能把标为 2012 的行当成未被修订的旧规则。

## 已定位的易错变化

- [R13.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.6.html) 当前是 Required；不要套用旧分类或仅因 sizeof 不求值就允许潜在副作用。
- [R17.5](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.5.html) 当前是 Required；数组形参的实参数量要按契约核对。
- [R21.11](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.11.html) 当前是 Advisory；[R21.12](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.12.html) 当前是 Required。
- [R7.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule7.6.html) 的 small integer 常量宏判断依赖目标 int 宽度；不能将四个宏名写成对所有目标都穷尽的列表。
- [R11.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.8.html) 的限定符判断包括 _Atomic；[R13.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule13.2.html) 包含线程交错；[R18.6](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.6.html) 包含线程局部寿命。
- [R18.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.8.html) 与 [R18.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule18.10.html) 分别检查 VLA 与可变修改数组指针，不能仅扫描数组声明。
- [R21.8](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.8.html)、[R21.19](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.19.html)、[R21.20](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.20.html)、[R21.21](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule21.21.html) 应分别核对终止、内部返回数据、寿命及 system，避免沿用修订前的合并解释。

## 例外不能简化成统一禁令

- [R3.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule3.1.html)：AMD4 的 URL 例外按原文核对，不因 URI 中有双斜线就改坏链接。
- [R9.2](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.2.html)、[R9.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule9.3.html)、[R11.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule11.9.html)：零初始化、指定初始化和包含指针聚合的允许形式分别核对，不机械禁止 `{0}`。
- [R10.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.3.html)、[R10.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule10.4.html)：常量、字符、枚举及复数的许可条件必须结合精确类型模型，不用“值能装下”代替所有约束。
- [R14.3](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule14.3.html)、[R17.4](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule17.4.html)：有意永循环与 C99 main 的特殊规则需单独核对。
- [R19.1](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule19.1.html)：完全重叠的兼容类型赋值及 memmove 的允许形式不能误报成不允许偏离的违规；普通 memcpy 的重叠仍有风险。
- [D4.9](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.9.html)、[D4.10](https://www.mathworks.com/help/bugfinder/ref/misrac2012d4.10.html)、[R20.7](https://www.mathworks.com/help/bugfinder/ref/misrac2012rule20.7.html)：泛型宏、重复包含协议和非表达式宏分别看上下文，不能一套括号/防重包含策略覆盖全部。

- R8.1 与 R17.3 仅适用 C90（主文 §8.8／§8.17）；C99 中隐式声明仍违反语言约束，按 R1.1 及声明规则核查，不能转换成 R17.3 Mandatory 偏离。
- R21.4 必须同时检查头文件与设施使用（AMD3.67，印刷p43／PDFp51），隐藏 include 或加包装不能消除 setjmp/longjmp。替代设计须保持原接口的不可返回、资源清理、队列与执行上下文义务；不能只改成错误返回。
- R21.6 覆盖标准 I/O 标识符／宏及 wide I/O（主文 §8.21，AMD2.79），包括内存格式化；没有主机用途自动例外。沿实际调用图找标准设施，封装或改成平台接口名字不自动证明该设施已消除，也不自动批准偏离。
- R21.8：AMD1 §3 移除 getenv，AMD2.81 整项替换为 abort/exit/_Exit/quick_exit；system 另由 AMD2.83 新增 R21.21 约束。不能把旧 TC1 的修辞性改动重新引入已删除的禁用项。
- R7.6 的 AMD4.4 正文和 AMD4.64 摘要均为 Required；AMD4.65 附录属性表却写 Advisory。保留正文及锁定目录的 Required，不用该不一致降低约束。

以上是定位提醒，不是全部例外的替代清单。对任何项的新构造或争议判定都访问该行的具体参考，核对适用授权原文。公开厂商说明中的版本支持、误报规避和 AGC 类别不是 MISRA 给本仓库的批准。

## Generated code 与工具覆盖

Rust 生成器输出 C 不意味着自动适用所有 AGC 重分类。先确认主文 §6.8／附录E及项目规则重分类计划的适用条件，再记录有效类别；以修订链有效类别为起点。BSW和主机基础设施不能仅因由生成器复制就视为 model-based 自动生成应用。SWC、BSW、RTE、主机适配和第三方采用代码要按角色/范围区分。

目录覆盖、分析器覆盖、证据覆盖分别记录：全部 221 项都有入口，不表示 Cppcheck 能检查全部项，也不表示项目已完成这 221 项检查。[厂商未检查项说明](https://www.mathworks.com/help/bugfinder/ug/misra-c2012-guidelines-not-checked.html) 明确 D3.1 与 D4.2 的非自动检查性质；目录不能因此省略它们。

## 外部材料复核

[MISRA 官方合订编号/分类来源](https://gitlab.com/MISRA/MISRA-C/MISRA-C-2012/tools/-/raw/main/misra_c_2023__headlines_for_cppcheck.txt) 说明其 CC BY-NC-ND 许可及用途；本仓库不复制、翻译或打包该 headline 文件。用户合法持有未改动本地文件时，可以只做编号/分类比对：

```powershell
python -X utf8 .agents/skills/misra-c2012/scripts/catalog.py validate --official-file <本地官方文件.txt>
```

基线快照记录官方来源 SHA256；更新时重新核对全部身份、分类与指导，不能自动用新下载内容覆盖锁定目录。2026-10-23 的原文复核未改变221项身份／有效类别或新增版次，因此保留原快照及两项 fingerprint。基线元数据分别标记旧目录快照与本次复核；它们不是当前源码评估日期。实际任务从 [原文定位与身份](standard-baseline.md#原文定位与身份) 检查可用材料，缺文件时只限定依赖它的判断。
