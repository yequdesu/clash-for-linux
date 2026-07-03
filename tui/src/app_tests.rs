use super::{
    command_output_lines, initial_tab_for_profiles, log_line_rank, mode_display, next_default_page,
    next_mode, parse_config_set_api_args, parse_config_set_ports_args,
    parse_geodata_update_version_args, parse_traffic_prune_retention_args, profile_interval_label,
    redact_sensitive_output, sanitize_terminal_line, scaled_bar, traffic_summary, App,
    LogLevelFilter, SettingsPromptKind, SubscriptionAddForm, SubscriptionEditField,
    SubscriptionEditForm, SubscriptionPrompt, SudoTarget, TrafficChartKind, TrafficDimension,
    TrafficRange,
};
use crate::action_registry::{self, ActionDanger};
use crate::api::{LogEntry, ProfileEntry, ProxyInfo, TrafficPoint, TrafficStatus};
use crate::config::Config;
use crate::event::DataEvent;
use crate::i18n::Msg;
use crate::mouse::{HitboxAction, NetworkAction, SettingsAction, TrafficAction};
use crate::settings::LanguageSetting;
use crate::ui::app_shell::register_tab_hitboxes;
use crate::ui::components::action_bar::{
    action_button_item, action_button_rects, action_button_width, display_width,
    hitbox_for_action_spec,
};
use crate::ui::components::nav::Tab;
use crate::ui::modals::subscription::register_subscription_form_hitboxes;
use crate::ui::pages::settings::{
    SETTINGS_CONFIG_ACTION_IDS, SETTINGS_DIAGNOSTIC_ACTION_IDS, SETTINGS_FORM_ACTION_IDS,
    SETTINGS_GENERAL_ACTION_IDS, SETTINGS_SECURITY_ACTION_IDS, SETTINGS_TRAFFIC_ACTION_IDS,
    SETTINGS_UPDATE_ACTION_IDS,
};
use crate::ui::pages::subscriptions::subscription_action_buttons;
use crate::ui::pages::traffic::{
    traffic_line_chart_lines, traffic_locked_bucket_lines, traffic_status_lines,
    traffic_window_bounds,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use std::sync::mpsc;

fn test_app(rt: &tokio::runtime::Runtime) -> App {
    let (tx, _rx) = mpsc::channel();
    App::new(
        Config {
            api_url: "http://127.0.0.1:9090".into(),
            api_key: String::new(),
        },
        rt.handle().clone(),
        tx,
    )
}

fn selector_proxy(now: &str, all: Vec<&str>) -> ProxyInfo {
    proxy_info("Selector", now, all)
}

fn proxy_info(proxy_type: &str, now: &str, all: Vec<&str>) -> ProxyInfo {
    ProxyInfo {
        proxy_type: proxy_type.into(),
        now: Some(now.into()),
        all: Some(all.into_iter().map(String::from).collect()),
        history: None,
        name: None,
    }
}

#[test]
fn tab_order_matches_authoritative_eight_page_model() {
    assert_eq!(
        Tab::all(),
        &[
            Tab::Subscriptions,
            Tab::Proxies,
            Tab::Connections,
            Tab::Traffic,
            Tab::Network,
            Tab::Logs,
            Tab::Settings,
            Tab::Help,
        ]
    );
    assert_eq!(Tab::Subscriptions.next(), Tab::Proxies);
    assert_eq!(Tab::Traffic.next(), Tab::Network);
    assert_eq!(Tab::Help.next(), Tab::Subscriptions);
}

#[test]
fn hitbox_width_uses_terminal_display_columns_for_cjk_labels() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_settings.language = LanguageSetting::ZhCn;

    assert_eq!(display_width("添加"), 4);
    assert_eq!(action_button_width("添加"), 6);

    register_tab_hitboxes(Rect::new(0, 0, 80, 1), &mut app);

    assert_eq!(
        app.ui_state.hitboxes.action_at(0, 0),
        Some(HitboxAction::SwitchTab(Tab::Subscriptions))
    );
    assert_eq!(
        app.ui_state.hitboxes.action_at(5, 0),
        Some(HitboxAction::SwitchTab(Tab::Subscriptions))
    );
    assert_eq!(
        app.ui_state.hitboxes.action_at(6, 0),
        Some(HitboxAction::SwitchTab(Tab::Proxies))
    );

    let hitboxes =
        crate::ui::components::nav::tab_hitboxes(Rect::new(0, 0, 80, 1), LanguageSetting::ZhCn);
    assert_eq!(hitboxes[0].area, Rect::new(0, 0, 6, 1));
    assert_eq!(hitboxes[1].area.x, 6);
}

#[test]
fn startup_default_page_prefers_subscriptions_until_profiles_exist() {
    assert_eq!(initial_tab_for_profiles(0, "auto"), Tab::Subscriptions);
    assert_eq!(initial_tab_for_profiles(1, "auto"), Tab::Proxies);
    assert_eq!(initial_tab_for_profiles(1, "Auto"), Tab::Proxies);
    assert_eq!(initial_tab_for_profiles(0, "traffic"), Tab::Traffic);
    assert_eq!(initial_tab_for_profiles(3, "network"), Tab::Network);
    assert_eq!(initial_tab_for_profiles(3, "bogus"), Tab::Proxies);
}

#[test]
fn window_move_and_zoom_keep_terminal_diff_cache() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.force_terminal_clear = false;

    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL),
    );
    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE),
    );
    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE),
    );

    assert!(!app.force_terminal_clear);
}

#[test]
fn default_page_setting_cycles_through_auto_and_tabs() {
    assert_eq!(next_default_page("auto"), "subscriptions");
    assert_eq!(next_default_page("subscriptions"), "proxies");
    assert_eq!(next_default_page("help"), "auto");
    assert_eq!(next_default_page("unknown"), "auto");
}

#[test]
fn command_output_lines_trim_empty_and_keep_recent_lines() {
    let input = "\n  first  \n\nsecond\nthird\nfourth\nfifth\nsixth\nseventh\neighth\nninth\n";
    let lines = command_output_lines(input);
    assert_eq!(lines.len(), 9);
    assert_eq!(lines.first().unwrap(), "first");
    assert_eq!(lines.last().unwrap(), "ninth");
}

#[test]
fn command_output_lines_strip_terminal_control_sequences() {
    let input = concat!(
        "\x1b[31m[+]\x1b[0m ok\x1b[K\n",
        "\x1b]0;clashctl\x07[i] done\rprogress 10%\rprogress 20%\n",
        "abc\x08d\n",
        "\x1b]8;;https://example.com\x07link\x1b]8;;\x07\n",
    );
    let lines = command_output_lines(input);

    assert_eq!(
        lines,
        vec![
            "[+] ok",
            "[i] done",
            "progress 10%",
            "progress 20%",
            "abd",
            "link"
        ]
    );
}

#[test]
fn sanitize_terminal_line_collapses_control_sequences_to_plain_text() {
    let line = sanitize_terminal_line("\x1b[31merror\x1b[0m\rretry\n\x1b]0;title\x07done");
    assert_eq!(line, "error retry done");
}

#[test]
fn settings_output_redacts_secret_like_lines() {
    let redacted = redact_sensitive_output("secret: abc\n\"secret\": \"abc\"\napi: ok");
    assert!(redacted.contains("secret: ********"));
    assert!(redacted.contains("\"secret\": ********"));
    assert!(!redacted.contains("abc"));
    assert!(redacted.contains("api: ok"));
}

