---
name: Classic CAN Workbench
description: A unified workbench visual baseline with neutral light and dark themes, compact engineering density, and clear semantic roles
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

[简体中文](DESIGN.zh-CN.md)

# Design System: Classic CAN Workbench

## Overview

**Creative North Star: "Unified Engineering Workbench"**

Combine the user-established JetBrains arrangement of menus, sidebars, and bottom tool windows with ChatGPT-style neutral light and dark themes to create a calm, continuous, scannable engineering interface. Brand expression comes from object roles, precise alignment, and a small number of primary actions—not marketing-style display fonts or oversized headings.

On 2026-10-03, the user confirmed that this iteration of the interface and the transparent coral A icon were “provisionally finalized,” establishing a provisional design baseline for subsequent implementation. This is not a record that the production software has been implemented, accepted, or released. Root DESIGN.md is the sole mutable visual authority for subsequent implementation; the file of the same name in the dated directory is this iteration's frozen export. The production application has not yet been migrated, and neither the prototype nor the new icon indicates that production software has been updated.

**Key Characteristics:**

- Light and dark themes share hierarchy and density.
- The project tree, object table, and inspector use consistent selection styling.
- Semantic colors distinguish object categories, selection, and results without filling the workspace.
- Native system fonts support Chinese-language tasks and engineering identifiers.

Visual values come from the theme variables and reusable components of the [completed prototype](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/index.html), not from the existing production UI's styling. Interaction, state, and task contracts are documented in [EXPERIENCE](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md); this document does not repeat the functional specifications.

## Colors

The light theme uses paper white and light gray; the dark theme uses graphite surfaces. In the YAML frontmatter, unprefixed tokens belong to the light theme, and corresponding `dark-` tokens belong to the dark theme. At runtime, switch the complete set through `data-theme`; do not mix the two sets. `colors.text-primary` and `colors.selection` are stable cross-specification references; dark-theme consumers map them to the corresponding `dark-` tokens.

### Primary

- **Primary-action coral**: `primary` / `primary-text` are used for primary buttons and active-tab indicator lines, connecting to the A graphic through restrained solid colors. Do not spread the icon's gradient across controls; this color does not represent success or failure.
- **Selection blue background**: `selection` / `selection-text` are used for tree focus, selected rows, and pressed toolbar buttons; `blue` also serves links, focus outlines, and the CAN object role.

### Secondary

- **Object roles**: Blue `blue` represents CAN, teal `teal` represents system / basic-software contexts, purple `purple` represents application objects, and gold `gold` represents diagnostics. Apply these colors to small icons and tags; body text remains neutral.
- **Result semantics**: Pair `success`, `warning`, and `error` with `success-bg`, `warning-bg`, and `error-bg`, respectively. Use them for notices, issues, and difference rows; do not interchange their meanings with object colors.

### Neutral

- `shell` is the outer shell, `canvas` is the main editing surface, `surface` is used for the project tree, inspector, and table headers, and `hover` provides control hover feedback.
- `text-primary` indicates primary task content, `text-secondary` indicates paths and explanations, and `text-muted` indicates low-emphasis metadata; `line` is used for fine separators and input boundaries.

**The Role Before Decoration Rule.** Object colors, selection colors, and status colors each serve their own purpose. The same object retains the same role in the tree, table, and inspector; status must not be conveyed by color alone.

### Identity

Identity assets are separate from UI semantic colors. The user prefers A (1＞3＞2), but requires it to harmonize better with the workbench; a green adaptation was explicitly rejected. Retain A's current form, using light peach pink, coral, and dark berry red for controlled light and dark tones, rather than an orange-pink-purple transition resembling JetBrains Toolbox. JetBrains / VS Code serve only to calibrate hierarchy and texture, not as sources of brand palettes to copy. Primary actions use readable solid colors from the same coral family; layout, the neutral shell, and object / result roles remain independent. The current transparent coral-material version has been provisionally finalized by the user; subsequent changes must return to this specification, and production assets will be replaced during implementation.

The current PNG graphic is `logo-workbench.png`, and the original-color A is `logo-coral.png`. Both are genuine RGBA transparent graphics: the areas outside the graphic, windows, and light cutouts reveal the actual background, with no white backing. The light theme uses graphite lights; the dark theme uses warm-white lights in the corresponding `-dark.png` file. Only the lights' RGB values change; the body color layers, contours, and alpha remain identical. Synchronize the graphic when the effective theme changes without resetting unapplied input. The general-purpose desktop master image `logo-app.png` uses neutral light colors; ICO / ICNS assets are not claimed to switch automatically with the system theme. Retain the generated form; there is no claim that an editable vector brand master already exists. Light and dark backgrounds and sizes of 16, 32, 64, and 128 CSS px are used to compare the overall silhouette; larger sizes are used to inspect color layers. Colors do not represent additional product capabilities. See [A and interface comparison](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/mockups/logo.html) for the actual prototype and icon review. The gradient adaptation is shown by default; `?logo=coral` preserves the flat original-color comparison. The production icon has not yet been replaced, and the official AUTOSAR trademark is not used.

## Typography

**Body Font:** Native Windows UI fonts, with Microsoft YaHei as the Chinese fallback and sans-serif as the final fallback.
**Label/Mono Font:** UI labels use the body font; engineering identifiers and source code use Cascadia Code, Consolas, Courier New, and monospace.

### Hierarchy

- **Headline**: Start-page heading; see the tokens for weight and tight letter spacing.
- **Title**: Editor-area heading; compact windows and dialogs actually use an 18px variant, rather than enlarging it into a display heading.
- **Section**: Section heading; inspector object names actually use a 15px variant.
- **Body**: Default control and task text.
- **Compact / Hint**: Trees, tables, and field explanations; Hint line height supports continuous explanatory text.
- **Label**: Small role tags and status metadata.
- **Code**: Monospaced identifiers; numbers use tabular-nums, and logs and previews may use increased line height as needed for readability.

