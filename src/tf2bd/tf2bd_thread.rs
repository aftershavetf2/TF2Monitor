use super::{ruleset_handler::RulesetHandler, Tf2bdMsg};
use crate::{
    appbus::{AppBus, AppEventMsg},
    models::{
        app_settings::AppSettings,
        steamid::{self, SteamID},
    },
    tf2::lobby::{Lobby, Player, PlayerFlag, Team},
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

const VOTE_PERIOD_SECONDS: u64 = 10;

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

    kick_bots: bool,
    kick_cheaters: bool,
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

            kick_bots: true,
            kick_cheaters: false,
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_bus();

            self.apply_rules_to_lobby();

            self.do_callvotes();

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

    fn process_lobby_bus(&mut self) {
        while let Ok(lobby) = self.lobby_bus_rx.try_recv() {
            self.last_lobbty = lobby;
        }
    }

    fn process_app_event_bus(&mut self) {
        while let Ok(app_event) = self.app_event_bus_rx.try_recv() {
            match app_event {
                AppEventMsg::SetPlayerFlag(steamid, flag, enable) => {
                    self.set_player_flag(steamid, flag, enable)
                }
                AppEventMsg::KickBots(enable) => self.kick_bots = enable,
                AppEventMsg::KickCheaters(enable) => self.kick_cheaters = enable,
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
        let cmd = format!("callvote kick \"{} cheating\"", player_to_kick.id);
        self.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());

        self.last_vote_time = Instant::now();
    }

    fn find_player_to_kick(&self) -> Option<&Player> {
        let me = self.last_lobbty.get_me();
        if me.is_none() {
            return None;
        }
        let me = me.unwrap();
        let team = me.team;

        if self.kick_bots {
            if let Some(bot_to_kick) = self.find_player_in_team(team, PlayerFlag::Bot) {
                return Some(bot_to_kick);
            }
        }

        if self.kick_cheaters {
            if let Some(cheater_to_kick) = self.find_player_in_team(team, PlayerFlag::Cheater) {
                return Some(cheater_to_kick);
            }
        }

        None
    }

    fn find_player_in_team(&self, team: Team, flag: PlayerFlag) -> Option<&Player> {
        let candidates: Vec<&Player> = self
            .last_lobbty
            .players
            .iter()
            .filter(|player| Self::ok_to_kick(player, team, flag))
            .collect();

        candidates.first().copied()
    }

    fn ok_to_kick(player: &Player, team: Team, flag: PlayerFlag) -> bool {
        if player.team != team {
            return false;
        }

        for (_, marking) in &player.flags {
            if marking.suggestion {
                // This marking was just a suggestion from some rule set
                continue;
            }

            if marking.flags.contains(&flag) {
                return true;
            }
        }

        false
    }
}
