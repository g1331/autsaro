# 主线收敛与新基线（目标，实施中）

本篇属于[架构](README.md)：标注“目标（实施中）”的内容尚未在 `master` 落地，其余描述当前代码；决定的背景与取舍见 [ADR](../adr/)。

以下方向已决定，正在按可独立验证的 PR 分阶段实施；完成前当前代码仍是[架构](README.md)其他各篇所述状态。

## 单一生成主线

新生成的工程全部走同一条 ECU 主线，单组件是 SWC 数为 1 的特例，不按组件数分派链路（[ADR 0010](../adr/0010-single-generation-mainline.md)）。主线按职责命名，不再称 multi。主线已拥有全部 BSW 来源：程序闭包只取自主线运行时目录（目前为 `runtime/bsw`），producer 身份与实际字节一致；Dem、NvM、Security 不在主线程序中，随旧链路删除。不同 target 独立选择，不同时链接互斥的 OS／RTE／BSW producer；`host-batch`、`probe`、`test` 的实际程序闭包分别核验。

## 稳定版本前不保留兼容

旧 profile（`host-can-v1`、单组件 profile）与旧格式工程、交付包按真实结构身份识别后**明确拒绝**，不静默误解析、不迁移、不改写摘要（[ADR 0011](../adr/0011-no-backward-compatibility-before-stable.md)）。用户的 R24-11 ARXML 源文件仍是合法输入，可导入新工程。契约变化仍须 ABI／第三方身份审阅并写入 CHANGELOG；新增标准必填依赖时官方定义保持严格。稳定版本后的兼容策略另定。

## 删除的平行实现（已完成，#33）

- 外部 XSD／MOD 校验模式：设置项、`schema_zip` 状态与全部 `*_legacy` 链路删除；`schema.rs` 的 XSD 校验只作为测试与 acceptance 中检验内置规则的官方对照（[ADR 0012](../adr/0012-remove-external-xsd-mod-mode.md)）。
- 帧／信号 host 链路：两个模板、“主机运行”页、`core/src/host/`、`generator/render.rs`、`arxml/host_profile.rs`、`package_host_reference` 删除（[ADR 0013](../adr/0013-remove-frame-signal-host-chain.md)）。
- **已知差距：**“在工作台内运行生成的 ECU”保留为产品目标，之后基于稳定的 host-batch 契约在主线上重做（#65）；在此之前用 `ecu-tool verify`、工作台的固定主机验证与原生测试验证。

删除前，旧链路中有价值的独立预期（协议、OS 时序、输入预期）先迁到主线并通过；只验证旧链路内部行为的测试才删除。

## Emitter 规则

（[ADR 0014](../adr/0014-emitter-rules.md)）以下规则已实施（#37）；emitter 按模块组织随各模块整理推进。

- 每个输出文件只有一个登记的生成方：ECU 工程的生成文件与运行时资产登记到 `integration/outputs.rs` 的输出表，两个生成方写同一路径、或留给 emitter 的模板未被渲染，都在写出文件前拒绝。MemMap 段、SchM 声明、类型补充等跨模块内容在计划阶段汇总后交给该文件的生成方。
- emitter 只消费计划，不回读、查找替换或追加其他 emitter 已生成的 C 文本；决定配置／接口的最终文本替换移入结构化计划字段。纯占位模板渲染可保留；受信模板（如 `Os_Cfg.h`）只经 `outputs::render` 按锚点渲染一次，锚点缺失时报告诊断。
- emitter 不 panic；计划无法保证的前提返回 `PlanDiagnostic` 并定位源对象。生成与集成模块的模块属性 deny clippy `unwrap_used`、`expect_used`、`panic` 及 `core/clippy.toml` 列出的文本修补与文件嵌入；其余模块启用默认规则集，遗留告警以 `#[expect]` 基线标记，只减不增。
- 重组 emitter 时用重新生成主线样本逐字节比较证明不变，有意差异逐项审阅。

## 仓库组织（目标）

（[ADR 0015](../adr/0015-repository-code-organization.md)）根 workspace、`runtime/bsw`、按职责命名、编号检查与格式版本表已实施（#36）；`runtime/host` 的拆分随交付布局按所有权分类一起完成（#38）。

