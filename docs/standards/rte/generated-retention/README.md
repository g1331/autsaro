# 生成接口的精确有界保留

项目选择来自[PM的2026-10-30具体决定](https://github.com/g1331/autsaro/pull/393#issuecomment-6516977768)，不是Boss直接逐键批准，也不从#381的124键或八个地址实例外推。绑定head `7ee1d773bf7bcdc0edc7f6423aeb8ae12236f24b` 的完整PR正文UTF-8 SHA256 `8644a6398d52ee8bd3af8aeea97a66d627bb041762bb3345ebcc8fdc9b7d93ae`。旧v3/v5输入、物理位置、raw及严格比较refusal分别保留。

本组只含146个R2.5、4个R2.3、8个R8.7、1个R11.4，共159个实体/body键；按profile为153/153/156/156。S=standard-ecu，U=user-application，C=component-pipeline，N=no-diagnostic；target Both仅linux-x64-controlled-v1/windows-x64-controlled-v1，程序Both仅host-batch/probe，语言C99。每行profiles表示其实际报告范围，不从一个程序或target推导另一个。标记不是范围外的批准；名字、body、表示、底层类型、集合、目标或包含/程序闭包变化均须重评。

- [状态、Counter与别名](preprocessing.md)
- [Dcm操作状态和协议](dcm-protocol.md)
- [Dcm响应常量](dcm-responses.md)
- [Dcm模式和类型](dcm-modes.md)
- [实例、模式和单处地址观察](exports.md)

## 有效规则与允许条件

七份合法MISRA原件SHA256SUMS已重新匹配。基线仍为C:2012 Third Edition + AMD1–4/TC1–2，C99。R2.3/R2.5为Advisory、Decidable/System（主文印刷43/44，PDF51/52）；R8.7为Advisory、Decidable/System（印刷70/PDF78，TC2.15印刷6/PDF12）；R11.4为Advisory、Decidable/Single Translation Unit（印刷97/PDF105）。修订未为本组提供“公共接口即可忽略”的规则例外，未采用新的规则重分类。R11.4的整数null-pointer-constant到对象指针例外不适用于此处对象指针到整数；uintptr_t也不自动满足本规则。没有Mandatory偏离。

项目选择保留真实unused、单TU引用和指针整数转换，未将它们判为误报或AUTOSAR不可替代。主文§5.4/§6.2.2的正式记录和审阅义务不由项目风险接受替代；本组不是合法MISRA偏离或符合性声明，221项not_assessed、passed=false、error及comparison refusal各自保持。

## 理由、替代与控制

固定宏和四个休眠mode typedef保持当前完整头文件/预处理合同；换enum、const、裁剪发布集合或改宏种类均是技术可行替代，但会改变#if能力、表示或调用方式。本次不把常量/类型发布解释为新增未实现服务能力。形态和完整数值/宽度由实际C/C++消费者及集合/body/种类反例约束，不制造生产使用以销掉告警。

五个Rte_Inst指针仍为不可变、静态CDS的外部句柄；[单实例句柄的实际用途](../README.md#单实例句柄的实际用途)所述未使用状态保持。新增C/C++回归单独读取句柄以验证ABI，这些测试消费者不属于host-batch/probe交付程序，不能消除R2.8的真实未使用状态。三个SchM模式函数保留既有C链接、模式存储/严格独占区及其对应消费者和生命周期控制。内部化、inline或新handle接口均可行，本次选择保留现有公开合同。R8.7接受不推定R2.8已满足，也不批准本组之外的规则；R2.8仍由人工评估记录具体未满足对象和退出条件。

R11.4只限Os_ArtiAlarmBaseReturn的native_address初始化。GetAlarmBase的真实有效对象地址经此转换进入Arti_CaptureAddress，再复制到事件event_address/address_valid；trace只取其低32位。两个受控x64 ABI下指针和uint64为64位，真实高位对象观察及高位截断反例约束完整位保留。没有整数到指针恢复、解引用、伪造所有权或延长pointee存储期，也不扩大现有事件观察生命周期。单处转换仍是实现相关的真实违反；新handle协议是可行替代。源码标记绑定局部native_address，不新增通用地址类别。

源码只补精确说明，不更改非注释C/预处理token或宏种类。模板静态实体由现有双向关联检查核对，emitter输出由正常native套件对本表完整shape、闭合集合及来源核对。集合摘要只冻结这次决定的实体/body/profile证据，不是诊断baseline或suppression；当前raw逐项核对及零未分类结论归实施PR，不能只改文档状态即宣称完成。

## 当前输入绑定

正常源码检查按[项目源码验收格式](../../../development/testing/source-acceptance.md)消费本组冻结上下文。输入覆盖应用字节、实际配置、producer、交付与包含／程序闭包及评估材料，不以目录名认定 S/U/C/N。上下文只关联本组原表，不授权其他人工记录；缺失、变化或 `formalStatus=pending` 均保持阻塞。本组正式记录义务尚未因 #426 的形态关联而完成，不能将采集到的当前输入摘要自动升级为批准。
<!-- source-assessment-context-begin -->
{
  "id": "generated-retention-428-inputs",
  "evidence": {
    "head": "425c126bb67855fe58a5ee87fd66cfeb3a245103",
    "receipt": "https://github.com/g1331/autsaro/pull/428#issuecomment-6682908054"
  },
  "cases": [
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "9a49a53d4d3d5cda22369d84bbc5399812fad075511810477cec89483a0cbf11"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "0e5cbb65e76e56383c1fae6e2841584db0e3061c27e6616426680bcc65852228"
    },
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "d5b4366d12638b4e85930a25301216087644f2bdf5a405f2aee3be571be370e0"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "68929e641d0146dbd96d9ddda5722b9570edace831aa769e663b8353f262b1af"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "ab97dddadeb63de82f793e82ab15a084bbdc5c86fc0a2eb8242d0b3df10bacd1"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "cd5b994382b97b616287410f07cac4489b4faec62e17835c922af6298bf4fdf4"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "0632d7e5a0a57bf275e6104691af04ccbe636c8403895d976eb2e72f719ca819"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "06a7a868ecfc5d6bc5d341a17151a3715f6d1a0863bd8d5534727ec436151b55"
    }
  ],
  "records": {
    "kind": "generated-retention",
    "cohortSha256": "10bcb694c9a0a2424b33513927e1e9f99251a0ab60e09b4af15329497cc843a5"
  },
  "formalStatus": "pending"
}
<!-- source-assessment-context-end -->
