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

pub(crate) fn render_sudo_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(prompt) = app.sudo_prompt.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 62, 28);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.warning))
        .title(Span::styled(
            format!(" sudo · {} ", prompt.label),
            Style::default().fg(CLASH_THEME.warning).bold(),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    let masked = if prompt.password.is_empty() {
        "<password>".to_string()
    } else {
        "•".repeat(prompt.password.chars().count())
    };
    let lines = vec![
        Line::from(Span::styled(
            "  This action needs elevated privileges.",
            CLASH_THEME.muted,
        )),
        Line::from(Span::styled(
            format!("  clashctl {}", prompt.args.join(" ")),
            CLASH_THEME.text,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Password  ", CLASH_THEME.muted),
            Span::styled(masked, CLASH_THEME.text),
            Span::styled("█", CLASH_THEME.primary),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", app.t(Msg::CommonContinue)),
                button_style(ActionDanger::Safe),
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

    app.ui_state.hitboxes.register(
        Rect::new(inner.x, inner.y.saturating_add(3), inner.width, 1),
        HitboxAction::FocusSudoPromptValue,
    );
    let action_y = inner.y.saturating_add(5);
    app.ui_state.hitboxes.register(
        Rect::new(
            inner.x,
            action_y,
            action_button_width(app.t(Msg::CommonContinue)),
            1,
        ),
        HitboxAction::SubmitSudoPrompt,
    );
    app.ui_state.hitboxes.register(
        Rect::new(
            inner
                .x
                .saturating_add(action_button_width(app.t(Msg::CommonContinue)).saturating_add(3)),
            action_y,
            action_button_width(app.t(Msg::CommonCancel)),
            1,
        ),
        HitboxAction::CancelSudoPrompt,
    );
}
