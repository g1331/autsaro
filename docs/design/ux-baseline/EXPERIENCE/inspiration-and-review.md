# Inspiration and Review Evidence

本篇属于[产品界面与交互基线](README.md)。视觉权威为根目录 [DESIGN.md](../../../../DESIGN.zh-CN.md)，行为权威为本规格。

## Inspiration & Anti-patterns

借鉴 JetBrains 的统一菜单、单一工程上下文、文档标签、边侧检查器与底部工具窗口；借鉴 ChatGPT 桌面的中性明暗层级、阅读舒适与有限角色色；重新设计独立身份标识，保留密集工程数据，不复制聊天结构、营销大标题或商业工具的能力承诺。

禁止：每个阶段单独做孤立页面；多棵竞争工程树；所有模块都像可编辑；底部切换重置草稿；问题跳转丢失输入；预览暗中写盘；生成自动构建／运行；旧结果覆盖新目标；用全绿仪表盘或新的保证等级矩阵概括工程；用纯颜色表示错误；无理由灰按钮；禁用输入充当只读说明；无契约的 ComM／NM／MCU／通用 ECUC 占位控件；把原型中的固定样例、设置路径或模拟成功写成真实核对结果。

## Prototype Coverage & Review Evidence

### 如何评审

打开 [冻结历史交互原型](../mockups/index.html)，先查看默认 CAN 工程；窗口外的“评审场景”可切换起始页、空工程、标准 ECU、缺失资源、问题阻断、来源变更和非本机目标。刷新重置全部内存状态。“设置 → 外观”可切换浅色、深色或跟随系统；更改外观后点击“应用”，取消仅丢弃尚未应用的设置草稿。其“官方资源”表面、缺失档案场景和先配置档案入口已被后续内置规范取代，仅供历史评审，不是新规则 UX 的验证。

原型覆盖工程入口、对象／文件导航、帧和信号草稿、诊断与 DTC、有序 DID、标准参数、保护性切换、逐文件保存预览、独立源码确认、预检／构建／运行结果窗口、取消／失效、设置和命令搜索。源码及 ARXML 仅提供标明用途的示意片段，不是完整生成物。设置“浏览”填入示例路径而非打开本机文件选择器。

规格完整但冻结原型不承担的正式接入细节：真实文件对话框、内置规则清单完整性与身份／覆盖展示、可选扩展的真实接受与消费者隔离、来源完整字节／真实生成差异、后端计划与拒绝原因、实际进程取消及残留文件处理、完整 CAN／DID／N_Cr 报告、所有字段错误关联及精确焦点回退、可调整分隔器、退出保护、持久主题和受支持权限下的桌面 IPC。旧原型也未接入其历史官方档案摘要核对与监听；该旧表面不是普通用户的新依赖。正式接入须保留当前 Workbench／Session 的安全与归属约束，并按后续 R5 契约迁移规则身份、设置和门控，不可把原型计时器、路径示例或本地校验当成生产逻辑。

### 已运行的原型验证

在无头 Chromium 中实际操作十组场景：名称／方向应用与保存；草稿保护及标签关闭；诊断参数／DID 排序与还原；标准可编辑范围；问题定位及位段重叠修复；生成后独立构建、取消旧回调、分类型运行及结果失效；非本机目标／来源变化门禁；设置拒绝、类别隔离与取消；未保存工程导入保护及新建；工程树键盘／筛选／命令搜索。十组通过，浏览器运行错误列表为空。生成、构建和运行结果均为原型模拟，不形成新的主机能力证据。

实际画面覆盖 [浅色桌面](../mockups/screenshots/desktop.png)、[深色桌面](../mockups/screenshots/desktop-dark.png)、[最低桌面尺寸](../mockups/screenshots/minimum-desktop.png)、[窄屏评审](../mockups/screenshots/mobile.png)、[标准输入](../mockups/screenshots/standard-input.png)、[诊断](../mockups/screenshots/diagnostic.png)、[资源设置](../mockups/screenshots/settings-resources.png)、[起始页](../mockups/screenshots/welcome.png)、[问题](../mockups/screenshots/problem-locate.png)、[保存预览](../mockups/screenshots/save-preview.png)、[源码预览](../mockups/screenshots/source-preview.png)、[主机验证窗口](../mockups/screenshots/host-verification.png)与 [logo 候选](../mockups/screenshots/logo-sizes.png)。CSS 视口分别为 1480×920、1080×720、390×844，DPR 为 1.25；页面级横向溢出均未出现，数据表保留局部滚动。

独立界面 finish review 返回 `disposition: ship`，仅表示界面可进入设计评审；未提供 catalog QUALITY BAR／decision comp，评审依据用户明确指定的工程工具体系。该历史 verdict 不包含品牌批准；当前透明珊瑚 A 图标与界面基线的暂时定稿来自用户 2026-10-03 的明确确认。未执行正式软件构建、原生桌面 IPC 或无障碍符合性验收。

上述实际原型验证及截图保留原始事实；[资源设置](../mockups/screenshots/settings-resources.png)和[起始页](../mockups/screenshots/welcome.png)中的旧官方资源行为为已取代的历史表面。本次仅更新规划，未验证原生规则、新设置、扩展恢复或无官方档案的真实工程旅程。实施验收仍须开发侧以合法持有的固定 R24-11 XSD／MOD 对照独立正例和结构／顺序／基数／类型／值／默认值／引用／跨模块聚焦负例及已知未支持案例；用户安装验收则在无官方档案路径／环境文件、无 checkout、无 Node／Rust／uv／编译器的机器上完成真实编辑、保存、重开、校验、生成及预期拒绝，不能用冻结原型通过结果替代。
