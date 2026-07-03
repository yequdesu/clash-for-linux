use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::action_registry::{self};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::components::panel::Panel;
use crate::ui::hitbox::register_table_row_hitboxes;

pub(crate) fn render_traffic(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(9),
    ])
    .split(area);

    let (down_total, up_total, down_rate, up_rate) = traffic_summary(&app.traffic_points);
    let filter = app
        .ui_state
        .traffic
        .filter_key
        .as_deref()
        .map(|key| trunc_str(key, 32))
        .unwrap_or_else(|| app.t(Msg::TrafficAll).into());
    let summary = vec![Line::from(vec![
        Span::styled(
            format!(
                "  {} [{}]  {} [{}]  {} [{}]  {} [{}]  ",
                app.t(Msg::TrafficRange),
                app.ui_state.traffic.range.label(),
                app.t(Msg::TrafficChart),
                app.ui_state.traffic.chart.label(),
                app.t(Msg::TrafficBy),
                app.ui_state.traffic.dimension.label(),
                app.t(Msg::TrafficFilter),
                filter
            ),
            CLASH_THEME.muted,
        ),
        Span::styled(
            format!(
                "{} {}  {} {}  {} ↓ {}/s ↑ {}/s",
                app.t(Msg::TrafficDown),
                format_bytes(down_total),
                app.t(Msg::TrafficUp),
                format_bytes(up_total),
                app.t(Msg::TrafficPeak),
                format_bytes(down_rate),
                format_bytes(up_rate)
            ),
            CLASH_THEME.text,
        ),
    ])];
    Panel::new(app.t(Msg::PageTraffic)).render(frame, rows[0], summary);
    register_traffic_control_hitboxes(app, rows[0]);

    let main =
        Layout::horizontal([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]).split(rows[1]);
    render_traffic_chart(frame, main[0], app);
    render_traffic_top(frame, main[1], app);
    render_traffic_detail(frame, rows[2], app);
}

pub(crate) fn register_traffic_control_hitboxes(app: &mut App, area: Rect) {
    if area.height < 3 || area.width < 12 {
        return;
    }
    let y = area.y + 1;
    let x = area.x + 2;
    register_clamped_hitbox(app, x, y, 14, area, HitboxAction::NextTrafficRange);
    register_clamped_hitbox(app, x + 15, y, 14, area, HitboxAction::ToggleTrafficChart);
    register_clamped_hitbox(app, x + 30, y, 12, area, HitboxAction::NextTrafficDimension);
}

pub(crate) fn register_clamped_hitbox(
    app: &mut App,
    x: u16,
    y: u16,
    width: u16,
    area: Rect,
    action: HitboxAction,
) {
    if x >= area.x.saturating_add(area.width) || y >= area.y.saturating_add(area.height) {
        return;
    }
    let available = area.x.saturating_add(area.width).saturating_sub(x);
    app.ui_state
        .hitboxes
        .register(Rect::new(x, y, width.min(available), 1), action);
}

