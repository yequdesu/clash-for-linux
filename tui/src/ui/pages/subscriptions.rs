use crate::ui::prelude::*;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::action_registry::{self, ActionDanger};
use crate::i18n::Msg;
use crate::mouse::HitboxAction;

use crate::ui::components::action_bar::*;
use crate::ui::components::panel::Panel;
use crate::ui::hitbox::register_table_row_hitboxes;

pub(crate) fn render_subscriptions(frame: &mut Frame, area: Rect, app: &mut App) {
    fill_area(frame, area, CLASH_THEME.surface);

    let (table_area, actions_area) = if area.height >= 12 {
        let action_height = if area.height >= 22 { 8 } else { 6 };
        let rows =
            Layout::vertical([Constraint::Min(8), Constraint::Length(action_height)]).split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    let header = Row::new(vec![
        app.t(Msg::SubscriptionsId),
        app.t(Msg::SubscriptionsName),
        app.t(Msg::SubscriptionsInterval),
        app.t(Msg::SubscriptionsNext),
        app.t(Msg::SubscriptionsStatus),
        app.t(Msg::SubscriptionsUrl),
    ])
    .style(Style::default().fg(CLASH_THEME.muted));
    let data_rows: Vec<Row> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let active_marker = if p.id == app.active_profile_id {
                "● "
            } else {
                "  "
            };
            let name = if p.name.is_empty() { &p.url } else { &p.name };
            let interval = profile_interval_label(p);
            let next = if p.next_update.is_empty() {
                "—".to_string()
            } else {
                p.next_update.chars().take(16).collect()
            };
            let status = if !p.last_error.is_empty() {
                app.t(Msg::SubscriptionsStatusError)
            } else if p.id == app.active_profile_id {
                app.t(Msg::SubscriptionsStatusActive)
            } else {
                app.t(Msg::SubscriptionsStatusReady)
            };
            let style = if i == app.ui_state.subscriptions.selected_idx {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.primary)
            } else if p.id == app.active_profile_id {
                Style::default()
                    .fg(CLASH_THEME.accent)
                    .bg(CLASH_THEME.surface)
            } else {
                Style::default()
                    .fg(CLASH_THEME.text)
                    .bg(CLASH_THEME.surface)
            };
            Row::new(vec![
                format!("{}{}", active_marker, p.id),
                name.chars().take(30).collect(),
                interval,
                next,
                status.to_string(),
                p.url.chars().take(50).collect(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(28),
        Constraint::Length(10),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Ratio(1, 1),
    ];
    let row_count = data_rows.len();
    let table = Table::new(data_rows, widths)
        .header(header)
        .style(Style::default().bg(CLASH_THEME.surface));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(CLASH_THEME.border)
                .bg(CLASH_THEME.surface),
        )
        .style(Style::default().bg(CLASH_THEME.surface))
        .title(Span::styled(
            format!(" {} ", app.t(Msg::PageSubscriptions)),
            Style::default().fg(CLASH_THEME.primary).bold(),
        ));
    let inner = block.inner(table_area);
    frame.render_widget(block, table_area);
    fill_area(frame, inner, CLASH_THEME.surface);
    register_table_row_hitboxes(app, inner, row_count, HitboxAction::SelectSubscription);
    if app.profiles.is_empty() {
        frame.render_widget(
            Paragraph::new(app.t(Msg::SubscriptionsNoProfiles)).style(
                Style::default()
                    .fg(CLASH_THEME.muted)
                    .bg(CLASH_THEME.surface),
            ),
            inner,
        );
    } else {
        frame.render_widget(table, inner);
    }
    if let Some(actions_area) = actions_area {
        render_subscription_actions(frame, actions_area, app);
    }
}

pub(crate) fn render_subscription_actions(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner = Panel::new(app.t(Msg::ProxyActions)).render_block(frame, area);
    fill_area(frame, inner, CLASH_THEME.surface);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let groups = subscription_action_groups(app);
    let mut y = inner.y;
    let max_y = inner.y.saturating_add(inner.height);
    for (title, buttons) in groups {
        if y >= max_y {
            break;
        }
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", title),
                CLASH_THEME.primary,
            )))
            .style(Style::default().bg(CLASH_THEME.surface)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y = y.saturating_add(1);
        let button_height = action_buttons_needed_height(inner.width, &buttons)
            .max(1)
            .min(max_y.saturating_sub(y));
        if button_height == 0 {
            break;
        }
        render_action_buttons(
            frame,
            app,
            Rect::new(inner.x, y, inner.width, button_height),
            &buttons,
        );
        y = y.saturating_add(button_height).saturating_add(1);
    }
}

