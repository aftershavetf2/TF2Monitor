use super::models::PlayerAttribute;
use crate::models::steamid::{MIN_STEAMID64, SteamID};
use chrono::Utc;
use reqwest::{Url, blocking::Client};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

const DIRECTORY: &str = "data/tf2bd";
const SETTINGS_FILE: &str = "sources.json";
const MAX_LIST_BYTES: u64 = 64 * 1024 * 1024;
const UPDATE_INTERVAL_SECONDS: i64 = 24 * 60 * 60;
const AUTO_CHECK_POLL_INTERVAL: Duration = Duration::from_secs(60);

pub const ATTRIBUTES: [PlayerAttribute; 6] = [
    PlayerAttribute::Cheater,
    PlayerAttribute::Bot,
    PlayerAttribute::Suspicious,
    PlayerAttribute::Exploiter,
    PlayerAttribute::Toxic,
    PlayerAttribute::Cool,
];

#[derive(Clone, Serialize, Deserialize)]
struct Source {
    url: String,
    filename: String,
    enabled: bool,
    last_checked_at: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    filters: Vec<PlayerAttribute>,
    sources: Vec<Source>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            filters: vec![PlayerAttribute::Cheater, PlayerAttribute::Bot],
            sources: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct PlayerListFile {
    file_info: Option<FileInfo>,
    players: Vec<Entry>,
}

#[derive(Deserialize)]
struct FileInfo {
    title: Option<String>,
    update_url: Option<String>,
}

#[derive(Deserialize)]
struct Entry {
    steamid: ListSteamId,
    attributes: Vec<Attribute>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Attribute {
    Known(PlayerAttribute),
    Other(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ListSteamId {
    Text(String),
    Number(u64),
}

impl ListSteamId {
    fn steamid(&self) -> Result<SteamID, String> {
        let value = match self {
            Self::Text(text) => {
                let value = text
                    .strip_prefix("[U:1:")
                    .and_then(|id| id.strip_suffix(']'))
                    .unwrap_or(text);
                value
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid SteamID: {text}"))?
            }
            Self::Number(value) => *value,
        };
        let steamid64 = if u32::try_from(value).is_ok() {
            MIN_STEAMID64 + value
        } else if (MIN_STEAMID64..=MIN_STEAMID64 + u64::from(u32::MAX)).contains(&value) {
            value
        } else {
            return Err(format!("Invalid SteamID: {value}"));
        };
        Ok(SteamID::from_u64(steamid64))
    }
}

struct ParsedList {
    title: Option<String>,
    update_url: Option<String>,
    players: HashMap<SteamID, Vec<PlayerAttribute>>,
    counts: [usize; ATTRIBUTES.len()],
}

impl ParsedList {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        let file: PlayerListFile =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let mut players: HashMap<_, Vec<_>> = HashMap::new();
        let mut counts = [0; ATTRIBUTES.len()];
        for entry in file.players {
            let attributes = players.entry(entry.steamid.steamid()?).or_default();
            for attribute in entry.attributes {
                if let Attribute::Known(attribute) = attribute
                    && !attributes.contains(&attribute)
                {
                    attributes.push(attribute);
                    let index = ATTRIBUTES
                        .iter()
                        .position(|&item| item == attribute)
                        .unwrap();
                    counts[index] += 1;
                }
            }
        }
        let (title, update_url) = file.file_info.map_or((None, None), |info| {
            (
                info.title.filter(|title| !title.trim().is_empty()),
                info.update_url
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.trim().to_owned()),
            )
        });
        Ok(Self {
            title,
            update_url,
            players,
            counts,
        })
    }
}

pub struct ListSummary<'a> {
    pub title: &'a str,
    pub filename: &'a str,
    pub url: &'a str,
    pub enabled: bool,
    pub counts: Option<(usize, &'a [usize; ATTRIBUTES.len()])>,
    pub status: &'a str,
    pub failed: bool,
}

#[derive(Default)]
struct CachedList {
    data: Option<ParsedList>,
    status: String,
    failed: bool,
}

struct Download {
    bytes: Vec<u8>,
    data: ParsedList,
    filename: String,
}

enum Request {
    Add(Vec<String>),
    Update {
        sources: Vec<(usize, UpdateSource)>,
        automatic: bool,
    },
}

struct UpdateSource {
    import_url: String,
    update_url: Option<String>,
}

enum DownloadResult {
    Add(String, Result<Download, String>),
    BeginUpdate {
        index: usize,
        automatic: bool,
        ready: SyncSender<bool>,
    },
    Update(usize, Result<Download, String>),
}

#[derive(Default)]
pub struct ImportProgress {
    pub total: usize,
    pub added: usize,
    pub errors: Vec<String>,
}

#[derive(Default)]
pub struct ListMatches {
    by_player: HashMap<SteamID, Vec<usize>>,
    titles: Vec<String>,
}

impl ListMatches {
    pub fn titles_for(&self, steamid: SteamID) -> impl Iterator<Item = &str> {
        self.by_player
            .get(&steamid)
            .into_iter()
            .flatten()
            .map(|&index| self.titles[index].as_str())
    }
}

#[derive(Clone, Default)]
pub struct SharedMatches(Arc<Mutex<Arc<ListMatches>>>);

impl SharedMatches {
    pub fn snapshot(&self) -> Arc<ListMatches> {
        self.0.lock().unwrap().clone()
    }
}

pub struct PlayerLists {
    directory: PathBuf,
    settings: Settings,
    lists: Vec<CachedList>,
    matches: Arc<ListMatches>,
    shared_matches: SharedMatches,
    downloads: Option<Receiver<DownloadResult>>,
    next_auto_check: Instant,
    pub import: ImportProgress,
    pub load_error: Option<String>,
}

impl Default for PlayerLists {
    fn default() -> Self {
        Self {
            directory: PathBuf::from(DIRECTORY),
            settings: Settings::default(),
            lists: Vec::new(),
            matches: Arc::default(),
            shared_matches: SharedMatches::default(),
            downloads: None,
            next_auto_check: Instant::now(),
            import: ImportProgress::default(),
            load_error: None,
        }
    }
}

impl PlayerLists {
    pub fn load(shared_matches: SharedMatches) -> Self {
        Self::load_from(PathBuf::from(DIRECTORY), shared_matches)
    }