pub(crate) fn render_traffic_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    let filter = app
        .ui_state
        .traffic
        .filter_key
        .as_deref()
        .map(|key| trunc_str(key, 28))
        .unwrap_or_else(|| app.t(Msg::TrafficAllTraffic).into());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(
                " {} · {} · {} · {} · {} ",
                app.t(Msg::TrafficHistory),
                app.ui_state.traffic.range.label(),
                app.ui_state.traffic.range.step_arg(),
                app.ui_state.traffic.chart.label(),
                filter
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    app.ui_state
        .hitboxes
        .register(inner, HitboxAction::ScrollTrafficChart);

    if app.traffic_points.is_empty() {
        let msg = app
            .ui_state
            .traffic
            .error
            .as_deref()
            .unwrap_or(app.t(Msg::TrafficNoHistory));
        frame.render_widget(
            Paragraph::new(msg).style(
                Style::default()
                    .fg(CLASH_THEME.muted)
                    .bg(CLASH_THEME.surface),
            ),
            inner,
        );
        return;
    }

    if app.ui_state.traffic.chart == TrafficChartKind::Line {
        render_traffic_line_chart(frame, inner, app);
        return;
    }

    let max_lines = inner.height.max(1) as usize;
    let (start, end) = traffic_window_bounds(
        app.traffic_points.len(),
        max_lines,
        app.ui_state.traffic.window_offset,
    );
    register_traffic_bar_bucket_hitboxes(app, inner, start, end.saturating_sub(start));
    let visible = &app.traffic_points[start..end];
    let max_down = visible
        .iter()
        .map(|p| p.download_delta.max(p.down_bps))
        .max()
        .unwrap_or(1)
        .max(1);
    let max_up = visible
        .iter()
        .map(|p| p.upload_delta.max(p.up_bps))
        .max()
        .unwrap_or(1)
        .max(1);
    let bar_width = inner.width.saturating_sub(26).max(8) as usize;
    let mut lines = Vec::new();
    for point in visible {
        let down_bar = scaled_bar(
            point.download_delta.max(point.down_bps),
            max_down,
            bar_width,
        );
        let up_bar = scaled_bar(point.upload_delta.max(point.up_bps), max_up, bar_width / 3);
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", point.ts.format("%H:%M")), CLASH_THEME.muted),
            Span::styled(down_bar, CLASH_THEME.primary),
            Span::styled(" ", CLASH_THEME.text),
            Span::styled(up_bar, CLASH_THEME.accent),
            Span::styled(
                format!(" ↓{}", format_bytes(point.download_delta)),
                CLASH_THEME.text,
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
}

pub(crate) fn render_traffic_line_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width < 16 || area.height < 4 {
        return;
    }
    let chart_width = area.width.saturating_sub(11).max(8) as usize;
    let chart_height = area.height.saturating_sub(2).max(2) as usize;
    let (start, end) = traffic_window_bounds(
        app.traffic_points.len(),
        chart_width,
        app.ui_state.traffic.window_offset,
    );
    register_traffic_line_bucket_hitboxes(
        app,
        area,
        start,
        end.saturating_sub(start),
        chart_width,
        10,
    );
    let visible = &app.traffic_points[start..end];
    let lines = traffic_line_chart_lines(app, visible, chart_width, chart_height);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        area,
    );
}

pub(crate) fn register_traffic_line_bucket_hitboxes(
    app: &mut App,
    area: Rect,
    start: usize,
    count: usize,
    plot_width: usize,
    plot_x_offset: u16,
) {
    if count == 0 || area.width == 0 || area.height == 0 {
        return;
    }
    for idx in 0..count {
        let x_offset = if count <= 1 || plot_width <= 1 {
            0
        } else {
            idx * (plot_width - 1) / (count - 1)
        };
        let x = area
            .x
            .saturating_add(plot_x_offset)
            .saturating_add(x_offset as u16);
        if x >= area.x.saturating_add(area.width) {
            continue;
        }
        app.ui_state.hitboxes.register(
            Rect::new(x, area.y, 1, area.height),
            HitboxAction::SelectTrafficBucket(start + idx),
        );
    }
}

pub(crate) fn register_traffic_bar_bucket_hitboxes(
    app: &mut App,
    area: Rect,
    start: usize,
    count: usize,
) {
    if count == 0 || area.width == 0 || area.height == 0 {
        return;
    }
    for idx in 0..count.min(area.height as usize) {
        app.ui_state.hitboxes.register(
            Rect::new(area.x, area.y.saturating_add(idx as u16), area.width, 1),
            HitboxAction::SelectTrafficBucket(start + idx),
        );
    }
}

pub(crate) fn traffic_window_bounds(
    total: usize,
    capacity: usize,
    offset: usize,
) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }
    let capacity = capacity.max(1).min(total);
    let offset = offset.min(total.saturating_sub(capacity));
    let end = total.saturating_sub(offset);
    let start = end.saturating_sub(capacity);
    (start, end)
}

