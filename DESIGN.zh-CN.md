---
name: Classic CAN 配置工作台
description: 中性双主题、紧凑工程密度与明确语义角色的统一工作台视觉基线
colors:
  shell: '#f3f3f3'
  canvas: '#ffffff'
  surface: '#fafafa'
  hover: '#e9e9e9'
  line: '#dddddd'
  text-primary: '#252525'
  text-secondary: '#626262'
  text-muted: '#6b6b6b'
  primary: '#b64c55'
  primary-text: '#ffffff'
  selection: '#e6effc'
  selection-text: '#1c4b86'
  blue: '#285f9e'
  teal: '#146c63'
  purple: '#7953a5'
  gold: '#856016'
  success: '#257447'
  warning: '#8c620f'
  error: '#b23936'
  error-bg: '#fceeed'
  warning-bg: '#fff5dd'
  success-bg: '#eaf5ed'
  dark-shell: '#1b1b1b'
  dark-canvas: '#232323'
  dark-surface: '#202020'
  dark-hover: '#303030'
  dark-line: '#3d3d3d'
  dark-text-primary: '#ededed'
  dark-text-secondary: '#b4b4b4'
  dark-text-muted: '#a1a1a1'
  dark-primary: '#ff979f'
  dark-primary-text: '#491f29'
  dark-selection: '#263e5a'
  dark-selection-text: '#c4dbfa'
  dark-blue: '#97bcef'
  dark-teal: '#8fd2c5'
  dark-purple: '#c0a4e6'
  dark-gold: '#e2c077'
  dark-success: '#91cea4'
  dark-warning: '#e5c27e'
  dark-error: '#eeaaa4'
  dark-error-bg: '#3b2827'
  dark-warning-bg: '#393224'
  dark-success-bg: '#25352b'
typography:
  headline:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '24px'
    fontWeight: 600
    letterSpacing: '-0.025em'
  title:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '19px'
    fontWeight: 600
    letterSpacing: '-0.015em'
  section:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '14px'
    fontWeight: 600
  body:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '13px'
    fontWeight: 400
  compact:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '12px'
    fontWeight: 400
  hint:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '12px'
    fontWeight: 400
    lineHeight: 1.6
  label:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei', sans-serif"
    fontSize: '11px'
    fontWeight: 400
    lineHeight: '20px'
  code:
    fontFamily: "'Cascadia Code', Consolas, 'Courier New', monospace"
    fontSize: '12px'
    fontWeight: 400
rounded:
  control: '5px'
  tag: '3px'
  menu: '6px'
  dialog: '9px'
spacing:
  gap: '8px'
  panel: '16px'
  editor: '20px'
  dialog-inline: '24px'
components:
  button-primary:
    backgroundColor: '{colors.primary}'
    textColor: '{colors.primary-text}'
    rounded: '{rounded.control}'
    padding: '5px 10px'
    typography: '{typography.body}'
  button-secondary:
    backgroundColor: '{colors.canvas}'
    textColor: '{colors.text-primary}'
    rounded: '{rounded.control}'
    padding: '5px 10px'
  button-ghost:
    textColor: '{colors.text-primary}'
    rounded: '{rounded.control}'
    padding: '5px 10px'
  input:
    backgroundColor: '{colors.canvas}'
    textColor: '{colors.text-primary}'
    rounded: '{rounded.control}'
    padding: '7px 9px'
  role-tag:
    backgroundColor: '{colors.shell}'
    textColor: '{colors.text-secondary}'
    rounded: '{rounded.tag}'
    padding: '0 6px'
    typography: '{typography.label}'
  tree-selected:
    backgroundColor: '{colors.selection}'
    textColor: '{colors.selection-text}'
    padding: '4px 6px'
    typography: '{typography.compact}'
  inspector:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.text-primary}'
    padding: '{spacing.panel}'
    width: '290px'
  object-table:
    backgroundColor: '{colors.canvas}'
    textColor: '{colors.text-primary}'
    typography: '{typography.compact}'
  editor-tab-active:
    backgroundColor: '{colors.canvas}'
    textColor: '{colors.text-primary}'
    padding: '7px 13px'
  dialog:
    backgroundColor: '{colors.canvas}'
    textColor: '{colors.text-primary}'
    rounded: '{rounded.dialog}'
---

[English](DESIGN.md)

# Design System: Classic CAN 配置工作台

## Overview

**Creative North Star: "统一工程工作台"**

以用户固定的 JetBrains 菜单、侧栏与底部工具窗口秩序，结合 ChatGPT 式中性浅深主题，形成安静、连续、可扫描的工程界面。品牌表达落在对象角色、精确对齐和少量主行动上，不采用营销式展示字体或巨型标题。

2026-10-03，用户确认本轮界面与透明珊瑚 A 图标“暂时定稿”，作为后续实施的暂定设计基线；这不是正式软件已实施、已验收或已发布的记录。根 DESIGN.md 是后续实现的唯一可变视觉权威；日期目录的同名文件是本轮冻结导出。正式应用尚未迁移，原型与新图标均不表示生产软件已更新。

