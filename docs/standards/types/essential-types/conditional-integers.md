# R7.2 条件控制表达式的有界证据

[Linux 部分交付 #422](https://github.com/g1331/autsaro/pull/422)固定原文与方法：MISRA七份原件通过SHA256SUMS（摘要见该PR）。R7.2为Required、Decidable、Single Translation Unit（印刷60–61／PDF68–69，含原表）；AMD2.46及语言表增加后续语言适用性，其余AMD／TC不改变本项C99判据。按literal标准类型判定，不取目的类型、后续转换或essential类型。

另核对[WG14 N1256](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1256.pdf) §6.4.4.1类型表、§6.10.1¶3–6及脚注145（C99＋TC1–TC3草案，收到字节SHA见#422）。两目标预处理均用64位intmax_t／uintmax_t；普通32位int表达式中为unsigned int的0xffffffff在这里仍为signed，0xffffffffffffffff才需要U。2147483648为signed decimal，负号是独立运算符。defined操作数不展开，其余宏按使用点展开，剩余标识符变0；求值为假的控制式仍检查literal，已选前支后的elif及外层排除条件不假造展开。

复用正常seal和补丁后工程，按scope.patchedLines区分项目patch与采用行，采用条件中的项目宏输入一并观察。一次性副本以当时的ifdef结果代换见证中的defined，经两层字符串化保留宏及##展开；elif转为else＋if，#line保持原位置，原控制式不改。移除见证pragma及空行后的-E -P输出逐TU与未插桩结果相同。独立token文法区分进制／后缀／值，编译器解析全部展开式，并以literal与-1比较验证预处理signedness。

Linux的进入结论、基线、seal及完整复现见#422：原GCC13.3.0、原flags／includes的228个TU -E -dD与捕获相同；GCC13／Clang18.1.3对172式解析／求值一致，每份92项plan及既有重建内核匹配。已闭合来源审计不重做。

| Linux profile | TU | 自有位置／实际求值位置／次数 | 项目patch位置／实际求值位置／次数 |
| --- | --- | --- | --- |
| standard-ecu | 56 | 747／745／34504 | 60／59／539 |
| user-application | 56 | 747／745／34504 | 60／59／539 |
| component-pipeline | 58 | 787／783／41085 | 60／59／539 |
| no-diagnostic | 58 | 761／759／37427 | 60／59／539 |

位置按文件＋源码行去重，次数保留跨TU／MemMap重复包含。Linux自有无U数字为0–20，unsigned仅0U／1U／4U／10U／0x00u／0x01u；patch仅0／1，均未发现本项违反。采用原件另见16／32／8及带u形式，不外推供应方符合。未求值自有位置为DcmService_MemMap.h:19/27（前两profile）、Observe_MemMap.h:19/27及上述DcmService两处（component-pipeline）、Observe两处（no-diagnostic）；均仅含defined，无literal。每份patch tasks.c:370位于未选多核else，直接literal为signed1，INCLUDE_vTaskDelete未在该位置求值。

Windows基线`4102db720bab7f99e5c32f68f6edc6f235012752`；[进入结论](https://github.com/g1331/autsaro/issues/58#issuecomment-6622074513)先于修改。[真实Windows运行41771007441](https://github.com/g1331/autsaro/actions/runs/41771007441)在Server2025使用GCC16.1.0 Rev5、x86_64-w64-mingw32；driver摘要与历史相同（完整身份见该运行）。实际headers／CRT为14.0.0.r598.ge3d18511f-1，区别于历史r569；不声称历史SDK输入等价。每profile实际进入的283个外部头均匹配所选provider原件。四份历史seal、当前170项C/H/patch及92项plan吻合（9项按历史CRLF核对）；原工件只读。

同一方法在原C99/O1/g/Wall/Wextra/Werror/fexceptions和include顺序下完成224个TU，全部输出一致；Windows GCC独立解析／求值167种展开式，并验证literal signedness及普通0xffffffff的unsigned int类型。控制fixture覆盖##、false短路式中的unsigned及无U十六／八进制／LL违反；带U、signed十进制／LL与0xffffffff不误判。defined存在／不存在、false后elif、已选elif及外层inactive均按预期记录／跳过，选中代码相同。首轮遗漏-MT的失败保留；仅补正常依赖目标后的运行成功，严格解析器不变。

| Windows profile | TU | 自有位置／实际求值位置／次数 | 项目patch位置／实际求值位置／次数 |
| --- | --- | --- | --- |
| standard-ecu | 55 | 747／742／33946 | 59／58／518 |
| user-application | 55 | 747／742／33946 | 59／58／518 |
| component-pipeline | 57 | 787／780／40527 | 59／58／518 |
| no-diagnostic | 57 | 761／756／36869 | 59／58／518 |

Windows自有展开比Linux少15／19，其余及patch数字相同，均未发现本项违反。除上述MemMap位置外，各profile另有Os_MemMap.h:294/358及Os_HostTypes.h:8未求值：前两处位于未选section，直接literal仅signed15／19；后者为未选Linux elif且只有defined。tasks.c:370同样不在该配置展开。defined代换的0／1仅为见证值；未实例化宏不冒充已展开。

本项闭合所列两目标、四profile实际自有runtime／generated／application及项目patch的条件证据缺口。其他配置／TU、未求值宏的其他实例、SDK内部及本次条件证据以外的普通表达式未由该交付评估；重评须取得相应输入、求值闭包及控制。#58承接剩余范围；raw passed=false／error、221 not_assessed、completeCompliance=false、66/155、retention及strict refusal保持，不宣布整条R7.2或总体符合。

相同正常 TU 的普通表达式现另见[普通整数评估](ordinary-integers.md)，其中 Windows SDK 宏的真实 Required 缺口继续保留。