pub(crate) fn traffic_line_chart_lines(
    app: &App,
    points: &[TrafficPoint],
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let width = width.max(1);
    let height = height.max(1);
    let down: Vec<u64> = points
        .iter()
        .map(|p| p.download_delta.max(p.down_bps))
        .collect();
    let up: Vec<u64> = points
        .iter()
        .map(|p| p.upload_delta.max(p.up_bps))
        .collect();
    let max_value = down
        .iter()
        .chain(up.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let mut canvas = vec![vec![' '; width]; height];
    plot_traffic_series(&mut canvas, &down, max_value, 'd');
    plot_traffic_series(&mut canvas, &up, max_value, 'u');

    let mut lines = vec![Line::from(vec![
        Span::styled("  d", CLASH_THEME.primary),
        Span::styled(
            format!(" {}  ", app.t(Msg::TrafficDownload)),
            CLASH_THEME.muted,
        ),
        Span::styled("u", CLASH_THEME.accent),
        Span::styled(
            format!(" {}  ", app.t(Msg::TrafficUpload)),
            CLASH_THEME.muted,
        ),
        Span::styled(
            format!("{} {}", app.t(Msg::TrafficMax), format_bytes(max_value)),
            CLASH_THEME.text,
        ),
    ])];
    for (row_idx, row) in canvas.into_iter().enumerate() {
        let label = traffic_axis_label(row_idx, height, max_value);
        let mut spans = vec![Span::styled(format!("{:>8} |", label), CLASH_THEME.muted)];
        for ch in row {
            let style = match ch {
                'd' => CLASH_THEME.primary,
                'u' => CLASH_THEME.accent,
                '*' => CLASH_THEME.warning,
                _ => CLASH_THEME.surface,
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    let from = points
        .first()
        .map(|p| p.ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into());
    let to = points
        .last()
        .map(|p| p.ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into());
    let gap = width.saturating_sub(from.len() + to.len()).max(1);
    lines.push(Line::from(vec![
        Span::styled("         +", CLASH_THEME.muted),
        Span::styled("-".repeat(width), CLASH_THEME.muted),
        Span::styled(" ", CLASH_THEME.muted),
        Span::styled(
            format!("{}{}{}", from, " ".repeat(gap), to),
            CLASH_THEME.muted,
        ),
    ]));
    lines
}

pub(crate) fn traffic_axis_label(row_idx: usize, height: usize, max_value: u64) -> String {
    if row_idx == 0 {
        trunc_str(&format_bytes(max_value), 8)
    } else if row_idx + 1 == height {
        "0".into()
    } else if row_idx == height / 2 {
        trunc_str(&format_bytes(max_value / 2), 8)
    } else {
        String::new()
    }
}

pub(crate) fn plot_traffic_series(
    canvas: &mut [Vec<char>],
    values: &[u64],
    max_value: u64,
    marker: char,
) {
    if values.is_empty() || canvas.is_empty() || canvas[0].is_empty() {
        return;
    }
    let height = canvas.len();
    let width = canvas[0].len();
    let mut previous: Option<(usize, usize)> = None;
    for (idx, value) in values.iter().enumerate() {
        let x = if values.len() <= 1 || width <= 1 {
            0
        } else {
            idx * (width - 1) / (values.len() - 1)
        };
        let y = traffic_chart_y(*value, max_value, height);
        if let Some((prev_x, prev_y)) = previous {
            draw_traffic_segment(canvas, prev_x, prev_y, x, y, marker);
        } else {
            mark_traffic_canvas(canvas, x, y, marker);
        }
        previous = Some((x, y));
    }
}

pub(crate) fn draw_traffic_segment(
    canvas: &mut [Vec<char>],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    marker: char,
) {
    if x0 == x1 {
        let start = y0.min(y1);
        let end = y0.max(y1);
        for y in start..=end {
            mark_traffic_canvas(canvas, x0, y, marker);
        }
        return;
    }
    let start = x0.min(x1);
    let end = x0.max(x1);
    let span = (x1 as isize - x0 as isize) as f64;
    for x in start..=end {
        let t = (x as isize - x0 as isize) as f64 / span;
        let y = (y0 as f64 + (y1 as f64 - y0 as f64) * t).round() as usize;
        mark_traffic_canvas(canvas, x, y, marker);
    }
}

pub(crate) fn traffic_chart_y(value: u64, max_value: u64, height: usize) -> usize {
    if height <= 1 {
        return 0;
    }
    let ratio = value as f64 / max_value.max(1) as f64;
    (((height - 1) as f64) * (1.0 - ratio.clamp(0.0, 1.0))).round() as usize
}

pub(crate) fn mark_traffic_canvas(canvas: &mut [Vec<char>], x: usize, y: usize, marker: char) {
    if y >= canvas.len() || x >= canvas[y].len() {
        return;
    }
    canvas[y][x] = match canvas[y][x] {
        ' ' => marker,
        existing if existing == marker => existing,
        _ => '*',
    };
}

pub(crate) fn render_traffic_top(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.border))
        .title(Span::styled(
            format!(
                " {} {} ",
                app.ui_state.traffic.dimension.label(),
                app.t(Msg::TrafficBreakdown)
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.ui_state
        .hitboxes
        .register(inner, HitboxAction::ScrollTrafficRows);

    if app.traffic_top.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::TrafficNoBreakdown))
                .style(Style::default().fg(CLASH_THEME.muted)),
            inner,
        );
        return;
    }

    let rows: Vec<Row> = app
        .traffic_top
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let style = if idx == app.ui_state.traffic.selected_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(vec![trunc_str(&row.key, 34), format_bytes(row.total_delta)]).style(style)
        })
        .collect();
    let table = Table::new(rows, [Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
        .header(
            Row::new(vec![
                app.ui_state.traffic.dimension.label(),
                app.t(Msg::TrafficTotal),
            ])
            .style(Style::default().fg(CLASH_THEME.muted)),
        )
        .style(Style::default().bg(CLASH_THEME.surface));
    register_table_row_hitboxes(
        app,
        inner,
        app.traffic_top.len(),
        HitboxAction::SelectTrafficRow,
    );
    frame.render_widget(table, inner);
}

pub(crate) fn render_traffic_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let selected = app.traffic_top.get(app.ui_state.traffic.selected_idx);
    let lines = if let Some(row) = selected {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {} {}: ",
                        app.t(Msg::TrafficSelected),
                        app.ui_state.traffic.dimension.label()
                    ),
                    CLASH_THEME.muted,
                ),
                Span::styled(row.key.clone(), CLASH_THEME.text),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficDownload)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.download_delta), CLASH_THEME.primary),
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficUpload)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.upload_delta), CLASH_THEME.accent),
                Span::styled(
                    format!("  {} ", app.t(Msg::TrafficTotal)),
                    CLASH_THEME.muted,
                ),
                Span::styled(format_bytes(row.total_delta), CLASH_THEME.text),
            ]),
        ];
        lines.extend(traffic_status_lines(app));
        lines.extend(traffic_locked_bucket_lines(app));
        lines.extend(traffic_output_lines(app));
        lines
    } else {
        let mut lines = vec![Line::from(Span::styled(
            format!("  {}", app.t(Msg::TrafficDataSource)),
            CLASH_THEME.muted,
        ))];
        lines.extend(traffic_status_lines(app));
        lines.extend(traffic_locked_bucket_lines(app));
        lines.extend(traffic_output_lines(app));
        lines
    };
    let inner = Panel::new(app.t(Msg::TrafficDetail)).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let action_height = if inner.height >= 6 { 2 } else { 1 };
    let text_height = inner.height.saturating_sub(action_height);
    let text_area = Rect::new(inner.x, inner.y, inner.width, text_height);
    if text_area.height > 0 {
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
            text_area,
        );
    }
    if action_height > 0 {
        let action_area = Rect::new(
            inner.x,
            inner.y.saturating_add(text_height),
            inner.width,
            action_height,
        );
        let buttons = action_buttons_from_specs(app, action_registry::traffic_action_specs());
        render_action_buttons(frame, app, action_area, &buttons);
    }
}

