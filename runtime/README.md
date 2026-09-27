# 主机虚拟 ECU C99 运行时

本目录是 **标准 11 位 Classical CAN 原始信号及单条主机虚拟 DoCAN 物理连接**的独立目标端代码，可选配一个由 Rx 帧超时触发、由主机 NvM 持久化的 UDS DTC。不声称完整 AUTOSAR/ISO 一致性或真实芯片验证。生成器须把本目录 `include/`、`src/` 的文件与自身输出的 `Ecu_Config.c`、`Dcm_Externals.h` 一起纳入每个 ECU 的独立工程；不依赖历史私有协议栈。每个进程只链接**一个** `const EcuConfig Ecu_Config`，未配置诊断时其 `diagnostic` 指针为 `NULL`，未配置故障记忆时其 `diagnostic->dtc` 指针为 `NULL`。

## 构建

在工程根目录，将 `generated/Ecu_Config.c` 换成该 ECU 实际生成文件：

```sh
# MinGW GCC（Windows 主机目标）
gcc -std=c99 -Wall -Wextra -pedantic -Iruntime/include runtime/src/*.c generated/Ecu_Config.c -o ecu.exe -lbcrypt
```

在 MSVC 开发者命令提示符中：

```bat
cl /TC /W4 /I runtime\include runtime\src\Can.c runtime\src\CanIf.c runtime\src\CanTp.c runtime\src\Com.c runtime\src\Dcm.c runtime\src\Dem.c runtime\src\Ecu_Runtime.c runtime\src\Ecu_Status.c runtime\src\LSduR.c runtime\src\NvM.c runtime\src\Os.c runtime\src\PduR.c runtime\src\Rte.c runtime\src\Security.c runtime\src\ecu_host_main.c generated\Ecu_Config.c bcrypt.lib /Fe:ecu.exe
```

独立交付工程应保留这些源码和头文件，包括工程根目录的生成回调声明 `Dcm_Externals.h`，并将其中的 include/source 路径调整为工程内路径。输入配置结构和容量上限定义在 `include/Ecu_Config.h`；启动时再次校验生成数据，错误返回 `E CONFIG` 并退出。

## 支持范围与调用链

每个 ECU 最多 32 帧、64 信号；CAN ID `0..2047`，DLC `1..8`，信号长度 `1..32` 位，无符号 LSB0 小端位序，一帧可有多个不重叠信号。每个信号 ID 在 ECU 内唯一，帧 ID 在 ECU 内唯一；每个信号恰属一帧。Tx 帧设置正周期且超时为零；Rx 帧设置正超时且周期为零。初值必须适合位宽。超出范围、不支持的位序/帧类型/参数不能被默默转换为此结构；生成器必须在生成前拒绝它们。

新建 ARXML 的 EcuC 子集提供 `EcuC/EcucConfigSet/EcucPduCollection/Pdu`：不超过 32 帧的主机配置使用 `PduIdTypeEnum=UINT8`，含 256 字节诊断 N-SDU 时使用 `PduLengthTypeEnum=UINT16`（仅信号时为 `UINT8`），每个 Pdu 记录字节长度。配置 Com 帧时提供 `ComGeneral` 的必需选项、`ComIPdu` 的 `IMMEDIATE/NORMAL` 与 Tx 未使用位初值。Com 与 CanIf 的 PDU 值引用指向独立全局 Pdu，工具 SDG 将其一一绑定到系统 I-PDU/N-PDU/DCM-I-PDU；这不是标准的系统 PDU 直接引用。固定主机剖面的 CanIf/CAN ECUC 还包含必需根、控制器/驱动、HOH、零容量 Tx 缓冲与 PDU 参数，并以单个虚拟 Mcu 时钟参考点闭合 `CanCpuClockRef`；生成的虚拟时钟、基地址、波特率不驱动真实硬件。主机 C API、控制器状态和调度仍是自有实现，不作为第三方 CanIf/CAN 驱动互操作证据。

