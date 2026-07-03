use crate::ui::prelude::*;

impl App {
    pub fn toggle_language(&mut self) {
        self.ui_settings.language = self.ui_settings.language.next();
        self.save_ui_settings(format!("language: {}", self.ui_settings.language.label()));
    }

    pub fn cycle_default_page(&mut self) {
        self.ui_settings.default_page = next_default_page(&self.ui_settings.default_page);
        self.save_ui_settings(format!("default page: {}", self.default_page_label()));
    }

    pub fn cycle_theme_preference(&mut self) {
        self.ui_settings.theme = next_theme_key(&self.ui_settings.theme).into();
        apply_theme_key(&self.ui_settings.theme);
        self.save_ui_settings(format!("theme: {}", theme_label(&self.ui_settings.theme)));
    }

    pub fn cycle_refresh_interval(&mut self) {
        self.ui_settings.refresh_interval_secs =
            next_refresh_interval_secs(self.ui_settings.refresh_interval_secs);
        self.save_ui_settings(format!(
            "refresh interval: {}",
            self.ui_settings.refresh_interval_label()
        ));
    }

    pub fn cycle_default_traffic_range(&mut self) {
        self.ui_state.traffic.range =
            TrafficRange::from_setting_key(&self.ui_settings.traffic_default_range).next();
        self.ui_settings.traffic_default_range = self.ui_state.traffic.range.setting_key().into();
        self.reset_traffic_view_window();
        self.save_ui_settings(format!(
            "default traffic range: {}",
            self.ui_state.traffic.range.label()
        ));
    }

    pub fn cycle_default_traffic_chart(&mut self) {
        self.ui_state.traffic.chart =
            TrafficChartKind::from_setting_key(&self.ui_settings.traffic_default_chart).next();
        self.ui_settings.traffic_default_chart = self.ui_state.traffic.chart.setting_key().into();
        self.save_ui_settings(format!(
            "default traffic chart: {}",
            self.ui_state.traffic.chart.label()
        ));
    }

    pub fn cycle_default_traffic_dimension(&mut self) {
        self.ui_state.traffic.dimension =
            TrafficDimension::from_setting_key(&self.ui_settings.traffic_default_dimension).next();
        self.ui_settings.traffic_default_dimension =
            self.ui_state.traffic.dimension.setting_key().into();
        self.ui_state.traffic.filter_key = None;
        self.ui_state.traffic.selected_idx = 0;
        self.reset_traffic_view_window();
        self.save_ui_settings(format!(
            "default traffic dimension: {}",
            self.ui_state.traffic.dimension.label()
        ));
    }

    pub fn toggle_mouse_preference(&mut self) {
        self.ui_settings.mouse_enabled = !self.ui_settings.mouse_enabled;
        let state = if self.ui_settings.mouse_enabled {
            "enabled"
        } else {
            "disabled"
        };
        self.save_ui_settings(format!("mouse capture: {state} after restart"));
    }

    pub fn toggle_dangerous_confirmations(&mut self) {
        if self.ui_settings.confirm_dangerous_actions {
            self.ui_state.modals.pending_confirmation = Some(PendingConfirmation {
                title: self.confirm_title(self.t(Msg::SettingsConfirmDanger)),
                message: self.t(Msg::ConfirmDisableDangerous).into(),
                action: PendingAction::ToggleDangerousConfirmations,
            });
            self.status_msg = Some(self.confirm_hint());
            return;
        }
        self.apply_dangerous_confirmations_toggle();
    }

    pub(crate) fn apply_dangerous_confirmations_toggle(&mut self) {
        self.ui_settings.confirm_dangerous_actions = !self.ui_settings.confirm_dangerous_actions;
        let state = if self.ui_settings.confirm_dangerous_actions {
            "on"
        } else {
            "off"
        };
        self.save_ui_settings(format!("dangerous confirmations: {state}"));
    }

    pub(crate) fn save_ui_settings(&mut self, success_msg: String) {
        match self.ui_settings.save() {
            Ok(()) => {
                self.error_msg = None;
                self.status_msg = Some(success_msg);
            }
            Err(e) => {
                self.error_msg = Some(self.settings_save_failed(&e));
            }
        }
    }

    pub(crate) fn default_page_label(&self) -> String {
        if is_auto_default_page(&self.ui_settings.default_page) {
            return "auto".into();
        }
        Tab::from_setting_key(&self.ui_settings.default_page)
            .map(|tab| self.t(tab.msg()).to_string())
            .unwrap_or_else(|| self.ui_settings.default_page.clone())
    }
}