pub(crate) fn traffic_locked_bucket_lines(app: &App) -> Vec<Line<'static>> {
    let Some(idx) = app.ui_state.traffic.locked_bucket else {
        return Vec::new();
    };
    let Some(point) = app.traffic_points.get(idx) else {
        return Vec::new();
    };
    vec![Line::from(vec![
        Span::styled(
            format!("  {} ", app.t(Msg::TrafficBucket)),
            CLASH_THEME.muted,
        ),
        Span::styled(point.ts.format("%H:%M:%S").to_string(), CLASH_THEME.text),
        Span::styled(format!("  {} ", app.t(Msg::TrafficDown)), CLASH_THEME.muted),
        Span::styled(format_bytes(point.download_delta), CLASH_THEME.primary),
        Span::styled(format!("  {} ", app.t(Msg::TrafficUp)), CLASH_THEME.muted),
        Span::styled(format_bytes(point.upload_delta), CLASH_THEME.accent),
        Span::styled(format!("  {} ", app.t(Msg::TrafficRate)), CLASH_THEME.muted),
        Span::styled(
            format!(
                "↓{}/s ↑{}/s  conn {}",
                format_bytes(point.down_bps),
                format_bytes(point.up_bps),
                point.connections
            ),
            CLASH_THEME.text,
        ),
    ])]
}

