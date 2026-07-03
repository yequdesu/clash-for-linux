#![allow(unused_imports)]

pub(crate) use crate::action_registry::{self, ActionDanger, ActionExecutor};
pub(crate) use crate::api::{Connection, ProfileEntry, ProxyInfo, TrafficPoint, TrafficStatus};
pub(crate) use crate::app::App;
pub(crate) use crate::event::DataEvent;
pub(crate) use crate::i18n::{
    tr, tr_action_button, tr_action_desc, tr_action_label, tr_settings_prompt_field_hint,
    tr_settings_prompt_field_label, tr_subscription_field_hint, tr_subscription_field_label, Msg,
    SettingsPromptFieldMsg, SubscriptionFieldMsg,
};
pub(crate) use crate::mouse::{HitboxAction, NetworkAction, SettingsAction, TrafficAction};
pub(crate) use crate::settings::next_refresh_interval_secs;
pub(crate) use crate::theme::{apply_theme_key, next_theme_key, theme_label, CLASH_THEME};
pub(crate) use crate::ui::components::nav::Tab;
pub(crate) use crate::ui::model::*;
pub(crate) use crate::ui::utils::*;
pub(crate) use ratatui::layout::Margin;
