# 汽车工程技能

本仓库安装 automotive-skills-suite 的 12 组 builder/reviewer，共 24 个技能。它们辅助编制及审阅 Excel 工程规格，BMad 继续管理需求、设计、任务和进度。按当前任务使用，不要求每次开发都制作文档。

## 清单

每组包含 `<名称>-builder` 和 `<名称>-checklist-reviewer`。

| 名称 | 用途 | 对应工作 |
| --- | --- | --- |
| traceability-matrix | 需求、设计、测试、结果追踪 | 各阶段 |
| test-case-catalog | 测试输入、独立预期、验收条件 | Epic 3 和后续 |
| verification-plan | 验证方法、环境、验收计划 | Epic 集成 |
| uds-services | 服务、会话、DID/RID、NRC、时序 | 主机诊断与 Epic 4–5 |
| dtc-catalog | 故障码与关联诊断数据 | Epic 2、5 |
| dem-config | 事件、去抖、恢复、存储映射 | Epic 2、5 |
| arxml-system | 系统、ECU、信号/PDU/帧映射 | Epic 4 |
| autosar-swc | 端口、类型、runnable、事件 | Epic 4 |
| autosar-composition | 组件实例及连接 | Epic 4 |
| autosar-rte-mapping | 应用、通信、OS 映射 | Epic 4 |
| autosar-bsw-config | 模块、参数、依赖和调度 | Epic 4–6 |
| secure-coding-guidelines | 安全编码规则与偏离处理 | C99 质量规则 |

## 安装与维护

技能位于 `.agents/skills/`，无需安装到个人全局目录。源为 [automotive-skills-suite](https://github.com/jherrodthomas/automotive-skills-suite)，固定提交 `6ad8818b6a566e0c2f5c977d8038a579f9fed9f1`；每个技能包含上游 MIT LICENSE。`scripts/automotive-skills-lock.json` 记录包 SHA-256 和安装文件清单/哈希，不记录任务状态。

原始包按原始字节计算 SHA-256；安装的文本文件和适配源文件按 LF 归一化后计算哈希，兼容 Windows Git 的 CRLF checkout，其他字节改动仍会触发冲突。二进制文件保持原始字节校验。

在仓库根目录准备独立的 Python 环境：

```powershell
python -m venv .automotive-skills-venv
& .\.automotive-skills-venv\Scripts\python.exe -m pip install -r scripts/requirements-automotive-skills.txt
python scripts/install_automotive_skills.py --check
& .\.automotive-skills-venv\Scripts\python.exe scripts/verify_automotive_skills.py
```

依赖为 `openpyxl==3.1.5` 与 `et-xmlfile==2.0.0`。专用环境和 Python 缓存不提交；普通生成与审阅不需要 Office。需要公式重算时只使用可用的无头工具，缺少时报告“未重算”，不启动用户桌面窗口。

全新 Git checkout 已携带团队共用技能。已记录的原样安装运行 `python scripts/install_automotive_skills.py` 只校验；部分缺失/本地修改需先审阅和恢复，安装器拒绝静默覆盖。更新时明确执行 `python scripts/install_automotive_skills.py --ref <完整提交SHA> --update` 并审阅差异；补丁不再匹配时停止。安装器不更新 BMad 文件。

本地补丁源位于 `scripts/automotive_skill_adapters.py` 和 `scripts/automotive_skill_patches/`。修改补丁源后对原样安装运行 `--update`，重新生成技能副本及锁文件。不要直接修改已安装副本再要求安装器覆盖。

## 输入、产物和结论

查看每个技能的 `SKILL.md`；上游字段和原文保留在 `references/upstream-SKILL.md`，其中脚本路径相对技能根目录。本地入口收紧触发范围并优先解释适用边界。

- JSON 是可维护源，可用 `provenance` 记录来源工件相对路径、Git 提交和验证状态。Excel 是派生产物；规划输入/导出放在相关 BMad 工件旁，验收输入/报告放在对应证据目录。
- 所有生成工作簿和审阅报告附 `Repository Scope` 页，标明文档用途、版次、建议阈值及固定模板限制。临时演练输出不入库。
- 上游 AUTOSAR 模板主要基于 R22-11；R24-11 参数和规范义务须对照合法官方资料，未核实就保留待核实。表内的旧版本信息保留为真实来源，不改字符串冒充迁移。
- 不从模板推断 ASIL/CAL、硬件、已实现模块、批准状态或规范符合性。固定示例、历史修订和建议必须经复核，不能当成项目发生过的事件。
- 内容检查、结构检查和待确认判断分开解释。空评分、`NA` 和标注 DRAFT 的结果不算通过；脚本退出 0 只证明报告生成成功。上游 95% 等阈值不成为仓库验收门。
- 追踪覆盖率仅统计输入清单及其关联，不证明规范清单完整，不代表运行测试通过或整个 AUTOSAR 的完成率。能力声明仍以独立证据门为准。

## 安装副本的最小修正

1. 所有入口：收紧触发，补充 R24-11/BMad/证据约定；原始 SKILL.md 留存；工作簿附仓库范围页，JSON 使用 UTF-8。
2. 测试目录：修复上游 builder 只写三列、reviewer 却读另一工作表的问题；导出前置条件、输入、预期、通过条件、需求、环境等字段，探针按列名读取，保留旧三列表兼容；检查空/重复 ID、缺失设计字段，语义充分性仍待审阅。
3. 验证计划：未实际评估的项目不再默认 LC；字段存在仅作结构检查；探针读取实际范围内容而非标题。
4. DTC reviewer：修复摘要访问结果元组错误字段导致崩溃的问题。
5. 安全编码：修复 reviewer 的 dataclass/字典、仪表盘分组和分类字段接口不匹配；builder 支持上游样例中的嵌套项目元数据，将过长工作表名缩短并同步探针。批准人姓名不证明审阅者资质；固定规则的编号、正文、严重性和 CWE 对应关系明确标为未经权威核实的上游模板，不直接采用为仓库规则。
6. RTE reviewer：任务优先级列表不证明优先级反转分析；栈和触发延迟没有分析证据时保留待确认。
7. 追踪矩阵：保留输入中报告的测试结果，在 reviewer 的 `Execution Evidence` 中展示；没有独立核实的结果不会升级为运行通过，未评估检查保持 NA。关联覆盖率与测试执行结果分开呈现。

没有把模板升级为完整标准检查器。使用方应审阅原始检查算法和输入范围；本次测试仅验证文档脚本及上述缺陷处理，不是产品运行验收。

## Epic 3.3 设计演练

```powershell
& .\.automotive-skills-venv\Scripts\python.exe scripts/verify_automotive_skills.py --demo
```

演练读取当前 Story 3.3，生成 CAN、诊断及拒绝/恢复路径的测试目录与追踪示例，然后调用配套 reviewer。输入记录 Story 摘要、Git 版本及文件哈希；用例全部标为“尚未执行”。产物仅在临时目录生成和检查，不改变 Story 状态或能力等级。它验证技能可用，不运行虚拟 ECU。
