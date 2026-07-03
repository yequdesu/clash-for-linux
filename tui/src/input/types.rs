#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AppInput {
    Key(AppKey),
    Mouse(AppMouse),
    Resize { cols: u16, rows: u16 },
    Paste(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppKey {
    pub code: AppKeyCode,
    pub modifiers: AppModifiers,
}

impl AppKey {
    pub(crate) fn new(code: AppKeyCode, modifiers: AppModifiers) -> Self {
        Self { code, modifiers }
    }

    #[cfg(test)]
    pub(crate) fn plain(code: AppKeyCode) -> Self {
        Self::new(code, AppModifiers::NONE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppKeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Esc,
    Char(char),
    F(u8),
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AppModifiers(u8);

impl AppModifiers {
    pub(crate) const NONE: Self = Self(0);
    pub(crate) const SHIFT: Self = Self(1 << 0);
    pub(crate) const CONTROL: Self = Self(1 << 1);
    pub(crate) const ALT: Self = Self(1 << 2);

    pub(crate) fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub(crate) fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppMouse {
    pub kind: AppMouseKind,
    pub x: u16,
    pub y: u16,
}

impl AppMouse {
    pub(crate) fn new(kind: AppMouseKind, x: u16, y: u16) -> Self {
        Self { kind, x, y }
    }

    #[cfg(test)]
    pub(crate) fn left_down(x: u16, y: u16) -> Self {
        Self::new(AppMouseKind::LeftClick, x, y)
    }

    #[cfg(test)]
    pub(crate) fn scroll_down(x: u16, y: u16) -> Self {
        Self::new(AppMouseKind::ScrollDown, x, y)
    }

    #[cfg(test)]
    pub(crate) fn scroll_up(x: u16, y: u16) -> Self {
        Self::new(AppMouseKind::ScrollUp, x, y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppMouseKind {
    LeftClick,
    ScrollDown,
    ScrollUp,
}
