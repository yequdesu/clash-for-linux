use crate::ui::prelude::*;

impl App {
    pub fn run_network_action(&mut self, action: NetworkAction) {
        if self.ui_state.active_page != Tab::Network {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.ui_state.modals.pending_confirmation = Some(PendingConfirmation {
                title: self.confirm_title(action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Network(action),
            });
            self.status_msg = Some(self.confirm_hint());
            return;
        }
        self.execute_network_action(action);
    }

    pub(crate) fn execute_network_action(&mut self, action: NetworkAction) {
        let label = action.label().to_string();
        let args = action.args();
        self.prepare_sudo_candidate(label.clone(), args.clone(), SudoTarget::Network);
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running network action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::NetworkResult(label, result));
        });
    }

    pub fn run_settings_action(&mut self, action: SettingsAction) {
        if self.ui_state.active_page != Tab::Settings {
            return;
        }
        if self.should_confirm() && action.requires_confirmation() {
            self.ui_state.modals.pending_confirmation = Some(PendingConfirmation {
                title: self.confirm_title(action.label()),
                message: format!("Run `clashctl {}`?", action.args().join(" ")),
                action: PendingAction::Settings(action),
            });
            self.status_msg = Some(self.confirm_hint());
            return;
        }
        self.execute_settings_action(action);
    }

    pub fn begin_settings_prompt(&mut self, kind: SettingsPromptKind) {
        if self.ui_state.active_page != Tab::Settings {
            return;
        }
        self.ui_state.settings.prompt = Some(SettingsPrompt::new(kind));
        self.error_msg = None;
        self.status_msg = Some(format!("editing {}", kind.label()));
    }

    pub fn begin_secret_set(&mut self) {
        self.begin_settings_prompt(SettingsPromptKind::SecretSet);
    }

    pub(crate) fn settings_section_label(&self, section: SettingsSection) -> &'static str {
        match section {
            SettingsSection::General => self.t(Msg::SettingsGeneral),
            SettingsSection::Core => self.t(Msg::SettingsCoreApi),
            SettingsSection::Traffic => self.t(Msg::SettingsTraffic),
            SettingsSection::Security => self.t(Msg::SettingsSecurity),
            SettingsSection::Diagnostics => self.t(Msg::SettingsDiagnostics),
            SettingsSection::Updates => self.t(Msg::SettingsUpdates),
        }
    }

    pub fn next_settings_section(&mut self) {
        if self.ui_state.active_page == Tab::Settings {
            self.ui_state.settings.section = self.ui_state.settings.section.next();
        }
    }

    pub fn prev_settings_section(&mut self) {
        if self.ui_state.active_page == Tab::Settings {
            self.ui_state.settings.section = self.ui_state.settings.section.prev();
        }
    }

    pub fn select_settings_section(&mut self, idx: usize) {
        if let Some(section) = SettingsSection::all().get(idx).copied() {
            self.ui_state.settings.section = section;
        }
    }

    pub fn cancel_settings_prompt(&mut self) {
        if self.ui_state.settings.prompt.take().is_some() {
            self.status_msg = Some("settings edit cancelled".into());
        }
    }

    pub fn focus_settings_prompt_value(&mut self) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_ref() {
            self.error_msg = None;
            let field = prompt
                .active_field()
                .map(|field| self.settings_prompt_field_label(field.field))
                .unwrap_or("value");
            self.status_msg = Some(format!(
                "editing {} · {}",
                self.settings_prompt_label(prompt.kind),
                field
            ));
        }
    }

    pub fn next_settings_prompt_field(&mut self) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_mut() {
            prompt.next_field();
            self.focus_settings_prompt_value();
        }
    }

    pub fn prev_settings_prompt_field(&mut self) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_mut() {
            prompt.prev_field();
            self.focus_settings_prompt_value();
        }
    }

    pub fn select_settings_prompt_field(&mut self, idx: usize) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_mut() {
            prompt.select_field(idx);
            self.focus_settings_prompt_value();
        }
    }

    pub fn push_settings_prompt_char(&mut self, c: char) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_mut() {
            prompt.push_char(c);
        }
    }

    pub fn pop_settings_prompt_char(&mut self) {
        if let Some(prompt) = self.ui_state.settings.prompt.as_mut() {
            prompt.pop_char();
        }
    }

    pub fn submit_settings_prompt(&mut self) {
        let Some(prompt) = self.ui_state.settings.prompt.take() else {
            return;
        };
        let value = prompt.value().trim().to_string();
        if value.is_empty() {
            self.error_msg = Some(format!(
                "{} cannot be empty",
                self.settings_prompt_label(prompt.kind)
            ));
            self.ui_state.settings.prompt = Some(prompt);
            return;
        }

        let label = prompt.kind.label().to_string();
        let args = match prompt.kind.args(value) {
            Ok(args) => args,
            Err(e) => {
                self.error_msg = Some(e);
                self.ui_state.settings.prompt = Some(prompt);
                return;
            }
        };
        if self.should_confirm() {
            let preview = prompt.kind.command_preview(&args);
            self.ui_state.modals.pending_confirmation = Some(PendingConfirmation {
                title: self.confirm_title(self.settings_prompt_label(prompt.kind)),
                message: format!("Run `{}`?", preview),
                action: PendingAction::SettingsCommand {
                    label,
                    args,
                    redact_output: true,
                },
            });
            self.status_msg = Some(self.confirm_hint());
            return;
        }
        self.execute_settings_command(label, args, true);
    }

    pub(crate) fn execute_settings_action(&mut self, action: SettingsAction) {
        let args = action.args();
        self.prepare_sudo_candidate(
            action.label().to_string(),
            args.clone(),
            SudoTarget::Settings(action),
        );
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running settings action: {}", action.label()));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SettingsResult(action, result));
        });
    }

    pub(crate) fn execute_settings_command(
        &mut self,
        label: String,
        args: Vec<String>,
        redact_output: bool,
    ) {
        self.prepare_sudo_candidate(
            label.clone(),
            args.clone(),
            SudoTarget::SettingsCommand { redact_output },
        );
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running settings action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SettingsCommandResult(
                label,
                redact_output,
                result,
            ));
        });
    }

    pub(crate) fn should_confirm(&self) -> bool {
        self.ui_settings.confirm_dangerous_actions
    }

    pub(crate) fn prepare_sudo_candidate(
        &mut self,
        label: String,
        args: Vec<String>,
        target: SudoTarget,
    ) {
        self.ui_state.modals.sudo_candidate = Some(SudoPrompt {
            label,
            args,
            target,
            password: String::new(),
        });
    }

    pub(crate) fn clear_sudo_candidate(&mut self, label: &str) {
        if self
            .ui_state
            .modals
            .sudo_candidate
            .as_ref()
            .is_some_and(|candidate| candidate.label == label)
        {
            self.ui_state.modals.sudo_candidate = None;
        }
    }

    pub(crate) fn maybe_open_sudo_prompt(&mut self, label: &str, error: &str) -> bool {
        if !sudo_required_error(error) {
            return false;
        }
        let Some(mut prompt) = self
            .ui_state
            .modals
            .sudo_candidate
            .take()
            .filter(|candidate| candidate.label == label)
        else {
            return false;
        };
        prompt.password.clear();
        self.error_msg = None;
        self.status_msg = Some(self.sudo_required_status(label));
        self.ui_state.modals.sudo_prompt = Some(prompt);
        true
    }

    pub(crate) fn maybe_open_pending_sudo_prompt(&mut self, error: &str) -> bool {
        if !sudo_required_error(error) {
            return false;
        }
        let Some(mut prompt) = self.ui_state.modals.sudo_candidate.take() else {
            return false;
        };
        prompt.password.clear();
        let label = prompt.label.clone();
        self.error_msg = None;
        self.status_msg = Some(self.sudo_required_status(&label));
        self.ui_state.modals.sudo_prompt = Some(prompt);
        true
    }

    pub fn sudo_prompt_active(&self) -> bool {
        self.ui_state.modals.sudo_prompt.is_some()
    }

    pub fn push_sudo_prompt_char(&mut self, c: char) {
        if let Some(prompt) = self.ui_state.modals.sudo_prompt.as_mut() {
            prompt.password.push(c);
        }
    }

    pub fn pop_sudo_prompt_char(&mut self) {
        if let Some(prompt) = self.ui_state.modals.sudo_prompt.as_mut() {
            prompt.password.pop();
        }
    }

    pub fn focus_sudo_prompt_value(&mut self) {
        if let Some(prompt) = self.ui_state.modals.sudo_prompt.as_ref() {
            self.status_msg = Some(self.sudo_required_status(&prompt.label));
            self.error_msg = None;
        }
    }

    pub fn cancel_sudo_prompt(&mut self) {
        if self.ui_state.modals.sudo_prompt.take().is_some() {
            self.status_msg = Some(self.t(Msg::ActionCancelled).into());
        }
    }

    pub fn submit_sudo_prompt(&mut self) {
        let Some(prompt) = self.ui_state.modals.sudo_prompt.take() else {
            return;
        };
        if prompt.password.is_empty() {
            self.error_msg = Some(self.t(Msg::SudoPasswordEmpty).into());
            self.ui_state.modals.sudo_prompt = Some(prompt);
            return;
        }
        let retry_candidate = SudoPrompt {
            label: prompt.label.clone(),
            args: prompt.args.clone(),
            target: prompt.target.clone(),
            password: String::new(),
        };
        self.ui_state.modals.sudo_candidate = Some(retry_candidate);
        let label = prompt.label.clone();
        let args = prompt.args.clone();
        let target = prompt.target.clone();
        let password = prompt.password;
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running sudo action: {}", label));
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl_sudo(&args, &password).await;
            match target {
                SudoTarget::Network => {
                    let _ = tx.send(DataEvent::NetworkResult(label, result));
                }
                SudoTarget::Settings(action) => {
                    let _ = tx.send(DataEvent::SettingsResult(action, result));
                }
                SudoTarget::SettingsCommand { redact_output } => {
                    let _ = tx.send(DataEvent::SettingsCommandResult(
                        label,
                        redact_output,
                        result,
                    ));
                }
                SudoTarget::Subscription => {
                    let _ = tx.send(DataEvent::SubscriptionResult(result));
                }
                SudoTarget::SubscriptionOutput { output_label } => {
                    let _ = tx.send(DataEvent::SubscriptionOutputResult(output_label, result));
                }
                SudoTarget::Traffic => {
                    let _ = tx.send(DataEvent::TrafficActionResult(label, result));
                }
            }
        });
    }

    pub fn confirm_pending_action(&mut self) {
        let Some(pending) = self.ui_state.modals.pending_confirmation.take() else {
            return;
        };
        match pending.action {
            PendingAction::Network(action) => self.execute_network_action(action),
            PendingAction::Settings(action) => self.execute_settings_action(action),
            PendingAction::SettingsCommand {
                label,
                args,
                redact_output,
            } => self.execute_settings_command(label, args, redact_output),
            PendingAction::ToggleDangerousConfirmations => {
                self.apply_dangerous_confirmations_toggle();
            }
            PendingAction::SubscriptionRemove(id) => self.execute_subscription_remove(id),
            PendingAction::Traffic(action) => self.execute_traffic_action(action),
        }
    }

    pub fn cancel_pending_action(&mut self) {
        if self.ui_state.modals.pending_confirmation.take().is_some() {
            self.status_msg = Some(self.t(Msg::ActionCancelled).into());
        }
    }
}