No globally fixed line height is defined. Do not invent a uniform type scale or display-font tokens; the stable reference for body size is `typography.body.fontSize`.

**The Native Operate Type Rule.** Use system UI fonts for tasks and controls, and monospaced fonts for identifiers, numbers, and source code; do not turn the engineering interface into a marketing showcase.

## Layout

Use a vertical shell and a horizontal engineering area, not a centered marketing-content container. The left tool rail is 42px wide, the project tree is 250px wide, and the inspector is 290px wide. The center fills the remaining space flexibly and allows independent scrolling. The menu is 34px high, the editor tab strip has a minimum height of 39px, and tree nodes have a minimum height of 30px. Panel padding references `spacing.panel`, control spacing references `spacing.gap`, and editor and dialog padding reference their respective tokens.

Under the maximum-width 1250px rule, the project tree / inspector shrink to 220 / 260px, and editor padding shrinks to panel padding. At a maximum width of 1079px, the inspector becomes a 300px-wide overlay panel. At a maximum width of 700px, the project tree also becomes an overlay panel, tables retain a minimum width of 560px and scroll horizontally within their containers, and forms and settings switch to a single column. Compact windows are not a claim of mobile-product capability.

Table-header padding is 9px 10px, and table-row padding is 11px 10px; both become 9px 8px in compact windows. Density supports comparison and navigation; do not turn each data item into a separate card.

## Elevation & Depth

**The Flat Workspace Rule.** Fixed workspace areas are separated by neutral surfaces and fine boundaries; shadows express layering only for menus, dialogs, and overlay side panels.

### Shadow Vocabulary

- **Menu overlay** (`0 8px 24px rgb(0 0 0 / 0.14)`): Temporary menus.
- **Modal dialog** (`0 20px 70px rgb(0 0 0 / 0.25)`): Differences and settings; the backdrop is `rgb(0 0 0 / 0.28)`.
- **Overlay inspector** (`-10px 0 24px rgb(0 0 0 / 0.08)`): Right-side overlay in narrow windows.
- **Overlay project tree** (`10px 0 24px rgb(0 0 0 / 0.1)`): Left-side overlay in compact windows.

The active editor tab's `inset 0 -2px` primary-color indicator line denotes state, not a raised container. The only recorded motion is the completed buttons' 140ms ease-out background / text transition. Enable it only when reduced motion is not requested; do not treat panel-animation intentions from the design-direction draft as implemented rules.

## Shapes

Inputs and buttons use subtle rounding, tags are more compact, and menus and dialogs use their own container radii; `rounded` defines the values. Structural tabs remain square, and workspace areas join through 1px neutral boundaries. Icons use inline stroke SVG, typically 17px with a 1.6 stroke and round caps; do not substitute character glyphs for icons. The application identity icon independently follows its SVG geometry and does not inherit control corner radii.

## Components

### Buttons

Restrained and scannable. Primary buttons use the primary-action color and its corresponding contrasting text color, with weight 600. Secondary buttons use the canvas background and a fine boundary; ghost buttons have a transparent background. Hover uses hover; primary buttons retain the primary color with brightness(0.93). Disabled text uses muted with opacity 0.65; disabled primary buttons use hover as their background. Keyboard focus uses a 2px blue outline with a 2px offset.

### Chips

Role tags use a shell background, tag corner radius, and small text; categories change only the text color. Tags are not primary buttons, and object categories must not be mislabeled as validation results.

### Cards / Containers

Fixed engineering panels use surface, fine boundaries, and panel padding; the central editor uses canvas. These are not decorative cards. Only temporary menus and modal layers use the shadows defined under Elevation and their corresponding corner radii.

### Inputs / Fields

Use a canvas background, a fine neutral boundary, and control corner radius; see the input token for padding. The caret uses primary, and placeholder text uses muted; disabled inputs use a shell background and secondary text. Focus uses the same outline as buttons. Error contexts use semantic notices with text; do not invent input-error borders that are not present.

### Navigation

Unified menus, the tool rail, and tabs retain engineering density. Selected tree nodes use selection and selection-text; indentation conveys hierarchy. Active editor tabs use a canvas background, primary text, and a bottom primary-color indicator line; bottom tool tabs use a primary-color border at the top. The two positions are deliberately distinct; do not convert them into oversized navigation cards.

### Object Table & Inspector

Object tables use compact text, surface table headers, and fine row separators. Selected rows use selection, hover uses surface, and selection takes precedence over hover. The inspector uses the same object colors and panel padding rather than establishing a separate palette.

## Do's and Don'ts

### Do:

- **Do** use the same theme-role mapping, replacing backgrounds, text, boundaries, selection, and status colors together in dark mode.
- **Do** use the established hierarchy of body, compact, and monospaced text, and retain visible keyboard focus.
- **Do** collapse side panels in compact windows and allow object tables to scroll horizontally within their own containers.
- **Do** convey semantics through object tags, icons, and text together; keep primary actions sparse and their context clear.

### Don't:

- **Don't** replace the logo's fixed identity colors with status colors or spread identity colors into large areas of workspace decoration.
- **Don't** add decorative gradients, grids, marketing display fonts, or repeated oversized heading cards to workspace backgrounds or interface text; the identity icon may use the controlled gradient explicitly requested by the user.
- **Don't** turn every container into a card or an elevated surface; maintain the workspace's flat structure.
- **Don't** claim that this baseline has already been migrated into the production application, approved for implementation, or certified for accessibility conformance.