#[test]
fn traffic_summary_sums_totals_and_tracks_peak_rates() {
    let points = vec![
        TrafficPoint {
            download_delta: 10,
            upload_delta: 1,
            down_bps: 100,
            up_bps: 20,
            ..TrafficPoint::default()
        },
        TrafficPoint {
            download_delta: 20,
            upload_delta: 4,
            down_bps: 80,
            up_bps: 30,
            ..TrafficPoint::default()
        },
    ];
    assert_eq!(traffic_summary(&points), (30, 5, 100, 30));
}

#[test]
fn scaled_bar_does_not_render_zero_value_bars() {
    assert_eq!(scaled_bar(0, 100, 12), "");
    assert_eq!(scaled_bar(100, 0, 12), "");
    assert_eq!(scaled_bar(1, 100, 12), "█");
    assert_eq!(scaled_bar(100, 100, 12), "████████████");
}

#[test]
fn traffic_line_chart_uses_available_vertical_space() {
    let base = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let points: Vec<TrafficPoint> = (0..20)
        .map(|idx| TrafficPoint {
            ts: base + chrono::Duration::minutes(idx),
            download_delta: (idx as u64 + 1) * 10,
            upload_delta: (20 - idx as u64) * 5,
            ..TrafficPoint::default()
        })
        .collect();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = test_app(&rt);
    let lines = traffic_line_chart_lines(&app, &points, 30, 12);
    assert_eq!(lines.len(), 14);
    let rendered = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("download"));
    assert!(rendered.contains("upload"));
    assert!(rendered.contains("12:00"));
    assert!(rendered.contains("12:19"));
}

#[test]
fn traffic_chart_mouse_pans_and_locks_time_buckets() {
    assert_eq!(traffic_window_bounds(20, 5, 0), (15, 20));
    assert_eq!(traffic_window_bounds(20, 5, 3), (12, 17));
    assert_eq!(traffic_window_bounds(20, 5, 99), (0, 5));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Traffic;
    let base = chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    app.traffic_points = (0..20)
        .map(|idx| TrafficPoint {
            ts: base + chrono::Duration::minutes(idx),
            download_delta: (idx as u64 + 1) * 100,
            upload_delta: (idx as u64 + 1) * 10,
            down_bps: (idx as u64 + 1) * 1000,
            up_bps: (idx as u64 + 1) * 100,
            connections: idx as usize,
            ..TrafficPoint::default()
        })
        .collect();
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 0, 80, 12), HitboxAction::ScrollTrafficChart);
    app.ui_state.hitboxes.register(
        Rect::new(20, 0, 1, 12),
        HitboxAction::SelectTrafficBucket(12),
    );

    app.handle_mouse_event(MouseEventKind::ScrollDown, 2, 2);
    assert_eq!(app.ui_state.traffic.window_offset, 3);
    app.handle_mouse_event(MouseEventKind::ScrollUp, 2, 2);
    assert_eq!(app.ui_state.traffic.window_offset, 0);

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 20, 2);
    assert_eq!(app.ui_state.traffic.locked_bucket, Some(12));
    let locked = traffic_locked_bucket_lines(&app)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(locked.contains("12:12:00"));
    assert!(locked.contains("conn 12"));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 2);
    assert_eq!(app.ui_state.traffic.locked_bucket, None);
}

#[test]
fn traffic_controls_map_to_cli_query_arguments() {
    assert_eq!(TrafficRange::LastHour.range_arg(), "1h");
    assert_eq!(TrafficRange::LastHour.step_arg(), "10s");
    assert_eq!(TrafficRange::Day.range_arg(), "24h");
    assert_eq!(TrafficRange::Day.step_arg(), "1m");
    assert_eq!(TrafficRange::Week.range_arg(), "168h");
    assert_eq!(TrafficDimension::Route.arg(), "route");
    assert_eq!(TrafficDimension::Node.arg(), "node");
    assert_eq!(TrafficDimension::Network.arg(), "network");
}

#[test]
fn traffic_controls_cycle_predictably() {
    assert_eq!(TrafficRange::LastHour.next(), TrafficRange::SixHours);
    assert_eq!(TrafficRange::LastHour.prev(), TrafficRange::Week);
    assert_eq!(TrafficChartKind::Line.next(), TrafficChartKind::Bar);
    assert_eq!(TrafficChartKind::Bar.next(), TrafficChartKind::Line);
    assert_eq!(TrafficDimension::Route.next(), TrafficDimension::Rule);
    assert_eq!(TrafficDimension::Network.next(), TrafficDimension::Route);
}

#[test]
fn traffic_default_settings_map_to_stable_keys() {
    assert_eq!(TrafficRange::from_setting_key("1h"), TrafficRange::LastHour);
    assert_eq!(TrafficRange::from_setting_key("168h"), TrafficRange::Week);
    assert_eq!(TrafficRange::from_setting_key("bogus"), TrafficRange::Day);
    assert_eq!(TrafficRange::Week.setting_key(), "7d");

    assert_eq!(
        TrafficChartKind::from_setting_key("bar"),
        TrafficChartKind::Bar
    );
    assert_eq!(
        TrafficChartKind::from_setting_key("unknown"),
        TrafficChartKind::Line
    );
    assert_eq!(TrafficChartKind::Line.setting_key(), "line");

    assert_eq!(
        TrafficDimension::from_setting_key("process"),
        TrafficDimension::Process
    );
    assert_eq!(
        TrafficDimension::from_setting_key("unknown"),
        TrafficDimension::Route
    );
    assert_eq!(TrafficDimension::Network.setting_key(), "network");
}

#[test]
fn registry_driven_visible_actions_have_hitbox_dispatch() {
    let settings_ids = [
        SETTINGS_GENERAL_ACTION_IDS,
        SETTINGS_TRAFFIC_ACTION_IDS,
        SETTINGS_DIAGNOSTIC_ACTION_IDS,
        SETTINGS_CONFIG_ACTION_IDS,
        SETTINGS_FORM_ACTION_IDS,
        SETTINGS_SECURITY_ACTION_IDS,
        SETTINGS_UPDATE_ACTION_IDS,
    ];

    for id in settings_ids.into_iter().flatten() {
        let spec = action_registry::action_by_id(id).expect("registered settings action");
        assert!(
            hitbox_for_action_spec(spec).is_some(),
            "{id} must dispatch to a hitbox action"
        );
    }

    for spec in action_registry::traffic_action_specs() {
        assert!(
            hitbox_for_action_spec(spec).is_some(),
            "{} must dispatch to a hitbox action",
            spec.id
        );
    }
    for spec in action_registry::proxy_action_specs()
        .chain(action_registry::connection_action_specs())
        .chain(action_registry::log_action_specs())
    {
        assert!(
            hitbox_for_action_spec(spec).is_some(),
            "{} must dispatch to a hitbox action",
            spec.id
        );
    }
}

#[test]
fn command_palette_filters_registry_actions() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.open_command_palette();
    for c in "traffic reset".chars() {
        app.push_command_palette_char(c);
    }

    let matches = app.command_palette_matches();
    assert!(matches.iter().any(|spec| spec.id == "traffic.reset"));
}

#[test]
fn command_palette_filters_translated_action_text() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_settings.language = LanguageSetting::ZhCn;

    app.open_command_palette();
    for c in "开启 TUN".chars() {
        app.push_command_palette_char(c);
    }

    let matches = app.command_palette_matches();
    assert!(matches.iter().any(|spec| spec.id == "network.tun.on"));
}

#[test]
fn command_palette_submit_can_switch_pages() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.open_command_palette();
    for c in "nav.network".chars() {
        app.push_command_palette_char(c);
    }
    app.submit_command_palette();

    assert_eq!(app.ui_state.active_page, Tab::Network);
    assert!(!app.command_palette_active());
}

