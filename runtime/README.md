# 主机虚拟 ECU C99 运行时

本目录是 **标准 11 位 Classical CAN 原始信号及单条主机虚拟 DoCAN 物理连接**的独立目标端代码，可选配一个由 Rx 帧超时触发、由主机 NvM 持久化的 UDS DTC。不声称完整 AUTOSAR/ISO 一致性或真实芯片验证。生成器须把本目录 `include/`、`src/` 的文件与自身输出的 `Ecu_Config.c` 一起纳入每个 ECU 的独立工程；不依赖历史私有协议栈。每个进程只链接**一个** `const EcuConfig Ecu_Config`，未配置诊断时其 `diagnostic` 指针为 `NULL`，未配置故障记忆时其 `diagnostic->dtc` 指针为 `NULL`。

## 构建

在工程根目录，将 `generated/Ecu_Config.c` 换成该 ECU 实际生成文件：

```sh
# MinGW GCC（或其他 C99 GCC）
gcc -std=c99 -Wall -Wextra -pedantic -Iruntime/include runtime/src/*.c generated/Ecu_Config.c -o ecu.exe
```

在 MSVC 开发者命令提示符中：

```bat
cl /TC /W4 /I runtime\include runtime\src\Can.c runtime\src\CanIf.c runtime\src\CanTp.c runtime\src\Com.c runtime\src\Dcm.c runtime\src\Dem.c runtime\src\Ecu_Runtime.c runtime\src\Ecu_Status.c runtime\src\LSduR.c runtime\src\NvM.c runtime\src\Os.c runtime\src\PduR.c runtime\src\Rte.c runtime\src\ecu_host_main.c generated\Ecu_Config.c /Fe:ecu.exe
```

独立交付工程应保留这些源码和头文件，并将其中的 include/source 路径调整为工程内路径。输入配置结构和容量上限定义在 `include/Ecu_Config.h`；启动时再次校验生成数据，错误返回 `E CONFIG` 并退出。

## 支持范围与调用链

每个 ECU 最多 32 帧、64 信号；CAN ID `0..2047`，DLC `1..8`，信号长度 `1..32` 位，无符号 LSB0 小端位序，一帧可有多个不重叠信号。每个信号 ID 在 ECU 内唯一，帧 ID 在 ECU 内唯一；每个信号恰属一帧。Tx 帧设置正周期且超时为零；Rx 帧设置正超时且周期为零。初值必须适合位宽。超出范围、不支持的位序/帧类型/参数不能被默默转换为此结构；生成器必须在生成前拒绝它们。

信号发送：`Os_Advance`（单核虚拟时间、周期任务）→ `Com_TriggerTransmit`（以 `Rte_WriteSignal` 写入的应用值打包）→ `PduR_Transmit` → `LSduR_PduRTransmit` → `CanIf_Transmit` → 虚拟 `Can_Transmit` → `X` 输出。信号接收：`R` 输入 → `Can_Inject` → `CanIf_RxIndication`（ID 过滤、DLC 检查）→ `LSduR_CanIfRxIndication` → `PduR_RxIndication` → `Com_RxIndication`（解包、更新有效性）→ `Rte_ReadSignal`。没有自行回显；主机编排器按 ID 优先级路由两个 ECU 的信号报文，也可丢帧。当前虚拟 CAN 控制器具有 STARTED/STOPPED/BUS_OFF 状态，但不模拟位级仲裁、电气错误计数器和真实中断。

## 诊断连接（可选）

一条物理 normal-addressing 11-bit Classical CAN 连接；请求与响应 ID 不相同，也不能与 Com 帧 ID 冲突。`CanIf` 按请求 ID 交给 `LSduR` → `CanTp`，PduR 持有上限 256 字节的 N-SDU 收发缓冲，Dcm 处理完成请求；响应由 Dcm → PduR → CanTp → LSduR → Can 输出。CanTp 按 SF/FF/CF/FC 分段，检查 FF 总长、CF 序号和 DLC，FF 后以 CTS/BS=0/STmin=0 回应；发送方遵守测试器给出的 FC 块大小、STmin（100 μs 单位在 1 ms 主机时钟上向上取整）、N_Bs，接收方按 N_Cr 终止不完整请求。错误后丢弃半包，下一个完整请求可以恢复。

Dcm 提供 0x10 默认/扩展会话、0x3E TesterPresent（子功能 0x80 抑制正响应）和 0x22 一个有序实时 DID。0x10 的会话切换仅在正响应发送确认后提交；扩展会话 S3 超时回默认。一个 DID 从 1–8 个 32-bit Tx Com 信号读取并以每项大端 4 字节拼接；默认会话或 DID 不匹配返回 `7F 22 31`，长度错误返回 NRC 0x13，当前值不可读返回 NRC 0x22。P2=50 ms、P2*=500 ms 写在 0x10 响应中；S3 配置不得小于 5000 ms。`profile.txt` 的 `DIAGNOSTIC` 行记录生成 ID、计时器与 DID 信号 ID 顺序，供独立主机测试器驱动。

可选单 DTC：`profile.txt` 的 `DTC code=<十进制> frame=<生成帧索引> id=<CAN ID> dlc=<字节数> timeout=<ms>` 指定一个带信号的 Rx 帧（超时为正），DTC 范围 `0x000100–0xFFFFFE`。首次有效 Rx 帧令监控测试完成；其后第一次达到超时阈值，Com → Dem 记故障并持久化。Dcm 在默认/扩展会话支持 0x19/0x02，返回状态可用掩码 `0x7F` 与按请求掩码过滤的单条 DTC；0x14 仅扩展会话和 `0xFFFFFF` 全部清除，默认会话返回 NRC 0x7F，其他组返回 NRC 0x31，持久化失败返回 NRC 0x72。未配置 DTC 时这两个服务不可用（NRC 0x11）。

配置 DTC 的 `ecu_host` **必须**以 `--nvm <独占的文件路径>` 启动；无 DTC 的旧工程仍不带参数运行。缺少参数返回 `E CONFIG`，既有文件损坏、配置指纹不匹配或写入失败返回 `E NVM`，不降级成空 DTC。不存在的文件会创建两个 32 字节 CRC32 保护槽位；每次状态改变交替写槽并 `fflush`、`fsync`/`_commit` 后才确认，启动要求两个槽位均完整，任一损坏即拒绝使用以避免旧状态覆盖最新故障。主机进程启动作为新的操作周期：初始/清除状态 `0x50`，有效 Rx 首次测试通过 `0x00`，首次超时 `0x2F`，故障后重启 `0x6D`，该周期再收到有效帧 `0x2C`；后续周期才清除 pending 标志。独立测试器为每次验证分配隔离文件，不复用实际 ECU 状态。

这是有边界的主机虚拟实现：未实现多事件 Dem、真实 NvM 设备与 Ea/Fee/MemIf 目标、0x27/0x2E/0x31、其他 0x19 子功能和 0x14 清除组、功能寻址、跨连接并发、实车确认时序或完整 ECUC 外部 ComM 引用；不以 XSD 通过代替 AUTOSAR/ISO 语义与互操作认证。Dem 事件到监测 Rx 帧只在工具专属 SDG 中绑定，操作周期只是进程启动。

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
