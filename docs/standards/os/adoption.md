# 固定 FreeRTOS 采用与补丁责任

## 原件身份与上游说明

依据[ADR 0025](../../adr/0025-misra-layered-scope.md)，以下来源整体分类一次为采用代码，
并非逐项规范评估或供应方符合性证明。范围为两个既有 controlled x64 C99 目标、四个分析
profile 的 HostBatch／probe、直接 native 与 OS 消费者，原件集合取两份 source manifest。

V11.3.1 的 `054e14f3397023aa83813a65aa065fc4597d481b` 是 annotated tag 对象，
指向 commit `3a22924e0a9ddbbc8b0758881c33b3422a5cc20d`。
清单历史字段名 `commit` 保存的是前者；下载引用及 archive SHA256
`dd832f699e6ebee00366d7c737e259860d9d1fd5f2c2311c90dff16e8cfbcf61` 保持。
两份清单共31个原件与该档案逐字节相同；本记录不升级采用版本。

该提交发布的[MISRA.md](https://github.com/FreeRTOS/FreeRTOS-Kernel/blob/3a22924e0a9ddbbc8b0758881c33b3422a5cc20d/MISRA.md)
（SHA256 `6c0553a54fb8566b4ab2ef9bab73df0f22a431faea59292bde636b107658b207`）
说明使用 Coverity 2023.6.1 检查 MISRA C:2012，并列出源码注释内及
[Coverity配置](https://github.com/FreeRTOS/FreeRTOS-Kernel/blob/3a22924e0a9ddbbc8b0758881c33b3422a5cc20d/examples/coverity/coverity_misra.config)
中的处置。其 unsuppressed 部分区分示例、验证 port 缺项和实际集成义务。
该说明不证明本项目的主机端口、配置、补丁或全部 AMD／TC 已符合；不采用其 suppression，
不把其动态分配／标准I/O说明当成本项目批准，也不把供应方说明转成 Mandatory 偏离。

## 原件到实际消费者

`source-manifest.json`／`posix-source-manifest.json` 固定原件及有序补丁字节，
`target_spec.rs` 选择平台顺序及文件子集，`core/build.rs` 在副本中应用并计算最终
`kernelSha256`。生成包的 `target.json`、资产来源和 files seal 保留各自身份。
离线工具及 CMake 使用同一选择并在编译前核对最终内核；c-check 在自己的副本应用
相同链，记录实际 TU／程序、编译数据库和诊断。每个补丁必须实际改变其选择文件。

项目补丁始终是自有代码，包括从上游文本出发的类型修复。分析逐补丁传递行归属，
后续补丁恢复成原件文本也不把责任退回上游；未改动行随插删搬移。
`patched`／`adopted` 是来源标记，不是评估结果。宏展开及多位置诊断还需追到实际定义／
实参与调用方，不能只看报告首个文件名；项目 FreeRTOSConfig 和适配代码不因展开进入
kernel 而成为上游代码。原始退出码、passed=false、221项not_assessed、seal及严格比较
refusal保持；分析驱动或patch链变化仍按原比较器身份拒绝旧基线，不重盖旧seal。

## 补丁诊断的具体处置

正常整改以 `0027-port-diagnostic-types.patch` 及 Linux `0003-port-diagnostic-types.patch`
追加在各自既有链末尾。Windows等待返回值按DWORD比较，等待成功零下标用0U；
boolean clear直接作条件，uint32请求与0U比较；调度启动结果使用其真实BaseType_t。
分支局部seen及xThreadState移入唯一消费分支。R7.2／R10.3／R10.4的Required义务
按主文印刷60、86–88／PDF68、94–96及AMD2相应C11扩充核对；不改变失败与成功路径。

R18.4（Advisory，主文147／PDF155）两处指针-=改为下标取址。栈填充值游标仍按
`-portSTACK_GROWTH`推进；队列只在readFrom不等于head时退一个item，先转ptrdiff_t再
取负，避免无符号取负回绕。所选静态队列容量与itemSize在PTRDIFF_MAX以内，原来的
head→tail-item保护保持。R18.1的同数组边界义务及真实队列／栈消费者仍须成立。

| 原始报告 | 所选配置下的依据 | 处置 |
| --- | --- | --- |
| R5.5，task.h的pxCurrentTCB声明 | configNUMBER_OF_CORES=1；tasks.c同名宏仅位于多核else分支。实际预处理保留单核对象，不定义该宏。主文53／PDF61、AMD2.39及§6.7要求按实际宏集合和预处理后标识符判断。 | 此精确报告为误报；不外推多核。 |
| R20.7，POSIX portTASK_FUNCTION_PROTO／portTASK_FUNCTION | 参数形成函数名／形参声明符，没有形成缺括号表达式；主文159–160／PDF167–168。 | 两处误报，复用[声明宏核查](implementation.md#声明宏与表达式参数)。 |
| R17.3，POSIX四个__atomic_*及Windows平台调用 | 主文137–138／PDF145–146及AMD2附录H仅适用C90；实际编译C99且使用真实声明／GCC builtin。 | 语言不适用，复用[C99声明核查](implementation.md#c90隐式声明规则与当前编译)；不免除R1.1／R8.2／R8.4。 |
| R15.5，两个自有端口的早返回 | ADR0025的自有主机层GRP；仍审阅退出、状态和资源。 | Disapplied；不是逐条偏离或删除报告。 |

## 精确补丁接口的 Advisory 保留

本节是在当前工程委托下作出的独立有界项目选择，未借用或扩展其他实体的精确124键
批准、ErrorHook R18.6或SDK来源责任。R2.5为Advisory/System（主文44／PDF52；
TC2.5／TC2.39修订术语），R8.7为Advisory/System（主文70／PDF78，TC2.15区分调用TU）。
下面保留真实报告与就近MR12理由，不声明MISRA符合或正式法律意义上的偏离完成。

| 规则 | 补丁后kernel路径 | 精确实体 | 选择及控制 |
| --- | --- | --- | --- |
| 2.5 | `portable/ThirdParty/GCC/Posix/portmacro.h` | `portTICK_PERIOD_MS` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `portable/ThirdParty/GCC/Posix/portmacro.h` | `portEND_SWITCHING_ISR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `portable/ThirdParty/GCC/Posix/portmacro.h` | `portYIELD_FROM_ISR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `portable/ThirdParty/GCC/Posix/portmacro.h` | `portENABLE_INTERRUPTS` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskDYNAMICALLY_ALLOCATED_STACK_AND_TCB` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskSTATICALLY_ALLOCATED_STACK_ONLY` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskSTATICALLY_ALLOCATED_STACK_AND_TCB` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskRUNNING_CHAR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskBLOCKED_CHAR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskREADY_CHAR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskDELETED_CHAR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `tskSUSPENDED_CHAR` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `taskTASK_NOT_RUNNING` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `taskTASK_SCHEDULED_TO_YIELD` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `taskATTRIBUTE_IS_IDLE` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `tasks.c` | `taskBITS_PER_BYTE` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `queue.c` | `queueSEMAPHORE_QUEUE_ITEM_LENGTH` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 2.5 | `queue.c` | `queueMUTEX_GIVE_BLOCK_TIME` | 保持下述固定内核选择接口与原值；不制造使用点。 |
| 8.7 | `portable/ThirdParty/GCC/Posix/port.c` | `vPortCloseRunningThread` | 保留tasks.c条件生命周期调用的外部端口出口；不按样本裁剪链接。 |
| 8.7 | `portable/ThirdParty/GCC/Posix/port.c` | `vPortDeleteThread` | 保留tasks.c条件生命周期调用的外部端口出口；不按样本裁剪链接。 |
| 2.5 | `portable/MSVC-MingW/port.c` | `portSIMULATED_TIMER_THREAD_PRIORITY` | 保持下述固定内核选择接口与原值；不制造使用点。 |

R2.5表内tasks.c／queue.c常量支撑静态／动态分配标记、统计字符、单核／多核状态及队列
接口的既有条件分支；当前程序不使用仍是真实Advisory报告。项目早前给这些常量加类型
的补丁不把它们变成上游责任。可按当前profile删除／包围定义或改为enum，但会扩大本次
对固定内核可选分支的改造；保留有限既有实体及数值，使采用差异和上游更新更易核对。
POSIX四宏保持tick单位、ISR请求返回／转发及critical解除接口，Windows一个宏保持
既有模拟timer优先级常量；它们不宣称开启未支持运行能力。风险是失效宏与名字污染，
由固定版本、目标选择、精确表列实体、就近说明及实际预处理／消费者限制。

Linux两个R8.7接口由tasks.c的portCLEAN_UP_TCB／portPRE_TASK_DELETE_HOOK入口使用；
外部定义所在port.c与调用tasks.c不同。保留报告以免以声明代替真实调用论证；当前静态
任务程序关闭delete分支时也不伪造调用。将接口static化会破坏可选内核调用，将实现放头内
则暴露端口私有状态。本次保留真实既有链接合同，风险由private port头、固定调用方和
现有task身份／关闭消费者控制，不授予任意外部调用权。

目标ABI、配置、上游版本、patch顺序、宏body、实体、程序组成或调用方变化时重评。
表外任何新键均不由本节分类；同名不同表达式／位置也须重新核查。完整诊断位置、
源码行与程序出现次数及独立审查head／发现／处置归实施PR，不在这里复制每轮报告。
来源分类不覆盖自有系统交互，66项无checker评估继续沿独立责任范围进行。