#[test]
fn command_palette_submit_can_open_settings_forms() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.open_command_palette();
    for c in "settings.config.set_dns".chars() {
        app.push_command_palette_char(c);
    }
    app.submit_command_palette();

    assert_eq!(
        app.ui_state
            .settings
            .prompt
            .as_ref()
            .map(|prompt| prompt.kind),
        Some(SettingsPromptKind::ConfigSetDnsMode)
    );

    app.close_command_palette();
    app.cancel_settings_prompt();
    app.open_command_palette();
    for c in "settings.traffic.prune_retention".chars() {
        app.push_command_palette_char(c);
    }
    app.submit_command_palette();

    assert_eq!(
        app.ui_state
            .settings
            .prompt
            .as_ref()
            .map(|prompt| prompt.kind),
        Some(SettingsPromptKind::TrafficPruneRetention)
    );

    app.close_command_palette();
    app.cancel_settings_prompt();
    app.open_command_palette();
    for c in "settings.geodata.version".chars() {
        app.push_command_palette_char(c);
    }
    app.submit_command_palette();

    assert_eq!(
        app.ui_state
            .settings
            .prompt
            .as_ref()
            .map(|prompt| prompt.kind),
        Some(SettingsPromptKind::GeodataUpdateVersion)
    );
}

#[test]
fn traffic_status_lines_include_store_counts_and_last_sample() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.traffic_status = TrafficStatus {
        store_dir: "/tmp/clashctl/traffic".into(),
        raw_samples: 3,
        rollup_10s: 2,
        rollup_1m: 1,
        last_sample: "2026-07-02 12:00:00".into(),
        tracked_connections: 4,
        collector_running: true,
        collector_pid: 1234,
        collector_interval: "1s".into(),
        collector_log: "/tmp/clashctl/traffic/collector.log".into(),
        ..TrafficStatus::default()
    };
    let rendered = traffic_status_lines(&app)
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("/tmp/clashctl/traffic"));
    assert!(rendered.contains("raw 3"));
    assert!(rendered.contains("10s 2"));
    assert!(rendered.contains("1m 1"));
    assert!(rendered.contains("tracked 4"));
    assert!(rendered.contains("2026-07-02 12:00:00"));
    assert!(rendered.contains("running pid=1234"));
    assert!(rendered.contains("interval=1s"));
}

#[test]
fn traffic_export_event_reports_saved_path_or_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::TrafficExport(Ok("/tmp/traffic.csv".into())));
    assert_eq!(
        app.status_msg.as_deref(),
        Some("traffic exported: /tmp/traffic.csv")
    );
    assert!(app.error_msg.is_none());

    app.apply_data_event(DataEvent::TrafficExport(Err("disk full".into())));
    assert_eq!(
        app.error_msg.as_deref(),
        Some("Traffic export failed: disk full")
    );
}

#[test]
fn traffic_action_event_reports_output_or_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::TrafficActionResult(
        "traffic sample".into(),
        Ok("[+] sampled".into()),
    ));
    assert_eq!(
        app.status_msg.as_deref(),
        Some("traffic action completed: traffic sample")
    );
    assert_eq!(app.ui_state.traffic.output, vec!["[+] sampled"]);

    app.apply_data_event(DataEvent::TrafficActionResult(
        "traffic sample".into(),
        Err("kernel unavailable".into()),
    ));
    assert_eq!(
        app.error_msg.as_deref(),
        Some("Traffic action failed: traffic sample: kernel unavailable")
    );
    assert_eq!(app.ui_state.traffic.output, vec!["kernel unavailable"]);
}

#[test]
fn settings_result_event_reports_output_or_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::SettingsResult(
        SettingsAction::Doctor,
        Ok("[+] ok\nsecret: clear".into()),
    ));
    assert_eq!(
        app.status_msg.as_deref(),
        Some("settings action completed: doctor")
    );
    assert!(app
        .ui_state
        .settings
        .output
        .iter()
        .any(|line| line.contains("[+] ok")));
    assert!(app
        .ui_state
        .settings
        .output
        .iter()
        .any(|line| line.contains("secret: ********")));

    app.apply_data_event(DataEvent::SettingsResult(
        SettingsAction::Doctor,
        Err("failed".into()),
    ));
    assert_eq!(
        app.error_msg.as_deref(),
        Some("Settings action failed: doctor: failed")
    );
}

#[test]
fn settings_secret_reveal_event_allows_explicit_secret_output() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::SettingsResult(
        SettingsAction::SecretReveal,
        Ok("current secret: visible-secret".into()),
    ));

    assert!(app
        .ui_state
        .settings
        .output
        .iter()
        .any(|line| line.contains("visible-secret")));
}

#[test]
fn settings_command_result_redacts_secret_set_output() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::SettingsCommandResult(
        "Set API secret".into(),
        true,
        Ok("secret: new-secret".into()),
    ));

    assert!(app
        .ui_state
        .settings
        .output
        .iter()
        .any(|line| line.contains("secret: ********")));
    assert!(!app
        .ui_state
        .settings
        .output
        .iter()
        .any(|line| line.contains("new-secret")));
}

#[test]
fn subscription_output_event_keeps_multiline_output() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);

    app.apply_data_event(DataEvent::SubscriptionOutputResult(
        "log".into(),
        Ok("line one\nline two".into()),
    ));

    assert_eq!(
        app.status_msg.as_deref(),
        Some("subscription action completed: log")
    );
    assert_eq!(
        app.ui_state.subscriptions.output,
        vec!["line one", "line two"]
    );
}

#[test]
fn subscription_permission_error_opens_sudo_prompt() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.prepare_sudo_candidate(
        "use subscription 5".into(),
        vec!["sub".into(), "use".into(), "5".into()],
        SudoTarget::Subscription,
    );

    app.apply_data_event(DataEvent::SubscriptionResult(Err(
        "systemctl stop clashctl: Interactive authentication required".into(),
    )));

    assert!(app.sudo_prompt_active());
    assert_eq!(
        app.ui_state
            .modals
            .sudo_prompt
            .as_ref()
            .map(|prompt| prompt.label.as_str()),
        Some("use subscription 5")
    );
    assert!(app.error_msg.is_none());
    assert!(app
        .ui_state
        .subscriptions
        .output
        .iter()
        .any(|line| line.contains("Sudo") || line.contains("sudo")));
}

#[test]
fn subscription_output_permission_error_opens_sudo_prompt() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.prepare_sudo_candidate(
        "subscription log".into(),
        vec!["sub".into(), "log".into()],
        SudoTarget::SubscriptionOutput {
            output_label: "log".into(),
        },
    );

    app.apply_data_event(DataEvent::SubscriptionOutputResult(
        "log".into(),
        Err("permission denied; sudo required".into()),
    ));

    assert!(app.sudo_prompt_active());
    assert_eq!(
        app.ui_state
            .modals
            .sudo_prompt
            .as_ref()
            .map(|prompt| prompt.label.as_str()),
        Some("subscription log")
    );
}

#[test]
fn dangerous_network_action_opens_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Network;

    app.run_network_action(NetworkAction::Stop);

    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("stop"));
    assert!(pending.message.contains("clashctl stop"));

    app.cancel_pending_action();
    assert!(app.ui_state.modals.pending_confirmation.is_none());
    assert_eq!(app.status_msg.as_deref(), Some("Action cancelled"));
}

#[test]
fn dangerous_settings_action_opens_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.run_settings_action(SettingsAction::KernelUpgrade);

    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("kernel upgrade"));
    assert!(pending.message.contains("clashctl upgrade-kernel"));
}

