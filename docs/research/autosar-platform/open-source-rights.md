# 独立开源 AUTOSAR Classic 实现：权利边界与待核事项

> 研究日期：2026-09-24。研究对象是**自行编写** Classic BSW／RTE／配置生成器并开源，而非复用既有栈；以下是公开一手材料的事实梳理，不构成法律意见或项目取得许可的证明。判断须按**发布版本、具体文件、拟实施行为、使用主体和分发方式**进行。

## 一手材料确认的边界

- AUTOSAR 的 [Classic 下载页](https://www.autosar.org/standards/classic-platform) 将所有公开文件（不限格式）标为仅供信息用途，指出其中材料受版权及其他知识产权保护，商业利用需相应权利许可，并要求参看各文件声明；[官网 Terms of Use PDF](https://www.autosar.org/fileadmin/user_upload/Documents/AUTOSAR_Terms_Of_Use.pdf) 与 [R23-11 BSW General 第 4 页 Disclaimer](https://www.autosar.org/fileadmin/standards/R23-11/CP/AUTOSAR_CP_SWS_BSWGeneral.pdf) 进一步允许**不修改**的利用／复制仅供信息用途，其他用途须取得出版者书面许可。这不是开源许可证，文件可下载不等于可将原文、表格、规范 PDF、源码或机器可读制品随开源仓库/发行包商业分发；但也不能把该声明直接扩张成“任何独立编写代码一律禁止”。
- AUTOSAR [FAQ 1.1.12、1.2.1](https://www.autosar.org/faq#c560) 称派生材料发表由作者自行负责遵守版权/IP 规则；要利用（exploit）AUTOSAR Work 的公司需成为合作伙伴，并举出组件集成/进一步加工的场景。[合作伙伴页面](https://www.autosar.org/about/partners) 区分权益和义务，列出 Royalty-Free Exploitation License 项及须签署 partner agreement 的流程；**公开概览不是已签协议全文**，不能据此断言本项目/贡献者已有授权，亦不能推定所有独立实现都必然需要某一档会员。应就计划中的免费公开源码、商用二进制、客户集成逐一向 AUTOSAR/法律顾问确认“Work／material／exploit”的适用范围及专利/其他 IP 授权。Associate Partner [Light] 在页面中只面向被选中的 AUTOSAR Work 子集，不应当作全栈通行证。
- [R24-11 Foundation XML Schema ZIP](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip) 内 `_readme.txt` 指明 `AUTOSAR_00053.xsd` 才是该包的 AUTOSAR STANDARD，`autosar.soc` 为辅助示例，`xml.xsd` 则适用另列的 W3C 条款；`_disclaimer.txt` 和主 XSD 文件也带 AUTOSAR 上述声明。**机器可读不等于开放再分发**：生成器可研究公开 XSD 与 ARXML 语义，但直接提交/镜像 XSD、把它嵌入发行包、改写成内部模型或复制示例 ARXML，都须逐件核对出处、权利和适用许可；独立产出的用户配置/生成 C 文件也需检查是否包含复制的受保护模板/样板/第三方代码。W3C 文件的许可不能自动覆盖 AUTOSAR XSD。
- [规范 Disclaimer](https://www.autosar.org/fileadmin/standards/R23-11/CP/AUTOSAR_CP_SWS_BSWGeneral.pdf) 指明 AUTOSAR 名称和 logo 为注册商标；[官网 Legal Disclosure「Brands and Logos」](https://www.autosar.org/impressum) 表明网站信息不授予商标许可，未经授权使用 AUTOSAR 商标可能被禁止并要求明确书面同意。描述兼容目标与将产品命名为官方/认证产品、使用 logo 是不同问题；任何“符合/通过 AUTOSAR 认证”或 endorsement 宣传应有定义清楚的版本、范围、测试证据并经商标/合规审查，勿以自行测试冒充官方认证。
- 外部标准**另有权利人**。例如 [AUTOSAR R24-11 DCM SWS](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_SWS_DiagnosticCommunicationManager.pdf) 的变更记录明确引用 ISO 14229-1；AUTOSAR 文件许可不转授 ISO 文本权利。[ISO Copyright](https://www.iso.org/copyright.html) 说明其出版物的未经授权复制、扫描、分发不被允许；[ISO/IEC 官方使用指南第 7–9 页](https://www.iso.org/files/live/sites/isoorg/files/store/en/PUB100206.pdf) 对共享副本、在软件中复制标准片段、翻译/修改举例要求另行许可，并区分个人购买与多人/网络使用。可在说明中**引用标准编号**，但把 ISO 要求原文、测试表、报文定义的原样表述、图表或 PDF 纳入仓库/AI 知识库/生成器之前，取得与版本及用途匹配的许可或书面确认；纯粹独立实现技术要求的法律边界（含可能的专利）须另审。
- MCU SDK 与构建工具不共用 AUTOSAR 的许可证：[NXP MCUXpresso SDK 官方文档](https://mcuxpresso.nxp.com/mcuxsdk/latest/html/) 明言整体交付适用其 Online Code Hosting 条款，`mcuxsdk-core` 为 BSD-3-Clause，其他仓库可采用不同许可证，各仓库有 SBOM；[ST 官方 STM32CubeH7 仓库组件许可表](https://github.com/STMicroelectronics/STM32CubeH7/blob/master/LICENSE.md) 同时列有 BSD-3-Clause、Apache-2.0、MIT、SLA0044、SLA0047 等组件。这些例子已足够否定“SDK 整包都是宽松开源”的推论。作为工具链例子，[Arm Development Studio Morello EULA 第 1、2.2(c)、3.1 节](https://developer.arm.com/GetEula?Id=a91be01e-7c0f-4a16-abe3-de903449d1f9) 将开发工具的使用授权与发行可分发文件区别，并限定 Redistributables File 所列文件、通常仅作为具实质额外功能的软件中的目标码；该 EULA **只说明该特定产品/版本**，不能移植到其他 Arm/GCC/商业编译器。

## 行为—权利/不确定性矩阵

| 拟采取行为 | 已由公开材料确认 | 未确认／发布前动作 |
| --- | --- | --- |
| 阅读公开 CP 规范、以原创代码实现接口/BSW、发布自写代码 | 规范可下载供信息用途；独立实现不等于复制规范文件。[Classic 页](https://www.autosar.org/standards/classic-platform) | **尚无独立开源+商业利用的一揽子授权结论**。法务核定拟实现要求、专利/其他 IPR、适用合作条款；必要时向 AUTOSAR 书面询问。将原创性、来源、具体版本留痕。 |
| 发布规范 PDF、提取的要求文本/表、官方 XSD、示例 ARXML 或衍生转换件 | 仅“未修改且供信息用途”例外与其他用途书面许可条件；XSD 包内有独立 disclaimer。[Terms of Use](https://www.autosar.org/fileadmin/user_upload/Documents/AUTOSAR_Terms_Of_Use.pdf) · [Schema ZIP](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip) | 不随仓库/vendor 包默认镜像；逐文件记录来源、版本、许可、拟分发范围，请求授权；如无需交付，仅给官网链接并让使用者自行取得。 |
| 开源配置器/生成器及其模板和生成输出 | 官网说明 ARXML 基于 Templates/Schema，生成器可消费描述支持 RTE/BSW 配置。[Classic 页「Methodology and Templates」](https://www.autosar.org/standards/classic-platform) | 对模板、生成头文件、示例、输出抽样核对是否复制 AUTOSAR/厂商表达；不要把“生成”视为清除上游权利。核定用户输入 ARXML 的许可，明确输出物来源及许可证。 |
| 在名称、徽标、README、销售材料声称“官方/认证/符合 AUTOSAR” | 商标受保护，网站无商标授权。[Legal Disclosure](https://www.autosar.org/impressum) | 事前审查措辞、标识和证明资料；声明仅限已验证 release/module/test scope，不暗示官方认可。 |
| 纳入 ISO 原文/测试资料或标准片段 | ISO/IEC 对复制到软件、共享、翻译要求许可。[ISO 指南](https://www.iso.org/files/live/sites/isoorg/files/store/en/PUB100206.pdf) | 清点 ISO/IEC/SAE 等引用的实际版本、授权主体及许可范围；不要提交标准 PDF/完整文本。 |
| 带 MCU SDK/HAL/MCAL、启动文件、设备头或工具链/runtime 发源码/镜像/固件 | NXP 与 ST 按组件授权；Arm 示范工具、可分发文件、最终二进制可有不同条件。[NXP](https://mcuxpresso.nxp.com/mcuxsdk/latest/html/) · [ST](https://github.com/STMicroelectronics/STM32CubeH7/blob/master/LICENSE.md) · [Arm](https://developer.arm.com/GetEula?Id=a91be01e-7c0f-4a16-abe3-de903449d1f9) | 以确定的芯片、SDK/工具版本建立组件 SBOM 与来源清单；读实际包内 LICENSE/NOTICE/redistributables 和第三方/硬件限制，区分链接固件、源码组件、编译器安装包的分发；无法确认者不捆绑。 |

## 发布前最小证据包与待法务/厂商确认

1. 固定目标 Classic release、规范和 schema 的 URL/版本；建立逐文件 provenance（AI 生成也保留输入来源、人工核对记录），检索并清除贴入的规范段落、XSD、示例及其他栈代码；AI 生成不自动解决版权、专利或第三方来源问题。
2. 明确分发形态：GitHub 源码、生成器二进制、商业集成、ECU 固件分别问 AUTOSAR 是否落入其公开条款所说的 AUTOSAR Work/material 商业利用与 partner agreement 范围，是否需版权、专利或书面许可；得到具体授权再据实声明，**不能预设“开源即豁免”或“凡实现皆违法”**。[FAQ](https://www.autosar.org/faq#c560) · [Terms of Use](https://www.autosar.org/fileadmin/user_upload/Documents/AUTOSAR_Terms_Of_Use.pdf)
3. 建立第三方标准/组件账本，逐项保存许可证、版本、版权公告、再分发或仅限芯片使用条件和法务/厂商确认记录；商标/一致性宣称单独审批。不能从一个供应商包的许可推断另一供应商、另一版本或 MCU 专用 MCAL 的权利。[ISO](https://www.iso.org/copyright.html) · [NXP](https://mcuxpresso.nxp.com/mcuxsdk/latest/html/) · [ST](https://github.com/STMicroelectronics/STM32CubeH7/blob/master/LICENSE.md)
