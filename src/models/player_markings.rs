use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use diesel::{
    dsl::{select, sql},
    prelude::*,
    sql_types::Bool,
};
use serde::{Deserialize, Serialize};

use super::steamid::{MIN_STEAMID64, SteamID};
use crate::{
    db::{
        connection::DbPool,
        schema::{account, player_flags, player_notes},
    },
    tf2bd::models::{PlayerAttribute, PlayerInfo, PlayerLastSeen, TF2BDPlayerList},
};

pub const FILENAME: &str = "player-markings.json";
const LEGACY_FILENAME: &str = "playerlist.json";
const VERSION: u32 = 1;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerMarking {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<PlayerAttribute>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<PlayerLastSeen>,
}

impl PlayerMarking {
    fn is_empty(&self) -> bool {
        self.flags.is_empty() && self.note.is_empty()
    }

    fn merge(&mut self, incoming: Self) {
        for flag in incoming.flags {
            if !self.flags.contains(&flag) {
                self.flags.push(flag);
            }
        }
        if self.note.is_empty() {
            self.note = incoming.note;
        }
        if let Some(last_seen) = incoming.last_seen
            && self
                .last_seen
                .as_ref()
                .is_none_or(|current| last_seen.time > current.time)
        {
            self.last_seen = Some(last_seen);
        }
    }
}

pub type MarkingsSnapshot = BTreeMap<u64, PlayerMarking>;

#[derive(Serialize, Deserialize)]
struct Document<Players> {
    version: u32,
    players: Players,
}

#[derive(Default)]
pub struct PlayerMarkings {
    path: Option<PathBuf>,
    _lock: Option<File>,
    writer: Mutex<()>,
    players: RwLock<Arc<MarkingsSnapshot>>,
}

impl PlayerMarkings {
    pub fn open(path: &Path, db: &DbPool) -> Result<Self> {
        // Hold a process lock for the lifetime of the in-memory copy.
        let lock_path = path.with_extension("lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.try_lock().map_err(|error| {
            io::Error::other(format!(
                "Could not lock {}. Close any other instance using this file. {error}",
                path.display()
            ))
        })?;

        let players = match fs::read(path) {
            Ok(bytes) => parse(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let legacy = path.with_file_name(LEGACY_FILENAME);
                let mut conn = db.get()?;
                let players = migrate(&mut conn, &legacy)?;
                write_json(path, &players, true)?;
                players
            }
            Err(error) => return Err(error.into()),
        };

        Ok(Self {
            path: Some(path.to_owned()),
            _lock: Some(lock),
            writer: Mutex::default(),
            players: RwLock::new(Arc::new(players)),
        })
    }

    pub fn snapshot(&self) -> Arc<MarkingsSnapshot> {
        // Readers only hold this lock while cloning the snapshot pointer.
        Arc::clone(&self.players.read().expect("Marking snapshot lock poisoned"))
    }

    pub fn player_info(&self, steamid: SteamID) -> Option<PlayerInfo> {
        let snapshot = self.snapshot();
        let marking = snapshot.get(&steamid.to_u64())?;
        if marking.flags.is_empty() {
            return None;
        }
        Some(PlayerInfo {
            steamid32: steamid.to_steam_id32(),
            attributes: marking.flags.clone(),
            last_seen: marking.last_seen.clone(),
        })
    }

    pub fn set_flag(
        &self,
        steamid: SteamID,
        name: &str,
        flag: PlayerAttribute,
        enable: bool,
    ) -> Result<()> {
        self.edit(steamid, |marking| {
            if enable && !marking.flags.contains(&flag) {
                marking.flags.push(flag);
                marking.last_seen = Some(PlayerLastSeen {
                    player_name: (!name.is_empty()).then(|| name.to_owned()),
                    time: chrono::Utc::now().timestamp(),
                });
            } else if !enable {
                marking.flags.retain(|value| *value != flag);
            }
        })
    }

    pub fn save_note(&self, steamid: SteamID, note: String) -> Result<()> {
        self.edit(steamid, |marking| marking.note = note)
    }

    fn edit(&self, steamid: SteamID, edit: impl FnOnce(&mut PlayerMarking)) -> Result<()> {
        if !steamid.is_valid() {
            return Err("Invalid SteamID.".into());
        }
        self.update(|players| {
            let id = steamid.to_u64();
            let marking = players.entry(id).or_default();
            edit(marking);
            if marking.is_empty() {
                players.remove(&id);
            }
        })
    }

    fn update(&self, edit: impl FnOnce(&mut MarkingsSnapshot)) -> Result<()> {
        let _writer = self
            .writer
            .lock()
            .map_err(|_| "Marking writer lock poisoned.")?;
        let current = self.snapshot();
        let mut players = (*current).clone();
        edit(&mut players);
        if players == *current {
            return Ok(());
        }
        if let Some(path) = &self.path {
            write_json(path, &players, true)?;
        }
        *self
            .players
            .write()
            .expect("Marking snapshot lock poisoned") = Arc::new(players);
        Ok(())
    }

    pub fn import(&self, path: &Path, replace: bool) -> Result<()> {
        let incoming = parse(&fs::read(path)?)?;
        self.update(|players| {
            if replace {
                *players = incoming;
            } else {
                for (id, marking) in incoming {
                    players.entry(id).or_default().merge(marking);
                }
            }
        })
    }

    pub fn export(&self, path: &Path) -> Result<()> {
        write_json(path, &self.snapshot(), false)
    }
}