    fn load_from(directory: PathBuf, shared_matches: SharedMatches) -> Self {
        let mut manager = Self {
            directory,
            shared_matches,
            ..Self::default()
        };
        manager.settings = match manager.read_settings() {
            Ok(settings) => settings,
            Err(error) => {
                log::warn!("Could not load TF2BD list settings: {error}");
                manager.load_error = Some(error);
                return manager;
            }
        };
        manager.lists = manager
            .settings
            .sources
            .iter()
            .map(|source| match manager.read_list(source) {
                Ok(data) => CachedList {
                    data: Some(data),
                    ..Default::default()
                },
                Err(error) => CachedList {
                    data: None,
                    status: error,
                    failed: true,
                },
            })
            .collect();
        manager.rebuild_matches();
        manager
    }

    fn read_settings(&self) -> Result<Settings, String> {
        let bytes = match fs::read(self.directory.join(SETTINGS_FILE)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(error) => return Err(error.to_string()),
        };
        let settings: Settings =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let mut filenames = HashSet::new();
        for source in &settings.sources {
            if !valid_filename(&source.filename)
                || !filenames.insert(source.filename.to_ascii_lowercase())
            {
                return Err(format!(
                    "Invalid or duplicate list filename: {}",
                    source.filename
                ));
            }
        }
        Ok(settings)
    }

    fn read_list(&self, source: &Source) -> Result<ParsedList, String> {
        let file = fs::File::open(self.directory.join(&source.filename))
            .map_err(|error| error.to_string())?;
        ParsedList::parse(&read_list_bytes(file)?)
    }

    fn save_settings(&mut self, settings: Settings) -> Result<(), String> {
        fs::create_dir_all(&self.directory).map_err(|error| error.to_string())?;
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
        replace_file(&self.directory.join(SETTINGS_FILE), &bytes)?;
        self.settings = settings;
        Ok(())
    }

    pub fn filters(&self) -> &[PlayerAttribute] {
        &self.settings.filters
    }

    pub fn set_filters(&mut self, filters: Vec<PlayerAttribute>) -> Result<(), String> {
        let mut settings = self.settings.clone();
        settings.filters = filters;
        self.save_settings(settings)?;
        self.rebuild_matches();
        Ok(())
    }

