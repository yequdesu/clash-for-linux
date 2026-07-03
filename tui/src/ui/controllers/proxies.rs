use crate::ui::prelude::*;

impl App {
    pub fn test_selected_delay(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if self.ui_state.active_page == Tab::Proxies {
            let Some(name) = self.selected_delay_target() else {
                return;
            };
            self.rt.spawn(async move {
                if let Ok(delay) = api.test_delay(&name).await {
                    let _ = tx.send(DataEvent::Delay(name, delay));
                }
            });
        }
    }

    pub fn open_node_picker(&mut self) {
        if self.ui_state.active_page != Tab::Proxies {
            return;
        }
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            self.status_msg = Some("selected group has no nodes".into());
            return;
        }
        let current = self.selected_proxy_current_node();
        self.ui_state.proxies.selected_node_idx =
            nodes.iter().position(|node| node == &current).unwrap_or(0);
        self.ui_state.proxies.node_picker_open = true;
        self.status_msg = Some("node picker opened".into());
    }

    pub fn close_node_picker(&mut self) {
        if self.ui_state.proxies.node_picker_open {
            self.ui_state.proxies.node_picker_open = false;
            self.status_msg = Some("node picker closed".into());
        }
    }

    pub fn select_node_down(&mut self) {
        let len = self.selected_proxy_nodes().len();
        if len == 0 {
            self.ui_state.proxies.selected_node_idx = 0;
            return;
        }
        self.ui_state.proxies.selected_node_idx =
            (self.ui_state.proxies.selected_node_idx + 1) % len;
    }

    pub fn select_node_up(&mut self) {
        let len = self.selected_proxy_nodes().len();
        if len == 0 {
            self.ui_state.proxies.selected_node_idx = 0;
            return;
        }
        self.ui_state.proxies.selected_node_idx = if self.ui_state.proxies.selected_node_idx == 0 {
            len - 1
        } else {
            self.ui_state.proxies.selected_node_idx - 1
        };
    }

    pub fn confirm_selected_node(&mut self) {
        let Some(group_name) = self.selected_proxy_group_name() else {
            return;
        };
        let nodes = self.selected_proxy_nodes();
        if nodes.is_empty() {
            return;
        }
        let target = nodes[self.ui_state.proxies.selected_node_idx.min(nodes.len() - 1)].clone();
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.switch_proxy(&group_name, &target).await;
            let switched = result.is_ok();
            let _ = tx.send(DataEvent::SwitchResult(result));
            if switched {
                if let Ok(delay) = api.test_delay(&target).await {
                    let _ = tx.send(DataEvent::Delay(target, delay));
                }
            }
        });
        self.ui_state.proxies.node_picker_open = false;
    }

    pub fn toggle_sort(&mut self) {
        self.ui_state.proxies.sort_by_delay = !self.ui_state.proxies.sort_by_delay;
        self.clamp_proxy_selection();
    }

    pub fn cycle_proxy_mode(&mut self) {
        let next = next_mode(&self.proxy_mode_str);
        let api_mode = next.to_lowercase();
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.set_mode(&api_mode).await.map(|_| next);
            let _ = tx.send(DataEvent::ModeResult(result));
        });
    }

    pub fn test_all_delays(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        let targets = if self.ui_state.proxies.node_picker_open {
            self.selected_proxy_nodes()
        } else {
            self.visible_proxy_groups()
                .into_iter()
                .map(|(name, _)| name)
                .collect()
        };
        for name in targets {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                if let Ok(delay) = api.test_delay(&name).await {
                    let _ = tx.send(DataEvent::Delay(name, delay));
                }
            });
        }
    }

    fn selected_delay_target(&self) -> Option<String> {
        if self.ui_state.proxies.node_picker_open {
            let nodes = self.selected_proxy_nodes();
            nodes
                .get(
                    self.ui_state
                        .proxies
                        .selected_node_idx
                        .min(nodes.len().saturating_sub(1)),
                )
                .cloned()
        } else {
            self.selected_proxy_group_name()
        }
    }
}
