# OS 标准服务的链接诊断

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

## 保留形式与替代

R8.7为Advisory/System。宏／header inline或按程序裁剪是可行替代，标准服务存在不证明外部链接必需；维护者选择在本表程序范围保留当前库形式。既有独立消费者、状态／错误语义、ARTI顺序及可配置CODE段保持。可见面误用风险以原有参数、上下文、资源与独占区控制限定。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `isOsStarted` | shape=`OS_MEMMAP boolean isOsStarted ( void ) {`; 范围 `All/Both/Both`；SWS_Os_91034；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `TerminateTask` | shape=`OS_MEMMAP StatusType TerminateTask ( void ) {`; 范围 `All/Both/Both`；OSEK OS 2.2.3 §13.2.3.2；Syntax:            StatusType TerminateTask ( void )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `NextScheduleTable` | shape=`OS_MEMMAP StatusType NextScheduleTable ( ScheduleTableType ScheduleTableID_From , ScheduleTableType ScheduleTableID_To ) {`; 范围 `All/Both/Both`；SWS_Os_00191；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 保留现有库／控制／观察出口；签名、原有参数／状态／同步与实际消费者合同保持。 | 维护者 2026-10-30精确124键决定 |

## 受控 C99 的头内服务入口

38个既有纯包装已由 Os_ArtiServices.c 迁入 Os.h 的原 CODE 映射区间，保留具名参数和全部函数体操作；其中29个原表位置以实际源码整改承接，待本批统一验证。不再提供这些入口的原外部定义；函数指针调用仍可用，地址属于各调用TU，不承诺跨TU地址相等。Os_Service.h只归并38个头内入口需要的既有实现声明和6个trace声明，不新增转发函数、状态或使用点。标准允许形式与固定配置依据为OS §4.6.2、BSWGeneral §7.1.10。

TerminateTask还读取原生栈角色和当前Task trace状态；引入完整栈布局会把FreeRTOS配置类型带入公开头，因此本批保留原外部实现。isOsStarted、NextScheduleTable也保留原名字、参数和命名基线；三项现按上述2026-10-30精确决定保留外部形式，不将头内迁移当作批准依据。错误、资源／上下文、嵌套、退出和trace控制沿现有实现，ErrorHook R18.6状态不变。

正常MemoryMap消费者通过真正头内SetEvent及函数指针观察 enter、entry、implementation、capture、return、leave 顺序；六种删除反例须先编译成功，再由专用退出4拒绝。段属性删除反例及全START／STOP结构继续保留；局部函数使用既有local_names机制核对真实ELF／PE段绑定，不再用同名公开桩定义代替服务。
