Type: research
Status: resolved

## 问题

AUTOSAR Adaptive Platform 的官方规范及机器可读资料有多完整、容易获取？独立开源的 AP 生成产品还依赖哪些主要运行环境和平台能力？与 Classic 的架构及工作量作高层比较，只判断是否值得将 AP 纳入后续产品决策，不立即承诺实施。

## 答案

官方 R25-11 Adaptive 文档、共享 XSD 和部分蓝图/机器配置 ARXML 可下载，但它们不能直接充当完整实现或可运行项目。AP 涉及 POSIX/C++ 运行环境、ARA 功能集群、服务通信、Manifest、部署与进程管理；不能把 Classic 的代码生成器增加几个模板就视为支持 AP。现阶段保持 CP 优先，不实施 AP。待 CP 产品和需求明确、目标 AP 版本及运行环境确定，并完成一条模型到部署运行的可验证闭环后，再单独决定 AP 产品范围。公开可下载资料的开源使用边界仍须调查。

证据与未定项见[研究记录](../research/adaptive-feasibility.md)。
