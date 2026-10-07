# AUTOSAR R25-11 本地参考资料

本目录的官方文件仅供本地查阅，不随开源生成器、源码仓库或发布包再分发。使用、复制、商业利用及商标限制见 [AUTOSAR 官网条款](https://www.autosar.org/standards/classic-platform)和各文件的声明。仓库通过 `.gitignore` 排除官方二进制资料及校验清单，只保留中文索引。

当前产品基线为 CP/FO R24-11。本目录用于后续 R25-11 升级研究，文件与 R24-11 的实现、配置和 Schema 分开使用。

## 版本与来源

- `CP/`：Classic Platform R25-11，内部版本 R4.11.0；[官方发布概览](https://www.autosar.org/fileadmin/standards/R25-11/CP/AUTOSAR_CP_TR_ReleaseOverview.pdf)。
- `FO/`：与 CP 配套的 Foundation R25-11；[官方发布概览](https://www.autosar.org/fileadmin/standards/R25-11/FO/AUTOSAR_FO_TR_ReleaseOverview.pdf)。
- 官方 `AUTOSAR_{CP,FO}_TR_SpecificationHashes.sha256` 列出发布文件的 SHA-256 与类别路径。本地按该类别归档，`download-status.json` 逐项记录来源 URL、预期哈希及下载状态。运行 `python scripts/collect_official_specs.py R25-11` 可重新校验并补取可访问文件；脚本使用系统 `curl.exe` 的证书校验，不关闭 TLS 验证。
- 旧版 CP R20-11 的 26 份本地 PDF 选集已移除；此目录仅保留 R25-11 资料，不能与当前 R24-11 基线混用版本与 Schema。

## 收集与校验结果

2026-09-24 按官方 SHA-256 清单逐件核验：**279/284 个清单条目通过**，其中 CP 217/219、FO 62/65；另保存了 CP、FO 两份校验清单自身。已获取的 266 份 PDF 均有 PDF 文件头，13 个 ZIP 均可完整解压校验，合计约 0.46 GB。逐件 URL、预期哈希和结果见 `download-status.json`。

## 已知发布清单缺口

官方 CP 校验清单列 219 个文件，FO 列 65 个文件，另各有一份校验清单自身。官网搜索提供 CP 218 个、FO 64 个索引项（包括校验清单）。两份清单中的 `MOD_SpecificationsARXML.zip` 和 `TR_ChangeDocumentation.zip` 均未出现在官网搜索中，常用发布目录的 URL 返回 404。FO 搜索还列出 `/AP/` 路径下的 `AUTOSAR_FO_MMOD_MetaModel.zip`；该 URL 返回 ZIP，但其 SHA-256 `87fc98e3bc4dd1f58ed6fe164f24f16f51ee09fcee3437a5ef43ec605efe24bc` 与 FO 清单预期值不一致，未纳入已验证资料。上述五项尚缺通过哈希校验的来源，本目录的发布资料因此仍不完整。核查范围和结果见 `docs/research/autosar-platform/` 下的 CP/FO 清单。

进一步核查仍未发现这五项可通过官方哈希验证的公开来源。官网给出的 MetaModel ZIP 仅 3,105 字节，只有说明与声明、没有模型文件；`AUTOSAR_FO_MMOD_XMLSchema.zip` 已校验，但它是另一种交付物，不能代替 MetaModel。可通过[官方组织联系方式](https://www.autosar.org/about/organization) `admin@autosar.org` 请求 R25-11 发布/文档维护方提供正确下载页或正式发行包，并核对清单哈希与该 ZIP 的差异。具体证据见 `docs/research/autosar-platform/missing-release-archives.md` 和 `metamodel-source.md`；官方未回复前维持缺项状态。

这些 PDF/ZIP 是规范参考资料。外部 ISO 等标准、具体 ECU 配置和 MCU 资料需另外准备；生成器的可用性通过构建和运行测试验证。
