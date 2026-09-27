# R25-11 CP／FO 四个发行档案的官方获取路径调查（2026-09-24）

## 结论

**尚未核实这四个 ZIP 的官方可下载地址或包含它们的完整发行包地址。**AUTOSAR 官方 R25-11 哈希清单明确列出它们，但 [CP R25-11 文档搜索](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B0%5D=category%3AR25-11&tx_solr%5Bfilter%5D%5B1%5D=platform%3ACP)和 [FO R25-11 文档搜索](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=)均未列出相应下载项；现有对网站常见扁平路径及哈希清单所用 `ReleaseDocumentation/` 分类路径的探测返回 404（核查记录见 [CP 清单](cp-r25-11-inventory.md#公开链接缺口与-zip)和 [FO 清单](fo-r25-11-inventory.md#官方哈希清单独有官网搜索未列出的-zip)）。**这不是“四个文件从未发布”或“必须付费/登录才能下载”的证明**，也不能把哈希清单中的相对路径推作网站直链。AUTOSAR 的[发布公告](https://www.autosar.org/news-events/detail/release-r25-11-is-now-available)称 R25-11 对应文档可在官网取得；[Classic](https://www.autosar.org/standards/classic-platform)和 [Foundation](https://www.autosar.org/standards/foundation)入口将用户引向官网搜索，但没有为这四件 ZIP 提供已核实的链接。

| 平台 | 官方哈希清单中的发布包相对路径 | 官方 SHA-256 | 官网直链 |
|---|---|---|---|
| CP | `ReleaseDocumentation/AUTOSAR_CP_MOD_SpecificationsARXML.zip` | `db5b078ce1de2b27820f63f47bf97906c5103a7f5b613df3539a8d31a79a1861` | 未核实 |
| CP | `ReleaseDocumentation/AUTOSAR_CP_TR_ChangeDocumentation.zip` | `2551d3b5dae1ddd49bc79f7f2d16660a35fe75a884770325ec2ed84f9f78f7e5` | 未核实 |
| FO | `ReleaseDocumentation/AUTOSAR_FO_MOD_SpecificationsARXML.zip` | `ec01a806dc85b3d69f9b8dadc810f7a8181df17fe6d59e2feb230941c5d4479d` | 未核实 |
| FO | `ReleaseDocumentation/AUTOSAR_FO_TR_ChangeDocumentation.zip` | `242757882680b5125d19c19357410ff2fda64327e3206a5fcd5bded886ca4465` | 未核实 |

来源：[CP 官方 `SpecificationHashes.sha256` 第 192–193 行](https://www.autosar.org/fileadmin/standards/R25-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256)、[FO 官方 `SpecificationHashes.sha256` 第 56–57 行](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256)。其中 `ReleaseDocumentation/` 是清单的**包内目录**，不是网页目录；这两个清单证明预期名称与校验值，**不证明文件当前位于公开站点**。另见官方 [FO Standardization Template，第 17、23 页](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_StandardizationTemplate.pdf)：`ChangeDocumentation` 指平台变更文档，`SpecificationsARXML` 指 ARXML 格式的规格；同名或相近用途的其他 ZIP 不能自动代替这些已列名的发行档案。

## 可执行的官方请求渠道

1. 从 [AUTOSAR 组织页面](https://www.autosar.org/about/organization)公布的官方联络地址 **[admin@autosar.org](mailto:admin@autosar.org)** 发邮件。该页将其 Support Functions 定义为公众信息请求的主要联络人及 AUTOSAR 标准的技术支持。也可在 [Classic Platform 页面](https://www.autosar.org/standards/classic-platform)的 **Join AUTOSAR** 表单选择 **“AUTOSAR access but already partner”**（仅在确实已有伙伴身份时）；没有伙伴身份仍可使用公开联系邮箱。没有证据表明伙伴门户一定存有这四件文件，故应请求核实，而非宣称登录即可下载。
2. 写明 **Release R25-11、CP/FO、上表四个精确文件名及各自 SHA-256**；说明官网对应平台的 R25-11 搜索不显示这四件，常见扁平和 `ReleaseDocumentation/` URL 返回 404。请 AUTOSAR 确认它们是否仍提供、是否有官方改名/更新版本或发布勘误，提供**可验证的官方下载页面/路径或正式发行包获取方法**，并解释若实际字节与官网清单不符时应遵循哪份官方校验值。可援引 [CP Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/CP/AUTOSAR_CP_TR_ReleaseOverview.pdf)及 [FO Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf)核对发行版本；两者的“Release Documentation”节不是这四个 ZIP 的下载链接。
3. 收到官方链接或发行包后，按上表**逐个文件的原始字节**计算 SHA-256；若不匹配，保留 URL、时间、响应及实测值，向 AUTOSAR 请求修正/勘误，不应改名、借用不同版本文件或将缺项标记为已完成。现有公开搜索中的其他 CP／FO 模型 ZIP 不是这四项的已核实替身；官网也没有在上述入口明确给出含四项的全量 CP／FO R25-11 ZIP。网站对下载的[用途声明](https://www.autosar.org/standards/foundation)是“INFORMATION ONLY”，商业利用另涉及 AUTOSAR 伙伴授权；**许可提示不等于本次 404 的原因**。

调查边界：没有下载或提交二进制文件，未使用第三方镜像；未找到官方可用直链前，这四件均保持“待获取/待校验”。
