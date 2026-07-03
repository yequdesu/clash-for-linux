use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};

use super::queue::InputQueue;
use super::types::{AppInput, AppKey, AppKeyCode, AppModifiers, AppMouse, AppMouseKind};

const ESC_SEQUENCE_FRAGMENT_LIMIT: u8 = 12;

#[derive(Debug, Default)]
pub(crate) struct InputNormalizer {
    esc_fragments_remaining: u8,
}

impl InputNormalizer {
    pub(crate) fn push_crossterm(&mut self, event: CrosstermEvent, queue: &mut InputQueue) {
        match event {
            CrosstermEvent::Key(key) => {
                if let Some(key) = self.normalize_key(key) {
                    queue.push(AppInput::Key(key));
                }
            }
            CrosstermEvent::Mouse(mouse) => {
                queue.push(AppInput::Mouse(AppMouse::new(
                    normalize_mouse_kind(mouse.kind),
                    mouse.column,
                    mouse.row,
                )));
            }
            CrosstermEvent::Resize(cols, rows) => {
                self.esc_fragments_remaining = 0;
                queue.push(AppInput::Resize { cols, rows });
            }
            CrosstermEvent::Paste(text) => queue.push(AppInput::Paste(text)),
            _ => {}
        }
    }

    pub(crate) fn on_idle(&mut self) {
        self.esc_fragments_remaining = 0;
    }

    fn normalize_key(&mut self, key: KeyEvent) -> Option<AppKey> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        let key = AppKey::new(
            normalize_key_code(key.code),
            normalize_modifiers(key.modifiers),
        );
        if self.drop_escape_fragment(key) {
            return None;
        }
        Some(key)
    }

    fn drop_escape_fragment(&mut self, key: AppKey) -> bool {
        if key.code == AppKeyCode::Esc && key.modifiers == AppModifiers::NONE {
            self.esc_fragments_remaining = ESC_SEQUENCE_FRAGMENT_LIMIT;
            return false;
        }

        if self.esc_fragments_remaining == 0 {
            return false;
        }

        if is_escape_fragment(key) {
            self.esc_fragments_remaining = self.esc_fragments_remaining.saturating_sub(1);
            return true;
        }

        self.esc_fragments_remaining = 0;
        false
    }
}

fn normalize_key_code(code: KeyCode) -> AppKeyCode {
    match code {
        KeyCode::Backspace => AppKeyCode::Backspace,
        KeyCode::Enter => AppKeyCode::Enter,
        KeyCode::Left => AppKeyCode::Left,
        KeyCode::Right => AppKeyCode::Right,
        KeyCode::Up => AppKeyCode::Up,
        KeyCode::Down => AppKeyCode::Down,
        KeyCode::Home => AppKeyCode::Home,
        KeyCode::End => AppKeyCode::End,
        KeyCode::PageUp => AppKeyCode::PageUp,
        KeyCode::PageDown => AppKeyCode::PageDown,
        KeyCode::Tab => AppKeyCode::Tab,
        KeyCode::BackTab => AppKeyCode::BackTab,
        KeyCode::Delete => AppKeyCode::Delete,
        KeyCode::Esc => AppKeyCode::Esc,
        KeyCode::Char(c) => AppKeyCode::Char(c),
        KeyCode::F(n) => AppKeyCode::F(n),
        _ => AppKeyCode::Unknown,
    }
}

fn normalize_modifiers(modifiers: KeyModifiers) -> AppModifiers {
    let mut app = AppModifiers::NONE;
    if modifiers.contains(KeyModifiers::SHIFT) {
        app.insert(AppModifiers::SHIFT);
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        app.insert(AppModifiers::CONTROL);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        app.insert(AppModifiers::ALT);
    }
    app
}

fn normalize_mouse_kind(kind: MouseEventKind) -> AppMouseKind {
    match kind {
        MouseEventKind::Down(MouseButton::Left) => AppMouseKind::LeftDown,
        MouseEventKind::ScrollDown => AppMouseKind::ScrollDown,
        MouseEventKind::ScrollUp => AppMouseKind::ScrollUp,
        _ => AppMouseKind::Other,
    }
}

fn is_escape_fragment(key: AppKey) -> bool {
    if key.modifiers.contains(AppModifiers::ALT) {
        return true;
    }

    match key.code {
        AppKeyCode::Tab
        | AppKeyCode::BackTab
        | AppKeyCode::Left
        | AppKeyCode::Right
        | AppKeyCode::Up
        | AppKeyCode::Down
        | AppKeyCode::Home
        | AppKeyCode::End
        | AppKeyCode::PageUp
        | AppKeyCode::PageDown
        | AppKeyCode::Delete
        | AppKeyCode::F(_) => true,
        AppKeyCode::Char(c) => is_escape_fragment_char(c),
        _ => false,
    }
}

fn is_escape_fragment_char(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(
            c,
            '[' | ']'
                | 'O'
                | 'A'
                | 'B'
                | 'C'
                | 'D'
                | 'H'
                | 'F'
                | 'M'
                | 'm'
                | ';'
                | ':'
                | '~'
                | '<'
                | '>'
                | '?'
                | '$'
                | '/'
                | '^'
                | '_'
                | 'y'
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> CrosstermEvent {
        CrosstermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn drops_leaked_escape_sequence_fragments() {
        let mut normalizer = InputNormalizer::default();
        let mut queue = InputQueue::default();

        for event in [
            key(KeyCode::Esc),
            key(KeyCode::Char('[')),
            key(KeyCode::Char('1')),
            key(KeyCode::Char(';')),
            key(KeyCode::Char('5')),
            key(KeyCode::Char('D')),
        ] {
            normalizer.push_crossterm(event, &mut queue);
        }

        assert_eq!(
            queue.pop(),
            Some(AppInput::Key(AppKey::plain(AppKeyCode::Esc)))
        );
        assert_eq!(queue.pop(), None);

        normalizer.on_idle();
        normalizer.push_crossterm(key(KeyCode::Char('2')), &mut queue);
        assert_eq!(
            queue.pop(),
            Some(AppInput::Key(AppKey::plain(AppKeyCode::Char('2'))))
        );
    }
}
