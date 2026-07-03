use crate::app::App;
use crate::event::DataEvent;

pub(crate) fn on_tick(app: &mut App) {
    app.on_tick();
}

pub(crate) fn refresh_data(app: &mut App) {
    app.refresh_data();
}

pub(crate) fn apply_data_event(app: &mut App, event: DataEvent) {
    app.apply_data_event(event);
}

pub(crate) fn confirm_pending_action(app: &mut App) {
    app.confirm_pending_action();
}

pub(crate) fn cancel_pending_action(app: &mut App) {
    app.cancel_pending_action();
}
