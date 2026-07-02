mod action_registry;
mod api;
mod app;
mod background;
mod config;
mod event;
mod i18n;
mod mouse;
mod settings;
mod theme;
mod widgets;
mod window;

use crossterm::cursor;
use crossterm::event::{self as crossterm_event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::panic;
use std::sync::mpsc;

use app::{App, SettingsPromptKind, SubscriptionEditField};
use event::{DataEvent, Event, EventHandler};
use mouse::{NetworkAction, SettingsAction, TrafficAction};
use widgets::tab_bar::Tab;

fn main() -> io::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let (data_tx, data_rx) = mpsc::channel::<DataEvent>();

    let config = config::Config::load();
    let mut app = App::new(config, rt.handle().clone(), data_tx);
    let mouse_capture_enabled = app.ui_settings.mouse_enabled;

    let _guard = panic_handler();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(cursor::Hide)?;
    if mouse_capture_enabled {
        stdout.execute(crossterm_event::EnableMouseCapture)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_handler = EventHandler::new(50, data_rx);

    app.refresh_data();

    let result = run(&mut terminal, &mut app, &event_handler);

    app.on_shutdown();
    terminal::disable_raw_mode()?;
    if mouse_capture_enabled {
        terminal
            .backend_mut()
            .execute(crossterm_event::DisableMouseCapture)?;
    }
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.backend_mut().execute(cursor::Show)?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    handler: &EventHandler,
) -> io::Result<()> {
    loop {
        while let Some(data_event) = handler.try_recv_data() {
            app.apply_data_event(data_event);
        }

        terminal.draw(|frame| app::render(frame, app))?;

        match handler.next() {
            Ok(Event::Key(key)) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                if ctrl {
                    match key.code {
                        KeyCode::Char('l') | KeyCode::Char('L') => app.toggle_language(),
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            if app.command_palette_active() {
                                app.close_command_palette();
                            } else {
                                app.open_command_palette();
                            }
                        }
                        KeyCode::Up => app.window.move_by(0, -1),
                        KeyCode::Down => app.window.move_by(0, 1),
                        KeyCode::Left => app.window.move_by(-2, 0),
                        KeyCode::Right => app.window.move_by(2, 0),
                        _ => {}
                    }
                    continue;
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
                    continue;
                }

                if app.pending_confirmation.is_some() {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                            app.cancel_pending_action()
                        }
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_pending_action()
                        }
                        _ => {}
                    }
                    continue;
                }

                if app.settings_prompt.is_some() {
                    match key.code {
                        KeyCode::Esc => app.cancel_settings_prompt(),
                        KeyCode::Enter => app.submit_settings_prompt(),
                        KeyCode::Tab | KeyCode::Down => app.next_settings_prompt_field(),
                        KeyCode::BackTab | KeyCode::Up => app.prev_settings_prompt_field(),
                        KeyCode::Backspace => app.pop_settings_prompt_char(),
                        KeyCode::Char(c) => app.push_settings_prompt_char(c),
                        _ => {}
                    }
                    continue;
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
                    continue;
                }

                if app.search_active {
                    match key.code {
                        KeyCode::Esc => {
                            app.search_active = false;
                            app.search_query.clear();
                            app.clamp_proxy_selection();
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.clamp_proxy_selection();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.clamp_proxy_selection();
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Esc => {
                        if app.node_picker_open {
                            app.close_node_picker();
                        } else {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Tab | KeyCode::Char('\t') | KeyCode::Right => app.next_tab(),
                    KeyCode::Left => app.prev_tab(),
                    KeyCode::Char('?') | KeyCode::Char('h') => {
                        app.tab = Tab::Help;
                        app.show_help = false;
                    }
                    KeyCode::Char('R') if app.tab == Tab::Settings => {
                        app.cycle_default_traffic_range();
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        app.refresh_data();
                        if app.tab == Tab::Traffic {
                            app.refresh_traffic();
                        }
                    }
                    KeyCode::Char('=') | KeyCode::Char('+') => app.window.zoom_in(),
                    KeyCode::Char('-') => app.window.zoom_out(),
                    KeyCode::Char('0') => app.window.reset(),
                    KeyCode::Char('/') => app.search_active = !app.search_active,
                    KeyCode::Down | KeyCode::Char('j') => app.select_down(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_up(),
                    KeyCode::Char('g') => {
                        app.selected_proxy_idx = 0;
                        app.connections_selected = 0;
                        app.selected_sub_idx = 0;
                        app.traffic_selected_idx = 0;
                    }
                    KeyCode::Char('G') => {
                        app.selected_proxy_idx = app.proxy_groups.len().saturating_sub(1);
                        app.connections_selected = app.connections.len().saturating_sub(1);
                        app.selected_sub_idx = app.profiles.len().saturating_sub(1);
                        app.traffic_selected_idx = app.traffic_top.len().saturating_sub(1);
                    }
                    KeyCode::Char('1') => {
                        app.tab = Tab::Subscriptions;
                        app.refresh_subscriptions();
                    }
                    KeyCode::Char('2') => {
                        app.tab = Tab::Proxies;
                    }
                    KeyCode::Char('3') => {
                        app.tab = Tab::Connections;
                    }
                    KeyCode::Char('4') => {
                        app.tab = Tab::Traffic;
                        app.refresh_traffic();
                    }
                    KeyCode::Char('5') => {
                        app.tab = Tab::Network;
                    }
                    KeyCode::Char('6') => {
                        app.tab = Tab::Logs;
                    }
                    KeyCode::Char('7') => {
                        app.tab = Tab::Settings;
                    }
                    KeyCode::Char('8') => {
                        app.tab = Tab::Help;
                    }
                    KeyCode::Enter => match app.tab {
                        Tab::Proxies => {
                            if app.node_picker_open {
                                app.switch_selected_node();
                            } else if key.modifiers.contains(KeyModifiers::ALT) {
                                app.switch_selected();
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
                    },
                    KeyCode::Char('[') if app.tab == Tab::Traffic => {
                        app.prev_traffic_range();
                    }
                    KeyCode::Char(']') if app.tab == Tab::Traffic => {
                        app.next_traffic_range();
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') if app.tab == Tab::Traffic => {
                        app.toggle_traffic_chart();
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') if app.tab == Tab::Traffic => {
                        app.next_traffic_dimension();
                    }
                    KeyCode::Char('e') if app.tab == Tab::Traffic => {
                        app.export_traffic_csv();
                    }
                    KeyCode::Char('s') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::SampleOnce);
                    }
                    KeyCode::Char('b') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::CollectorStart);
                    }
                    KeyCode::Char('x') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::CollectorStop);
                    }
                    KeyCode::Char('n') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::CollectorRestart);
                    }
                    KeyCode::Char('p') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::PruneDefault);
                    }
                    KeyCode::Char('D') if app.tab == Tab::Traffic => {
                        app.run_traffic_action(TrafficAction::Reset);
                    }
                    KeyCode::Char('u') if app.tab == Tab::Subscriptions => {
                        app.update_selected_subscription();
                    }
                    KeyCode::Char('a') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_add();
                    }
                    KeyCode::Char('I') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_import();
                    }
                    KeyCode::Char('X') if app.tab == Tab::Subscriptions => {
                        app.remove_selected_subscription();
                    }
                    KeyCode::Char('L') if app.tab == Tab::Subscriptions => {
                        app.show_subscription_log();
                    }
                    KeyCode::Char('e') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_profile_edit();
                    }
                    KeyCode::Char('i') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::Interval);
                    }
                    KeyCode::Char('U') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::Url);
                    }
                    KeyCode::Char('A') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::UserAgent);
                    }
                    KeyCode::Char('P') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::UpdateProxy);
                    }
                    KeyCode::Char('M') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::ConvertMode);
                    }
                    KeyCode::Char('t') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::AddTag);
                    }
                    KeyCode::Char('T') if app.tab == Tab::Subscriptions => {
                        app.begin_subscription_edit(SubscriptionEditField::RemoveTag);
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') if app.tab == Tab::Proxies => {
                        app.toggle_sort();
                    }
                    KeyCode::Char('x') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::Stop);
                    }
                    KeyCode::Char('n') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::Restart);
                    }
                    KeyCode::Char('v') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::Status);
                    }
                    KeyCode::Char('V') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::Doctor);
                    }
                    KeyCode::Char('c') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::ConfigDoctor);
                    }
                    KeyCode::Char('t') if app.tab == Tab::Network => {
                        let action = if app.tun_enabled {
                            NetworkAction::TunOff
                        } else {
                            NetworkAction::TunOn
                        };
                        app.run_network_action(action);
                    }
                    KeyCode::Char('e') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::Env);
                    }
                    KeyCode::Char('p') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::ProxyOn);
                    }
                    KeyCode::Char('P') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::ProxyOff);
                    }
                    KeyCode::Char('d') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::DesktopStatus);
                    }
                    KeyCode::Char('D') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::DesktopOn);
                    }
                    KeyCode::Char('O') if app.tab == Tab::Network => {
                        app.run_network_action(NetworkAction::DesktopOff);
                    }
                    KeyCode::Char('d') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::Doctor);
                    }
                    KeyCode::Char('c') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ConfigDoctor);
                    }
                    KeyCode::Char('B') if app.tab == Tab::Settings => {
                        app.cycle_default_page();
                    }
                    KeyCode::Char('E') if app.tab == Tab::Settings => {
                        app.cycle_theme_preference();
                    }
                    KeyCode::Char('F') if app.tab == Tab::Settings => {
                        app.cycle_refresh_interval();
                    }
                    KeyCode::Char('M') if app.tab == Tab::Settings => {
                        app.toggle_mouse_preference();
                    }
                    KeyCode::Char('C') if app.tab == Tab::Settings => {
                        app.toggle_dangerous_confirmations();
                    }
                    KeyCode::Char('T') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::TrafficPruneRetention);
                    }
                    KeyCode::Char('H') if app.tab == Tab::Settings => {
                        app.cycle_default_traffic_chart();
                    }
                    KeyCode::Char('y') if app.tab == Tab::Settings => {
                        app.cycle_default_traffic_dimension();
                    }
                    KeyCode::Char('V') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ConfigView);
                    }
                    KeyCode::Char('W') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ConfigRaw);
                    }
                    KeyCode::Char('v') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::Version);
                    }
                    KeyCode::Char('t') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ProxyTest);
                    }
                    KeyCode::Char('m') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ConfigMerge);
                    }
                    KeyCode::Char('a') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ConfigMergeAutofix);
                    }
                    KeyCode::Char('s') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::SecretStatus);
                    }
                    KeyCode::Char('S') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::SecretReveal);
                    }
                    KeyCode::Char('N') if app.tab == Tab::Settings => {
                        app.begin_secret_set();
                    }
                    KeyCode::Char('P') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::ConfigSetPorts);
                    }
                    KeyCode::Char('A') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::ConfigSetApi);
                    }
                    KeyCode::Char('Y') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::ConfigSetDnsMode);
                    }
                    KeyCode::Char('L') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::ConfigSetLan);
                    }
                    KeyCode::Char('z') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::GeodataUpdate);
                    }
                    KeyCode::Char('Z') if app.tab == Tab::Settings => {
                        app.begin_settings_prompt(SettingsPromptKind::GeodataUpdateVersion);
                    }
                    KeyCode::Char('u') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::ApiUpgrade);
                    }
                    KeyCode::Char('K') if app.tab == Tab::Settings => {
                        app.run_settings_action(SettingsAction::KernelUpgrade);
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        if app.tab == Tab::Proxies {
                            if app.node_picker_open {
                                app.close_node_picker();
                            } else {
                                app.open_node_picker();
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if app.tab == Tab::Proxies {
                            app.test_selected_delay();
                        }
                    }
                    KeyCode::Char('D') => {
                        if app.tab == Tab::Proxies {
                            app.test_all_delays();
                        }
                    }
                    KeyCode::Char('p') => {
                        if app.tab != Tab::Logs {
                            app.cycle_proxy_mode();
                        } else {
                            app.toggle_log_pause();
                        }
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        if app.tab == Tab::Logs {
                            app.cycle_log_level();
                        }
                    }
                    KeyCode::Char('c') => {
                        if app.tab == Tab::Connections {
                            app.close_selected_connection();
                        } else if app.tab == Tab::Logs {
                            app.clear_logs();
                        }
                    }
                    KeyCode::Char('C') if app.tab == Tab::Connections => {
                        app.close_all_connections();
                    }
                    _ => {}
                }
            }
            Ok(Event::Mouse(mouse)) => app.handle_mouse_event(mouse.kind, mouse.column, mouse.row),
            Ok(Event::Tick) => app.on_tick(),
            Ok(_) => {}
            Err(e) => app.error_msg = Some(e.to_string()),
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn panic_handler() -> impl Drop {
    struct PanicGuard;
    impl Drop for PanicGuard {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
            let _ = io::stdout().execute(crossterm_event::DisableMouseCapture);
            let _ = io::stdout().execute(LeaveAlternateScreen);
            let _ = io::stdout().execute(cursor::Show);
        }
    }
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = io::stdout().execute(crossterm_event::DisableMouseCapture);
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = io::stdout().execute(cursor::Show);
        prev(info);
    }));
    PanicGuard
}
