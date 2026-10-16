# 0019. 工作台交互组件采用 Radix Primitives，外观由自有 token 决定

## 背景

工作台的交互组件全部手写：菜单栏与菜单（`App.tsx` 的 `role="menu"`、`workbench/menu.tsx`）、对话框（`workbench/Dialog.tsx` 自行管理焦点与层叠）、命令面板（`App.tsx`）、编辑器与面板标签（`workbench/Tabs.tsx`），提示只用 `title` 属性。界面审计（[#227](https://github.com/g1331/autsaro/issues/227)）的发现里有相当一部分来自这类组件的焦点、键盘与读屏细节：菜单键盘支持不完整（I7）、命令面板不能用键盘操作（E3）、禁用按钮的原因只在 `title` 里，键盘用户读不到。第二波 [#230](https://github.com/g1331/autsaro/issues/230)、[#231](https://github.com/g1331/autsaro/issues/231) 还要继续实现菜单、对话框、弹出层与命令面板。

这类组件的行为有成熟的规范（WAI-ARIA Authoring Practices）和成熟的实现：焦点进入与归还、焦点约束、方向键与首字母导航、Esc 与外部点击关闭、弹层定位与翻转、读屏语义。逐个手写并补齐这些细节，投入大且容易在边角处出错；它们不是本产品的差异所在。

外观已经有自己的体系：`DESIGN.md` 规定了工程工作台的密度、层次与对象角色色，[#228](https://github.com/g1331/autsaro/issues/228) 把它落成 `styles.css` 的 token 与共享按钮、控件。

## 决策

1. **复杂交互组件使用 Radix Primitives**（npm 包 `radix-ui`，MIT），只用它的无样式组件：Menubar、DropdownMenu、ContextMenu、Dialog、AlertDialog、Popover、Tooltip。新写或重写这些组件时直接使用，不再手写同类行为。
2. **命令面板使用 `cmdk`**（MIT），负责过滤、方向键与 Enter 选择和空状态，放在 Radix Dialog 中。
3. **外观完全由自有样式决定。** 通过 `className` 与 Radix 暴露的 `data-state`、`data-disabled`、`data-highlighted` 等属性在 `styles.css` 中用现有 token 写样式；不引入 Tailwind，不复制 shadcn/ui 的组件外观。`DESIGN.md` 继续是唯一的视觉规范。
4. **禁用项的原因必须可读。** Radix 菜单项的 `disabled` 会让该项退出键盘导航（`RovingFocusGroup.Item focusable={!disabled}`），与交互规格“不可用项旁给原因”冲突。需要读到原因的项不用 `disabled`，改用 `aria-disabled="true"`，在 `onSelect` 中 `preventDefault()` 并以 `data-disabled` 样式呈现，原因用 `aria-describedby` 关联。
5. **保留原生与已合格的实现。** 原生 `<select>` 的键盘与读屏行为由平台提供，继续使用并按 token 设置样式；列表项需要富内容或搜索时才换成 Radix Select。`workbench/Tabs.tsx` 已按标签模式实现且带“Delete 关闭”扩展，保留，不为统一而重写。
6. **依赖随首个使用者加入。** `radix-ui` 与 `cmdk` 由第一个实际使用它们的 PR 加入 `ui/package.json`，不预先添加未使用的依赖；版本由 `package-lock.json` 锁定。

拒绝的方案：

- **继续手写。** 每个组件都要重复实现并测试同一套焦点与键盘规则，审计已经显示遗漏会持续出现。
- **采用 shadcn/ui。** 它是 Radix 加 Tailwind 写成的默认外观；外观部分与 `DESIGN.md` 的工作台密度和层次不符，采用后仍需全部改写，还要把样式体系换成 Tailwind。需要的只是它下面的 Radix 这一层。
- **React Aria Components（Adobe）。** 无障碍与国际化同样完整，也能配合自有样式使用，能力上可以替代。选 Radix 是因为它的组件集合正好对应所需的几类、示例与社区资料更多（shadcn/ui 即建在它上面）；两者对本项目差别不大，没有必要同时引入。
- **带完整外观的组件库（MUI、Ant Design 等）。** 自带设计体系，与 `DESIGN.md` 冲突，覆盖样式的成本高于直接使用无样式组件。

## 后果

- 菜单、对话框、弹出层与提示的焦点和键盘行为由库保证；项目测试只覆盖自身的契约（命令可用性与原因、对话框按用途的尺寸、确认只出现一次），不重复测试库已经保证的通用行为。
- Radix 的弹层通过 Portal 渲染到 `document.body`，桌面场景测试按角色与可访问名称定位元素，不依赖 DOM 层级。jsdom 缺少部分浏览器接口（如 `ResizeObserver`、`hasPointerCapture`、`scrollIntoView`），用到相应组件的前端测试需在 Vitest 的测试准备中补上最小实现。
- 运行时依赖增加 `radix-ui` 与 `cmdk`，打包体积随实际引入的组件增加；许可均为 MIT，依赖更新按 `dependencies` 标签的常规流程审阅。
- `workbench/Dialog.tsx` 中自行维护的焦点与层叠逻辑在 #231 迁移到 Radix Dialog 后删除，不保留两套实现。

## 状态

已接受（2026-10-17，[#241](https://github.com/g1331/autsaro/issues/241)）；迁移在 #230（菜单、菜单栏、提示）与 #231（对话框、弹出层、命令面板）中实施。
