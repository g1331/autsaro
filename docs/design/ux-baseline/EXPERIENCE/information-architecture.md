# Information Architecture

本篇属于[产品界面与交互基线](README.md)。视觉权威为根目录 [DESIGN.md](../../../../DESIGN.zh-CN.md)，行为权威为本规格。

## 单一壳层与焦点

固定区域按顺序为：应用菜单、紧凑工程／上下文操作条、左侧工具图标轨与一棵持久工程树、中心文档标签与内容、右侧检查器、底部工具窗口、固定状态栏。无工程时显示起始页；设置始终可达。项目切换不是页面切换，必须走保护性工程关闭流程。

- **工程树**只有一份，提供“对象／文件”两种视图及作用域明确的筛选。对象视图包含 CAN 帧／信号、诊断节点，以及标准 ECU 计划实际提供的组件／端口／类型／引用节点；没有数据的未来模块不预占目录。文件视图为原始配置集合，标示只读与保留项。应用源码、生成输出和构建路径显示不同归属，不混为可写 ARXML。
- **中心文档**为 CAN 通信、诊断配置、标准输入／集成、源文件只读视图、源码交付。标签复用同一逻辑文档，不每次选中创建副本。树选择对象时切到所属文档并定位表格行；表格选择反向同步树和检查器。诊断参数编辑与“问题”是不同用途，不再把诊断检查结果伪装成诊断配置页。
- **检查器**固定“属性／引用”切换；属性展示当前对象路径、所属文件、可编辑字段及应用／还原；引用只显示真实关系并支持已有对象定位。关闭面板不关闭对象、不丢草稿。源文件显示绝对路径、访问权限和保留项信息。原始源文本只读查看是拟新增呈现；当前控制器已有文件元数据，未提供独立任意源文件读取方法，正式实现不得把演示文本当作真实文件内容。
- **底部工具窗口**为“问题／生成／构建／主机验证／操作日志”，同一时刻展开一个，切换不卸载编辑草稿。问题呈现 severity、code、message、对象路径与来源文件；生成呈现输出文件／差异／归属；构建和主机验证显示其真实日志；操作日志按实际返回事件保留本次会话记录，不宣称已有持久审计后端。
- **状态栏**仅给当前工程剖面、未应用／未保存提示、目标／宿主、当前操作或结果摘要。点击摘要打开对应窗口；不放新的保证等级总表。

## 统一菜单

| 菜单        | 条目与路由                                                                                      | 不允许的隐式行为                                       |
| ----------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| File／文件  | 新建、导入 ARXML、导入主机交付包、重导入 ECU 交接包、切换工程、预览保存、设置                   | 不用“保存”直接跳过差异确认；导入不叠加到已打开工程     |
| Edit／编辑  | 当前对象应用、还原草稿；CAN 剖面添加帧／信号；诊断有界创建／移除                                | 不承诺全局 Undo/Redo、复制粘贴对象、批量编辑或通用删除 |
| View／视图  | 工程树、检查器、工具窗口、对象／文件视图、主题入口                                              | 不改变工程剖面或数据                                   |
| Tools／工具 | 校验／标准输入检查、源码预览、确认生成、标准 ECU 显式编译预检、独立构建、适用主机验证、取消操作 | 生成不自动编译、预检不自动通过构建、验证不运行旧二进制 |
| Help／帮助  | 当前支持范围与依赖说明，链接现有产品／使用说明                                                  | 不宣称在线支持、授权服务、认证或客户成果               |

上下文操作条只展示当前剖面与文档适用动作。相同动作无论菜单、工具条或上下文入口都调用同一命令和前置条件；不可用项旁给原因及能解决它的入口，而非只给悬浮提示。

## 当前全部用户操作映射

下表的“方法”指现有 `useWorkbench` 公共方法或 UI 使用的状态 setter；拟新增纯界面行为单独说明。它记录当前软件到统一表面的映射，不是内置规则迁移已完成的证据。当前 `configureResources` 和外部资源门控仍存在；内置规则／扩展定义的后端契约与表面迁移属于后续 R5 实施，不能把旧方法当成内置规则配置接口。

