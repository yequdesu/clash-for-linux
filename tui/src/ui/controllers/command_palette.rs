use crate::ui::prelude::*;

impl App {
    pub fn command_palette_active(&self) -> bool {
        self.ui_state.command_palette.open
    }

    pub fn open_command_palette(&mut self) {
        if self.ui_state.modals.pending_confirmation.is_some()
            || self.ui_state.settings.prompt.is_some()
            || self.subscription_input_active()
        {
            self.status_msg = Some(self.t(Msg::DialogCloseFirst).into());
            return;
        }
        self.ui_state.command_palette.open = true;
        self.ui_state.command_palette.query.clear();
        self.ui_state.command_palette.selected_idx = 0;
        self.ui_state.proxies.search_active = false;
        self.show_help = false;
        self.error_msg = None;
    }

    pub fn close_command_palette(&mut self) {
        self.ui_state.command_palette.open = false;
        self.ui_state.command_palette.query.clear();
        self.ui_state.command_palette.selected_idx = 0;
    }

    pub fn push_command_palette_char(&mut self, c: char) {
        self.ui_state.command_palette.query.push(c);
        self.ui_state.command_palette.selected_idx = 0;
    }

    pub fn pop_command_palette_char(&mut self) {
        self.ui_state.command_palette.query.pop();
        self.ui_state.command_palette.selected_idx = 0;
    }

    pub fn command_palette_select_down(&mut self) {
        let len = self.command_palette_matches().len();
        if len > 0 {
            self.ui_state.command_palette.selected_idx =
                (self.ui_state.command_palette.selected_idx + 1).min(len - 1);
        }
    }

    pub fn command_palette_select_up(&mut self) {
        self.ui_state.command_palette.selected_idx =
            self.ui_state.command_palette.selected_idx.saturating_sub(1);
    }

    pub fn submit_command_palette(&mut self) {
        let matches = self.command_palette_matches();
        let Some(spec) = matches
            .get(
                self.ui_state
                    .command_palette
                    .selected_idx
                    .min(matches.len().saturating_sub(1)),
            )
            .copied()
        else {
            self.error_msg = Some("no matching command".into());
            return;
        };
        self.close_command_palette();
        self.execute_action_spec(spec);
    }

    pub(crate) fn command_palette_matches(&self) -> Vec<&'static action_registry::ActionSpec> {
        let query = self.ui_state.command_palette.query.trim().to_lowercase();
        action_registry::all_actions()
            .iter()
            .filter(|spec| {
                if query.is_empty() {
                    return true;
                }
                let page = self.t(spec.page.msg()).to_lowercase();
                let label = self.action_label(spec).to_lowercase();
                let desc = self.action_desc(spec).to_lowercase();
                let button = self.action_button(spec).to_lowercase();
                let haystack = format!(
                    "{} {} {} {} {} {} {} {} {}",
                    spec.id,
                    spec.label,
                    label,
                    button,
                    spec.shortcut,
                    page,
                    spec.description,
                    desc,
                    spec.executor.command()
                )
                .to_lowercase();
                haystack.contains(&query)
            })
            .collect()
    }

    pub(crate) fn execute_action_spec(&mut self, spec: &action_registry::ActionSpec) {
        if spec.id.starts_with("nav.") {
            self.ui_state.active_page = spec.page;
            self.show_help = false;
            self.ui_state.proxies.node_picker_open = false;
            self.reset_selection();
            if self.ui_state.active_page == Tab::Subscriptions {
                self.refresh_subscriptions();
            }
            if self.ui_state.active_page == Tab::Traffic {
                self.refresh_traffic();
            }
            self.status_msg = Some(format!("page: {}", self.t(self.ui_state.active_page.msg())));
            return;
        }
        if self.ui_state.active_page != spec.page {
            self.ui_state.active_page = spec.page;
            self.show_help = false;
            self.ui_state.proxies.node_picker_open = false;
            self.reset_selection();
            if self.ui_state.active_page == Tab::Subscriptions {
                self.refresh_subscriptions();
            }
            if self.ui_state.active_page == Tab::Traffic {
                self.refresh_traffic();
            }
        }
        if let Some(action) = crate::ui::components::action_bar::hitbox_for_action_spec(spec) {
            self.dispatch_hitbox_action(action);
            return;
        }
        match spec.id {
            "sub.use" => self.use_selected_subscription(),
            "sub.update" => self.update_selected_subscription(),
            "sub.edit" => self.begin_subscription_profile_edit(),
            "sub.remove" => self.remove_selected_subscription(),
            "proxy.mode.cycle" => self.cycle_proxy_mode(),
            "proxy.node.switch" => self.switch_selected(),
            "proxy.delay.selected" => self.test_selected_delay(),
            "conn.close.selected" => self.close_selected_connection(),
            "conn.close.all" => self.close_all_connections(),
            "logs.pause" => self.toggle_log_pause(),
            "logs.filter" => self.cycle_log_level(),
            _ => {
                self.status_msg = Some(format!("command not implemented yet: {}", spec.id));
            }
        }
    }
}
