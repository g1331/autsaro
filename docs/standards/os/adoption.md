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

采用分类仍需检查自有配置的接缝。例如旧双平台工件的IDLE名字越界报告位于未改动的
任务循环；真实-dM固定configIDLE_TASK_NAME为"IDLE"、configMAX_TASK_NAME_LEN为16u，
复制到索引4的终止NUL后即break，不读取字符串之外的索引。固定上游说明的R18.1节
描述同一边界。它不需要项目偏离，也不允许把别的缓冲区／配置一并判为安全。

## 原件到实际消费者

`source-manifest.json`／`posix-source-manifest.json` 固定原件及有序补丁字节，
`target_spec.rs` 选择平台顺序及文件子集，`core/build.rs` 在副本中应用并计算最终
`kernelSha256`。生成包的 `target.json`、资产来源和 files seal 保留各自身份。
离线工具及 CMake 使用同一选择并在编译前核对最终内核；c-check 在自己的副本应用
相同链，复用`require_kernel_identity`核对同一最终map，再验证C99翻译单元，
记录实际TU／程序、编译数据库和诊断。每个补丁必须实际改变其选择文件。
`test_ecu_analysis_requires_the_same_patched_kernel_bytes_as_builds`以正常分析及补丁后
额外一个换行的有效C副本证明身份不符会在TU编译／分析前拒绝，保留passed=false及具体文件。

项目补丁始终是自有代码，包括从上游文本出发的类型修复。分析逐补丁传递行归属，
后续补丁恢复成原件文本也不把责任退回上游；未改动行随插删搬移。
`patched`／`adopted` 是来源标记，不是评估结果。宏展开及多位置诊断还需追到实际定义／
实参与调用方，不能只看报告首个文件名；项目 FreeRTOSConfig 和适配代码不因展开进入
kernel 而成为上游代码。Windows旧工件的lSuccess赋值、ulSwitchRequired清零／比较及
lMutexNeedsReleasing赋值虽落在未改动行，根因包括项目0015修改的类型声明／BaseType_t；
本次按自有因果一并整改，旧adopted标记及原始报告不改写。原始退出码、passed=false、221项not_assessed、seal及严格比较
refusal保持；分析驱动或patch链变化仍按原比较器身份拒绝旧基线，不重盖旧seal。

## 补丁诊断的具体处置

正常整改以 `0027-port-diagnostic-types.patch` 及 Linux `0003-port-diagnostic-types.patch`
追加在既有公共链及Linux端口链末尾。Windows等待返回值按DWORD比较，等待成功零下标用0U；
boolean clear直接作条件，uint32请求与0U比较；调度启动结果及mutex释放标志使用其真实BaseType_t，uint32切换位图以0U清零／比较。
分支局部seen及xThreadState移入唯一消费分支。
等待值依据Microsoft的[单对象](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)
与[多对象](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitformultipleobjects)契约；
原等待对象、wait-all选择、超时／abandoned／失败拒绝保持。R7.2／R10.3／R10.4的Required义务
按主文印刷60、86–88／PDF68、94–96及AMD2相应C11扩充核对；不改变失败与成功路径。

R18.4（Advisory，主文147／PDF155）两处指针-=改为下标取址。栈填充值游标仍按
`-portSTACK_GROWTH`推进；队列只在readFrom不等于head时退一个item，先转ptrdiff_t再
取负，避免无符号取负回绕。所选静态队列容量与itemSize在PTRDIFF_MAX以内，原来的
head→tail-item保护保持。R18.1的同数组边界义务保持。
既有public-types实际执行FIFO／front／overwrite／reset与零item队列，
`test_queue_front_pointer_bounds_and_order_mutations`继续拒绝先减后判、丢失步进及错误wrap。
`test_patched_kernel_stack_scan_uses_each_byte`从封存构建包含真实tasks.c，观察整word／
不足一个word的扫描结果；加倍步进的可编译变异须在行为断言失败，检测器同样执行。

