# 历史归档说明

本文档是 Ratatui 组件化/retained UI 重构方向的历史设计资料，已归档。它不代表当前 TUI 已实现结构，也不作为当前开发计划。若要恢复其中设计，应先基于当前代码重新审查。

English: Archived historical Ratatui refactor plan. It is not the current implementation plan.

# Ratatui Component Refactor Plan

Status: worktree-first deterministic architecture plan
Date: 2026-07-02

This document is the governing plan for refactoring the current TUI implementation from a large procedural `app.rs` into a component-oriented, retained-state, declarative-layout architecture.

This plan intentionally prioritizes refactor quality over short-term edit speed. The primary objective is to prevent `tui/src/app.rs` from remaining or becoming a monolithic file. Test infrastructure is not a goal of this plan; lightweight checks are useful, but they must not become the center of the refactor.

Large refactors must happen in a separate git worktree so the current working tree can remain a usable baseline for comparison. Do not destructively reshape the only checkout.

## 1. Refactor Direction And Feasibility

### 1.1 Decision

The project should keep `ratatui` as the rendering backend and build an internal component and layout layer above it.

The target architecture is:

- `ratatui` remains the low-level immediate-mode renderer.
- Project-owned `UiState` owns retained UI state, focus, scroll, modal stack, and hitboxes.
- Components are mostly stateless render/event adapters over explicit props and explicit retained state.
- Layout is declared through project-owned `Stack`, `Flex`, `Grid`, `Panel`, and `Slot` primitives instead of hand-splitting rectangles inside each page.
- Event routing is component-id based, not area-function based.
- Pages are thin compositions of reusable components.
- Old procedural page rendering is deleted as each page migrates.

The project should not immediately migrate to `tui-realm`, `iocraft`, or another framework. Those libraries are useful references and may be used for isolated experiments, but the mainline should first build a small internal UI kit that matches this project's specific command, modal, hitbox, and audit requirements.

### 1.2 Why This Is Feasible

`ratatui` is explicitly an immediate-mode UI library: every frame redraws from application state, and it does not keep permanent widget objects. This does not prevent retained UI architecture. It only means retention must live in our layer, not in `ratatui` widgets.

The correct model is:

```text
Retained application/component state
        |
        v
Declarative component tree per frame
        |
        v
Resolved layout rectangles and hitboxes
        |
        v
Immediate ratatui rendering
```

This is compatible with ratatui because the final renderer still calls `frame.render_widget(...)` every frame. The retained part is the project-owned model: selected row, scroll offset, focused component, current modal, command output scroll, form fields, and last layout tree.

### 1.3 Why This Is Not Against Ratatui's Direction

The goal is not to force `ratatui::widgets::Widget` into a browser DOM. The goal is to impose architecture above a deliberately unopinionated rendering library.

Ratatui's own documentation describes component architecture as a valid application organization approach where each component encapsulates state, event handling, update, and rendering logic. It also states that ratatui itself does not dictate application structure. Therefore, a project-level component architecture is aligned with ratatui's intended use.

### 1.4 Ecosystem Assessment

Relevant current ecosystem facts:

- `ratatui` rendering is immediate-mode. It redraws the UI every frame from state.
- `ratatui` layout provides constraint-based layout and `Flex` alignment, but not a full CSS FlexBox/Grid component system.
- `tui-realm` is a ratatui-based framework inspired by Elm and React. Its 4.1.0 release was published on 2026-05-02 and shows it is still maintained.
- `iocraft` provides a React/SwiftUI-like declarative syntax and flexbox layouts powered by `taffy`, but it is a different TUI framework, not a thin ratatui layer.
- `taffy` implements CSS Block, Flexbox, and CSS Grid layout algorithms and is suitable if this project needs a real FlexBox/Grid solver instead of ratatui's simpler constraints.

References:

- Ratatui rendering: https://ratatui.rs/concepts/rendering/
- Ratatui component architecture: https://ratatui.rs/concepts/application-patterns/component-architecture/
- Ratatui layout/flex: https://ratatui.rs/concepts/layout/
- Ratatui FAQ on library vs framework: https://ratatui.rs/faq/
- tui-realm v4.1.0 release: https://github.com/veeso/tui-realm/releases/tag/v4.1.0
- tui-realm project: https://github.com/veeso/tui-realm
- iocraft: https://github.com/ccbrown/iocraft
- taffy: https://github.com/DioxusLabs/taffy

### 1.5 Framework Decision

Do not adopt a full external framework in the first refactor.

Reasons:

- The current TUI is already tightly integrated with custom `ActionRegistry`, shell commands, sudo modal flow, traffic collector controls, route-risk UX, and mouse hitboxes.
- A framework migration would change event loop, component lifecycle, focus model, and state ownership all at once.
- The immediate problem is not ratatui itself. The problem is that project code has no stable UI architecture boundary.
- A small internal UI kit can be introduced incrementally while preserving visible behavior.

External framework usage is allowed only after a spike proves it reduces total project code and does not weaken auditability. Such a spike must be isolated in its own worktree or branch and must compare:

- event routing,
- mouse hitbox behavior,
- modal focus behavior,
- scroll behavior,
- code reduction in the migrated vertical slice,
- dependency risk,
- binary size and build complexity,
- ability to preserve current CLI/API action coverage.

Adoption threshold:

- The vertical slice should remove more code than it adds after old code is deleted.
- The framework must not force command execution, route-risk handling, sudo prompts, or action registry metadata into hidden framework callbacks.
- A dependency that significantly increases compile time, binary size, or maintenance risk requires an explicit follow-up decision before adoption.

## 2. Current Code State Analysis

### 2.1 Current File Scale

Current measured TUI file sizes:

```text
tui/src/app.rs              8823 physical lines
tui/src/action_registry.rs  1795 lines
tui/src/main.rs              535 lines
tui/src/mouse.rs             470 lines
```

Line counts should be measured with physical line count, for example:

```powershell
(Get-Content tui/src/app.rs).Count
```

Current dependency baseline:

```toml
ratatui = "0.29"
crossterm = "0.28"
```

The implementation works functionally, but the architecture is not sustainable. The largest issue is not one bad function; it is that `app.rs` owns too many responsibilities.

### 2.2 Responsibilities Currently Collapsed Into `app.rs`

`app.rs` currently mixes:

- application state,
- UI state,
- action execution,
- async result application,
- command output formatting,
- page rendering,
- layout calculation,
- popup rendering,
- hitbox registration,
- mouse dispatch,
- scroll dispatch,
- form state,
- command palette behavior,
- traffic chart rendering,
- settings prompt logic,
- subscription form logic,
- sudo prompt logic,
- tests.

This creates several failure modes:

- Any visual change risks event routing regressions.
- Any action change risks layout or modal regressions.
- Mouse hitboxes can drift from visual rendering.
- Layout code becomes page-specific instead of reusable.
- Test coverage becomes broad but hard to localize.
- Code review is expensive because unrelated concerns are interleaved.
- Adding a new component encourages copy-paste instead of composition.

### 2.3 Current Positive Assets To Preserve

The refactor should preserve these assets:

- `action_registry.rs` as the authoritative action list.
- Existing CLI/API action coverage tests.
- Existing i18n coverage model.
- Existing route-risk and sudo modal UX.
- Current visible page ordering and page design unless separately changed.
- Current mouse support requirement.
- Current traffic chart and persistence feature direction.
- Existing `widgets/` primitives only if they are explicitly classified as keep, migrate, or delete.

### 2.4 Current Anti-Patterns To Remove

The refactor must remove these patterns:

- page-specific action button rendering,
- page-specific hitbox math,
- direct `Layout::vertical/horizontal` chains scattered in every page,
- modal rendering embedded in root `app.rs`,
- form rendering duplicated between subscription/settings/sudo prompts,
- command output areas implemented differently per page,
- title-bottom hint text used as ad-hoc UI,
- multiple sources of truth for action labels, shortcut hints, and mouse behavior,
- giant page render functions that mix data preparation, layout, rendering, and hitbox registration.