#[test]
fn secret_set_prompt_opens_redacted_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.begin_secret_set();
    app.push_settings_prompt_char('s');
    app.push_settings_prompt_char('3');
    app.push_settings_prompt_char('c');
    app.submit_settings_prompt();

    assert!(app.ui_state.settings.prompt.is_none());
    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("Set secret"));
    assert!(pending.message.contains("clashctl secret ********"));
    assert!(!pending.message.contains("s3c"));
    match &pending.action {
        super::PendingAction::SettingsCommand {
            label,
            args,
            redact_output,
        } => {
            assert_eq!(label, "Set API secret");
            assert_eq!(args, &vec!["secret".to_string(), "s3c".to_string()]);
            assert!(*redact_output);
        }
        other => panic!("unexpected pending action: {other:?}"),
    }
}

#[test]
fn config_set_ports_prompt_parses_cli_arguments() {
    assert_eq!(
        parse_config_set_ports_args("7897").unwrap(),
        vec![
            "config".to_string(),
            "set-port".to_string(),
            "7897".to_string()
        ]
    );
    assert_eq!(
        parse_config_set_ports_args("mixed=7897 http=7898 socks=7899").unwrap(),
        vec![
            "config".to_string(),
            "set-port".to_string(),
            "7897".to_string(),
            "--http".to_string(),
            "7898".to_string(),
            "--socks".to_string(),
            "7899".to_string()
        ]
    );
    assert!(parse_config_set_ports_args("mixed=0").is_err());
    assert!(parse_config_set_ports_args("mixed=7897 redir=7892").is_err());
}

#[test]
fn config_set_api_prompt_parses_controller_secret_and_unsafe_flag() {
    assert_eq!(
        parse_config_set_api_args("127.0.0.1:9090").unwrap(),
        vec![
            "config".to_string(),
            "set-api".to_string(),
            "127.0.0.1:9090".to_string()
        ]
    );
    assert_eq!(
        parse_config_set_api_args("controller=0.0.0.0:9090 secret=s3c allow-unsafe=true").unwrap(),
        vec![
            "config".to_string(),
            "set-api".to_string(),
            "0.0.0.0:9090".to_string(),
            "--secret".to_string(),
            "s3c".to_string(),
            "--allow-unsafe".to_string(),
        ]
    );
    assert_eq!(
        parse_config_set_api_args("0.0.0.0:9090 --secret s3c --allow-unsafe").unwrap(),
        vec![
            "config".to_string(),
            "set-api".to_string(),
            "0.0.0.0:9090".to_string(),
            "--secret".to_string(),
            "s3c".to_string(),
            "--allow-unsafe".to_string(),
        ]
    );
    assert!(parse_config_set_api_args("secret=s3c").is_err());
    assert!(parse_config_set_api_args("127.0.0.1:9090 allow-unsafe=maybe").is_err());
    assert!(parse_config_set_api_args("127.0.0.1:9090 unknown=value").is_err());
}

#[test]
fn traffic_prune_retention_prompt_parses_duration_windows() {
    assert_eq!(
        parse_traffic_prune_retention_args("24h").unwrap(),
        vec![
            "traffic".to_string(),
            "prune".to_string(),
            "--retention".to_string(),
            "24h".to_string(),
        ]
    );
    assert_eq!(
        parse_traffic_prune_retention_args("raw=24h rollup-10s=7d rollup-1m=90d").unwrap(),
        vec![
            "traffic".to_string(),
            "prune".to_string(),
            "--retention".to_string(),
            "24h".to_string(),
            "--rollup-10s-retention".to_string(),
            "168h".to_string(),
            "--rollup-1m-retention".to_string(),
            "2160h".to_string(),
        ]
    );
    assert!(parse_traffic_prune_retention_args("").is_err());
    assert!(parse_traffic_prune_retention_args("0h").is_err());
    assert!(parse_traffic_prune_retention_args("raw=24h yearly=1y").is_err());
    assert!(parse_traffic_prune_retention_args("1.5h").is_err());
}

#[test]
fn geodata_update_version_prompt_builds_version_flag() {
    assert_eq!(
        parse_geodata_update_version_args("latest").unwrap(),
        vec![
            "geodata".to_string(),
            "update".to_string(),
            "--version".to_string(),
            "latest".to_string(),
        ]
    );
    assert_eq!(
        parse_geodata_update_version_args("v2025.01.01").unwrap(),
        vec![
            "geodata".to_string(),
            "update".to_string(),
            "--version".to_string(),
            "v2025.01.01".to_string(),
        ]
    );
    assert!(parse_geodata_update_version_args("").is_err());
    assert!(parse_geodata_update_version_args("latest extra").is_err());
}

#[test]
fn config_set_api_prompt_redacts_secret_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
    for c in "controller=0.0.0.0:9090 secret=s3c allow-unsafe=true".chars() {
        app.push_settings_prompt_char(c);
    }
    app.submit_settings_prompt();

    assert!(app.ui_state.settings.prompt.is_none());
    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("Set API"));
    assert!(pending
        .message
        .contains("clashctl config set-api 0.0.0.0:9090 --secret ******** --allow-unsafe"));
    assert!(!pending.message.contains("s3c"));
    match &pending.action {
        super::PendingAction::SettingsCommand {
            label,
            args,
            redact_output,
        } => {
            assert_eq!(label, "Set API controller");
            assert_eq!(
                args,
                &vec![
                    "config".to_string(),
                    "set-api".to_string(),
                    "0.0.0.0:9090".to_string(),
                    "--secret".to_string(),
                    "s3c".to_string(),
                    "--allow-unsafe".to_string(),
                ]
            );
            assert!(*redact_output);
        }
        other => panic!("unexpected pending action: {other:?}"),
    }
}

#[test]
fn config_set_prompt_opens_confirmation_with_cli_args() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
    for c in "mixed=7897 http=7898 socks=7899".chars() {
        app.push_settings_prompt_char(c);
    }
    app.submit_settings_prompt();

    assert!(app.ui_state.settings.prompt.is_none());
    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("Set ports"));
    assert!(pending
        .message
        .contains("clashctl config set-port 7897 --http 7898 --socks 7899"));
    match &pending.action {
        super::PendingAction::SettingsCommand {
            label,
            args,
            redact_output,
        } => {
            assert_eq!(label, "Set proxy ports");
            assert_eq!(
                args,
                &vec![
                    "config".to_string(),
                    "set-port".to_string(),
                    "7897".to_string(),
                    "--http".to_string(),
                    "7898".to_string(),
                    "--socks".to_string(),
                    "7899".to_string()
                ]
            );
            assert!(*redact_output);
        }
        other => panic!("unexpected pending action: {other:?}"),
    }
}

#[test]
fn settings_prompt_structured_fields_build_cli_args() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
    for c in "7897".chars() {
        app.push_settings_prompt_char(c);
    }
    app.next_settings_prompt_field();
    for c in "7898".chars() {
        app.push_settings_prompt_char(c);
    }
    app.next_settings_prompt_field();
    for c in "7899".chars() {
        app.push_settings_prompt_char(c);
    }
    app.submit_settings_prompt();

    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    match &pending.action {
        super::PendingAction::SettingsCommand { args, .. } => {
            assert_eq!(
                args,
                &vec![
                    "config".to_string(),
                    "set-port".to_string(),
                    "7897".to_string(),
                    "--http".to_string(),
                    "7898".to_string(),
                    "--socks".to_string(),
                    "7899".to_string(),
                ]
            );
        }
        other => panic!("unexpected pending action: {other:?}"),
    }
}

