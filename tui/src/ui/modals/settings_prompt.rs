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

pub(crate) fn render_settings_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(prompt) = app.ui_state.settings.prompt.clone() else {
        return;
    };
    let popup = centered_rect(area, 68, 44);
    clear_floating_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.warning))
        .title(Span::styled(
            format!(" {} ", app.settings_prompt_label(prompt.kind)),
            Style::default().fg(CLASH_THEME.warning).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SettingsPromptControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let active_hint = prompt
        .active_field()
        .map(|field| app.settings_prompt_field_hint(field.field))
        .unwrap_or_else(|| app.settings_prompt_hint(prompt.kind));
    let mut lines = vec![
        Line::from(Span::styled(
            app.settings_prompt_hint(prompt.kind),
            CLASH_THEME.muted,
        )),
        Line::from(Span::styled(active_hint, CLASH_THEME.muted)),
        Line::from(""),
    ];
    for (idx, field) in prompt.fields.iter().enumerate() {
        let active = idx == prompt.active;
        let marker = if active { ">" } else { " " };
        let mut displayed = field.display_value();
        if active {
            displayed.push('█');
        }
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "{} {:<14} ",
                    marker,
                    app.settings_prompt_field_label(field.field)
                ),
                if active {
                    CLASH_THEME.warning
                } else {
                    CLASH_THEME.muted
                },
            ),
            Span::styled(displayed, CLASH_THEME.text),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", app.t(Msg::CommonContinue)),
            button_style(ActionDanger::Safe),
        ),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!(" {} ", app.t(Msg::CommonCancel)),
            button_style(ActionDanger::Confirm),
        ),
    ]));
    frame.render_widget(block, popup);
    for idx in 0..prompt.fields.len() {
        app.ui_state.hitboxes.register(
            Rect::new(
                inner.x,
                inner.y.saturating_add(3 + idx as u16),
                inner.width,
                1,
            ),
            HitboxAction::SelectSettingsPromptField(idx),
        );
    }
    app.ui_state.hitboxes.register(
        Rect::new(
            inner.x,
            inner.y.saturating_add(4 + prompt.fields.len() as u16),
            action_button_width(app.t(Msg::CommonContinue)),
            1,
        ),
        HitboxAction::SubmitSettingsPrompt,
    );
    app.ui_state.hitboxes.register(
        Rect::new(
            inner
                .x
                .saturating_add(action_button_width(app.t(Msg::CommonContinue)).saturating_add(3)),
            inner.y.saturating_add(4 + prompt.fields.len() as u16),
            action_button_width(app.t(Msg::CommonCancel)),
            1,
        ),
        HitboxAction::CancelSettingsPrompt,
    );
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}
