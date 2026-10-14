# 目标、工具链与交付

本篇属于[架构](README.md)：标注“目标（实施中）”的内容尚未在 `master` 落地，其余描述当前代码；决定的背景与取舍见 [ADR](../adr/)。

## 目标与工具链边界

| 项 | 当前固定值 |
| --- | --- |
| 主机目标 | `windows-x64-controlled-v1`（MSYS2 GCC 16.1.0 Rev5，PE/TLS）；`linux-x64-controlled-v1`（Ubuntu 24.04 GCC 13.3.0＋binutils 2.42，ELF／pthread）；受控内核补丁见 `runtime/os/patches/` |
| 工具链清单 | `runtime/os/toolchain.json`、`runtime/os/toolchain-linux.json` |
| 内核 | FreeRTOS V11.3.1，commit `054e14f3397023aa83813a65aa065fc4597d481b`，MIT；`third_party/freertos/*source-manifest.json` |
| 工作台 | Rust 由 `rust-toolchain.toml` 固定；npm／Cargo／uv 锁文件 |
| C 分析 | Cppcheck 2.21.0＋固定 MISRA addon 摘要，见 [测试说明](../development/testing/generated-c.md) |

- 目标、工具链、内核、补丁和许可逐项固定版本与摘要，不在构建时取浮动版本。错误宿主或工具身份不符明确失败，不降级到其他编译器。
- **证据按层取得，不互相借用：**浏览器／mock、原生 Tauri／IPC、安装包、离线接收者、主机 ECU 行为、MCU 实时性／ISR／内存段各自取证；未运行记为未执行，失败记为失败。Windows 结果不代替 Linux，主机结果不代替 MCU。
- macOS 原生构建、bundle 与 IPC 未验证。Windows MSI、Linux deb／AppImage 按各自平台配置，安装包验收独立于 build／test。
- 首个硬件候选为 FRDM-A-S32K344，**尚非已验证目标**。供应商 RTD 所标 AUTOSAR 版次若不同于 R24-11，须显式检查 ECUC、配置结构、公开接口与生成物的转换边界；驱动许可与再分发条件单独记录。
- AP（Adaptive Platform）只保留版次与平台边界，未取得真实 AP 工件与运行样本前不合并 CP／AP 模型。

## 交付与离线接收

封存工程按[生成工程布局](mainline.md#生成工程布局已实施38)的所有权目录包含生成源码、受信 runtime 资产、固定内核与补丁、应用快照、所有权台账、`target.json` 构建清单与同源的 `CMakeLists.txt`、`meta/files.list`／`meta/files.sha256` 和 stdlib-only 离线工具。CMake 是供 IDE／CI 使用的标准构建入口，与 `ecu-tool build` 编译同一组源文件；固定工具链身份、目标段检查和 `verify` 只由 `ecu-tool` 承担。接收者只需受支持的 CPython、匹配的 GCC／binutils 与 Git，不依赖 checkout、Rust、Node 或 uv；包内工具以 `-I -S` 运行，只从自身 `tools/` 导入。入口 `tools/ecu-tool.py build|verify` 的用法见 [runtime/README.md](../../runtime/README.md)。

当前交接格式为 `autosar-workbench-handoff-v4`（内置规则、多应用输入、按所有权分目录）。旧 `autosar-workbench-handoff-v3`、`autosar-workbench-handoff-v2`、`autosar-host-handoff-v1` 与 `autosar-ecu-handoff-v1` 在导入时拒绝，其 ARXML 源文件可导入新工程。

只有持久化、交付或协议格式带版本。格式的结构或取值域变化时版本加一，版本号不回收；旧版本按身份明确拒绝，不迁移。模板 ID、生成方 ID 和消息键等内部标识不带版本。

| 格式 | 载体 | 版本 |
| --- | --- | --- |
| `autosar-workbench-handoff` | 包内 `meta/handoff.json`、`target.json` 的 `nativeDelivery` | v4 |
| 所有权台账 `formatVersion` | 包内 `meta/workbench-ownership.json` | 3 |
| `autosar-ecu-integration`；计划描述 `formatVersion` | 包内 `meta/integration.json` | v2；2 |
| `autosar-component-contract` | 组件契约包 `contract.json`、ECU 包 `meta/contract.json` | v2 |
| `autosar-asset-inventory` | `runtime/contracts/assets.json` 及包内 `meta/assets.json` | v2 |
| `autosar-workbench-build-tool-inventory` | 包内 `tools/build-tool-inventory.json` | v2 |
| `autosar-build-target` | 包内 `target.json` | v2 |
| `autosar-ecu-test-inputs`、`autosar-os-generation-timing` | 包内 `meta/verification-inputs.json`、`meta/os-generation-timing.json` | v1 |
| 工程清单 `formatVersion` | 用户工程 `workbench-project.json` | 1 |
| 扩展目录 `formatVersion` | 外部 `catalog.json` | 1 |
| profile `singlecore-ecu-v1`；target `windows-x64-controlled-v1`、`linux-x64-controlled-v1`；协议 `HostBatchV1` | 工程、设置、交付包、主机批处理协议 | v1 |
