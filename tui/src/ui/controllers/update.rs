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
        if self.ui_state.active_page == Tab::Logs && !self.log_paused {
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
        let level = self.log_level.api_level().to_string();
        self.rt.spawn(async move {
            let result = api.get_logs(&level).await;
            let _ = tx.send(DataEvent::Logs(result));
        });
    }

    pub(crate) fn fetch_traffic(&mut self) {
        let tx = self.data_tx.clone();
        let range = self.traffic_range.range_arg().to_string();
        let step = self.traffic_range.step_arg().to_string();
        let by = self.traffic_dimension.arg().to_string();
        let key = self.traffic_filter_key.clone();
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
                self.traffic_history
                    .push(resp.upload_total as f64, resp.download_total as f64);
            }
            DataEvent::Connections(Err(_)) => {}
            DataEvent::Version(Ok(info)) => {
                self.version = info.version.unwrap_or_default();
                self.mode = mode_display(&info.mode);
                self.proxy_mode_str = self.mode.clone();
            }
            DataEvent::Version(Err(_)) => {}
            DataEvent::Logs(Ok(entries)) => {
                if self.log_paused {
                    return;
                }
                for e in entries {
                    let line = format!(
                        "{} {}",
                        e.level.to_uppercase(),
                        &e.payload[..e.payload.len().min(120)]
                    );
                    self.logs.push(line);
                }
                if self.logs.len() > 500 {
                    self.logs.drain(0..self.logs.len() - 500);
                }
                self.log_scroll = self.visible_logs().len().saturating_sub(1);
            }
            DataEvent::Logs(Err(_)) => {}
            DataEvent::Delay(name, delay) => {
                self.delays.insert(name, delay);
            }
            DataEvent::SwitchResult(Ok(())) => {
                self.error_msg = None;
                self.status_msg = Some("proxy switched".into());
                self.refresh_data();
            }
            DataEvent::SwitchResult(Err(e)) => {
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
                self.subscription_output.clear();
                self.refresh_subscriptions();
                self.refresh_data();
            }
            DataEvent::SubscriptionResult(Err(e)) => {
                self.error_msg = Some(format!("Subscription action failed: {}", e));
                self.subscription_output = command_output_lines(&e);
            }
            DataEvent::SubscriptionOutputResult(label, Ok(output)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("subscription action completed: {}", label));
                self.subscription_output = command_output_lines(&output);
            }
            DataEvent::SubscriptionOutputResult(label, Err(e)) => {
                self.error_msg = Some(format!("Subscription action failed: {}: {}", label, e));
                self.subscription_output = command_output_lines(&e);
            }
            DataEvent::NetworkResult(label, Ok(output)) => {
                self.clear_sudo_candidate(&label);
                self.error_msg = None;
                self.status_msg = Some(format!("network action completed: {}", label));
                self.network_output = command_output_lines(&output);
                self.network_output_scroll = 0;
                self.tun_enabled = crate::api::read_tun_status();
                self.refresh_data();
            }
            DataEvent::NetworkResult(label, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.network_output = command_output_lines("sudo password required");
                    self.network_output_scroll = 0;
                    return;
                }
                self.error_msg = Some(format!("Network action failed: {}: {}", label, e));
                self.network_output = command_output_lines(&e);
                self.network_output_scroll = 0;
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
                self.settings_output = command_output_lines(&output);
                self.refresh_data();
            }
            DataEvent::SettingsResult(action, Err(e)) => {
                let label = action.label();
                if self.maybe_open_sudo_prompt(label, &e) {
                    self.settings_output = command_output_lines("sudo password required");
                    return;
                }
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.settings_output = command_output_lines(&redact_sensitive_output(&e));
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
                self.settings_output = command_output_lines(&output);
                self.refresh_data();
            }
            DataEvent::SettingsCommandResult(label, _redact_output, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.settings_output = command_output_lines("sudo password required");
                    return;
                }
                self.error_msg = Some(format!("Settings action failed: {}: {}", label, e));
                self.settings_output = command_output_lines(&redact_sensitive_output(&e));
            }
            DataEvent::Traffic(Ok(snapshot)) => {
                self.traffic_points = snapshot.history;
                self.traffic_top = snapshot.top;
                self.traffic_status = snapshot.status;
                self.traffic_error = None;
                self.clamp_traffic_selection();
                self.clamp_traffic_window();
            }
            DataEvent::Traffic(Err(e)) => {
                self.traffic_error = Some(e);
            }
            DataEvent::TrafficExport(Ok(path)) => {
                self.error_msg = None;
                self.status_msg = Some(format!("traffic exported: {}", path));
                self.traffic_output = vec![format!("exported: {}", path)];
            }
            DataEvent::TrafficExport(Err(e)) => {
                self.error_msg = Some(format!("Traffic export failed: {}", e));
                self.traffic_output = command_output_lines(&e);
            }
            DataEvent::TrafficActionResult(label, Ok(output)) => {
                self.clear_sudo_candidate(&label);
                self.error_msg = None;
                self.status_msg = Some(format!("traffic action completed: {}", label));
                self.traffic_output = command_output_lines(&output);
                self.refresh_traffic();
            }
            DataEvent::TrafficActionResult(label, Err(e)) => {
                if self.maybe_open_sudo_prompt(&label, &e) {
                    self.traffic_output = command_output_lines("sudo password required");
                    return;
                }
                self.error_msg = Some(format!("Traffic action failed: {}: {}", label, e));
                self.traffic_output = command_output_lines(&e);
            }
        }
    }

    pub fn on_shutdown(&mut self) {
        self.window.save();
        if let Err(e) = self.ui_settings.save() {
            self.error_msg = Some(format!("save settings failed: {}", e));
        }
    }
}
