# Components

本篇属于[设计系统](README.zh-CN.md)。取值来自根目录 [DESIGN.zh-CN.md](../../DESIGN.zh-CN.md) 的 token，行为来自[交互规格](ux-baseline/EXPERIENCE/README.md)。

## Buttons

克制、可扫描。主按钮用主行动色与对应反色文字，权重 600。次按钮为画布底与 `line` 边界。幽灵按钮透明，用于菜单、标签条和行内操作；图标按钮是 28px 见方的幽灵按钮，带 16px 图标、无障碍名称和提示。危险按钮用于破坏性或放弃类确认：画布底、`error` 文字与边框，悬停用 `error-bg`。悬停使用 hover，`:active` 使用 active；主按钮保持主色，悬停 brightness(0.93)，按下 0.86。按下的切换按钮（`aria-pressed`）使用选择色，不被悬停覆盖。禁用文字使用 muted、透明度 0.65；主按钮禁用底为 hover。不可用的命令仍可聚焦；所在动作组下方逐条列出原因，能解决的附幽灵按钮操作。键盘焦点为 2px blue 轮廓、偏移 2px；在会裁切的容器内（滚动的树和列表、标签条、菜单、状态栏）偏移为 -2px，保证焦点环完整。

## Chips

角色标签采用 shell 底、tag 圆角与小字号；类别只改变文字色。标签不是主按钮，也不将对象类别误写成验证结果。

## Cards / Containers

固定工程面板采用 surface、细边界与面板内距，中心编辑器用 canvas；不是装饰卡片。临时菜单和模态层才使用 Elevation 的阴影与对应圆角。

## Inputs / Fields

画布底、`border-strong` 边界与 control 圆角；内距见 input token。下拉框去掉原生外观，与输入框一致，并绘制 text-secondary 色的箭头。复选框自绘，16px、tag 圆角、`border-strong` 边界；勾选时为 `blue` 填充与画布色勾。插入符使用 primary，占位文字用 muted；禁用输入用 shell 底与 secondary 文字。焦点沿用按钮轮廓，错误上下文使用带文字的语义提示，不编造未出现的输入错误边框。

## Navigation

统一菜单、工具轨与标签保持工程密度。菜单以分隔线分组；聚焦或悬停的菜单项与已展开菜单的标题使用 hover，同组共有的禁用原因只在组末显示一次。选中的树节点、表格行和列表项使用选择色，并在起始边加 2px selection-text 竖条，不只依赖背景；按下的工具轨按钮在工具轨边缘显示同样的竖条。层级以缩进表达。所有标签条中活动标签为 text-primary，其余为 text-secondary；编辑标签和面板标签以底部主色标线标示活动项，编辑标签另用画布底。底部工具窗口按钮是切换按钮，打开时顶部有主色标线。标签遵循键盘标签模式：一个 Tab 停靠点、方向键、Home、End。不转换成大号导航卡片。

## Object Table & Inspector

对象表采用紧凑字号、surface 表头和逐行细分隔，选中行用 selection，悬停用 hover，选中优先于悬停。检查器采用同一对象色和 panel 内距，不另建配色世界。
