use crate::ui::prelude::*;

impl App {
    pub(crate) fn lock_traffic_bucket(&mut self, idx: usize) {
        if self.traffic_points.is_empty() {
            return;
        }
        let idx = idx.min(self.traffic_points.len().saturating_sub(1));
        self.ui_state.traffic.locked_bucket = Some(idx);
        if let Some(point) = self.traffic_points.get(idx) {
            self.status_msg = Some(format!(
                "traffic bucket locked: {}",
                point.ts.format("%H:%M")
            ));
        }
    }

    pub(crate) fn clear_traffic_bucket_lock(&mut self) {
        if self.ui_state.traffic.locked_bucket.take().is_some() {
            self.status_msg = Some("traffic bucket lock cleared".into());
        }
    }

    pub(crate) fn pan_traffic_window(&mut self, amount: i32) {
        if self.traffic_points.is_empty() {
            self.ui_state.traffic.window_offset = 0;
            return;
        }
        if amount > 0 {
            self.ui_state.traffic.window_offset = self
                .ui_state
                .traffic
                .window_offset
                .saturating_add(amount as usize)
                .min(self.traffic_points.len().saturating_sub(1));
        } else {
            self.ui_state.traffic.window_offset = self
                .ui_state
                .traffic
                .window_offset
                .saturating_sub(amount.unsigned_abs() as usize);
        }
        self.status_msg = Some(if self.ui_state.traffic.window_offset == 0 {
            "traffic window: latest".into()
        } else {
            format!(
                "traffic window: -{} buckets",
                self.ui_state.traffic.window_offset
            )
        });
    }

    pub(crate) fn reset_traffic_view_window(&mut self) {
        self.ui_state.traffic.window_offset = 0;
        self.ui_state.traffic.locked_bucket = None;
    }

    pub fn refresh_traffic(&mut self) {
        self.fetch_traffic();
    }

    pub fn next_traffic_range(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.ui_state.traffic.range = self.ui_state.traffic.range.next();
        self.ui_state.traffic.filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!(
            "traffic range: {}",
            self.ui_state.traffic.range.label()
        ));
        self.fetch_traffic();
    }

    pub fn prev_traffic_range(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.ui_state.traffic.range = self.ui_state.traffic.range.prev();
        self.ui_state.traffic.filter_key = None;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!(
            "traffic range: {}",
            self.ui_state.traffic.range.label()
        ));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_chart(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.ui_state.traffic.chart = self.ui_state.traffic.chart.next();
        self.status_msg = Some(format!(
            "traffic chart: {}",
            self.ui_state.traffic.chart.label()
        ));
    }

    pub fn next_traffic_dimension(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        self.ui_state.traffic.dimension = self.ui_state.traffic.dimension.next();
        self.ui_state.traffic.filter_key = None;
        self.ui_state.traffic.selected_idx = 0;
        self.reset_traffic_view_window();
        self.status_msg = Some(format!(
            "traffic dimension: {}",
            self.ui_state.traffic.dimension.label()
        ));
        self.fetch_traffic();
    }

    pub fn toggle_traffic_filter(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        if self.ui_state.traffic.filter_key.is_some() {
            self.ui_state.traffic.filter_key = None;
            self.reset_traffic_view_window();
            self.status_msg = Some("traffic filter cleared".into());
            self.fetch_traffic();
            return;
        }
        let Some(row) = self.traffic_top.get(self.ui_state.traffic.selected_idx) else {
            return;
        };
        let key = row.key.clone();
        self.ui_state.traffic.filter_key = Some(key.clone());
        self.reset_traffic_view_window();
        self.status_msg = Some(format!("traffic filter: {}", trunc_str(&key, 48)));
        self.fetch_traffic();
    }

    pub fn export_traffic_csv(&mut self) {
        if self.ui_state.active_page != Tab::Traffic {
            return;
        }
        let range = self.ui_state.traffic.range.range_arg().to_string();
        let by = self.ui_state.traffic.dimension.arg().to_string();
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!(
            "exporting traffic: {} / {}",
            self.ui_state.traffic.range.label(),
            self.ui_state.traffic.dimension.label()
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
            self.ui_state.modals.pending_confirmation = Some(PendingConfirmation {
                title: self.confirm_title(action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Traffic(action),
            });
            self.status_msg = Some(self.confirm_hint());
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