诊断的 `CanTpRx/TxNSduRef`、四项 CanTp N-PDU/FC N-PDU 引用与 `DcmDslProtocolRx/TxPduRef` 全部以 `DEST="ECUC-CONTAINER-VALUE"` 绑定相应全局 Pdu；系统 N-PDU/DCM-I-PDU 只由工具 SDG 指明关系。System Template `[constr_3448]` 不允许在本剖面的 I-SIGNAL-I-PDU、N-PDU、DCM-I-PDU 对应全局 Pdu 上输出 `DynamicLength`，系统 N-PDU 也不输出 `HAS-DYNAMIC-LENGTH`。N-PDU 的长度由传输层处理，DcmIPdu 本身动态；解析器不靠 EcuC 的 `DynamicLength` 推断运行时行为，带有该字段的导入文件只读阻断。

解析消耗的 ComIPdu/ComSignal 时要求唯一的本项目 `ComCfg` 模块、正确的模块定义以及直接的 `ComConfig`/`ComGeneral` 归属。来源模块被重命名、父级定义错误或子容器挂到外部模块，即使局部 PDU/信号引用仍可解析，也仅只读导入并禁止生成。

信号发送：`Os_Advance`（单核虚拟时间、周期任务）→ `Com_TriggerTransmit`（以 `Rte_WriteSignal` 写入的应用值打包）→ `PduR_Transmit` → `LSduR_PduRTransmit` → `CanIf_Transmit` → 主机 `Can_Transmit` 包装层 → `Can_Write` → `X` 输出。信号接收：`R` 输入 → `Can_Inject` → `CanIf_RxIndication`（ID 过滤、DLC 检查）→ `LSduR_CanIfRxIndication` → `PduR_RxIndication` → `Com_RxIndication`（解包、更新有效性）→ `Rte_ReadSignal`。没有自行回显；主机编排器按 ID 优先级路由两个 ECU 的信号报文，也可丢帧。`Can_Init`、`Can_SetControllerMode`、`Can_GetControllerMode` 和 `Can_Write` 已有 R24-11 的公开签名及本主机剖面的类型；初始化配置仍包含主机输出回调，状态和发送入口以锁保护，生成的主机程序仍以单线程同步发送。其他必需服务、回调、MemMap、BSWMD、MISRA 与第三方互操作尚未闭合，不据此声明完整标准 Can Driver。当前虚拟 CAN 控制器具有 STARTED/STOPPED/BUS_OFF 状态；bus-off 对标准状态查询表现为 STOPPED，但仍拒绝报文直到主机明确恢复。它不模拟位级仲裁、电气错误计数器和真实中断。

## 诊断连接（可选）

一条物理 normal-addressing 11-bit Classical CAN 连接；请求与响应 ID 不相同，也不能与 Com 帧 ID 冲突。`CanIf` 按请求 ID 交给 `LSduR` → `CanTp`，PduR 持有上限 256 字节的 N-SDU 收发缓冲，Dcm 处理完成请求；响应由 Dcm → PduR → CanTp → LSduR → Can 输出。CanTp 按 SF/FF/CF/FC 分段，检查 FF 总长、CF 序号和 DLC，FF 后以 CTS/BS=0/STmin=0 回应；发送方遵守测试器给出的 FC 块大小、STmin（100 μs 单位在 1 ms 主机时钟上向上取整）、N_Bs，接收方按 N_Cr 终止不完整请求。错误后丢弃半包，下一个完整请求可以恢复。

Dcm 提供 0x10 默认/扩展会话、0x3E TesterPresent（子功能 0x80 抑制正响应）和 0x22 一个有序实时 DID。0x10 的会话切换仅在正响应发送确认后提交；扩展会话 S3 超时回默认。一个 DID 从 1–8 个 32-bit Tx Com 信号读取并以每项大端 4 字节拼接；配置 DID 在默认会话不可读；保留 DID `0xF186` 通过 0x22 返回当前默认/扩展会话编号，不依赖应用信号。0x22 请求可按顺序列出多个 DID，响应只包含当前可读的 DID，全部不可读时返回 `7F 22 31`；请求缺失 DID 或字节数不成对返回 NRC 0x13，响应超过 256 字节返回 NRC 0x14，当前值不可读返回 NRC 0x22。P2=50 ms、P2*=500 ms 写在 0x10 响应中；S3 配置不得小于 5000 ms。`profile.txt` 的 `DIAGNOSTIC` 行记录生成 ID、计时器与 DID 信号 ID 顺序，供独立主机测试器驱动。

