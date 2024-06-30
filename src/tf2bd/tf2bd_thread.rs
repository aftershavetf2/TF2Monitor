use super::{models::PlayerAttribute, ruleset_handler::RulesetHandler, Tf2bdMsg};
use crate::{
    appbus::{AppBus, AppEventMsg},
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::lobby::{Lobby, Player, Team},
};
use bus::BusReader;
use std::{
    collections::HashSet,
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

    app_settings: AppSettings,

    ruleset_handler: RulesetHandler,

    last_lobbty: Lobby,
    last_vote_time: Instant,

    notifications_send: HashSet<SteamID>,
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

            app_settings: settings.clone(),

            ruleset_handler,

            last_lobbty: Lobby::new(settings.self_steamid64),

            last_vote_time: Instant::now(),

            notifications_send: HashSet::new(),
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_bus();

            self.apply_rules_to_lobby();
            self.send_notifications();

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
            if self.last_lobbty.lobby_id != lobby.lobby_id {
                log::info!("New lobby detected: {}", lobby.lobby_id);
                self.notifications_send.clear();
            }

            self.last_lobbty = lobby;
        }
    }

    fn process_app_event_bus(&mut self) {
        while let Ok(app_event) = self.app_event_bus_rx.try_recv() {
            match app_event {
                AppEventMsg::SetPlayerFlag(player, flag, enable) => {
                    self.set_player_flag(player, flag, enable)
                }
                AppEventMsg::UpdatedSettings(settings) => self.app_settings = settings,
            }
        }
    }

    fn set_player_flag(&mut self, player: Player, player_attribute: PlayerAttribute, enable: bool) {
        log::info!(
            "Setting player attribute {:?} for {}({}) to {}",
            player_attribute,
            player.name,
            player.steamid.to_u64(),
            enable
        );

        // Send out the updated marking
        let data = self.ruleset_handler.get_player_marking(&player.steamid);
        self.send(Tf2bdMsg::Tf2bdPlayerMarking(player.steamid, data.cloned()));

        self.ruleset_handler
            .set_player_flags(player, player_attribute, enable);
    }

    fn apply_rules_to_lobby(&mut self) {
        for player in &self.last_lobbty.players {
            let data = self
                .ruleset_handler
                .get_player_marking(&player.steamid)
                .cloned();
            self.send(Tf2bdMsg::Tf2bdPlayerMarking(player.steamid, data));
        }
    }

    fn do_callvotes(&mut self) {
        if !self.app_settings.kick_cheaters && !self.app_settings.kick_bots {
            return;
        }

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
        me?;

        let me = me.unwrap();
        let team = me.team;

        if self.app_settings.kick_bots {
            if let Some(bot_to_kick) = self.find_player_in_team(team, PlayerAttribute::Bot) {
                return Some(bot_to_kick);
            }
        }

        if self.app_settings.kick_cheaters {
            if let Some(cheater_to_kick) = self.find_player_in_team(team, PlayerAttribute::Cheater)
            {
                return Some(cheater_to_kick);
            }
        }

        None
    }

    fn find_player_in_team(
        &self,
        team: Team,
        player_attribute: PlayerAttribute,
    ) -> Option<&Player> {
        let candidates: Vec<&Player> = self
            .last_lobbty
            .players
            .iter()
            .filter(|player| Self::ok_to_kick(player, team, player_attribute))
            .collect();

        candidates.first().copied()
    }

    fn ok_to_kick(player: &Player, team: Team, player_attribute: PlayerAttribute) -> bool {
        if player.team != team {
            return false;
        }

        if let Some(player_info) = &player.player_info {
            if player_info.attributes.contains(&player_attribute) {
                return true;
            }
        }

        false
    }

    fn send_notifications(&mut self) {
        for player in &self.last_lobbty.players {
            let player_info = self
                .ruleset_handler
                .get_player_marking(&player.steamid)
                .cloned();
            if let Some(player_info) = player_info {
                if !self.notifications_send.contains(&player.steamid) {
                    let is_dodgy = player_info.attributes.contains(&PlayerAttribute::Cheater)
                        || player_info.attributes.contains(&PlayerAttribute::Bot);

                    if is_dodgy {
                        log::info!(
                            "Informing party about flags {:?} on player {}",
                            player_info.attributes,
                            player.name
                        );

                        let cmd = format!(
                            "say_party \"Player {} is marked as {:?}\"",
                            player.name, player_info.attributes
                        );
                        self.bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
                    }

                    self.notifications_send.insert(player.steamid);
                }
            }
        }
    }
}
