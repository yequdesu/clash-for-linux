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

pub(crate) fn render_subscription_prompt(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.subscription_add_form.is_some() {
        render_subscription_add_form(frame, area, app);
        return;
    }
    if app.subscription_edit_form.is_some() {
        render_subscription_edit_form(frame, area, app);
        return;
    }
    let Some(prompt) = app.subscription_prompt.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 72, 32);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            if prompt.field == SubscriptionEditField::ImportDirectory {
                format!(" {} ", app.subscription_edit_field_label(prompt.field))
            } else {
                format!(
                    " {} {} · [{}] ",
                    app.t(Msg::SubscriptionsEditTitle),
                    app.subscription_edit_field_label(prompt.field),
                    prompt.profile_id
                )
            },
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsQuickPromptControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let value = if prompt.value.is_empty() {
        format!("<{}>", app.t(Msg::CommonEmpty))
    } else {
        prompt.value.clone()
    };
    let lines = vec![
        Line::from(Span::styled(
            app.subscription_edit_field_hint(prompt.field),
            CLASH_THEME.muted,
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{}: ", app.t(Msg::CommonValue)),
                CLASH_THEME.primary,
            ),
            Span::styled(value, CLASH_THEME.text),
            Span::styled("█", CLASH_THEME.primary),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", app.t(Msg::CommonSave)),
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
    app.ui_state.hitboxes.register(
        Rect::new(inner.x, inner.y.saturating_add(2), inner.width, 1),
        HitboxAction::FocusSettingsPromptValue,
    );
    let button_y = inner.y.saturating_add(4);
    let save_width = action_button_width(app.t(Msg::CommonSave));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.ui_state.hitboxes.register(
        Rect::new(inner.x, button_y, save_width, 1),
        HitboxAction::SubmitSubscriptionPrompt,
    );
    app.ui_state.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(save_width).saturating_add(3),
            button_y,
            cancel_width,
            1,
        ),
        HitboxAction::CancelSubscriptionPrompt,
    );
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

pub(crate) fn render_subscription_edit_form(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(form) = app.subscription_edit_form.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 82, 56);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(
                " {} {} ",
                app.t(Msg::SubscriptionsEditTitle),
                form.profile_id
            ),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsFormControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let mut lines = vec![
        Line::from(Span::styled(
            app.subscription_add_field_hint(form.active_field()),
            CLASH_THEME.muted,
        )),
        Line::from(""),
    ];
    for (idx, field) in SubscriptionAddField::all().iter().enumerate() {
        let active = idx == form.active;
        let label_style = if active {
            Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg)
        };
        let value_style = if active {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
        };
        let value = edit_form_display_value(app, form, *field, active);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<14}", app.subscription_add_field_label(*field)),
                label_style,
            ),
            Span::styled(" ", value_style),
            Span::styled(value, value_style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", app.t(Msg::CommonSave)),
            button_style(ActionDanger::Safe),
        ),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!(" {} ", app.t(Msg::CommonCancel)),
            button_style(ActionDanger::Confirm),
        ),
    ]));

    frame.render_widget(block, popup);
    register_subscription_form_hitboxes(app, inner);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

pub(crate) fn render_subscription_add_form(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(form) = app.subscription_add_form.as_ref() else {
        return;
    };
    let popup = centered_rect(area, 82, 56);
    fill_area(frame, popup, CLASH_THEME.bg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLASH_THEME.primary))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsAddTitle)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ))
        .title_bottom(Span::styled(
            format!(" {} ", app.t(Msg::SubscriptionsFormControlsHint)),
            Style::default().fg(CLASH_THEME.muted),
        ));
    let inner = block.inner(popup).inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let mut lines = vec![
        Line::from(Span::styled(
            app.subscription_add_field_hint(form.active_field()),
            CLASH_THEME.muted,
        )),
        Line::from(""),
    ];
    for (idx, field) in SubscriptionAddField::all().iter().enumerate() {
        let active = idx == form.active;
        let label_style = if active {
            Style::default().fg(CLASH_THEME.bg).bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.primary).bg(CLASH_THEME.bg)
        };
        let value_style = if active {
            Style::default()
                .fg(CLASH_THEME.text)
                .bg(CLASH_THEME.primary)
        } else {
            Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)
        };
        let value = add_form_display_value(app, form, *field, active);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<14}", app.subscription_add_field_label(*field)),
                label_style,
            ),
            Span::styled(" ", value_style),
            Span::styled(value, value_style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", app.t(Msg::CommonSave)),
            button_style(ActionDanger::Safe),
        ),
        Span::styled("   ", CLASH_THEME.text),
        Span::styled(
            format!(" {} ", app.t(Msg::CommonCancel)),
            button_style(ActionDanger::Confirm),
        ),
    ]));

    frame.render_widget(block, popup);
    register_subscription_form_hitboxes(app, inner);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(CLASH_THEME.text).bg(CLASH_THEME.bg)),
        inner,
    );
}

