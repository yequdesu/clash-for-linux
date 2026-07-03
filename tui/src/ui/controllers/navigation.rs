use crate::ui::prelude::*;

impl App {
    pub fn next_tab(&mut self) {
        self.ui_state.active_page = self.ui_state.active_page.next();
        self.show_help = false;
        self.node_picker_open = false;
        self.reset_selection();
        if self.ui_state.active_page == Tab::Traffic {
            self.fetch_traffic();
        }
    }

    pub fn prev_tab(&mut self) {
        self.ui_state.active_page = self.ui_state.active_page.prev();
        self.show_help = false;
        self.node_picker_open = false;
        self.reset_selection();
        if self.ui_state.active_page == Tab::Traffic {
            self.fetch_traffic();
        }
    }

    pub(crate) fn reset_selection(&mut self) {
        self.selected_proxy_idx = 0;
        self.selected_node_idx = 0;
        self.connections_selected = 0;
        self.selected_sub_idx = 0;
        self.traffic_selected_idx = 0;
    }

    pub fn clamp_proxy_selection(&mut self) {
        let visible_len = self.visible_proxy_groups().len();
        if visible_len == 0 {
            self.selected_proxy_idx = 0;
            self.node_picker_open = false;
        } else if self.selected_proxy_idx >= visible_len {
            self.selected_proxy_idx = visible_len - 1;
        }
        self.clamp_node_selection();
    }

    pub fn visible_proxy_groups(&self) -> Vec<(String, String)> {
        let query = self.search_query.to_lowercase();
        let mut groups: Vec<(String, String)> = self
            .proxy_groups
            .iter()
            .filter(|(name, current)| {
                query.is_empty()
                    || name.to_lowercase().contains(&query)
                    || current.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();

        if self.sort_mode {
            groups.sort_by(|a, b| {
                let da = self.delays.get(&a.0).copied().unwrap_or(u64::MAX);
                let db = self.delays.get(&b.0).copied().unwrap_or(u64::MAX);
                da.cmp(&db).then_with(|| a.0.cmp(&b.0))
            });
        } else {
            groups.sort_by(|a, b| a.0.cmp(&b.0));
        }
        groups
    }

    pub(crate) fn selected_proxy_group(&self) -> Option<(String, String)> {
        let groups = self.visible_proxy_groups();
        if groups.is_empty() {
            None
        } else {
            groups
                .get(self.selected_proxy_idx.min(groups.len() - 1))
                .cloned()
        }
    }

    pub(crate) fn selected_proxy_group_name(&self) -> Option<String> {
        self.selected_proxy_group().map(|(name, _)| name)
    }

    pub(crate) fn selected_proxy_nodes(&self) -> Vec<String> {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return vec![];
        };
        self.proxies
            .get(&group_name)
            .and_then(|info| info.all.clone())
            .unwrap_or_default()
    }

    pub(crate) fn selected_proxy_current_node(&self) -> String {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return String::new();
        };
        self.proxies
            .get(&group_name)
            .and_then(|info| info.now.clone())
            .unwrap_or_default()
    }

    pub(crate) fn clamp_node_selection(&mut self) {
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            self.selected_node_idx = 0;
            return;
        }
        if self.selected_node_idx >= nodes.len() {
            self.selected_node_idx = nodes.len() - 1;
        }
    }

    pub(crate) fn clamp_subscription_selection(&mut self) {
        if self.profiles.is_empty() {
            self.selected_sub_idx = 0;
        } else if self.selected_sub_idx >= self.profiles.len() {
            self.selected_sub_idx = self.profiles.len() - 1;
        }
    }

    pub(crate) fn clamp_traffic_selection(&mut self) {
        if self.traffic_top.is_empty() {
            self.traffic_selected_idx = 0;
        } else if self.traffic_selected_idx >= self.traffic_top.len() {
            self.traffic_selected_idx = self.traffic_top.len() - 1;
        }
    }

    pub(crate) fn clamp_traffic_window(&mut self) {
        if self.traffic_points.is_empty() {
            self.traffic_window_offset = 0;
            self.traffic_locked_bucket = None;
            return;
        }
        let max_idx = self.traffic_points.len() - 1;
        self.traffic_window_offset = self.traffic_window_offset.min(max_idx);
        if let Some(idx) = self.traffic_locked_bucket {
            self.traffic_locked_bucket = Some(idx.min(max_idx));
        }
    }

    pub(crate) fn selected_subscription_id(&self) -> Option<i32> {
        self.profiles.get(self.selected_sub_idx).map(|p| p.id)
    }

    pub(crate) fn selected_subscription(&self) -> Option<&ProfileEntry> {
        self.profiles.get(self.selected_sub_idx)
    }

    pub fn select_down(&mut self) {
        if self.node_picker_open {
            self.select_node_down();
            return;
        }
        match self.ui_state.active_page {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.selected_proxy_idx = (self.selected_proxy_idx + 1) % len;
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.connections_selected =
                    (self.connections_selected + 1) % self.connections.len();
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.selected_sub_idx = (self.selected_sub_idx + 1) % self.profiles.len();
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.traffic_selected_idx =
                    (self.traffic_selected_idx + 1) % self.traffic_top.len();
            }
            _ => {}
        }
    }

    pub fn select_up(&mut self) {
        if self.node_picker_open {
            self.select_node_up();
            return;
        }
        match self.ui_state.active_page {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.selected_proxy_idx = if self.selected_proxy_idx == 0 {
                    len.saturating_sub(1)
                } else {
                    self.selected_proxy_idx - 1
                };
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.connections_selected = if self.connections_selected == 0 {
                    self.connections.len().saturating_sub(1)
                } else {
                    self.connections_selected - 1
                };
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.selected_sub_idx = if self.selected_sub_idx == 0 {
                    self.profiles.len().saturating_sub(1)
                } else {
                    self.selected_sub_idx - 1
                };
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.traffic_selected_idx = if self.traffic_selected_idx == 0 {
                    self.traffic_top.len().saturating_sub(1)
                } else {
                    self.traffic_selected_idx - 1
                };
            }
            _ => {}
        }
    }
}
