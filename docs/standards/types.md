# 标准类型、平台类型与通信栈类型

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

| 文档 | 已定位条款 | 约束要点 |
| --- | --- | --- |
| `AUTOSAR_CP_SWS_StandardTypes.pdf`（`a9f0cc790f8b325485b6b4a2c6f44c89d9f64192274a2a0ca8e827976bc06a81`） | — | `Std_ReturnType`、boolean 与标准常量的实际类型 |
| `AUTOSAR_CP_SWS_PlatformTypes.pdf`（`c0e4b1b97900ebea81d701542e55de05326fe18e245cfb81dbe9c154c493e78b`） | — | 公共整型、boolean 及受控目标 CPU／位序／字节序；只按实际受控 x64 C99 profile 核定 |
| `AUTOSAR_CP_SWS_CommunicationStackTypes.pdf`（`31f62487c5e27ec1863b3febc79e336ef42eb4a50f2dc52776518d92741a7786`） | `SWS_COMTYPE_00005／00008／00011／00012／00027／00037／91001` | PduId／Length／Info、BufReq、Retry、Cbk handle 类型；`PduIdType`／`PduLengthType` 由真实 EcuC 最大值派生 `ComStack_Cfg.h` |

## C99 essential type 与实际常量

`Platform_Types.h` 的 `boolean`、TRUE和FALSE分别按固定R24-11 `SWS_Platform_00026`（页19）、`00054`／`00055`／`00056`（页29–30，含原表及续页）核对：本实现用_Bool，宏保留规定的true／false映射及ifndef。MISRA 2013主文附录D.6“Boolean constants”（页199／PDF207）明确给予stdbool.h的true／false essentially Boolean类型。因此，赋给boolean或与boolean比较的这些宏展开，不能仅因底层0／1为int就认定R10.3或R10.4真实违反；其他整数、原生BOOL参数和返回值须按各自类型另外核查。固定addon在嵌套宏的识别限制不改变原始报告，不以改写规定的宏、删除include或强制转换用户代码解决。

整数状态接口与布尔局部状态分别处理：`Os_ArtiServiceEnter`、`Os_BackendInterruptEnabled`、`Os_InterruptAllows`和中断服务许可保留既有sint32的0／1结果，并显式用条件表达式选择整数值；`Os_ErrorResult`的ISR判断和中断`change`的running局部量使用boolean及直接布尔条件。依据R10.3（主文页86–87／PDF94–95）、R10.4（页88／PDF96）；TC2的case标签勘误及AMD2语言扩充不改变这些C99赋值。没有用Boolean到整数的强制转换取代类型选择。

INVALID_TASK按OSEK OS 2.2.3 §13.2.4（页54）的TaskType常量义务显式保持TaskType；INVALID_ISR是本实现对`SWS_Os_00782`（页156，implementation_specific）的类型选择，同样保留ISRType。两个哨兵的值仍为255，ABI中的TaskType／ISRType均未改宽；C99消费者同时检查实际宏表达式宽度和值，防止整数提升后的UINT8_MAX被误当成有目标类型的常量。该选择不新增可接受ID或改变GetTaskID／GetISRID的无活动对象语义。
