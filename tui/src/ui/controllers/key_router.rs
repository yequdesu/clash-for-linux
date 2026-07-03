use crate::app::{App, SettingsPromptKind, SubscriptionEditField};
use crate::input::{AppKey, AppKeyCode as KeyCode, AppModifiers as KeyModifiers};
use crate::mouse::{NetworkAction, SettingsAction, TrafficAction};
use crate::ui::components::nav::Tab;
use crate::update;

pub(crate) fn handle_key_event(app: &mut App, key: AppKey) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if handle_modal_key(app, key, ctrl) {
        return;
    }
    if ctrl {
        handle_control_key(app, key.code);
        return;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        return;
    }
    if handle_search_key(app, key.code) {
        return;
    }

    handle_page_key(app, key);
}

pub(crate) fn handle_paste(app: &mut App, text: &str) {
    if !text_input_active(app) {
        return;
    }

    for c in text.chars().filter(|c| !c.is_control()).take(4096) {
        handle_key_event(app, AppKey::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
}

fn text_input_active(app: &App) -> bool {
    app.sudo_prompt_active()
        || app.command_palette_active()
        || app.ui_state.settings.prompt.is_some()
        || app.subscription_input_active()
        || (app.ui_state.active_page == Tab::Logs && app.ui_state.logs.search_active)
        || (app.ui_state.active_page == Tab::Proxies && app.ui_state.proxies.search_active)
}

fn handle_modal_key(app: &mut App, key: AppKey, ctrl: bool) -> bool {
    if app.sudo_prompt_active() {
        if !ctrl {
            match key.code {
                KeyCode::Esc => app.cancel_sudo_prompt(),
                KeyCode::Enter => app.submit_sudo_prompt(),
                KeyCode::Backspace => app.pop_sudo_prompt_char(),
                KeyCode::Char(c) => app.push_sudo_prompt_char(c),
                _ => {}
            }
        }
        return true;
    }

    if app.command_palette_active() {
        match key.code {
            KeyCode::Esc => app.close_command_palette(),
            KeyCode::Enter => app.submit_command_palette(),
            KeyCode::Down | KeyCode::Tab => app.command_palette_select_down(),
            KeyCode::Up | KeyCode::BackTab => app.command_palette_select_up(),
            KeyCode::Backspace => app.pop_command_palette_char(),
            KeyCode::Char(c) => app.push_command_palette_char(c),
            _ => {}
        }
        return true;
    }

    if app.ui_state.modals.pending_confirmation.is_some() {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                update::cancel_pending_action(app)
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                update::confirm_pending_action(app)
            }
            _ => {}
        }
        return true;
    }

    if app.ui_state.settings.prompt.is_some() {
        match key.code {
            KeyCode::Esc => app.cancel_settings_prompt(),
            KeyCode::Enter => app.submit_settings_prompt(),
            KeyCode::Tab | KeyCode::Down => app.next_settings_prompt_field(),
            KeyCode::BackTab | KeyCode::Up => app.prev_settings_prompt_field(),
            KeyCode::Backspace => app.pop_settings_prompt_char(),
            KeyCode::Char(c) => app.push_settings_prompt_char(c),
            _ => {}
        }
        return true;
    }

    if app.subscription_input_active() {
        match key.code {
            KeyCode::Esc => app.cancel_subscription_prompt(),
            KeyCode::Enter => app.submit_subscription_prompt(),
            KeyCode::Tab | KeyCode::Down => app.next_subscription_form_field(),
            KeyCode::BackTab | KeyCode::Up => app.prev_subscription_form_field(),
            KeyCode::Backspace => app.pop_subscription_prompt_char(),
            KeyCode::Char(c) => app.push_subscription_prompt_char(c),
            _ => {}
        }
        return true;
    }

    if app.ui_state.proxies.node_picker_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('O') => app.close_node_picker(),
            KeyCode::Enter => app.confirm_selected_node(),
            KeyCode::Down | KeyCode::Char('j') => app.select_node_down(),
            KeyCode::Up | KeyCode::Char('k') => app.select_node_up(),
            KeyCode::Char('d') => app.test_selected_delay(),
            KeyCode::Char('D') => app.test_all_delays(),
            _ => {}
        }
        return true;
    }

    if app.command_output_visible() && matches!(key.code, KeyCode::Esc) {
        app.close_command_output();
        return true;
    }

    false
}

