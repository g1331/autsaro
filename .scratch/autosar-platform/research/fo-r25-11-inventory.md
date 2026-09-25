# AUTOSAR Foundation R25-11 官方公开清单（2026-09-24）

## 范围、交叉核对与核验方法

- [Foundation 官方入口](https://www.autosar.org/standards/foundation)标示当前版本为 **R25-11**，并指向按平台 `FO`、版本 `R25-11` 过滤的[官方文档搜索](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=)。[Foundation Release Overview](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf)本身也在下表中。版本标识按官方目录记为 `R25-11`；未下载 PDF 正文，故不将可能的内部版本号或 schema 号当作已独立核验事实。
- 官方搜索显示 **64 Documents found**，共四页（20 + 20 + 20 + 4），下表逐项记录搜索结果的原始文件名及官方 URL，未猜测 URL。逐一发送不取正文的 **HTTP HEAD**：64/64 返回 `200`，并返回匹配扩展名的 `Content-Type`（PDF 54、ZIP 9、`application/octet-stream` 的 SHA-256 清单 1）。这只能证明检查当时链接可访问，**不证明文件下载完成、校验通过或 ZIP 内部内容**；实际保存时仍须核查响应体及官方哈希。因本地抓取环境的证书链校验失败，HEAD 检查使用不验证 TLS 证书的连接；官方域名/路径来自 HTTPS 官方页面，不应将该探测视为 TLS 身份验证。
- 分类：**搜索可访问 64 项**＝FO 目录 63 项（PDF 54、ZIP 8、`.sha256` 1）＋**官方 FO 搜索列出、实际位于 AP 目录的 ZIP 1 项**（表末 `AUTOSAR_FO_MMOD_MetaModel.zip`）。若按 URL 归档 FO，请将此项单独记录并和 AP 清单去重，不要把它改造成未列出的 `/FO/` URL。
- 官方搜索页来源：[第 1 页](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=)、[第 2 页](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=&tx_solr%5Bpage%5D=2)、[第 3 页](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=&tx_solr%5Bpage%5D=3)、[第 4 页](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=&tx_solr%5Bpage%5D=4)。下表 `索引页` 可回溯到各自来源；`HEAD` 一律为检查时 `200`。原链接的相对路径以 `https://www.autosar.org/` 为基址解析。
- 再与官方[FO R25-11 Specification Hashes](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256)交叉核对：哈希清单覆盖 **65 个文件名**，官网搜索只覆盖其中 **63/65**；官网搜索第 64 项为 `.sha256` 清单本身。哈希独有 **2 个 ZIP**（下文列出），两份官方来源的**并集是 66 个文件名**，不能把搜索结果 64 项误称为完整发布内容；其中 2 项没有找到可证实的官方公开下载直链。

## 可机读清单

| 文件名 | 官方目录平台 | 类型 | 官方直接 URL | 索引页 | HEAD |
|---|---|---|---|---:|---:|
| `AUTOSAR_FO_MOD_GeneralBlueprints.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MOD_GeneralBlueprints.zip | 1 | 200 |
| `AUTOSAR_FO_MOD_MiscSupport.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MOD_MiscSupport.zip | 1 | 200 |
| `AUTOSAR_FO_ASWS_HealthMonitoring.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_ASWS_HealthMonitoring.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_HealthMonitoring.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_HealthMonitoring.pdf | 1 | 200 |
| `AUTOSAR_FO_EXP_SystemHealthMonitoring.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_SystemHealthMonitoring.pdf | 1 | 200 |
| `AUTOSAR_FO_MMOD_XMLSchema.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip | 1 | 200 |
| `AUTOSAR_FO_TPS_StandardizationTemplate.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_StandardizationTemplate.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_SecureOnboardCommunication.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_SecureOnboardCommunication.pdf | 1 | 200 |
| `AUTOSAR_FO_EXP_QoSPoliciesInTheScopeOfSOMEIP.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_QoSPoliciesInTheScopeOfSOMEIP.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_NetworkManagement.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_NetworkManagement.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_LogAndTrace.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_LogAndTrace.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_DebugTraceProfile.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_DebugTraceProfile.pdf | 1 | 200 |
| `AUTOSAR_FO_EXP_TimeSensitiveNetworkFeatures.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_TimeSensitiveNetworkFeatures.pdf | 1 | 200 |
| `AUTOSAR_FO_PRS_SecOCProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_SecOCProtocol.pdf | 1 | 200 |
| `AUTOSAR_FO_PRS_NetworkManagementProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_NetworkManagementProtocol.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_IPsecProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_IPsecProtocol.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_SOMEIPServiceDiscoveryProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_SOMEIPServiceDiscoveryProtocol.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_SOMEIPProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_SOMEIPProtocol.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_TimeSync.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_TimeSync.pdf | 1 | 200 |
| `AUTOSAR_FO_RS_VDP.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_VDP.pdf | 1 | 200 |
| `AUTOSAR_FO_TR_Glossary.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_Glossary.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_AbstractPlatformSpecification.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_AbstractPlatformSpecification.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_LogAndTraceExtract.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_LogAndTraceExtract.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_ARXMLSerializationRules.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_ARXMLSerializationRules.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_SecurityExtractTemplate.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_SecurityExtractTemplate.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_XMLSchemaProductionRules.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_XMLSchemaProductionRules.pdf | 2 | 200 |
| `AUTOSAR_FO_MOD_GeneralDefinitions.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MOD_GeneralDefinitions.zip | 2 | 200 |
| `AUTOSAR_FO_PRS_SOMEIPProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_SOMEIPProtocol.pdf | 2 | 200 |
| `AUTOSAR_FO_TR_Features.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_Features.pdf | 2 | 200 |
| `AUTOSAR_FO_TPS_FeatureModelExchangeFormat.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_FeatureModelExchangeFormat.pdf | 2 | 200 |
| `AUTOSAR_FO_RS_E2E.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_E2E.pdf | 2 | 200 |
| `AUTOSAR_FO_RS_MACsec.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_MACsec.pdf | 2 | 200 |
| `AUTOSAR_FO_RS_IEEE1722.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_IEEE1722.pdf | 2 | 200 |
| `AUTOSAR_FO_RS_Diagnostics.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_Diagnostics.pdf | 2 | 200 |
| `AUTOSAR_FO_EXP_DiagramSource.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_DiagramSource.zip | 2 | 200 |
| `AUTOSAR_FO_MOD_Features.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MOD_Features.zip | 2 | 200 |
| `AUTOSAR_FO_EXP_SWArchitecturalDecisions.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_SWArchitecturalDecisions.pdf | 2 | 200 |
| `AUTOSAR_FO_EXP_SafetyOverview.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_SafetyOverview.pdf | 2 | 200 |
| `AUTOSAR_FO_RS_ProjectObjectives.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_ProjectObjectives.pdf | 2 | 200 |
| `AUTOSAR_FO_EXP_SecurityOverview.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_EXP_SecurityOverview.pdf | 2 | 200 |
| `AUTOSAR_FO_TR_AutosarModelConstraints.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_AutosarModelConstraints.zip | 3 | 200 |
| `AUTOSAR_FO_TR_XMLSchemaSupplement.zip` | FO | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_XMLSchemaSupplement.zip | 3 | 200 |
| `AUTOSAR_FO_TPS_GenericStructureTemplate.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TPS_GenericStructureTemplate.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_DDSCommunicationProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_DDSCommunicationProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_IntrusionDetectionSystem.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_IntrusionDetectionSystem.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_LogAndTraceProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_LogAndTraceProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_DDSServiceDiscoveryProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_DDSServiceDiscoveryProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_TimeSyncOverEthernetProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_TimeSyncOverEthernetProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_V2XRemoteAccessLayer.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_V2XRemoteAccessLayer.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_TimeSyncOverCANProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_TimeSyncOverCANProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_RS_DataDistributionService.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_DataDistributionService.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_VDP.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_VDP.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_E2EProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_E2EProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_PRS_SOMEIPServiceDiscoveryProtocol.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_PRS_SOMEIPServiceDiscoveryProtocol.pdf | 3 | 200 |
| `AUTOSAR_FO_TR_SpecificationHashes.sha256` | FO | SHA256 | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256 | 3 | 200 |
| `AUTOSAR_FO_TR_ReleaseOverview.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf | 3 | 200 |
| `AUTOSAR_FO_RS_Safety.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_Safety.pdf | 3 | 200 |
| `AUTOSAR_FO_RS_Firewall.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_Firewall.pdf | 3 | 200 |
| `AUTOSAR_FO_RS_IntrusionDetectionSystem.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_RS_IntrusionDetectionSystem.pdf | 3 | 200 |
| `AUTOSAR_FO_TR_ListOfKnownIssuesSecureHardwareExtensions.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ListOfKnownIssuesSecureHardwareExtensions.pdf | 3 | 200 |
| `AUTOSAR_FO_TR_SecureHardwareExtensions.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SecureHardwareExtensions.pdf | 4 | 200 |
| `AUTOSAR_FO_TR_SecurityEventsSpecification.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SecurityEventsSpecification.pdf | 4 | 200 |
| `AUTOSAR_FO_TR_TimingAnalysis.pdf` | FO | PDF | https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_TimingAnalysis.pdf | 4 | 200 |
| `AUTOSAR_FO_MMOD_MetaModel.zip` | AP | ZIP | https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_FO_MMOD_MetaModel.zip | 4 | 200 |

## 官方哈希清单独有、官网搜索未列出的 ZIP

| 文件名 | 官方哈希清单中的相对路径 | 官方直接 URL | 已核实的 HTTP HEAD 证据 |
|---|---|---|---|
| `AUTOSAR_FO_MOD_SpecificationsARXML.zip` | `ReleaseDocumentation/AUTOSAR_FO_MOD_SpecificationsARXML.zip` | 未找到 | `https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MOD_SpecificationsARXML.zip` → **404 Not Found** |
| `AUTOSAR_FO_TR_ChangeDocumentation.zip` | `ReleaseDocumentation/AUTOSAR_FO_TR_ChangeDocumentation.zip` | 未找到 | `https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ChangeDocumentation.zip` → **404 Not Found** |

来源：官方 [Specification Hashes](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256) 第 56–57 行；404 为 2026-09-24 使用系统 `curl.exe -I`（未禁用 TLS 验证）对上表确切 URL 获取的服务端响应。哈希清单相对路径是发行包内部路径，**不是网站直链**。目前只可断定这两个直观 URL 不可下载，官方搜索亦未列出它们；**不能证明它们在官方其他位置绝对不可访问**，也没有证据可提供替代 URL。不要将 404 URL 纳入下载清单。若未来获得官方整包，应用清单中的 SHA-256 校验。

## 完整性边界

两份官方来源合并识别 **66 个文件名**，其中哈希清单所列 **65 项中的 63 项（63/65）**经官网 FO/R25-11 搜索列出并于检查时 HEAD 200，另外搜索列出的哈希清单本身亦 HEAD 200；剩余 **2 个哈希所列 ZIP 仅见于哈希清单、公开下载位置未确定**。官网搜索的 64 项可逐一获取，但不能因此声称 FO 发布内容完全可公开下载；也不能排除官方搜索与哈希清单均未覆盖的其他文件。官方搜索没有单独列出 `.xsd`、`.arxml` 文件；`AUTOSAR_FO_MMOD_XMLSchema.zip`、`AUTOSAR_FO_TR_XMLSchemaSupplement.zip`、`AUTOSAR_FO_MOD_GeneralDefinitions.zip`、`AUTOSAR_FO_MOD_GeneralBlueprints.zip`、`AUTOSAR_FO_MOD_MiscSupport.zip`、`AUTOSAR_FO_TR_AutosarModelConstraints.zip`、`AUTOSAR_FO_MOD_Features.zip` 与位于 AP 路径的 `AUTOSAR_FO_MMOD_MetaModel.zip` 等均按官方 ZIP 原名列出，**没有下载/展开，不能声称其中具体 XSD/ARXML 文件名或数量**。`AUTOSAR_FO_EXP_DiagramSource.zip` 也属于 FO 目录的 ZIP，不是 PDF。
