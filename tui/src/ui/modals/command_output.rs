use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::i18n::Msg;
use crate::mouse::HitboxAction;

pub(crate) fn render_command_output_window(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some((page, output)) = active_output(app) else {
        return;
    };
    if output.is_empty() || area.width < 48 || area.height < 12 {
        return;
    }

    let popup = output_window_area(area);
    if popup.width == 0 || popup.height == 0 {
        return;
    }
    clear_floating_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg))
        .style(Style::default().bg(CLASH_THEME.bg))
        .title(Span::styled(
            format!(" {} · {} ", app.t(Msg::NetworkCommandOutput), page),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::CommandOutputScrollHint)),
            Style::default().fg(CLASH_THEME.muted).bg(CLASH_THEME.bg),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(block, popup);
    fill_area(frame, inner, CLASH_THEME.bg);

    let visible_height = inner.height as usize;
    let max_scroll = output.len().saturating_sub(visible_height);
    app.ui_state.command_output.scroll = app.ui_state.command_output.scroll.min(max_scroll);
    let lines = output
        .iter()
        .skip(app.ui_state.command_output.scroll)
        .take(visible_height)
        .map(|line| {
            Line::from(Span::styled(
                trunc_str(line, inner.width as usize),
                CLASH_THEME.text,
            ))
        })
        .collect::<Vec<_>>();

    app.ui_state
        .hitboxes
        .register(popup, HitboxAction::ScrollCommandOutput);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

fn active_output(app: &App) -> Option<(&'static str, Vec<String>)> {
    match app.ui_state.active_page {
        Tab::Subscriptions => Some((
            app.t(Msg::PageSubscriptions),
            app.ui_state.subscriptions.output.clone(),
        )),
        Tab::Traffic => Some((app.t(Msg::PageTraffic), app.ui_state.traffic.output.clone())),
        Tab::Network => Some((app.t(Msg::PageNetwork), app.network_output.clone())),
        Tab::Settings => Some((
            app.t(Msg::PageSettings),
            app.ui_state.settings.output.clone(),
        )),
        _ => None,
    }
}

fn output_window_area(area: Rect) -> Rect {
    let width = if area.width >= 120 {
        (area.width * 3 / 5)
            .max(72)
            .min(area.width.saturating_sub(4))
    } else {
        area.width.saturating_sub(2)
    };
    let height = (area.height / 3)
        .max(7)
        .min(14)
        .min(area.height.saturating_sub(2));
    Rect::new(
        area.x
            .saturating_add(area.width)
            .saturating_sub(width)
            .saturating_sub(1),
        area.y
            .saturating_add(area.height)
            .saturating_sub(height)
            .saturating_sub(1),
        width,
        height,
    )
}
