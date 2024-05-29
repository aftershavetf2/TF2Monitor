use super::{models::RulesFile, ruleset_handler::RulesetHandler};
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

    ruleset_handler: RulesetHandler,
}

impl Tf2bdThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();

        let rules = RulesFile::from_file(FILENAME);
        let ruleset_handler = RulesetHandler::new(&rules, FILENAME, false);

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
            ruleset_handler,
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        let mut nth = 0;
        loop {
            nth = (nth + 1) % 10;
            let apply_rules = nth == 0;
            self.process_bus(apply_rules);

            sleep(LOOP_DELAY);
        }
    }

    fn send(&self, msg: SteamApiMsg) {
        self.bus.lock().unwrap().steamapi_bus.broadcast(msg);
    }

    fn process_bus(&mut self, apply_rules: bool) {
        if apply_rules {
            if let Ok(_lobby) = self.lobby_bus_rx.try_recv() {
                // log::info!("Applying rules to lobby");
                self.apply_rules_to_lobby(&_lobby);
            }
        }

        // Drain the lobby bus
        while let Ok(_lobby) = self.lobby_bus_rx.try_recv() {}
    }

    fn apply_rules_to_lobby(&mut self, lobby: &Lobby) {
        for player in &lobby.players {
            if let Some(data) = self.ruleset_handler.get_player_marking(&player.steamid) {
                log::info!(
                    "Player matched: {} - flags: {}",
                    player.name,
                    data.flags
                        .iter()
                        .map(|x| format!("{:?}", x))
                        .collect::<Vec<String>>()
                        .join(", ")
                );
                self.send(SteamApiMsg::Tf2bdPlayerMarking(
                    player.steamid,
                    data.clone(),
                ));
            }
        }
    }
}
