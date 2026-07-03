use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::action_registry::{self, ActionDanger, ApiAction, InternalAction, PromptAction};
use crate::mouse::HitboxAction;

pub(crate) struct ActionButtonItem {
    pub(crate) label: String,
    pub(crate) action: HitboxAction,
    pub(crate) danger: ActionDanger,
}

pub(crate) fn action_button_item(
    label: impl Into<String>,
    action: HitboxAction,
    danger: ActionDanger,
) -> ActionButtonItem {
    ActionButtonItem {
        label: label.into(),
        action,
        danger,
    }
}

pub(crate) fn action_buttons_from_specs<'a, I>(app: &App, specs: I) -> Vec<ActionButtonItem>
where
    I: IntoIterator<Item = &'a action_registry::ActionSpec>,
{
    specs
        .into_iter()
        .filter_map(|spec| {
            hitbox_for_action_spec(spec)
                .map(|action| action_button_item(app.action_button(spec), action, spec.danger))
        })
        .collect()
}

pub(crate) fn button_style(danger: ActionDanger) -> Style {
    let bg = match danger {
        ActionDanger::Dangerous => CLASH_THEME.danger,
        ActionDanger::Confirm | ActionDanger::Sensitive => CLASH_THEME.warning,
        ActionDanger::Safe => CLASH_THEME.primary,
    };
    Style::default().fg(CLASH_THEME.bg).bg(bg).bold()
}

pub(crate) fn render_action_buttons(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    buttons: &[ActionButtonItem],
) {
    render_action_buttons_on(frame, app, area, buttons, CLASH_THEME.surface);
}

pub(crate) fn render_action_buttons_on(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    buttons: &[ActionButtonItem],
    bg: ratatui::style::Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    fill_area(frame, area, bg);
    for (rect, idx) in action_button_rects(area, buttons) {
        let button = &buttons[idx];
        app.ui_state.hitboxes.register(rect, button.action.clone());
        frame.render_widget(
            Paragraph::new(format!(" {} ", button.label)).style(button_style(button.danger)),
            rect,
        );
    }
}

pub(crate) fn action_button_rects(area: Rect, buttons: &[ActionButtonItem]) -> Vec<(Rect, usize)> {
    let mut rects = Vec::new();
    let mut x = area.x;
    let mut y = area.y;
    let max_x = area.x.saturating_add(area.width);
    let max_y = area.y.saturating_add(area.height);

    for (idx, button) in buttons.iter().enumerate() {
        let width = action_button_width(&button.label);
        if x > area.x && x.saturating_add(width) > max_x {
            x = area.x;
            y = y.saturating_add(1);
        }
        if y >= max_y {
            break;
        }
        let available = max_x.saturating_sub(x);
        let rect = Rect::new(x, y, width.min(available), 1);
        rects.push((rect, idx));
        x = x.saturating_add(width + 1);
    }
    rects
}

pub(crate) fn action_button_width(label: &str) -> u16 {
    display_width(label).saturating_add(2)
}

pub(crate) fn display_width(value: &str) -> u16 {
    UnicodeWidthStr::width(value).min(u16::MAX as usize) as u16
}

