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

    update::refresh_data(&mut app);

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
            update::apply_data_event(app, data_event);
        }

        if app.force_terminal_clear {
            terminal.clear()?;
            app.force_terminal_clear = false;
        }
        terminal.draw(|frame| ui::app_shell::render(frame, app))?;

        match handler.next() {
            Ok(Event::Key(key)) => ui::controllers::key_router::handle_key_event(app, key),
            Ok(Event::Mouse(mouse)) => app.handle_mouse_event(mouse.kind, mouse.column, mouse.row),
            Ok(Event::Init) => {}
            Ok(Event::Tick) => update::on_tick(app),
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