**Key Characteristics:**

- 浅深主题共享层级与密度。
- 工程树、对象表、检查器保持一致的选中语言。
- 语义色区分对象类别、选择与结果，不铺满工作区。
- 原生系统字体服务中文任务与工程标识符。

视觉取值来自 [完成原型](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/index.html) 的主题变量与复用组件，不采纳现有生产 UI 的样式；交互、状态与任务契约见 [EXPERIENCE](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md)，本文不重复功能规格。

## Colors

浅色采用纸白与浅灰，深色采用石墨面层；前置 YAML 中无前缀 token 为浅色，`dark-` 同名 token 为深色。运行时以 `data-theme` 成组切换，不混用两组。`colors.text-primary`、`colors.selection` 是跨规格稳定引用；深色消费方映射到对应 `dark-` token。

### Primary

- **主行动珊瑚**：`primary`／`primary-text` 用于主按钮和活动标签标线，以克制的实色衔接 A 图形；不把图标渐变铺到控件上，也不等于成功或失败状态。
- **选择蓝底**：`selection`／`selection-text` 用于树焦点、选中行和按下工具按钮；`blue` 也承担链接、焦点轮廓与 CAN 对象角色。

### Secondary

- **对象角色**：蓝色 `blue` 对应 CAN，青绿 `teal` 对应系统／基础软件语境，紫色 `purple` 对应应用对象，金色 `gold` 对应诊断。颜色附着小图标与标签，正文仍使用中性文字。
- **结果语义**：`success`、`warning`、`error` 分别配 `success-bg`、`warning-bg`、`error-bg`；用于提示、问题和差异行，不与对象色交换意义。

### Neutral

- `shell` 为外围壳层，`canvas` 为主编辑面，`surface` 为工程树、检查器及表头，`hover` 为控件悬停反馈。
- `text-primary` 表示任务主体，`text-secondary` 表示路径与说明，`text-muted` 表示弱元信息；`line` 为细分隔与输入边界。

**The Role Before Decoration Rule.** 对象色、选择色和状态色各司其职；同一种对象在树、表和检查器中保持同一角色，状态不能只靠颜色表达。

### Identity

身份资产与 UI 语义色分离。用户偏好 A（1＞3＞2），但要求它与工作台更协调；绿色适配已被明确否决。当前保留 A 的形体，以浅桃红、珊瑚与深莓红形成受控亮暗，不采用近似 JetBrains Toolbox 的橙粉紫转色。JetBrains／VS Code 只校准层次与质感，不提供可复制的品牌配色；主行动使用同一珊瑚色系的可读实色，布局、中性壳层及对象／结果角色保持独立。当前透明珊瑚材质版已由用户暂时定稿，后续变更回到本规格；正式资产在实施时替换。

当前 PNG 图形为 `logo-workbench.png`，原色 A 为 `logo-coral.png`；两者均为真实 RGBA 透明图形，图形外围、车窗及灯位留白透出实际背景，不带白色承载底。浅色用石墨灯位，深色用对应 `-dark.png` 的暖白灯位；仅灯位 RGB 改变，车身色层、轮廓与 alpha 一致。有效主题变化时同步图形，不重置未应用的输入。桌面通用母图 `logo-app.png` 使用中性灯色，ICO／ICNS 不声称随系统主题自动切换。保留生成形体，未声称已有可编辑矢量品牌母版。浅深背景与 16、32、64、128 CSS px 用于比较整体轮廓，较大尺寸看色层；颜色不代表额外产品能力。实际原型与图标评审见 [A 与界面对照](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/logo.html)，默认展示渐变适配，`?logo=coral` 保留平面原色对照。正式图标尚未替换，不使用 AUTOSAR 官方商标。

## Typography

**Body Font:** 原生 Windows UI 字体，中文回退 Microsoft YaHei，最终 sans-serif。
**Label/Mono Font:** UI 标签沿用正文字体；工程标识符及源码使用 Cascadia Code、Consolas、Courier New 与 monospace。

### Hierarchy

- **Headline**：起始页标题，权重与紧缩字距见 token。
- **Title**：编辑区标题；小窗与对话框实际使用 18px 变体，不扩大为展示型标题。
- **Section**：分区标题；检查器对象名实际使用 15px 变体。
- **Body**：默认控件与任务文字。
- **Compact / Hint**：树、表、字段说明；Hint 的行高用于连续说明。
- **Label**：小角色标签与状态元信息。
- **Code**：等宽标识符；数值采用 tabular-nums，日志和预览可按阅读需要增加行高。

未设置全局固定行高，不捏造统一比例尺或展示字体 token；正文尺寸的稳定引用为 `typography.body.fontSize`。