| 原始报告 | 所选配置下的依据 | 处置 |
| --- | --- | --- |
| R7.2，Windows prvWaitForObject中的INFINITE | 实际TU的-dM收据为`#define INFINITE 0xffffffff`；项目及内核源码未定义该宏，编译器内建宏亦无该定义。它由所选MinGW SDK头展开，整改前已有同一报告。主文60／PDF68要求预处理后unsigned常量带U后缀。 | 保留真实Required诊断及未评估状态，归SDK来源责任（#384）；不因调用行标为patched而改称自有常量，也不声明通过或偏离完成。自有等待参数及失败语义仍由本篇负责。 |
| R5.5，task.h的pxCurrentTCB声明 | configNUMBER_OF_CORES=1；tasks.c同名宏仅位于多核else分支。实际预处理保留单核对象，不定义该宏。主文53／PDF61、AMD2.39要求按实际宏集合和预处理后标识符判断。 | 此精确报告为误报；不外推多核。 |
| R20.7，POSIX portTASK_FUNCTION_PROTO／portTASK_FUNCTION | 参数形成函数名／形参声明符，没有形成缺括号表达式；主文159–160／PDF167–168。 | 两处误报，复用[声明宏核查](implementation.md#声明宏与表达式参数)。 |
| R17.3，POSIX四个__atomic_*及Windows平台调用 | 主文137–138／PDF145–146及AMD2附录H仅适用C90；实际编译C99且使用真实声明／GCC builtin。 | 语言不适用，复用[C99声明核查](implementation.md#c90隐式声明规则与当前编译)；不免除R1.1／R8.2／R8.4。 |
| R15.5，两个自有端口的早返回 | ADR0025的自有主机层GRP；仍审阅退出、状态和资源。 | Disapplied；不是逐条偏离或删除报告。 |

## 精确补丁接口的 Advisory 保留

本节是在当前工程委托下作出的独立有界项目选择，未借用或扩展其他实体的精确124键
批准、ErrorHook R18.6或SDK来源责任。R2.5为Advisory/System（主文44／PDF52；
TC2.5／TC2.39修订术语），R8.7为Advisory/System（主文70／PDF78，TC2.15区分调用TU）。
下面保留原始报告与就近MR12理由，不声明MISRA符合或正式法律意义上的偏离完成。
表内 R2.5 保持既有接口与原值，控制见下文；记录来源均为 #383／PR #395 的既有项目选择。

| 规则 | 补丁后 kernel 路径与实体 | 选择及形状 | 记录来源 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 2.5 | `kernel/portable/ThirdParty/GCC/Posix/portmacro.h` `portTICK_PERIOD_MS` | shape=`#define portTICK_PERIOD_MS ((TickType_t)1000u / configTICK_RATE_HZ)` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/portable/ThirdParty/GCC/Posix/portmacro.h` `portEND_SWITCHING_ISR` | shape=`#define portEND_SWITCHING_ISR(requested) return (requested)` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/portable/ThirdParty/GCC/Posix/portmacro.h` `portYIELD_FROM_ISR` | shape=`#define portYIELD_FROM_ISR(requested) portEND_SWITCHING_ISR(requested)` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/portable/ThirdParty/GCC/Posix/portmacro.h` `portENABLE_INTERRUPTS` | shape=`#define portENABLE_INTERRUPTS() vPortExitCritical()` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskDYNAMICALLY_ALLOCATED_STACK_AND_TCB` | shape=`#define tskDYNAMICALLY_ALLOCATED_STACK_AND_TCB ( ( uint8_t ) 0 )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskSTATICALLY_ALLOCATED_STACK_ONLY` | shape=`#define tskSTATICALLY_ALLOCATED_STACK_ONLY ( ( uint8_t ) 1 )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskSTATICALLY_ALLOCATED_STACK_AND_TCB` | shape=`#define tskSTATICALLY_ALLOCATED_STACK_AND_TCB ( ( uint8_t ) 2 )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskRUNNING_CHAR` | shape=`#define tskRUNNING_CHAR ( 'X' )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskBLOCKED_CHAR` | shape=`#define tskBLOCKED_CHAR ( 'B' )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskREADY_CHAR` | shape=`#define tskREADY_CHAR ( 'R' )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskDELETED_CHAR` | shape=`#define tskDELETED_CHAR ( 'D' )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `tskSUSPENDED_CHAR` | shape=`#define tskSUSPENDED_CHAR ( 'S' )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `taskTASK_NOT_RUNNING` | shape=`#define taskTASK_NOT_RUNNING ( ( BaseType_t ) ( -1 ) )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `taskTASK_SCHEDULED_TO_YIELD` | shape=`#define taskTASK_SCHEDULED_TO_YIELD ( ( BaseType_t ) ( -2 ) )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD` | shape=`#define taskTASK_IS_RUNNING_OR_SCHEDULED_TO_YIELD( pxTCB ) ( ( ( pxTCB )->xTaskRunState != taskTASK_NOT_RUNNING ) ? ( pdTRUE ) : ( pdFALSE ) )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `taskATTRIBUTE_IS_IDLE` | shape=`#define taskATTRIBUTE_IS_IDLE ( UBaseType_t ) ( 1U << 0U )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/tasks.c` `taskBITS_PER_BYTE` | shape=`#define taskBITS_PER_BYTE ( ( size_t ) 8 )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/queue.c` `queueSEMAPHORE_QUEUE_ITEM_LENGTH` | shape=`#define queueSEMAPHORE_QUEUE_ITEM_LENGTH ( ( UBaseType_t ) 0 )` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/queue.c` `queueMUTEX_GIVE_BLOCK_TIME` | shape=`#define queueMUTEX_GIVE_BLOCK_TIME ( ( TickType_t ) 0U )` | 维护者 #383 |
| MISRA C:2012 Rule 8.7 | `kernel/portable/ThirdParty/GCC/Posix/port.c` `vPortCloseRunningThread` | 保留tasks.c条件生命周期调用的外部端口出口；不按样本裁剪链接。 shape=`void vPortCloseRunningThread(void *task, volatile BaseType_t *pending_yield) {` | 维护者 #383 |
| MISRA C:2012 Rule 8.7 | `kernel/portable/ThirdParty/GCC/Posix/port.c` `vPortDeleteThread` | 保留tasks.c条件生命周期调用的外部端口出口；不按样本裁剪链接。 shape=`void vPortDeleteThread(void *task) {` | 维护者 #383 |
| MISRA C:2012 Rule 2.5 | `kernel/portable/MSVC-MingW/port.c` `portSIMULATED_TIMER_THREAD_PRIORITY` | shape=`#define portSIMULATED_TIMER_THREAD_PRIORITY THREAD_PRIORITY_NORMAL` | 维护者 #383 |

R2.5表内tasks.c／queue.c常量支撑静态／动态分配标记、统计字符、单核／多核状态及队列
接口的既有条件分支。分配组合、统计格式化、多核／delete、计数／递归信号量分支由当前
配置排除，实际-dM输出中没有这些定义。这里保守记录保留源定义及原始Advisory报告，
不把每条都称为当前程序执行中的真实违反，也不只凭-dM缺席宣称R2.5通过。
项目早前给这些常量加类型的补丁不把它们变成上游责任。
可按当前profile物理裁剪定义或改为enum，但会扩大本次
对固定内核可选分支的改造；保留有限既有实体及数值，使采用差异和上游更新更易核对。
POSIX四宏保持tick单位、ISR请求返回／转发及critical解除接口，Windows一个宏保持
既有模拟timer优先级常量；它们不宣称开启未支持运行能力。风险是失效宏与名字污染，
由固定版本、目标选择、精确表列实体、就近说明及实际预处理／消费者限制。

Linux两个R8.7接口分别连接tasks.c中受INCLUDE_vTaskDelete控制的
portCLEAN_UP_TCB／portPRE_TASK_DELETE_HOOK入口。当前该开关为0，本批程序没有这些
跨TU调用，保留真实R8.7报告；不能用另一个TU中的声明或条件源码调用反写为误报。
保持既有private port链接合同，避免为当前样本裁剪内核／端口接缝；物理删除或把实现及
私有状态移入头文件是另外的设计取舍。本次不开放删除能力，不制造调用；两个hook如被
调用均走既有E_OS_STATE终止。风险由固定私有头／实体、fail-fast函数体及配置范围限制，
task身份／关闭回归用于确认整体协议未变，不据此证明可选动态删除能力。

目标ABI、配置、上游版本、patch顺序、宏body、实体、程序组成或调用方变化时重评。
表外任何新键均不由本节分类；同名不同表达式／位置也须重新核查。完整诊断位置、
源码行与程序出现次数及独立审查head／发现／处置归实施PR，不在这里复制每轮报告。
来源分类不覆盖自有系统交互，66项无checker评估继续沿独立责任范围进行。

## 当前集成义务与消费

本篇的原件分类由 `freertos-integration` 上下文消费，格式沿用源码验收的
`source-assessment-context`；补丁 R2.5／R8.7 表复用 `c_standard.deviation_findings`
的 MR12、实体、shape 和双向关联，表中“记录来源”引用原 #383 的有限项目选择。
当前消费者只接纳该表的两项 Advisory 规则，Required／Mandatory 不经保留表接纳。
R5.5／R20.7 只接纳上节所列精确声明与声明宏，R17.3 取实际最后一个语言选项为 C99；
R15.5 继续走既有 GRP。SDK 来源及表外补丁位置保留各自阻塞。

每轮从两份 manifest 核对原件与 ownership 摘要，再从这些原件重放实际 target 的有序
补丁，沿每次修改传递行归属，并核对 replay 和已分析副本的最终 kernelSha256。
因此补丁后来恢复成原件文本仍保留项目责任。混合诊断逐位置核查；采用上下文额外绑定
含 kernel 位置的完整诊断集合（包括程序、非 kernel 位置与来源字段），新增或改变报告
需要重新评估。上下文的 inputIdentity 由 `freertos_identity(summary, input_identity(...))`
计算，包含两份原件 manifest 的当前字节；普通三组和 owned-source 保留原算法。

集成证据的适用范围为当前两个 x64 C99 controlled 目标和四个 profile 的两个程序。
复核以下接缝后才可把本组上下文标记为 approved，并附实际 head 与验证收据。
没有对应当前输入的记录，或记录仍为 pending 时，原件及补丁的本组位置持续阻塞。

| 固定上游说明中的义务 | 项目侧实际接缝与核查依据 | 实际消费者 |
| --- | --- | --- |
| D4.7 的返回信息及端口补全 R8.6 | `Os_Backend.c` 检查 `xTaskCreateStatic` 的 NULL、`xTaskResumeFromISR` 的切换请求；`FreeRTOSConfig.h` 的 configASSERT 进入 Os_BackendAssert；target 选择实际 port，与 Hook／内存回调一起链接 | 正常 HostBatch／probe 编译链接；OS public-types、nonstatus-errors、error-hooks、calling-context |
| R8.4 的外部对象及 R11.1 的可选回调 | 单核 pxCurrentTCB 由补丁后的 task.h 声明；queue registry 默认大小为 0；stream_buffer.c／event_groups.c／timers.c 均未进入 kernelSources，可选回调与 EventGroup handle 无实际调用 | 实际头声明、TU 与宏收据；正常双程序链接 |
| R2.2 的 scheduler end | 两端口的 vPortEndScheduler 与 Os_Backend 的关闭顺序连接；启动失败及正常关闭沿真实宿主线程终止和回收路径 | OS lifecycle、returned-task、idle-state、port-restart；native HostBatch |
| D4.12／R21.3 的 heap 选择 | 静态 Task／Idle 对象，configSUPPORT_STATIC_ALLOCATION=1、DYNAMIC_ALLOCATION=0；kernelSources 没有 heap；宿主线程、栈和 gate 的动态资源沿 OS 主机设施既有类别单独负责 | OS stack、lifecycle、capacity；正常目标构建与来源检查 |
| R11.3 的静态对象别名 | StaticTask_t 的项目存储通过 xTaskCreateStatic 进入内核，核对大小、对齐、生命周期及 configASSERT；配置关闭 mutex／timers／co-routines，已交付队列由独立原生消费者验证 | OS public-types、stack；真实内核队列 front／wrap／overwrite／reset 反例 |
| R11.5 的 list owner／handle／缓冲区 | TaskHandle 来自静态创建，list owner 在同一个任务对象身份下存取；list sentinel 使用完整 ListItem_t；栈扫描及队列容量边界由实际对象和配置限定 | OS activation、finish、events、public-types；实际 patched tasks.c 的逐字节栈扫描变异 |
| R14.3／R18.1 的配置与 IDLE 字符串 | 固定 configMAX_TASK_NAME_LEN=16u、IDLE="IDLE"；遇 NUL 终止后停止复制；固定配置的单核声明与实际宏集合支撑上节 R5.5 判断 | 实际 TU 编译和宏收据；OS idle-state、task-hooks |
| R21.6 的演示／统计工具 | examples 未交付；configUSE_STATS_FORMATTING_FUNCTIONS 的默认关闭使统计格式化实现退出本配置；本项目调用方和主机 I/O 仍按自己的规则消费 | 最终 kernelSources／程序 TU、实际预处理与符号／链接证据 |

以上列的是集成方核查对象。固定上游说明及其 Coverity 配置只说明供应方所述范围；
本项目继续保留全部原始报告、规则未评估状态和未完成正式义务。上游原件的接纳表示
当前采用责任与集成证据已建立，整体 MISRA 结论继续按 221 项的独立评估出口处理。
本篇及表、原件、补丁顺序／字节、配置、程序、工具或实际交互改变时，旧绑定失效。
<!-- source-assessment-context-begin -->
{
 "id": "freertos-current-inputs",
 "evidence": {
  "head": "4ae3fe56823490c2f114e90bc6bb4b02940ff90c",
  "receipt": "https://github.com/g1331/autsaro/pull/455#issuecomment-6714031019"
 },
 "cases": [
  {
   "caseId": "standard-ecu",
   "target": "linux-x64-controlled-v1",
   "inputIdentity": "04f84e2905cc6eacd0a97dd8c75eade558b5911626fa8f28a2c088f4c974f410"
  },
  {
   "caseId": "user-application",
   "target": "linux-x64-controlled-v1",
   "inputIdentity": "bd97145b509edddfb2a3ab891a191646f104cc354f80f859ae7bde3e9d511d9c"
  },
  {
   "caseId": "component-pipeline",
   "target": "linux-x64-controlled-v1",
   "inputIdentity": "872425bbfc994d75a94cbf21447db7207de427cb7051c5c922b7e57f5d9d1f4e"
  },
  {
   "caseId": "no-diagnostic",
   "target": "linux-x64-controlled-v1",
   "inputIdentity": "96d7e5df5130cb563ece186e667af1fff7df2cc7cc39af52275ceb90715f1789"
  },
  {
   "caseId": "standard-ecu",
   "target": "windows-x64-controlled-v1",
   "inputIdentity": "8cb5fcbb7c3143fbc3015bc3282ea41b3423d8236384116ff665b3dda1ce3844"
  },
  {
   "caseId": "user-application",
   "target": "windows-x64-controlled-v1",
   "inputIdentity": "0d42bdff4b3540b745ec662510412bb1fd59631a923ec6e2ab382858f980181f"
  },
  {
   "caseId": "component-pipeline",
   "target": "windows-x64-controlled-v1",
   "inputIdentity": "a1c9258412f412255c836393a71d7f3d2a03a948550a45276aea70e323fc9d24"
  },
  {
   "caseId": "no-diagnostic",
   "target": "windows-x64-controlled-v1",
   "inputIdentity": "32d7d958a183a4be4ec662c06ba17d30293ff66166b2a20d0178cb7436cd4adb"
  }
 ],
 "records": {
  "kind": "freertos-integration"
 },
 "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