#[test]
fn mouse_selects_settings_prompt_field() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;
    app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
    app.ui_state.hitboxes.register(
        Rect::new(0, 1, 20, 1),
        HitboxAction::SelectSettingsPromptField(1),
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

    assert_eq!(app.ui_state.settings.prompt.as_ref().unwrap().active, 1);
    assert_eq!(app.status_msg.as_deref(), Some("editing Set API · Secret"));
}

#[test]
fn config_set_prompt_validation_keeps_prompt_open() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;

    app.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
    for c in "invalid".chars() {
        app.push_settings_prompt_char(c);
    }
    app.submit_settings_prompt();

    assert!(app.ui_state.modals.pending_confirmation.is_none());
    assert!(app.ui_state.settings.prompt.is_some());
    assert!(app
        .error_msg
        .as_deref()
        .unwrap_or("")
        .contains("DNS mode must be"));
}

#[test]
fn settings_mouse_config_form_actions_open_prompts() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 1, 10, 1), HitboxAction::BeginConfigSetLan);

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

    assert_eq!(
        app.ui_state
            .settings
            .prompt
            .as_ref()
            .map(|prompt| prompt.kind),
        Some(SettingsPromptKind::ConfigSetLan)
    );
}

#[test]
fn mouse_click_focuses_settings_prompt_value() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Settings;
    app.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
    app.error_msg = Some("stale error".into());
    app.ui_state.hitboxes.register(
        Rect::new(0, 1, 20, 1),
        HitboxAction::FocusSettingsPromptValue,
    );
    app.ui_state.hitboxes.register(
        Rect::new(0, 2, 20, 1),
        HitboxAction::RunSettings(SettingsAction::Doctor),
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 1);

    assert!(app.ui_state.settings.prompt.is_some());
    assert!(app.error_msg.is_none());
    assert_eq!(
        app.status_msg.as_deref(),
        Some("editing Update geodata version · Version")
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 2);
    assert!(app.ui_state.modals.pending_confirmation.is_none());
    assert_eq!(
        app.ui_state
            .settings
            .prompt
            .as_ref()
            .map(|prompt| prompt.kind),
        Some(SettingsPromptKind::GeodataUpdateVersion)
    );
}

#[test]
fn dangerous_traffic_action_opens_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Traffic;

    app.run_traffic_action(TrafficAction::Reset);

    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("traffic reset"));
    assert!(pending.message.contains("clashctl traffic reset --yes"));
}

#[test]
fn profile_interval_label_prefers_new_metadata_and_disables_file_profiles() {
    let mut profile = ProfileEntry {
        id: 1,
        path: String::new(),
        url: "https://example.test/sub".into(),
        name: String::new(),
        updated: String::new(),
        interval: "12h".into(),
        update_enabled: None,
        update_interval: "6h".into(),
        update_proxy: String::new(),
        user_agent: String::new(),
        convert_mode: String::new(),
        tags: Vec::new(),
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    };
    assert_eq!(profile_interval_label(&profile), "6h");

    profile.update_interval.clear();
    assert_eq!(profile_interval_label(&profile), "12h");

    profile.update_enabled = Some(false);
    assert_eq!(profile_interval_label(&profile), "off");

    profile.update_enabled = None;
    profile.interval.clear();
    profile.url = "file:///tmp/local.yaml".into();
    assert_eq!(profile_interval_label(&profile), "off");
}

#[test]
fn subscription_edit_field_builds_cli_args() {
    assert_eq!(
        SubscriptionEditField::ImportDirectory.args(0, "/tmp/subs".into()),
        vec![
            "sub".to_string(),
            "import".to_string(),
            "/tmp/subs".to_string()
        ]
    );
    assert_eq!(
        SubscriptionEditField::Interval.args(7, "24h".into()),
        vec![
            "sub".to_string(),
            "set-interval".to_string(),
            "7".to_string(),
            "24h".to_string()
        ]
    );
    assert_eq!(
        SubscriptionEditField::AddTag.args(7, "work".into()),
        vec![
            "sub".to_string(),
            "tag".to_string(),
            "add".to_string(),
            "7".to_string(),
            "work".to_string()
        ]
    );
}

#[test]
fn subscription_add_form_builds_parameterized_cli_args() {
    let mut form = SubscriptionAddForm::new();
    form.source = "https://example.test/sub".into();
    form.name = "Work".into();
    form.interval = "6h".into();
    form.update_proxy = "Core".into();
    form.user_agent = "clashctl-test".into();
    form.convert_mode = "Force".into();
    form.tags = "daily, work, daily".into();

    assert_eq!(
        form.args().unwrap(),
        vec![
            "sub".to_string(),
            "add".to_string(),
            "https://example.test/sub".to_string(),
            "--name".to_string(),
            "Work".to_string(),
            "--interval".to_string(),
            "6h".to_string(),
            "--update-proxy".to_string(),
            "core".to_string(),
            "--user-agent".to_string(),
            "clashctl-test".to_string(),
            "--convert".to_string(),
            "force".to_string(),
            "--tag".to_string(),
            "daily".to_string(),
            "--tag".to_string(),
            "work".to_string(),
        ]
    );
}

#[test]
fn subscription_add_form_validates_required_and_enum_fields() {
    let mut form = SubscriptionAddForm::new();
    assert!(form.args().unwrap_err().contains("source"));

    form.source = "https://example.test/sub".into();
    form.update_proxy = "vpn".into();
    assert!(form.args().unwrap_err().contains("update proxy"));

    form.update_proxy = "auto".into();
    form.convert_mode = "maybe".into();
    assert!(form.args().unwrap_err().contains("convert mode"));
}

#[test]
fn subscription_edit_form_builds_changed_cli_commands() {
    let profile = ProfileEntry {
        id: 7,
        path: String::new(),
        url: "https://example.test/old".into(),
        name: "Old".into(),
        updated: String::new(),
        interval: "12h".into(),
        update_enabled: Some(true),
        update_interval: "12h".into(),
        update_proxy: "auto".into(),
        user_agent: "old-ua".into(),
        convert_mode: "auto".into(),
        tags: vec!["daily".into(), "home".into()],
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    };
    let mut form = SubscriptionEditForm::from_profile(&profile);
    form.name = "New".into();
    form.url = "https://example.test/new".into();
    form.interval = "6h".into();
    form.update_proxy = "Core".into();
    form.user_agent = "new-ua".into();
    form.convert_mode = "Force".into();
    form.tags = "daily, work".into();

    assert_eq!(
        form.commands().unwrap(),
        vec![
            vec![
                "sub".to_string(),
                "rename".to_string(),
                "7".to_string(),
                "New".to_string()
            ],
            vec![
                "sub".to_string(),
                "set-url".to_string(),
                "7".to_string(),
                "https://example.test/new".to_string()
            ],
            vec![
                "sub".to_string(),
                "set-interval".to_string(),
                "7".to_string(),
                "6h".to_string()
            ],
            vec![
                "sub".to_string(),
                "set-update-proxy".to_string(),
                "7".to_string(),
                "core".to_string()
            ],
            vec![
                "sub".to_string(),
                "set-user-agent".to_string(),
                "7".to_string(),
                "new-ua".to_string()
            ],
            vec![
                "sub".to_string(),
                "set-convert".to_string(),
                "7".to_string(),
                "force".to_string()
            ],
            vec![
                "sub".to_string(),
                "tag".to_string(),
                "remove".to_string(),
                "7".to_string(),
                "home".to_string()
            ],
            vec![
                "sub".to_string(),
                "tag".to_string(),
                "add".to_string(),
                "7".to_string(),
                "work".to_string()
            ],
        ]
    );
}

