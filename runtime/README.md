# 主机虚拟 ECU C99 运行时

本目录是首阶段 **标准 11 位 Classical CAN 原始信号**的独立目标端代码，不声称完整 AUTOSAR 一致性或真实芯片验证。生成器须把本目录 `include/`、`src/` 的文件与自身输出的 `Ecu_Config.c` 一起纳入每个 ECU 的独立工程；不依赖历史私有协议栈。每个进程只链接**一个** `const EcuConfig Ecu_Config`。

## 构建

在工程根目录，将 `generated/Ecu_Config.c` 换成该 ECU 实际生成文件：

```sh
# MinGW GCC（或其他 C99 GCC）
gcc -std=c99 -Wall -Wextra -pedantic -Iruntime/include runtime/src/*.c generated/Ecu_Config.c -o ecu.exe
```

在 MSVC 开发者命令提示符中：

```bat
cl /TC /W4 /I runtime\include runtime\src\Can.c runtime\src\CanIf.c runtime\src\Com.c runtime\src\Ecu_Runtime.c runtime\src\Ecu_Status.c runtime\src\Os.c runtime\src\PduR.c runtime\src\Rte.c runtime\src\ecu_host_main.c generated\Ecu_Config.c /Fe:ecu.exe
```

独立交付工程应保留这些源码和头文件，并将其中的 include/source 路径调整为工程内路径。输入配置结构和容量上限定义在 `include/Ecu_Config.h`；启动时再次校验生成数据，错误返回 `E CONFIG` 并退出。

## 支持范围与调用链

每个 ECU 最多 32 帧、64 信号；CAN ID `0..2047`，DLC `1..8`，信号长度 `1..32` 位，无符号 LSB0 小端位序，一帧可有多个不重叠信号。每个信号 ID 在 ECU 内唯一，帧 ID 在 ECU 内唯一；每个信号恰属一帧。Tx 帧设置正周期且超时为零；Rx 帧设置正超时且周期为零。初值必须适合位宽。超出范围、不支持的位序/帧类型/参数不能被默默转换为此结构；生成器必须在生成前拒绝它们。

发送：`Os_Advance`（单核虚拟时间、周期任务）→ `Com_TriggerTransmit`（以 `Rte_WriteSignal` 写入的应用值打包）→ `PduR_Transmit` → `CanIf_Transmit` → 虚拟 `Can_Transmit` → `X` 输出。接收：`R` 输入 → `Can_Inject` → `CanIf_RxIndication`（ID 过滤、DLC 检查）→ `PduR_RxIndication` → `Com_RxIndication`（解包、更新有效性）→ `Rte_ReadSignal`。没有自行回显；主机编排器按 ID 优先级路由两个 ECU 的报文，也可丢帧。当前虚拟 CAN 控制器具有 STARTED/STOPPED/BUS_OFF 状态，但不模拟位级仲裁、电气错误计数器和真实中断。

## 逐行 stdin/stdout 协议

启动时控制器为 STARTED、虚拟时间为 0。输入使用十进制非负整数、严格的大写十六进制；每条指令独占一行。输出立即刷新。

| 输入 | 行为 |
| --- | --- |
| `T <uint64_ms>` | 推进非递减绝对虚拟时间。跨过一个或多个周期边界时，每个到期的 Tx 帧至多输出一次 `X <id> <dlc> <2*dlc 个大写十六进制字符>`；大跨度跳时合并过期周期，不补发历史帧。Rx 自最后一次有效接收起达到超时阈值时变为无效。 |
| `R <decimal_can_id> <dlc> <hex>` | 注入一帧；只有 STARTED 状态接受，未知 ID 被过滤，已配置 Rx ID 的 DLC 不符返回 `E FRAME_DLC`。 |
| `S <signal_id> <uint32_value>` | 修改 Tx 应用信号，下个 Tx 周期采用新值；Rx 信号返回 `E DIRECTION`，超出信号位宽返回 `E SIGNAL_VALUE`。 |
| `G <signal_id>` | 返回 `V <signal_id> <value> <0或1>`；Rx 初始无效，超时后保留最后数值但 `valid=0`。Tx 初始有效。 |
| `M <0\|1\|2>` | 依次设置 STOPPED、STARTED、BUS_OFF；关闭时拒绝 Rx 并跳过 Tx 周期，重新 STARTED 不补发已跳过的帧。 |

语义错误（例如未知信号、错误 DLC、控制器关闭时注入）输出 `E <code>` 并继续处理后续行；非法命令、错误数字或 hex 输出 `E PROTOCOL` 并以非零码退出；时间倒退输出 `E TIME` 并非零退出。未配置 ID 的合法报文被过滤，不是错误。实际总线丢帧由外部主机编排，Rx 有效位会随时间真实超时。
