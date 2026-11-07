# 类型化同步调用与端口常量

主线同步 C/S 复用 `rte_types` 的标量身份与既有四字节 OUT 数组；显式参数仍属于
operation，端口常量属于提供端的 `PortAPIOption`。`PortDefinedArgumentValue` 按输入顺序
进入 server 参数列表，client 的参数列表只含 operation 参数。多个 P-port/event 可以共用
参数类型、方向和返回类型兼容的 runnable，符号和应用实现均只有一份。

已核对（R24-11 RTE 页 366、810–811 的签名与正文；`rte_typed_contract.c` 实际验证两个端口的
槽位、偏移和显式参数）：SWS_Rte_01360、01166、08913。

已核对（R24-11 SWCT 页 664 图 7.33、页 666–667；`port_arguments` 检查 provide-port
归属、VALUE 类型、必需值与数值表示范围；输入反例定位到对应对象）：TPS_SWCT_01357、01358。
已核对（`invalid_port_arguments_and_shared_server_signatures_locate_the_source` 的端口、类型和
缺值反例）：constr_1386、1961、1962、1150。

无 possibleErrors 的 server 返回 `void`；含 possibleErrors 时返回 `Std_ReturnType`，包括
仅列 `E_OK` 的情况。应用错误值为 1–63，`E_OK` 为 0。直接调用按原引用传递 OUT/INOUT；
应用错误返回仍保留 server 的写入。未连接、停止和 owner 拒绝发生在 server 调用之前，
因此输出与 server 状态保持。独立函数指针消费者同时验证 `uint8`、`boolean`、标量
OUT/INOUT、无显式参数、E_OK-only 和多错误返回；原有 void 与固定数组回归沿用原消费者。

`typed_port_arguments_and_errors_reach_real_servers_and_kill_behavior_mutations` 将遗漏端口值、
换序、固定端口值、丢弃返回和停止后写 OUT 的变异逐个编译后运行，由行为断言拒绝。
服务到 BSW 实现的绑定归 #436，模式接线归 #437，其他输入变体继续由 #212 跟踪。

已核对（R24-11 SWCT 页 112–113；一般错误的范围、归属及同名错误宏冲突由
`application_errors`、`check_names` 和源对象反例检查）：constr_1108、1038、1102。
不同错误名可以使用同一数值；同名公开宏的相同值合并，冲突值在生成前拒绝。

已核对（R24-11 RTE 页 689；`contract_headers` 在每个应用头按 interface／error／value
聚合符号，`typed_servers_keep_port_constants_private_and_share_one_implementation` 检查重复端口与
操作仍只声明一次，C 消费者独立核对数值）：SWS_Rte_02575、02576、07143。
