use crate::ui::prelude::*;

impl App {
    pub(crate) fn t(&self, msg: Msg) -> &'static str {
        tr(self.ui_settings.language, msg)
    }

    pub(crate) fn action_label(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_label(self.ui_settings.language, spec.id, spec.label)
    }

    pub(crate) fn action_desc(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_desc(self.ui_settings.language, spec.id, spec.description)
    }

    pub(crate) fn action_button(&self, spec: &action_registry::ActionSpec) -> &'static str {
        tr_action_button(self.ui_settings.language, spec.id, spec.mouse)
    }

    pub(crate) fn settings_prompt_label(&self, kind: SettingsPromptKind) -> &'static str {
        action_registry::action_by_id(kind.action_id())
            .map(|spec| self.action_label(spec))
            .unwrap_or_else(|| kind.label())
    }

    pub(crate) fn settings_prompt_hint(&self, kind: SettingsPromptKind) -> &'static str {
        action_registry::action_by_id(kind.action_id())
            .map(|spec| self.action_desc(spec))
            .unwrap_or_else(|| kind.hint())
    }

    pub(crate) fn settings_prompt_field_label(
        &self,
        field: SettingsPromptFieldMsg,
    ) -> &'static str {
        tr_settings_prompt_field_label(self.ui_settings.language, field)
    }

    pub(crate) fn settings_prompt_field_hint(&self, field: SettingsPromptFieldMsg) -> &'static str {
        tr_settings_prompt_field_hint(self.ui_settings.language, field)
    }

    pub(crate) fn subscription_add_field_label(&self, field: SubscriptionAddField) -> &'static str {
        tr_subscription_field_label(self.ui_settings.language, field.msg())
    }

    pub(crate) fn subscription_add_field_hint(&self, field: SubscriptionAddField) -> &'static str {
        tr_subscription_field_hint(self.ui_settings.language, field.msg())
    }

    pub(crate) fn subscription_edit_field_label(
        &self,
        field: SubscriptionEditField,
    ) -> &'static str {
        tr_subscription_field_label(self.ui_settings.language, field.msg())
    }

    pub(crate) fn subscription_edit_field_hint(
        &self,
        field: SubscriptionEditField,
    ) -> &'static str {
        tr_subscription_field_hint(self.ui_settings.language, field.msg())
    }

    pub(crate) fn action_danger_label(&self, danger: ActionDanger) -> &'static str {
        match danger {
            ActionDanger::Safe => self.t(Msg::CommonSafe),
            ActionDanger::Confirm => self.t(Msg::RiskConfirm),
            ActionDanger::Sensitive => self.t(Msg::RiskSensitive),
            ActionDanger::Dangerous => self.t(Msg::RiskDangerous),
        }
    }
}
