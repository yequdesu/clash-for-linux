use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row, Cell};

use super::Tab;
use crate::app::App;

pub struct ConnectionsTab;

impl Tab for ConnectionsTab {
    fn name(&self) -> &str {
        "Connections"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Connections ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.connections.iter().map(|conn| {
            Row::new(vec![
                conn.host(),
                conn.conn_type(),
                conn.chain_str(),
                format!("{} KB/s", conn.dl_speed() / 1024),
            ])
        }).collect();

        let table = Table::new(rows, &[Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
            .block(block);

        frame.render_widget(table, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('c') => {
                // 关闭所有连接
                true
            }
            crossterm::event::KeyCode::Char('k') => {
                // 关闭选中的连接
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("c: Close all | k: Close selected | q: Quit")
    }
}
