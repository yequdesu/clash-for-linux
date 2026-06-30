use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::CLASH_THEME;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Proxies,
    Subscriptions,
    Connections,
    Logs,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Overview,
            Tab::Proxies,
            Tab::Subscriptions,
            Tab::Connections,
            Tab::Logs,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Proxies => "Proxies",
            Tab::Subscriptions => "Subscriptions",
            Tab::Connections => "Connections",
            Tab::Logs => "Logs",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tab::Overview => "1",
            Tab::Proxies => "2",
            Tab::Subscriptions => "3",
            Tab::Connections => "4",
            Tab::Logs => "5",
        }
    }

    pub fn next(self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(self) -> Tab {
        let all = Self::all();
        let idx = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

pub fn render_tab_bar(frame: &mut Frame, area: Rect, active: Tab) {
    if area.width < 10 {
        return;
    }

    let bg_fill = " ".repeat(area.width as usize);
    frame.buffer_mut().set_string(
        area.x, area.y, &bg_fill,
        Style::default().bg(CLASH_THEME.bg),
    );

    let tab_count = Tab::all().len();
    let tab_width = (area.width / tab_count as u16).min(22);

    for (i, tab) in Tab::all().iter().enumerate() {
        let tx = area.x + (i as u16 * tab_width);
        let tab_area = Rect::new(tx, area.y, tab_width, 2);

        let is_active = *tab == active;
        let label = format!(" {} {} ", tab.shortcut(), tab.label());

        let display = if label.len() + 2 > tab_width as usize {
            format!(" {} ", tab.label())
        } else {
            label
        };

        let (fg, bg, border_color) = if is_active {
            (CLASH_THEME.text, CLASH_THEME.tab_active_bg, CLASH_THEME.primary)
        } else {
            (CLASH_THEME.muted, CLASH_THEME.tab_inactive_bg, CLASH_THEME.border)
        };

        let tab_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg));

        frame.render_widget(tab_block, tab_area);

        let inner = Rect::new(tx + 1, area.y, tab_width.saturating_sub(2), 1);
        let padding = (inner.width.saturating_sub(display.len() as u16)) / 2;
        let label_x = inner.x + padding;
        let label_area = Rect::new(label_x, inner.y, display.len() as u16, 1);

        let label_span = Span::styled(
            display,
            Style::default().fg(fg).bg(bg).add_modifier(if is_active {
                ratatui::style::Modifier::BOLD
            } else {
                ratatui::style::Modifier::empty()
            }),
        );
        frame.render_widget(
            Paragraph::new(Line::from(label_span)).style(Style::default().bg(bg)),
            label_area,
        );

        if is_active {
            let highlight = "▔".repeat(tab_width.saturating_sub(2) as usize);
            let hl_area = Rect::new(tx + 1, area.y + 1, tab_width.saturating_sub(2), 1);
            frame.buffer_mut().set_string(
                hl_area.x, hl_area.y, &highlight,
                Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.tab_active_bg),
            );
        }
    }
}