fn parse(bytes: &[u8]) -> Result<MarkingsSnapshot> {
    let mut document: Document<MarkingsSnapshot> = serde_json::from_slice(bytes)?;
    if document.version != VERSION {
        return Err(format!("Unsupported player markings version {}.", document.version).into());
    }
    for (&id, marking) in &mut document.players {
        if !SteamID::from_u64(id).is_valid() {
            return Err(format!("Invalid SteamID {id} in player markings.").into());
        }
        let mut flags = Vec::new();
        for flag in marking.flags.drain(..) {
            if !flags.contains(&flag) {
                flags.push(flag);
            }
        }
        marking.flags = flags;
    }
    document.players.retain(|_, marking| !marking.is_empty());
    Ok(document.players)
}

fn migrate(conn: &mut SqliteConnection, legacy: &Path) -> Result<MarkingsSnapshot> {
    conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
        let mut players = BTreeMap::<u64, PlayerMarking>::new();
        if table_exists(conn, "player_flags")? {
            let flags = player_flags::table
                .left_join(account::table)
                .select((
                    player_flags::steam_id,
                    player_flags::flag_type,
                    player_flags::last_seen,
                    account::name.nullable(),
                ))
                .load::<(i64, String, i64, Option<String>)>(conn)?;
            for (id, flag, time, name) in flags {
                let flag = match flag.as_str() {
                    "Cheater" => PlayerAttribute::Cheater,
                    "Bot" => PlayerAttribute::Bot,
                    "Suspicious" => PlayerAttribute::Suspicious,
                    "Toxic" | "Racist" => PlayerAttribute::Toxic,
                    "Exploiter" => PlayerAttribute::Exploiter,
                    "Cool" => PlayerAttribute::Cool,
                    _ => {
                        return Err(
                            format!("Unknown legacy marking {flag} for SteamID {id}.").into()
                        );
                    }
                };
                legacy_entry(&mut players, id)?.merge(PlayerMarking {
                    flags: vec![flag],
                    last_seen: Some(PlayerLastSeen {
                        player_name: name,
                        time,
                    }),
                    ..Default::default()
                });
            }
        }
        if table_exists(conn, "player_notes")? {
            for (id, note) in player_notes::table
                .select((player_notes::steam_id, player_notes::note))
                .load::<(i64, String)>(conn)?
            {
                legacy_entry(&mut players, id)?.note = note;
            }
        }

        // Once imported into SQLite, playerlist.json was only an export and could be stale.
        let already_imported = table_exists(conn, "player_markings_metadata")?
            && select(sql::<Bool>(
                "EXISTS(SELECT 1 FROM player_markings_metadata WHERE id = 1)",
            ))
            .get_result::<bool>(conn)?;
        if !already_imported {
            match fs::read(legacy) {
                Ok(bytes) => merge_legacy_list(&mut players, &bytes)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        players.retain(|_, marking| !marking.is_empty());
        Ok(players)
    })
}