pub(crate) fn hitbox_for_action_spec(spec: &action_registry::ActionSpec) -> Option<HitboxAction> {
    match spec.executor {
        ActionExecutor::Network(action) => Some(HitboxAction::RunNetwork(action)),
        ActionExecutor::Settings(action) => Some(HitboxAction::RunSettings(action)),
        ActionExecutor::Traffic(action) => Some(HitboxAction::RunTraffic(action)),
        ActionExecutor::Api(ApiAction::CycleProxyMode) => Some(HitboxAction::CycleProxyMode),
        ActionExecutor::Api(ApiAction::SwitchProxyNode) => {
            Some(HitboxAction::SwitchSelectedProxyNode)
        }
        ActionExecutor::Api(ApiAction::TestProxyDelay) => {
            Some(HitboxAction::TestSelectedProxyDelay)
        }
        ActionExecutor::Api(ApiAction::CloseSelectedConnection) => {
            Some(HitboxAction::CloseSelectedConnection)
        }
        ActionExecutor::Api(ApiAction::CloseAllConnections) => {
            Some(HitboxAction::CloseAllConnections)
        }
        ActionExecutor::Prompt(PromptAction::ConfigSetPort) => {
            Some(HitboxAction::BeginConfigSetPorts)
        }
        ActionExecutor::Prompt(PromptAction::ConfigSetApi) => Some(HitboxAction::BeginConfigSetApi),
        ActionExecutor::Prompt(PromptAction::ConfigSetDnsMode) => {
            Some(HitboxAction::BeginConfigSetDns)
        }
        ActionExecutor::Prompt(PromptAction::ConfigSetLan) => Some(HitboxAction::BeginConfigSetLan),
        ActionExecutor::Prompt(PromptAction::TrafficPruneRetention) => {
            Some(HitboxAction::BeginTrafficPruneRetention)
        }
        ActionExecutor::Prompt(PromptAction::GeodataUpdateVersion) => {
            Some(HitboxAction::BeginGeodataUpdateVersion)
        }
        ActionExecutor::Prompt(PromptAction::SecretSet) => Some(HitboxAction::BeginSecretSet),
        ActionExecutor::Prompt(PromptAction::SubAdd) => Some(HitboxAction::BeginSubscriptionAdd),
        ActionExecutor::Prompt(PromptAction::SubImport) => {
            Some(HitboxAction::BeginSubscriptionImport)
        }
        ActionExecutor::Internal(InternalAction::ToggleProxySort) => {
            Some(HitboxAction::ToggleProxySort)
        }
        ActionExecutor::Internal(InternalAction::ToggleNodePicker) => {
            Some(HitboxAction::ToggleNodePicker)
        }
        ActionExecutor::Internal(InternalAction::CloseNodePicker) => {
            Some(HitboxAction::CloseNodePicker)
        }
        ActionExecutor::Internal(InternalAction::TestAllProxyDelays) => {
            Some(HitboxAction::TestAllProxyDelays)
        }
        ActionExecutor::Internal(InternalAction::CycleUiLanguage) => {
            Some(HitboxAction::CycleUiLanguage)
        }
        ActionExecutor::Internal(InternalAction::CycleThemePreference) => {
            Some(HitboxAction::CycleThemePreference)
        }
        ActionExecutor::Internal(InternalAction::CycleDefaultPage) => {
            Some(HitboxAction::CycleDefaultPage)
        }
        ActionExecutor::Internal(InternalAction::CycleRefreshInterval) => {
            Some(HitboxAction::CycleRefreshInterval)
        }
        ActionExecutor::Internal(InternalAction::ToggleMousePreference) => {
            Some(HitboxAction::ToggleMousePreference)
        }
        ActionExecutor::Internal(InternalAction::ToggleDangerousConfirmations) => {
            Some(HitboxAction::ToggleDangerousConfirmations)
        }
        ActionExecutor::Internal(InternalAction::CycleDefaultTrafficRange) => {
            Some(HitboxAction::CycleDefaultTrafficRange)
        }
        ActionExecutor::Internal(InternalAction::CycleDefaultTrafficChart) => {
            Some(HitboxAction::CycleDefaultTrafficChart)
        }
        ActionExecutor::Internal(InternalAction::CycleDefaultTrafficDimension) => {
            Some(HitboxAction::CycleDefaultTrafficDimension)
        }
        ActionExecutor::Internal(InternalAction::ToggleLogPause) => {
            Some(HitboxAction::ToggleLogPause)
        }
        ActionExecutor::Internal(InternalAction::CycleLogLevel) => {
            Some(HitboxAction::CycleLogFilter)
        }
        ActionExecutor::Internal(InternalAction::ClearLogs) => Some(HitboxAction::ClearLogs),
        ActionExecutor::Internal(InternalAction::TrafficExportCsv) => {
            Some(HitboxAction::ExportTraffic)
        }
        ActionExecutor::Internal(InternalAction::SubLog) => Some(HitboxAction::ShowSubscriptionLog),
        _ => None,
    }
}

pub(crate) fn action_buttons_from_ids(app: &App, action_ids: &[&str]) -> Vec<ActionButtonItem> {
    action_buttons_from_specs(
        app,
        action_ids
            .iter()
            .filter_map(|id| action_registry::action_by_id(id)),
    )
}

pub(crate) fn action_buttons_needed_height(width: u16, buttons: &[ActionButtonItem]) -> u16 {
    if width == 0 || buttons.is_empty() {
        return 0;
    }
    let mut rows = 1u16;
    let mut x = 0u16;
    for button in buttons {
        let button_width = action_button_width(&button.label).min(width);
        if x > 0 && x.saturating_add(button_width) > width {
            rows = rows.saturating_add(1);
            x = 0;
        }
        x = x.saturating_add(button_width.saturating_add(1));
    }
    rows
}