## 3. Target Architecture

### 3.1 Module Layout

Target module tree:

```text
tui/src/
  app.rs                    // AppModel wiring only; no page rendering
  update.rs                 // DataEvent, Action, command result application
  main.rs                   // terminal loop and top-level event bridge only
  action_registry.rs        // authoritative action metadata
  i18n.rs
  api.rs
  settings.rs
  theme.rs
  mouse.rs                  // low-level mouse event types only

  ui/
    mod.rs
    app_shell.rs            // root shell composition
    model.rs                // UiState, ModalState, FocusPath, PageState
    action.rs               // UiAction, ActionDispatchResult
    component.rs            // Component trait, RenderCtx, EventCtx
    node.rs                 // declarative Node tree if used
    layout.rs               // Stack/Flex/Grid/Slot DSL
    hitbox.rs               // ComponentId -> Rect -> UiAction
    focus.rs                // focus manager
    style.rs                // project semantic styles
    text.rs                 // truncation, width, wrapping

  ui/components/
    mod.rs
    panel.rs
    button.rs
    action_bar.rs
    data_table.rs
    scroll_area.rs
    form.rs
    modal.rs
    nav.rs
    status_bar.rs
    command_output.rs
    chart.rs
    key_hint.rs

  ui/pages/
    mod.rs
    subscriptions.rs
    proxies.rs
    connections.rs
    traffic.rs
    network.rs
    logs.rs
    settings.rs
    help.rs

  ui/modals/
    mod.rs
    confirm.rs
    sudo.rs
    settings_form.rs
    subscription_form.rs
    node_picker.rs
    command_palette.rs
```

The current `tui/src/widgets/` directory is transitional. It must not become a second permanent component system. Each existing widget must be migrated into `ui/components/`, merged into a richer component, or deleted.

### 3.2 State Split

Use a hard split between business model and UI model.

```text
AppModel
  subscriptions
  proxies
  connections
  logs
  traffic
  config
  kernel/runtime status
  async command status

UiState
  active page
  page states
  focused component
  modal stack
  last layout/hitboxes
  user visual settings

PageState
  selected rows
  scroll offsets
  chart viewport
  current form field
  local filters/sort mode
```

Rules:

- Business data does not know about `Rect`.
- Rendering does not spawn commands.
- Components emit `UiAction`; update code decides what that means.
- Page state is explicit and typed.
- No loose `usize` fields remain in root `App` unless they are truly global.
- Stateful components receive state from `UiState` or page state. They do not hide retained state inside ad-hoc component structs.

### 3.3 Component Contract

Initial internal component trait:

```rust
pub trait Component {
    fn id(&self) -> ComponentId;
    fn layout(&self, cx: &mut LayoutCtx) -> LayoutNode;
    fn render(&self, area: Rect, cx: &mut RenderCtx);
    fn event(&self, event: UiEvent, cx: &mut EventCtx) -> EventResult;
}
```

This trait may be simplified during implementation. The invariant is more important than the exact signature:

- layout and render must be separate,
- event routing must use component identity,
- hitbox registration must live with the component that draws the interactive element,
- retained state must be typed and owned by `UiState` or page state, not hidden in render helpers.
- components may be simple structs carrying props; they should not become mini application containers.

Stateful component examples:

```rust
ScrollArea {
    id: ComponentId,
    state_key: ScrollStateKey,
    child: Box<dyn Renderable>,
}

DataTable {
    id: ComponentId,
    selection_key: SelectionStateKey,
    rows: Vec<TableRow>,
}
```

The state key points to typed state owned outside the component. This keeps retention explicit and makes it possible to inspect or reset UI state from page logic.

### 3.4 Retained Mode Definition For This Project

This project will use retained UI state, not retained ratatui widgets.

Retained:

- selected row,
- scroll offset,
- focus path,
- active modal,
- current form values,
- component ids,
- last layout tree,
- hitbox map,
- chart viewport,
- table sort/filter mode.

Not retained:

