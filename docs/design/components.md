# Components

Part of the [design system](README.md). Values come from the tokens in the root [DESIGN.md](../../DESIGN.md); behavior comes from the [interaction spec](ux-baseline/EXPERIENCE/README.md).

## Buttons

Restrained and scannable. Primary buttons use the primary-action color and its contrasting text, weight 600. Secondary buttons use canvas and a `line` boundary. Ghost buttons are transparent, for menus, tab strips, and inline actions; icon buttons are 28px square ghosts with a 16px icon, an accessible name, and a tooltip. Danger buttons, for destructive or discarding confirmations, use `error` text and border on canvas and `error-bg` on hover. Hover uses hover and `:active` uses active; primary buttons keep their color with brightness(0.93) on hover and 0.86 pressed. Pressed toggles (`aria-pressed`) use selection colors, unaffected by hover. Disabled text uses muted with opacity 0.65; disabled primary buttons use hover as background. Unavailable commands stay focusable; their group lists each reason below it, with any fix as a ghost action. Keyboard focus is a 2px blue outline offset 2px, or -2px inside clipping containers (scrolling trees and lists, tab strips, menus, the status bar) so the ring stays whole.

## Chips

Role tags use a shell background, tag corner radius, and small text; categories change only the text color. Tags are not primary buttons, and object categories must not be mislabeled as validation results.

## Cards / Containers

Fixed engineering panels use surface, fine boundaries, and panel padding; the central editor uses canvas. These are not decorative cards. Only temporary menus and modal layers use the shadows defined under Elevation and their corresponding corner radii.

## Inputs / Fields

Use a canvas background, a `border-strong` boundary, and control corner radius; see the input token for padding. Selects drop the native look, match the input, and draw a text-secondary chevron. Checkboxes are drawn at 16px with tag radius and a `border-strong` boundary; checked is a `blue` fill with a canvas check. The caret uses primary, and placeholder text uses muted; disabled inputs use a shell background and secondary text. Focus reuses the button outline; error context uses a semantic notice with text, never an invented error border on an input that has no error.

## Navigation

Unified menus, the tool rail, and tabs retain engineering density. Menus group items with separators; the focused or hovered item and the open menu's title use hover, and a disabled reason shared within a group appears once at its end. Selected tree nodes, table rows, and list items use selection colors plus a 2px selection-text bar on the leading edge, never background alone; a pressed tool-rail button shows the bar at the rail edge. Indentation conveys hierarchy. In every tab strip the active tab is text-primary and the rest text-secondary; editor and panel tabs mark the active tab with a bottom primary line, editor tabs also with canvas. Bottom tool-window buttons are toggles with a top primary line when open. Tabs follow the keyboard tab pattern: one tab stop, arrows, Home, End. Do not convert them into oversized navigation cards.

## Configuration Editor

The editing model is set by [ADR 0020](../adr/0020-module-container-editing-model.md): module → container tree, container view in the center, inspector for reading.

- **Tree groups** are tree rows at depth 1 with a section-weight label and a member count in label type; they carry no icon color. Module and object icons take their object role color. Row trailing metadata (`small`) shows error and warning counts as icon plus number in `error` / `warning`, then a modified dot in text-secondary; counts never replace the row name.
- **Container view** stacks a heading (title type for the name, role tag, monospaced path in text-secondary), the sub-container table, then the parameter grid, separated by section headings, not cards. Both tables use the table rules below.
- **Parameter grid** puts the value column first after the name and gives it the widest share; unit, default, range and multiplicity use compact text-secondary, monospaced for numbers and identifiers. A value cell is an input at table density (padding 4px 8px, control radius) that shows its border only on hover, focus or error, so a column of values reads as text. A parameter that has a description carries a help toggle (icon button, `aria-expanded`) after its name; it opens the description inline under the name in text-secondary, so the dense grid stays one line per parameter until asked. The alternate base of an integer and a type hint sit to the right of the value in label type. An unapplied cell carries a `warning` leading bar and the word “unapplied”; a value differing from the default shows a text-secondary “changed from default” mark; errors use the `error` text and icon with the message on hover and focus.
- **Tables** use compact text, surface headers (9px 10px) and fine row separators; rows are 11px 10px, or 6px 10px in the parameter grid where inputs set the height. Selected rows use selection with the 2px leading bar, hover uses hover; selection wins. Sortable headers show the sort direction with an icon and `aria-sort`.
- **Inspector** uses the same object colors and panel padding rather than establishing a separate palette. A parameter's description follows its name, before the definition facts, and leaves no empty block when absent. Definition facts are label / value pairs in compact text; long paths wrap and stay selectable.
- **Problems table** is a table in the bottom tool window: severity icon and word, module, object or parameter, message, file. Filters sit in one row above it (severity toggles as pressed buttons, module select, text filter). The located row stays selected after the focus moves to the editor.
