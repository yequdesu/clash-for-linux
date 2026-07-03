use crate::ui::prelude::*;

use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::action_registry::ActionDanger;
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::layout::centered_rect;

pub(crate) fn render_confirmation_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(prompt) = app.ui_state.modals.pending_confirmation.clone() else {
        return;
    };
    let popup = centered_rect(area, 64, 28);
    clear_floating_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.warning))
        .title(Span::styled(
            format!(" {} ", prompt.title),
            Style::default().fg(CLASH_THEME.warning).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::ConfirmHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let lines = vec![
        Line::from(Span::styled(
            app.t(Msg::ConfirmSystemStateWarning),
            CLASH_THEME.muted,
        )),
        Line::from(""),
        Line::from(Span::styled(prompt.message, CLASH_THEME.text)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", app.t(Msg::CommonConfirm)),
                button_style(ActionDanger::Dangerous),
            ),
            Span::styled("   ", CLASH_THEME.text),
            Span::styled(
                format!(" {} ", app.t(Msg::CommonCancel)),
                button_style(ActionDanger::Confirm),
            ),
        ]),
    ];
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
    let confirm_width = action_button_width(app.t(Msg::CommonConfirm));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.ui_state.hitboxes.register(
        Rect::new(inner.x, inner.y + 4, confirm_width, 1),
        HitboxAction::ConfirmPendingAction,
    );
    app.ui_state.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(confirm_width).saturating_add(3),
            inner.y + 4,
            cancel_width,
            1,
        ),
        HitboxAction::CancelPendingAction,
    );
}
