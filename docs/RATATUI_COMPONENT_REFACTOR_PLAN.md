# Ratatui Component Refactor Plan

Status: deterministic architecture plan
Date: 2026-07-02

This document is the governing plan for refactoring the current TUI implementation from a large procedural `app.rs` into a component-oriented, retained-state, declarative-layout architecture.

This plan intentionally prioritizes correctness of architecture over short-term edit speed. No large TUI refactor should begin without first updating this document or a phase-specific implementation plan derived from it.

## 1. Refactor Direction And Feasibility

### 1.1 Decision

The project should keep `ratatui` as the rendering backend and build an internal component and layout layer above it.

The target architecture is:

- `ratatui` remains the low-level immediate-mode renderer.
- Project-owned components own retained UI state, focus, scroll, modal stack, and hitboxes.
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

External framework usage is allowed only after a spike proves it reduces total project code and does not weaken auditability. Such a spike must be isolated from mainline and must compare:

- event routing,
- mouse hitbox behavior,
- modal focus behavior,
- scroll behavior,
- testability,
- dependency risk,
- binary size and build complexity,
- ability to preserve current CLI/API action coverage.

## 2. Current Code State Analysis

### 2.1 Current File Scale

Current measured TUI file sizes:

```text
tui/src/app.rs              8193 lines
tui/src/action_registry.rs  1795 lines
tui/src/main.rs              535 lines
tui/src/mouse.rs             470 lines
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
- Existing `widgets/` primitives where they are already clean.

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
  page-local selected rows
  page-local scroll offsets
  focused component
  modal stack
  current form field
  last layout/hitboxes
  user visual settings
```

Rules:

- Business data does not know about `Rect`.
- Rendering does not spawn commands.
- Components emit `UiAction`; update code decides what that means.
- Page state is explicit and typed. No loose `usize` fields in root `App` unless they are truly global.

### 3.3 Component Contract

Initial internal component trait:

```rust
pub trait Component {
    type Props;
    type State;

    fn id(&self) -> ComponentId;
    fn layout(&self, props: &Self::Props, cx: &mut LayoutCtx) -> LayoutNode;
    fn render(&self, props: &Self::Props, state: &Self::State, area: Rect, cx: &mut RenderCtx);
    fn event(&self, state: &mut Self::State, event: UiEvent, cx: &mut EventCtx) -> EventResult;
}
```

This trait may be simplified during implementation. The invariant is more important than the exact signature:

- layout and render must be separate,
- event routing must use component identity,
- hitbox registration must live with the component that draws the interactive element,
- local retained state must be typed and owned.

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
3. Add testable layout snapshots.
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

## 4. Deterministic Refactor Plan

### Phase 0: Freeze Baseline

Goal: establish a measurable baseline before architecture movement.

Tasks:

- Record current screenshot set for all pages at 80x24, 120x36, 180x48.
- Record current mouse smoke scenarios:
  - sidebar/top nav click,
  - action button click,
  - modal button click,
  - scroll command output,
  - scroll help,
  - scroll traffic chart/table.
