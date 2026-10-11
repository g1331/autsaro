# 0007. 用户应用源 live／sealed 所有权

## 背景

应用开发者维护 SWC 算法源码，工作台负责生成接口与集成代码并封存可独立构建的工程。如果生成器可以写用户源码，再生成会覆盖用户改动；如果封存工程中的应用文件可编辑，严格完整性校验就会失效或被迫放宽。

## 决策

1. 工程 manifest 显式声明应用输入（`applicationInputs[{path,producerSlot}]`）。槽位由可信计划派生（producerSlot、componentPath、sourcePaths、generatedHeaders、entrySymbols），前台只选择与回显，未知槽或路径拒绝。
2. **live 源属于用户**，永远不在生成器可写集内。封存输出中的应用文件是逐字节**不可变快照**，`snapshotOf` 指向已声明源成员，不记录作者绝对路径。
3. 初始化 **create-only**：只创建尚不存在的 live 成员，并与 manifest 接纳在同一事务内；目标已存在不覆盖，失败不留下部分成员。
4. 所有权台账（`workbench-ownership.json`）记录 `files[{path,owner,producerId,sha256,snapshotOf?}]`；所有权由实际 profile 与槽重建，不信任台账中的 owner 字段作为写权限。台账与快照进入 `files.list`／`files.sha256` 闭包，台账不记录自身与 seal 元数据。
5. 应用源字节、规则与定义身份属于生成准备身份：预览捕获 live 字节，确认前核对未变；任何外部改动使旧确认失效。
6. 重导入时核对完整 seal 与身份，在新的私有工程目录恢复 live 输入；封存快照永不直接转为可写源。被改动的封存快照一律拒绝。

## 后果

- 再生成、异地重导入、构建和运行逐字节保留全部用户源码；篡改可检测。
- 生成改变使旧 build／run 结果失效；用户算法的正确性仍需显式构建与行为验证，源码准备只证明接口与来源。
- 用户代码以快照进入交付包，不同于常见的“在工程目录里直接写代码”做法，这是有意选择（布局见 ADR 0016）。

## 状态

已接受（2026-10-03，多组件扩展 2026-10-09）。

来源：`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md`（AD-7、AD-9、Ownership 与 sealing）、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-7/reviews/review-seams.md`、`planning-archive-2026-10:_bmad-output/planning-artifacts/architecture/epic-8/ARCHITECTURE-SPINE.md`（AD-7、AD-8）、`planning-archive-2026-10:_bmad-output/specs/spec-multi-component-scheduling/application-contract.md`（生成与源码归属）。