生成工程按 DID 信号顺序提供外部链接的 `Std_ReturnType Ecu_DcmRead_<index>(uint8_t *data)`，声明在 `Dcm_Externals.h`；主机 Dcm 的 0x22 实际调用它读取 Tx Com 值并写入 4 字节大端数据，读取失败仍返回 NRC 0x22。`include/Ecu_DcmCallbackTypes.h` 只定义主机剖面所需的 `Std_ReturnType`、`Dcm_NegativeResponseCodeType` 和返回码；它不是完整 AUTOSAR `Std_Types.h` 或 `Rte_Dcm_Type.h`，此回调闭包不证明第三方 Dcm 互操作或完整 BSW/RTE 符合性。

可选 0x2E WriteDataByIdentifier 仅作用于当前诊断配置的一个 DID，须显式启用；`profile.txt` 增加 `WRITE_DID did=<十进制>` 行，未启用时不输出该行并返回 NRC 0x11。Dcm 只在扩展会话接受与 DID 的 1–8 个 32-bit Tx 信号相符的完整数据记录，每项按大端 4 字节传入生成的 `Ecu_DcmWrite_<index>` 回调，回调经 Rte → Com 修改当前值；0x22 读回与周期 CAN 帧立即可见。默认会话或 DID 不匹配返回 NRC 0x31，记录长度不符返回 NRC 0x13。写入可以是单帧或多帧请求，S3 回默认会话后不能继续写；ECU 进程重启后值恢复配置初值。此功能只写易失的主机虚拟应用状态，不使用 NvM/Flash；若启用下述 0x27 档案，写入还须解锁。

启用 0x2E 时还提供可外部链接的 `Std_ReturnType Ecu_DcmWrite_<index>(const uint8_t *data, Dcm_NegativeResponseCodeType *error_code)`，由同一头文件声明；主机 Dcm 实际调用它，经 Rte → Com 写入，失败时回调填入 0x72 并返回 `E_NOT_OK`，主机发送相应 NRC。未启用写入时不生成写回调声明或定义。

可选的**Windows 主机安全档案**提供单级 0x27：`27 01` 返回 16 字节随机 seed，`27 02` 携带 16 字节 key；key 为 HMAC-SHA256(`32 字节密钥`, `AUTOSAR-HOST-SECURITY-v1` 与 seed 的拼接)的前 16 字节。仅扩展会话可请求，成功后允许当前进程的 `0x2E`、`0x31/0x01`、`0x14`、`0x85`；未解锁时返回 NRC 0x33。错误顺序、错误 key、第三次失败及延时中请求 seed 分别返回 0x24、0x35、0x36、0x37。成功切换会话、S3 超时及重启会重新锁定。失败次数（最多 3 次）保存在独立的 16 字节 CRC32 校验状态文件；第三次失败后等待 5 秒虚拟时间，重启后仍须等待 5 秒；到期清零。密钥文件必须恰为 32 字节原始数据，由运行者保管，不能写入 ARXML 或生成工程。生成的 `ecu_host.exe` 启用本档案时须传 `--security-key <密钥文件> --security-state <独占状态文件>`；同时配置 DTC 时另传 `--nvm <独占故障文件>`。缺密钥/参数返回 `E CONFIG`，损坏或不可写的安全状态返回 `E NVM`。状态文件的 CRC 只检测损坏，不防止有本机文件写权限者篡改；本档案不证明硬件密钥保护或量产级认证。

