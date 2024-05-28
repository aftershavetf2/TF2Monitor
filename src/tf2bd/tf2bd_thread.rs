use crate::{
    appbus::AppBus,
    models::app_settings::AppSettings,
    tf2::{lobby::Lobby, steamapi::SteamApiMsg},
};
use bus::BusReader;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

use super::models::RulesFile;

const FILENAME: &str = "playerlist.json";

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut tf2bd_thread = Tf2bdThread::new(settings, bus);

    thread::spawn(move || tf2bd_thread.run())
}

pub struct Tf2bdThread {
    bus: Arc<Mutex<AppBus>>,
    lobby_bus_rx: BusReader<Lobby>,
}

impl Tf2bdThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();

        let _rules = RulesFile::from_file(FILENAME);

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_bus();

            sleep(LOOP_DELAY);
        }
    }

    fn send(&mut self, msg: SteamApiMsg) {
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn process_bus(&mut self) {
        while let Ok(_lobby) = self.lobby_bus_rx.try_recv() {
            // log::info!("process_bus - received lobby");
        }
    }
}
