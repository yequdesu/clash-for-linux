use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::Tab;
use crate::app::App;

pub struct OverviewTab;

impl Tab for OverviewTab {
    fn name(&self) -> &str {
        "Overview"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Overview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let content = format!(
            "Kernel: {}\nMode: {}\nTraffic: {} up / {} down",
            app.kernel_version,
            app.kernel_mode,
            app.traffic.up,
            app.traffic.down
        );

        let paragraph = Paragraph::new(content)
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('r') => {
                // 刷新数据
                true
            }
            crossterm::event::KeyCode::Char('s') => {
                // 启动/停止内核
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("r: Refresh | s: Start/Stop kernel | q: Quit")
    }

    fn status_info(&self, app: &App) -> Option<String> {
        let status = if app.kernel_running { "Running" } else { "Stopped" };
        Some(format!("Kernel: {} | {}", status, app.kernel_version))
    }
}