可选 0x31/0x01 StartRoutine 只能在已启用 0x2E 时配置一个 RID；`profile.txt` 增加 `RESET_ROUTINE id=<十进制>` 行，未配置时不输出该行且服务返回 NRC 0x11。请求载荷为 `31 01 <RID 高字节> <RID 低字节>`，扩展会话中生成的内部 `Ecu_HostRestoreDid` 按 DID 配置顺序经 Rte → Com 将绑定的 Tx 信号写回各自的初始值；成功响应 `71 01 <RID 高字节> <RID 低字节>`，0x22 与周期 CAN 均能观察到恢复。默认会话或 RID 不匹配返回 NRC 0x31，StopRoutine/RequestRoutineResults 返回 NRC 0x12，长度错误返回 NRC 0x13，写入失败返回 NRC 0x22。没有选项/状态记录、持久化或失败时的事务回滚；例程只作用于当前进程的易失状态，启用安全档案时还须先通过 0x27 解锁。

0x31 是本工程的**主机专属行为**：生成 ARXML 在 DID 的 `ADMIN-DATA/SDGS` 下存储唯一 `AutosarWorkbenchHostRestoreDidV1` 工具组，其中 `Rid` 为十进制 16-bit 编号、`SessionRef` 固定指向本项目的 Extended 会话；不生成 DcmDsd 0x31 服务或 DcmDspRoutine/StartRoutine/CommonAuthorization 的标准 ECUC 节点。主机读取工具记录生成内部例程，不把 `Ecu_HostRestoreDid` 作为第三方 Dcm 回调。R24-11 `DcmDspRoutineFncSignature` 本属草案；独立的第三方 Dcm 不会从该 ARXML 获得本例程，也未验证例程的标准 ECUC 接口。

工作区只接受当前 R24-11 工具配置结构：Com/CanIf/CanTp/Dcm 的 ECUC PDU 引用须指向 EcuC 全局 Pdu，主机 0x31 例程由 DID 工具 SDG 描述。旧工具生成的系统 PDU 直接引用与草案 Dcm 例程配置不会自动迁移或隐式补全；不符合当前结构的输入保留原文件并阻止保存和生成。无关且不影响有效配置的未知内容仍可原样保留。

可选单 DTC：`profile.txt` 的 `DTC code=<十进制> frame=<生成帧索引> id=<CAN ID> dlc=<字节数> timeout=<ms>` 指定一个带信号的 Rx 帧（超时为正），DTC 范围 `0x000100–0xFFFFFE`。首次有效 Rx 帧令监控测试完成；其后第一次达到超时阈值，Com → Dem 记故障并持久化。Dcm 在默认/扩展会话支持 0x19/0x01，按状态掩码返回匹配 DTC 数量及状态可用掩码 `0x7F`；当前最多一个 DTC，因此数量为 0 或 1。支持单 DTC 的 0x19/0x02，按请求掩码读取；`19 0A` 列出支持的 DTC，返回 `59 0A 7F <DTC 三字节> <当前状态>`，即使状态为 0x00 仍返回该监测项。0x0A 在默认/扩展会话均可读，启用安全档案也无需解锁，暂停 DTC 设置时仍可读取；读取不修改 NvM。0x0A 请求长度不为 2 返回 NRC 0x13，其他 0x19 子功能（含 0x8A 抑制位）返回 NRC 0x12。0x14 仅扩展会话和 `0xFFFFFF` 全部清除，默认会话返回 NRC 0x7F，其他组返回 NRC 0x31，持久化失败返回 NRC 0x72。未配置 DTC 时 0x19/0x01、0x19/0x02、0x19/0x0A 与 0x14 返回 NRC 0x11。

配置单 DTC 时，DcmDsd 还声明仅扩展会话使用的 0x85 服务，`DcmDspControlDTCSetting` 明确不支持选项记录，诊断协议行通过 `DcmDemClientRef` 引用唯一的 DemClient。`85 02` 暂停事件状态更新与 NvM 写入，`85 01` 恢复；正常 Rx 有效性仍更新，已有 DTC 保持可读且可由 0x14 清除。请求长度不为 2 返回 NRC 0x13，未支持的子功能返回 NRC 0x12，默认会话返回 NRC 0x7F，未配置 DTC 返回 NRC 0x11。成功响应分别为 `C5 02` 和 `C5 01`；S3 回默认会话、成功切换默认会话或进程重启会重新启用记录，不回放暂停期间的事件，也不将设置模式写入 NvM。

