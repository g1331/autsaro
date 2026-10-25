# 0024. 受控 Linux 平台三处标准头违反的有界保留

## 决定与来源

**项目取舍已接受（2026-10-25）；三处技术内容待逐处独立复核。** 人类维护者对[完整提案](https://github.com/g1331/autsaro/pull/371#issuecomment-6391387787)明确回复“接受”，经项目秘书转达；本记录依据项目内的[PR 决定记录](https://github.com/g1331/autsaro/pull/371#issuecomment-6392165780)和[Issue 决定记录](https://github.com/g1331/autsaro/issues/58#issuecomment-6392165958)。接受者是人类维护者，转达和实现不产生新的批准人。

本决定独立于 [ADR 0023](0023-autosar-required-conflict-disposition.md)。相关 SWS 要求调度上下文、栈故障处理和软件 Counter 行为，没有指定这三个 POSIX 头；不能把平台实现选择写成 AUTOSAR 不可替代的强制选择。

## 严格范围

仅限 `linux-x64-controlled-v1` 当前 C99／glibc-pthread x86-64 实现的三个显式包含位置，以下行号固定于 `ab2ea4591d37bdcba5bec4d8223853dfa7bdb48c`：

- `runtime/os/src/host/linux/Os_HostLinux.c:10` 的 `<signal.h>`，MISRA R21.5。
- 同文件第 24 行的 `<time.h>`，MISRA R21.10。
- `runtime/os/src/host/linux/Os_StackLinux.c:8` 的 `<signal.h>`，MISRA R21.5。

覆盖现有 `standard-ecu`、`user-application`、`component-pipeline`、`no-diagnostic` 四个 profile 的 HostBatch／probe，以及复用同一实现的原生验证构建。保留既有公共 ABI、线程／物理栈／ARTI 身份、异步信号、真实 guard fault、备用栈、线程 mask／pending SIGPIPE、单调等待和超时合同。provider、ABI、flags、handler 或相关合同变化必须重新评估。

R21.5／R21.10 仍是真实违反；`Os_HostLinux.h → pthread.h → time.h` 等 SDK 间接链及其他根路径保持可见、分别核查。不得搬移包含或声明、复制 SDK 类型、硬编码 SDK 值、重标采用代码或隐藏诊断来消除记录。本决定不扩展到其他文件、规则、平台或新增设施，Mandatory 和其他义务不变。

## 项目取舍与技术复核分开

本决定允许保留这些技术违反，不是 MISRA 合法偏离或符合性批准，不解除 `SWS_BSW_00115`，也不自动完成 #58／#55／#56。全部原始 `passed`、`error`、`not_assessed` 和严格比较拒绝保持。

三处技术复核当前均为 **pending**：须分别核对固定 MISRA 修订原文、实际包含与调用闭包、SWS 所要求的行为、替代排除、风险／控制和本决定的边界。实现者自审不代替独立复核；发现须修复或以依据答复，变化只作必要的聚焦复审。

复核和发现处置完成前，不填三处技术批准表、不落 MR12。完成后沿现有 OS 模块记录和就近注释路径连接本决定来源、评估、复核 head／发现／处置与实际文件／规则／include；记录须明确保留“项目平台保留，非 MISRA 符合性批准”。必要的 include 精确关联只扩展现有 `c_standard` 及其正负例，不伪装函数、不造新宏／台账、不修改 Cppcheck、诊断或基线。

## 证据与剩余责任

逐处调用链、固定 SWS／MISRA 定位、被排除的替代、风险控制、候选技术理由和后续正常验证集中在[原 PR 提案](https://github.com/g1331/autsaro/pull/371#issuecomment-6391387787)；SDK 与实际链接 provider 身份的证据边界见[只读核查](https://github.com/g1331/autsaro/pull/371#issuecomment-6391241519)。本决定不把已有测试、声明、符号或绿色 CI 当成这些剩余义务的证明。

本决定只接受上述项目取舍，不授权其他实验或监视。未提交的 HostBatch 字符串 fixture 增量及受限反例不属于本决定的实现或复核范围。
