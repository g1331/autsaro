# FO R25-11 `AUTOSAR_FO_MMOD_MetaModel.zip` 来源与校验冲突（2026-09-24）

## 结论

**未找到可验证且与官方 SHA-256 一致的 R25-11 MetaModel ZIP 下载地址；该项仍未解决，不能标记为已获取。** [Foundation R25-11 发布概览](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf)第 3 章明确将 `FO MMOD MetaModel`（Meta Model）与 `FO MMOD XMLSchema`（Meta Model-generated XML Schema）列为两个不同交付物。[官方 FO 哈希清单](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256)第 22–23 行也分别给出两份 ZIP 的校验值。官网搜索虽然把目标文件列在 FO/R25-11 下，实际链接却指向 `/R25-11/AP/`；该链接可下载，但 **SHA-256 不符，而且压缩包没有模型文件**。不宜通过重命名、用 XML Schema ZIP 或上一版本模型替代它。

## 可复核证据

| 来源 / 检查 | 结果 |
|---|---|
| [官方 FO R25-11 搜索第 4 页](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=&tx_solr%5Bpage%5D=4) | `AUTOSAR_FO_MMOD_MetaModel.zip` 的原始链接为 [`https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_FO_MMOD_MetaModel.zip`](https://www.autosar.org/fileadmin/standards/R25-11/AP/AUTOSAR_FO_MMOD_MetaModel.zip)，虽然搜索条目标成 FO。由 [Foundation 入口](https://www.autosar.org/standards/foundation)进入 FO/R25-11 搜索也是此结果；[Adaptive Platform 入口](https://www.autosar.org/standards/adaptive-platform)另有 AP/R25-11 搜索，并未提供经验证的另一份 FO 模型 ZIP。URL 的 `/AP/` 是官网索引原值，不应自行改写为 `/FO/`。 |
| [官方 FO R25-11 `.sha256` 清单](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256) | `MethodologyAndTemplates/AUTOSAR_FO_MMOD_MetaModel.zip` 的**预期 SHA-256**：`75aefbf9d5eac9623a2619f183db20f0815db8e3bb824a5c6cb7a377675e1ea9`。这里的相对路径是发布包中的分类路径，并非已证实的网站 URL。 |
| 按上述**官网搜索提供的 AP URL**实际下载，使用系统 `curl.exe`，本地 `certutil.exe -hashfile ... SHA256` 计算 | HTTP 200，`application/zip`，`Content-Length: 3105`；**实际 SHA-256**：`87fc98e3bc4dd1f58ed6fe164f24f16f51ee09fcee3437a5ef43ec605efe24bc`，与清单不一致。解开目录可见 `AUTOSAR_FO_MMOD_MetaModel/_readme.txt`、`_disclaimer.txt`、`.DS_Store` 和 `__MACOSX` 元数据，共 8 个 ZIP 条目，**没有 UML/模型文件**。其中 `_readme.txt` 自称 Document ID 059、Foundation R25-11、2025-11-27，说明它并非简单返回的 HTML 错误页，但不能证明它是完整模型。 |
| [官网文件名搜索](https://www.autosar.org/search?tx_solr%5Bq%5D=AUTOSAR_FO_MMOD_MetaModel.zip)与[官方 GitHub 公开仓库列表](https://github.com/AUTOSAR?tab=repositories) | 官网文件名搜索列有 R23-11、R24-11 的旧版 FO ZIP 及上述 R25-11/AP 链接，未找到第二条 R25-11 模型下载链接；AUTOSAR GitHub 公开仓库列表只有 `capi`，没有这份 R25-11 MetaModel 的公开仓库。搜索未找到≠证明官方不存在未索引文件或内部交付包。 |
| [Foundation R25-11 发布概览，第 4 章](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf) | 已列出的已知技术缺陷只涉及 `FO PRS TimeSyncOverCANProtocol`，未解释上述 MetaModel ZIP/哈希矛盾；**不能据此断言不存在后续勘误**。 |

官网搜索以外已按项目此前检查过的直观 FO 平铺及 `MethodologyAndTemplates` 位置均返回 **404**；这只是对那些具体地址的否定，不是另一个经官网确认的下载链接，也不能由 404 推断文件绝对不存在。不要把这些拼出的地址写入正式下载清单。项目当前 `docs/official/R25-11/download-status.json` 将本项标为 `hash-mismatch`，与本次独立下载结果一致。

## XML Schema 与工具实现不是目标 ZIP

[官方清单](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256)给 `AUTOSAR_FO_MMOD_XMLSchema.zip` 的 SHA-256 为 `2d4e7ac2d23616260e8ed53906575d64cde68edf556782273996eb68dcf9fa0d`；[官网 FO 搜索](https://www.autosar.org/search?tx_solr%5Bfilter%5D%5B1%5D=platform%3AFO&tx_solr%5Bfilter%5D%5B2%5D=category%3AR25-11&tx_solr%5Bq%5D=)提供独立的[官方 XMLSchema ZIP](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_MMOD_XMLSchema.zip)。本地已下载该 ZIP 并核算 SHA-256 一致；其 `_readme.txt` 标为 Document ID 230，档案含 `AUTOSAR_00054.xsd`、`autosar.soc`、`xml.xsd`，而不是 Meta Model 的 UML 原档。[发布概览的版本映射表](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf)确认 R25-11 对应 `AUTOSAR_00054` / R4.11.0。可把校验成功的 XSD 单独用于 schema 工作，**不能填补 MetaModel 缺口**。

[官方 Artop 页面](https://www.autosar.org/working-groups/cross-standard/artop)称 Artop 4.19 为 AUTOSAR 25-11 更新其 `AUTOSAR.metamodel454`，其 SDK/Technology Demonstrator 经 AUTOSAR artifact server 提供，**仅 AUTOSAR Partners 可下载**，并给出 Artop 产品联系邮箱 `artop-support@autosar.org`。这是一条可能用于**工具层模型实现**的官方合作伙伴途径，**没有证据表明 Artop SDK 内含上述同名字节完全一致、能通过 FO `.sha256` 的发布 ZIP**；不能把它记为该 ZIP 的已确认来源。

## 下一步官方行动

使用 AUTOSAR [Organization 官方联系方式](https://www.autosar.org/about/organization)中的普通联络地址 `admin@autosar.org`（该页另列的 `comm.support@autosar.org` 明确是 media contact，不应冒充技术支持），请求转给 Foundation 发布/文档维护方；已有合作伙伴渠道者可同时向内部 AUTOSAR 发布门户/接口核实**正式 FO R25-11 发行包**。附上目标文件名、发布包相对路径、预期及实际两个完整 SHA-256、官网 FO 搜索指向 AP 的 URL、3,105 字节/无模型文件的 ZIP 列表、测试日期；请求 **(a)** 一条可下载并经预期 SHA-256 校验的官方文件或正式发行包获取路径，或者 **(b)** 官方更正后的哈希与重新发布/勘误说明。收到答复后再下载、算 SHA-256 并检查 ZIP 成员，核对成功前维持 `hash-mismatch`/未完成状态。
