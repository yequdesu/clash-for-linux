use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::action_registry::{self, ActionDanger};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

pub(crate) fn render_help(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg))
        .style(Style::default().bg(CLASH_THEME.bg))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::HelpTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.bg);
    app.ui_state
        .hitboxes
        .register(inner, HitboxAction::ScrollHelp);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {:<16}", app.t(Msg::HelpKey)),
                CLASH_THEME.primary,
            ),
            Span::styled(format!("{:<15}", app.t(Msg::HelpPage)), CLASH_THEME.primary),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpAction)),
                CLASH_THEME.primary,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::HelpRisk)), CLASH_THEME.primary),
            Span::styled(app.t(Msg::HelpDescription), CLASH_THEME.primary),
        ]),
        Line::from(vec![
            Span::styled("  Tab/1-8        ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpSwitchTabs)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpSwitchTabs), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  r/q/?           ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpRefreshQuitHelp)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpRefreshQuitHelp), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled("  j/k/↑↓         ", CLASH_THEME.primary),
            Span::styled(
                format!("{:<15}", app.t(Msg::CommonGlobal)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!("{:<31}", app.t(Msg::HelpNavigateLists)),
                CLASH_THEME.text,
            ),
            Span::styled(format!("{:<11}", app.t(Msg::CommonSafe)), CLASH_THEME.text),
            Span::styled(app.t(Msg::HelpNavigateLists), CLASH_THEME.text),
        ]),
    ];

    for spec in action_registry::all_actions() {
        lines.push(action_help_line(app, spec));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(
            "  {}: {} {}, {} {}.",
            app.t(Msg::HelpGenerated),
            action_registry::all_actions().len(),
            app.t(Msg::HelpActionsUnit),
            action_registry::cli_backed_enum_count(),
            app.t(Msg::HelpCliBackedEnumActions)
        ),
        CLASH_THEME.muted,
    )));
    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    let scroll = app.help_scroll.min(max_scroll);
    app.help_scroll = scroll;
    let visible = lines
        .into_iter()
        .skip(scroll)
        .take(inner.height as usize)
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(visible).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

pub(crate) fn action_help_line(app: &App, spec: &action_registry::ActionSpec) -> Line<'static> {
    let page = app.t(spec.page.msg());
    let label = app.action_label(spec);
    let description = app.action_desc(spec);
    let danger_style = match spec.danger {
        ActionDanger::Dangerous => Style::default().fg(CLASH_THEME.danger),
        ActionDanger::Confirm | ActionDanger::Sensitive => Style::default().fg(CLASH_THEME.warning),
        ActionDanger::Safe => Style::default().fg(CLASH_THEME.text),
    };
    Line::from(vec![
        Span::styled(
            format!("  {:<14}", trunc_str(spec.shortcut, 14)),
            CLASH_THEME.primary,
        ),
        Span::styled(format!("{:<15}", trunc_str(page, 14)), CLASH_THEME.muted),
        Span::styled(format!("{:<31}", trunc_str(label, 30)), CLASH_THEME.text),
        Span::styled(
            format!("{:<11}", app.action_danger_label(spec.danger)),
            danger_style,
        ),
        Span::styled(
            trunc_str(
                &format!("{} · {}", description, spec.executor.command()),
                72,
            ),
            CLASH_THEME.text,
        ),
    ])
}
