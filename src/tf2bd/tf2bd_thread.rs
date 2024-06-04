use super::{models::RulesFile, ruleset_handler::RulesetHandler, Tf2bdMsg};
use crate::{
    appbus::{AppBus, AppEventMsg},
    models::{app_settings::AppSettings, steamid},
    tf2::lobby::{Lobby, PlayerFlag},
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
    app_event_bus_rx: BusReader<AppEventMsg>,

    ruleset_handler: RulesetHandler,
}

impl Tf2bdThread {
    pub fn new(_settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();
        let app_event_bus_rx = bus.lock().unwrap().app_event_bus.add_rx();

        let ruleset_handler = RulesetHandler::new(FILENAME, false);

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
            app_event_bus_rx,
            ruleset_handler,
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_bus();

            sleep(LOOP_DELAY);
        }
    }

    fn send(&self, msg: Tf2bdMsg) {
        self.bus.lock().unwrap().tf2bd_bus.broadcast(msg);
    }

    fn process_bus(&mut self) {
        self.process_lobby_bus();
        self.process_app_event_bus();
    }

    fn process_app_event_bus(&mut self) {
        while let Ok(app_event) = self.app_event_bus_rx.try_recv() {
            match app_event {
                AppEventMsg::SetPlayerFlag(steamid, flag, enable) => {
                    self.set_player_flag(steamid, flag, enable)
                }
            }
        }
    }

    fn set_player_flag(&mut self, steamid: steamid::SteamID, flag: PlayerFlag, enable: bool) {
        log::info!(
            "Setting player flag {:?} for {} to {}",
            flag,
            steamid.to_u64(),
            enable
        );
        self.ruleset_handler.set_player_flags(steamid, flag, enable);
        if let Some(data) = self.ruleset_handler.get_player_marking(&steamid) {
            self.send(Tf2bdMsg::Tf2bdPlayerMarking(steamid, data.clone()));
        }
    }

    fn process_lobby_bus(&mut self) {
        while let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            // log::info!("Applying rules to lobby");
            self.apply_rules_to_lobby(&lobby);
        }
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
                self.send(Tf2bdMsg::Tf2bdPlayerMarking(player.steamid, data.clone()));
            }
        }
    }
}
