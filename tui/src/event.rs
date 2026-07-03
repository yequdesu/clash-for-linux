use crossterm::event::{self, Event as CrosstermEvent};
use std::sync::mpsc;
use std::time::Duration;

use crate::input::{AppInput, InputNormalizer, InputQueue};
use crate::{api, mouse::SettingsAction};

#[derive(Debug)]
pub enum Event {
    Init,
    Input(AppInput),
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
    input: InputNormalizer,
    input_queue: InputQueue,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64, data_rx: mpsc::Receiver<DataEvent>) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
            data_rx,
            input: InputNormalizer::default(),
            input_queue: InputQueue::default(),
        }
    }

    pub fn next(&mut self) -> Result<Event, Box<dyn std::error::Error>> {
        if let Some(input) = self.input_queue.pop() {
            return Ok(Event::Input(input));
        }

        if event::poll(self.tick_rate)? {
            let event = event::read()?;
            if matches!(
                event,
                CrosstermEvent::FocusGained | CrosstermEvent::FocusLost
            ) {
                return self.next();
            }
            self.input.push_crossterm(event, &mut self.input_queue);
            Ok(self
                .input_queue
                .pop()
                .map(Event::Input)
                .unwrap_or(Event::Init))
        } else {
            self.input.on_idle(&mut self.input_queue);
            Ok(self
                .input_queue
                .pop()
                .map(Event::Input)
                .unwrap_or(Event::Tick))
        }
    }

    pub fn try_recv_data(&self) -> Option<DataEvent> {
        self.data_rx.try_recv().ok()
    }
}
