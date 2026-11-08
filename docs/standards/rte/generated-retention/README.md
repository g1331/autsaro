# 生成接口的精确有界保留

历史依据为[2026-10-30 的原 159 键具体决定](https://github.com/g1331/autsaro/pull/393#issuecomment-6516977768)，不从 #381 的 124 键或八个地址实例外推。原决定绑定 head `7ee1d773bf7bcdc0edc7f6423aeb8ae12236f24b` 的完整 PR 正文 UTF-8 SHA256 `8644a6398d52ee8bd3af8aeea97a66d627bb041762bb3345ebcc8fdc9b7d93ae`。历史输入和原始结果保留，当前适用范围由下文的输入绑定单独限定。

2026-11-08，编排者决定按实际生成实体重新评估活动集合：[PR #465](https://github.com/g1331/autsaro/pull/465) 将实例句柄改为共享 CDS 的取址宏，EchoApplication、DcmService、Ingress、Process、Observe 的五个独立指针对象已停止生成，删除对应 R8.7 活动记录。原 159 键决定保留为历史依据；本次不增加退役格式。

同日，编排者决定保留共享 CDS 的取址宏表示，并将其真实未使用的 R2.5 按项目选择的 Advisory 保留登记。四份实际样本共有 EchoApplication、DcmService、Ingress、Process、Observe、ComMService 六个具体宏；新增六条记录，producer 均为 `contracts::contract_headers`。先删除五条旧对象记录的 154 键中间集合保留为本次重评依据。

活动集合含 152 个 R2.5、4 个 R2.3、3 个 R8.7、1 个 R11.4，共 160 个实体/body 键；按 profile 为 154／154／157／157。S=standard-ecu，U=user-application，C=component-pipeline，N=no-diagnostic；target Both仅linux-x64-controlled-v1/windows-x64-controlled-v1，程序Both仅host-batch/probe，语言C99。每行profiles表示其实际报告范围，不从一个程序或target推导另一个。标记不是范围外的批准；名字、body、表示、底层类型、集合、目标或包含/程序闭包变化均须重评。

- [状态、Counter与别名](preprocessing.md)
- [Dcm操作状态和协议](dcm-protocol.md)
- [Dcm响应常量](dcm-responses.md)
- [Dcm模式和类型](dcm-modes.md)
- [实例、模式和单处地址观察](exports.md)

## 有效规则与允许条件

七份合法MISRA原件SHA256SUMS已重新匹配。基线仍为C:2012 Third Edition + AMD1–4/TC1–2，C99。R2.3/R2.5为Advisory、Decidable/System（主文印刷43/44，PDF51/52）；R8.7为Advisory、Decidable/System（印刷70/PDF78，TC2.15印刷6/PDF12）；R11.4为Advisory、Decidable/Single Translation Unit（印刷97/PDF105）。修订未为本组提供“公共接口即可忽略”的规则例外，未采用新的规则重分类。R11.4的整数null-pointer-constant到对象指针例外不适用于此处对象指针到整数；uintptr_t也不自动满足本规则。没有Mandatory偏离。

本组保留真实 unused、单 TU 引用和指针整数转换，不将其判为误报或 AUTOSAR 不可替代。主文 §6.2.3（印刷 13／PDF 21）允许对 Advisory 使用非符合项记录安排，无须正式偏离；[ADR 0025](../../../adr/0025-misra-layered-scope.md#grp指南重分类计划) 未提高这四条规则的类别。下列实体表、理由、替代与风险控制构成本组的记录，不改变规则类别或声明原构造符合规则。Required 的正式义务沿 AMD2.5 修订后 §6.2.2 所引的 MISRA Compliance:2020 第 4 节，Mandatory 的禁止偏离要求保持；本组处置仅限上述 Advisory 记录。

## 理由、替代与控制

固定宏和四个休眠mode typedef保持当前完整头文件/预处理合同；换enum、const、裁剪发布集合或改宏种类均是技术可行替代，但会改变#if能力、表示或调用方式。本次不把常量/类型发布解释为新增未实现服务能力。形态和完整数值/宽度由实际C/C++消费者及集合/body/种类反例约束，不制造生产使用以销掉告警。

实例句柄由应用头中的 `Rte_Inst_<cts>` 宏取得唯一共享 const CDS 的地址；实际存储和消费者见[单实例句柄的实际用途](../README.md#单实例句柄的实际用途)。SWS_Rte_03793／01012／01148／06810／03711／03712／03715 约束名称、实例身份、指针类型及 const CDS；宏形式是本项目的表示选择。R2.5 为 Advisory，按 §6.2.3 记录真实未使用的六个宏，授权为编排者 2026-11-08 的上述决定，不归入标准强制宏类别。

选择宏的收益是不分配独立指针存储、提供地址常量表达式并支持 C99 静态初始化；宏本身不增加独立对象的 R8.7／R2.8 负担，共享 CDS 的对象义务分别评估。可行替代是在单个 TU 定义同名 const 指针并在头中声明，但会增加每实例一个指针宽度的存储及其 R8.7／R2.8 评估，并失去地址常量表达式能力；头内 static const 指针还会产生每 TU 存储和 R5.9 评估。风险是公开宏未被当前程序展开、名字或替换体遭误改；既有 C99／C++11 消费者验证类型、静态初始化和跨 TU 地址身份，精确集合／shape／标记检查及删除记录变异控制保留范围。名字、替换体、表示、profile、target、程序或包含闭包变化时重评；原始 R2.5 诊断保留，当前适用输入见下节绑定。

原五个独立指针对象的 R8.7 活动记录已移除。三个 SchM 模式函数保留既有 C 链接、模式存储／严格独占区及其对应消费者和生命周期控制。内部化、inline 或新 handle 接口均可行，本次选择保留这些现有公开合同。R8.7 的本组记录仅适用于表中剩余三个函数。

R11.4只限Os_ArtiAlarmBaseReturn的native_address初始化。GetAlarmBase的真实有效对象地址经此转换进入Arti_CaptureAddress，再复制到事件event_address/address_valid；trace只取其低32位。两个受控x64 ABI下指针和uint64为64位，真实高位对象观察及高位截断反例约束完整位保留。没有整数到指针恢复、解引用、伪造所有权或延长pointee存储期，也不扩大现有事件观察生命周期。单处转换仍是实现相关的真实违反；新handle协议是可行替代。源码标记绑定局部native_address，不新增通用地址类别。

本次集合变更对应 PR #465 的句柄表示与 Advisory 项目选择。模板静态实体由现有双向关联检查核对，emitter输出由正常native套件对本表完整shape、闭合集合及来源核对。集合摘要只冻结这次决定的实体/body/profile证据，不是诊断baseline或suppression；当前raw逐项核对及零未分类结论归实施PR，不能只改文档状态即宣称完成。

## 当前输入绑定

正常源码检查按[项目源码验收格式](../../../development/testing/source-acceptance.md)消费本组上下文。
`evidence_identity(summary, project, "generated-retention")` 绑定当前活动集合的交付文件、
相关实际 TU／传递头、ownership／原件、编译参数／宏及完整混合诊断；完整副本先经
原身份验证。集合、表示、相关字节及评估正文变化时重新评估；闭包外的模板 carrier
变化可复用证据。当前输入从本 PR 的双平台工件重算，原输入与收据保留在 Git 历史中。
原始分析结果与整体未完成状态保持。
<!-- source-assessment-context-begin -->
{
  "id": "generated-retention-458-inputs",
  "evidence": {
    "head": "182f8df819760e0ea027766f19a362436c64df3e",
    "receipt": "https://github.com/g1331/autsaro/actions/runs/42479363863"
  },
  "cases": [
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "65c9f66f3025a31685eb5641a954953abfc83167ea2e1549cc34a1866f53cbbb"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "6eb304bd97868f0e92ff50ca79b7eca19427e8acb88c75ea6b1cfa8558dd1a60"
    },
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "f27a7e6ac891a05da55a45aae9b8031f0ceaad0a5a9510832aeaf6a60345a236"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "4ce69c047e22a97616ca87dd66ff1de9f433465a1c2fce1c07839b2958221291"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "465a024ca36b7e3833f237ea5fe344822ddece841567adcbda438caf133eded8"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "934850164a933835ecbe150564a5e43e8f76568ce6d42f778bfb62208d74fe10"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "f74fb588d71beddd4b42aa9d157949d9f3a4e9027fce595e12bc2d7d2e0cdcf5"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "2f5aeaceb24078349ae7140c673482adce50348f9e0fc9b9cc0dd7dea6db46f0"
    }
  ],
  "records": {
    "kind": "generated-retention",
    "cohortSha256": "f300270c060c15fe7c9d449e1e38a6a7994c02b12280578acc8cd8956e500224"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