#[test]
fn subscription_edit_form_validates_required_fields_and_noop() {
    let profile = ProfileEntry {
        id: 8,
        path: String::new(),
        url: "https://example.test/sub".into(),
        name: String::new(),
        updated: String::new(),
        interval: String::new(),
        update_enabled: None,
        update_interval: String::new(),
        update_proxy: String::new(),
        user_agent: String::new(),
        convert_mode: String::new(),
        tags: Vec::new(),
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    };
    let mut form = SubscriptionEditForm::from_profile(&profile);
    assert!(form.commands().unwrap().is_empty());

    form.url.clear();
    assert!(form.commands().unwrap_err().contains("source"));

    form.url = profile.url.clone();
    form.update_proxy = "vpn".into();
    assert!(form.commands().unwrap_err().contains("update proxy"));
}

#[test]
fn subscription_edit_prompt_prefills_profile_metadata() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.profiles = vec![ProfileEntry {
        id: 9,
        path: String::new(),
        url: "https://example.test/sub.yaml".into(),
        name: "Example".into(),
        updated: String::new(),
        interval: "12h".into(),
        update_enabled: Some(true),
        update_interval: "6h".into(),
        update_proxy: "core".into(),
        user_agent: "clashctl-test".into(),
        convert_mode: "force".into(),
        tags: vec!["daily".into()],
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    }];

    app.begin_subscription_edit(SubscriptionEditField::Interval);
    let prompt = app.ui_state.subscriptions.prompt.as_ref().unwrap();
    assert_eq!(prompt.profile_id, 9);
    assert_eq!(prompt.value, "6h");

    app.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
    assert_eq!(
        app.ui_state.subscriptions.prompt.as_ref().unwrap().value,
        "core".to_string()
    );
}

#[test]
fn subscription_profile_edit_form_prefills_profile_metadata() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.profiles = vec![ProfileEntry {
        id: 10,
        path: String::new(),
        url: "https://example.test/sub.yaml".into(),
        name: "Example".into(),
        updated: String::new(),
        interval: "12h".into(),
        update_enabled: Some(true),
        update_interval: "6h".into(),
        update_proxy: "core".into(),
        user_agent: "clashctl-test".into(),
        convert_mode: "force".into(),
        tags: vec!["daily".into(), "work".into()],
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    }];

    app.begin_subscription_profile_edit();

    let form = app.ui_state.subscriptions.edit_form.as_ref().unwrap();
    assert_eq!(form.profile_id, 10);
    assert_eq!(form.name, "Example");
    assert_eq!(form.url, "https://example.test/sub.yaml");
    assert_eq!(form.interval, "6h");
    assert_eq!(form.update_proxy, "core");
    assert_eq!(form.user_agent, "clashctl-test");
    assert_eq!(form.convert_mode, "force");
    assert_eq!(form.tags, "daily, work");
}

#[test]
fn subscription_add_and_import_prompts_do_not_require_existing_profile() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;

    app.begin_subscription_add();
    let form = app.ui_state.subscriptions.add_form.as_ref().unwrap();
    assert!(form.source.is_empty());
    assert_eq!(form.update_proxy, "auto");
    assert_eq!(form.convert_mode, "auto");
    assert!(app.ui_state.subscriptions.prompt.is_none());

    app.begin_subscription_import();
    let prompt = app.ui_state.subscriptions.prompt.as_ref().unwrap();
    assert_eq!(prompt.field, SubscriptionEditField::ImportDirectory);
    assert_eq!(prompt.profile_id, 0);
    assert!(prompt.value.is_empty());
    assert!(app.ui_state.subscriptions.add_form.is_none());
}

#[test]
fn subscription_add_form_keyboard_navigation_targets_active_field() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;

    app.begin_subscription_add();
    app.push_subscription_prompt_char('h');
    app.next_subscription_form_field();
    app.push_subscription_prompt_char('W');
    app.prev_subscription_form_field();
    app.push_subscription_prompt_char('t');

    let form = app.ui_state.subscriptions.add_form.as_ref().unwrap();
    assert_eq!(form.source, "ht");
    assert_eq!(form.name, "W");
}

#[test]
fn subscription_add_form_empty_source_keeps_form_open() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;

    app.begin_subscription_add();
    app.submit_subscription_prompt();

    assert!(app.ui_state.subscriptions.add_form.is_some());
    assert!(app.error_msg.as_deref().unwrap_or("").contains("source"));
}

#[test]
fn remove_subscription_opens_confirmation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.profiles = vec![ProfileEntry {
        id: 5,
        path: String::new(),
        url: "https://example.test/sub".into(),
        name: String::new(),
        updated: String::new(),
        interval: String::new(),
        update_enabled: None,
        update_interval: String::new(),
        update_proxy: String::new(),
        user_agent: String::new(),
        convert_mode: String::new(),
        tags: Vec::new(),
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    }];

    app.remove_selected_subscription();

    let pending = app.ui_state.modals.pending_confirmation.as_ref().unwrap();
    assert!(pending.title.contains("remove subscription 5"));
    assert!(pending.message.contains("clashctl sub remove 5"));
}

#[test]
fn mouse_click_switches_tab_through_hitbox_registry() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.hitboxes.register(
        Rect::new(1, 1, 10, 1),
        HitboxAction::SwitchTab(Tab::Connections),
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(app.ui_state.active_page, Tab::Connections);
}

#[test]
fn mouse_click_selects_subscription_row() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.profiles = vec![
        ProfileEntry {
            id: 1,
            path: String::new(),
            url: "https://one.example/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        },
        ProfileEntry {
            id: 2,
            path: String::new(),
            url: "https://two.example/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        },
    ];
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 5, 40, 1), HitboxAction::SelectSubscription(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 5);

    assert_eq!(app.ui_state.subscriptions.selected_idx, 1);
}

#[test]
fn mouse_click_subscription_edit_action_opens_prompt() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.profiles = vec![ProfileEntry {
        id: 7,
        path: String::new(),
        url: "https://example.test/sub".into(),
        name: "Example".into(),
        updated: String::new(),
        interval: "12h".into(),
        update_enabled: None,
        update_interval: "6h".into(),
        update_proxy: "auto".into(),
        user_agent: String::new(),
        convert_mode: String::new(),
        tags: Vec::new(),
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    }];
    app.ui_state.hitboxes.register(
        Rect::new(0, 5, 20, 1),
        HitboxAction::EditSubscriptionInterval,
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 1, 5);

    let prompt = app.ui_state.subscriptions.prompt.as_ref().unwrap();
    assert_eq!(prompt.profile_id, 7);
    assert_eq!(prompt.field, SubscriptionEditField::Interval);
    assert_eq!(prompt.value, "6h");
}

#[test]
fn pending_confirmation_blocks_underlying_mouse_actions() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.profiles = vec![
        ProfileEntry {
            id: 1,
            path: String::new(),
            url: "https://one.example/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        },
        ProfileEntry {
            id: 2,
            path: String::new(),
            url: "https://two.example/sub".into(),
            name: String::new(),
            updated: String::new(),
            interval: String::new(),
            update_enabled: None,
            update_interval: String::new(),
            update_proxy: String::new(),
            user_agent: String::new(),
            convert_mode: String::new(),
            tags: Vec::new(),
            last_error: String::new(),
            last_updated: String::new(),
            next_update: String::new(),
        },
    ];
    app.ui_state.modals.pending_confirmation = Some(super::PendingConfirmation {
        title: "Confirm stop".into(),
        message: "Run `clashctl stop`?".into(),
        action: super::PendingAction::Network(NetworkAction::Stop),
    });
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 5, 40, 1), HitboxAction::SelectSubscription(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 5);

    assert_eq!(app.ui_state.subscriptions.selected_idx, 0);
    assert!(app.ui_state.modals.pending_confirmation.is_some());
}

