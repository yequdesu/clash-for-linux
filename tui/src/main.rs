mod api;
mod app;
mod background;
mod config;
mod event;
mod theme;
mod widgets;
mod window;

use crossterm::cursor;
use crossterm::event::{self as crossterm_event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::panic;
use std::sync::mpsc;

use app::App;
use event::{DataEvent, Event, EventHandler};
use widgets::tab_bar::Tab;

fn handle_proxy_click(app: &mut App, col: u16, row: u16) {
    // Click on mode bar → switch mode by column
    if row == app.proxy_mode_y {
        let rel = col.saturating_sub(app.proxy_content_x);
        // " Mode: [Rule] Global Direct  |  p: switch mode"
        if rel >= 7 && rel <= 13 {
            set_proxy_mode(app, "rule");
        } else if rel >= 16 && rel <= 21 {
            set_proxy_mode(app, "global");
        } else if rel >= 23 && rel <= 28 {
            set_proxy_mode(app, "direct");
        }
        return;
    }

    // Click in content area → find proxy at position
    if row < app.proxy_content_y || row >= app.proxy_content_y + app.proxy_content_h {
        return;
    }
    let abs_line = row as usize - app.proxy_content_y as usize + app.proxy_scroll_offset;

    if abs_line < 2 {
        return; // header or divider
    }

    let mut line = abs_line - 2;
    for gi in 0..app.proxy_groups.len() {
        let n = app.proxy_groups[gi].proxies.len();
        let group_lines = 1 + n + 1; // header + proxies + border

        if line < group_lines {
            if line >= 1 && line <= n {
                let pi = line - 1;
                app.proxy_group_selected = gi;
                app.proxy_selected = pi;
            }
            break;
        }
        line -= group_lines;
    }
}

fn set_proxy_mode(app: &App, mode: &str) {
    let api = app.api.clone();
    let tx = app.data_tx.clone();
    let m = mode.to_string();
    let _ = app.rt.spawn(async move {
        let result = api.set_mode(&m).await;
        let _ = tx.send(DataEvent::ModeSet(result));
    });
}

fn main() -> io::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let (data_tx, data_rx) = mpsc::channel::<DataEvent>();

    let config = config::Config::load();
    let mut app = App::new(config, rt.handle().clone(), data_tx);

    let _guard = panic_handler();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(cursor::Hide)?;
    stdout.execute(crossterm_event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_handler = EventHandler::new(50, data_rx);

    app.refresh_data();

    let result = run(&mut terminal, &mut app, &event_handler);

    app.on_shutdown();
    terminal::disable_raw_mode()?;
    terminal.backend_mut().execute(crossterm_event::DisableMouseCapture)?;
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

                // Ctrl+Arrows for window management
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                if ctrl {
                    match key.code {
                        KeyCode::Up => app.window.move_by(0, -1),
                        KeyCode::Down => app.window.move_by(0, 1),
                        KeyCode::Left => app.window.move_by(-2, 0),
                        KeyCode::Right => app.window.move_by(2, 0),
                        _ => {}
                    }
                    continue;
                }

                // Confirmation mode
                if app.confirm_action {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_action = false;
                            app.confirm_timer = 0;
                            // Action is handled by the calling context
                        }
                        _ => {
                            app.confirm_action = false;
                            app.confirm_timer = 0;
                        }
                    }
                    continue;
                }

                // Global keys
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::Tab | KeyCode::Right => app.next_tab(),
                    KeyCode::Left => app.prev_tab(),
                    KeyCode::Char('?') => app.show_help = !app.show_help,
                    KeyCode::Char('r') | KeyCode::Char('R') => app.refresh_data(),
                    KeyCode::Char('=') | KeyCode::Char('+') => app.window.zoom_in(),
                    KeyCode::Char('-') => app.window.zoom_out(),
                    KeyCode::Char('0') => app.window.reset(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_down(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_up(),

                    // Tab-specific g/G (Proxies: top/bottom; others: scroll)
                    KeyCode::Char('g') if app.tab == Tab::Proxies => {
                        app.proxy_group_selected = 0;
                        app.proxy_selected = 0;
                        app.proxy_scroll_offset = 0;
                    }
                    KeyCode::Char('G') if app.tab == Tab::Proxies => {
                        app.proxy_group_selected = app.proxy_groups.len().saturating_sub(1);
                        if !app.proxy_groups.is_empty() {
                            let g = &app.proxy_groups[app.proxy_group_selected];
                            app.proxy_selected = g.proxies.len().saturating_sub(1);
                        }
                        app.ensure_proxy_visible();
                    }
                    KeyCode::Char('g') => {
                        match app.tab {
                            Tab::Logs => app.log_scroll = 0,
                            Tab::Connections => app.connection_selected = 0,
                            Tab::Subscriptions => app.sub_selected = 0,
                            _ => {}
                        }
                    }
                    KeyCode::Char('G') => {
                        match app.tab {
                            Tab::Logs => app.log_scroll = app.logs.len().saturating_sub(1),
                            Tab::Connections => app.connection_selected = app.connections.len().saturating_sub(1),
                            Tab::Subscriptions => app.sub_selected = app.subscriptions.len().saturating_sub(1),
                            _ => {}
                        }
                    }
                    KeyCode::Char('1') => app.tab = Tab::Overview,
                    KeyCode::Char('2') => app.tab = Tab::Proxies,
                    KeyCode::Char('3') => app.tab = Tab::Subscriptions,
                    KeyCode::Char('4') => app.tab = Tab::Connections,
                    KeyCode::Char('5') => app.tab = Tab::Logs,

                    // ===== Tab-specific keys =====
                    // Proxies Tab
                    KeyCode::Char('d') if app.tab == Tab::Proxies => {
                        test_current_group(app);
                    }
                    KeyCode::Char('D') if app.tab == Tab::Proxies => {
                        test_all_delays(app);
                    }
                    KeyCode::Char('p') if app.tab == Tab::Proxies || app.tab == Tab::Overview => {
                        cycle_proxy_mode(app);
                    }
                    KeyCode::Enter if app.tab == Tab::Proxies => {
                        switch_current_proxy(app);
                    }

                    // Subscriptions Tab
                    KeyCode::Char('a') if app.tab == Tab::Subscriptions => {
                        app.error_msg = Some("Use CLI: clashctl sub add <url>".to_string());
                    }
                    KeyCode::Char('u') if app.tab == Tab::Subscriptions => {
                        app.error_msg = Some("Use CLI: clashctl sub update".to_string());
                    }
                    KeyCode::Char('U') if app.tab == Tab::Subscriptions => {
                        app.error_msg = Some("Use CLI: clashctl sub update".to_string());
                    }
                    KeyCode::Enter if app.tab == Tab::Subscriptions => {
                        if !app.subscriptions.is_empty() {
                            let s = &app.subscriptions[app.sub_selected.min(app.subscriptions.len() - 1)];
                            app.error_msg = Some(format!("Use CLI: clashctl sub use {}", s.id));
                        }
                    }
                    KeyCode::Char('d') if app.tab == Tab::Subscriptions => {
                        if !app.subscriptions.is_empty() {
                            let s = &app.subscriptions[app.sub_selected.min(app.subscriptions.len() - 1)];
                            app.error_msg = Some(format!("Use CLI: clashctl sub remove {}", s.id));
                        }
                    }

                    // Connections Tab
                    KeyCode::Char('c') if app.tab == Tab::Connections => {
                        close_selected_connection(app);
                    }
                    KeyCode::Char('C') if app.tab == Tab::Connections => {
                        close_all_connections_handler(app);
                    }

                    // Logs Tab
                    KeyCode::Char('f') if app.tab == Tab::Logs => {
                        // Cycle log level filter
                        app.log_level_filter = match app.log_level_filter.as_str() {
                            "ALL" => "INFO".into(),
                            "INFO" => "WARN".into(),
                            "WARN" => "ERROR".into(),
                            "ERROR" => "DEBUG".into(),
                            "DEBUG" => "ALL".into(),
                            _ => "ALL".into(),
                        };
                    }
                    KeyCode::Char('p') if app.tab == Tab::Logs => {
                        app.log_paused = !app.log_paused;
                    }

                    _ => {}
                }
            }
            Ok(Event::Mouse(mouse)) => {
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        match app.tab {
                            Tab::Proxies => {
                                app.proxy_scroll_offset = app.proxy_scroll_offset.saturating_add(3);
                            }
                            Tab::Logs => {
                                app.log_scroll = app.log_scroll.saturating_add(3);
                            }
                            Tab::Connections => {
                                app.connection_selected = (app.connection_selected + 1)
                                    .min(app.connections.len().saturating_sub(1));
                            }
                            Tab::Subscriptions => {
                                app.sub_selected = (app.sub_selected + 1)
                                    .min(app.subscriptions.len().saturating_sub(1));
                            }
                            _ => {}
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        match app.tab {
                            Tab::Proxies => {
                                app.proxy_scroll_offset = app.proxy_scroll_offset.saturating_sub(3);
                            }
                            Tab::Logs => {
                                app.log_scroll = app.log_scroll.saturating_sub(3);
                            }
                            Tab::Connections => {
                                app.connection_selected = app.connection_selected.saturating_sub(1);
                            }
                            Tab::Subscriptions => {
                                app.sub_selected = app.sub_selected.saturating_sub(1);
                            }
                            _ => {}
                        }
                    }
                    MouseEventKind::Down(crossterm_event::MouseButton::Left) => {
                        match app.tab {
                            Tab::Proxies => handle_proxy_click(app, mouse.column, mouse.row),
                            Tab::Connections => {
                                let idx = app.connection_selected;
                                if idx < app.connections.len() {
                                    let id = app.connections[idx].id.clone();
                                    let api = app.api.clone();
                                    let tx = app.data_tx.clone();
                                    let _ = app.rt.spawn(async move {
                                        let result = api.close_connection(&id).await;
                                        let _ = tx.send(DataEvent::ConnectionClosed(result));
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Tick) => app.on_tick(),
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

// Helper methods that need access to App state + API

fn test_current_group(app: &App) {
    if app.proxy_groups.is_empty() {
        return;
    }
    let g = &app.proxy_groups[app.proxy_group_selected.min(app.proxy_groups.len() - 1)];
    for proxy in &g.proxies {
        let api = app.api.clone();
        let tx = app.data_tx.clone();
        let name = proxy.name.clone();
        let _ = app.rt.spawn(async move {
            let result = api.test_delay(&name, "http://www.gstatic.com/generate_204", 5000).await;
            let _ = tx.send(DataEvent::DelayTested(name, result));
        });
    }
}

fn test_all_delays(app: &App) {
    for group in &app.proxy_groups {
        for proxy in &group.proxies {
            let api = app.api.clone();
            let tx = app.data_tx.clone();
            let name = proxy.name.clone();
            let _ = app.rt.spawn(async move {
                let result = api.test_delay(&name, "http://www.gstatic.com/generate_204", 5000).await;
                let _ = tx.send(DataEvent::DelayTested(name, result));
            });
        }
    }
}

fn cycle_proxy_mode(app: &App) {
    let next_mode = match app.kernel_mode.as_str() {
        "rule" => "global",
        "global" => "direct",
        "direct" => "rule",
        _ => "rule",
    };

    let api = app.api.clone();
    let tx = app.data_tx.clone();
    let mode = next_mode.to_string();
    let _ = app.rt.spawn(async move {
        let result = api.set_mode(&mode).await;
        let _ = tx.send(DataEvent::ModeSet(result));
    });
}

fn switch_current_proxy(app: &App) {
    if app.proxy_groups.is_empty() {
        return;
    }
    let gi = app.proxy_group_selected.min(app.proxy_groups.len() - 1);
    let group = &app.proxy_groups[gi];
    if group.proxies.is_empty() {
        return;
    }
    let pi = app.proxy_selected.min(group.proxies.len().saturating_sub(1));
    let proxy = &group.proxies[pi];

    let api = app.api.clone();
    let tx = app.data_tx.clone();
    let group_name = group.name.clone();
    let proxy_name = proxy.name.clone();
    let _ = app.rt.spawn(async move {
        let result = api.switch_proxy(&group_name, &proxy_name).await;
        let _ = tx.send(DataEvent::ProxySwitched(result));
    });
}

fn close_selected_connection(app: &App) {
    if app.connections.is_empty() {
        return;
    }
    let ci = app.connection_selected.min(app.connections.len() - 1);
    let conn = &app.connections[ci];
    let id = conn.id.clone();

    let api = app.api.clone();
    let tx = app.data_tx.clone();
    let _ = app.rt.spawn(async move {
        let result = api.close_connection(&id).await;
        let _ = tx.send(DataEvent::ConnectionClosed(result));
    });
}

fn close_all_connections_handler(app: &App) {
    let api = app.api.clone();
    let tx = app.data_tx.clone();
    let _ = app.rt.spawn(async move {
        let result = api.close_all_connections().await;
        let _ = tx.send(DataEvent::ConnectionClosed(result));
    });
}
