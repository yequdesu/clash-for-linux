use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::Tab;
use crate::app::App;

pub struct LogsTab;

impl Tab for LogsTab {
    fn name(&self) -> &str {
        "Logs"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Logs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let content = if app.logs.is_empty() {
            "No logs available".to_string()
        } else {
            app.logs.join("\n")
        };

        let paragraph = Paragraph::new(content)
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('p') => {
                app.log_paused = !app.log_paused;
                true
            }
            crossterm::event::KeyCode::Char('c') => {
                app.logs.clear();
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("p: Pause/Resume | c: Clear | q: Quit")
    }
}
