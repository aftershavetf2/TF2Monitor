use super::{Tf2bdMsg, models::PlayerAttribute, player_lists::SharedMatches};
use crate::config::TF2BD_LOOP_DELAY;
use crate::models::player_markings::PlayerMarkings;
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

const VOTE_PERIOD_SECONDS: u64 = 10;

pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let settings = settings.clone();
    let bus = Arc::clone(bus);
    thread::spawn(move || Tf2bdThread::new(&settings, &bus).run())
}

struct Tf2bdThread {
    bus: Arc<Mutex<AppBus>>,
    shared_lobby: crate::tf2::lobby::shared_lobby::SharedLobby,
    app_event_bus_rx: BusReader<AppEventMsg>,

    app_settings: AppSettings,

    markings: Arc<PlayerMarkings>,

    last_lobby_id: String,
    last_vote_time: Instant,

    local_notifications_seen: HashSet<SteamID>,
    tf2bd_matches: SharedMatches,
    tf2bd_notifications_sent: HashSet<SteamID>,
    sourceban_notifications_sent: HashSet<SteamID>,
}

impl Tf2bdThread {
    fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let mut events = bus.lock().unwrap();
        Self {
            bus: Arc::clone(bus),
            shared_lobby: events.shared_lobby.clone(),
            app_event_bus_rx: events.app_event_bus.add_rx(),
            tf2bd_matches: events.tf2bd_matches.clone(),
            app_settings: settings.clone(),
            markings: events.markings.clone(),

            last_lobby_id: String::new(),

            last_vote_time: Instant::now(),

            local_notifications_seen: HashSet::new(),
            tf2bd_notifications_sent: HashSet::new(),
            sourceban_notifications_sent: HashSet::new(),
        }
    }

    pub fn run(&mut self) {
        log::info!("TF2BD background thread started");

        loop {
            self.process_app_event_bus();

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
        while let Ok(AppEventMsg::UpdatedSettings(settings)) = self.app_event_bus_rx.try_recv() {
            self.app_settings = settings;
        }
    }

    fn apply_rules_to_lobby(&mut self) {
        let lobby = self.shared_lobby.get();
        for player in &lobby.players {
            let data = self.markings.player_info(player.steamid);
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
            self.sourceban_notifications_sent.clear();
            self.last_lobby_id = lobby.lobby_id.clone();
        }

        let matches = self
            .app_settings
            .notify_tf2bd
            .then(|| self.tf2bd_matches.snapshot());

        let markings = self.markings.snapshot();
        for player in &lobby.players {
            let local_flags = if !self.local_notifications_seen.contains(&player.steamid)
                && let Some(info) = markings.get(&player.steamid.to_u64())
                && !info.flags.is_empty()
            {
                self.local_notifications_seen.insert(player.steamid);
                info.flags
                    .iter()
                    .any(|attr| self.app_settings.party_notifications_for.contains(attr))
                    .then_some(info.flags.as_slice())
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

            let sourceban_warning = self.take_sourceban_warning(player);
            if let Some(command) = party_notification(
                &player.name,
                local_flags,
                &source_matches,
                sourceban_warning,
            ) {
                self.bus.lock().unwrap().send_rcon_cmd(&command);
            }
        }
    }

    fn take_sourceban_warning<'a>(&mut self, player: &'a Player) -> Option<(&'a str, &'a str)> {
        if !self.app_settings.notify_sourcebans
            || self.sourceban_notifications_sent.contains(&player.steamid)
        {
            return None;
        }
        let warning = player.reputation.as_ref()?.first_warning_ban()?;
        self.sourceban_notifications_sent.insert(player.steamid);
        Some(warning)
    }
}

fn party_notification(
    player_name: &str,
    local_flags: Option<&[PlayerAttribute]>,
    source_matches: &[(&str, Vec<PlayerAttribute>)],
    sourceban_warning: Option<(&str, &str)>,
) -> Option<String> {
    if local_flags.is_none() && source_matches.is_empty() && sourceban_warning.is_none() {
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
    if let Some((reason, community)) = sourceban_warning {
        if local_flags.is_some() || !source_matches.is_empty() {
            message.push_str(" and");
        }
        message.push_str(&format!(" has SourceBans: {reason} on {community}"));
    }

    // Keep player names, list names, and ban details inside one chat command.
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
                None,
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
                &[("List A - 2026-11-18", vec![Cheater])],
                None,
            )
            .as_deref(),
            Some(
                r#"say_party "Player Example matched TF2BD: List A (Cheater) and is marked as [Bot]""#
            ),
        );
        assert_eq!(party_notification("Example", None, &[], None), None);
    }

    #[test]
    fn party_notification_keeps_untrusted_names_and_ban_details_inside_the_chat_message() {
        assert_eq!(
            party_notification(
                "Example\";\n",
                None,
                &[("List\\A\r", vec![PlayerAttribute::Bot])],
                Some(("aimbotting\n", "Skial\";")),
            )
            .as_deref(),
            Some(
                r#"say_party "Player Example    matched TF2BD: List A  (Bot) and has SourceBans: aimbotting  on Skial  ""#
            ),
        );
    }

    #[test]
    fn sourceban_notification_waits_for_a_warning_and_sends_once_per_lobby() {
        let bus = Arc::new(Mutex::new(AppBus::default()));
        let mut commands = bus.lock().unwrap().rcon_bus.add_rx();
        let mut thread = Tf2bdThread::new(&AppSettings::default(), &bus);
        let steamid = SteamID::from_u64(76561197960265729);
        let mut lobby = thread.shared_lobby.get();
        lobby.players.push(Player {
            steamid,
            name: "Example".into(),
            ..Player::default()
        });
        let mut record = crate::ban_data::tests::ban();
        record.reason = Some("aimbotting".into());
        let reputation = crate::reputation::Reputation {
            steamid,
            source_bans: vec![crate::reputation::SourceBan {
                source: "Skial".into(),
                record,
            }],
            complete: true,
        };
        lobby.players[0].reputation = Some(reputation.clone());
        thread.shared_lobby.set(lobby.clone());
        thread.send_notifications();
        assert!(commands.try_recv().is_err());

        let settings = AppSettings {
            notify_sourcebans: true,
            ..AppSettings::default()
        };
        bus.lock()
            .unwrap()
            .app_event_bus
            .broadcast(AppEventMsg::UpdatedSettings(settings));
        thread.process_app_event_bus();
        lobby.players[0].reputation = None;
        thread.shared_lobby.set(lobby.clone());
        thread.send_notifications();
        assert!(commands.try_recv().is_err());

        lobby.players[0].reputation = Some(reputation);
        thread.shared_lobby.set(lobby.clone());
        thread.send_notifications();
        let expected = r#"say_party "Player Example has SourceBans: aimbotting on Skial""#;
        assert_eq!(commands.try_recv().unwrap(), expected);
        thread.send_notifications();
        assert!(commands.try_recv().is_err());

        lobby.lobby_id = "next lobby".into();
        thread.shared_lobby.set(lobby);
        thread.send_notifications();
        assert_eq!(commands.try_recv().unwrap(), expected);
    }
}
