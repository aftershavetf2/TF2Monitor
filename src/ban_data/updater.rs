use chrono::Utc;
use diesel::SqliteConnection;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    time::Duration,
};

use super::{
    Control, Result,
    models::{Community, ExportBan, Manifest},
    storage,
};

const LATEST_RELEASE: &str =
    "https://api.github.com/repos/aftershavetf2/tf2-ban-data/releases/latest";
pub const CHECK_INTERVAL: i64 = 24 * 60 * 60;

pub fn due(last_attempt: Option<i64>, now: i64) -> bool {
    last_attempt.is_none_or(|last| now.saturating_sub(last) >= CHECK_INTERVAL)
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

fn json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let mut body = Vec::new();
    client
        .get(url)
        .send()?
        .error_for_status()?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut body)?;
    if body.len() > 1024 * 1024 {
        return Err("Release metadata exceeds size limit".into());
    }
    Ok(serde_json::from_slice(&body)?)
}

/// Discover the release once. All files use that fixed tag, even if latest changes.
fn release_manifest(client: &Client) -> Result<(String, reqwest::Url, Manifest)> {
    let release: Release = json(client, LATEST_RELEASE)?;
    if release.tag_name.is_empty() {
        return Err("Release has no tag".into());
    }
    let mut base =
        reqwest::Url::parse("https://github.com/aftershavetf2/tf2-ban-data/releases/download/")?;
    base.path_segments_mut()
        .map_err(|_| "Invalid release URL")?
        .push(&release.tag_name)
        .push("");
    let manifest: Manifest = json(client, base.join("manifest.json")?.as_str())?;
    manifest.validate()?;
    Ok((release.tag_name, base, manifest))
}

fn hex_digest(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Stage and hash the exact compressed bytes, then verify the decompressed JSON.
fn download(
    client: &Client,
    url: &str,
    path: &Path,
    community: &Community,
) -> Result<Vec<ExportBan>> {
    let result = (|| {
        let mut response = client
            .get(url)
            .send()?
            .error_for_status()?
            .take(community.gzip.bytes + 1);
        let mut file = File::create(path)?;
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            hasher.update(&buffer[..count]);
            file.write_all(&buffer[..count])?;
        }
        if bytes != community.gzip.bytes
            || !hex_digest(&hasher.finalize()).eq_ignore_ascii_case(&community.gzip.sha256)
        {
            return Err(format!("Compressed size or SHA-256 mismatch for {}", community.id).into());
        }
        file.sync_all()?;
        drop(file);
        let mut raw = Vec::new();
        GzDecoder::new(File::open(path)?)
            .take(community.bytes + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 != community.bytes
            || !hex_digest(&Sha256::digest(&raw)).eq_ignore_ascii_case(&community.sha256)
        {
            return Err(format!("JSON size or SHA-256 mismatch for {}", community.id).into());
        }
        let bans: Vec<ExportBan> = serde_json::from_slice(&raw)?;
        if bans.len() != community.ban_count {
            return Err(format!("Ban count mismatch for {}", community.id).into());
        }
        Ok(bans)
    })();
    // Downloads are temporary; the database is the durable local snapshot.
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("Could not remove ban staging file: {error}");
    }
    result
}