- `ratatui::widgets::Block`,
- `ratatui::widgets::Table`,
- raw `Buffer`,
- per-frame `Line` or `Span` lists unless cached for a measured reason.

### 3.5 Declarative Layout DSL

Breakpoints are part of the foundation, not a late-stage detail.

Initial breakpoint constants:

```text
narrow: < 100 columns
wide:   >= 100 columns
short:  < 24 rows
tall:   >= 36 rows
```

Minimum internal layout API:

```rust
Stack::vertical()
    .gap(1)
    .child(Length(3), header)
    .child(Fill(1), content)
    .child(Length(1), footer)

Flex::row()
    .gap(1)
    .when(width >= 100, |row| row.child(Length(18), nav))
    .child(Fill(1), main)

Grid::new()
    .columns([Fill(2), Fill(1)])
    .rows([Length(7), Fill(1)])
```

Implementation phases:

1. Wrap ratatui `Layout` with a project DSL.
2. Add breakpoint helpers and named slots.
3. Add simple layout helpers that can be visually compared in the worktree.
4. Only introduce `taffy` after a spike proves ratatui constraints are insufficient.

The first goal is not full CSS parity. The first goal is to remove ad-hoc rectangle math from page render functions.

### 3.6 Component Composition Model

Example target page shape:

```text
NetworkPage
  Panel("Kernel")
    StatusLine
  Row
    Panel("Traffic")
    Panel("Current Proxy")
  ActionBar(network_actions)
  Row
    Column
      Panel("Connections")
      Panel("System")
    CommandOutput(network_output)
```

The page module should read like a declarative description of the interface. It should not manually calculate individual button x coordinates.

### 3.7 Event And Hitbox Model

All clickable or scrollable UI elements must register hitboxes through a shared component API.

```text
Component render
  -> Button renders itself
  -> Button registers ComponentId + Rect + UiAction
  -> EventRouter receives mouse event
  -> HitboxRegistry resolves action
  -> Update layer applies action
```

Rules:

- The function that draws an interactive thing owns its hitbox.
- Visual width and hitbox width must come from the same calculation.
- No page may manually duplicate button width math.
- Modals own event priority. If a modal is open, underlying page hitboxes are ignored.
- Scroll areas must register their own scroll hitboxes.

### 3.8 Text And I18n Model

Reusable components must not hard-code display strings.

Use a small text abstraction:

```rust
pub enum UiText {
    Msg(Msg),
    ActionLabel(&'static str),
    ActionButton(&'static str),
    Static(&'static str),
    Owned(String),
}
```

Resolution happens through `RenderCtx`, which has access to language settings and action registry helpers.

Rules:

- `Button` should usually receive an action id or `UiText::ActionButton`, not a pretranslated raw label.
- `Panel` titles should usually be `UiText::Msg`.
- Page-specific dynamic values may use `Owned(String)`.
- Components must not call global translation functions directly unless passed through `RenderCtx`.
- Action labels, button labels, shortcut text, and command palette entries remain registry-driven.

### 3.9 Existing `widgets/` Migration Map

Current widgets must be handled explicitly:

```text
tui/src/widgets/card.rs        -> migrate into ui/components/panel.rs, then delete or leave as shim briefly.
tui/src/widgets/gauge.rs       -> migrate into ui/components/chart.rs or gauge.rs.
tui/src/widgets/sparkline.rs   -> migrate into ui/components/chart.rs.
tui/src/widgets/status_dot.rs  -> migrate into ui/components/status_bar.rs or status.rs.
tui/src/widgets/tab_bar.rs     -> migrate into ui/components/nav.rs.
tui/src/widgets/table.rs       -> migrate into ui/components/data_table.rs.
tui/src/widgets/mod.rs         -> deleted or re-export transitional shims only during migration.
```

Final state should not have both `widgets/` and `ui/components/` as competing component systems. Keeping `widgets/` is allowed only if it becomes a compatibility re-export layer with no independent logic, and even that should be temporary.

## 4. Deterministic Refactor Plan

### 4.1 Worktree-First Workflow

All real refactor work should happen in a separate git worktree. The current checkout remains the visual and behavioral baseline.

