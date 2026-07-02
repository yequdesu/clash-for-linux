use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
use ratatui::Frame;

use crate::i18n::{tr, Msg};
use crate::settings::LanguageSetting;
use crate::theme::CLASH_THEME;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Subscriptions,
    Proxies,
    Connections,
    Traffic,
    Network,
    Logs,
    Settings,
    Help,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Subscriptions,
            Tab::Proxies,
            Tab::Connections,
            Tab::Traffic,
            Tab::Network,
            Tab::Logs,
            Tab::Settings,
            Tab::Help,
        ]
    }

    pub fn msg(self) -> Msg {
        match self {
            Tab::Subscriptions => Msg::PageSubscriptions,
            Tab::Proxies => Msg::PageProxies,
            Tab::Connections => Msg::PageConnections,
            Tab::Traffic => Msg::PageTraffic,
            Tab::Network => Msg::PageNetwork,
            Tab::Logs => Msg::PageLogs,
            Tab::Settings => Msg::PageSettings,
            Tab::Help => Msg::PageHelp,
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

    pub fn setting_key(self) -> &'static str {
        match self {
            Tab::Subscriptions => "subscriptions",
            Tab::Proxies => "proxies",
            Tab::Connections => "connections",
            Tab::Traffic => "traffic",
            Tab::Network => "network",
            Tab::Logs => "logs",
            Tab::Settings => "settings",
            Tab::Help => "help",
        }
    }

    pub fn from_setting_key(raw: &str) -> Option<Tab> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "sub" | "subscription" | "subscriptions" => Some(Tab::Subscriptions),
            "proxy" | "proxies" => Some(Tab::Proxies),
            "conn" | "connection" | "connections" => Some(Tab::Connections),
            "traffic" => Some(Tab::Traffic),
            "net" | "network" => Some(Tab::Network),
            "log" | "logs" => Some(Tab::Logs),
            "setting" | "settings" => Some(Tab::Settings),
            "help" => Some(Tab::Help),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn from_index(idx: usize) -> Tab {
        Self::all().get(idx).copied().unwrap_or(Tab::Subscriptions)
    }
}

pub fn render_tab_bar(frame: &mut Frame, area: Rect, active: Tab, language: LanguageSetting) {
    let tabs: Vec<Line> = Tab::all()
        .iter()
        .map(|tab| {
            let label = format!(" {} ", tr(language, tab.msg()));
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
