use crate::ui::prelude::*;

impl App {
    pub fn begin_subscription_edit(&mut self, field: SubscriptionEditField) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        let Some(profile) = self.selected_subscription() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        let profile_id = profile.id;
        let value = field.initial_value(profile);
        self.subscription_add_form = None;
        self.subscription_edit_form = None;
        self.subscription_prompt = Some(SubscriptionPrompt {
            field,
            profile_id,
            value,
        });
        self.error_msg = None;
        self.status_msg = Some(format!(
            "editing subscription {}",
            self.subscription_edit_field_label(field)
        ));
    }

    pub fn begin_subscription_profile_edit(&mut self) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        let Some(profile) = self.selected_subscription() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        let profile_id = profile.id;
        let form = SubscriptionEditForm::from_profile(profile);
        self.subscription_prompt = None;
        self.subscription_add_form = None;
        self.subscription_edit_form = Some(form);
        self.error_msg = None;
        self.status_msg = Some(format!("editing subscription {}", profile_id));
    }

    pub fn begin_subscription_add(&mut self) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        self.subscription_prompt = None;
        self.subscription_edit_form = None;
        self.subscription_add_form = Some(SubscriptionAddForm::new());
        self.error_msg = None;
        self.status_msg = Some("adding subscription".into());
    }

    pub fn begin_subscription_import(&mut self) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        self.subscription_add_form = None;
        self.subscription_edit_form = None;
        self.subscription_prompt = Some(SubscriptionPrompt {
            field: SubscriptionEditField::ImportDirectory,
            profile_id: 0,
            value: String::new(),
        });
        self.error_msg = None;
        self.status_msg = Some("importing subscriptions".into());
    }

    pub fn cancel_subscription_prompt(&mut self) {
        if self.subscription_prompt.take().is_some()
            || self.subscription_add_form.take().is_some()
            || self.subscription_edit_form.take().is_some()
        {
            self.status_msg = Some("subscription edit cancelled".into());
        }
    }

    pub fn push_subscription_prompt_char(&mut self, c: char) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.push_char(c);
            return;
        }
        if let Some(form) = self.subscription_edit_form.as_mut() {
            form.push_char(c);
            return;
        }
        if let Some(prompt) = self.subscription_prompt.as_mut() {
            prompt.value.push(c);
        }
    }

    pub fn pop_subscription_prompt_char(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.pop_char();
            return;
        }
        if let Some(form) = self.subscription_edit_form.as_mut() {
            form.pop_char();
            return;
        }
        if let Some(prompt) = self.subscription_prompt.as_mut() {
            prompt.value.pop();
        }
    }

    pub fn subscription_input_active(&self) -> bool {
        self.subscription_prompt.is_some()
            || self.subscription_add_form.is_some()
            || self.subscription_edit_form.is_some()
    }

    pub fn next_subscription_form_field(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.next_field();
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.next_field();
        }
    }

    pub fn prev_subscription_form_field(&mut self) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.prev_field();
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.prev_field();
        }
    }

    pub fn select_subscription_add_field(&mut self, idx: usize) {
        if let Some(form) = self.subscription_add_form.as_mut() {
            form.set_active(idx);
        } else if let Some(form) = self.subscription_edit_form.as_mut() {
            form.set_active(idx);
        }
    }

    pub fn submit_subscription_prompt(&mut self) {
        if let Some(form) = self.subscription_add_form.take() {
            let args = match form.args() {
                Ok(args) => args,
                Err(e) => {
                    self.error_msg = Some(e);
                    self.subscription_add_form = Some(form);
                    return;
                }
            };
            self.run_subscription_args("Add subscription".into(), "new subscription".into(), args);
            return;
        }
        if let Some(form) = self.subscription_edit_form.take() {
            let profile_id = form.profile_id;
            let commands = match form.commands() {
                Ok(commands) => commands,
                Err(e) => {
                    self.error_msg = Some(e);
                    self.subscription_edit_form = Some(form);
                    return;
                }
            };
            if commands.is_empty() {
                self.status_msg = Some(format!("subscription {} unchanged", profile_id));
                return;
            }
            self.run_subscription_command_sequence(
                "Edit subscription".into(),
                format!("subscription {}", profile_id),
                commands,
            );
            return;
        }

        let Some(prompt) = self.subscription_prompt.take() else {
            return;
        };
        let value = prompt.value.trim().to_string();
        if value.is_empty() {
            self.error_msg = Some(format!(
                "{} cannot be empty",
                self.subscription_edit_field_label(prompt.field)
            ));
            self.subscription_prompt = Some(prompt);
            return;
        }

        let args = prompt.field.args(prompt.profile_id, value);
        let field_label = self.subscription_edit_field_label(prompt.field).to_string();
        let profile_id = prompt.profile_id;
        let target = prompt.field.status_target(profile_id);
        self.run_subscription_args(field_label, target, args);
    }

    pub(crate) fn run_subscription_args(
        &mut self,
        field_label: String,
        target: String,
        args: Vec<String>,
    ) {
        self.run_subscription_command_sequence(field_label, target, vec![args]);
    }

    pub(crate) fn run_subscription_command_sequence(
        &mut self,
        field_label: String,
        target: String,
        commands: Vec<Vec<String>>,
    ) {
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("running {} for {}", field_label, target));
        self.rt.spawn(async move {
            let mut outputs = Vec::new();
            let mut failed = None;
            for args in commands {
                match crate::api::run_clashctl(&args).await {
                    Ok(stdout) => {
                        if !stdout.is_empty() {
                            outputs.push(stdout);
                        }
                    }
                    Err(e) => {
                        failed = Some(format!("clashctl {}: {}", args.join(" "), e));
                        break;
                    }
                }
            }
            let result = if let Some(e) = failed {
                Err(e)
            } else if outputs.is_empty() {
                Ok(format!("{} completed for {}", field_label, target))
            } else {
                Ok(outputs.join("\n"))
            };
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn refresh_subscriptions(&mut self) {
        let meta = crate::api::read_profiles();
        self.profiles = meta.profiles;
        self.active_profile_id = meta.active_id;
        self.clamp_subscription_selection();
    }

    pub fn use_selected_subscription(&mut self) {
        let Some(id) = self.selected_subscription_id() else {
            return;
        };
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl_sub("use", id).await;
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn update_selected_subscription(&mut self) {
        let Some(id) = self.selected_subscription_id() else {
            return;
        };
        let tx = self.data_tx.clone();
        self.rt.spawn(async move {
            let result = crate::api::run_clashctl_sub("update", id).await;
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn remove_selected_subscription(&mut self) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        let Some(id) = self.selected_subscription_id() else {
            self.status_msg = Some("no subscription selected".into());
            return;
        };
        if self.should_confirm() {
            self.pending_confirmation = Some(PendingConfirmation {
                title: format!("Confirm remove subscription {}", id),
                message: format!("Run `clashctl sub remove {}`?", id),
                action: PendingAction::SubscriptionRemove(id),
            });
            self.status_msg = Some("confirm action with Enter/y, cancel with Esc/n".into());
            return;
        }
        self.execute_subscription_remove(id);
    }

    pub(crate) fn execute_subscription_remove(&mut self, id: i32) {
        let tx = self.data_tx.clone();
        self.status_msg = Some(format!("removing subscription {}", id));
        self.rt.spawn(async move {
            let args = vec!["sub".into(), "remove".into(), id.to_string()];
            let result = crate::api::run_clashctl(&args).await.map(|stdout| {
                if stdout.is_empty() {
                    format!("subscription removed: [{}]", id)
                } else {
                    stdout.lines().last().unwrap_or("").to_string()
                }
            });
            let _ = tx.send(DataEvent::SubscriptionResult(result));
        });
    }

    pub fn show_subscription_log(&mut self) {
        if self.ui_state.active_page != Tab::Subscriptions {
            return;
        }
        let tx = self.data_tx.clone();
        self.status_msg = Some("loading subscription log".into());
        self.rt.spawn(async move {
            let args = vec!["sub".into(), "log".into()];
            let result = crate::api::run_clashctl(&args).await;
            let _ = tx.send(DataEvent::SubscriptionOutputResult("log".into(), result));
        });
    }
}