    pub fn summaries(&self) -> impl Iterator<Item = ListSummary<'_>> {
        self.settings
            .sources
            .iter()
            .zip(&self.lists)
            .map(|(source, list)| ListSummary {
                title: list
                    .data
                    .as_ref()
                    .and_then(|data| data.title.as_deref())
                    .unwrap_or(&source.filename),
                filename: &source.filename,
                url: &source.url,
                enabled: source.enabled,
                counts: list
                    .data
                    .as_ref()
                    .map(|data| (data.players.len(), &data.counts)),
                status: &list.status,
                failed: list.failed,
            })
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) -> Result<(), String> {
        let mut settings = self.settings.clone();
        settings.sources[index].enabled = enabled;
        self.save_settings(settings)?;
        self.rebuild_matches();
        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<(), String> {
        if self.busy() {
            return Err("Wait for the current download to finish.".into());
        }
        let mut settings = self.settings.clone();
        let source = settings.sources.remove(index);
        self.save_settings(settings)?;
        self.lists.remove(index);
        self.rebuild_matches();
        match fs::remove_file(self.directory.join(&source.filename)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "List removed from TF2Monitor, but could not delete {}: {error}",
                source.filename
            )),
        }
    }

    fn rebuild_matches(&mut self) {
        let mut matches = ListMatches {
            by_player: HashMap::new(),
            titles: self.summaries().map(|list| list.title.to_owned()).collect(),
        };
        for (index, (source, cached)) in self.settings.sources.iter().zip(&self.lists).enumerate() {
            if source.enabled
                && let Some(list) = &cached.data
            {
                for (&steamid, attributes) in &list.players {
                    if attributes
                        .iter()
                        .any(|attr| self.settings.filters.contains(attr))
                    {
                        matches.by_player.entry(steamid).or_default().push(index);
                    }
                }
            }
        }
        self.matches = Arc::new(matches);
        *self.shared_matches.0.lock().unwrap() = Arc::clone(&self.matches);
    }

    pub fn has_match(&self, steamid: SteamID) -> bool {
        self.matches.by_player.contains_key(&steamid)
    }

    pub fn sources_for(
        &self,
        steamid: SteamID,
    ) -> impl Iterator<Item = (&str, &str, Vec<PlayerAttribute>)> {
        self.matches
            .by_player
            .get(&steamid)
            .into_iter()
            .flatten()
            .map(move |&index| {
                let source = &self.settings.sources[index];
                let data = self.lists[index].data.as_ref().unwrap();
                let title = self.matches.titles[index].as_str();
                let attributes = self
                    .settings
                    .filters
                    .iter()
                    .copied()
                    .filter(|attr| data.players[&steamid].contains(attr))
                    .collect();
                (title, source.url.as_str(), attributes)
            })
    }

    pub fn busy(&self) -> bool {
        self.downloads.is_some()
    }

    pub fn has_source_url(&self, url: &str) -> bool {
        self.settings.sources.iter().any(|source| source.url == url)
    }

    pub fn add_from_url(&mut self, url: &str) -> Result<(), String> {
        let url = parse_url(url)?.to_string();
        if self.has_source_url(&url) {
            return Err("This URL is already in the list.".into());
        }
        self.start_imports(vec![url])
    }

