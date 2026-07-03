use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::i18n::Msg;

use crate::ui::layout::centered_rect;

pub(crate) fn render_command_palette(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(area, 74, 58);
    fill_area(frame, popup, CLASH_THEME.bg);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::CommandPaletteTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::CommandPaletteHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(block, popup);

    let matches = app.command_palette_matches();
    let max_rows = inner.height.saturating_sub(3) as usize;
    let selected = app
        .command_selected_idx
        .min(matches.len().saturating_sub(1));
    let first = selected.saturating_sub(max_rows.saturating_sub(1));

    let mut lines = vec![Line::from(vec![
        Span::styled("> ", CLASH_THEME.primary),
        Span::styled(&app.command_query, CLASH_THEME.text),
        Span::styled(
            format!("  {}", app.t(Msg::CommandPaletteSearchHint)),
            CLASH_THEME.muted,
        ),
    ])];
    lines.push(Line::from(""));

    if matches.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", app.t(Msg::CommandPaletteNoMatch)),
            CLASH_THEME.warning,
        )));
    } else {
        for (row_idx, spec) in matches.iter().skip(first).take(max_rows).enumerate() {
            let idx = first + row_idx;
            let marker = if idx == selected { ">" } else { " " };
            let style = if idx == selected {
                Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
            } else {
                Style::default().fg(CLASH_THEME.text)
            };
            let danger = app.action_danger_label(spec.danger);
            let summary = format!(
                "{} {:<24} {:<11} {:<9} {}",
                marker,
                trunc_str(app.action_label(spec), 23),
                trunc_str(app.t(spec.page.msg()), 10),
                danger,
                trunc_str(&spec.executor.command(), 48)
            );
            lines.push(Line::from(Span::styled(summary, style)));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text)),
        inner,
    );
}
