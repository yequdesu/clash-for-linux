use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::action_registry::{self, ActionDanger};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;
use crate::ui::components::action_bar::display_width;

const HELP_KEY_WIDTH: u16 = 17;
const HELP_PAGE_WIDTH: u16 = 16;
const HELP_ACTION_WIDTH: u16 = 30;
const HELP_RISK_WIDTH: u16 = 11;

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
                help_cell(app.t(Msg::HelpKey), HELP_KEY_WIDTH),
                CLASH_THEME.primary,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpPage), HELP_PAGE_WIDTH),
                CLASH_THEME.primary,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpAction), HELP_ACTION_WIDTH),
                CLASH_THEME.primary,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpRisk), HELP_RISK_WIDTH),
                CLASH_THEME.primary,
            ),
            Span::styled(app.t(Msg::HelpDescription), CLASH_THEME.primary),
        ]),
        Line::from(vec![
            Span::styled(help_cell("Tab/1-8", HELP_KEY_WIDTH), CLASH_THEME.primary),
            Span::styled(
                help_cell(app.t(Msg::CommonGlobal), HELP_PAGE_WIDTH),
                CLASH_THEME.muted,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpSwitchTabs), HELP_ACTION_WIDTH),
                CLASH_THEME.text,
            ),
            Span::styled(
                help_cell(app.t(Msg::CommonSafe), HELP_RISK_WIDTH),
                CLASH_THEME.text,
            ),
            Span::styled(app.t(Msg::HelpSwitchTabs), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled(help_cell("r/q/?", HELP_KEY_WIDTH), CLASH_THEME.primary),
            Span::styled(
                help_cell(app.t(Msg::CommonGlobal), HELP_PAGE_WIDTH),
                CLASH_THEME.muted,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpRefreshQuitHelp), HELP_ACTION_WIDTH),
                CLASH_THEME.text,
            ),
            Span::styled(
                help_cell(app.t(Msg::CommonSafe), HELP_RISK_WIDTH),
                CLASH_THEME.text,
            ),
            Span::styled(app.t(Msg::HelpRefreshQuitHelp), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled(help_cell("j/k/↑↓", HELP_KEY_WIDTH), CLASH_THEME.primary),
            Span::styled(
                help_cell(app.t(Msg::CommonGlobal), HELP_PAGE_WIDTH),
                CLASH_THEME.muted,
            ),
            Span::styled(
                help_cell(app.t(Msg::HelpNavigateLists), HELP_ACTION_WIDTH),
                CLASH_THEME.text,
            ),
            Span::styled(
                help_cell(app.t(Msg::CommonSafe), HELP_RISK_WIDTH),
                CLASH_THEME.text,
            ),
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
    let scroll = app.ui_state.help.scroll.min(max_scroll);
    app.ui_state.help.scroll = scroll;
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
            help_cell(spec.shortcut, HELP_KEY_WIDTH),
            CLASH_THEME.primary,
        ),
        Span::styled(help_cell(page, HELP_PAGE_WIDTH), CLASH_THEME.muted),
        Span::styled(help_cell(label, HELP_ACTION_WIDTH), CLASH_THEME.text),
        Span::styled(
            help_cell(app.action_danger_label(spec.danger), HELP_RISK_WIDTH),
            danger_style,
        ),
        Span::styled(
            fit_help_width(
                &format!("{} · {}", description, spec.executor.command()),
                72,
            ),
            CLASH_THEME.text,
        ),
    ])
}

fn help_cell(value: &str, width: u16) -> String {
    let mut out = fit_help_width(value, width.saturating_sub(1));
    let used = display_width(&out);
    if used < width {
        out.push_str(&" ".repeat((width - used) as usize));
    }
    out
}

fn fit_help_width(value: &str, width: u16) -> String {
    if display_width(value) <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".into();
    }
    let keep_width = width - 1;
    let mut out = String::new();
    for ch in value.chars() {
        let mut next = out.clone();
        next.push(ch);
        if display_width(&next) > keep_width {
            break;
        }
        out = next;
    }
    out.push('…');
    out
}
