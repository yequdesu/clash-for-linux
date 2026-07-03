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
    pending_left_click: Option<PendingClick>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingClick {
    column: u16,
    row: u16,
    dragged: bool,
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
                if let Some(mouse) = self.normalize_mouse(mouse.kind, mouse.column, mouse.row) {
                    queue.push(AppInput::Mouse(mouse));
                }
            }
            CrosstermEvent::Resize(cols, rows) => {
                self.esc_fragments_remaining = 0;
                self.pending_left_click = None;
                queue.push(AppInput::Resize { cols, rows });
            }
            CrosstermEvent::Paste(text) => queue.push(AppInput::Paste(text)),
            _ => {}
        }
    }

    pub(crate) fn on_idle(&mut self, queue: &mut InputQueue) {
        self.esc_fragments_remaining = 0;
        if let Some(click) = self.pending_left_click.take() {
            push_pending_click(queue, click);
        }
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

    fn normalize_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) -> Option<AppMouse> {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.pending_left_click = Some(PendingClick {
                    column,
                    row,
                    dragged: false,
                });
                None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(click) = self.pending_left_click.as_mut() {
                    click.dragged = true;
                }
                None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let click = self.pending_left_click.take()?;
                (click.column == column && click.row == row && !click.dragged)
                    .then(|| AppMouse::new(AppMouseKind::LeftClick, column, row))
            }
            MouseEventKind::ScrollDown => {
                Some(AppMouse::new(AppMouseKind::ScrollDown, column, row))
            }
            MouseEventKind::ScrollUp => Some(AppMouse::new(AppMouseKind::ScrollUp, column, row)),
            _ => {
                self.pending_left_click = None;
                None
            }
        }
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

fn push_pending_click(queue: &mut InputQueue, click: PendingClick) {
    if !click.dragged {
        queue.push(AppInput::Mouse(AppMouse::new(
            AppMouseKind::LeftClick,
            click.column,
            click.row,
        )));
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
    use crossterm::event::{KeyEvent, KeyModifiers, MouseEvent};

    fn key(code: KeyCode) -> CrosstermEvent {
        CrosstermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> CrosstermEvent {
        CrosstermEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        })
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

        normalizer.on_idle(&mut queue);
        normalizer.push_crossterm(key(KeyCode::Char('2')), &mut queue);
        assert_eq!(
            queue.pop(),
            Some(AppInput::Key(AppKey::plain(AppKeyCode::Char('2'))))
        );
    }

    #[test]
    fn mouse_down_without_release_falls_back_to_click_on_idle() {
        let mut normalizer = InputNormalizer::default();
        let mut queue = InputQueue::default();

        normalizer.push_crossterm(
            mouse(MouseEventKind::Down(MouseButton::Left), 5, 6),
            &mut queue,
        );
        assert_eq!(queue.pop(), None);

        normalizer.on_idle(&mut queue);

        assert_eq!(
            queue.pop(),
            Some(AppInput::Mouse(AppMouse::new(
                AppMouseKind::LeftClick,
                5,
                6
            )))
        );
    }

    #[test]
    fn mouse_click_requires_release_without_drag() {
        let mut normalizer = InputNormalizer::default();
        let mut queue = InputQueue::default();

        normalizer.push_crossterm(
            mouse(MouseEventKind::Down(MouseButton::Left), 3, 4),
            &mut queue,
        );
        assert_eq!(queue.pop(), None);
        normalizer.push_crossterm(
            mouse(MouseEventKind::Up(MouseButton::Left), 3, 4),
            &mut queue,
        );
        assert_eq!(
            queue.pop(),
            Some(AppInput::Mouse(AppMouse::new(
                AppMouseKind::LeftClick,
                3,
                4
            )))
        );

        normalizer.push_crossterm(
            mouse(MouseEventKind::Down(MouseButton::Left), 3, 4),
            &mut queue,
        );
        normalizer.push_crossterm(
            mouse(MouseEventKind::Drag(MouseButton::Left), 4, 4),
            &mut queue,
        );
        normalizer.push_crossterm(
            mouse(MouseEventKind::Up(MouseButton::Left), 4, 4),
            &mut queue,
        );
        normalizer.on_idle(&mut queue);
        assert_eq!(queue.pop(), None);
    }
}
