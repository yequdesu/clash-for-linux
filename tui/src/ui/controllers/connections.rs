use crate::ui::prelude::*;

impl App {
    pub fn close_selected_connection(&mut self) {
        if self.ui_state.active_page != Tab::Connections || self.connections.is_empty() {
            return;
        }
        let api = self.api.clone();
        let conn_id = self.connections[self.connections_selected].id.clone();
        self.rt.spawn(async move {
            let _ = api.close_connection(&conn_id).await;
        });
    }

    pub fn close_all_connections(&mut self) {
        if self.ui_state.active_page != Tab::Connections {
            return;
        }
        let api = self.api.clone();
        self.rt.spawn(async move {
            let _ = api.close_all_connections().await;
        });
    }
}
