# Epic 3 Context: 交接可重建且可离线复验的受限主机工程

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

使 ECU 集成工程师能把已保存的 R24-11 受限主机配置、生成的 C99 工程和可复验的结果交给另一位工程师。接收者能在新目录用明示的工具重新导入、生成、构建，并在固定参考双 ECU 场景中独立检查 CAN 与适用诊断行为。它是完整 Classic 参考 ECU 前的主机交付成果，不声称完整 ECU Extract、SWC/RTE、AUTOSAR OS、MCU 或模块符合性。

## Stories

- Story 3.1: Package saved host ARXML inputs with generated ECU
- Story 3.2: Reopen delivered inputs and reproduce host C99 project
- Story 3.3: Verify delivered host behavior without the workbench

## Requirements & Constraints

- 仅对已保存、仍与磁盘一致且经固定主机剖面校验的 R24-11 多文件 ARXML 声称“可重建交付”。源文件、逻辑身份、版次/工具/Windows 目标信息和完整性记录须随包可见；未保存、外部改动、缺失、断开的有效引用及影响生成的未知变体必须拒绝。
- 保持安全保存、生成预览、确定性 C99 产物和旧输出保护。失败不覆盖用户源文件或旧交付；路径冲突、链接或越界不得写入包外。工具生成的路径映射不泄露原机器绝对路径，不额外打包密钥及运行状态；原始 ARXML 内容由交付者审阅。
- 区分“源输入可重建”“工程已构建”“固定主机行为已验证”与“完整标准/实机支持”。只有独立参考向量运行通过才记录相应主机行为结果；不得自动升级能力档案。
- 重新导入需要同版工作台以及另行合法取得的 R24-11 XSD；当前工作台从源码目录运行，尚无独立安装包。生成的 C99 工程和固定离线复验不依赖工作台；后者的外部工具限于明示的 Windows、PowerShell 和 MinGW GCC。

## Technical Decisions

- 新增明确的“导出可重建主机交付包”操作，区别于普通“生成主机工程”。核心交付函数负责校验和组包，桌面后端与界面只调用该入口，不自行拼装文件。
- 源 ARXML 按原文字节交付，并用安全的包内相对路径和逻辑映射保留跨文件身份；输入快照、映射和交接说明进入现有文件清单与哈希保护。
- 固定双 ECU 参考包携带独立预期和离线 `verify.ps1`/测试器；在临时构建目录执行，不修改原包或用户已存在的二进制。预期不由被测生成代码即时计算，超时、缺工具、文件损坏及报文不符均失败关闭。

## UX & Interaction Patterns

- 工作区明确区分普通生成、可重建包导出和行为复验。交付操作拒绝时显示定位原因；生成成功不自动显示行为或实机已验证。
- 交接说明列出外部依赖及重导入、生成、构建、复验步骤；不把本仓库内部路径当作隐含交付内容。

## Cross-Story Dependencies

- Story 3.1 建立完整输入包与安全导出；Story 3.2 在其基础上从新位置重建；Story 3.3 对固定参考包执行独立离线行为复验。三项及整体验收均完成后，Epic 才可结束。
