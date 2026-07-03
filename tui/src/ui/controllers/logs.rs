use crate::ui::prelude::*;

impl App {
    pub(crate) fn refresh_logs_if_visible(&mut self) {
        if self.ui_state.active_page == Tab::Logs && !self.ui_state.logs.paused {
            self.fetch_logs();
        }
    }

    pub fn toggle_log_pause(&mut self) {
        self.ui_state.logs.paused = !self.ui_state.logs.paused;
        self.status_msg = Some(if self.ui_state.logs.paused {
            "logs paused".into()
        } else {
            "logs resumed".into()
        });
        self.refresh_logs_if_visible();
    }

    pub fn cycle_log_level(&mut self) {
        self.ui_state.logs.level = self.ui_state.logs.level.next();
        self.clamp_log_scroll();
        self.status_msg = Some(format!("log level: {}", self.ui_state.logs.level.label()));
        self.refresh_logs_if_visible();
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.ui_state.logs.scroll = 0;
        self.status_msg = Some("logs cleared".into());
    }

    pub fn scroll_logs_down(&mut self, amount: usize) {
        self.ui_state.logs.scroll = self.ui_state.logs.scroll.saturating_add(amount);
        self.clamp_log_scroll();
    }

    pub fn scroll_logs_up(&mut self, amount: usize) {
        self.ui_state.logs.scroll = self.ui_state.logs.scroll.saturating_sub(amount);
    }

    pub(crate) fn clamp_log_scroll(&mut self) {
        let len = self.visible_logs().len();
        if len == 0 {
            self.ui_state.logs.scroll = 0;
        } else if self.ui_state.logs.scroll >= len {
            self.ui_state.logs.scroll = len - 1;
        }
    }

    pub(crate) fn visible_logs(&self) -> Vec<&String> {
        let query = self.ui_state.proxies.search_query.to_lowercase();
        self.logs
            .iter()
            .filter(|line| self.ui_state.logs.level.matches_line(line))
            .filter(|line| query.is_empty() || line.to_lowercase().contains(&query))
            .collect()
    }
}
