# 主机 ECU C99 运行时

本目录提供随主线 ECU 工程（`singlecore-ecu-v1`）交付的 C99 运行时：`bsw/` 为 Can、CanIf、CanTp、Com、ComM、BswM、Dcm、Det、LSduR、PduR 等 BSW 及其公共头文件，`host/` 为这些模块的主机适配（`Can_Host*`、`Can_HostLock*`、`Det_Host*`、`SchM_*_Host.c`、`Ecu_HostBusSM*`、`Ecu_Status*`），`ecu/` 为主机执行 adapter、HostBatch 入口与配置模板，`os/` 为受控 OS 目标（见 [os/README.md](os/README.md)），`contracts/` 为交付资产清单。交付时 `bsw/` 原样进入工程的 `bsw/`，`host/` 与 `ecu/` 进入 `integration/`，`os/` 进入 `os/`；各模块的 `*_MemMap.h`、`SchM_*.h`、`Rte_*_Type.h` 与配置模板只作为生成方的模板，渲染结果进入 `generated/`。工程布局见[生成工程布局](../docs/architecture/mainline.md#生成工程布局已实施38)。验证范围限于主机行为，尚未完成完整 AUTOSAR/ISO 一致性或真实芯片验证。

## 构建

生成工程的 `target.json` 明确声明 `windows-x64-controlled-v1` 或 `linux-x64-controlled-v1`、工具链身份和源文件选集；源码准备/预览不执行编译器。本机编译需要 Python 3.11 或更新版本、目标 GCC/binutils 和 Git。将 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT` 设置为绝对工具路径，再运行包内 stdlib-only 入口：

```text
<python> tools/ecu-tool.py build --project <sealed-source-directory> --output <new-empty-outside-directory> --mode host-batch
```

各模式的 main 记录在 `target.json` 的 `entries` 中。`host-batch` 产出实际生产批入口，`probe` 产出独立启动消费者，`test` 启用私有 OS/ECU 探针。`--control-source <external.c>` 只用于 `probe|test`，生产 HostBatch 拒绝它。编译器版本、目标与摘要必须匹配包内工具链，错误宿主或缺依赖明确失败，不降级到其他编译器。

构建目录必须位于源码之外，且为新目录或空目录。构建前后会核对文件清单和 SHA-256，缺失文件、额外文件、链接或 reparse point 均会导致失败。补丁只应用于私有内核副本；Windows 检查 PE/TLS 和段信息，Linux 检查 ELF 和相应段信息。失败日志保留在构建目录中。外部命令使用参数数组启动，子孙进程由受管进程范围按单调截止时间回收，无需 PowerShell 命令拼接或 taskkill 回退。

工程根目录的 `CMakeLists.txt` 由同一份 `target.json` 生成，供 IDE 和 CI 使用：源文件、include 路径、编译参数、内核补丁和链接库与 `ecu-tool build` 相同，配置阶段在构建目录复制并修补内核，两个 main 分别生成 `ecu_host_batch` 与 `ecu_probe`。构建目录必须在工程之外：

```text
cmake -S <sealed-source-directory> -B <outside-build-directory> -G Ninja -DCMAKE_C_COMPILER=<gcc>
cmake --build <outside-build-directory>
```

CMake 不核对固定工具链身份和目标段，也不运行 `verify`；这两项仍由 `ecu-tool` 负责。调用方设置的 `CMAKE_C_FLAGS`／`CFLAGS` 会叠加在清单参数之上。

集成工程的独立行为入口为 `tools/ecu-tool.py verify --project <sealed-source-directory> --build-directory <new-empty-outside-directory>`：重建实际生产binary，检查 CAN/DID、至多两个DID、N_Cr超时恢复和非法批次，预期不由生成配置回显提供。离线接收者不需要 checkout、uv、Rust 或 Node。工作台暂不提供交互式运行生成 ECU 的页面（[#65](https://github.com/g1331/autsaro/issues/65)），主机行为以此入口和原生测试验证。

发行工作台从内嵌可信资源生成同一封存工程；源码及其全部 stdlib 工具随工程交付，后续离线 build/verify 不回读工作台安装目录或原构建 checkout。配置交接（`autosar-workbench-handoff-v4`）不依赖官方 XSD/MOD；旧 host／单组件交付包在打开时被拒绝。CPython/GCC/binutils/Git 不随工作台或 ECU 包分发。工作台启动依赖与原生发行包后台复验入口见docs/development/testing/ 与 docs/maintainers/releasing.md。

工作台的预检和封存工具调用使用 CPython `-I -S`，忽略外部 Python home/path、用户及环境 site-packages；工具入口只从自身封存 `tools/` 导入 helper，禁用字节码，不向源码增加 `__pycache__`。这也适用于 AppImage 启动器设置 Python 相关变量的场景；开发虚拟环境的可编辑安装不是离线包依赖。

输入配置结构和容量上限见生成的 `generated/include/Ecu_Config.h`。验证结果限于已执行的主机用例，其他工程需单独复验；MCU、硬实时、ASIL 和完整标准符合性尚未验证。

## 显式 profile 策略与执行边界

`Ecu_Config.h` 定义业务策略，生成的 `generated/src/Ecu_Config.c` 提供 const 值；核心 BSW 不按操作系统或 Epic 宏选择语义。`Ecu_ProfileLimits.h` 显式提供 `ECU_MAX_PDU_PAYLOAD`，当前为 64 字节。

| 业务边界 | 行为 |
| --- | --- |
| 发送确认 | queued；native sink 实际 write/flush 成功后经 mailbox 在 owner 上确认；所属 epoch 的周期之前到达的确认在该周期之后按序提交 |
| 同 epoch 收帧与 deadline | 先处理该 epoch 的 RX，再检查 deadline |
| CanTp DLC | 固定补齐到 8 字节 |
| WFTmax=0 的 WAIT | 终止当前传输，返回 flow error；后续完整请求可恢复 |
| 单请求 DID 数 | 最多 2 个 |
| 已配置 DID 会话 | default/extended，来自验证后的会话掩码 |
| SID dispatch | `10/3e/22`；其余返回 unsupported-service NRC |
| P2/P2* | 来自验证后的 Dcm 会话参数；固定参考为 50/5000 ms |

ReadData 应用服务返回 `Std_ReturnType`，提供四个有效或替代字节并返回 `E_OK`；标准 C/S 接口声明 `E_NOT_OK=1`，保留 RTE 基础设施失败的返回语义，普通无 application error 的 C/S 服务仍返回 void。应用 DID 不可使用 F186，或 F400–F4E0／F800–F8E0 中每隔 0x20 的 availability 标识。后者由 Dcm 按已配置 DID 生成支持位图，不调用应用。

ReadData 保留请求顺序、重复DID、`0xF186`及不可读/unsupported DID跳过；先验证格式、DID数量和完整响应容量，再调用应用。容量不足返回NRC 0x14且不读取应用数据，RTE失败不遮盖先发生的拒绝。const Rx/Tx 表明确区分 CanIf 下层句柄和 Com/CanTp 上层句柄，来自 canonical PDU 计划。配置重初始化不会将 Tx-only 帧当成 Rx 路由。

发送、时钟和诊断 admission/pending/process 由链接选中的 adapter 实现；queued 请求 ring 与 owner 操作属于 ECU adapter，不进入公共 Dcm 接口。`Os_IntegrationOnWaiting` 在原持锁 quiescence 点、完成收据发布前通知 ECU adapter；无 ECU consumer 的独立 OS harness 使用无观察者 adapter，不发布伪造的 ECU 收据。私有 `OS_*_TESTS`/`ECU_TARGET_TESTS` 仅用于对应测试构建，生产 HostBatch 拒绝混合控制源。

## CAN 驱动边界

`Can_SetMode`、`Can_GetMode`、`Can_TransmitPdu`、`Ecu_StatusName` 与 `Ecu_HostBatchInitialize` 是类型检查的头内定义，不再提供原外部符号；全部消费者须重新生成并编译，不承诺跨 TU 函数地址或状态字符串地址相等。

标准调用方使用 `#include "Can.h"`；主机输出、状态注入和接收注入显式使用 `Can_Host.h`。`Can_CfgTypes.h` 的单 sink 配置是本目标的硬件专用配置，标准允许目标自定布局。Id 保持 32 位、HOH 保持 16 位，PDU 句柄／长度使用 `ComStack_Types.h`。`Can.h` 的服务都具有 R24-11 的公开签名并使用 AUTOSAR 类型；`CanSetBaudrateApi` 不提供，因此没有 `Can_SetBaudrate`。`Can_Write` 接纳 0–8 字节，不解析 CAN FD 标志位（控制器没有 FD 配置），一个发送槽占用时立即返回 `CAN_BUSY`。控制器状态机按规范图：STARTED 只能从 STOPPED 进入，STOPPED 可从任何已初始化状态进入（含自环），SLEEP 是逻辑睡眠。`SWS_CANDriver` 的条目已逐条处置（见 [Can](../docs/standards/can.md)）；尚缺已分配 vendor 身份、完整 MISRA 人工评估和硬件互操作证据，控制器硬件义务由 R10 目标验收（#170），不声明完整标准 Can Driver。虚拟控制器不模拟位级仲裁、电气错误计数器和真实中断。

`Can.c` 的 CODE 和 VAR_CLEARED_UNSPECIFIED 通过 `Can_MemMap.h` 显式映射至受控 GCC PE/ELF 的 `.text`／`.bss`，由链接器及进程装载器提供代码和清零存储；BSWMD 记录对应标记及头文件。重复 include 必须带单一合法 marker，禁止嵌套和错配。该映射不定义 MCU linker 布局。

`Can.c` 唯一管理发送、接收快照和生命周期；`Can_Host.h` 的 mode/transmit 头内定义调用三个既有 driver 操作声明，其余主机转发保留在 `Can_Host.c`。这些声明不暴露 driver 存储，标准 `Can.h` 仍不导入主机接缝。CanIf、CanTp、Dcm、ComM 与 BswM 使用各自独立的独占区。Dcm 在区外调用 PduR、ComM 与应用；单飞执行／发布协议保护回调、模式顺序与重启身份。driver 在调用 sink、CanIf 和 Det 前释放自己的锁，在途槽继续拒绝同 HTH 的新请求。STOP、BUS_OFF 和 DeInit 取消未确认工作；STOPPED／SLEEP 的 DeInit 同步清理，不等待模式通知。未初始化的主机业务被拒绝，主机 `Can_GetMode` 查询保留。已由平台接纳后发生的接收 shadow overrun 经私有接缝记录 overrun，并由实际 `Can_MainFunction_Read` 接收服务报告 `Det_ReportRuntimeError(80, 0, 0x08, 0x01)`（polling 在下一次 Read 消费，即时接收在检测后执行同一接收处理）；可重试的主机 BUSY 不报告丢帧。当前主机没有硬件 overwrite 事件源或 wakeup IRQ；两个模式轮询入口是同一消费路径的兼容别名。

`CanVersionInfoApi` 缺失（`MULTIPLICITY`）或为 true（`CAN_FEATURE_UNSUPPORTED`）时在生成前拒绝，`Can.h` 不声明 `Can_GetVersionInfo`，也不提供版本信息开关宏。`CAN_MODULE_ID` 为 80，软件版本与项目 `0.1.0` 一致。R24-11 MOD 的模块 DOC-REVISION 固定为 4.10.0；模块的 `_Version.h` 是所属公共／配置头的 release 来源，适用的直接外部头检查 major/minor，允许 revision 差异。MemMap 身份头不保护重复 START/STOP 处理。项目尚无已分配的 AUTOSAR vendor ID，正式供应方／代码使用方身份、软件版本政策与 API 启用仍由 #59 承接。ComM 源／头另校验 release 三分量一致，软件一致性仍待该政策。

## 错误报告

生成工程须包含显式 Det ECUC 配置：`DetGeneral/DetVersionInfoApi=false`，`DetForwardToDlt` 省略或为 false，`DetNotification/DetReportRuntimeErrorCallout=Ecu_DetRuntimeCallout`。内置模板已提供这些值。其他 hooks、版本 API、DLT 与非空 `DetConfigSet/DetModule` 在生成前拒绝；已有工程须补齐配置后重新生成。

BSW 通过 `Det_ReportRuntimeError` 报告运行时错误，配置通知按顺序执行，返回值始终为 `E_OK`；尚未初始化时不通知。开发错误通过 `Det_ReportError` 通知后停止，受控主机终止 ECU 进程。两个报告入口都在模块独占区外执行通知，标准头不包含主机故障控制。已选 callout 的声明在 `generated/include/Det_Externals.h`，定义在 `generated/src/Det_Callouts.c`；CanTp 超时仍由主机收据返回。配置、接口与验证边界见 [Det 标准说明](../docs/standards/det.md)。

## 许可证

项目原创运行时、工具及生成模板采用 [Apache-2.0](../LICENSE)。交付工程附带 LICENSE/NOTICE，FreeRTOS 保留独立 MIT 声明，用户配置与应用代码由其权利人决定许可。详见 [许可说明](../docs/maintainers/licensing.md)。