- Record current test commands:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy -- -D warnings`
  - `go test ./...`
  - `go vet ./...`
  - `git diff --check`
- Add this document to the refactor checklist.

Exit criteria:

- Baseline screenshots and smoke actions are documented.
- No refactor branch starts with unknown current behavior.

### Phase 1: Extract Core UI Infrastructure

Goal: create the reusable core without migrating pages.

Create:

- `ui/component.rs`
- `ui/layout.rs`
- `ui/hitbox.rs`
- `ui/focus.rs`
- `ui/style.rs`
- `ui/text.rs`

Move or duplicate temporarily:

- display width helpers,
- truncation helpers,
- action button width calculation,
- hitbox registration helpers,
- semantic styles.

Rules:

- New infrastructure must have unit tests before page migration uses it.
- No page behavior changes in this phase unless required to compile.
- Existing `app.rs` may call the new helpers, but page functions stay where they are until Phase 2.

Exit criteria:

- Shared button width and hitbox tests pass.
- Shared layout split tests pass.
- `app.rs` line count starts decreasing only after helpers are used.

### Phase 2: Componentize Primitive Controls

Goal: remove duplicated rendering patterns.

Implement:

- `Button`
- `ActionBar`
- `Panel`
- `ScrollArea`
- `CommandOutput`
- `DataTable`
- `Modal`
- `Form`
- `Nav`
- `StatusBar`

Migration order:

1. `Button`
2. `ActionBar`
3. `Panel`
4. `ScrollArea`
5. `Modal`
6. `Form`
7. `DataTable`
8. `Chart`

Rules:

- Each component owns rendering and hitboxes.
- Each component has a small test for layout/hitbox behavior.
- Replace old helpers immediately after equivalent component is adopted.

Exit criteria:

- No page-local button rendering remains.
- No page-local action button hitbox math remains.
- Modal buttons share one implementation.
- Scroll areas share one implementation.

### Phase 3: Split App State And Update Logic

Goal: make rendering independent from command execution and async result application.

Create:

- `ui/model.rs` for `UiState`, page states, modal states.
- `update.rs` for data event application and command dispatch.
- `ui/action.rs` for UI-level actions.

Move from `app.rs`:

- data event application,
- command result normalization,
- sudo retry state,
- confirmation state,
- page-local selection and scroll fields,
- command palette state,
- form state.

Rules:

- `App` becomes a container of `AppModel`, `UiState`, API client, runtime handle, and channels.
- Rendering functions cannot spawn tasks.
- Event handling cannot directly call shell commands; it emits actions.

Exit criteria:

- `app.rs` no longer contains page render functions.
- `app.rs` no longer contains low-level widget layout.
- async result handling is tested independently from rendering.

### Phase 4: Migrate Pages One By One

Goal: move pages from procedural rendering into declarative page modules.

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

- Help and Logs are simplest and prove scroll behavior.
- Settings and Network stress action bars, command output, forms, sudo, and modal rules.
- Traffic stresses chart layout and retained viewport state.
- Proxies and Subscriptions are interaction-heavy and should migrate after primitives are stable.

Per-page method:

1. Write a page-specific design sketch in comments or a phase note.
2. Define page state type.
3. Define page props/view model.
4. Implement page composition using components.
5. Route events through `UiAction`.
6. Delete old page render function.
7. Delete old page-specific helper functions.
8. Run tests.

Exit criteria per page:

- Old render function deleted.
- Old hitbox math deleted.
- Page tests cover layout, action hitboxes, and primary keyboard/mouse actions.
- Screenshots match or intentionally improve baseline.

### Phase 5: Replace Layout Internals With Constraint DSL

Goal: make responsive layout declarative and testable.

Tasks:

- Add named slots for common page patterns:
  - `ShellLayout`
  - `TwoColumnLayout`
  - `SummaryActionsLayout`
  - `ChartWithSidebarLayout`
  - `FormModalLayout`
- Add breakpoint definitions:
  - narrow: `< 100 columns`
  - wide: `>= 100 columns`
  - short: `< 24 rows`
  - tall: `>= 36 rows`
- Add snapshot tests for slot rectangles.

Only after this, evaluate `taffy`.

Taffy adoption criteria:

- Current DSL cannot express required wrapping or responsive behavior without complex manual code.
- The taffy adapter can return deterministic `Rect` trees for terminal cell sizes.
- It does not make simple page layout harder to read.
- It has tests for width/height rounding and terminal cell constraints.

Exit criteria:

- Page modules no longer call raw `Layout::vertical/horizontal` except inside layout infrastructure.
- Layout snapshots cover all major page templates.

### Phase 6: Optional Framework Spike

Goal: decide whether external framework adoption is worth it.

Allowed candidates:

- `tui-realm`
- `iocraft`
- a direct `taffy` layout adapter

The spike must not touch mainline page code. It must implement one vertical slice:

- Settings page shell,
- one action bar,
- one form modal,
- one scroll area,
- one table/list.

Decision criteria:

- Less code than internal UI kit.
- Equal or better testability.
- Equal or better mouse support.
- No loss of action registry auditability.
- No loss of modal/focus predictability.
- No dependency fragility.

If the spike fails any criterion, do not adopt the framework.

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
- Add or update behavior tests.
- Run `rg` for old function names.
- Run `cargo clippy -- -D warnings`.
- Run `git diff --check`.

### 5.3 Root `app.rs` Shrink Targets

Line-count targets:

```text
Current app.rs: about 8193 lines
After Phase 2: <= 6500 lines
After Phase 3: <= 4500 lines
After Phase 4: <= 1800 lines
Final target:  <= 1200 lines
```

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
- make responsive layout testable,
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

### 6.9 Test The Architecture, Not Just Functions

Required test categories:

- layout rectangle tests,
- hitbox visual-width tests,
- modal event priority tests,
- scroll dispatch tests,
- action registry coverage tests,
- page smoke render tests,
- command result update tests.

Tests should fail when visual and hitbox geometry diverge.

## 7. Refactor Process Rules

### 7.1 Plan First

No major refactor phase starts without:

- a written phase plan,
- affected files,
- exact old code to delete,
- migration order,
- verification commands,
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

Each slice must leave the app compiling and tested.

A good slice:

```text
Introduce ActionBar -> migrate Network actions -> delete old Network action rendering -> test.
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

### 7.6 Verification Gates

Every refactor slice must run at minimum:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
git diff --check
```

If Go-facing behavior is touched:

```bash
go test ./...
go vet ./...
```

If visual layout is touched:

- capture screenshots for narrow/wide terminal sizes,
- test mouse clicks in tmux if the change affects hitboxes,
- test scroll behavior for every changed scroll area.

### 7.7 Stop Conditions

Stop and redesign if any of these happen:

- a new component requires page-specific geometry hacks,
- modal priority becomes unclear,
- hitboxes are calculated away from the visible component,
- page code grows after migration,
- tests need large brittle snapshots to pass,
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