fn handle_control_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('l') | KeyCode::Char('L') => app.toggle_language(),
        KeyCode::Char('p') | KeyCode::Char('P') => {
            if app.command_palette_active() {
                app.close_command_palette();
            } else {
                app.open_command_palette();
            }
        }
        KeyCode::Up => {
            app.window.move_by(0, -1);
        }
        KeyCode::Down => {
            app.window.move_by(0, 1);
        }
        KeyCode::Left => {
            app.window.move_by(-2, 0);
        }
        KeyCode::Right => {
            app.window.move_by(2, 0);
        }
        _ => {}
    }
}

fn handle_search_key(app: &mut App, code: KeyCode) -> bool {
    if app.ui_state.active_page == Tab::Logs && app.ui_state.logs.search_active {
        match code {
            KeyCode::Esc => app.close_log_search(),
            KeyCode::Backspace => app.pop_log_search_char(),
            KeyCode::Char(c) => app.push_log_search_char(c),
            _ => {}
        }
        return true;
    }

    if app.ui_state.active_page == Tab::Proxies && app.ui_state.proxies.search_active {
        match code {
            KeyCode::Esc => {
                app.ui_state.proxies.search_active = false;
                app.ui_state.proxies.search_query.clear();
                app.clamp_proxy_selection();
            }
            KeyCode::Backspace => {
                app.ui_state.proxies.search_query.pop();
                app.clamp_proxy_selection();
            }
            KeyCode::Char(c) => {
                app.ui_state.proxies.search_query.push(c);
                app.clamp_proxy_selection();
            }
            _ => {}
        }
        return true;
    }

    false
}