| 当前用户操作（含参数选择）                            | 拟定统一表面                     | 现有控制器方法／处理器                                                                                                                |
| ----------------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 切换起始来源、新建名称／目录、创建工程                | 起始页／File 新建                | `setSource`, `setProjectName`, `setProjectDirectory`, `chooseDirectory`, `createProject`                                              |
| 多文件选择、按行填写 ARXML 路径、导入                 | 起始页／File 导入 ARXML          | `chooseFiles`, `setImportPathText`, `setImportPaths`, `importProject`                                                                 |
| 导入可重建主机交付包                                  | 起始页／File 导入主机交付包      | `importHandoff`（选择目录后 `open_handoff_project`）                                                                                  |
| 切换工程／返回起始页                                  | File 切换工程                    | `startProject`, `confirmDiscard`（后端 `close_project`）                                                                              |
| 选择文件、帧、信号；切换现有工作页                    | 工程树／对象表／中心文档         | `choose`, `setPage`；文档标签为拟新增壳层                                                                                             |
| 打开帧／信号创建、填写并添加、取消创建                | Edit／树上下文／检查器           | `openCreator`, `setFrameInput`, `setSignalInput`, `addFrame`, `addSignal`, `setCreating(null)`                                        |
| 修改帧／信号字段、应用、还原                          | 对象检查器                       | `setDraft`, `updateSelected`；还原用 `draftFor(workspace, selection)`＋`setDraft`                                                     |
| 编辑 DoCAN／DID／写入／复位／安全选项                 | 诊断文档与检查器                 | `setDiagnosticDraft`, `configureDiagnostic`                                                                                           |
| DID 信号候选选择、添加、上移／下移／移除              | 诊断文档有序引用列表             | `setDiagnosticSignal`, `setDiagnosticDraft`（现有 EditorPage 本地列表处理器）                                                         |
| 还原诊断草稿、移除诊断配置                            | 诊断文档／Edit 上下文            | `setDiagnosticDraft(diagnosticFields(...))`, `setDiagnosticSignal`, `setDiagnosticError`, `clearDiagnostic`                           |
| 编辑／应用 DTC、选择监测帧、还原、移除                | 诊断文档故障记忆区               | `setDtcDraft`, `setDtcError`, `configureDtc`, `setDtcDraft(dtcFields(...))`, `clearDtc`                                               |
| CAN 配置预览保存、切换差异文件、关闭预览、确认保存    | File／保存差异对话框             | `saveProject`, `setPreviewPath`, `setSavePreview(null)`, `confirmSave`                                                                |
| 校验、查看问题、按对象／文件定位                      | Tools 校验／问题窗口             | `validateProject`, `choose`；现有 DiagnosticsPage 解析可定位目标                                                                      |
| 标准输入检查、CAN ID／周期草稿、应用并校验            | 标准输入文档／检查器             | `inspectIntegration`, `setIntegrationIds`, `setIntegrationPeriod`, `setIntegrationUnapplied`, `applyIntegration`                      |
| 标准输入还原、重开来源                                | 标准输入文档／File 上下文        | `restoreIntegrationDraft`, `reopenIntegration`                                                                                        |
| 标准输入保存预览、差异文件、确认保存                  | 同一保存差异表面                 | `previewIntegrationSave`, `setIntegrationPreviewPath`, `saveIntegration`；关闭用 `setIntegrationPreview(null)`                        |
| CAN 源码／可重建交付包输出目录选择及预览              | 源码交付文档／生成窗口           | `generateProject(false/true)` 内部 `chooseDirectory`                                                                                  |
| CAN 生成预览文件选择、关闭、确认写入                  | 生成预览／确认对话框             | `setGenerationPreviewPath`, `setGenerationPreview(null)`, `confirmGenerate`                                                           |
| 标准 ECU 输出目录、源码／交接种类、源码预览、确认生成 | 同一源码交付文档                 | `changeEcuOutput`, `chooseDirectory`, `setGenerationKind`, `previewEcu`, `generateEcu`；种类变化后调用 `changeEcuOutput` 清除旧结果   |
| 标准 ECU 显式编译预检                                 | Tools／构建窗口                  | `preflightEcu`（报告不等于构建结果）                                                                                                  |
| 选择 Windows／Linux 受控目标                          | 操作条／交付文档目标选择         | `changeTarget`；真实值 `windows-x64-controlled-v1`／`linux-x64-controlled-v1`                                                         |
| 输入／选择独立构建目录、构建 CAN 或标准 ECU           | 源码交付文档／构建窗口           | `changeEcuBuildDirectory`, `chooseDirectory`, `buildProject`／`buildEcu`                                                              |
| 选择对端工程目录与已有主机二进制、双 ECU 信号运行     | 主机验证窗口 CAN 页              | `chooseDirectory`, `setPeerDirectory`, `setPeerBinaryPath`, `chooseBinary`, `setVirtualResult(null)`, `runVirtual`                    |
| CAN 诊断独立测试                                      | 主机验证窗口诊断页               | `runDiagnostic`                                                                                                                       |
| 标准 ECU CAN／DID／N_Cr 主机行为验证                  | 主机验证窗口标准 ECU 页          | `verifyEcu`                                                                                                                           |
| 输入并重导入 ECU 交接目录                             | File 重导入 ECU 交接包／交付文档 | `setEcuImportDirectory`, `importHandoffDirectory`（同一 `open_handoff_project` 后端）                                                 |
| 打开设置、编辑／核对保存 XSD 与 MOD（当前旧流程）    | 历史“官方资源”；后续仅兼容／开发用途，非普通设置 | `setSettingsOpen`, `setResourceDraft`, `configureResources`（未迁移）                                                                 |
| 编辑／保存 compiler、objdump、Git、CPython 路径       | 设置／执行工具                   | `setToolDraft`, `configureTools`                                                                                                      |
| 关闭设置并还原未保存设置草稿                          | 设置关闭按钮                     | SettingsPage close 处理器：从 capabilities 恢复 `setResourceDraft`／`setToolDraft`，`setSettingsNotice('')`, `setSettingsOpen(false)` |
| 取消后台操作、确认保护性操作                          | 操作条取消／保护对话框           | `cancelOperation`, `confirmAction`／`confirmDiscard`                                                                                  |

工程树／对象筛选、工具窗口展开、主题、标签生命周期、焦点管理与会话操作日志属于 PROPOSED，本地界面状态负责；不能以映射表中的 setter 暗示已有真实全文搜索、永久日志、草稿持久化或主题配置接口。