Recommended setup from the current repository:

```powershell
git fetch origin
git worktree add ..\clash-for-linux-tui-refactor -b codex/tui-component-refactor origin/codex/harden-clash-linux
```

If the branch already exists:

```powershell
git worktree add ..\clash-for-linux-tui-refactor codex/tui-component-refactor
```

Workflow rules:

- Keep the original checkout available for comparison.
- Do not perform destructive architecture work in the only checkout.
- Use the refactor worktree for all module moves, page migrations, and deletion.
- Compare against the baseline worktree visually and with `git diff` when needed.
- A failed phase can be abandoned by removing the worktree and branch.
- A successful phase can be squashed or merged back intentionally.

Useful commands:

```powershell
git worktree list
git -C ..\clash-for-linux-tui-refactor status -sb
git -C ..\clash-for-linux-tui-refactor diff --stat origin/codex/harden-clash-linux...HEAD
```

The point of this workflow is not bureaucracy. It is to make architecture refactoring reversible and comparable while the current TUI remains usable.

### Phase 0: Establish Refactor Worktree And Baseline

Goal: create a safe place to refactor without damaging the baseline checkout.

Tasks:

- Create the worktree.
- Open the current TUI once in the baseline checkout and once in the refactor worktree when needed.
- Record only the observations that matter for refactor quality:
  - page layout still recognizable,
  - mouse clicks still hit visible buttons,
  - modals still block underlying pages,
  - `app.rs` line count is decreasing after page migrations.

No dedicated screenshot infrastructure, golden tests, or snapshot framework is required.

Exit criteria:

- Refactor worktree exists.
- Baseline checkout remains untouched.
- The branch purpose is clear.

### Phase 1: State, Action, Layout, And Text Foundation

Goal: create the foundations that stateful components need before those components are migrated.

Create:

- `ui/model.rs`
- `ui/action.rs`
- `ui/component.rs`
- `ui/layout.rs`
- `ui/hitbox.rs`
- `ui/focus.rs`
- `ui/style.rs`
- `ui/text.rs`

Define:

- `UiState`
- `PageStates`
- `ModalState`
- `FocusPath`
- `ComponentId`
- `UiAction`
- `UiText`
- `Breakpoint`
- `LayoutCtx`
- `RenderCtx`
- `EventCtx`

Rules:

- This phase may increase total code temporarily.
- This phase should not attempt to migrate all pages.
- This phase must make state ownership explicit before `ScrollArea`, `Form`, `DataTable`, `Modal`, or `Chart` are migrated.
- Breakpoints are defined here, not delayed until a later layout phase.
- i18n resolution strategy is defined here through `UiText` and `RenderCtx`.

Exit criteria:

- New core modules exist.
- `UiState` has typed page state containers, even if not every page uses them yet.
- Event and render contexts exist.
- Old code still runs.

### Phase 2: Stateless Visual Components And Existing Widget Classification

Goal: remove obvious rendering duplication without touching retained behavior first.

Implement:

- `Button`
- `ActionBar`
- `Panel`
- `Nav`
- `StatusBar`
- `KeyHint`

Classify and migrate existing `widgets/`:

- `card.rs` -> `Panel`
- `tab_bar.rs` -> `Nav`
- `status_dot.rs` -> `StatusBar` or status component
- `gauge.rs` -> later chart/gauge component
- `sparkline.rs` -> later chart component
- `table.rs` -> later `DataTable`

Rules:

- Stateless components can be adopted before the full state split.
- If a component draws an interactive element, it owns the hitbox.
- Delete old helpers as soon as an equivalent component replaces them.
- Do not leave `widgets/` and `ui/components/` as two permanent systems.

Exit criteria:

- Common button/action/panel/nav rendering no longer lives in page functions.
- Existing clean widgets have a declared migration target.
- `app.rs` starts losing duplicated rendering helpers.

### Phase 3: Stateful Components, Modal Stack, And Event Router

Goal: make retained interactions explicit before migrating complex pages.

Implement:

- `ScrollState`
- `SelectionState`
- `FormState`
- `ModalStack`
- `CommandOutputState`
- `ChartViewport`
- `EventRouter`

Then implement components that depend on retained state:

- `ScrollArea`
- `CommandOutput`
- `DataTable`
- `Modal`
- `Form`
- `Chart`

Rules:

- Stateful components receive state keys or typed state references.
- Stateful components must not hide their own retained state in render-only structs.
- Modal routing is centralized. Open modals receive keyboard and mouse events before pages.
- Scroll routing is centralized. Scrollable components register scroll areas through the same hitbox path used for clicks.

Exit criteria:

- Modal, form, scroll, table selection, and chart viewport behavior all have typed state owners.
- Existing page code can call these components without duplicating state fields.
- Root `App` starts moving page-local state into `UiState`.

### Phase 4: Page Migration In Vertical Slices

Goal: move pages out of procedural `app.rs` one by one and delete the old code in the same slice.

Migration order:

1. `HelpPage`
2. `LogsPage`
3. `SettingsPage`
4. `NetworkPage`
5. `TrafficPage`
6. `ConnectionsPage`
7. `ProxiesPage`
8. `SubscriptionsPage`

Rationale:

- Help and Logs prove scroll and panel basics.
- Settings and Network prove action bars, forms, command output, sudo modal behavior, and page sections.
- Traffic proves chart state and long visual areas.
- Connections, Proxies, and Subscriptions migrate after table/list/form components are stable.

Per-page method:

1. Define page view model.
2. Define page state type if the page has local retained state.
3. Compose the page from components.
4. Route events through `UiAction`.
5. Delete old page render function from `app.rs`.
6. Delete old page-specific hitbox/layout helpers.
7. Delete obsolete i18n keys and dead helpers.
8. Compare visually with baseline checkout.

Exit criteria per page:

- The old page render function is gone.
- The old page-specific hitbox math is gone.
- The page module owns page composition.
- `app.rs` line count decreases unless a documented temporary move is still in progress.

### Phase 5: App.rs Demolition And Ownership Cleanup

Goal: reduce `app.rs` to orchestration only.

Move out of `app.rs`:

- data event application,
- command execution decisions,
- command output normalization,
- modal state transitions,
- page-local state,
- page rendering,
- form rendering,
- chart rendering,
- table rendering,
- hitbox routing.

Final `app.rs` responsibilities:

- create `AppModel`,
- create `UiState`,
- own runtime/channel handles,
- call update layer,
- call render entry point,
- coordinate top-level refresh.

Exit criteria:

- `app.rs` is no longer the place where pages are implemented.
- Adding a page or component does not require editing thousands of lines in `app.rs`.
- The file is small enough to review in one pass.

### Phase 6: Optional Layout Engine Or Framework Spike

Goal: decide whether a bigger dependency is worth it after the internal architecture exists.

Allowed candidates:

- direct `taffy` layout adapter,
- `tui-realm`,
- `iocraft`.

The spike must happen in a separate worktree or branch and must implement one vertical slice:

- one page shell,
- one action bar,
- one modal/form,
- one scroll area,
- one table/list or chart.

Decision criteria:

- The slice deletes more project code than it adds.
- The resulting code is easier to read than the internal UI kit.
- Mouse routing, modal priority, and action registry integration stay explicit.
- Dependency cost is justified by removed complexity.

If the spike fails, discard the worktree. Do not merge a framework experiment that makes the architecture harder to understand.

## 5. Old Code Cleanup Method

### 5.1 Deletion Policy

Old code should be deleted aggressively once replaced.

No compatibility layer is required for old internal render functions. The project only needs to preserve external user behavior and CLI/TUI features, not old private Rust function boundaries.

Rules:

- Do not keep both old and new page renderers.
- Do not keep unused helpers "just in case".
- Do not keep old hitbox code after component-owned hitboxes are live.
- Do not keep duplicate i18n keys after text moves.
- Do not keep old tests that assert obsolete implementation details.
- Do not keep dead action ids.

### 5.2 Cleanup Checklist Per Migration

For each migrated page or component:

- Delete old render function.
- Delete old layout helper.
- Delete old hitbox helper.
- Delete old ad-hoc text formatter if the new component handles it.
- Delete old page-local action button code.
- Delete obsolete tests.
- Run `rg` for old function names.
- Run lightweight checks when practical.
- Prefer deleting old code over preserving obsolete tests.

### 5.3 Root `app.rs` Shrink Targets

Line-count targets:

```text
Current app.rs: about 8823 physical lines
After Phase 1: app.rs may stay similar or grow slightly while foundations land
After Phase 2: <= 7800 lines
After Phase 3: <= 6500 lines
After four migrated pages: <= 4500 lines
After all pages migrate: <= 1800 lines
Final target:  <= 1200 lines
```

These targets are not production metrics. They exist to prevent the refactor from becoming another layer on top of the same monolithic `app.rs`. If a phase adds abstractions but does not eventually delete old `app.rs` code, the phase is incomplete.

Final `app.rs` should contain:

- root app construction,
- model ownership,
- top-level refresh orchestration,
- references to update/render entry points.

It should not contain:

- page rendering,
- modal rendering,
- button drawing,
- table drawing,
- page-specific layout,
- page-specific mouse hitbox registration.

## 6. Code Quality Optimization Principles

### 6.1 Smallest Correct Abstraction

Abstractions must reduce total complexity. They are justified only when they:

- remove real duplication,
- make hitboxes and rendering share one source of truth,
- make responsive layout easier to reason about,
- isolate state ownership,
- make event routing predictable,
- reduce page code size,
- improve auditability.

Do not introduce abstractions for naming aesthetics alone.

### 6.2 Composition Over Inheritance

Use composition:

```text
Panel(ActionBar(Button...))
Panel(ScrollArea(CommandOutput))
Modal(Form(...))
```

Do not create deep component inheritance or trait hierarchies. Rust traits should define narrow contracts, not simulate an object-oriented GUI framework.

### 6.3 Data First, Rendering Second

Each page should first build a view model:

```rust
NetworkViewModel {
    kernel,
    traffic,
    current_proxy,
    actions,
    command_output,
}
```

Rendering consumes the view model. Rendering should not query raw app state repeatedly throughout the function.

### 6.4 One Source Of Truth For Actions

Action metadata must remain registry-driven:

- labels,
- i18n,
- shortcuts,
- mouse text,
- danger level,
- CLI/API executor.

Buttons and command palette must derive from the same action metadata. Page-local labels are forbidden unless the action is purely local and registered as such.

### 6.5 One Source Of Truth For Hitboxes

The component that draws the visible interactive element owns the hitbox.

Bad:

```text
render button in page A
calculate hitbox in page A helper
dispatch action in root mouse match
```

Good:

```text
Button renders visual span
Button registers hitbox from same measured width
EventRouter emits UiAction
Update applies UiAction
```

### 6.6 Prefer Typed State Over Loose Fields

Bad:

```rust
settings_section: usize
traffic_selected_idx: usize
network_output_scroll: usize
```

Better:

```rust
SettingsPageState {
    section: SettingsSection,
}

TrafficPageState {
    selected_row: RowIndex,
    viewport: TrafficViewport,
}

ScrollState {
    offset: usize,
}
```

### 6.7 Reduce Code By Removing Special Cases

If two pages need similar bottom action buttons, build one `ActionBar`.

If two modals need Save/Cancel, build one `ModalFooter`.

If two pages need scrollable output, build one `ScrollArea<CommandOutput>`.

If two tables need selection and wheel behavior, build one `DataTable`.

### 6.8 Avoid Premature Genericity

Start concrete, then generalize.

Allowed first version:

```rust
ActionBar::new(actions).render(...)
```

Avoid first-version complexity:

```rust
trait IntoReactiveActionBarNodeWithLifecycleHooks<'a, M, E>
```

Generic abstractions are allowed only after at least two concrete components prove the same shape.

### 6.9 Prefer Structural Clarity Over Test Infrastructure

This project is not currently treated as a production system, so a heavy TUI testing stack is not required for the refactor.