fn handle_page_key(app: &mut App, key: AppKey) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            app.status_msg = Some("press q to quit".into());
        }
        KeyCode::Tab | KeyCode::Char('\t') => app.next_tab(),
        KeyCode::Right if app.ui_state.active_page == Tab::Settings => app.next_settings_section(),
        KeyCode::Left if app.ui_state.active_page == Tab::Settings => app.prev_settings_section(),
        KeyCode::Right => app.next_tab(),
        KeyCode::Left => app.prev_tab(),
        KeyCode::Char('?') | KeyCode::Char('h') => {
            app.ui_state.active_page = Tab::Help;
            app.show_help = false;
        }
        KeyCode::Char('R') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_default_traffic_range();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            update::refresh_data(app);
            if app.ui_state.active_page == Tab::Traffic {
                app.refresh_traffic();
            }
        }
        KeyCode::Char('=') | KeyCode::Char('+') => {
            app.window.zoom_in();
        }
        KeyCode::Char('-') => {
            app.window.zoom_out();
        }
        KeyCode::Char('0') => {
            app.window.reset();
        }
        KeyCode::Char('/') if app.ui_state.active_page == Tab::Proxies => {
            app.ui_state.proxies.search_active = !app.ui_state.proxies.search_active
        }
        KeyCode::Char('/') if app.ui_state.active_page == Tab::Logs => app.toggle_log_search(),
        KeyCode::Down | KeyCode::Char('j') => app.select_down(),
        KeyCode::Up | KeyCode::Char('k') => app.select_up(),
        KeyCode::Char('g') => app.reset_selection(),
        KeyCode::Char('G') => app.select_last_visible_item(),
        KeyCode::Char('1') => {
            app.ui_state.active_page = Tab::Subscriptions;
            app.refresh_subscriptions();
        }
        KeyCode::Char('2') => {
            app.ui_state.active_page = Tab::Proxies;
        }
        KeyCode::Char('3') => {
            app.ui_state.active_page = Tab::Connections;
        }
        KeyCode::Char('4') => {
            app.ui_state.active_page = Tab::Traffic;
            app.refresh_traffic();
        }
        KeyCode::Char('5') => {
            app.ui_state.active_page = Tab::Network;
        }
        KeyCode::Char('6') => {
            app.ui_state.active_page = Tab::Logs;
            app.refresh_logs_if_visible();
        }
        KeyCode::Char('7') => {
            app.ui_state.active_page = Tab::Settings;
        }
        KeyCode::Char('8') => {
            app.ui_state.active_page = Tab::Help;
        }
        KeyCode::Enter => handle_enter_key(app),
        KeyCode::Char('[') if app.ui_state.active_page == Tab::Traffic => {
            app.prev_traffic_range();
        }
        KeyCode::Char(']') if app.ui_state.active_page == Tab::Traffic => {
            app.next_traffic_range();
        }
        KeyCode::Char('m') | KeyCode::Char('M') if app.ui_state.active_page == Tab::Traffic => {
            app.toggle_traffic_chart();
        }
        KeyCode::Char('y') | KeyCode::Char('Y') if app.ui_state.active_page == Tab::Traffic => {
            app.next_traffic_dimension();
        }
        KeyCode::Char('e') if app.ui_state.active_page == Tab::Traffic => {
            app.export_traffic_csv();
        }
        KeyCode::Char('s') if app.ui_state.active_page == Tab::Traffic => {
            app.run_traffic_action(TrafficAction::SampleOnce);
        }
        KeyCode::Char('b') if app.ui_state.active_page == Tab::Traffic => {
            app.run_traffic_action(TrafficAction::CollectorStart);
        }
        KeyCode::Char('x') if app.ui_state.active_page == Tab::Traffic => {
            app.run_traffic_action(TrafficAction::CollectorStop);
        }
        KeyCode::Char('n') if app.ui_state.active_page == Tab::Traffic => {
            app.run_traffic_action(TrafficAction::CollectorRestart);
        }
        KeyCode::Char('p') if app.ui_state.active_page == Tab::Traffic => {
            app.run_traffic_action(TrafficAction::PruneDefault);
        }
        KeyCode::Char('u') if app.ui_state.active_page == Tab::Subscriptions => {
            app.update_selected_subscription();
        }
        KeyCode::Char('a') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_add();
        }
        KeyCode::Char('I') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_import();
        }
        KeyCode::Char('X') if app.ui_state.active_page == Tab::Subscriptions => {
            app.remove_selected_subscription();
        }
        KeyCode::Char('L') if app.ui_state.active_page == Tab::Subscriptions => {
            app.show_subscription_log();
        }
        KeyCode::Char('e') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_profile_edit();
        }
        KeyCode::Char('i') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::Interval);
        }
        KeyCode::Char('U') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::Url);
        }
        KeyCode::Char('A') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::UserAgent);
        }
        KeyCode::Char('P') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
        }
        KeyCode::Char('M') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::ConvertMode);
        }
        KeyCode::Char('t') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::AddTag);
        }
        KeyCode::Char('T') if app.ui_state.active_page == Tab::Subscriptions => {
            app.begin_subscription_edit(SubscriptionEditField::RemoveTag);
        }
        KeyCode::Char('s') | KeyCode::Char('S') if app.ui_state.active_page == Tab::Proxies => {
            app.toggle_sort();
        }
        KeyCode::Char('x') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::Stop);
        }
        KeyCode::Char('n') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::Restart);
        }
        KeyCode::Char('v') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::Status);
        }
        KeyCode::Char('V') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::Doctor);
        }
        KeyCode::Char('c') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::ConfigDoctor);
        }
        KeyCode::Char('t') if app.ui_state.active_page == Tab::Network => {
            let action = if app.tun_enabled {
                NetworkAction::TunOff
            } else {
                NetworkAction::TunOn
            };
            app.run_network_action(action);
        }
        KeyCode::Char('e') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::Env);
        }
        KeyCode::Char('p') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::ProxyOn);
        }
        KeyCode::Char('P') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::ProxyOff);
        }
        KeyCode::Char('d') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::DesktopStatus);
        }
        KeyCode::Char('D') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::DesktopOn);
        }
        KeyCode::Char('O') if app.ui_state.active_page == Tab::Network => {
            app.run_network_action(NetworkAction::DesktopOff);
        }
        KeyCode::Char('d') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::Doctor);
        }
        KeyCode::Char('D') if app.ui_state.active_page == Tab::Settings => {
            app.run_traffic_action(TrafficAction::Reset);
        }
        KeyCode::Char('c') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ConfigDoctor);
        }
        KeyCode::Char('B') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_default_page();
        }
        KeyCode::Char('E') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_theme_preference();
        }
        KeyCode::Char('F') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_refresh_interval();
        }
        KeyCode::Char('M') if app.ui_state.active_page == Tab::Settings => {
            app.toggle_mouse_preference();
        }
        KeyCode::Char('C') if app.ui_state.active_page == Tab::Settings => {
            app.toggle_dangerous_confirmations();
        }
        KeyCode::Char('T') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::TrafficPruneRetention);
        }
        KeyCode::Char('H') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_default_traffic_chart();
        }
        KeyCode::Char('y') if app.ui_state.active_page == Tab::Settings => {
            app.cycle_default_traffic_dimension();
        }
        KeyCode::Char('V') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ConfigView);
        }
        KeyCode::Char('W') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ConfigRaw);
        }
        KeyCode::Char('v') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::Version);
        }
        KeyCode::Char('t') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ProxyTest);
        }
        KeyCode::Char('m') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ConfigMerge);
        }
        KeyCode::Char('a') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ConfigMergeAutofix);
        }
        KeyCode::Char('s') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::SecretStatus);
        }
        KeyCode::Char('S') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::SecretReveal);
        }
        KeyCode::Char('N') if app.ui_state.active_page == Tab::Settings => {
            app.begin_secret_set();
        }
        KeyCode::Char('P') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
        }
        KeyCode::Char('A') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
        }
        KeyCode::Char('Y') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
        }
        KeyCode::Char('L') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::ConfigSetLan);
        }
        KeyCode::Char('z') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::GeodataUpdate);
        }
        KeyCode::Char('Z') if app.ui_state.active_page == Tab::Settings => {
            app.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
        }
        KeyCode::Char('u') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::ApiUpgrade);
        }
        KeyCode::Char('K') if app.ui_state.active_page == Tab::Settings => {
            app.run_settings_action(SettingsAction::KernelUpgrade);
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            if app.ui_state.active_page == Tab::Proxies {
                if app.ui_state.proxies.node_picker_open {
                    app.close_node_picker();
                } else {
                    app.open_node_picker();
                }
            }
        }
        KeyCode::Char('d') => {
            if app.ui_state.active_page == Tab::Proxies {
                app.test_selected_delay();
            }
        }
        KeyCode::Char('D') => {
            if app.ui_state.active_page == Tab::Proxies {
                app.test_all_delays();
            }
        }
        KeyCode::Char('p') => {
            if app.ui_state.active_page != Tab::Logs {
                app.cycle_proxy_mode();
            } else {
                app.toggle_log_pause();
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            if app.ui_state.active_page == Tab::Logs {
                app.cycle_log_level();
            }
        }
        KeyCode::Char('c') => {
            if app.ui_state.active_page == Tab::Connections {
                app.close_selected_connection();
            } else if app.ui_state.active_page == Tab::Logs {
                app.clear_logs();
            }
        }
        KeyCode::Char('C') if app.ui_state.active_page == Tab::Connections => {
            app.close_all_connections();
        }
        _ => {}
    }
}

fn handle_enter_key(app: &mut App) {
    match app.ui_state.active_page {
        Tab::Proxies => {
            if app.ui_state.proxies.node_picker_open {
                app.confirm_selected_node();
            } else {
                app.test_selected_delay();
            }
        }
        Tab::Subscriptions => {
            app.use_selected_subscription();
        }
        Tab::Traffic => {
            app.toggle_traffic_filter();
        }
        Tab::Network => {
            app.run_network_action(NetworkAction::Start);
        }
        _ => {}
    }
}