pub(crate) fn subscription_action_buttons(app: &App) -> Vec<(String, HitboxAction)> {
    let registry_button = |id: &str, action| {
        let label = action_registry::action_by_id(id)
            .map(|spec| app.action_button(spec))
            .unwrap_or("action")
            .to_string();
        (label, action)
    };
    vec![
        registry_button("sub.add", HitboxAction::BeginSubscriptionAdd),
        registry_button("sub.import", HitboxAction::BeginSubscriptionImport),
        registry_button("sub.log", HitboxAction::ShowSubscriptionLog),
        registry_button("sub.remove", HitboxAction::RemoveSubscription),
        registry_button("sub.use", HitboxAction::UseSubscription),
        registry_button("sub.update", HitboxAction::UpdateSubscription),
        registry_button("sub.edit", HitboxAction::EditSubscriptionName),
        (
            app.t(Msg::SubscriptionsQuickUrl).to_string(),
            HitboxAction::EditSubscriptionUrl,
        ),
        (
            app.t(Msg::SubscriptionsQuickInterval).to_string(),
            HitboxAction::EditSubscriptionInterval,
        ),
        (
            app.t(Msg::SubscriptionsQuickUserAgent).to_string(),
            HitboxAction::EditSubscriptionUserAgent,
        ),
        (
            app.t(Msg::SubscriptionsQuickUpdateProxy).to_string(),
            HitboxAction::EditSubscriptionUpdateProxy,
        ),
        (
            app.t(Msg::SubscriptionsQuickConvert).to_string(),
            HitboxAction::EditSubscriptionConvertMode,
        ),
        (
            app.t(Msg::SubscriptionsQuickAddTag).to_string(),
            HitboxAction::EditSubscriptionAddTag,
        ),
        (
            app.t(Msg::SubscriptionsQuickRemoveTag).to_string(),
            HitboxAction::EditSubscriptionRemoveTag,
        ),
    ]
}

fn subscription_action_groups(app: &App) -> Vec<(&'static str, Vec<ActionButtonItem>)> {
    let item = |label: String, action| action_button_item(label, action, ActionDanger::Safe);
    let mut groups = Vec::new();
    let buttons = subscription_action_buttons(app);
    let find = |action: &HitboxAction| {
        buttons
            .iter()
            .find(|(_, candidate)| candidate == action)
            .map(|(label, action)| item(label.clone(), action.clone()))
    };

    groups.push((
        app.t(Msg::ActionsPrimary),
        [
            HitboxAction::UseSubscription,
            HitboxAction::UpdateSubscription,
            HitboxAction::BeginSubscriptionAdd,
            HitboxAction::BeginSubscriptionImport,
            HitboxAction::ShowSubscriptionLog,
            HitboxAction::RemoveSubscription,
        ]
        .iter()
        .filter_map(|action| find(action))
        .collect(),
    ));
    groups.push((
        app.t(Msg::ActionsEdit),
        [
            HitboxAction::EditSubscriptionName,
            HitboxAction::EditSubscriptionUrl,
            HitboxAction::EditSubscriptionInterval,
            HitboxAction::EditSubscriptionUserAgent,
            HitboxAction::EditSubscriptionUpdateProxy,
            HitboxAction::EditSubscriptionConvertMode,
        ]
        .iter()
        .filter_map(|action| find(action))
        .collect(),
    ));
    groups.push((
        app.t(Msg::ActionsTags),
        [
            HitboxAction::EditSubscriptionAddTag,
            HitboxAction::EditSubscriptionRemoveTag,
        ]
        .iter()
        .filter_map(|action| find(action))
        .collect(),
    ));
    groups
}