pub(crate) fn traffic_output_lines(app: &App) -> Vec<Line<'static>> {
    app.ui_state
        .traffic
        .output
        .iter()
        .take(2)
        .map(|line| {
            Line::from(Span::styled(
                format!("  {}", trunc_str(line, 92)),
                CLASH_THEME.muted,
            ))
        })
        .collect()
}

pub(crate) fn traffic_status_lines(app: &App) -> Vec<Line<'static>> {
    let status = &app.traffic_status;
    let last = if status.last_sample.is_empty() {
        app.t(Msg::TrafficNever).to_string()
    } else {
        status.last_sample.clone()
    };
    let collector_interval = if status.collector_interval.is_empty() {
        "-"
    } else {
        status.collector_interval.as_str()
    };
    let collector = if status.collector_running {
        format!(
            "{} pid={} interval={} log={}",
            app.t(Msg::TrafficRunning),
            status.collector_pid,
            collector_interval,
            trunc_str(&status.collector_log, 42)
        )
    } else if status.collector_stale {
        format!(
            "{} pid={} log={}",
            app.t(Msg::TrafficStale),
            status.collector_pid,
            trunc_str(&status.collector_log, 42)
        )
    } else {
        format!(
            "{} log={}",
            app.t(Msg::TrafficStopped),
            trunc_str(&status.collector_log, 42)
        )
    };
    vec![
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficStore)),
                CLASH_THEME.muted,
            ),
            Span::styled(trunc_str(&status.store_dir, 42), CLASH_THEME.text),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficSamples)),
                CLASH_THEME.muted,
            ),
            Span::styled(
                format!(
                    "raw {}  10s {}  1m {}  {} {}  {} {}",
                    status.raw_samples,
                    status.rollup_10s,
                    status.rollup_1m,
                    app.t(Msg::TrafficTracked),
                    status.tracked_connections,
                    app.t(Msg::TrafficLast),
                    last
                ),
                CLASH_THEME.text,
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} ", app.t(Msg::TrafficCollector)),
                CLASH_THEME.muted,
            ),
            Span::styled(collector, CLASH_THEME.text),
            if status.collector_status_read_error.is_empty() {
                Span::raw("")
            } else {
                Span::styled(
                    format!(
                        "  {} {}",
                        app.t(Msg::TrafficStatusError),
                        trunc_str(&status.collector_status_read_error, 32)
                    ),
                    CLASH_THEME.warning,
                )
            },
        ]),
    ]
}