配置 DTC 的 `ecu_host` **必须**以 `--nvm <独占的文件路径>` 启动；未配置 DTC 的工程不带参数运行。缺少参数返回 `E CONFIG`，既有文件损坏、配置指纹不匹配或写入失败返回 `E NVM`，不降级成空 DTC。不存在的文件会创建两个 32 字节 CRC32 保护槽位；每次状态改变交替写槽并 `fflush`、`fsync`/`_commit` 后才确认，启动要求两个槽位均完整，任一损坏即拒绝使用以避免旧状态覆盖最新故障。主机进程启动作为新的操作周期：初始/清除状态 `0x50`，有效 Rx 首次测试通过 `0x00`，首次超时 `0x2F`，故障后重启 `0x6D`，该周期再收到有效帧 `0x2C`；后续周期才清除 pending 标志。独立测试器为每次验证分配隔离文件，不复用实际 ECU 状态。

这是有边界的主机虚拟实现：未实现多事件 Dem、真实 NvM 设备与 Ea/Fee/MemIf 目标、多级 0x27 与其他 0x31 例程/子功能、其他写入 DID/持久写入、其他 0x19 子功能和 0x14 清除组、功能寻址、跨连接并发、实车确认时序或完整 ECUC 外部 ComM 引用；不以 XSD 通过代替 AUTOSAR/ISO 语义与互操作认证。Dem 事件到监测 Rx 帧只在工具专属 SDG 中绑定，操作周期只是进程启动。

## 逐行 stdin/stdout 协议

启动时控制器为 STARTED、虚拟时间为 0。输入使用十进制非负整数、严格的大写十六进制；每条指令独占一行。输出立即刷新。配置 DTC 时须先传启动参数 `--nvm <path>`；该路径必须与其他 ECU/测试进程隔离。

| 输入 | 行为 |
| --- | --- |
| `T <uint64_ms>` | 推进非递减绝对虚拟时间。跨过一个或多个周期边界时，每个到期的 Tx 帧至多输出一次 `X <id> <dlc> <2*dlc 个大写十六进制字符>`；大跨度跳时合并过期周期，不补发历史帧。Rx 自最后一次有效接收起达到超时阈值时变为无效；受监控帧只在首次有效接收后的有效→超时边沿报告 Dem 故障，落盘失败返回 `E NVM`。 |
| `R <decimal_can_id> <dlc> <hex>` | 注入一帧；只有 STARTED 状态接受，未知 ID 被过滤。Com Rx 帧要求已配置 DLC；受监控帧通过有效性检查后报告测试通过并持久化状态。诊断请求 ID 由 CanTp 检查 SF/FF/CF/FC 及总长，响应以 `X <response_id> <dlc> <hex>` 输出。 |
| `S <signal_id> <uint32_value>` | 修改 Tx 应用信号，下个 Tx 周期采用新值；Rx 信号返回 `E DIRECTION`，超出信号位宽返回 `E SIGNAL_VALUE`。 |
| `G <signal_id>` | 返回 `V <signal_id> <value> <0或1>`；Rx 初始无效，超时后保留最后数值但 `valid=0`。Tx 初始有效。 |
| `M <0\|1\|2>` | 依次设置 STOPPED、STARTED、BUS_OFF；关闭时拒绝 Rx 并跳过 Tx 周期，重新 STARTED 不补发已跳过的帧。 |

语义错误（例如未知信号、错误 DLC、诊断序号错误 `E TP_SEQUENCE`、N_Bs/N_Cr 超时 `E TP_TIMEOUT`、控制器关闭时注入）输出 `E <code>` 并继续处理后续行；非法命令、错误数字或 hex 输出 `E PROTOCOL` 并以非零码退出；时间倒退输出 `E TIME` 并非零退出。未配置 ID 的合法报文被过滤，不是错误。实际总线丢帧由外部主机编排，Rx 信号有效位会随时间真实超时。诊断失败路径不会把半包交给 Dcm，也不会把错误当作诊断正响应。
