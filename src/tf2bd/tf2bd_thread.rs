use super::{
    Tf2bdMsg, models::PlayerAttribute, player_lists::SharedMatches, ruleset_handler::RulesetHandler,
};
use crate::config::TF2BD_LOOP_DELAY;
use crate::db::connection::DbPool;
use crate::{
    appbus::{AppBus, AppEventMsg},
    models::{app_settings::AppSettings, steamid::SteamID},
    tf2::lobby::{Lobby, Player, Team, player_attribute_description},
};
use bus::BusReader;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Instant,
};

const FILENAME: &str = "playerlist.json";

const VOTE_PERIOD_SECONDS: u64 = 10;

pub fn start(
    settings: &AppSettings,
    bus: &Arc<Mutex<AppBus>>,
    db: &DbPool,
) -> thread::JoinHandle<()> {
    let settings = settings.clone();
    let bus = Arc::clone(bus);
    let db = db.clone();
    thread::spawn(move || match Tf2bdThread::new(&settings, &bus, &db) {
        Ok(mut worker) => worker.run(),
        Err(error) => log::error!(
            "Could not initialize database-backed player markings: {}",
            error
        ),
    })
}

struct Tf2bdThread {
    bus: Arc<Mutex<AppBus>>,
    shared_lobby: crate::tf2::lobby::shared_lobby::SharedLobby,
    app_event_bus_rx: BusReader<AppEventMsg>,

    app_settings: AppSettings,

    ruleset_handler: RulesetHandler,

    last_lobby_id: String,
    last_vote_time: Instant,

    local_notifications_seen: HashSet<SteamID>,
    tf2bd_matches: SharedMatches,
    tf2bd_notifications_sent: HashSet<SteamID>,
}

impl Tf2bdThread {
    pub fn new(
        settings: &AppSettings,
        bus: &Arc<Mutex<AppBus>>,
        db: &DbPool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();
        let app_event_bus_rx = bus.lock().unwrap().app_event_bus.add_rx();
        let tf2bd_matches = bus.lock().unwrap().tf2bd_matches.clone();

        let ruleset_handler = RulesetHandler::new(FILENAME, db)?;

        Ok(Self {
            bus: Arc::clone(bus),
            shared_lobby,
            app_event_bus_rx,

            app_settings: settings.clone(),

            ruleset_handler,

            last_lobby_id: String::new(),

            last_vote_time: Instant::now(),

            local_notifications_seen: HashSet::new(),
            tf2bd_matches,
            tf2bd_notifications_sent: HashSet::new(),
        })
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_app_event_bus();

            self.ruleset_handler.sync_export();
            self.apply_rules_to_lobby();
            self.send_notifications();

            self.do_callvotes();

            sleep(TF2BD_LOOP_DELAY);
        }
    }

    fn send(&self, msg: Tf2bdMsg) {
        self.bus.lock().unwrap().tf2bd_bus.broadcast(msg);
    }

    fn process_app_event_bus(&mut self) {
        while let Ok(app_event) = self.app_event_bus_rx.try_recv() {
            match app_event {
                AppEventMsg::SetPlayerFlag {
                    steamid,
                    name,
                    flag,
                    enable,
                } => self.set_player_flag(steamid, &name, flag, enable),
                AppEventMsg::UpdatedSettings(settings) => self.app_settings = settings,
            }
        }
    }

    fn set_player_flag(
        &mut self,
        steamid: SteamID,
        player_name: &str,
        player_attribute: PlayerAttribute,
        enable: bool,
    ) {
        log::info!(
            "Setting player attribute {:?} for {} ({}) to {}",
            player_attribute,
            player_name,
            steamid.to_u64(),
            enable
        );

        if let Err(error) = self.ruleset_handler.set_player_flags(
            steamid,
            Some(player_name),
            player_attribute,
            enable,
        ) {
            log::error!(
                "Failed to save player marking for {}: {}",
                steamid.to_u64(),
                error
            );
            return;
        }
        self.bus.lock().unwrap().profile_events.broadcast(
            crate::models::player_profile::ProfileEvent::Changed(steamid),
        );
        let data = self.ruleset_handler.get_player_marking(&steamid);
        self.send(Tf2bdMsg::Tf2bdPlayerMarking(steamid, data.cloned()));
    }

    fn apply_rules_to_lobby(&mut self) {
        let lobby = self.shared_lobby.get();
        for player in &lobby.players {
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

    fn find_player_to_kick(&self) -> Option<Player> {
        let lobby = self.shared_lobby.get();
        let me = lobby.get_me()?;
        let team = me.team;

        if self.app_settings.kick_bots
            && let Some(bot_to_kick) = self.find_player_in_team(&lobby, team, PlayerAttribute::Bot)
        {
            return Some(bot_to_kick);
        }

        if self.app_settings.kick_cheaters
            && let Some(cheater_to_kick) =
                self.find_player_in_team(&lobby, team, PlayerAttribute::Cheater)
        {
            return Some(cheater_to_kick);
        }

        None
    }

    fn find_player_in_team(
        &self,
        lobby: &Lobby,
        team: Team,
        player_attribute: PlayerAttribute,
    ) -> Option<Player> {
        let candidates: Vec<&Player> = lobby
            .players
            .iter()
            .filter(|player| Self::ok_to_kick(player, team, player_attribute))
            .collect();

        candidates.first().map(|p| (*p).clone())
    }

    fn ok_to_kick(player: &Player, team: Team, player_attribute: PlayerAttribute) -> bool {
        if player.team != team {
            return false;
        }

        if let Some(player_info) = &player.player_info
            && player_info.attributes.contains(&player_attribute)
        {
            return true;
        }

        false
    }

    fn send_notifications(&mut self) {
        let lobby = self.shared_lobby.get();
        if self.last_lobby_id != lobby.lobby_id {
            log::info!("New lobby detected: {}", lobby.lobby_id);
            self.local_notifications_seen.clear();
            self.tf2bd_notifications_sent.clear();
            self.last_lobby_id = lobby.lobby_id.clone();
        }

        let matches = self
            .app_settings
            .notify_tf2bd
            .then(|| self.tf2bd_matches.snapshot());

        for player in &lobby.players {
            let local_flags = if !self.local_notifications_seen.contains(&player.steamid)
                && let Some(info) = self.ruleset_handler.get_player_marking(&player.steamid)
            {
                self.local_notifications_seen.insert(player.steamid);
                info.attributes
                    .iter()
                    .any(|attr| self.app_settings.party_notifications_for.contains(attr))
                    .then_some(info.attributes.as_slice())
            } else {
                None
            };

            let source_matches = if let Some(matches) = &matches
                && !self.tf2bd_notifications_sent.contains(&player.steamid)
            {
                let sources: Vec<_> = matches
                    .sources_for(player.steamid)
                    .map(|(title, _, attributes)| (title, attributes))
                    .collect();
                if !sources.is_empty() {
                    self.tf2bd_notifications_sent.insert(player.steamid);
                }
                sources
            } else {
                Vec::new()
            };

            if let Some(command) = party_notification(&player.name, local_flags, &source_matches) {
                self.bus.lock().unwrap().send_rcon_cmd(&command);
            }
        }
    }
}

fn party_notification(
    player_name: &str,
    local_flags: Option<&[PlayerAttribute]>,
    source_matches: &[(&str, Vec<PlayerAttribute>)],
) -> Option<String> {
    if local_flags.is_none() && source_matches.is_empty() {
        return None;
    }
    let mut message = format!("Player {player_name}");
    if !source_matches.is_empty() {
        let sources = source_matches
            .iter()
            .map(|(title, attributes)| {
                let title = title_without_date(title);
                let labels = attributes
                    .iter()
                    .map(|&attribute| player_attribute_description(attribute).1)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{title} ({labels})")
            })
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!(" matched TF2BD: {sources}"));
    }
    if let Some(flags) = local_flags {
        if !source_matches.is_empty() {
            message.push_str(" and");
        }
        message.push_str(&format!(" is marked as {flags:?}"));
    }

    // Names come from the game server and remote lists; keep them inside one chat command.
    let message = message.replace(
        |c: char| c.is_control() || matches!(c, '"' | ';' | '\\' | '\u{2028}' | '\u{2029}'),
        " ",
    );
    Some(format!("say_party \"{message}\""))
}

