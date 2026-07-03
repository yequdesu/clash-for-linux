use crate::ui::prelude::*;

impl App {
    pub fn next_tab(&mut self) {
        self.ui_state.active_page = self.ui_state.active_page.next();
        self.show_help = false;
        self.ui_state.proxies.node_picker_open = false;
        self.reset_selection();
        if self.ui_state.active_page == Tab::Traffic {
            self.fetch_traffic();
        }
    }

    pub fn prev_tab(&mut self) {
        self.ui_state.active_page = self.ui_state.active_page.prev();
        self.show_help = false;
        self.ui_state.proxies.node_picker_open = false;
        self.reset_selection();
        if self.ui_state.active_page == Tab::Traffic {
            self.fetch_traffic();
        }
    }

    pub(crate) fn reset_selection(&mut self) {
        self.ui_state.proxies.selected_idx = 0;
        self.ui_state.proxies.selected_node_idx = 0;
        self.ui_state.connections.selected_idx = 0;
        self.ui_state.subscriptions.selected_idx = 0;
        self.ui_state.traffic.selected_idx = 0;
    }

    pub(crate) fn select_last_visible_item(&mut self) {
        self.ui_state.proxies.selected_idx = self.proxy_groups.len().saturating_sub(1);
        self.ui_state.connections.selected_idx = self.connections.len().saturating_sub(1);
        self.ui_state.subscriptions.selected_idx = self.profiles.len().saturating_sub(1);
        self.ui_state.traffic.selected_idx = self.traffic_top.len().saturating_sub(1);
    }

    pub fn clamp_proxy_selection(&mut self) {
        let visible_len = self.visible_proxy_groups().len();
        if visible_len == 0 {
            self.ui_state.proxies.selected_idx = 0;
            self.ui_state.proxies.node_picker_open = false;
        } else if self.ui_state.proxies.selected_idx >= visible_len {
            self.ui_state.proxies.selected_idx = visible_len - 1;
        }
        self.clamp_node_selection();
    }

    pub fn visible_proxy_groups(&self) -> Vec<(String, String)> {
        let query = self.ui_state.proxies.search_query.to_lowercase();
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

        if self.ui_state.proxies.sort_by_delay {
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
                .get(self.ui_state.proxies.selected_idx.min(groups.len() - 1))
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
            self.ui_state.proxies.selected_node_idx = 0;
            return;
        }
        if self.ui_state.proxies.selected_node_idx >= nodes.len() {
            self.ui_state.proxies.selected_node_idx = nodes.len() - 1;
        }
    }

    pub(crate) fn clamp_subscription_selection(&mut self) {
        if self.profiles.is_empty() {
            self.ui_state.subscriptions.selected_idx = 0;
        } else if self.ui_state.subscriptions.selected_idx >= self.profiles.len() {
            self.ui_state.subscriptions.selected_idx = self.profiles.len() - 1;
        }
    }

    pub(crate) fn clamp_traffic_selection(&mut self) {
        if self.traffic_top.is_empty() {
            self.ui_state.traffic.selected_idx = 0;
        } else if self.ui_state.traffic.selected_idx >= self.traffic_top.len() {
            self.ui_state.traffic.selected_idx = self.traffic_top.len() - 1;
        }
    }

    pub(crate) fn clamp_traffic_window(&mut self) {
        if self.traffic_points.is_empty() {
            self.ui_state.traffic.window_offset = 0;
            self.ui_state.traffic.locked_bucket = None;
            return;
        }
        let max_idx = self.traffic_points.len() - 1;
        self.ui_state.traffic.window_offset = self.ui_state.traffic.window_offset.min(max_idx);
        if let Some(idx) = self.ui_state.traffic.locked_bucket {
            self.ui_state.traffic.locked_bucket = Some(idx.min(max_idx));
        }
    }

    pub(crate) fn selected_subscription_id(&self) -> Option<i32> {
        self.profiles
            .get(self.ui_state.subscriptions.selected_idx)
            .map(|p| p.id)
    }

    pub(crate) fn selected_subscription(&self) -> Option<&ProfileEntry> {
        self.profiles.get(self.ui_state.subscriptions.selected_idx)
    }

    pub fn select_down(&mut self) {
        if self.ui_state.proxies.node_picker_open {
            self.select_node_down();
            return;
        }
        match self.ui_state.active_page {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.ui_state.proxies.selected_idx = (self.ui_state.proxies.selected_idx + 1) % len;
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.ui_state.connections.selected_idx =
                    (self.ui_state.connections.selected_idx + 1) % self.connections.len();
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.ui_state.subscriptions.selected_idx =
                    (self.ui_state.subscriptions.selected_idx + 1) % self.profiles.len();
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.ui_state.traffic.selected_idx =
                    (self.ui_state.traffic.selected_idx + 1) % self.traffic_top.len();
            }
            _ => {}
        }
    }

    pub fn select_up(&mut self) {
        if self.ui_state.proxies.node_picker_open {
            self.select_node_up();
            return;
        }
        match self.ui_state.active_page {
            Tab::Proxies => {
                let len = self.visible_proxy_groups().len();
                if len == 0 {
                    return;
                }
                self.ui_state.proxies.selected_idx = if self.ui_state.proxies.selected_idx == 0 {
                    len.saturating_sub(1)
                } else {
                    self.ui_state.proxies.selected_idx - 1
                };
            }
            Tab::Connections => {
                if self.connections.is_empty() {
                    return;
                }
                self.ui_state.connections.selected_idx =
                    if self.ui_state.connections.selected_idx == 0 {
                        self.connections.len().saturating_sub(1)
                    } else {
                        self.ui_state.connections.selected_idx - 1
                    };
            }
            Tab::Subscriptions => {
                if self.profiles.is_empty() {
                    return;
                }
                self.ui_state.subscriptions.selected_idx =
                    if self.ui_state.subscriptions.selected_idx == 0 {
                        self.profiles.len().saturating_sub(1)
                    } else {
                        self.ui_state.subscriptions.selected_idx - 1
                    };
            }
            Tab::Traffic => {
                if self.traffic_top.is_empty() {
                    return;
                }
                self.ui_state.traffic.selected_idx = if self.ui_state.traffic.selected_idx == 0 {
                    self.traffic_top.len().saturating_sub(1)
                } else {
                    self.ui_state.traffic.selected_idx - 1
                };
            }
            _ => {}
        }
    }
}
