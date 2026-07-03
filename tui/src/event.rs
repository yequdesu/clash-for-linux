use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::time::Duration;

use crate::{api, mouse::SettingsAction};

#[derive(Debug)]
pub enum Event {
    Init,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

#[derive(Debug)]
pub enum DataEvent {
    Proxies(Result<api::ProxiesResponse, String>),
    Connections(Result<api::ConnectionsResponse, String>),
    Logs(Result<Vec<api::LogEntry>, String>),
    Version(Result<api::KernelInfo, String>),
    DelayResult(String, Result<u64, String>),
    SwitchResult(String, String, Result<(), String>),
    ModeResult(Result<String, String>),
    SubscriptionResult(Result<String, String>),
    SubscriptionOutputResult(String, Result<String, String>),
    NetworkResult(String, Result<String, String>),
    SettingsResult(SettingsAction, Result<String, String>),
    SettingsCommandResult(String, bool, Result<String, String>),
    Traffic(Result<api::TrafficSnapshot, String>),
    TrafficExport(Result<String, String>),
    TrafficActionResult(String, Result<String, String>),
}

pub struct EventHandler {
    tick_rate: Duration,
    data_rx: mpsc::Receiver<DataEvent>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64, data_rx: mpsc::Receiver<DataEvent>) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
            data_rx,
        }
    }

    pub fn next(&self) -> Result<Event, Box<dyn std::error::Error>> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Mouse(mouse) => Ok(Event::Mouse(mouse)),
                CrosstermEvent::Resize(_, _) => Ok(Event::Init),
                _ => self.next(),
            }
        } else {
            Ok(Event::Tick)
        }
    }

    pub fn try_recv_data(&self) -> Option<DataEvent> {
        self.data_rx.try_recv().ok()
    }
}
