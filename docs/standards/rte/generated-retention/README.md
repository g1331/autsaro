# 生成接口的精确有界保留

项目选择沿用[2026-10-30 的具体决定](https://github.com/g1331/autsaro/pull/393#issuecomment-6516977768)，不从 #381 的 124 键或八个地址实例外推。原决定绑定 head `7ee1d773bf7bcdc0edc7f6423aeb8ae12236f24b` 的完整 PR 正文 UTF-8 SHA256 `8644a6398d52ee8bd3af8aeea97a66d627bb041762bb3345ebcc8fdc9b7d93ae`。历史输入和原始结果保留，当前适用范围由下文的输入绑定单独限定。

本组只含146个R2.5、4个R2.3、8个R8.7、1个R11.4，共159个实体/body键；按profile为153/153/156/156。S=standard-ecu，U=user-application，C=component-pipeline，N=no-diagnostic；target Both仅linux-x64-controlled-v1/windows-x64-controlled-v1，程序Both仅host-batch/probe，语言C99。每行profiles表示其实际报告范围，不从一个程序或target推导另一个。标记不是范围外的批准；名字、body、表示、底层类型、集合、目标或包含/程序闭包变化均须重评。

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

五个Rte_Inst指针仍为不可变、静态CDS的外部句柄；[单实例句柄的实际用途](../README.md#单实例句柄的实际用途)所述未使用状态保持。新增C/C++回归单独读取句柄以验证ABI，这些测试消费者不属于host-batch/probe交付程序，不能消除R2.8的真实未使用状态。三个SchM模式函数保留既有C链接、模式存储/严格独占区及其对应消费者和生命周期控制。内部化、inline或新handle接口均可行，本次选择保留现有公开合同。R8.7接受不推定R2.8已满足，也不批准本组之外的规则；R2.8仍由人工评估记录具体未满足对象和退出条件。

R11.4只限Os_ArtiAlarmBaseReturn的native_address初始化。GetAlarmBase的真实有效对象地址经此转换进入Arti_CaptureAddress，再复制到事件event_address/address_valid；trace只取其低32位。两个受控x64 ABI下指针和uint64为64位，真实高位对象观察及高位截断反例约束完整位保留。没有整数到指针恢复、解引用、伪造所有权或延长pointee存储期，也不扩大现有事件观察生命周期。单处转换仍是实现相关的真实违反；新handle协议是可行替代。源码标记绑定局部native_address，不新增通用地址类别。

源码只补精确说明，不更改非注释C/预处理token或宏种类。模板静态实体由现有双向关联检查核对，emitter输出由正常native套件对本表完整shape、闭合集合及来源核对。集合摘要只冻结这次决定的实体/body/profile证据，不是诊断baseline或suppression；当前raw逐项核对及零未分类结论归实施PR，不能只改文档状态即宣称完成。

## 当前输入绑定

正常源码检查按[项目源码验收格式](../../../development/testing/source-acceptance.md)消费本组冻结上下文。当前生成物、C/C++ 消费者、原实体与 body、诊断位置和程序归属核对后，本组按 §6.2.3 记录安排参与局部项目验收；不将旧批准自动移植到新输入。`formalStatus=approved` 在本组表示该 Advisory 记录安排已完成，不是 Required 正式偏离。应用、配置、producer、交付、包含或程序闭包及评估正文变化均使旧绑定失效，`pending` 不放行。其他人工记录、Required 义务、SDK 和采用代码不获授权，原始分析结果与整体未完成状态不变。
<!-- source-assessment-context-begin -->
{
  "id": "generated-retention-449-inputs",
  "evidence": {
    "head": "5bfab91c06c3d4d9ae08fc1fa51d9b5563cebefb",
    "receipt": "https://github.com/g1331/autsaro/pull/449#issuecomment-6710885804"
  },
  "cases": [
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "c52c6c7c53f862e5e187578f407d4e9c1da3a1125c8a365825cd3bf8838f7044"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "7b6792af290737cca41a2a9986e0622eecf19ad001ea068f51665392cb6d731a"
    },
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "e3f7018c286fdd05d6a9e6e67bd2c51c75d32178ec6a1ff16e8095ae2b44d305"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "b59381fd2370eb7079c14d088f01fe677e35028aa4111997403c1504a57c2660"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "e5a91a998122966c4119b1ec4588f8ef410a1ce60476be13547593fc7144f1b9"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "1336b609125c347ac0cad3103cc50564c3cf3596f776f08860d8f4293cc06a2f"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "4689fdfbf311889e5558cd00bdafd0ba67b09593d9ab28590d191e684ca53e4c"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "a04b13596e8d08980c83193de7973b6397579e07d1f4345e1c16e836d4920ef9"
    }
  ],
  "records": {
    "kind": "generated-retention",
    "cohortSha256": "10bcb694c9a0a2424b33513927e1e9f99251a0ab60e09b4af15329497cc843a5"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