#[test]
fn mouse_prompt_submit_and_cancel_actions_are_dispatched() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.subscriptions.prompt = Some(SubscriptionPrompt {
        field: SubscriptionEditField::Url,
        profile_id: 1,
        value: String::new(),
    });
    app.ui_state.hitboxes.register(
        Rect::new(1, 1, 10, 1),
        HitboxAction::SubmitSubscriptionPrompt,
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);
    assert!(app
        .error_msg
        .as_deref()
        .unwrap_or("")
        .contains("cannot be empty"));
    assert!(app.ui_state.subscriptions.prompt.is_some());

    app.ui_state.hitboxes.clear();
    app.ui_state.hitboxes.register(
        Rect::new(1, 2, 10, 1),
        HitboxAction::CancelSubscriptionPrompt,
    );
    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 2);
    assert!(app.ui_state.subscriptions.prompt.is_none());
}

#[test]
fn mouse_selects_subscription_add_form_field() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.begin_subscription_add();
    app.ui_state.hitboxes.register(
        Rect::new(1, 1, 20, 1),
        HitboxAction::SelectSubscriptionAddField(3),
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(
        app.ui_state.subscriptions.add_form.as_ref().unwrap().active,
        3
    );
}

#[test]
fn mouse_selects_subscription_edit_form_field() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Subscriptions;
    app.profiles = vec![ProfileEntry {
        id: 12,
        path: String::new(),
        url: "https://example.test/sub".into(),
        name: "Example".into(),
        updated: String::new(),
        interval: String::new(),
        update_enabled: None,
        update_interval: String::new(),
        update_proxy: String::new(),
        user_agent: String::new(),
        convert_mode: String::new(),
        tags: Vec::new(),
        last_error: String::new(),
        last_updated: String::new(),
        next_update: String::new(),
    }];
    app.begin_subscription_profile_edit();
    app.ui_state.hitboxes.register(
        Rect::new(1, 1, 20, 1),
        HitboxAction::SelectSubscriptionAddField(5),
    );

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(
        app.ui_state
            .subscriptions
            .edit_form
            .as_ref()
            .unwrap()
            .active,
        5
    );
}

#[test]
fn subscription_action_hitboxes_follow_translated_buttons() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_settings.language = LanguageSetting::ZhCn;
    let area = Rect::new(10, 20, 180, 4);
    let buttons = subscription_action_buttons(&app)
        .into_iter()
        .map(|(label, action)| action_button_item(label, action, ActionDanger::Safe))
        .collect::<Vec<_>>();
    for (rect, idx) in action_button_rects(area, &buttons) {
        app.ui_state
            .hitboxes
            .register(rect, buttons[idx].action.clone());
        assert_eq!(
            app.ui_state.hitboxes.action_at(rect.x, rect.y),
            Some(buttons[idx].action.clone())
        );
    }
}

#[test]
fn subscription_form_buttons_follow_translated_widths() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_settings.language = LanguageSetting::ZhCn;
    let inner = Rect::new(4, 8, 80, 16);
    let field_y = inner.y.saturating_add(2);
    let button_y = field_y
        .saturating_add(super::SubscriptionAddField::all().len() as u16)
        .saturating_add(1);
    let save_width = action_button_width(app.t(crate::i18n::Msg::CommonSave));

    register_subscription_form_hitboxes(&mut app, inner);

    assert_eq!(
        app.ui_state.hitboxes.action_at(inner.x, button_y),
        Some(HitboxAction::SubmitSubscriptionPrompt)
    );
    assert_eq!(
        app.ui_state.hitboxes.action_at(
            inner.x.saturating_add(save_width).saturating_add(3),
            button_y
        ),
        Some(HitboxAction::CancelSubscriptionPrompt)
    );
}

#[test]
fn mouse_click_selects_proxy_node_when_picker_is_open() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies
        .insert("Proxy".into(), selector_proxy("B", vec!["A", "B", "C"]));
    app.proxy_groups = vec![("Proxy".into(), "B".into())];
    app.open_node_picker();
    app.ui_state
        .hitboxes
        .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxyNode(2));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(app.ui_state.proxies.selected_node_idx, 2);
}

#[test]
fn proxy_group_second_click_opens_node_picker() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxy_groups = vec![("One".into(), "A".into()), ("Two".into(), "B".into())];
    app.proxies
        .insert("One".into(), selector_proxy("A", vec!["A"]));
    app.proxies
        .insert("Two".into(), selector_proxy("B", vec!["B"]));
    app.ui_state
        .hitboxes
        .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxy(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);
    assert_eq!(app.ui_state.proxies.selected_idx, 1);
    assert!(!app.ui_state.proxies.node_picker_open);

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);
    assert!(app.ui_state.proxies.node_picker_open);
}

#[test]
fn proxy_node_second_click_confirms_selection() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies
        .insert("Proxy".into(), selector_proxy("B", vec!["A", "B", "C"]));
    app.proxy_groups = vec![("Proxy".into(), "B".into())];
    app.open_node_picker();
    app.ui_state
        .hitboxes
        .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxyNode(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert!(app.ui_state.proxies.node_picker_open);
    assert_eq!(app.status_msg.as_deref(), Some("switching node: B"));
}

#[test]
fn automatic_proxy_node_second_click_tests_without_manual_switch() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies.insert(
        "Auto".into(),
        proxy_info("URLTest", "B", vec!["A", "B", "C"]),
    );
    app.proxy_groups = vec![("Auto".into(), "B".into())];
    app.open_node_picker();
    app.ui_state
        .hitboxes
        .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxyNode(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(app.selected_proxy_current_node(), "B");
    assert!(app.delay_pending.contains("B"));
    assert!(app
        .status_msg
        .as_deref()
        .is_some_and(|msg| msg.contains(app.t(Msg::ProxyAutoGroupKeepsChoice))));
}

#[test]
fn proxy_delay_result_tracks_timeout_and_success_states() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;

    app.mark_delay_pending("A");
    assert_eq!(app.proxy_delay_text("A"), app.t(Msg::ProxyDelayTesting));

    app.apply_data_event(DataEvent::DelayResult(
        "A".into(),
        Err("504 timeout".into()),
    ));
    assert!(!app.delay_pending.contains("A"));
    assert_eq!(app.proxy_delay_text("A"), app.t(Msg::ProxyDelayTimeout));

    app.apply_data_event(DataEvent::DelayResult("A".into(), Ok(42)));
    assert_eq!(app.proxy_delay_text("A"), "42ms");
    assert!(!app.delay_errors.contains_key("A"));
}

#[test]
fn proxy_switch_result_updates_current_node_locally() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies
        .insert("Proxy".into(), selector_proxy("A", vec!["A", "B"]));
    app.proxy_groups = vec![("Proxy".into(), "A".into())];
    app.open_node_picker();

    app.apply_data_event(DataEvent::SwitchResult("Proxy".into(), "B".into(), Ok(())));

    assert_eq!(app.selected_proxy_current_node(), "B");
    assert_eq!(app.proxy_groups[0].1, "B");
    assert_eq!(app.ui_state.proxies.selected_node_idx, 1);
}

