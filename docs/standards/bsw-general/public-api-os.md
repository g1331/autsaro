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

## 自有源码记录的当前输入消费

本节的 `owned-source` 上下文只绑定原标准篇已有的精确记录与类别实例，沿用
[ADR 0023](../../adr/0023-autosar-required-conflict-disposition.md)、
[ADR 0025](../../adr/0025-misra-layered-scope.md) 的原决定范围。机器关联复用
`c_standard` 的实体、shape、MR12 双向检查和主机地址八个表达式；MemMap R20.1 位置须
通过[原协议检查](../memmap/README.md#memmap-类别偏离)，R20.5 仅取原表精确登记位置，
未登记的内部属性／状态撤销继续由原任务接续。C99 下 R17.3 的适用性沿
[OS 原裁决](../os/implementation.md#c90隐式声明规则与当前编译)。

当前有界消费只接纳上述匹配中的 Advisory 记录及已说明的语言不适用位置。
原生成 159 键继续使用独立的 `generated-retention` 上下文和消费者。
Advisory 的记录安排沿各原篇既有理由、替代、风险及控制，适用主文 §6.2.3；
上下文的 `approved` 表示这些已有记录在所列输入交集内有效。Required 的精确关联
仍逐位置保留 `formal_deviation_pending`，Mandatory 保留项仍拒绝，其他模块裁决、
SDK、采用内核与人工组保持各自未完成状态。新增实体或 body、目标、程序、producer、
工具、来源和评估正文变化时重评；本节没有增加偏离、类别或诊断基线。

完整双平台四样本的来源、原始结果、逐位置消费和拒绝证据归
[实施 PR](https://github.com/g1331/autsaro/pull/449)。输入绑定采用
[源码验收格式](../../development/testing/source-acceptance.md)，绑定块自身排除于摘要；
其他正文及解析器保持逐字节绑定。剩余完整消费与 Required 正式材料由
[#439](https://github.com/g1331/autsaro/issues/439) 接续。
<!-- source-assessment-context-begin -->
{
  "id": "owned-source-449-inputs",
  "evidence": {
    "head": "dd5477cae69217e89fe7b57063aeb268b0e5f675",
    "receipt": "https://github.com/g1331/autsaro/pull/465#issuecomment-6718847981"
  },
  "cases": [
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "93f1a0540ec43bc94f5f964e2990707e6fa93ef2ef7ad82827fcb8e9e4a5ea50"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "ca8da517c0a24b1543e38f7535bd720bc32d48aa25e8a1256e8006f3ce8c3c8e"
    },
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "7c20ac22dfefcee4b9e0884f76b58127489e9d602cafa2d14c03f7affe245d4f"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "6783bbdc24f98b027f2aa62889280275ebed2c35c6d72f5223e394cb44d9f286"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "e04d9c2f4481ccb9471f44bae8832a4c4d8a5e0bc46f6930a19d2a5d0a4a73b6"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "913c1d0a6b50b24a31f24c1ecb5eb670417fc1040466e10fb5a3e5fef6e024ed"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "5b2da999ff83696dfd62cc78e2efa6500429ab5a70dcac20e09265014edaa427"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "5df130fdd239ca83d32c5f700e1a140c90310c25d6cff782456868abe2b0fe66"
    }
  ],
  "records": {
    "kind": "owned-source"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
