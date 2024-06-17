use super::{ruleset_handler::RulesetHandler, Tf2bdMsg};
use crate::{
    appbus::{AppBus, AppEventMsg},
    models::{
        app_settings::AppSettings,
        steamid::{self, SteamID},
    },
    tf2::lobby::{Lobby, Player, PlayerFlag},
};
use bus::BusReader;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Instant,
};

const FILENAME: &str = "playerlist.json";

/// The delay between loops in run()
const LOOP_DELAY: std::time::Duration = std::time::Duration::from_millis(250);

const VOTE_PERIOD_SECONDS: u64 = 15;

pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut tf2bd_thread = Tf2bdThread::new(settings, bus);

    thread::spawn(move || tf2bd_thread.run())
}

struct Tf2bdThread {
    bus: Arc<Mutex<AppBus>>,
    lobby_bus_rx: BusReader<Lobby>,
    app_event_bus_rx: BusReader<AppEventMsg>,

    ruleset_handler: RulesetHandler,

    last_lobbty: Lobby,
    last_vote_time: Instant,
}

impl Tf2bdThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let lobby_bus_rx = bus.lock().unwrap().lobby_report_bus.add_rx();
        let app_event_bus_rx = bus.lock().unwrap().app_event_bus.add_rx();

        let ruleset_handler = RulesetHandler::new(FILENAME, false);

        Self {
            bus: Arc::clone(bus),
            lobby_bus_rx,
            app_event_bus_rx,
            ruleset_handler,

            last_lobbty: Lobby::new(settings.self_steamid64),

            last_vote_time: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_bus();

            self.apply_rules_to_lobby();

            // self.do_callvotes();

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
                AppEventMsg::CallVote(_, _, _) => todo!(),
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

        // Send out the updated marking
        let data = self.ruleset_handler.get_player_marking(&steamid);
        self.send(Tf2bdMsg::Tf2bdPlayerMarking(
            steamid,
            self.ruleset_handler.source.clone(),
            data.cloned(),
        ));
    }

    fn process_lobby_bus(&mut self) {
        while let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            self.last_lobbty = lobby;
        }
    }

    fn apply_rules_to_lobby(&mut self) {
        for player in &self.last_lobbty.players {
            let data = self.ruleset_handler.get_player_marking(&player.steamid);
            self.send(Tf2bdMsg::Tf2bdPlayerMarking(
                player.steamid,
                self.ruleset_handler.source.clone(),
                data.cloned(),
            ));
        }
    }

    fn do_callvotes(&mut self) {
        let passed_time = self.last_vote_time.elapsed().as_secs();
        if passed_time < VOTE_PERIOD_SECONDS {
            // log::info!("Not time yet to call a vote");
            return;
        }

        let player_to_kick = self.find_player_to_kick();
        if player_to_kick.is_none() {
            self.last_vote_time = Instant::now();
            log::info!("Found no player to kick");
            return;
        }

        let player_to_kick = player_to_kick.unwrap();

        log::info!("Calling vote to kick player {}", player_to_kick.name);

        self.last_vote_time = Instant::now();
    }

    fn find_player_to_kick(&self) -> Option<&Player> {
        // let me = self.last_lobbty.get_player(None, Some())
        // let player = self.last_lobbty.players.iter().find(|player| {
        //     let marking = self.ruleset_handler.get_player_marking(&player.steamid);
        //     marking.is_some()
        // });

        // if let Some(player) = player {
        //     self.send(Tf2bdMsg::Tf2bdCallVote(player.steamid));
        // }
        None
    }
}