fn table_exists(conn: &mut SqliteConnection, name: &str) -> QueryResult<bool> {
    use diesel::sql_types::Text;
    select(
        sql::<Bool>("EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ")
            .bind::<Text, _>(name)
            .sql(")"),
    )
    .get_result(conn)
}

fn legacy_entry(players: &mut MarkingsSnapshot, id: i64) -> Result<&mut PlayerMarking> {
    if id < MIN_STEAMID64 as i64 {
        return Err(format!("Invalid legacy SteamID {id}.").into());
    }
    Ok(players.entry(id as u64).or_default())
}

fn merge_legacy_list(players: &mut MarkingsSnapshot, bytes: &[u8]) -> Result<()> {
    let list: TF2BDPlayerList = serde_json::from_slice(bytes)?;
    for player in list.players {
        let raw = player
            .steamid32
            .strip_prefix("[U:1:")
            .and_then(|id| id.strip_suffix(']'))
            .unwrap_or(&player.steamid32);
        let id = MIN_STEAMID64 + u64::from(raw.parse::<u32>()?);
        players.entry(id).or_default().merge(PlayerMarking {
            flags: player.attributes,
            last_seen: player.last_seen,
            ..Default::default()
        });
    }
    Ok(())
}

fn write_json(path: &Path, players: &MarkingsSnapshot, replace: bool) -> Result<()> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (temporary, mut file) = loop {
        let name = format!(
            ".tf2monitor-markings-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        );
        let temporary = directory.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    let result = (|| -> Result<()> {
        serde_json::to_writer_pretty(
            &mut file,
            &Document {
                version: VERSION,
                players,
            },
        )?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        if replace {
            fs::rename(&temporary, path)?;
        } else {
            // Publish atomically without overwriting a file at the selected path.
            fs::hard_link(&temporary, path)?;
        }
        Ok(())
    })();
    if let Err(error) = fs::remove_file(&temporary)
        && error.kind() != io::ErrorKind::NotFound
    {
        log::warn!("Could not remove {}: {error}", temporary.display());
    }
    #[cfg(unix)]
    if result.is_ok()
        && let Err(error) = File::open(directory).and_then(|directory| directory.sync_all())
    {
        log::warn!("Could not sync {}: {error}", directory.display());
    }
    result
}

pub type SaveResult = std::result::Result<(), String>;

pub enum MarkingCommand {
    SetFlag {
        steamid: SteamID,
        name: String,
        flag: PlayerAttribute,
        enable: bool,
    },
    SaveNote {
        steamid: SteamID,
        note: String,
        reply: std::sync::mpsc::Sender<SaveResult>,
    },
    Transfer {
        path: PathBuf,
        operation: TransferOperation,
        reply: std::sync::mpsc::Sender<SaveResult>,
    },
    Stop,
}

#[derive(Clone, Copy)]
pub enum TransferOperation {
    Import { replace: bool },
    Export,
}

pub fn start(
    db: &DbPool,
    bus: &Arc<Mutex<crate::appbus::AppBus>>,
) -> Result<(
    std::sync::mpsc::Sender<MarkingCommand>,
    std::thread::JoinHandle<()>,
)> {
    let markings = Arc::new(PlayerMarkings::open(Path::new(FILENAME), db)?);
    let (sender, receiver) = std::sync::mpsc::channel();
    {
        let mut events = bus.lock().expect("App bus lock poisoned");
        events.markings = markings.clone();
        events.marking_commands = Some(sender.clone());
    }
    let bus = Arc::clone(bus);
    let thread = std::thread::spawn(move || {
        for command in receiver {
            match command {
                MarkingCommand::SetFlag {
                    steamid,
                    name,
                    flag,
                    enable,
                } => {
                    if let Err(error) = markings.set_flag(steamid, &name, flag, enable) {
                        let message = format!("Could not save player marking: {error}");
                        log::error!("{message}");
                        bus.lock()
                            .expect("App bus lock poisoned")
                            .profile_events
                            .broadcast(super::player_profile::ProfileEvent::MarkingError(message));
                    }
                }
                MarkingCommand::SaveNote {
                    steamid,
                    note,
                    reply,
                } => {
                    let result = markings.save_note(steamid, note).map_err(|error| {
                        log::error!("Could not save note for {}: {error}", steamid.to_u64());
                        error.to_string()
                    });
                    let _ = reply.send(result);
                }
                MarkingCommand::Transfer {
                    path,
                    operation,
                    reply,
                } => {
                    let result = match operation {
                        TransferOperation::Import { replace } => markings.import(&path, replace),
                        TransferOperation::Export => markings.export(&path),
                    };
                    let _ = reply.send(result.map_err(|error| error.to_string()));
                }
                MarkingCommand::Stop => break,
            }
        }
    });
    Ok((sender, thread))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{connection::SimpleConnection, r2d2::ConnectionManager};

    struct Fixture {
        directory: PathBuf,
        db: DbPool,
    }

    impl Fixture {
        fn new() -> Self {
            static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
            let directory = std::env::temp_dir().join(format!(
                "tf2monitor-markings-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            let db = DbPool::builder()
                .max_size(1)
                .build(ConnectionManager::new(":memory:"))
                .unwrap();
            crate::db::connection::setup_schema(&mut db.get().unwrap()).unwrap();
            Self { directory, db }
        }

        fn path(&self) -> PathBuf {
            self.directory.join(FILENAME)
        }

        fn open(&self) -> PlayerMarkings {
            PlayerMarkings::open(&self.path(), &self.db).unwrap()
        }

        fn old_database(&self, imported: bool) {
            let mut conn = self.db.get().unwrap();
            conn.batch_execute(
                "CREATE TABLE player_flags (
                    steam_id INTEGER NOT NULL,
                    flag_type TEXT NOT NULL,
                    source TEXT NOT NULL,
                    first_seen INTEGER NOT NULL,
                    last_seen INTEGER NOT NULL,
                    notified INTEGER NOT NULL,
                    PRIMARY KEY (steam_id, flag_type, source)
                 );
                 INSERT INTO player_flags VALUES
                    (76561197960265729, 'Racist', 'playerlist.json', 10, 10, 0),
                    (76561197960265729, 'Toxic', 'manual', 20, 20, 0);
                 CREATE TABLE player_notes (
                    steam_id INTEGER PRIMARY KEY NOT NULL,
                    note TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                 );
                 INSERT INTO player_notes VALUES (76561197960265730, 'Line one
🎮 Line two', 30);",
            )
            .unwrap();
            if imported {
                conn.batch_execute(
                    "CREATE TABLE player_markings_metadata (id INTEGER PRIMARY KEY, template TEXT);
                     INSERT INTO player_markings_metadata VALUES (1, '{}');",
                )
                .unwrap();
            }
        }

        fn old_list(&self) {
            fs::write(
                self.directory.join(LEGACY_FILENAME),
                br#"{"$schema":"","players":[{"steamid":"[U:1:3]","attributes":["cheater"]}]}"#,
            )
            .unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    fn id(value: u64) -> SteamID {
        SteamID::from_u64(MIN_STEAMID64 + value)
    }

    #[test]
    fn migrates_sqlite_notes_and_flags_without_reviving_stale_json() {
        let fixture = Fixture::new();
        fixture.old_database(true);
        fixture.old_list();
        let store = fixture.open();
        let players = store.snapshot();
        assert_eq!(players.len(), 2);
        let flagged = &players[&id(1).to_u64()];
        assert_eq!(flagged.flags, [PlayerAttribute::Toxic]);
        assert_eq!(flagged.last_seen.as_ref().unwrap().time, 20);
        let note_only = &players[&id(2).to_u64()];
        assert_eq!(note_only.note, "Line one\n🎮 Line two");
        assert!(note_only.flags.is_empty());

        assert!(PlayerMarkings::open(&fixture.path(), &fixture.db).is_err());
        store
            .set_flag(id(1), "", PlayerAttribute::Toxic, false)
            .unwrap();
        store.save_note(id(2), "Edited".into()).unwrap();
        drop(store);

        let store = fixture.open();
        assert!(store.player_info(id(1)).is_none());
        assert_eq!(store.snapshot()[&id(2).to_u64()].note, "Edited");
        store.save_note(id(2), String::new()).unwrap();
        drop(store);
        assert!(fixture.open().snapshot().is_empty());

        let mut conn = fixture.db.get().unwrap();
        assert_eq!(
            player_flags::table
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap(),
            2
        );
        assert_eq!(
            player_notes::table
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap(),
            1
        );
    }

    #[test]
    fn merges_unimported_legacy_playerlist_and_retries_failed_migration() {
        let fixture = Fixture::new();
        fixture.old_database(false);
        let legacy = fixture.directory.join(LEGACY_FILENAME);
        fs::write(&legacy, b"truncated json").unwrap();
        assert!(PlayerMarkings::open(&fixture.path(), &fixture.db).is_err());
        assert!(!fixture.path().exists());

        fixture.old_list();
        let store = fixture.open();
        assert_eq!(store.snapshot().len(), 3);
        assert_eq!(
            store.player_info(id(3)).unwrap().attributes,
            [PlayerAttribute::Cheater]
        );
    }

    #[test]
    fn failed_save_leaves_the_last_committed_snapshot_and_can_be_retried() {
        let fixture = Fixture::new();
        let store = fixture.open();
        store.save_note(id(1), "Saved".into()).unwrap();
        let original = fs::read(fixture.path()).unwrap();
        let backup = fixture.directory.join("saved.json");
        fs::rename(fixture.path(), &backup).unwrap();
        fs::create_dir(fixture.path()).unwrap();

        assert!(store.save_note(id(1), "Edited".into()).is_err());
        assert_eq!(store.snapshot()[&id(1).to_u64()].note, "Saved");
        assert_eq!(fs::read(&backup).unwrap(), original);

        fs::remove_dir(fixture.path()).unwrap();
        fs::rename(&backup, fixture.path()).unwrap();
        store.save_note(id(1), "Edited".into()).unwrap();
        drop(store);
        assert_eq!(fixture.open().snapshot()[&id(1).to_u64()].note, "Edited");
    }

    #[test]
    fn import_merges_by_default_and_replace_is_explicit() {
        let fixture = Fixture::new();
        let store = fixture.open();
        store.save_note(id(1), "Existing note".into()).unwrap();
        store.save_note(id(2), "Unlisted player".into()).unwrap();
        store
            .set_flag(id(1), "Player", PlayerAttribute::Cool, true)
            .unwrap();
        let other = PlayerMarkings::default();
        other
            .set_flag(id(1), "Player", PlayerAttribute::Bot, true)
            .unwrap();
        other.save_note(id(1), "Incoming note".into()).unwrap();
        other.save_note(id(3), "New note".into()).unwrap();
        let path = fixture.directory.join("export.json");
        other.export(&path).unwrap();
        assert!(store.export(&path).is_err());
        let invalid = fixture.directory.join("invalid.json");
        fs::write(&invalid, br#"{"version":99,"players":{}}"#).unwrap();
        assert!(store.import(&invalid, true).is_err());

        store.import(&path, false).unwrap();
        let merged = store.snapshot();
        assert_eq!(
            merged[&id(1).to_u64()].flags,
            [PlayerAttribute::Cool, PlayerAttribute::Bot]
        );
        assert_eq!(merged[&id(1).to_u64()].note, "Existing note");
        assert_eq!(merged[&id(2).to_u64()].note, "Unlisted player");
        assert_eq!(merged[&id(3).to_u64()].note, "New note");

        store.import(&path, true).unwrap();
        drop(store);
        assert!(*fixture.open().snapshot() == *other.snapshot());
    }
}
