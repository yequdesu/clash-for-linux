use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row, Cell};

use super::Tab;
use crate::app::App;

pub struct SubscriptionsTab;

impl Tab for SubscriptionsTab {
    fn name(&self) -> &str {
        "Subscriptions"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Subscriptions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.subscriptions.iter().map(|sub| {
            Row::new(vec![
                sub.name.clone(),
                sub.status.clone(),
                sub.updated.clone(),
                format!("{} proxies", sub.proxies_count),
            ])
        }).collect();

        let table = Table::new(rows, &[Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
            .block(block);

        frame.render_widget(table, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('u') => {
                // 更新订阅
                true
            }
            crossterm::event::KeyCode::Char('s') => {
                // 切换订阅
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("u: Update | s: Switch | q: Quit")
    }
}
