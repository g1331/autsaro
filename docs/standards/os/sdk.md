# SDK 原头来源与接口责任

本篇承接[声明与 SDK 接口](declarations.md#sdk-声明与常量)，记录采用来源与自有调用责任。

## SDK 原头来源类别

按 [ADR 0025](../../adr/0025-misra-layered-scope.md)，原头以 target、provider 包名／版本、
包内头路径和实际 SHA-256 分类。范围为 Linux 的 libc6-dev/glibc、libgcc 开发头、实际
包含的 Linux UAPI，以及 Windows 受控目标选择的 MinGW-w64 API／CRT 头。项目源码、
FreeRTOS 项目补丁及自有适配按各自责任评估。

正常 c-check 在 sealed target／ABI 校验后，以同一 GCC、flags 和 include 顺序逐 TU
编译、取宏值，并运行 -E -dD／-MD。sdkTranslationUnits 保存实际打开头的闭包，
sdkHeaders 保存包版本、包内路径、原件校验和内容摘要，sdkMacroInputs 保存使用时刻
的活动定义及传递替换链。Linux 核对 dpkg 原件清单，Windows 核对所选 compiler 安装的
MSYS2 pacman desc／mtree；发行签名与最终运行库身份各自有责任，见[实际 provider](native-facilities/README.md#实际provider及异步调用)。

sourceOrigins 只附加来源，保留原始诊断和项目 patch 归属。macro-input 是活动宏输入，
macro-reference 是未展开的替换列表引用；同一行含 SDK 宏仍需证明具体违反的来源。
R7.2 将实际 addon 的 unsigned 操作数与活动对象宏字面量及别名链关联，只有唯一匹配才
记录 unsigned-literal。函数宏、复杂链及多义操作数保持原通用解析的未决状态；下节
精确类别独立核对已审定义。

编译器预定义须与实际 compiler 收据的名字、值及函数形式一致。包外头、未知 provider、
目标／包错误、原件变更和项目重定义均拒绝；项目 #line、依赖缺失、循环及跨行实参歧义
保持可见，反例分别验证。目标、provider、头字节或编译选择改变即重评。
semanticIdentity.version=2 绑定 SDK 和分析驱动；原始诊断、退出码与评估状态保持。

## 当前集成义务与类别处置

[用户决定（2026-11-08）](https://github.com/g1331/autsaro/issues/444#issuecomment-6716477031)
批准 Windows 六个宏的 R7.2／R7.3 及 CONTEXT_CONTROL 的 R10.1，并授权无法在项目侧
修复的 SDK 定义按 ADR 0025 登记有界类别。评估基线为 C99、固定主文第三版及
AMD1–4／TC1–2；七份原件按 SHA256SUMS 核验。范围为两个受控 x64 目标、四个既有
profile、HostBatch／probe 的实际 TU。项目侧处置由下列记录消费，完整取证与复核归 PR。

| 规则（均 Required） | 类别与原文 | 原因、风险控制与替代判断 |
| --- | --- | --- |
| R7.1 | Linux O_RDONLY；主文 59–60／PDF 67–68，AMD2.27 追加 C11 | glibc 原头 bits/fcntl-linux.h:43 定义为 00，实际 open 实参展开为八进制零。保留公开只读标志，避免复制或重定义 SDK 数值；Os_LinuxMappedStack 检查 descriptor、read 结果及 close 失败。仅覆盖下列一个使用点及 HostBatch／probe；头身份、使用点或程序变化即重评。 |
| R7.2 | Windows INFINITE、STATUS_ACCESS_VIOLATION、STATUS_STACK_OVERFLOW；主文 60–61／PDF 68–69，AMD2.46 | 真实 GCC 中 0xffffffff、0xC0000005、0xC00000FD 为 unsigned int 且缺 U；DWORD cast 保留字面量义务。保持供应方名字与数值，拒绝复制常量或重定义。等待返回检查、异常种类／参数数／guard 地址筛选保持。每 profile 11 处。 |
| R7.3 | Windows CONTEXT_CONTROL、WAIT_TIMEOUT、ERROR_BROKEN_PIPE；主文 61–62／PDF 69–70 | SDK 的 __MSABI_LONG 拼接产生 0x1l、258l、109l。外层 cast／包装保留小写后缀；修改或分叉原头会失去包原件身份。保持实际 GCC 展开值，分别由上下文、等待和断管消费者验证。每 profile 7 处。 |
| R10.1 | Windows CONTEXT_CONTROL signed OR；主文 82–84／PDF 90–92，TC2.17–18、AMD3.20、AMD4.51 | 0x100000 与 0x1l 均 signed；结果 DWORD cast 保留内部运算。非负 signed 常量例外只涉及移位右操作数。固定掩码值为 1048577，ContextFlags 后由受控 GetThreadContext 接口消费。Backend 三处、项目 port 一处；checker 零报告仍逐点核验。 |
| R5.5 | Linux sa_sigaction／si_addr；主文 53／PDF 61，AMD2.39、§6.7 | 宏展开后仍留下同名成员；标准头处理内的对象按 §6.7 归供应方，四个头外接口使用保留本条义务。直接改写访问会依赖 glibc 私有 union 布局，包装或 typedef 继续使用同一宏。采用公开 SA_SIGINFO 接口，注册检查失败返回；guard handler 检查实际故障地址并发布预分配记录。 |
| R5.7 | Linux sigaction tag／函数名字；主文 55／PDF 63，AMD2.40、§6.7 | 头外 struct sigaction 和函数 sigaction 使用跨 namespace 同名；关联 typedef 与同头重复定义例外均未消除该接口义务。重命名 SDK 声明、手写 ABI 或避开结构体的包装均无法保留公开类型契约。两处类型、四处调用分别记录；SA_SIGINFO handler 的类型和失败路径由实际 GCC／native 消费。 |

R7.1 的项目侧类别依据 [2026-11-13 批准](https://github.com/g1331/autsaro/pull/476#issuecomment-6851299668)，
仅增加当前 libc6-dev 2.39-0ubuntu8.9 的 `O_RDONLY→00`、`Os_LinuxNative.c:180`
和实际包含它的 HostBatch／probe。八进制拼写的阅读风险由公开宏名、固定原件及
只读模式控制；`descriptor >= 0` 后才读取，读取终止后检查 `close`。返回 0 的失败路径
由调用者继续拒绝该映射结果。保持原始违反；正式义务沿本组现有程序记录。

R5.5／R5.7 落实同一决定的 SDK 通用原则，自有名字及运算另评。原头历史调查见
[PR #371](https://github.com/g1331/autsaro/pull/371)；原头内部承担供应方责任，
自有接口的参数、失败和生命周期沿[原生设施](native-facilities/README.md)、
[平台适配](platform.md)与[内核集成](adoption.md)核对。

Windows 原件为 mingw-w64-x86_64-headers 14.0.0.r598.ge3d18511f-1，Linux 为
libc6-dev 2.39-0ubuntu8.9；完整包／头摘要归正常 sdkHeaders，实际定义归 sdkMacroInputs。
关闭控制器经 stop_thread／GetThreadContext 停止 actor；restart、Sleep／事件等待与
actor gate 保持原生命周期。stack_exception 只发布已识别故障；ERROR_BROKEN_PIPE
区分管道 EOF；WAIT_TIMEOUT 在 Time／port 决定等待结果。Linux park／fault-stop／guard
通过 sigaction 注册，si_addr 仅在匹配信号帧的路径读取。双平台 native、OS、检测器、
port-restart 与 HostBatch 管道消费者验证这些调用；其运行及原始展开收据归同一 PR。

类别风险为拼写、命名和 essential type 判断的阅读歧义。固定 provider、实际 C99
编译／展开、常量值与返回／异常检查承担控制；包、头、目标、选项及使用点变化即重评。

### 精确实例与验收

以下为原标准篇的可消费记录，文件为交付工程路径，行号为物理源码行；每项同时绑定
源码和传递 SDK 定义。R7.2 再要求原 addon 的 unsigned-literal 位置证据。函数宏拼接和
成员访问在通用追踪器中标记复杂展开，本组只核对以下精确已审定义，不放宽通用解析。
R7.1／R10.1／R7.3／R5.5／R5.7 即使 checker 未报告也检查实例，未登记的新使用点继续拒绝。

SDK 使用既有 source-assessment context 的 sdk-integration 组；`sdk_identity(summary, project)`
绑定实际使用 SDK 的 TU 文件闭包、编译参数／宏、完整诊断、头依赖和活动宏集。
完整封存身份仍先验证，生成模板 carrier 的无关字节变化通过实际 TU 输出反映。缺失、pending、过期、表外规则／文件／行、同名项目宏、
未知或改过原头均阻塞。项目偏离与现行正式程序沿 AMD2.5／Compliance:2020 §4 记录，
本篇提供技术理由、批准来源、控制和重评条件；完整审查证据留 PR。221 项原始评估与
整体符合性仍按 ADR 0026 各自保留。

<!-- sdk-integration-records-begin -->
{"definitions":{"CONTEXT_CONTROL":{"CONTEXT_CONTROL":{"line":1795,"value":"(CONTEXT_AMD64 | __MSABI_LONG(0x1))","function":false,"header":"mingw64/include/winnt.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"bc318d60f2c55e8cc39bf2c207fb0fa688ac6c4e5eb8d854d7545ae9802a4321"},"CONTEXT_AMD64":{"line":1793,"value":"0x100000","function":false,"header":"mingw64/include/winnt.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"bc318d60f2c55e8cc39bf2c207fb0fa688ac6c4e5eb8d854d7545ae9802a4321"},"__MSABI_LONG":{"line":219,"value":"(x) x ## l","function":true,"header":"mingw64/include/_mingw_mac.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"a23e0228c8b760492cd7e150221f75d1888fcb59eefaabf71b7296d903533eb1"}},"INFINITE":{"INFINITE":{"line":518,"value":"0xffffffff","function":false,"header":"mingw64/include/winbase.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"20e60d35e414f149ee5790af23d8a5551a2bb692025dd1d42071610dde5a2841"}},"STATUS_ACCESS_VIOLATION":{"STATUS_ACCESS_VIOLATION":{"line":1440,"value":"((DWORD)0xC0000005)","function":false,"header":"mingw64/include/winnt.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"bc318d60f2c55e8cc39bf2c207fb0fa688ac6c4e5eb8d854d7545ae9802a4321"},"EXCEPTION_ACCESS_VIOLATION":{"line":334,"value":"STATUS_ACCESS_VIOLATION","function":false,"header":"mingw64/include/minwinbase.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"fc14c6402971196b5a99b41f692e5df57c51be3d1b2cdd5929fe4fc45e1e96d2"}},"STATUS_STACK_OVERFLOW":{"STATUS_STACK_OVERFLOW":{"line":1459,"value":"((DWORD)0xC00000FD)","function":false,"header":"mingw64/include/winnt.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"bc318d60f2c55e8cc39bf2c207fb0fa688ac6c4e5eb8d854d7545ae9802a4321"},"EXCEPTION_STACK_OVERFLOW":{"line":352,"value":"STATUS_STACK_OVERFLOW","function":false,"header":"mingw64/include/minwinbase.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"fc14c6402971196b5a99b41f692e5df57c51be3d1b2cdd5929fe4fc45e1e96d2"}},"WAIT_TIMEOUT":{"WAIT_TIMEOUT":{"line":356,"value":"__MSABI_LONG(258)","function":false,"header":"mingw64/include/winerror.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"348689c29c78e5645f7801b5ddef38bc0de62052d40d9aaf4e499854a5e0e526"},"__MSABI_LONG":{"line":219,"value":"(x) x ## l","function":true,"header":"mingw64/include/_mingw_mac.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"a23e0228c8b760492cd7e150221f75d1888fcb59eefaabf71b7296d903533eb1"}},"ERROR_BROKEN_PIPE":{"ERROR_BROKEN_PIPE":{"line":248,"value":"__MSABI_LONG(109)","function":false,"header":"mingw64/include/winerror.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"348689c29c78e5645f7801b5ddef38bc0de62052d40d9aaf4e499854a5e0e526"},"__MSABI_LONG":{"line":219,"value":"(x) x ## l","function":true,"header":"mingw64/include/_mingw_mac.h","provider":"mingw-w64-x86_64-headers","version":"14.0.0.r598.ge3d18511f-1","sha256":"a23e0228c8b760492cd7e150221f75d1888fcb59eefaabf71b7296d903533eb1"}},"sa_sigaction":{"sa_sigaction":{"line":40,"value":"__sigaction_handler.sa_sigaction","function":false,"header":"usr/include/x86_64-linux-gnu/bits/sigaction.h","provider":"libc6-dev","version":"2.39-0ubuntu8.9","sha256":"b2f76a26bbfa3668859e8e8a93af9321e14083914df0aa0367cdffaabf4ece81"}},"si_addr":{"si_addr":{"line":138,"value":"_sifields._sigfault.si_addr","function":false,"header":"usr/include/x86_64-linux-gnu/bits/types/siginfo_t.h","provider":"libc6-dev","version":"2.39-0ubuntu8.9","sha256":"88b1392b88af3a08d908866f4e9c5650c5cddb0c8e021b0a3f8293cef6616feb"}},"sigaction":{"header":"usr/include/x86_64-linux-gnu/bits/sigaction.h","provider":"libc6-dev","version":"2.39-0ubuntu8.9","sha256":"b2f76a26bbfa3668859e8e8a93af9321e14083914df0aa0367cdffaabf4ece81"},"O_RDONLY":{"O_RDONLY":{"line":43,"value":"00","function":false,"header":"usr/include/x86_64-linux-gnu/bits/fcntl-linux.h","provider":"libc6-dev","version":"2.39-0ubuntu8.9","sha256":"8d50bc7737bcc76f42b1f406244acb287bcf0463d180b6b0566d27d1f23b3e98"}}},
"sites":[
{"target":"linux-x64-controlled-v1","rule":"7.1","file":"os/src/host/linux/Os_LinuxNative.c","line":180,"symbol":"O_RDONLY","text":"descriptor = open(                 , O_RDONLY);"},
{"target":"linux-x64-controlled-v1","rule":"5.5","file":"os/src/host/linux/Os_HostLinux.c","line":216,"symbol":"sa_sigaction","text":"handler.sa_sigaction = &park_signal;"},
{"target":"linux-x64-controlled-v1","rule":"5.5","file":"os/src/host/linux/Os_HostLinux.c","line":227,"symbol":"sa_sigaction","text":"handler.sa_sigaction = &fault_stop_signal;"},
{"target":"linux-x64-controlled-v1","rule":"5.5","file":"os/src/host/linux/Os_StackLinux.c","line":189,"symbol":"si_addr","text":"uintptr_t address = (uintptr_t)details->si_addr;"},
{"target":"linux-x64-controlled-v1","rule":"5.5","file":"os/src/host/linux/Os_StackLinux.c","line":231,"symbol":"sa_sigaction","text":"handler.sa_sigaction = &guard_signal;"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_HostLinux.c","line":208,"symbol":"sigaction","text":"struct sigaction handler = {0};"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_HostLinux.c","line":222,"symbol":"sigaction","text":"if (sigaction(suspend_signal, &handler, NULL) != 0) {"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_HostLinux.c","line":229,"symbol":"sigaction","text":"return (sigaction(suspend_signal + 1, &handler, NULL) == 0) ? 1 : 0;"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_StackLinux.c","line":222,"symbol":"sigaction","text":"struct sigaction handler = {0};"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_StackLinux.c","line":233,"symbol":"sigaction","text":"if (sigaction(SIGSEGV, &handler, NULL) != 0) {"},
{"target":"linux-x64-controlled-v1","rule":"5.7","file":"os/src/host/linux/Os_StackLinux.c","line":237,"symbol":"sigaction","text":"if (sigaction(SIGBUS, &handler, NULL) != 0) {"},
{"target":"windows-x64-controlled-v1","rule":"10.1","file":"kernel/portable/MSVC-MingW/port.c","line":515,"symbol":"CONTEXT_CONTROL","text":"xContext.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"10.1","file":"os/src/Os_Backend.c","line":1158,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"10.1","file":"os/src/Os_Backend.c","line":1583,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"10.1","file":"os/src/Os_Backend.c","line":1597,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"kernel/portable/MSVC-MingW/port.c","line":54,"symbol":"INFINITE","text":"configASSERT( WaitForSingleObject( xObject, INFINITE ) == 0U );"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1263,"symbol":"INFINITE","text":"Sleep(INFINITE);"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1266,"symbol":"INFINITE","text":"waited = WaitForSingleObjectEx(event, INFINITE, 1);"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1353,"symbol":"INFINITE","text":"Sleep(INFINITE);"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1483,"symbol":"INFINITE","text":"Sleep(INFINITE);"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1523,"symbol":"INFINITE","text":"if (WaitForSingleObject(start->gate, INFINITE) != (DWORD)WAIT_OBJECT_0) {"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Backend.c","line":1539,"symbol":"INFINITE","text":"if (WaitForSingleObject(start->gate, INFINITE) != (DWORD)WAIT_OBJECT_0) {"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Stack.c","line":167,"symbol":"STATUS_ACCESS_VIOLATION","text":"if ((code == EXCEPTION_ACCESS_VIOLATION) &&"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Stack.c","line":171,"symbol":"STATUS_STACK_OVERFLOW","text":"if ((code != EXCEPTION_STACK_OVERFLOW) &&"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Stack.c","line":172,"symbol":"STATUS_ACCESS_VIOLATION","text":"!((code == EXCEPTION_ACCESS_VIOLATION) && (address >= current_stack->reserve_low) &&"},
{"target":"windows-x64-controlled-v1","rule":"7.2","file":"os/src/Os_Stack.c","line":411,"symbol":"INFINITE","text":"Sleep(INFINITE);"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"integration/src/ecu_host_batch.c","line":20,"symbol":"ERROR_BROKEN_PIPE","text":"} else if (GetLastError() == (DWORD)ERROR_BROKEN_PIPE) {"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"kernel/portable/MSVC-MingW/port.c","line":493,"symbol":"WAIT_TIMEOUT","text":"if( xWinApiResult != ( DWORD ) WAIT_TIMEOUT )"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"kernel/portable/MSVC-MingW/port.c","line":515,"symbol":"CONTEXT_CONTROL","text":"xContext.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"os/src/Os_Backend.c","line":1158,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"os/src/Os_Backend.c","line":1583,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"os/src/Os_Backend.c","line":1597,"symbol":"CONTEXT_CONTROL","text":"context.ContextFlags = CONTEXT_CONTROL;"},
{"target":"windows-x64-controlled-v1","rule":"7.3","file":"os/src/Os_Time.c","line":776,"symbol":"WAIT_TIMEOUT","text":"if ((waited == (DWORD)WAIT_TIMEOUT) || (waited == 1u)) {"}
]}
<!-- sdk-integration-records-end -->
<!-- source-assessment-context-begin -->
{
  "id": "sdk-integration-443-declarations",
  "evidence": {
    "head": "c7a047590cd3abef395f1fb8e28de5e8d140e2c3",
    "receipt": "https://github.com/g1331/autsaro/actions/runs/43411175434"
  },
  "cases": [
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "95912048af221849e1ddb15cb8491493a3d1afb0a30e2e1e961100df6248a25b"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "c1aca4aacaa54d5bcd33cec66f9a30a46c057d7d2925c0bdbed2c52fdbdd334c"
    },
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "b3484f8f3679d7d89c4d8e5951098afba23db133b3bb4173251c3b5da8b59bb8"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "c3402183e537fec251fb202aadf6772dd569c4d824dce113a6d3897fc6270427"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "9e7c2c93a38c1e2a662539b65157395efb9a44ec5ec8a0a4308c9995d0ab117e"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "c226e0f87abfcabe666640af6b9ab4de2586666f4abfcc25cf6406c00b2bf8f7"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "bd36b992849c24507e495a9cbd780f9b4eff98206cb559d396a94358e72952ab"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "51d1c1f909359c1ae541bed6e53864789e8503c5b55022fd0d9ed35889768646"
    }
  ],
  "records": {
    "kind": "sdk-integration"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