pub(crate) fn register_subscription_form_hitboxes(app: &mut App, inner: Rect) {
    let field_y = inner.y.saturating_add(2);
    for (idx, _) in SubscriptionAddField::all().iter().enumerate() {
        if field_y.saturating_add(idx as u16) < inner.y.saturating_add(inner.height) {
            app.ui_state.hitboxes.register(
                Rect::new(inner.x, field_y + idx as u16, inner.width, 1),
                HitboxAction::SelectSubscriptionAddField(idx),
            );
        }
    }
    let button_y = field_y
        .saturating_add(SubscriptionAddField::all().len() as u16)
        .saturating_add(1);
    let save_width = action_button_width(app.t(Msg::CommonSave));
    let cancel_width = action_button_width(app.t(Msg::CommonCancel));
    app.ui_state.hitboxes.register(
        Rect::new(inner.x, button_y, save_width, 1),
        HitboxAction::SubmitSubscriptionPrompt,
    );
    app.ui_state.hitboxes.register(
        Rect::new(
            inner.x.saturating_add(save_width).saturating_add(3),
            button_y,
            cancel_width,
            1,
        ),
        HitboxAction::CancelSubscriptionPrompt,
    );
}

pub(crate) fn add_form_display_value(
    app: &App,
    form: &SubscriptionAddForm,
    field: SubscriptionAddField,
    active: bool,
) -> String {
    let raw = match field {
        SubscriptionAddField::Source => &form.source,
        SubscriptionAddField::Name => &form.name,
        SubscriptionAddField::Interval => &form.interval,
        SubscriptionAddField::UpdateProxy => &form.update_proxy,
        SubscriptionAddField::UserAgent => &form.user_agent,
        SubscriptionAddField::ConvertMode => &form.convert_mode,
        SubscriptionAddField::Tags => &form.tags,
    };
    let mut value = if raw.is_empty() {
        match field {
            SubscriptionAddField::Source => format!("<{}>", app.t(Msg::CommonRequired)),
            SubscriptionAddField::Name
            | SubscriptionAddField::Interval
            | SubscriptionAddField::UserAgent
            | SubscriptionAddField::Tags => format!("<{}>", app.t(Msg::CommonOptional)),
            SubscriptionAddField::UpdateProxy | SubscriptionAddField::ConvertMode => {
                format!("<{}>", app.t(Msg::CommonDefault))
            }
        }
    } else {
        trunc_str(raw, 76)
    };
    if active {
        value.push('█');
    }
    value
}

pub(crate) fn edit_form_display_value(
    app: &App,
    form: &SubscriptionEditForm,
    field: SubscriptionAddField,
    active: bool,
) -> String {
    let raw = match field {
        SubscriptionAddField::Source => &form.url,
        SubscriptionAddField::Name => &form.name,
        SubscriptionAddField::Interval => &form.interval,
        SubscriptionAddField::UpdateProxy => &form.update_proxy,
        SubscriptionAddField::UserAgent => &form.user_agent,
        SubscriptionAddField::ConvertMode => &form.convert_mode,
        SubscriptionAddField::Tags => &form.tags,
    };
    let mut value = if raw.is_empty() {
        match field {
            SubscriptionAddField::Source => format!("<{}>", app.t(Msg::CommonRequired)),
            SubscriptionAddField::Name
            | SubscriptionAddField::UserAgent
            | SubscriptionAddField::Tags => format!("<{}>", app.t(Msg::CommonEmpty)),
            SubscriptionAddField::Interval
            | SubscriptionAddField::UpdateProxy
            | SubscriptionAddField::ConvertMode => format!("<{}>", app.t(Msg::CommonRequired)),
        }
    } else {
        trunc_str(raw, 76)
    };
    if active {
        value.push('█');
    }
    value
}
