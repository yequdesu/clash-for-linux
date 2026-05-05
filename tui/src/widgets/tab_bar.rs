use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
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
        &[Tab::Overview, Tab::Proxies, Tab::Subscriptions, Tab::Connections, Tab::Logs]
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

    #[allow(dead_code)]
    pub fn from_index(idx: usize) -> Tab {
        Self::all().get(idx).copied().unwrap_or(Tab::Overview)
    }
}

pub fn render_tab_bar(frame: &mut Frame, area: Rect, active: Tab) {
    let tabs: Vec<Line> = Tab::all()
        .iter()
        .map(|tab| {
            let label = format!(" {} ", tab.label());
            let style = if *tab == active {
                Style::default()
                    .fg(CLASH_THEME.primary)
                    .bg(CLASH_THEME.surface)
            } else {
                Style::default().fg(CLASH_THEME.muted).bg(CLASH_THEME.bg)
            };
            Line::from(Span::styled(label, style))
        })
        .collect();

    let tabs_widget = Tabs::new(tabs)
        .style(Style::default().bg(CLASH_THEME.bg))
        .divider("");

    frame.render_widget(tabs_widget, area);
}
