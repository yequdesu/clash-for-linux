use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

#[derive(Debug)]
pub enum DataEvent {
    ProxiesFetched(Result<crate::api::ProxiesResponse, String>),
    TrafficFetched(Result<crate::api::TrafficInfo, String>),
    ConnectionsFetched(Result<Vec<crate::api::Connection>, String>),
    MemoryFetched(Result<crate::api::MemoryInfo, String>),
    ConfigFetched(Result<crate::api::RuntimeConfig, String>),
    VersionFetched(Result<String, String>),
    ModeSet(Result<(), String>),
    ProxySwitched(Result<(), String>),
    DelayTested(String, Result<i64, String>),
    ConnectionClosed(Result<(), String>),
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
                CrosstermEvent::Resize(_, _) => Ok(Event::Tick),
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