```
Cargo.toml            # 虚拟 manifest：members = ["core", "src-tauri"]，resolver = "3"，唯一 Cargo.lock 与 target
core/
  build.rs            # 只写 OUT_DIR
  resources/          # 产品内置资源（模板 ARXML 等），产品代码不引用测试目录
  src/{arxml,definitions,rules,integration,generator,execution}/
  tests/
    it/main.rs        # 普通集成测试，一个二进制，内部按能力分模块
    native/main.rs    # required-features = ["native-tests"]
    official/main.rs  # required-features = ["official-oracles"]
    support/  fixtures/<能力>/
src-tauri/            # 位置与名称不变，作为 workspace 成员
runtime/
  bsw/include/        # 标准公共头与 <Mip>_MemMap.h
  bsw/src/            # <Mip>.c 与私有头 <Mip>_Internal.h
  os/  ecu/  host/    # host 只放主机适配，不改标准头
tools/python/src/  tests/python/  tests/desktop/
```

- 产品代码、测试、fixture、工具、bin、目录名、测试名与读者文档不出现开发计划编号，由 `quality` 检查强制。目录和模块按职责命名，不用 multi、legacy、new 等相对词。标准模块文件、头文件和内存段名称按 SWS BSW General 与 RTE 保持不变。
- 仓库工具只分三类：交付工具（`ecu_tools`）、统一开发 CLI（`doctor`、`verify`、`quality`、`c-check`、`assets`）、放在 cargo test／unittest／vitest／node:test 中的测试。测试不做成 CLI 子命令；桌面端到端优先使用 Tauri 官方 WebDriver 与 runner 级隔离。
- 只有持久化或交付格式带版本并登记在格式版本表中；模板 ID 等内部标识不带版本。
- 不为未稳定的边界拆 crate，不建只有一两个模块的分层空目录，不让 `build.rs` 写源码树。

## 生成工程布局（已实施，#38）

（[ADR 0016](../adr/0016-generated-project-layout-by-ownership.md)）

```
<工程>/
  bsw/{include,src}/        静态 BSW 公共头、实现与私有头，只读，受资源摘要约束
  integration/{include,src}/ 主机适配与 ECU 执行 adapter、两个 main
  os/  kernel/              静态 OS 适配、补丁与固定第三方内核（kernel/compat/ 为上游别名头，不在 include 路径中）
  generated/{include,src}/  全部配置生成物：*_Cfg.h、*_PBcfg.c、Rte*、SchM_*、*_MemMap.h、Os_Cfg.h、Ecu 配置；每次生成整体重写
  generated/descriptions/   Host_Implementation.arxml、Os_Arti.arxml、Os_Service.arxml、MODULES.md
  generated/templates/      SWC 骨架（与 live 初始化种子同源），不参与编译
  app/<SWC>/                用户代码快照（用户在 live 工程中修改）
  inputs/                   原始 ARXML 与工程清单快照
  meta/                     交接与集成元数据、台账、seal 期望、资产清单、files.list／files.sha256
  tools/                    离线 ecu-tool 与封存检查（不随配置变化）
  CMakeLists.txt  target.json   从同一清单生成的标准构建描述与 ecu-tool 构建清单
```

文件归属在产生处确定：资产按来源目录映射（`runtime/bsw`→`bsw/`，`runtime/host`／`runtime/ecu`→`integration/`，`runtime/os`→`os/`），emitter 的 `include/`、`src/` 逻辑名相对 `generated/`。`{Mip}_MemMap.h`（`[SWS_MemMap_00032]`）和 RTE 生成头（`[SWS_Rte_07501]`、`[SWS_Rte_07120]`）由生成方交付，台账 owner 为 `generated`。所有 include 目录中的头文件名不区分大小写唯一（`[SWS_BSW_00171]`），生成 `target.json` 时检查。修改配置后重新生成，只有 `inputs/`、`generated/`、`app/`、`meta/` 与根目录的 `README.md`、`target.json`、`CMakeLists.txt` 变化。CMake 与 `ecu-tool` 构建同一组源文件，由 CI 在两个平台核对。主机目标的 MemMap 头归入 `generated/`；接入 MCU 时由集成层显式配置覆盖，不手改生成文件。SWC 的 `{componentTypeName}_MemMap.h`（`[SWS_MemMap_00029]`）随 RTE 段约定一起由 [#51](https://github.com/g1331/autsaro/issues/51) 承接。

## CI 与源码门禁（目标）

两平台完整主线样本与每个实际 main 的分析闭包每轮重新生成和分析；缓存只复用可信依赖／构建中间物，按 runner／target／toolchain／source 身份隔离，不缓存扫描结果。严格门禁要求完整选集、`error=null`、`passed=true`、上游全部 success，接入每次 PR 的必需聚合与分支保护，并用故意违规、缺样本、工具失败、skip／cancel 的真实 PR 验证阻断。当前严格源码门禁仍只在手动运行，见 [测试说明](../development/testing/generated-c.md)。
