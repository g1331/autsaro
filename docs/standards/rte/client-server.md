# 类型化同步调用与端口常量

## 所选 BSW 的静态绑定

服务端 runnable 的 `bswTarget` 保存输入 mapping、behavior、entity、entry、执行属性及
所选 `SymbolContract`。应用 runnable 沿原来的应用 producer 路径生成；BSW target 由
`RuntimeCatalog` 核定参数、返回、唯一声明与定义 owner，复用普通 C/S 的直接调用。
绑定后的 runnable 从应用种子和应用原型中排除，实际 BSW 头由 RTE 实现包含。

固定 R24-11 SpecificationsARXML 的 TPS_SWCT_01407–01409、TPS_BSWMDT_04028、
constr_4071／10067 及 SWS_Rte_01132／06704 正文已核对；BSWMDT PDF 页 124 图 5.18
连接端口常量、operation 参数与有序 `SwServiceArg`。输入要求 mapping 两端属于各自的
behavior，服务 behavior 有一个选中的 C `SwcImplementation`；所选同步实体与 runnable
的 symbol、重入性和零 minimumStartInterval 保持一致。显式内存方法沿输入引用核对。
服务连接至少有一个 ServiceSwComponentType 端点。

已核对（`bsw_mapping_rejects_unselected_behavior_in_the_same_module` 拒绝同一 module 内
未被唯一 C `BswImplementation` 选中的 behavior，并定位源 mapping）：TPS_SWCT_01408。
已核对（`bsw_mapping_rejects_module_identity_inconsistent_with_the_producer` 拒绝与 catalog
固定 producer 不符的源 module 身份；结合已有 scheduler prefix、entry 与 runnable symbol
校验，保留所选 backend 的实际 `SymbolContract`）：TPS_BSWMDT_04138、constr_4071。
两个反例均通过官方 R24-11 XSD，正常校验与计划构建同时断言诊断码和源 mapping 位置。

已核对（`bsw_role_parameters_use_the_selected_entry_names_and_native_aliases` 将输入
`Handle`／`Reading` 命名为所选 entry 的 `CounterID`／`Value`，并核对实际调用表达式）：
SWS_Rte_06704。

Counter 的描述与既有 `Rte_Os.h`／`Rte_OsService.c` 绑定消费同一组 `Operation`、端口
常量和 BSW target。`GetCounterValue`／`GetElapsedValue` 的声明及 inline 定义归
`Os.h`，状态与错误处理经 `Os_Implementation_*` 进入 `Os_Time.c`；RTE wrapper 继续
保留原 API、tick 单位及原生返回和输出保护。测试分别使用 SystemCounter 和 AuxClock。
普通 `Ping` fixture 显式映射到 `Can_MainFunction_Wakeup`，并保留应用 Transform server，
从真实 Can 实现的独占区进入次数观察调用；删除调用的行为变异应失败。

## 普通同步调用

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
