# 变更记录

**简体中文** · [English](CHANGELOG.md)

## Unreleased

- 新增 `python -m autosar_tooling fetch-official`：把官方 R24-11 XSD、ECUC MOD 和建模示例档案下载到 `core/resources/official.json` 登记的位置，并按固定 SHA-256 校验；建模示例档案也已固定哈希。`official` 对照套件改在 GitHub 托管的 Linux 与 Windows runner 上运行（手动、每日定时，以及修改定义、规则或官方测试的 PR），不再依赖未注册的自托管 runner；它不属于必需检查。
- 根目录面向读者的文档默认使用英文，完整中文版以 `.zh-CN.md` 后缀保留；更新双向语言入口。

- 项目原创代码与文档采用 Apache-2.0，补齐包元数据及交付工程的 LICENSE/NOTICE；第三方保留原许可。
- 开发直接使用 npm、Cargo 和 uv，移除重复任务编排；工具源码、测试和可选技能维护分开组织。
- 开发 Python 支持 3.12+，离线工具支持 3.11+，不再要求特定补丁版本；前端使用 Vitest。
- 拆分工程、编辑、交付与设置逻辑，保留 IPC 和原子提交边界；草稿变化使旧批次预览失效。
- 资源路径和工具变化产生新的可信身份，旧封存包仍需匹配原生产者，不就地改写。
- 拆分基础、官方对照、原生运行测试，增加前端行为回归。
- 增加贡献指南、基础 CI、专用验收入口和资产维护说明。
- 补齐运行时字节身份所需的 Git 行尾规则。
- 只含一个应用组件的工程改由多组件主线规划；只有服务组件的组合以 `APPLICATION_COMPONENT_MISSING` 拒绝。
- 主线 Dcm 对 DID 数超过所配置 `DcmDspMaxDidToRead` 的 ReadDataByIdentifier 请求回 NRC 0x13（SWS_Dcm_01335）。`Dcm_ConfigType` 新增 `max_did_to_read`（未配置为 0），计划描述记录 `maxDidToRead`。
- 主线 CanTp 超时时报告 `CANTP_E_RX_COM`／`CANTP_E_TX_COM` 运行时错误（SWS_CanTp_00229）；生成的主机目标经 Det runtime callout 接收，并作为 HostBatch 传输失败报告。
- 含一个应用组件、一个 32 位 CAN 接收与发送信号及应用 DID 的主线工程交付 `verification/inputs.json`，`ecu-tool verify` 据此检查参考回显行为。
- 主线计划对大于 255 的 CanIf PDU 句柄以 `CANIF_HANDLE_WIDTH` 拒绝（交付的 `PduIdType` 为 `uint8_t`），不再报告 `CAN_ID_CONFLICT`；缺失的选定 BSW 入口在引用它的模块描述处报告，不再指向无关对象。
- 主线规划以 `SYMBOL_PRODUCER_DUPLICATE` 拒绝遮蔽生成 OS 计数器服务生产者（`OsService_<Counter>_*`、`Rte_Call_OsService_*`）的 runnable 符号。
- **不兼容：**ECU 生成主线 profile 更名为 `singlecore-ecu-v1`（原 `singlecore-multi-swc-v1`），所有内置规则校验的工程都在主线上规划，包括只有一个应用组件的工程。
- **不兼容：**内置模板 `standard-ecu`（原 `standard-ecu-v1`）在主线上创建含 ComM／BswM 模式管理的单应用组件工程；其仅创建一次的应用种子即 `ecu-tool verify` 检查的参考回显应用。
- **不兼容：**以 `epic4-win64-sr-cs-v1` 或 `singlecore-multi-swc-v1` 记录的工作台工程与原生交付包在打开时拒绝，不修改文件；桌面端不再导入 `autosar-ecu-handoff-v1`。其中的 ARXML 源文件可导入新工程。
- **不兼容：**删除帧／信号 host 链路（模板 `can-empty-v1`、`can-signals-v1`，帧／信号与诊断／DTC 编辑、主机源码交付与构建、“主机运行”页的双 ECU 与诊断运行）、单组件链路及外部 XSD／MOD 校验模式。以 `host-can-v1` 记录的工程、`target.json` 为 `profile: "host"` 的交付包和 `autosar-host-handoff-v1` 交接包在打开时拒绝，不修改文件；其 ARXML 源文件可导入新工程。新建工程只提供主线模板 `standard-ecu`。工作台暂不能交互式运行生成的 ECU（#65），主机行为用包内 `ecu-tool verify`、工作台的固定主机验证和原生测试验证。
- 设置文件中旧的 `xsdArchive`／`modArchive` 不再导致启动失败：工作台忽略它们并在设置中提示，下次保存设置时移除；工作台不再读取 `AUTOSAR_XSD_ARCHIVE`／`AUTOSAR_MOD_ARCHIVE`。官方档案只用于开发期的 `official-oracles` 对照测试。
- **契约变化：**组件契约包改为随附主线 `Platform_Types.h` 与 `Std_Types.h`，`Rte_Type.h` 只包含 `Std_Types.h`；此前 `Rte_Type.h` 自行定义 `uint8`／`uint32`，所依赖的旧 `Std_Types.h` 中 `boolean` 为 `uint8_t`，主线 `Platform_Types.h` 中为 `_Bool`。按旧契约编译的应用需用新契约包重新编译。
- `Ecu_Target.h` 删除只服务旧链路的 `Ecu_TargetInitializeRte` 声明，`Ecu_TargetReportTransportTimeout` 不再受 `ECU_MULTI_COMPONENT` 条件保护（签名不变）；`Ecu_Target.c`、`Ecu_HostBridge.c`、`SchM_Diagnostic_Host.c` 与模板 `Ecu_Config.c.in` 删除主线始终不编译的旧分支。这些文件的资产摘要随之改变；除删除的该声明外，主线预处理后的源码与生成的 `src/Ecu_Config.c` 不变，不涉及第三方来源。
- 删除只服务旧链路的运行时资产（`runtime/src`、`runtime/include`、`runtime/host`、参考包说明与向量、`bsw-v1.json` 等）；13 个主线 BSW 源文件改由 `assets-v1.json` 直接登记，字节不变。主线交付包不再包含没有程序编译或使用的 `src/Ecu_Diagnostic.h`、`tools/ecu/templates/Application.c.in` 与 `Rte.c.in`、重复的 `tools/multi/include` 头文件及 `inventory/contracts/bsw-v1.json`；交付的 `include/Rte_Type.h` 不再直接包含 `<stdint.h>`。包内 `ecu-tool build` 不再接受 `--mode host`，`ecu-tool verify` 不再接受 `--report-path`。删除开发命令 `bsw-catalog` 与 `input-oracles`，以及桌面验收的 `--builtin-only` 选项（内置场景成为唯一场景）；C 静态分析样本缩减为三个主线样本。
- 集成面板的 CAN ID 与应用周期编辑也适用于主线工程；Com Tx 主函数与应用共用触发器时，其时基和发送 PDU 周期随应用周期一同修改。
- 主线 BSW 来源只取自主线运行时目录：主机 CAN 驱动（Can、Can_Host、Can_HostLock）与 Ecu_Status 迁入该目录，字节不变，资产摘要不变，仅记录路径变化。公开签名、布局、句柄与 wire 格式无变化。主线交付不再包含任何程序都不编译的 5 个头文件（旧聚合视图 `include/Can.h`、`Dem.h`、`NvM.h`、`Security.h`、`Ecu_DcmCallbackTypes.h`）；参与编译的源文件和头文件逐字节一致。
- 主线生产者元数据与交付字节一致：DID 读取服务的消费者为主线 Dcm，CanTp／Dcm 调度头与 OS 计数器服务记录实际声明文件，`integration.json` 的 `originalBswSources` 只列出原样交付的文件（生成器扩展过的模板属于生成输出）。
- HostBatch 确认每条输出帧前不再等满一个主机定时器周期：目标通过新增的 `Os_TargetNotifyNative` 唤醒等待中的控制线程，`Os_TargetWaitTick` 收到该唤醒时提前返回 `E_OS_NOFUNC`。Windows 上 1 ms 定时等待实际约 15.6 ms，带周期输出的 1000 ms 批次在负载下可能超过 5000 ms 的 HostBatch watchdog（#61）。期限和汽车时间不变。
- 生成的主机 ECU 对在某 epoch 周期之前发出的帧，无论宿主线程何时报告，都在该周期之后提交其发送确认。由发送确认启动的 CanTp 定时器（如流控之后的 N_Cr）因此不会再随宿主调度提前一个周期到期，包内 `ecu-tool verify` 的 N_Cr 检查也不再偶发失败（#86）。
- 删除开发命令 `probe`、`os`、`os-arti-native`、`os-counter-service` 与 `protocol-oracles`；CLI 保留 `doctor`、`verify`、`quality`、`c-check`、`assets` 与 `desktop`。原生 OS suites 改由 `unittest` 运行（`tests/python/native/test_os_suites.py`，可用 `-k` 筛选），协议对照向量由 `cargo test --test it -- protocol_oracles::` 检查，受管进程探针改为测试辅助脚本 `tests/python/process/probe.py`。core 集成测试按能力分为 `it`、`native`（`native-tests`）与 `official`（`official-oracles`）三个二进制；必需 CI 在 Windows 与 Linux 运行全部原生套件及 OS suites，C 静态分析增加不含诊断路由的多组件样本 `no-diagnostic`。交付的 `runtime/os/README.md` 改为新的 OS suite 命令。
- Linux 与 macOS 上的受管执行以进程监督器的认证就绪响应为准，不再以 socket 文件出现作为就绪，监督器启动后立即运行的命令不会再偶发 `Connection refused` 失败。
- **不兼容：**原生交付包改用交接格式 `autosar-workbench-handoff-v3`；桌面端打开 `autosar-workbench-handoff-v2` 包时拒绝且不改动文件，其 ARXML 源文件仍可导入新工程。包内封存校验改为 `tools/sealed_package.py`，配合 `tools/build-tool-inventory.json`（格式 v2）；复制的独立工具改为 `tools/standalone-ecu-tool.py`；资产清单改为 `inventory/contracts/assets.json`（格式 v2，只有 `ecu` 一个 profile）；`workbench-ownership.json` 的 `formatVersion` 为 2，生成方 ID 为 `autosar-workbench-config` 与 `autosar-workbench-build`；`integration.json` 为 `autosar-ecu-integration-v2`，`contract.json` 为 `autosar-component-contract-v2`。计划描述（`formatVersion` 2）与 `integration.json` 中的组件组合键名改为 `composition`（原 `multi`）。生成的 ECU 栈接口改名为 `Ecu_Stack*`，`Ecu_ProfileConfig.h` 不再定义无人使用的 `ECU_MULTI_COMPONENT`。BSW 公开接口、组件契约头文件和第三方身份不变，应用无需重新编译。
- 仓库根目录成为 Cargo workspace，只有一个 `Cargo.lock` 和一个 `target/`；用 `-p autosar-config-core` 或 `-p autosar-config-desktop` 选择 crate。主线 BSW 运行时从 `runtime/multi` 移到 `runtime/bsw`，交付路径和字节不变。integration 模块、测试 fixture、C 分析样本 `component-pipeline` 和桌面场景 `composition-editing` 按职责命名，`quality` 拒绝仓库路径和文本中的开发计划编号。
- 生成 ECU 的每个文件只有一个生成方。同一路径出现第二个生成方（`OUTPUT_PRODUCER_CONFLICT`）、运行时模板未渲染（`OUTPUT_MISSING`）或模板不再含编辑锚点（`TEMPLATE_ANCHOR`）时拒绝生成，不再覆盖或修补已生成文本。生成源码与 README 逐字节不变。始终被生成的 `include/Rte_MemMap.h` 取代、从未交付的资产 `runtime/ecu/include/Rte_MemMap.h` 移出资产清单，因此交付的 `inventory/contracts/assets.json` 与规则集身份随之改变。
- 已核定计划不能保证的生成前提（缺少所有者、映射、被引用对象或 ECU 计划部分，或周期为零）报告为定位到源对象的诊断（`PLAN_PREMISE`、`REFERENCE_UNRESOLVED`、`INPUT_OBJECTS_MISSING`、`ECU_SOURCE_CLOSURE`），不再 panic。Clippy 以默认规则集加告警即错误检查两个 crate；生成与集成模块另外拒绝 `unwrap`、`expect`、`panic!` 和修补已渲染文本。

早期开发结果保留在 Git 历史和相关实施工件中；本记录不重新声明历史发行或验收结果。
