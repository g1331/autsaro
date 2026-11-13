# 声明与初始化的检查边界

输入、固定工具、来源责任和待完成项的解释沿[语言与词法](lexical.md)。
同号入口以解析出的函数、变量、类型、scope 和值流为输入；CTU 必须按两个程序分别收齐。
既有[声明用途](../declarations.md)、[公共类型](../public-types.md)与
[初始化层级](../../bsw-general/language.md#初始化层级与实际求值)保留各自实体证据。
本表逐项明确恢复条件，尚未完成的实际核查由 #443 承担。

R8.1 已重新阅读主文印刷 63–64／PDF 71–72 的正文及适用性，明确仅适用 C90。
AMD2.27 的语言扩展表没有为 R8.1 追加 C11；其余 AMD1／AMD3／AMD4／TC1／TC2
检索未发现修改该语言范围的条目。所选 CI 两目标实际编译数据库使用 C99，
因此该条语言适用性成立。C99 的声明约束继续由编译器、R1.1、R8.2／R8.4 承担。
该行仍须本组当前绑定；本篇其余行保留适用修订合成和当前源码复核的具体缺口。

<!-- misra-checker-obligations-begin -->
| 规则 | 类别 | 评估范围 | 状态 | 结论 |
| --- | --- | --- | --- | --- |
| R8.1 | Required | 双目标四 profile 的 C99 HostBatch／probe | not_applicable | 主文 63–64／PDF 71–72 仅适用 C90；AMD2.27 未扩展本条，AMD1／3／4 与 TC1／2 未改变语言范围。当前实际 TU 使用 -std=c99；同号 addon 的 implicit-int 诊断仍保留，C99 声明和扩展义务继续按各自规则核查。 |
| R8.2 | Required | 公开及内部函数类型、原型与定义 | not_assessed | 同号入口结合 semantic／raw token 核查 void 和参数名字，存在 Det 函数指针 typedef 误报的既有实例。#443 须按真实 typedef／回调／生成角色逐类型复核，并合成 TC2 对非原型、变参声明的修订；SDK 声明接缝归 #444。 |
| R8.3 | Required | 同一对象／函数的全部声明 | not_assessed | 核心 funcArgNamesDifferent 只提供参数名不一致候选。#443 须同时比较跨 TU 的兼容类型、限定符和命名，核查条件接口、实际头 owner 与调用者；尚缺当前完整声明集合及适用修订合成。 |
| R8.4 | Required | 外部链接定义处可见的兼容声明 | not_assessed | 同号入口按函数 token／tokenDef 和全局变量初始化形态判断，跳过 main。#443 须逐定义核对实际 include 顺序、兼容声明及 TC1 的 main 例外；生成桥和补丁的可见性证据须回接当前 producer。 |
| R8.5 | Required | 外部对象／函数声明的头文件 owner | not_assessed | 同号入口保存 identifier CTU 摘要；#443 尚缺头内声明唯一来源、同头重复包含与实际 include 闭包的核对，须合成适用修订，保留显式标准接口声明与实现的关联。 |
| R8.6 | Required | 每个程序的外部定义唯一性 | not_assessed | 同号入口保存 identifier CTU 摘要。实际编译／链接可发现部分重复定义，tentative、inline、条件编译和独立程序的合法重复须分别核定；#443 尚缺当前全部定义集合及 AMD4 对 R8.15 的交叉关系核查。 |
| R8.7 | Advisory | 外部链接实体的跨 TU 真实引用 | not_assessed | 同号入口保存使用摘要，固定 CTU 去重可能漏掉单 TU 已声明的实体。#443 须核对公开句柄、生成桥与直接消费者，连接[链接可见性](../declarations.md#链接可见性与实际引用)；#465 已交付 #195 的 ComM 用户服务端口；本组仍须核对当前生成导出与跨 TU 直接消费者，结算归 #443。 |
| R8.9 | Advisory | 只在单个函数出现的对象标识符 | not_assessed | 同号入口统计各函数体使用的全局 static 对象，范围小于 AMD4 修改后的声明义务。#443 须按声明而非定义核对 TLS、跨回调状态和段归属，接续[单函数对象](../declarations.md#单函数对象与存储期)，并检查全部当前对象。 |
| R8.10 | Required | inline 函数的存储类 | not_assessed | 同号入口检查 isInlineKeyword 且非 static 的函数，足以定位已正确建模的声明。#443 尚缺宏形式 inline、GNU 拼写及函数全部声明在当前 TU 中的模型对应核查和适用修订合成；语言标签本身不完成该核查。 |
| R8.11 | Advisory | 外部链接数组声明的显式大小 | not_assessed | 同号入口只检查 isExtern 且全局 name 后为 [] 的变量。#443 须核对 typedef 数组、不同声明拼写与全部公开配置数组，确认大小契约和直接索引消费者，并合成适用修订。 |
| R8.12 | Required | 枚举列表的隐式值唯一性 | not_assessed | 同号入口比较已有 valueType／values 的枚举 token 与隐式值；未知值流会留下缺口。#443 须核对宏、目标宽度和隐式自增后的实际值，保留显式同值例外及适用修订依据。 |
| R8.13 | Advisory | 指针所指对象的可修改性 | not_assessed | 核心 constPointer 给出可加 const 的候选；别名、回调与 SDK 输出约定仍需跨调用追踪。#443 尚缺当前实参到最终写入者的闭包和适用修订合成，避免通过 cast 或错误 const 声明改变真实接口。 |
| R8.14 | Required | 原声明及实际宏展开的 restrict | not_assessed | 同号入口只查 rawTokens 的 restrict 字面 token。#443 须核对 typedef、限定符宏及项目实际声明／展开；SDK 原头的采用责任和头外自有用途沿 #444 分开，适用修订仍待合成。 |
| R9.1 | Mandatory | 自动对象读取之前的赋值路径 | not_assessed | 核心 uninitvar 依赖值流及库输出模型。#443 尚缺每条实际失败返回、部分输出、别名与回调路径的初始化证据，并核对 AMD4 将 _Atomic 交给 R9.7 的范围；Mandatory 保持逐路径整改责任。 |
| R9.2 | Required | aggregate／union 初始值的层级 | not_assessed | 同号入口调用固定 misra_9 的 902 模型，以 ElementDef 跟踪层级和子对象。#443 须核对真实宏展开、括号层级、字符串及零初始化例外，回接既有初始化证据，并合成 AMD4 chained-designator 交叉约束。 |
| R9.3 | Required | 数组的显式元素初始化与例外 | not_assessed | 同号入口调用 misra_9 的 903 模型；#443 尚缺当前全部数组、稀疏 designated／{0}／字符串例外与消费者有效索引的核查，既有 priorities 的有界证据保持自身输入范围，适用修订待合成。 |
| R9.4 | Required | 每个子对象的重复初始化 | not_assessed | 同号入口调用 misra_9 的 904 模型及 numInits 计数。AMD4.47–50 要求追踪指定后续的位置式覆盖；#443 尚缺真实展开下索引、成员和回退后覆盖的逐对象复核及模型反例，名称去重不足以完成。 |
| R9.5 | Required | 有 designated initializer 数组的显式大小 | not_assessed | 同号入口调用 misra_9 的 905 模型，另传 raw token；#443 须按 AMD2 对显式大小义务的修改核对数组声明、typedef、链式指定及最大索引，回接实际配置长度和直接消费者。 |
<!-- misra-checker-obligations-end -->

## 当前输入重评

第三阶段第一批沿 master `40665cd7` 的正常
[run 43410036518](https://github.com/g1331/autsaro/actions/runs/43410036518) 八份完整工件重评。
按两目标 Git 换行分别核验 seal、最终 kernel、92 项 producer、12 个驱动以及
两个程序的编译数据库／addon 回执和 GCC／Cppcheck 实际输入闭包；888 个程序 TU
实例均为 C99。`checker-obligations` 沿完整 `input_identity` 绑定全部当前评估正文。
本批的具体方法、有效修订与逐项结论见 [lexical.md](lexical.md)；当前 144 项为
10 项 satisfied、3 项 not_applicable、131 项 not_assessed。仍阻塞项逐项保留 #443
责任；R7.1 的 O_RDONLY→00 未新增偏离，R7.3 连接已有 SDK 类别及原正式义务。

本次只更新本组原上下文。FreeRTOS、owned-source、generated-retention、SDK 四组的
实际局部证据摘要在正文修改前后保持，已有具体诊断消费及其他组状态保持。
<!-- source-assessment-context-begin -->
{
  "id": "checker-obligations-443-lexical",
  "evidence": {
    "head": "fd217a143785cf634807cb66761751fcabe23704",
    "receipt": "https://github.com/g1331/autsaro/issues/443#issuecomment-6850947544"
  },
  "cases": [
    {
      "caseId": "component-pipeline",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "af3e6de0bd5b16a11a5bc623aca399ab8b0d92805cf48d3d57d126a7dd1124a9"
    },
    {
      "caseId": "no-diagnostic",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "cb413519a0a4abefade39e0612a44ff05e793048eefec480882add7cd8fa204e"
    },
    {
      "caseId": "standard-ecu",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "dbc87fdfbfcc708dbf3a648e0d45e8fa46057e2c088c3060b5fa1f6b9bc77299"
    },
    {
      "caseId": "user-application",
      "target": "linux-x64-controlled-v1",
      "inputIdentity": "4627f7bf043ac79fd7d73f04427f3e1f007fee5824dbc9f36c87bfd12707a468"
    },
    {
      "caseId": "component-pipeline",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "bfa6acf7665c560a7e3700b2c63ed1a3b1ae9ecaedf6423a023849453fd94d37"
    },
    {
      "caseId": "no-diagnostic",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "4df4f5792630a509bb00da7c6ca11d6cfbb9b8bcf80efac870f6fb468ddc63c2"
    },
    {
      "caseId": "standard-ecu",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "3d46e621d6a2a051ff5e1957517787f0f240a35f24c02dc02ca5c7f11723be8a"
    },
    {
      "caseId": "user-application",
      "target": "windows-x64-controlled-v1",
      "inputIdentity": "54284a4d068f83d9cd723e783148721ecb0c2932b27705c80a4b6c26bee4ae66"
    }
  ],
  "records": {
    "kind": "checker-obligations"
  },
  "formalStatus": "approved"
}
<!-- source-assessment-context-end -->