#[test]
fn node_picker_consumes_navigation_keys() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies
        .insert("Proxy".into(), selector_proxy("A", vec!["A", "B"]));
    app.proxy_groups = vec![("Proxy".into(), "A".into())];
    app.open_node_picker();

    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
    );
    assert_eq!(app.ui_state.active_page, Tab::Proxies);
    assert!(app.ui_state.proxies.node_picker_open);

    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_eq!(app.ui_state.active_page, Tab::Proxies);
    assert!(!app.should_quit);
    assert!(!app.ui_state.proxies.node_picker_open);
}

#[test]
fn proxy_actions_do_not_expose_sequential_switch() {
    let proxy_ids = action_registry::proxy_action_specs()
        .map(|spec| spec.id)
        .collect::<Vec<_>>();
    assert!(!proxy_ids.contains(&"proxy.node.switch"));

    let picker_ids = action_registry::node_picker_action_specs()
        .map(|spec| spec.id)
        .collect::<Vec<_>>();
    assert_eq!(
        picker_ids,
        vec![
            "proxy.delay.selected",
            "proxy.delay.all",
            "proxy.node.close"
        ]
    );
}

#[test]
fn node_picker_blocks_underlying_mouse_actions() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxy_groups = vec![("One".into(), "A".into()), ("Two".into(), "B".into())];
    app.proxies
        .insert("One".into(), selector_proxy("A", vec!["A"]));
    app.proxies
        .insert("Two".into(), selector_proxy("B", vec!["B"]));
    app.open_node_picker();
    app.ui_state
        .hitboxes
        .register(Rect::new(1, 1, 20, 1), HitboxAction::SelectProxy(1));

    app.handle_mouse_event(MouseEventKind::Down(MouseButton::Left), 2, 1);

    assert_eq!(app.ui_state.proxies.selected_idx, 0);
    assert!(app.ui_state.proxies.node_picker_open);
}

#[test]
fn mouse_wheel_scrolls_proxy_node_picker() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxies.insert(
        "Proxy".into(),
        selector_proxy("A", vec!["A", "B", "C", "D"]),
    );
    app.proxy_groups = vec![("Proxy".into(), "A".into())];
    app.open_node_picker();
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 0, 40, 10), HitboxAction::ScrollProxyNodes);

    app.handle_mouse_event(MouseEventKind::ScrollDown, 5, 5);
    assert_eq!(app.ui_state.proxies.selected_node_idx, 3);
    app.handle_mouse_event(MouseEventKind::ScrollUp, 5, 5);
    assert_eq!(app.ui_state.proxies.selected_node_idx, 0);
}

#[test]
fn mouse_wheel_dispatches_to_registered_scroll_area() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Logs;
    app.logs = (0..10).map(|idx| format!("INFO line {idx}")).collect();
    app.ui_state
        .hitboxes
        .register(Rect::new(0, 0, 40, 10), HitboxAction::ScrollLogs);

    app.handle_mouse_event(MouseEventKind::ScrollDown, 5, 5);
    assert_eq!(app.ui_state.logs.scroll, 3);
    app.handle_mouse_event(MouseEventKind::ScrollUp, 5, 5);
    assert_eq!(app.ui_state.logs.scroll, 0);
}

#[test]
fn log_events_are_unicode_safe_and_deduplicated() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Logs;
    let payload = format!("{} {}", "🇰🇷Korea".repeat(80), "failed");
    let event = || {
        DataEvent::Logs(Ok(vec![LogEntry {
            level: "error".into(),
            payload: payload.clone(),
        }]))
    };

    app.apply_data_event(event());
    app.apply_data_event(event());

    assert_eq!(app.logs.len(), 1);
    assert!(app.logs[0].starts_with("ERROR "));
    assert!(app.logs[0].ends_with("..."));
}

#[test]
fn log_page_uses_dedicated_search_state() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Logs;
    app.logs = vec!["INFO alpha".into(), "INFO beta".into()];
    app.ui_state.proxies.search_query = "missing".into();

    assert_eq!(app.visible_logs().len(), 2);

    app.toggle_log_search();
    app.push_log_search_char('b');

    let visible = app.visible_logs();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].as_str(), "INFO beta");
}

#[test]
fn escape_and_alt_navigation_do_not_leak_to_page_switching() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Logs;

    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_eq!(app.ui_state.active_page, Tab::Logs);
    assert!(!app.should_quit);

    crate::ui::controllers::key_router::handle_key_event(
        &mut app,
        KeyEvent::new(KeyCode::Left, KeyModifiers::ALT),
    );
    assert_eq!(app.ui_state.active_page, Tab::Logs);
}

#[test]
fn mode_display_normalizes_api_values() {
    assert_eq!(mode_display("rule"), "Rule");
    assert_eq!(mode_display("Global"), "Global");
    assert_eq!(mode_display("DIRECT"), "Direct");
    assert_eq!(mode_display("unexpected"), "Rule");
}

#[test]
fn next_mode_cycles_rule_global_direct() {
    assert_eq!(next_mode("Rule"), "Global");
    assert_eq!(next_mode("Global"), "Direct");
    assert_eq!(next_mode("Direct"), "Rule");
}

#[test]
fn log_level_filter_is_minimum_severity() {
    assert_eq!(log_line_rank("DEBUG noisy"), 0);
    assert_eq!(log_line_rank("INFO ready"), 1);
    assert_eq!(log_line_rank("WARNING slow"), 2);
    assert_eq!(log_line_rank("ERROR failed"), 3);

    assert!(LogLevelFilter::Info.matches_line("INFO ready"));
    assert!(!LogLevelFilter::Info.matches_line("DEBUG noisy"));
    assert!(LogLevelFilter::Warning.matches_line("ERROR failed"));
    assert!(!LogLevelFilter::Warning.matches_line("INFO ready"));
    assert!(LogLevelFilter::Error.matches_line("ERROR failed"));
    assert!(!LogLevelFilter::Error.matches_line("WARNING slow"));
}

#[test]
fn log_level_cycle_covers_all_filters() {
    assert_eq!(LogLevelFilter::Debug.next(), LogLevelFilter::Info);
    assert_eq!(LogLevelFilter::Info.next(), LogLevelFilter::Warning);
    assert_eq!(LogLevelFilter::Warning.next(), LogLevelFilter::Error);
    assert_eq!(LogLevelFilter::Error.next(), LogLevelFilter::Debug);
}

#[test]
fn node_picker_opens_on_current_node_and_wraps_selection() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxy_groups = vec![("Auto".into(), "B".into())];
    app.proxies
        .insert("Auto".into(), selector_proxy("B", vec!["A", "B", "C"]));

    app.open_node_picker();
    assert!(app.ui_state.proxies.node_picker_open);
    assert_eq!(app.ui_state.proxies.selected_node_idx, 1);

    app.select_node_down();
    assert_eq!(app.ui_state.proxies.selected_node_idx, 2);
    app.select_node_down();
    assert_eq!(app.ui_state.proxies.selected_node_idx, 0);
    app.select_node_up();
    assert_eq!(app.ui_state.proxies.selected_node_idx, 2);
}

#[test]
fn node_picker_stays_closed_when_selected_group_has_no_nodes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut app = test_app(&rt);
    app.ui_state.active_page = Tab::Proxies;
    app.proxy_groups = vec![("Direct".into(), String::new())];
    app.proxies
        .insert("Direct".into(), selector_proxy("", Vec::new()));

    app.open_node_picker();
    assert!(!app.ui_state.proxies.node_picker_open);
    assert_eq!(app.ui_state.proxies.selected_node_idx, 0);
    assert_eq!(
        app.status_msg.as_deref(),
        Some("selected group has no nodes")
    );
}
