use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::components::panel::Panel;

pub(crate) const SETTINGS_GENERAL_ACTION_IDS: &[&str] = &[
    "settings.ui.language",
    "settings.ui.theme",
    "settings.ui.default_page",
    "settings.ui.refresh_interval",
    "settings.ui.mouse",
    "settings.ui.confirm",
];

pub(crate) const SETTINGS_TRAFFIC_ACTION_IDS: &[&str] = &[
    "settings.traffic.default_range",
    "settings.traffic.default_chart",
    "settings.traffic.default_dimension",
    "settings.traffic.prune_retention",
    "traffic.reset",
];

pub(crate) const SETTINGS_DIAGNOSTIC_ACTION_IDS: &[&str] = &[
    "settings.doctor",
    "settings.config_doctor",
    "settings.proxy.test",
    "settings.version",
];
pub(crate) const SETTINGS_CONFIG_ACTION_IDS: &[&str] = &[
    "settings.config.view",
    "settings.config.raw",
    "settings.config.merge",
    "settings.config.autofix",
];
pub(crate) const SETTINGS_FORM_ACTION_IDS: &[&str] = &[
    "settings.config.set_ports",
    "settings.config.set_api",
    "settings.config.set_dns",
    "settings.config.set_lan",
];
pub(crate) const SETTINGS_SECURITY_ACTION_IDS: &[&str] = &[
    "settings.secret.status",
    "settings.secret.reveal",
    "settings.secret.set",
];
pub(crate) const SETTINGS_UPDATE_ACTION_IDS: &[&str] = &[
    "settings.geodata.update",
    "settings.geodata.version",
    "settings.api.upgrade",
    "settings.kernel.upgrade",
];

pub(crate) fn render_settings(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).split(area);
    render_settings_sections(frame, rows[0], app);
    match app.ui_state.settings.section {
        SettingsSection::General => render_settings_general(frame, rows[1], app),
        SettingsSection::Core => render_settings_core(frame, rows[1], app),
        SettingsSection::Traffic => render_settings_traffic(frame, rows[1], app),
        SettingsSection::Security => render_settings_action_panel(
            frame,
            rows[1],
            app,
            app.t(Msg::SettingsSecurity),
            &[(app.t(Msg::SettingsSecurity), SETTINGS_SECURITY_ACTION_IDS)],
        ),
        SettingsSection::Diagnostics => render_settings_action_panel(
            frame,
            rows[1],
            app,
            app.t(Msg::SettingsDiagnostics),
            &[
                (
                    app.t(Msg::SettingsDiagnostics),
                    SETTINGS_DIAGNOSTIC_ACTION_IDS,
                ),
                (app.t(Msg::SettingsConfig), SETTINGS_CONFIG_ACTION_IDS),
                (app.t(Msg::SettingsForms), SETTINGS_FORM_ACTION_IDS),
            ],
        ),
        SettingsSection::Updates => render_settings_action_panel(
            frame,
            rows[1],
            app,
            app.t(Msg::SettingsUpdates),
            &[(app.t(Msg::SettingsUpdates), SETTINGS_UPDATE_ACTION_IDS)],
        ),
    }
}

pub(crate) fn render_settings_sections(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner = Panel::new(app.t(Msg::PageSettings)).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut x = inner.x;
    let mut y = inner.y;
    let max_x = inner.x.saturating_add(inner.width);
    let max_y = inner.y.saturating_add(inner.height);
    for (idx, section) in SettingsSection::all().iter().copied().enumerate() {
        let label = app.settings_section_label(section);
        let width = display_width(label).saturating_add(4);
        if x > inner.x && x.saturating_add(width) > max_x {
            x = inner.x;
            y = y.saturating_add(1);
        }
        if y >= max_y {
            break;
        }
        let rect = Rect::new(x, y, width.min(max_x.saturating_sub(x)), 1);
        let style = if app.ui_state.settings.section == section {
            Style::default()
                .fg(CLASH_THEME.bg)
                .bg(CLASH_THEME.primary)
                .bold()
        } else {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.surface)
        };
        frame.render_widget(Paragraph::new(format!(" {} ", label)).style(style), rect);
        app.ui_state
            .hitboxes
            .register(rect, HitboxAction::SelectSettingsSection(idx));
        x = x.saturating_add(width + 1);
    }
}

pub(crate) fn render_settings_general(frame: &mut Frame, area: Rect, app: &mut App) {
    let (summary_area, action_area) = settings_summary_action_areas(area);
    let mouse = if app.ui_settings.mouse_enabled {
        "on"
    } else {
        "off"
    };
    let confirm = if app.ui_settings.confirm_dangerous_actions {
        "on"
    } else {
        "off"
    };
    let lines = vec![
        setting_value_line(
            app.t(Msg::SettingsLanguage),
            app.ui_settings.language.label(),
        ),
        setting_value_line(
            app.t(Msg::SettingsTheme),
            theme_label(&app.ui_settings.theme),
        ),
        setting_value_line(app.t(Msg::SettingsDefaultPage), app.default_page_label()),
        setting_value_line(
            app.t(Msg::SettingsRefreshInterval),
            app.ui_settings.refresh_interval_label(),
        ),
        setting_value_line(app.t(Msg::SettingsMouse), mouse),
        setting_value_line(app.t(Msg::SettingsConfirmDanger), confirm),
    ];
    render_settings_summary(frame, summary_area, app.t(Msg::SettingsGeneral), lines);
    render_settings_action_panel(
        frame,
        action_area,
        app,
        app.t(Msg::SettingsGeneral),
        &[(app.t(Msg::SettingsGeneral), SETTINGS_GENERAL_ACTION_IDS)],
    );
}

