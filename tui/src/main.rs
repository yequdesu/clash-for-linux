mod action_registry;
mod api;
mod app;
mod background;
mod config;
mod event;
mod i18n;
mod input;
mod mouse;
mod settings;
mod theme;
mod ui;
mod update;
mod window;

use crossterm::cursor;
use crossterm::event::{self as crossterm_event};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::panic;
use std::sync::mpsc;

use app::App;
use event::{DataEvent, Event, EventHandler};
use input::AppInput;

fn main() -> io::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let (data_tx, data_rx) = mpsc::channel::<DataEvent>();

    let config = config::Config::load();
    let mut app = App::new(config, rt.handle().clone(), data_tx);
    let mut mouse_capture_enabled = app.ui_settings.mouse_enabled;

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

    let mut event_handler = EventHandler::new(50, data_rx);

    update::refresh_data(&mut app);

    let result = run(
        &mut terminal,
        &mut app,
        &mut event_handler,
        &mut mouse_capture_enabled,
    );

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
    handler: &mut EventHandler,
    mouse_capture_enabled: &mut bool,
) -> io::Result<()> {
    loop {
        while let Some(data_event) = handler.try_recv_data() {
            update::apply_data_event(app, data_event);
        }

        if app.force_terminal_clear {
            terminal.clear()?;
            app.force_terminal_clear = false;
        }
        terminal.draw(|frame| ui::app_shell::render(frame, app))?;

        match handler.next() {
            Ok(Event::Input(AppInput::Key(key))) => {
                ui::controllers::key_router::handle_key_event(app, key)
            }
            Ok(Event::Input(AppInput::Mouse(mouse))) => app.handle_mouse_event(mouse),
            Ok(Event::Input(AppInput::Resize { .. })) => {}
            Ok(Event::Input(AppInput::Paste(text))) => {
                ui::controllers::key_router::handle_paste(app, &text)
            }
            Ok(Event::Init) => {}
            Ok(Event::Tick) => update::on_tick(app),
            Err(e) => app.error_msg = Some(e.to_string()),
        }
        sync_mouse_capture(terminal, app, mouse_capture_enabled)?;

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn sync_mouse_capture(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mouse_capture_enabled: &mut bool,
) -> io::Result<()> {
    if app.ui_settings.mouse_enabled == *mouse_capture_enabled {
        return Ok(());
    }

    if app.ui_settings.mouse_enabled {
        terminal
            .backend_mut()
            .execute(crossterm_event::EnableMouseCapture)?;
        *mouse_capture_enabled = true;
        app.status_msg = Some("mouse capture enabled".into());
    } else {
        terminal
            .backend_mut()
            .execute(crossterm_event::DisableMouseCapture)?;
        *mouse_capture_enabled = false;
        app.status_msg = Some("mouse capture disabled; press 7 then M to enable".into());
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
