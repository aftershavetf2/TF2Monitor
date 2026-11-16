use crate::{
    ban_data::{self, models::ExportBan, storage},
    models::steamid::SteamID,
};
use chrono::Utc;
use diesel::{Connection, SqliteConnection};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub mod etf2l;

#[derive(Debug, Clone)]
pub struct SourceBan {
    pub source: String,
    pub record: ExportBan,
}

#[derive(Debug, Clone)]
pub struct Reputation {
    pub steamid: SteamID,
    pub source_bans: Vec<SourceBan>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Coverage metadata is part of the lookup result and verified by tests"
        )
    )]
    pub complete: bool,
}

const SOURCEBAN_WARNING_KEYWORDS: &[&str] = &[
    "smac",
    "aimbot",
    "exploit",
    "crash",
    "cheat",
    "hack",
    "wallhack",
    "speedhack",
    "triggerbot",
];

impl Reputation {
    /// Match the reason across the entire history, regardless of ban status.
    pub fn has_warning_ban(&self) -> bool {
        self.source_bans.iter().any(|ban| {
            ban.record.reason.as_ref().is_some_and(|reason| {
                let reason = reason.to_ascii_lowercase();
                SOURCEBAN_WARNING_KEYWORDS
                    .iter()
                    .any(|keyword| reason.contains(keyword))
            })
        })
    }
}

fn lookup(conn: &mut SqliteConnection, steamid: SteamID) -> ban_data::Result<Option<Reputation>> {
    conn.transaction::<_, Box<dyn std::error::Error + Send + Sync>, _>(|conn| {
        let Some(complete) = storage::coverage(conn)? else {
            return Ok(None);
        };
        let source_bans = storage::lookup(conn, steamid.to_u64() as i64)?
            .into_iter()
            .map(|row| {
                Ok(SourceBan {
                    source: row.source,
                    record: serde_json::from_str(&row.record)?,
                })
            })
            .collect::<ban_data::Result<Vec<_>>>()?;
        Ok(Some(Reputation {
            steamid,
            source_bans,
            complete,
        }))
    })
}

/// Local lookups run independently of network checks and imports.
pub fn start(
    bus: &Arc<Mutex<crate::appbus::AppBus>>,
    path: PathBuf,
    control: ban_data::Control,
) -> ban_data::Result<()> {
    let bus = Arc::clone(bus);
    let lobby = bus.lock().unwrap().shared_lobby.clone();
    let mut conn = storage::open(&path)?;
    std::thread::Builder::new()
        .name("community-reputation".into())
        .spawn(move || {
            let mut sent = HashSet::new();
            let mut generation = (control.revision(), Utc::now().date_naive());
            loop {
                let current = (control.revision(), Utc::now().date_naive());
                if current != generation {
                    sent.clear();
                    generation = current;
                }
                let players = lobby.get().players;
                let present: HashSet<_> = players.iter().map(|player| player.steamid).collect();
                sent.retain(|steamid| present.contains(steamid));
                for player in players {
                    if sent.contains(&player.steamid) && player.reputation.is_some() {
                        continue;
                    }
                    match lookup(&mut conn, player.steamid) {
                        Ok(Some(reputation)) => {
                            bus.lock().unwrap().steamapi_bus.broadcast(
                                crate::tf2::steamapi::SteamApiMsg::Reputation(reputation),
                            );
                            sent.insert(player.steamid);
                        }
                        Ok(None) => {}
                        Err(error) => log::warn!("Local ban lookup failed: {error}"),
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ban_data::tests::{ban, community};

    #[test]
    fn warning_keywords_match_any_ban_reason_case_insensitively_including_history() {
        let mut reputation = Reputation {
            steamid: SteamID::from_u64(76561197960265729),
            source_bans: vec![],
            complete: true,
        };
        assert!(!reputation.has_warning_ban());
        let mut record = ban();
        record.reason = Some("Spam".into());
        reputation.source_bans.push(SourceBan {
            source: "SMAC community".into(),
            record,
        });
        assert!(!reputation.has_warning_ban()); // Match reasons, not community names.
        let mut historical = ban();
        historical.expires_at = Some("2020-01-01".into());
        historical.removed_at = Some("2020-01-01".into());
        reputation.source_bans.push(SourceBan {
            source: "Example".into(),
            record: historical,
        });
        for reason in [
            "SMAC 0.8 detection",
            "Suspected AiMbOt",
            "Map EXPLOITing",
            "Server CrAsH attempt",
        ] {
            reputation.source_bans[1].record.reason = Some(reason.into());
            assert!(reputation.has_warning_ban(), "{reason}");
        }
        for reason in [None, Some(""), Some("Spam")] {
            reputation.source_bans[1].record.reason = reason.map(str::to_owned);
            assert!(!reputation.has_warning_ban());
        }
    }

    #[test]
    fn local_lookup_keeps_active_expired_and_removed_bans() {
        let mut conn = storage::open(std::path::Path::new(":memory:")).unwrap();
        let mut record = ban();
        let steamid = SteamID::from_u64(record.steam_id().unwrap() as u64);
        assert!(lookup(&mut conn, steamid).unwrap().is_none());
        storage::expect_communities(&mut conn, &[community()]).unwrap();
        storage::import(&mut conn, &community(), &[record.clone()]).unwrap();
        let rep = lookup(&mut conn, steamid).unwrap().unwrap();
        assert!(!rep.source_bans.is_empty());
        assert!(rep.complete);
        record.expires_at = Some("2020-01-01".into());
        storage::import(&mut conn, &community(), &[record.clone()]).unwrap();
        let rep = lookup(&mut conn, steamid).unwrap().unwrap();
        assert_eq!(rep.source_bans.len(), 1);
        assert_eq!(
            rep.source_bans[0].record.expires_at.as_deref(),
            Some("2020-01-01")
        );
        record.removed_at = Some("2026-11-01".into());
        storage::import(&mut conn, &community(), &[record]).unwrap();
        let rep = lookup(&mut conn, steamid).unwrap().unwrap();
        assert_eq!(rep.source_bans.len(), 1);
        let empty = lookup(&mut conn, SteamID::from_u64(76561197960265730))
            .unwrap()
            .unwrap();
        assert!(empty.source_bans.is_empty());
    }
}