fn update(conn: &mut SqliteConnection, directory: &Path, control: &Control) -> Result<String> {
    let client = Client::builder()
        .user_agent(concat!("TF2Monitor/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()?;
    let (release, base, manifest) = release_manifest(&client)?;
    apply_manifest(conn, directory, control, &client, &base, &manifest)?;
    storage::record_success(conn, Utc::now().timestamp(), &release)?;
    Ok(release)
}

fn apply_manifest(
    conn: &mut SqliteConnection,
    directory: &Path,
    control: &Control,
    client: &Client,
    base: &reqwest::Url,
    manifest: &Manifest,
) -> Result<()> {
    manifest.validate()?;
    storage::expect_communities(conn, &manifest.communities)?;
    control.changed();
    for community in &manifest.communities {
        control.message(format!("Checking {}...", community.name));
        if storage::content_hash(conn, &community.id)?.as_deref() == Some(&community.sha256) {
            storage::update_metadata(conn, community)?;
            continue;
        }
        control.message(format!("Downloading {}...", community.name));
        let path = directory.join(format!("{}.part", community.gzip.file));
        let bans = download(
            client,
            base.join(&community.gzip.file)?.as_str(),
            &path,
            community,
        )?;
        control.message(format!("Importing {}...", community.name));
        storage::import(conn, community, &bans)?;
        control.changed();
    }
    // Only prune communities after every manifest entry has succeeded.
    storage::retain_communities(conn, &manifest.communities)?;
    control.changed();
    Ok(())
}

pub fn run(mut conn: SqliteConnection, directory: std::path::PathBuf, control: Control) {
    loop {
        let now = Utc::now().timestamp();
        let state = match storage::state(&mut conn) {
            Ok(state) => state,
            Err(error) => {
                control.finish(format!("Could not read ban update state: {error}"), None);
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };
        if control.begin(due(state.last_attempt, now)) {
            // Persist the attempt before making any network requests (also on failure).
            let result = storage::record_attempt(&mut conn, now)
                .and_then(|()| update(&mut conn, &directory, &control));
            let message = match result {
                Ok(release) => format!("Ban data is up to date ({release})"),
                Err(error) => {
                    let message = format!("Ban data check failed: {error}");
                    log::warn!("{message}");
                    if let Err(error) = storage::record_error(&mut conn, &message) {
                        log::error!("Could not save ban update error: {error}");
                    }
                    message
                }
            };
            control.finish(message, storage::state(&mut conn).ok());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ban_data::{
        models::CompressedFile,
        tests::{ban, community},
    };
    use std::{
        net::TcpListener,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn temp() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tf2monitor-download-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn fixture() -> (Community, Vec<u8>) {
        let raw = serde_json::to_vec(&vec![ban()]).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut community = community();
        community.bytes = raw.len() as u64;
        community.sha256 = hex_digest(&Sha256::digest(&raw));
        community.gzip = CompressedFile {
            file: "example.json.gz".into(),
            bytes: compressed.len() as u64,
            sha256: hex_digest(&Sha256::digest(&compressed)),
        };
        (community, compressed)
    }

    /// One response per connection with an explicit deadline so failures cannot hang tests.
    fn server(bodies: Vec<(u16, Vec<u8>)>) -> (reqwest::Url, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url =
            reqwest::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        listener.set_nonblocking(true).unwrap();
        let handle = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            for (status, body) in bodies {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && start.elapsed() < Duration::from_secs(15) =>
                        {
                            std::thread::sleep(Duration::from_millis(10))
                        }
                        Err(error) => panic!("Fixture HTTP server: {error}"),
                    }
                };
                // Windows can inherit the listener's nonblocking mode on accepted sockets.
                // Poll accept, but wait for request headers with the read timeout below.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0; 8192];
                let mut received = 0;
                while !request[..received].ends_with(b"\r\n\r\n") {
                    assert!(
                        received < request.len(),
                        "Fixture HTTP request is too large"
                    );
                    let count = stream.read(&mut request[received..]).unwrap();
                    assert!(count > 0, "Fixture HTTP request ended before its headers");
                    received += count;
                }
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        (url, handle)
    }

    #[test]
    fn fixture_server_waits_for_complete_request_headers() {
        let (url, server) = server(vec![(200, b"OK".to_vec())]);
        let mut client =
            std::net::TcpStream::connect((url.host_str().unwrap(), url.port().unwrap())).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client.write_all(b"GET / HTTP/1.1\r\n").unwrap();
        // Leave the server waiting for the remaining headers, as TCP may do naturally.
        std::thread::sleep(Duration::from_millis(50));
        client.write_all(b"Host: localhost\r\n\r\n").unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 "));
        assert!(response.ends_with("\r\n\r\nOK"));
        server.join().unwrap();
    }

    #[test]
    fn sha256_hex_matches_known_digest() {
        assert_eq!(
            hex_digest(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hex_digest(&[0, 1, 15, 16, 255]), "00010f10ff");
    }

    #[test]
    fn downloads_verify_both_hashes_sizes_count_and_keep_old_data_on_failure() {
        let (mut community, compressed) = fixture();
        let directory = temp();
        let control = Control::default();
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let (base, server) = server(vec![
            (200, compressed.clone()),
            (200, compressed.clone()),
            (200, compressed.clone()),
            (200, compressed.clone()),
            (200, compressed.clone()),
            (404, Vec::new()),
        ]);
        let mut conn = storage::open(&directory.join("bans.sqlite")).unwrap();
        let manifest = Manifest {
            manifest_version: 1,
            data_format_version: 2,
            communities: vec![community],
        };
        apply_manifest(&mut conn, &directory, &control, &client, &base, &manifest).unwrap();
        let id = ban().steam_id().unwrap();
        assert_eq!(storage::lookup(&mut conn, id).unwrap().len(), 1);
        community = manifest.communities.into_iter().next().unwrap();
        let original_hash = community.sha256.clone();
        community.sha256 = "0".repeat(64); // Force a download and fail decompressed hash verification.
        let manifest = Manifest {
            manifest_version: 1,
            data_format_version: 2,
            communities: vec![community],
        };
        assert!(
            apply_manifest(&mut conn, &directory, &control, &client, &base, &manifest).is_err()
        );
        assert_eq!(
            storage::content_hash(&mut conn, "example")
                .unwrap()
                .as_deref(),
            Some(original_hash.as_str())
        );
        assert_eq!(storage::lookup(&mut conn, id).unwrap().len(), 1);
        community = manifest.communities.into_iter().next().unwrap();
        community.sha256 = original_hash;
        let path = directory.join("test.part");
        let gzip_hash = community.gzip.sha256.clone();
        community.gzip.sha256 = "0".repeat(64);
        assert!(download(&client, base.as_str(), &path, &community).is_err());
        community.gzip.sha256 = gzip_hash;
        community.gzip.bytes -= 1;
        assert!(download(&client, base.as_str(), &path, &community).is_err());
        community.gzip.bytes += 1;
        community.ban_count += 1;
        assert!(download(&client, base.as_str(), &path, &community).is_err());
        assert!(download(&client, base.as_str(), &path, &community).is_err());
        assert!(!path.exists());
        server.join().unwrap();
        drop(conn);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unchanged_content_updates_coverage_without_downloading() {
        let directory = temp();
        let mut conn = storage::open(&directory.join("bans.sqlite")).unwrap();
        let mut community = community();
        storage::import(&mut conn, &community, &[ban()]).unwrap();
        community.complete = false;
        let manifest = Manifest {
            manifest_version: 1,
            data_format_version: 2,
            communities: vec![community],
        };
        // No HTTP listener exists: success proves the unchanged data was not fetched.
        apply_manifest(
            &mut conn,
            &directory,
            &Control::default(),
            &Client::new(),
            &reqwest::Url::parse("http://127.0.0.1:1/").unwrap(),
            &manifest,
        )
        .unwrap();
        assert_eq!(storage::coverage(&mut conn).unwrap(), Some(false));
        drop(conn);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "Downloads the public release; run explicitly with network access"]
    fn live_release_import() {
        let directory = temp();
        let mut conn = storage::open(&directory.join("bans.sqlite")).unwrap();
        let control = Control::default();
        storage::record_attempt(&mut conn, Utc::now().timestamp()).unwrap();
        let release = update(&mut conn, &directory, &control).unwrap();
        assert!(storage::coverage(&mut conn).unwrap().is_some());
        assert_eq!(
            storage::state(&mut conn).unwrap().release.as_deref(),
            Some(release.as_str())
        );
        let first_revision = control.revision();
        let repeated = update(&mut conn, &directory, &control).unwrap();
        assert_eq!(release, repeated);
        // Only metadata/finalization invalidates the cache; no community imports.
        assert_eq!(control.revision() - first_revision, 2);
        assert!(!directory.join("example.json.gz.part").exists());
        eprintln!("Verified live release {release}; second check skipped all community downloads");
        drop(conn);
        fs::remove_dir_all(directory).unwrap();
    }
}