**The Native Operate Type Rule.** 系统 UI 字体用于任务与控件，等宽字体用于标识符、数值和源码；不把工程界面改造成营销展示页。

## Layout

采用纵向壳层与横向工程区，不使用居中的营销内容容器。左工具轨宽 42px、工程树宽 250px、检查器宽 290px，中心弹性填充且允许独立滚动。菜单高 34px，编辑标签带最小高 39px；树节点最小高 30px。面板内距引用 `spacing.panel`，控件间距引用 `spacing.gap`，编辑区和对话框内距引用各自 token。

在最大宽度 1250px 的规则内，工程树／检查器缩至 220／260px，编辑区内距缩至 panel；最大宽度 1079px 时检查器成为宽 300px 的覆盖面板；最大宽度 700px 时工程树也成为覆盖面板，表格保留最小 560px 宽并在容器中横向滚动，表单与设置转为单列。小窗不是移动产品能力声明。

表头内距 9px 10px、表行 11px 10px，小窗统一至 9px 8px。密度服务比较与定位，不把每条数据改成独立卡片。

## Elevation & Depth

**The Flat Workspace Rule.** 固定工作区依靠中性面层和细边界分区；阴影仅表达菜单、对话框与覆盖式侧面板的前后关系。

### Shadow Vocabulary

- **菜单浮层**（`0 8px 24px rgb(0 0 0 / 0.14)`）：临时菜单。
- **模态对话框**（`0 20px 70px rgb(0 0 0 / 0.25)`）：差异与设置；背景遮罩为 `rgb(0 0 0 / 0.28)`。
- **覆盖检查器**（`-10px 0 24px rgb(0 0 0 / 0.08)`）：窄窗右侧覆盖。
- **覆盖工程树**（`10px 0 24px rgb(0 0 0 / 0.1)`）：小窗左侧覆盖。

活动编辑标签的 `inset 0 -2px` 主色标线是状态指示，不是容器升起。动效只记录成品按钮背景／文字的 140ms ease-out 过渡；仅在不要求减少动态效果时启用，不把方向稿中的面板动画意图当作已实现规则。

## Shapes

输入和按钮采用轻微圆角，标签更紧凑，菜单与对话框采用各自容器圆角；值以 `rounded` 为准。结构标签页保持直角，工作区用 1px 中性边界衔接。图标使用内联描边 SVG，通常 17px、1.6 描边、圆端点，不以字符字形冒充图标。应用身份图标独立遵循其 SVG 几何，不套用控件圆角。

## Components

### Buttons

克制、可扫描。主按钮用主行动色与对应反色文字，权重 600；次按钮为画布底与细边界，幽灵按钮为透明底。悬停使用 hover，主按钮保持主色并 brightness(0.93)。禁用文字使用 muted、透明度 0.65；主按钮禁用底为 hover。键盘焦点为 2px blue 轮廓、偏移 2px。

### Chips

角色标签采用 shell 底、tag 圆角与小字号；类别只改变文字色。标签不是主按钮，也不将对象类别误写成验证结果。

### Cards / Containers

固定工程面板采用 surface、细边界与面板内距，中心编辑器用 canvas；不是装饰卡片。临时菜单和模态层才使用 Elevation 的阴影与对应圆角。

### Inputs / Fields

画布底、中性细边界与 control 圆角；内距见 input token。插入符使用 primary，占位文字用 muted；禁用输入用 shell 底与 secondary 文字。焦点沿用按钮轮廓，错误上下文使用带文字的语义提示，不编造未出现的输入错误边框。

### Navigation

统一菜单、工具轨与标签保持工程密度。树节点选中使用 selection 与 selection-text；层级以缩进表达。编辑标签活动状态采用画布底、主体文字与底部主色标线；底部工具标签在顶部使用主色边界。两种位置有意区分，不转换成大号导航卡片。

### Object Table & Inspector

对象表采用紧凑字号、surface 表头和逐行细分隔，选中行用 selection，悬停用 surface，选中优先于悬停。检查器采用同一对象色和 panel 内距，不另建配色世界。

## Do's and Don'ts

### Do:

- **Do** 使用同一套主题角色映射，深色模式同时替换背景、文字、边界、选择和状态色。
- **Do** 使用正文、紧凑文字与等宽文字的既定层级，并保留可见键盘焦点。
- **Do** 在紧凑窗口中收纳侧面板，让对象表在自己的容器内横向滚动。
- **Do** 以对象标签、图标和文字共同表达语义；保持主行动稀少且上下文明确。

### Don't:

- **Don't** 将 logo 固定身份色替换为状态色，或将身份色扩散为大面积工作区装饰。
- **Don't** 在工作区背景或界面文字添加装饰渐变、网格、营销展示字体及重复的大标题卡片；身份图标可使用用户明确要求的受控渐变。
- **Don't** 将所有容器卡片化、浮起化；工作区维持平面结构。
- **Don't** 声称本基线已经迁入正式应用、已获实施批准或通过无障碍符合性认证。
