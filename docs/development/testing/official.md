# 官方对照

`official` 二进制把内置规则、模块定义和生成的 OS 描述与 R24-11 官方 XSD、ECUC MOD 及建模示例档案比较。档案受 AUTOSAR 条款限制，不入库、不随产品分发；这三份档案与条款核算用的 `AUTOSAR_CP_MOD_SpecificationsARXML.zip` 的地址、默认位置、环境变量和 SHA-256 登记在 `core/resources/official.json`。本机用下面的命令下载到登记位置（已设置 `AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE`／`AUTOSAR_SAMPLE_ARCHIVE`／`AUTOSAR_SPECIFICATIONS_ARCHIVE` 时下载到它们指向的位置）：

```powershell
uv run --locked python -m autosar_tooling fetch-official
```

已存在的文件只校验、不覆盖，哈希不一致即失败；新下载的文件哈希不一致时丢弃。`www.autosar.org` 在 TLS 握手中不发送中间证书，命令从证书 AIA 地址取得 Starfield Secure Certificate Authority - G2，校验固定指纹后与系统信任库一起使用，不关闭证书校验。站点更换证书后指纹校验会失败，需审阅新证书链后再更新 `autosar_tooling/official.py` 中的固定值。

CI 中的 `Official archive comparison`（`acceptance.yml`）在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` runner 上运行 `fetch-official`、`cargo test --locked -p autosar-config-core --features official-oracles --test official --no-fail-fast` 和条款处置核算（见[标准基线 §5](../../standards/README.md#5-条款处置与核算)）：`core/resources/requirements/` 中已提交清单的文档从档案做完整检查，并核对清单与档案一致；其余被引用的文档只检查失效引用，列表在工作流的 `Requirement dispositions` 步骤中。档案按 `official.json` 的哈希存入本仓库的 Actions 缓存，不作为 artifact 上传，日志只记录路径和哈希。触发方式：

- 手动运行（workflow_dispatch）；
- 每天对 master 定时运行一次，该提交已有成功运行时跳过；
- 修改 `core/src/definitions*`、`core/src/schema.rs`、`core/src/rules*`、`core/resources/official.json`、`core/resources/requirements/`、`core/tests/official/`、`docs/standards/`、下载或核算入口或该工作流的 PR。未登记文档的测试编号引用变化由每日运行发现，模块整改 PR 应在本地运行核算。

它不属于 `CI required`：外部站点不可用不阻塞合并，PR 上的失败需要自行查看。已整改模块的处置由必需检查保护：`quality` 任务运行 `uv run --locked python -m autosar_tooling requirements`，按提交的清单完整核算，不需要档案，只改 `docs/standards/` 的 PR 也会运行；`verify --scope all` 包含同一检查。清单过期由本工作流发现。下载、证书或哈希失败时，`Download and verify official archives` 步骤失败并标注 `Official archives unavailable`，测试不会运行；测试失败标注 `Official comparison failed`，核算失败标注 `Requirement dispositions failed`。

完整检查入口 `uv run --locked python -m autosar_tooling verify --scope all --base <基准提交>` 组合原有完整检查，不安装依赖，不编排另一套任务或写汇总报告。失败直接保留原输出和退出状态。
