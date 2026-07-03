use crate::ui::prelude::*;

impl App {
    pub fn toggle_log_pause(&mut self) {
        self.log_paused = !self.log_paused;
        self.status_msg = Some(if self.log_paused {
            "logs paused".into()
        } else {
            "logs resumed".into()
        });
        if !self.log_paused && self.ui_state.active_page == Tab::Logs {
            self.fetch_logs();
        }
    }

    pub fn cycle_log_level(&mut self) {
        self.log_level = self.log_level.next();
        self.clamp_log_scroll();
        self.status_msg = Some(format!("log level: {}", self.log_level.label()));
        if self.ui_state.active_page == Tab::Logs && !self.log_paused {
            self.fetch_logs();
        }
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.log_scroll = 0;
        self.status_msg = Some("logs cleared".into());
    }

    pub fn scroll_logs_down(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_add(amount);
        self.clamp_log_scroll();
    }

    pub fn scroll_logs_up(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_sub(amount);
    }

    pub(crate) fn clamp_log_scroll(&mut self) {
        let len = self.visible_logs().len();
        if len == 0 {
            self.log_scroll = 0;
        } else if self.log_scroll >= len {
            self.log_scroll = len - 1;
        }
    }

    pub(crate) fn visible_logs(&self) -> Vec<&String> {
        let query = self.search_query.to_lowercase();
        self.logs
            .iter()
            .filter(|line| self.log_level.matches_line(line))
            .filter(|line| query.is_empty() || line.to_lowercase().contains(&query))
            .collect()
    }
}