pub(crate) fn render_settings_core(frame: &mut Frame, area: Rect, app: &mut App) {
    let (summary_area, action_area) = settings_summary_action_areas(area);
    let api = if app.config.api_url.is_empty() {
        app.t(Msg::SettingsNotConnected).to_string()
    } else {
        app.config.api_url.clone()
    };
    let kernel = if app.version.is_empty() {
        app.t(Msg::SettingsNotConnected).to_string()
    } else {
        app.version.clone()
    };
    let secret = if app.config.api_key.is_empty() {
        "empty"
    } else {
        "configured"
    };
    let lines = vec![
        setting_value_line(app.t(Msg::SettingsApi), api),
        setting_value_line(app.t(Msg::SettingsKernel), kernel),
        setting_value_line("Mode", app.mode.clone()),
        setting_value_line("API secret", secret),
    ];
    render_settings_summary(frame, summary_area, app.t(Msg::SettingsCoreApi), lines);
    render_settings_action_panel(
        frame,
        action_area,
        app,
        app.t(Msg::SettingsCoreApi),
        &[
            (app.t(Msg::SettingsConfig), SETTINGS_CONFIG_ACTION_IDS),
            (app.t(Msg::SettingsForms), SETTINGS_FORM_ACTION_IDS),
        ],
    );
}

pub(crate) fn render_settings_traffic(frame: &mut Frame, area: Rect, app: &mut App) {
    let (summary_area, action_area) = settings_summary_action_areas(area);
    let lines = vec![
        setting_value_line("Default range", app.ui_state.traffic.range.label()),
        setting_value_line(
            app.t(Msg::SettingsDefaultChart),
            app.ui_state.traffic.chart.label(),
        ),
        setting_value_line("Default by", app.ui_state.traffic.dimension.label()),
        setting_value_line("Store", app.traffic_status.store_dir.clone()),
        setting_value_line(
            "Collector",
            if app.traffic_status.collector_running {
                "running"
            } else if app.traffic_status.collector_stale {
                "stale"
            } else {
                "stopped"
            },
        ),
    ];
    render_settings_summary(frame, summary_area, app.t(Msg::SettingsTraffic), lines);
    render_settings_action_panel(
        frame,
        action_area,
        app,
        app.t(Msg::SettingsTraffic),
        &[(app.t(Msg::SettingsTraffic), SETTINGS_TRAFFIC_ACTION_IDS)],
    );
}

pub(crate) fn settings_summary_action_areas(area: Rect) -> (Rect, Rect) {
    if area.width >= 100 && area.height >= 8 {
        let chunks =
            Layout::horizontal([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)]).split(area);
        (chunks[0], chunks[1])
    } else if area.height >= 14 {
        let chunks = Layout::vertical([Constraint::Length(7), Constraint::Min(5)]).split(area);
        (chunks[0], chunks[1])
    } else {
        let chunks =
            Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
        (chunks[0], chunks[1])
    }
}

pub(crate) fn render_settings_summary(
    frame: &mut Frame,
    area: Rect,
    title: &'static str,
    lines: Vec<Line<'static>>,
) {
    let inner = Panel::new(title).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(CLASH_THEME.surface)),
        inner,
    );
}

pub(crate) fn setting_value_line(
    label: impl Into<String>,
    value: impl Into<String>,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<18}", trunc_str(&label.into(), 18)),
            CLASH_THEME.muted,
        ),
        Span::styled(trunc_str(&value.into(), 86), CLASH_THEME.text),
    ])
}

pub(crate) fn render_settings_action_panel(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    title: &'static str,
    groups: &[(&'static str, &[&str])],
) {
    let inner = Panel::new(title).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut y = inner.y;
    let max_y = inner.y.saturating_add(inner.height);
    for (group, action_ids) in groups {
        if y >= max_y {
            return;
        }
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", group),
                CLASH_THEME.primary,
            )))
            .style(Style::default().bg(CLASH_THEME.surface)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y = y.saturating_add(1);

        let buttons = action_buttons_from_ids(app, action_ids);
        let button_height = action_buttons_needed_height(inner.width, &buttons)
            .max(1)
            .min(max_y.saturating_sub(y));
        if button_height > 0 {
            let button_area = Rect::new(inner.x, y, inner.width, button_height);
            render_action_buttons(frame, app, button_area, &buttons);
            y = y.saturating_add(button_height);
        }
        if y < max_y {
            y = y.saturating_add(1);
        }
    }
}
