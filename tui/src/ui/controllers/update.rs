use crate::ui::prelude::*;

impl App {
    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        let now = chrono::Utc::now().timestamp();
        let refresh_interval = self.ui_settings.effective_refresh_interval_secs() as i64;
        if now - self.last_refresh >= refresh_interval {
            self.last_refresh = now;
            self.refresh_data();
        }
    }

    pub fn refresh_data(&mut self) {
        self.fetch_proxies();
        self.fetch_connections();
        self.fetch_version();
        self.refresh_subscriptions();
        self.tun_enabled = crate::api::read_tun_status();
        if self.tick_count.is_multiple_of(3) && !self.proxy_groups.is_empty() {
            self.test_selected_delay();
        }
        if self.ui_state.active_page == Tab::Logs && !self.ui_state.logs.paused {
            self.fetch_logs();
        }
        if self.ui_state.active_page == Tab::Traffic && self.tick_count.is_multiple_of(3) {
            self.fetch_traffic();
        }
    }

    pub(crate) fn fetch_proxies(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_proxies().await;
            let _ = tx.send(DataEvent::Proxies(result));
        });
    }

    pub(crate) fn fetch_connections(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_connections().await;
            let _ = tx.send(DataEvent::Connections(result));
        });
    }

    pub(crate) fn fetch_version(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = api.get_version().await;
            let _ = tx.send(DataEvent::Version(result));
        });
    }

    pub(crate) fn fetch_logs(&mut self) {
        let api = self.api.clone();
        let tx = self.data_tx.clone();
        let level = self.ui_state.logs.level.api_level().to_string();
        self.rt.spawn(async move {
            let result = api.get_logs(&level).await;
            let _ = tx.send(DataEvent::Logs(result));
        });
    }

    pub(crate) fn fetch_traffic(&mut self) {
        let tx = self.data_tx.clone();
        let range = self.ui_state.traffic.range.range_arg().to_string();
        let step = self.ui_state.traffic.range.step_arg().to_string();
        let by = self.ui_state.traffic.dimension.arg().to_string();
        let key = self.ui_state.traffic.filter_key.clone();
        self.rt.spawn(async move {
            let result =
                crate::api::read_traffic_snapshot(&range, &step, &by, key.as_deref()).await;
            let _ = tx.send(DataEvent::Traffic(result));
        });
    }

    pub fn apply_data_event(&mut self, event: DataEvent) {
        match event {
            DataEvent::Proxies(Ok(resp)) => {
                self.proxies = resp.proxies;
                self.proxy_groups.clear();
                for (name, info) in &self.proxies {
                    if info.proxy_type == "Selector"
                        || info.proxy_type == "Fallback"
                        || info.proxy_type == "URLTest"
                    {
                        let current = info.now.clone().unwrap_or_default();
                        self.proxy_groups.push((name.clone(), current));
                    }
                }
                self.clamp_proxy_selection();
            }
            DataEvent::Proxies(Err(e)) => {
                if self.error_msg.is_none() {
                    self.error_msg = Some(e);
                }
            }
            DataEvent::Connections(Ok(resp)) => {
                self.prev_upload = self.upload_total;
                self.prev_download = self.download_total;
                self.upload_total = resp.upload_total;
                self.download_total = resp.download_total;
                self.upload_rate = (self.upload_total.saturating_sub(self.prev_upload)) as f64;
                self.download_rate =
                    (self.download_total.saturating_sub(self.prev_download)) as f64;
                let conn_len = resp.connections.len();
                self.connections = resp.connections;
                self.connections_active = self.connections.len();
                self.connections_total = conn_len;
            }
            DataEvent::Connections(Err(_)) => {}
            DataEvent::Version(Ok(info)) => {
                self.version = info.version.unwrap_or_default();
                self.mode = mode_display(&info.mode);
                self.proxy_mode_str = self.mode.clone();
            }
            DataEvent::Version(Err(_)) => {}
            DataEvent::Logs(Ok(entries)) => {
                if self.ui_state.logs.paused {
                    return;
                }
                for e in entries {
                    let line = format!(
                        "{} {}",
                        e.level.to_uppercase(),
                        trim_log_payload(&e.payload)
                    );
                    if !self.logs.iter().rev().take(500).any(|old| old == &line) {
                        self.logs.push(line);
                    }
                }
                if self.logs.len() > 500 {
                    self.logs.drain(0..self.logs.len() - 500);
                }
                self.ui_state.logs.scroll = self.visible_logs().len().saturating_sub(1);
            }
            DataEvent::Logs(Err(_)) => {}
            DataEvent::DelayResult(name, Ok(delay)) => {
                self.delays.insert(name.clone(), delay);
                self.delay_pending.remove(&name);
                self.delay_errors.remove(&name);
                self.status_msg = Some(format!("delay updated: {} {}ms", name, delay));
            }
            DataEvent::DelayResult(name, Err(e)) => {
                self.delay_pending.remove(&name);
                self.delay_errors.insert(name.clone(), e.clone());
                let label = if crate::ui::controllers::proxies::delay_error_is_timeout(&e) {
                    self.t(Msg::ProxyDelayTimeout)
                } else {
                    self.t(Msg::ProxyDelayError)
                }
                .to_string();
                self.status_msg = Some(format!("delay {}: {}", label, name));
            }
            DataEvent::SwitchResult(group, target, Ok(())) => {
                self.error_msg = None;
                self.status_msg = Some("proxy switched".into());
                self.mark_proxy_group_current(&group, &target);
                self.mark_delay_pending(&target);
                self.refresh_data();
            }
            DataEvent::SwitchResult(_group, _target, Err(e)) => {
                self.error_msg = Some(format!("Switch failed: {}", e));
            }
            DataEvent::ModeResult(Ok(mode)) => {
                self.mode = mode.clone();
                self.proxy_mode_str = mode;
                self.error_msg = None;
                self.status_msg = Some("mode switched".into());
                self.refresh_data();
            }
            DataEvent::ModeResult(Err(e)) => {
                self.error_msg = Some(format!("Mode switch failed: {}", e));
            }
            DataEvent::SubscriptionResult(Ok(msg)) => {
                self.error_msg = None;
                self.status_msg = Some(msg);
                self.ui_state.modals.sudo_candidate = None;
                self.ui_state.subscriptions.output.clear();
                self.refresh_subscriptions();
                self.refresh_data();
            }
            DataEvent::SubscriptionResult(Err(e)) => {
                if self.maybe_open_pending_sudo_prompt(&e) {
                    self.ui_state.subscriptions.output =
                        command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.ui_state.modals.sudo_candidate = None;
                self.error_msg = Some(format!("Subscription action failed: {}", e));
                self.ui_state.subscriptions.output = command_output_lines(&e);
                self.reveal_command_output();
            }
            DataEvent::SubscriptionOutputResult(label, Ok(output)) => {
                self.error_msg = None;
                self.ui_state.modals.sudo_candidate = None;
                self.status_msg = Some(format!("subscription action completed: {}", label));
                self.ui_state.subscriptions.output = command_output_lines(&output);
                self.reveal_command_output();
            }
            DataEvent::SubscriptionOutputResult(label, Err(e)) => {
                if self.maybe_open_pending_sudo_prompt(&e) {
                    self.ui_state.subscriptions.output =
                        command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.ui_state.modals.sudo_candidate = None;
                self.error_msg = Some(format!("Subscription action failed: {}: {}", label, e));
                self.ui_state.subscriptions.output = command_output_lines(&e);
                self.reveal_command_output();
            }
            DataEvent::NetworkResult(label, Ok(output)) => {
                self.clear_sudo_candidate(&label);
                self.error_msg = None;
                self.status_msg = Some(format!("network action completed: {}", label));
                self.network_output = command_output_lines(&output);
                self.reveal_command_output();
                self.tun_enabled = crate::api::read_tun_status();
                self.refresh_data();
            }
            DataEvent::NetworkResult(label, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.network_output = command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.error_msg = Some(format!("Network action failed: {}: {}", label, e));
                self.network_output = command_output_lines(&e);
                self.reveal_command_output();
                self.tun_enabled = crate::api::read_tun_status();
            }
            DataEvent::SettingsResult(action, Ok(output)) => {
                self.error_msg = None;
                let label = action.label();
                self.clear_sudo_candidate(label);
                self.status_msg = Some(format!("settings action completed: {}", label));
                let output = if action.redact_output() {
                    redact_sensitive_output(&output)
                } else {
                    output
                };
                self.ui_state.settings.output = command_output_lines(&output);
                self.reveal_command_output();
                self.refresh_data();
            }
            DataEvent::SettingsResult(action, Err(e)) => {
                let label = action.label();
                if self.maybe_open_sudo_prompt(label, &e) {
                    self.ui_state.settings.output =
                        command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.ui_state.settings.output = command_output_lines(&redact_sensitive_output(&e));
                self.reveal_command_output();
            }
            DataEvent::SettingsCommandResult(label, redact_output, Ok(output)) => {
                self.clear_sudo_candidate(&label);
                self.error_msg = None;
                self.status_msg = Some(format!("settings action completed: {}", label));
                let output = if redact_output {
                    redact_sensitive_output(&output)
                } else {
                    output
                };
                self.ui_state.settings.output = command_output_lines(&output);
                self.reveal_command_output();
                self.refresh_data();
            }
            DataEvent::SettingsCommandResult(label, _redact_output, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.ui_state.settings.output =
                        command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.ui_state.settings.output = command_output_lines(&redact_sensitive_output(&e));
                self.reveal_command_output();
            }
            DataEvent::Traffic(Ok(snapshot)) => {
                self.traffic_points = snapshot.history;
                self.traffic_top = snapshot.top;
                self.traffic_status = snapshot.status;
                self.ui_state.traffic.error = None;
                self.clamp_traffic_selection();
                self.clamp_traffic_window();
            }
            DataEvent::Traffic(Err(e)) => {
                self.ui_state.traffic.error = Some(e);
            }
            DataEvent::TrafficExport(Ok(path)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("traffic exported: {}", path));
                self.ui_state.traffic.output = vec![format!("exported: {}", path)];
                self.reveal_command_output();
            }
            DataEvent::TrafficExport(Err(e)) => {
                self.error_msg = Some(format!("Traffic export failed: {}", e));
                self.ui_state.traffic.output = command_output_lines(&e);
                self.reveal_command_output();
            }
            DataEvent::TrafficActionResult(label, Ok(output)) => {
                self.clear_sudo_candidate(&label);
                self.error_msg = None;
                self.status_msg = Some(format!("traffic action completed: {}", label));
                self.ui_state.traffic.output = command_output_lines(&output);
                self.reveal_command_output();
                self.refresh_traffic();
            }
            DataEvent::TrafficActionResult(label, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.ui_state.traffic.output =
                        command_output_lines(self.t(Msg::SudoPasswordRequired));
                    self.reveal_command_output();
                    return;
                }
                self.error_msg = Some(format!("Traffic action failed: {}: {}", label, e));
                self.ui_state.traffic.output = command_output_lines(&e);
                self.reveal_command_output();
            }
        }
    }

    pub fn on_shutdown(&mut self) {
        self.window.save();
        if let Err(e) = self.ui_settings.save() {
            self.error_msg = Some(self.settings_save_failed(&e));
        }
    }
}

pub(crate) fn trim_log_payload(payload: &str) -> String {
    let max_chars = 240;
    let mut chars = payload.chars();
    let trimmed: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", trimmed)
    } else {
        trimmed
    }
}
