use crate::ui::prelude::*;

impl App {
    pub fn test_selected_delay(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        if self.ui_state.active_page == Tab::Proxies {
            let Some(name) = self.selected_delay_target() else {
                return;
            };
            self.mark_delay_pending(&name);
            self.status_msg = Some(format!("testing delay: {}", name));
            self.rt.spawn(async move {
                let result = api.test_delay(&name).await;
                let _ = tx.send(DataEvent::DelayResult(name, result));
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
        if !self.selected_proxy_group_supports_manual_switch() {
            self.test_selected_delay();
            self.status_msg = Some(format!(
                "{}: {} / {}",
                self.t(Msg::ProxyAutoGroupKeepsChoice),
                group_name,
                target
            ));
            return;
        }
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        let event_group = group_name.clone();
        let event_target = target.clone();
        let status_target = target.clone();
        self.rt.spawn(async move {
            let result = api.switch_proxy(&group_name, &target).await;
            let switched = result.is_ok();
            let _ = tx.send(DataEvent::SwitchResult(event_group, event_target, result));
            if switched {
                let delay_result = api.test_delay(&target).await;
                let _ = tx.send(DataEvent::DelayResult(target, delay_result));
            }
        });
        self.status_msg = Some(format!("switching node: {}", status_target));
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
        let mut targets = if self.ui_state.proxies.node_picker_open {
            self.selected_proxy_nodes()
        } else {
            self.visible_proxy_groups()
                .into_iter()
                .map(|(name, _)| name)
                .collect()
        };
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            return;
        }
        for name in &targets {
            self.mark_delay_pending(name);
        }
        self.status_msg = Some(format!("testing delay: {} targets", targets.len()));
        for name in targets {
            let api = api.clone();
            let tx = tx.clone();
            self.rt.spawn(async move {
                let result = api.test_delay(&name).await;
                let _ = tx.send(DataEvent::DelayResult(name, result));
            });
        }
    }

    pub(crate) fn mark_proxy_group_current(&mut self, group: &str, target: &str) {
        if let Some(info) = self.proxies.get_mut(group) {
            info.now = Some(target.to_string());
        }
        for (name, current) in &mut self.proxy_groups {
            if name == group {
                *current = target.to_string();
            }
        }
        let nodes = self.selected_proxy_nodes();
        if let Some(idx) = nodes.iter().position(|node| node == target) {
            self.ui_state.proxies.selected_node_idx = idx;
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

    pub(crate) fn selected_proxy_group_type(&self) -> Option<&str> {
        let group_name = self.selected_proxy_group_name()?;
        self.proxies
            .get(&group_name)
            .map(|info| info.proxy_type.as_str())
    }

    pub(crate) fn selected_proxy_group_supports_manual_switch(&self) -> bool {
        matches!(self.selected_proxy_group_type(), Some("Selector"))
    }

    pub(crate) fn mark_delay_pending(&mut self, name: &str) {
        self.delay_pending.insert(name.to_string());
        self.delay_errors.remove(name);
    }

    pub(crate) fn proxy_delay_text(&self, name: &str) -> String {
        if self.delay_pending.contains(name) {
            return self.t(Msg::ProxyDelayTesting).to_string();
        }
        if let Some(error) = self.delay_errors.get(name) {
            if delay_error_is_timeout(error) {
                return self.t(Msg::ProxyDelayTimeout).to_string();
            }
            return self.t(Msg::ProxyDelayError).to_string();
        }
        self.delays
            .get(name)
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "—".into())
    }
}

pub(crate) fn delay_error_is_timeout(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("timeout") || lower.contains("timed out") || lower.contains("504")
}
