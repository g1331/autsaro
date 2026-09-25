# AUTOSAR R24-11 首版产品基线

本目录用于单独收集 Classic Platform（`CP/`）及匹配的 Foundation（`FO/`）R24-11 官方资料。首版生成软件以这组版本为基线；R20-11 选集已移除，R25-11 留作独立升级，配置模型与 Schema 不跨版混用。

官方 PDF、ZIP 与哈希清单仅供本地查阅，不随开源代码再分发；`.gitignore` 排除这些原件。本目录按官方 CP/FO 校验清单收集，**不能把“清单条目齐全”解释成外部标准、具体芯片资料和项目配置也已齐全**。

官方 [CP SHA-256 清单](https://www.autosar.org/fileadmin/standards/R24-11/CP/AUTOSAR_CP_TR_SpecificationHashes.sha256)列 232 项、[FO SHA-256 清单](https://www.autosar.org/fileadmin/standards/R24-11/FO/AUTOSAR_FO_TR_SpecificationHashes.sha256)列 69 项，均未把清单自身计入。运行 `python scripts/collect_official_specs.py R24-11` 按两份清单下载与核验，分类目录沿用清单中的包内路径；结果写入 `download-status.json`。校验失败或官网缺项不得算作已收集。

## 校验结果

2026-09-24—25，官方两份 SHA-256 清单中的 **301/301 个文件**下载并逐项匹配：CP 232/232、FO 69/69，合计 284 份 PDF、17 个 ZIP（约 0.89 GB）；两份校验清单本身另存。PDF 文件头与 ZIP 全部成员完整性检查均通过。FO 元模型 ZIP 包含 `AUTOSAR_FO_MMOD_MetaModel.qea`，独立的 Schema ZIP 包含 `AUTOSAR_00053.xsd`；CP/FO 的 `MOD_SpecificationsARXML.zip` 均在清单中且已通过哈希校验。

R24-11 的 CP/FO 官方清单**没有列出** `TR_ChangeDocumentation.zip`；因此 301/301 的分母不含这两份同名 ZIP，不能宣称已获取它们。可查各 Release Overview 和单份规范的变更记录；跨版本差异仍需单独核对。本地不纳入 R25-11 的不匹配元模型 ZIP，也不把旧版模型当作新版产物。
