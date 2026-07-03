use crate::ui::prelude::*;

impl App {
    pub fn handle_mouse_event(&mut self, kind: crossterm::event::MouseEventKind, x: u16, y: u16) {
        let action = self.ui_state.hitboxes.action_at(x, y);
        match kind {
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                self.handle_mouse_click(action)
            }
            crossterm::event::MouseEventKind::ScrollDown => self.handle_mouse_scroll(action, 3),
            crossterm::event::MouseEventKind::ScrollUp => self.handle_mouse_scroll(action, -3),
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_click(&mut self, action: Option<HitboxAction>) {
        if self.ui_state.modals.pending_confirmation.is_some()
            && !matches!(
                action,
                Some(HitboxAction::ConfirmPendingAction) | Some(HitboxAction::CancelPendingAction)
            )
        {
            return;
        }
        if self.ui_state.settings.prompt.is_some()
            && !matches!(
                action,
                Some(HitboxAction::FocusSettingsPromptValue)
                    | Some(HitboxAction::SelectSettingsPromptField(_))
                    | Some(HitboxAction::SubmitSettingsPrompt)
                    | Some(HitboxAction::CancelSettingsPrompt)
            )
        {
            return;
        }
        if self.ui_state.modals.sudo_prompt.is_some()
            && !matches!(
                action,
                Some(HitboxAction::FocusSudoPromptValue)
                    | Some(HitboxAction::SubmitSudoPrompt)
                    | Some(HitboxAction::CancelSudoPrompt)
            )
        {
            return;
        }
        if self.subscription_input_active()
            && !matches!(
                action,
                Some(HitboxAction::SubmitSubscriptionPrompt)
                    | Some(HitboxAction::CancelSubscriptionPrompt)
                    | Some(HitboxAction::SelectSubscriptionAddField(_))
            )
        {
            return;
        }
        if self.ui_state.proxies.node_picker_open
            && !matches!(
                action,
                Some(HitboxAction::SelectProxyNode(_))
                    | Some(HitboxAction::SwitchSelectedProxyNode)
                    | Some(HitboxAction::TestSelectedProxyDelay)
                    | Some(HitboxAction::CloseNodePicker)
            )
        {
            return;
        }
        if let Some(action) = action {
            self.dispatch_hitbox_action(action);
        }
    }

    pub(crate) fn dispatch_hitbox_action(&mut self, action: HitboxAction) {
        match action {
            HitboxAction::SwitchTab(tab) => {
                self.ui_state.active_page = tab;
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
            HitboxAction::RunNetwork(action) => self.run_network_action(action),
            HitboxAction::RunSettings(action) => self.run_settings_action(action),
            HitboxAction::BeginSecretSet => self.begin_secret_set(),
            HitboxAction::BeginConfigSetPorts => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
            }
            HitboxAction::BeginConfigSetApi => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
            }
            HitboxAction::BeginConfigSetDns => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
            }
            HitboxAction::BeginConfigSetLan => {
                self.begin_settings_prompt(SettingsPromptKind::ConfigSetLan);
            }
            HitboxAction::BeginTrafficPruneRetention => {
                self.begin_settings_prompt(SettingsPromptKind::TrafficPruneRetention);
            }
            HitboxAction::BeginGeodataUpdateVersion => {
                self.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
            }
            HitboxAction::RunTraffic(action) => self.run_traffic_action(action),
            HitboxAction::SelectProxy(idx) => {
                self.ui_state.active_page = Tab::Proxies;
                self.ui_state.proxies.selected_idx =
                    idx.min(self.visible_proxy_groups().len().saturating_sub(1));
                self.clamp_proxy_selection();
            }
            HitboxAction::SelectProxyNode(idx) => {
                self.ui_state.proxies.selected_node_idx =
                    idx.min(self.selected_proxy_nodes().len().saturating_sub(1));
                self.clamp_node_selection();
            }
            HitboxAction::SelectSubscription(idx) => {
                self.ui_state.subscriptions.selected_idx =
                    idx.min(self.profiles.len().saturating_sub(1));
                self.clamp_subscription_selection();
            }
            HitboxAction::BeginSubscriptionAdd => self.begin_subscription_add(),
            HitboxAction::BeginSubscriptionImport => self.begin_subscription_import(),
            HitboxAction::UseSubscription => self.use_selected_subscription(),
            HitboxAction::UpdateSubscription => self.update_selected_subscription(),
            HitboxAction::RemoveSubscription => self.remove_selected_subscription(),
            HitboxAction::ShowSubscriptionLog => self.show_subscription_log(),
            HitboxAction::EditSubscriptionName => {
                self.begin_subscription_profile_edit();
            }
            HitboxAction::EditSubscriptionInterval => {
                self.begin_subscription_edit(SubscriptionEditField::Interval);
            }
            HitboxAction::EditSubscriptionUrl => {
                self.begin_subscription_edit(SubscriptionEditField::Url);
            }
            HitboxAction::EditSubscriptionUserAgent => {
                self.begin_subscription_edit(SubscriptionEditField::UserAgent);
            }
            HitboxAction::EditSubscriptionUpdateProxy => {
                self.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
            }
            HitboxAction::EditSubscriptionConvertMode => {
                self.begin_subscription_edit(SubscriptionEditField::ConvertMode);
            }
            HitboxAction::EditSubscriptionAddTag => {
                self.begin_subscription_edit(SubscriptionEditField::AddTag);
            }
            HitboxAction::EditSubscriptionRemoveTag => {
                self.begin_subscription_edit(SubscriptionEditField::RemoveTag);
            }
            HitboxAction::SelectSubscriptionAddField(idx) => {
                self.select_subscription_add_field(idx);
            }
            HitboxAction::SelectConnection(idx) => {
                self.ui_state.connections.selected_idx =
                    idx.min(self.connections.len().saturating_sub(1));
            }
            HitboxAction::SelectTrafficRow(idx) => {
                self.ui_state.traffic.selected_idx =
                    idx.min(self.traffic_top.len().saturating_sub(1));
                self.clamp_traffic_selection();
            }
            HitboxAction::SelectTrafficBucket(idx) => self.lock_traffic_bucket(idx),
            HitboxAction::ScrollTrafficChart => self.clear_traffic_bucket_lock(),
            HitboxAction::NextTrafficRange => self.next_traffic_range(),
            HitboxAction::ToggleTrafficChart => self.toggle_traffic_chart(),
            HitboxAction::NextTrafficDimension => self.next_traffic_dimension(),
            HitboxAction::ExportTraffic => self.export_traffic_csv(),
            HitboxAction::SubmitSubscriptionPrompt => self.submit_subscription_prompt(),
            HitboxAction::CancelSubscriptionPrompt => self.cancel_subscription_prompt(),
            HitboxAction::ConfirmPendingAction => self.confirm_pending_action(),
            HitboxAction::CancelPendingAction => self.cancel_pending_action(),
            HitboxAction::FocusSettingsPromptValue => self.focus_settings_prompt_value(),
            HitboxAction::SelectSettingsPromptField(idx) => self.select_settings_prompt_field(idx),
            HitboxAction::SelectSettingsSection(idx) => self.select_settings_section(idx),
            HitboxAction::SubmitSettingsPrompt => self.submit_settings_prompt(),
            HitboxAction::CancelSettingsPrompt => self.cancel_settings_prompt(),
            HitboxAction::FocusSudoPromptValue => self.focus_sudo_prompt_value(),
            HitboxAction::SubmitSudoPrompt => self.submit_sudo_prompt(),
            HitboxAction::CancelSudoPrompt => self.cancel_sudo_prompt(),
            HitboxAction::CycleUiLanguage => self.toggle_language(),
            HitboxAction::CycleThemePreference => self.cycle_theme_preference(),
            HitboxAction::CycleDefaultPage => self.cycle_default_page(),
            HitboxAction::CycleRefreshInterval => self.cycle_refresh_interval(),
            HitboxAction::ToggleMousePreference => self.toggle_mouse_preference(),
            HitboxAction::ToggleDangerousConfirmations => self.toggle_dangerous_confirmations(),
            HitboxAction::CycleDefaultTrafficRange => self.cycle_default_traffic_range(),
            HitboxAction::CycleDefaultTrafficChart => self.cycle_default_traffic_chart(),
            HitboxAction::CycleDefaultTrafficDimension => self.cycle_default_traffic_dimension(),
            HitboxAction::CycleProxyMode => self.cycle_proxy_mode(),
            HitboxAction::ToggleProxySort => self.toggle_sort(),
            HitboxAction::ToggleNodePicker => {
                if self.ui_state.proxies.node_picker_open {
                    self.close_node_picker();
                } else {
                    self.open_node_picker();
                }
            }
            HitboxAction::CloseNodePicker => self.close_node_picker(),
            HitboxAction::SwitchSelectedProxyNode => {
                if self.ui_state.proxies.node_picker_open {
                    self.switch_selected_node();
                } else {
                    self.switch_selected();
                }
            }
            HitboxAction::TestSelectedProxyDelay => self.test_selected_delay(),
            HitboxAction::TestAllProxyDelays => self.test_all_delays(),
            HitboxAction::CloseSelectedConnection => self.close_selected_connection(),
            HitboxAction::CloseAllConnections => self.close_all_connections(),
            HitboxAction::ToggleLogPause => self.toggle_log_pause(),
            HitboxAction::CycleLogFilter => self.cycle_log_level(),
            HitboxAction::ClearLogs => self.clear_logs(),
            HitboxAction::ScrollLogs
            | HitboxAction::ScrollProxyNodes
            | HitboxAction::ScrollTrafficRows
            | HitboxAction::ScrollHelp
            | HitboxAction::ScrollCommandOutput => {}
        }
    }

    pub(crate) fn handle_mouse_scroll(&mut self, action: Option<HitboxAction>, amount: i32) {
        match action {
            Some(HitboxAction::ScrollLogs) => {
                if amount > 0 {
                    self.scroll_logs_down(amount as usize);
                } else {
                    self.scroll_logs_up(amount.unsigned_abs() as usize);
                }
            }
            Some(HitboxAction::ScrollTrafficRows) | Some(HitboxAction::SelectTrafficRow(_)) => {
                if amount > 0 {
                    for _ in 0..amount {
                        self.select_down();
                    }
                } else {
                    for _ in 0..amount.unsigned_abs() {
                        self.select_up();
                    }
                }
            }
            Some(HitboxAction::ScrollTrafficChart) | Some(HitboxAction::SelectTrafficBucket(_)) => {
                self.pan_traffic_window(amount);
            }
            Some(HitboxAction::ScrollHelp) => {
                if amount > 0 {
                    self.ui_state.help.scroll =
                        self.ui_state.help.scroll.saturating_add(amount as usize);
                } else {
                    self.ui_state.help.scroll = self
                        .ui_state
                        .help
                        .scroll
                        .saturating_sub(amount.unsigned_abs() as usize);
                }
            }
            Some(HitboxAction::ScrollCommandOutput) => {
                if amount > 0 {
                    self.ui_state.command_output.scroll = self
                        .ui_state
                        .command_output
                        .scroll
                        .saturating_add(amount as usize);
                } else {
                    self.ui_state.command_output.scroll = self
                        .ui_state
                        .command_output
                        .scroll
                        .saturating_sub(amount.unsigned_abs() as usize);
                }
            }
            Some(HitboxAction::ScrollProxyNodes) | Some(HitboxAction::SelectProxyNode(_)) => {
                if amount > 0 {
                    for _ in 0..amount {
                        self.select_node_down();
                    }
                } else {
                    for _ in 0..amount.unsigned_abs() {
                        self.select_node_up();
                    }
                }
            }
            _ => {}
        }
    }
}
