use crate::ui::prelude::*;

use crate::mouse::HitboxAction;
use ratatui::layout::Rect;

pub(crate) fn register_table_row_hitboxes<F>(
    app: &mut App,
    inner: Rect,
    count: usize,
    mut action: F,
) where
    F: FnMut(usize) -> HitboxAction,
{
    if inner.height <= 1 || inner.width == 0 {
        return;
    }
    let visible = count.min(inner.height.saturating_sub(1) as usize);
    for idx in 0..visible {
        app.ui_state.hitboxes.register(
            Rect::new(inner.x, inner.y + 1 + idx as u16, inner.width, 1),
            action(idx),
        );
    }
}