    pub fn add_from_urls<'a>(
        &mut self,
        urls: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), String> {
        let mut pending = Vec::new();
        let mut seen = HashSet::new();
        for url in urls {
            let url = parse_url(url)?.to_string();
            if !self.has_source_url(&url) && seen.insert(url.clone()) {
                pending.push(url);
            }
        }
        if pending.is_empty() {
            return Ok(());
        }
        self.start_imports(pending)
    }

    fn start_imports(&mut self, urls: Vec<String>) -> Result<(), String> {
        let total = urls.len();
        self.start_downloads(Request::Add(urls))?;
        self.import = ImportProgress {
            total,
            ..Default::default()
        };
        Ok(())
    }

    pub fn check_for_updates(&mut self) {
        let now = Instant::now();
        if self.load_error.is_some() || self.busy() || now < self.next_auto_check {
            return;
        }
        self.next_auto_check = now + AUTO_CHECK_POLL_INTERVAL;
        if let Err(error) = self.start_updates(true, Utc::now().timestamp()) {
            log::warn!("Could not start automatic TF2BD updates: {error}");
        }
    }

    pub fn update_all(&mut self) -> Result<(), String> {
        self.start_updates(false, Utc::now().timestamp())
    }

    fn start_updates(&mut self, automatic: bool, now: i64) -> Result<(), String> {
        let sources: Vec<_> = self
            .settings
            .sources
            .iter()
            .zip(&self.lists)
            .enumerate()
            .filter(|(_, (source, _))| {
                !automatic
                    || (source.enabled
                        && source.last_checked_at.is_none_or(|checked| {
                            now.saturating_sub(checked) >= UPDATE_INTERVAL_SECONDS
                        }))
            })
            .map(|(index, (source, list))| {
                (
                    index,
                    UpdateSource {
                        import_url: source.url.clone(),
                        update_url: list.data.as_ref().and_then(|data| data.update_url.clone()),
                    },
                )
            })
            .collect();
        if sources.is_empty() {
            return Ok(());
        }
        let indices: Vec<_> = sources.iter().map(|&(index, _)| index).collect();
        self.start_downloads(Request::Update { sources, automatic })?;
        for index in indices {
            self.lists[index].status = "Waiting for update…".into();
            self.lists[index].failed = false;
        }
        Ok(())
    }

    fn record_update_attempt(&mut self, index: usize, automatic: bool) -> Result<bool, String> {
        if automatic && !self.settings.sources[index].enabled {
            self.lists[index].status.clear();
            return Ok(false);
        }
        let mut settings = self.settings.clone();
        settings.sources[index].last_checked_at = Some(Utc::now().timestamp());
        self.save_settings(settings)?;
        self.lists[index].status = "Checking for update…".into();
        Ok(true)
    }

    fn start_downloads(&mut self, request: Request) -> Result<(), String> {
        if self.busy() {
            return Err("Wait for the current download to finish.".into());
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("tf2bd-player-lists".into())
            .spawn(move || match request {
                Request::Add(urls) => {
                    for url in urls {
                        let result = download(&client, &url);
                        if sender.send(DownloadResult::Add(url, result)).is_err() {
                            break;
                        }
                    }
                }
                Request::Update { sources, automatic } => {
                    for (index, source) in sources {
                        // Persist the attempt before this source starts network I/O.
                        let (ready, start) = mpsc::sync_channel(1);
                        if sender
                            .send(DownloadResult::BeginUpdate {
                                index,
                                automatic,
                                ready,
                            })
                            .is_err()
                        {
                            break;
                        }
                        match start.recv() {
                            Ok(true) => {}
                            Ok(false) => continue,
                            Err(_) => break,
                        }
                        let result = download_update(&client, source);
                        if sender.send(DownloadResult::Update(index, result)).is_err() {
                            break;
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        self.downloads = Some(receiver);
        Ok(())
    }

    pub fn poll(&mut self) {
        let Some(receiver) = &self.downloads else {
            return;
        };
        match receiver.try_recv() {
            Ok(DownloadResult::Add(url, result)) => {
                match result.and_then(|download| self.save_new_list(url.clone(), download)) {
                    Ok(()) => self.import.added += 1,
                    Err(error) => self.import.errors.push(format!("{url}\n{error}")),
                }
            }
            Ok(DownloadResult::BeginUpdate {
                index,
                automatic,
                ready,
            }) => {
                let start = match self.record_update_attempt(index, automatic) {
                    Ok(start) => start,
                    Err(error) => {
                        self.lists[index].failed = true;
                        self.lists[index].status = error;
                        false
                    }
                };
                let _ = ready.send(start);
            }
            Ok(DownloadResult::Update(index, result)) => {
                let result = result.and_then(|download| {
                    let path = self.directory.join(&self.settings.sources[index].filename);
                    replace_file(&path, &download.bytes)?;
                    self.lists[index].data = Some(download.data);
                    self.rebuild_matches();
                    Ok(())
                });
                self.lists[index].failed = result.is_err();
                self.lists[index].status = result.err().unwrap_or_else(|| "Updated.".into());
            }
            Err(TryRecvError::Disconnected) => self.downloads = None,
            Err(TryRecvError::Empty) => {}
        }
    }

    fn save_new_list(&mut self, url: String, download: Download) -> Result<(), String> {
        let stem = download.filename.trim_end_matches(".json").to_owned();
        let mut filename = download.filename;
        let mut suffix = 2;
        while self.directory.join(&filename).exists()
            || self
                .settings
                .sources
                .iter()
                .any(|source| source.filename.eq_ignore_ascii_case(&filename))
        {
            filename = format!("{stem}-{suffix}.json");
            suffix += 1;
        }
        fs::create_dir_all(&self.directory).map_err(|error| error.to_string())?;
        let path = self.directory.join(&filename);
        replace_file(&path, &download.bytes)?;
        let mut settings = self.settings.clone();
        settings.sources.push(Source {
            url,
            filename,
            enabled: true,
            last_checked_at: Some(Utc::now().timestamp()),
        });
        if let Err(error) = self.save_settings(settings) {
            if let Err(cleanup) = fs::remove_file(&path) {
                log::warn!(
                    "Could not remove {} after failed import: {cleanup}",
                    path.display()
                );
            }
            return Err(error);
        }
        self.lists.push(CachedList {
            data: Some(download.data),
            ..Default::default()
        });
        self.rebuild_matches();
        Ok(())
    }
}

fn parse_url(value: &str) -> Result<Url, String> {
    let mut url = Url::parse(value.trim()).map_err(|error| format!("Invalid URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Use an HTTP or HTTPS URL.".into());
    }
    url.set_fragment(None);
    Ok(url)
}

fn download(client: &Client, value: &str) -> Result<Download, String> {
    let url = parse_url(value)?;
    let response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
    let url = response.url().clone();
    let filename = list_filename(&url);
    let bytes = read_list_bytes(response)?;
    let data = ParsedList::parse(&bytes)?;
    Ok(Download {
        bytes,
        data,
        filename,
    })
}

fn download_update(client: &Client, source: UpdateSource) -> Result<Download, String> {
    let Some(update_url) = source.update_url.filter(|url| url != &source.import_url) else {
        return download(client, &source.import_url);
    };
    download(client, &update_url).or_else(|update_error| {
        download(client, &source.import_url).map_err(|import_error| {
            format!("Update URL failed ({update_error}). Import URL failed ({import_error}).")
        })
    })
}

fn read_list_bytes(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_LIST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_LIST_BYTES {
        return Err("The list exceeds the 64 MiB limit.".into());
    }
    Ok(bytes)
}

fn list_filename(url: &Url) -> String {
    let name = url.path().rsplit('/').next().unwrap_or_default();
    let name = name.trim_end_matches(".json");
    let stem: String = name
        .chars()
        .take(120)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{}.json", stem.trim_matches('.'));
    if valid_filename(&filename) {
        filename
    } else {
        "playerlist.json".into()
    }
}

fn valid_filename(filename: &str) -> bool {
    let basename = filename
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    filename.ends_with(".json")
        && !filename.eq_ignore_ascii_case(SETTINGS_FILE)
        && !basename.is_empty()
        && !matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(basename.len() == 4
            && (basename.starts_with("COM") || basename.starts_with("LPT"))
            && basename.as_bytes()[3].is_ascii_digit())
        && filename
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_' | b'.'))
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    let result = (|| -> io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result.map_err(|error| format!("Could not save {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const A: &[u8] = br#"{
        "file_info": { "title": "List A" },
        "players": [
            {"steamid": "[U:1:42]", "attributes": ["cheater"]},
            {"steamid": 76561197960265770, "attributes": ["cheater", "suspicious"]},
            {"steamid": 43, "attributes": ["exploiter", "unknown"]}
        ]
    }"#;
    const B: &[u8] = br#"{
        "file_info": { "title": "List B" },
        "players": [{"steamid": 42, "attributes": ["bot"]}]
    }"#;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "tf2monitor-player-lists-{}-{nonce}",
                std::process::id()
            )))
        }

        fn open(&self) -> PlayerLists {
            PlayerLists::load_from(self.0.clone(), SharedMatches::default())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn wait_for_downloads(lists: &mut PlayerLists) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while lists.busy() {
            assert!(Instant::now() < deadline, "Download did not complete");
            lists.poll();
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn cached_lists_keep_distinct_sources_and_restore_filters_and_toggles() {
        let directory = TestDirectory::new();
        let mut lists = directory.open();
        let a = "https://a.example/playerlist.json";
        let b = "https://b.example/playerlist.json";
        for (url, bytes) in [(a, A), (b, B)] {
            lists
                .save_new_list(
                    url.to_owned(),
                    Download {
                        filename: list_filename(&parse_url(url).unwrap()),
                        data: ParsedList::parse(bytes).unwrap(),
                        bytes: bytes.to_vec(),
                    },
                )
                .unwrap();
        }

        assert_eq!(
            lists
                .summaries()
                .map(|list| list.filename)
                .collect::<Vec<_>>(),
            ["playerlist.json", "playerlist-2.json",]
        );
        assert_eq!(
            lists.summaries().next().unwrap().counts,
            Some((2, &[1, 0, 1, 1, 0, 0]))
        );
        let player = SteamID::from_u64(MIN_STEAMID64 + 42);
        assert_eq!(
            lists.sources_for(player).collect::<Vec<_>>(),
            [
                ("List A", a, vec![PlayerAttribute::Cheater]),
                ("List B", b, vec![PlayerAttribute::Bot]),
            ]
        );
        assert!(!lists.has_match(SteamID::from_u64(MIN_STEAMID64 + 43)));
        let shared = lists.shared_matches.clone();
        assert_eq!(
            shared.snapshot().titles_for(player).collect::<Vec<_>>(),
            ["List A", "List B"],
        );

        lists
            .set_filters(vec![PlayerAttribute::Suspicious, PlayerAttribute::Bot])
            .unwrap();
        lists.set_enabled(1, false).unwrap();
        assert_eq!(
            shared.snapshot().titles_for(player).collect::<Vec<_>>(),
            ["List A"],
        );
        let mut lists = directory.open();
        assert_eq!(
            lists.sources_for(player).collect::<Vec<_>>(),
            [("List A", a, vec![PlayerAttribute::Suspicious]),]
        );
        lists.remove(0).unwrap();
        assert!(!lists.has_match(player));
        assert!(!directory.0.join("playerlist.json").exists());
        assert!(directory.0.join("playerlist-2.json").exists());
        assert_eq!(directory.open().summaries().next().unwrap().url, b);
    }

    fn serve(listener: std::net::TcpListener, responses: &[(&str, &[u8])]) {
        use std::io::{BufRead, BufReader};

        for &(path, bytes) in responses {
            let (mut connection, _) = listener.accept().unwrap();
            connection
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&connection);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with(&format!("GET {path} ")), "{line}");
            loop {
                line.clear();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
            }
            write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            connection.write_all(bytes).unwrap();
        }
    }

    #[test]
    fn batch_import_skips_existing_sources_and_continues_after_invalid_json() {
        let directory = TestDirectory::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let existing = format!("{origin}/existing.json");
        let invalid = format!("{origin}/invalid.json");
        let new = format!("{origin}/new.json");
        let server = thread::spawn(move || {
            serve(
                listener,
                &[
                    ("/existing.json", A),
                    ("/invalid.json", b"not JSON"),
                    ("/new.json", B),
                ],
            );
        });

        let mut lists = directory.open();
        lists.add_from_url(&existing).unwrap();
        wait_for_downloads(&mut lists);
        lists.set_enabled(0, false).unwrap();

        lists
            .add_from_urls([existing.as_str(), &invalid, &new, &new])
            .unwrap();
        wait_for_downloads(&mut lists);
        assert_eq!(lists.import.total, 2);
        assert_eq!(lists.import.added, 1);
        assert_eq!(lists.import.errors.len(), 1);
        assert!(lists.import.errors[0].starts_with(&invalid));

        let reopened = directory.open();
        assert_eq!(
            reopened
                .summaries()
                .map(|list| (list.url, list.enabled))
                .collect::<Vec<_>>(),
            [(existing.as_str(), false), (new.as_str(), true)]
        );
        server.join().unwrap();
    }

    #[test]
    fn automatic_updates_throttle_each_active_source_even_after_failure_and_restart() {
        let directory = TestDirectory::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let active = format!("{origin}/active.json");
        let disabled = format!("{origin}/disabled.json");
        let server = thread::spawn(move || {
            serve(
                listener,
                &[
                    ("/active.json", A),
                    ("/disabled.json", A),
                    ("/active.json", b"not JSON"),
                    ("/active.json", B),
                    ("/disabled.json", B),
                ],
            );
        });

        let mut lists = directory.open();
        lists
            .add_from_urls([active.as_str(), disabled.as_str()])
            .unwrap();
        wait_for_downloads(&mut lists);
        lists.set_enabled(1, false).unwrap();
        let mut settings = lists.settings.clone();
        let last_checked = Utc::now().timestamp() - UPDATE_INTERVAL_SECONDS;
        settings.sources[0].last_checked_at = Some(last_checked);
        settings.sources[1].last_checked_at = Some(last_checked);
        lists.save_settings(settings).unwrap();

        lists
            .start_updates(true, last_checked + UPDATE_INTERVAL_SECONDS - 1)
            .unwrap();
        assert!(!lists.busy());
        lists.check_for_updates();
        assert!(lists.busy());
        wait_for_downloads(&mut lists);
        assert!(lists.summaries().next().unwrap().failed);

        let mut lists = directory.open();
        assert!(lists.settings.sources[0].last_checked_at.unwrap() > last_checked);
        assert_eq!(
            lists.settings.sources[1].last_checked_at,
            Some(last_checked)
        );
        assert_eq!(fs::read(directory.0.join("active.json")).unwrap(), A);
        lists.check_for_updates();
        assert!(!lists.busy());

        lists.update_all().unwrap();
        wait_for_downloads(&mut lists);
        let lists = directory.open();
        assert!(!lists.settings.sources[1].enabled);
        assert!(lists.settings.sources[1].last_checked_at.unwrap() > last_checked);
        assert_eq!(fs::read(directory.0.join("active.json")).unwrap(), B);
        assert_eq!(fs::read(directory.0.join("disabled.json")).unwrap(), B);
        server.join().unwrap();
    }

    #[test]
    fn existing_sources_without_a_timestamp_can_be_loaded() {
        let source: Source = serde_json::from_str(
            r#"{"url":"https://example.org/playerlist.json","filename":"playerlist.json","enabled":true}"#,
        )
        .unwrap();
        assert!(source.last_checked_at.is_none());
    }

    #[test]
    fn manual_update_falls_back_to_import_url_and_preserves_the_last_valid_file() {
        let directory = TestDirectory::new();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let update_url = format!("{origin}/update.json");
        let initial = format!(
            r#"{{"file_info":{{"update_url":"{update_url}"}},"players":[{{"steamid":42,"attributes":["cheater"]}}]}}"#
        );
        let server = thread::spawn(move || {
            serve(
                listener,
                &[
                    ("/original.json", initial.as_bytes()),
                    ("/update.json", b"not JSON"),
                    ("/original.json", b"also not JSON"),
                    ("/update.json", b"still not JSON"),
                    ("/original.json", B),
                ],
            );
        });

        let mut lists = directory.open();
        lists
            .add_from_url(&format!("{origin}/original.json"))
            .unwrap();
        wait_for_downloads(&mut lists);
        assert_eq!(lists.import.added, 1, "{:?}", lists.import.errors);
        let path = directory.0.join("original.json");
        let original = fs::read(&path).unwrap();

        // Reopening the app loads the cached file and its update URL.
        let mut lists = directory.open();
        let player = SteamID::from_u64(MIN_STEAMID64 + 42);
        lists.update_all().unwrap();
        wait_for_downloads(&mut lists);
        assert!(lists.summaries().next().unwrap().failed);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(
            lists.sources_for(player).next().unwrap().2,
            [PlayerAttribute::Cheater]
        );

        lists.update_all().unwrap();
        wait_for_downloads(&mut lists);
        assert!(!lists.summaries().next().unwrap().failed);
        assert_eq!(fs::read(&path).unwrap(), B);
        assert_eq!(
            lists.sources_for(player).next().unwrap().2,
            [PlayerAttribute::Bot]
        );
        server.join().unwrap();
    }
}
