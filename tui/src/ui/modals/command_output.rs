use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::i18n::Msg;
use crate::mouse::HitboxAction;
use crate::ui::components::action_bar::{action_button_width, display_width};

pub(crate) fn render_command_output_window(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.ui_state.command_output.hidden {
        return;
    }
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
        .style(Style::default().bg(CLASH_THEME.bg));
    let shell = block.inner(popup);
    frame.render_widget(block, popup);
    fill_area(frame, shell, CLASH_THEME.bg);

    app.ui_state
        .hitboxes
        .register(popup, HitboxAction::ScrollCommandOutput);

    let header = Rect::new(
        shell.x.saturating_add(1),
        shell.y,
        shell.width.saturating_sub(2),
        1,
    );
    let body = Rect::new(
        shell.x.saturating_add(2),
        shell.y.saturating_add(1),
        shell.width.saturating_sub(4),
        shell.height.saturating_sub(2),
    );
    let footer = Rect::new(
        shell.x.saturating_add(1),
        shell.y.saturating_add(shell.height.saturating_sub(1)),
        shell.width.saturating_sub(2),
        1,
    );
    render_header(frame, header, app, page);
    render_footer(frame, footer, app);
    fill_area(frame, body, CLASH_THEME.bg);

    let visible_height = body.height as usize;
    let max_scroll = output.len().saturating_sub(visible_height);
    app.ui_state.command_output.scroll = app.ui_state.command_output.scroll.min(max_scroll);
    let line_width = body.width.saturating_sub(1) as usize;
    let lines = output
        .iter()
        .skip(app.ui_state.command_output.scroll)
        .take(visible_height)
        .map(|line| Line::from(Span::styled(fit_width(line, line_width), CLASH_THEME.text)))
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        body,
    );
}

fn render_header(frame: &mut Frame, area: Rect, app: &mut App, page: &'static str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    fill_area(frame, area, CLASH_THEME.bg);
    let label = app.t(Msg::CommonClose);
    let close_width = action_button_width(label);
    let title_gap = 1;
    if area.width <= close_width.saturating_add(title_gap) {
        render_title(frame, area, app, page);
        return;
    }
    let close_rect = Rect::new(
        area.x
            .saturating_add(area.width)
            .saturating_sub(close_width),
        area.y,
        close_width,
        1,
    );
    let title_area = Rect::new(
        area.x,
        area.y,
        close_rect
            .x
            .saturating_sub(area.x)
            .saturating_sub(title_gap),
        1,
    );
    render_title(frame, title_area, app, page);
    app.ui_state
        .hitboxes
        .register(close_rect, HitboxAction::CloseCommandOutput);
    frame.render_widget(
        Paragraph::new(format!(" {} ", label)).style(
            Style::default()
                .fg(CLASH_THEME.bg)
                .bg(CLASH_THEME.warning)
                .bold(),
        ),
        close_rect,
    );
}

fn render_title(frame: &mut Frame, area: Rect, app: &App, page: &'static str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let title = format!("{} · {}", app.t(Msg::NetworkCommandOutput), page);
    frame.render_widget(
        Paragraph::new(fit_width(&title, area.width as usize)).style(
            Style::default()
                .fg(CLASH_THEME.primary)
                .bg(CLASH_THEME.bg)
                .bold(),
        ),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    fill_area(frame, area, CLASH_THEME.bg);
    frame.render_widget(
        Paragraph::new(fit_width(
            app.t(Msg::CommandOutputScrollHint),
            area.width as usize,
        ))
        .style(Style::default().fg(CLASH_THEME.muted).bg(CLASH_THEME.bg)),
        area,
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

fn fit_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(value) as usize <= max_width {
        return value.to_string();
    }
    let ellipsis = "…";
    let keep_width = max_width.saturating_sub(display_width(ellipsis) as usize);
    if keep_width == 0 {
        return ellipsis.to_string();
    }
    let mut out = String::new();
    for ch in value.chars() {
        let mut next = out.clone();
        next.push(ch);
        if display_width(&next) as usize > keep_width {
            break;
        }
        out = next;
    }
    out.push_str(ellipsis);
    out
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
