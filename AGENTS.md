# Agent 开发指南

先读 [CONTRIBUTING.md](CONTRIBUTING.md)、README 和相关模块说明，再核对目标版本、分支及工作区。保留用户修改，使用项目正常开发入口。

## 结构与开发

- `core/`：Rust 配置、解析、生成及验证；`src-tauri/`：桌面后端；`ui/`：React 界面。
- `runtime/`：交付的 C99 运行时；`tools/python/src/`：开发工具与离线 ECU 工具；`tests/`：Python 与桌面测试。
- `scripts/`：官方资料收集；`tools/automotive/`：可选技能维护；`docs/development/` 与 `docs/maintainers/`：开发与发行说明。

采用 npm、Cargo、uv 和正常测试运行器，不新增代理专用编排、报告或状态系统。格式按修改范围检查，不全仓重排。自动 GUI 验收使用隔离入口，不占用用户桌面。

修改 C、头文件或生成 C 的 Rust 模板前读取 [misra-c2012](.agents/skills/misra-c2012/SKILL.md)。交付摘要使用 `python -m autosar_tooling assets check|update`；明确审阅 ABI 与第三方身份，不用摘要更新吸收契约变化。

BMad 是规格与任务状态来源，按安装技能推进；记录结果到相关规格，不额外生成报告、矩阵或台账。状态查询只读。普通项目启动和开发不依赖代理。
