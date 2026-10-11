# 0002. 固定标准基线 CP／FO R24-11 与 C99

## 背景

AUTOSAR 不同版次的配置模型、Schema 与 SWS 条款不同，混用会让输入解析、内置规则和生成接口失去单一依据。项目需要一个可下载、可按官方哈希核验的固定版次，并让生成 C 的语言版本与编码规范同样固定。

## 决策

1. 产品首个规范基线为 AUTOSAR **CP／FO R24-11**。官方文件身份以 R24-11 CP／FO `TR_SpecificationHashes.sha256` 清单为准；Schema 为 `AUTOSAR_00053.xsd`。
2. 生成和交付的 C 代码语言目标为 **C99**；编码规范基线见 ADR 0008。
3. 版次与 Schema／ECUC 来源显式绑定；未经规则与往返验证不自动迁移其他版次。R25-11 资料只作为后续升级参考。
4. 官方 PDF、ZIP 与哈希清单只在本地查阅（`docs/official/`，由 `.gitignore` 排除），不随仓库或产品再分发。
5. 当标准正文与现有代码、ABI 清单或测试不一致时，以固定版次的正文为准；DRAFT 条款按原文标注使用，不宣称为成熟标准。

## 后果

- 内置规则、模块定义、生成接口与测试预期都以同一版次核对；跨版本差异需单独立项。
- 修订支持 C11／C18 的 MISRA 修订不构成升级编译标准的授权。
- 合法取得官方原文是标准裁决的前置条件；原文缺失只阻塞相应裁决，不阻塞不依赖该裁决的工作。

## 状态

已接受（2026-09）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-4/R24-11-CONTRACT.md`、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-9/ARCHITECTURE-SPINE.md`（AD-5）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture.md`。