Useful lightweight checks:

- compile the TUI when practical,
- run existing tests when they are cheap,
- compare the refactor worktree visually with the baseline worktree,
- manually check mouse click and scroll behavior for migrated pages,
- inspect `app.rs` line count and deleted old functions.

Do not build elaborate snapshot or golden-image infrastructure just to start the refactor. The main success criterion is architectural: fewer responsibilities in `app.rs`, less duplication, clearer state ownership, and components that own their rendering/hitbox behavior.

## 7. Refactor Process Rules

### 7.1 Plan First

No major refactor phase starts without:

- a written phase plan,
- affected files,
- exact old code to delete,
- migration order,
- optional verification commands,
- rollback criteria.

Implementation speed is secondary. A fast patch that worsens architecture is not acceptable.

### 7.2 Design Before Patch

If implementation reveals a missing concept, stop and update the design.

Do not patch around missing architecture with:

- more boolean flags,
- more page-specific `if area.width`,
- more duplicate hitbox math,
- more ad-hoc modal branches,
- more helper functions in `app.rs`.

When a missing abstraction is discovered, choose one:

- extend the planned abstraction,
- split the phase,
- revert the partial implementation and redesign.

### 7.3 Refactor In Vertical Slices

Each slice should leave the app compiling when practical. It does not need to introduce new test infrastructure.

A good slice:

```text
Introduce ActionBar -> migrate Network actions -> delete old Network action rendering -> compile or visually check.
```

A bad slice:

```text
Create many modules -> half-migrate all pages -> keep old and new paths -> fix compile later.
```

### 7.4 Delete Old Code In The Same Slice

If new code replaces old behavior, the old code must be deleted before the slice is complete.

Temporary duplication is allowed only inside one local phase and must be listed in the phase plan. It cannot be pushed as a final state unless the document explicitly records the follow-up deletion task.

### 7.5 No Internal Backward Compatibility Requirement

Private Rust APIs inside `tui/src` do not require backward compatibility.

Allowed:

- renaming modules,
- deleting old helper functions,
- changing component contracts,
- changing internal state shape,
- replacing event routing internals,
- replacing layout primitives.

Not allowed without a user-facing design decision:

- removing TUI features,
- reducing mouse support,
- reducing CLI coverage,
- reducing i18n coverage,
- weakening route-risk/sudo safety behavior.

### 7.6 Lightweight Verification

Recommended checks:

```bash
cargo check
cargo fmt --check
cargo test
cargo clippy -- -D warnings
git diff --check
```

These checks are recommended, not the purpose of the refactor. If a check becomes noisy because old tests assert obsolete internals, update or delete those tests instead of preserving the old architecture.

If Go-facing behavior is touched, these remain useful:

```bash
go test ./...
go vet ./...
```

If visual layout is touched, a manual worktree comparison is enough:

- open the same page in the baseline checkout,
- open the migrated page in the refactor worktree,
- compare layout and core interactions,
- verify that old code was deleted.

### 7.7 Stop Conditions

Stop and redesign if any of these happen:

- a new component requires page-specific geometry hacks,
- modal priority becomes unclear,
- hitboxes are calculated away from the visible component,
- page code grows after migration,
- a phase adds new abstractions but does not delete old `app.rs` code,
- a component needs access to unrelated global state,
- an abstraction needs many optional fields to support first use.

## 8. Final Target

The final TUI code should have this shape:

```text
main.rs
  terminal/event loop only

app.rs
  AppModel + UiState ownership
  top-level orchestration

update.rs
  state transitions
  command execution decisions

ui/app_shell.rs
  root shell layout

ui/pages/*.rs
  declarative page composition

ui/components/*.rs
  reusable visual and interactive building blocks

ui/layout.rs
  project layout DSL

ui/hitbox.rs
  deterministic hitbox registry
```

The desired end state is not "more framework code". The desired end state is less page code, fewer special cases, deterministic mouse behavior, stable modal behavior, and a TUI codebase that can keep adding features without turning `app.rs` into a larger procedural file.
