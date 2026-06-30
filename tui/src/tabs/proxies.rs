use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Table, Row, Cell};

use super::Tab;
use crate::app::App;

pub struct ProxiesTab;

impl Tab for ProxiesTab {
    fn name(&self) -> &str {
        "Proxies"
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),  // 代理组列表
                Constraint::Percentage(70),  // 节点列表
            ])
            .split(area);

        // 渲染代理组列表
        self.render_proxy_groups(frame, chunks[0], app);

        // 渲染节点列表
        self.render_proxy_nodes(frame, chunks[1], app);
    }

    fn handle_key(&self, key: crossterm::event::KeyCode, app: &mut App) -> bool {
        match key {
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                // 向下选择
                true
            }
            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                // 向上选择
                true
            }
            crossterm::event::KeyCode::Enter => {
                // 切换代理
                true
            }
            crossterm::event::KeyCode::Char('t') => {
                // 测试延迟
                true
            }
            _ => false
        }
    }

    fn help_text(&self) -> Option<&str> {
        Some("j/k: Navigate | Enter: Switch | t: Test delay | q: Quit")
    }
}

impl ProxiesTab {
    fn render_proxy_groups(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Proxy Groups ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = app.proxy_groups.iter().map(|group| {
            Row::new(vec![
                group.name.clone(),
                group.group_type.clone(),
                group.now.clone(),
            ])
        }).collect();

        let table = Table::new(rows, &[Constraint::Percentage(50), Constraint::Percentage(30), Constraint::Percentage(20)])
            .block(block);

        frame.render_widget(table, area);
    }

    fn render_proxy_nodes(&self, frame: &mut Frame, area: Rect, app: &App) {
        let block = Block::default()
            .title(" Proxy Nodes ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let rows: Vec<Row> = if let Some(group) = app.proxy_groups.get(app.proxy_group_selected) {
            group.proxies.iter().map(|node| {
                Row::new(vec![
                    node.name.clone(),
                    node.proxy_type.clone(),
                    format!("{} ms", node.delay),
                ])
            }).collect()
        } else {
            vec![]
        };

        let table = Table::new(rows, &[Constraint::Percentage(40), Constraint::Percentage(30), Constraint::Percentage(30)])
            .block(block);

        frame.render_widget(table, area);
    }
}
