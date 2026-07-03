use crate::ui::prelude::*;

impl App {
    pub(crate) fn lock_traffic_bucket(&mut self, idx: usize) {
        if self.traffic_points.is_empty() {
            return;
        }
        let idx = idx.min(self.traffic_points.len().saturating_sub(1));
        self.traffic_locked_bucket = Some(idx);
        if let Some(point) = self.traffic_points.get(idx) {
            self.status_msg = Some(format!(
                "traffic bucket locked: {}",
                point.ts.format("%H:%M")
            ));
        }
    }

    pub(crate) fn clear_traffic_bucket_lock(&mut self) {
        if self.traffic_locked_bucket.take().is_some() {
            self.status_msg = Some("traffic bucket lock cleared".into());
        }
    }

    pub(crate) fn pan_traffic_window(&mut self, amount: i32) {
        if self.traffic_points.is_empty() {
            self.traffic_window_offset = 0;
            return;
        }
        if amount > 0 {
            self.traffic_window_offset = self
                .traffic_window_offset
                .saturating_add(amount as usize)
                .min(self.traffic_points.len().saturating_sub(1));
        } else {
            self.traffic_window_offset = self
                .traffic_window_offset
                .saturating_sub(amount.unsigned_abs() as usize);
        }
        self.status_msg = Some(if self.traffic_window_offset == 0 {
            "traffic window: latest".into()
        } else {
            format!("traffic window: -{} buckets", self.traffic_window_offset)
        });
    }

    pub(crate) fn reset_traffic_view_window(&mut self) {
        self.traffic_window_offset = 0;
        self.traffic_locked_bucket = None;
    }

    pub fn refresh_traffic(&mut self) {
        self.fetch_traffic();
    }

    pub fn next_traffic_range(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.traffic_range = self.traffic_range.next();
        self.traffic_filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic range: {}", self.traffic_range.label()));
        self.fetch_traffic();
    }

    pub fn prev_traffic_range(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.traffic_range = self.traffic_range.prev();
        self.traffic_filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic range: {}", self.traffic_range.label()));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_chart(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.traffic_chart = self.traffic_chart.next();
        self.status_msg = Some(format!("traffic chart: {}", self.traffic_chart.label()));
    }

    pub fn next_traffic_dimension(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.traffic_dimension = self.traffic_dimension.next();
        self.traffic_filter_key = None;
        self.traffic_selected_idx = 0;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!(
            "traffic dimension: {}",
            self.traffic_dimension.label()
        ));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_filter(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        if self.traffic_filter_key.is_some() {
            self.traffic_filter_key = None;
            self.reset_traffic_view_window();
            self.status_msg = Some("traffic filter cleared".into());
            self.fetch_traffic();
            return;
        }
        let Some(row) = self.traffic_top.get(self.traffic_selected_idx) else {
            return;
        };
        let key = row.key.clone();
        self.traffic_filter_key = Some(key.clone());
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic filter: {}", trunc_str(&key, 48)));
        self.fetch_traffic();
    }

    pub fn export_traffic_csv(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        let range = self.traffic_range.range_arg().to_string();
        let by = self.traffic_dimension.arg().to_string();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!(
            "exporting traffic: {} / {}",
            self.traffic_range.label(),
            self.traffic_dimension.label()
        ));
        self.rt.spawn(async move {
            let result = crate::api::export_traffic_csv(&range, &by).await;
            let _ = tx.send(DataEvent::TrafficExport(result));
        });
    }

    pub fn run_traffic_action(&mut self, action: TrafficAction) {
        if self.ui_state.active_page != Tab::Traffic && self.ui_state.active_page != Tab::Settings {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm {}", action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Traffic(action),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_traffic_action(action);
    }

    pub(crate) fn execute_traffic_action(&mut self, action: TrafficAction) {
        let label = action.label().to_string();
        let args = action.args();
        self.prepare_sudo_candidate(label.clone(), args.clone(), SudoTarget::Traffic);
        let tx = self.data_tx.clone();
        let report_to_settings = self.ui_state.active_page == Tab::Settings;
        self.status_msg = Some(format!("running traffic action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            if report_to_settings {
                let _ = tx.send(DataEvent::SettingsCommandResult(label, true, result));
            } else {
                let _ = tx.send(DataEvent::TrafficActionResult(label, result));
            }
        });
    }
}