fn title_without_date(title: &str) -> &str {
    if let Some((name, suffix)) = title.rsplit_once(" - ") {
        let suffix = suffix.trim();
        if chrono::NaiveDate::parse_from_str(suffix, "%Y-%m-%d").is_ok()
            || chrono::NaiveDateTime::parse_from_str(suffix, "%Y-%m-%d %H:%M:%S%.f").is_ok()
            || chrono::DateTime::parse_from_rfc3339(suffix).is_ok()
        {
            return name;
        }
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_notifications_include_matching_attributes_per_list_without_timestamp_suffixes() {
        use PlayerAttribute::{Bot, Cheater, Suspicious};

        assert_eq!(
            party_notification(
                "Example",
                None,
                &[
                    ("List A - 2026-11-18 11:28:14", vec![Cheater, Suspicious]),
                    ("List B - EU", vec![Bot]),
                ],
            )
            .as_deref(),
            Some(
                r#"say_party "Player Example matched TF2BD: List A (Cheater, Suspicious), List B - EU (Bot)""#
            ),
        );
        assert_eq!(
            party_notification(
                "Example",
                Some(&[Bot]),
                &[("List A - 2026-11-18", vec![Cheater])]
            )
            .as_deref(),
            Some(
                r#"say_party "Player Example matched TF2BD: List A (Cheater) and is marked as [Bot]""#
            ),
        );
        assert_eq!(party_notification("Example", None, &[]), None);
    }

    #[test]
    fn party_notification_keeps_untrusted_names_inside_the_chat_message() {
        assert_eq!(
            party_notification(
                "Example\";\n",
                None,
                &[("List\\A\r", vec![PlayerAttribute::Bot])]
            )
            .as_deref(),
            Some(r#"say_party "Player Example    matched TF2BD: List A  (Bot)""#),
        );
    }
}
