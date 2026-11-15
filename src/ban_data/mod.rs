//! Local community ban snapshots, independently updated from GitHub Releases.
pub mod models;
pub mod storage;
mod updater;

use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use storage::UpdateState;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const DIRECTORY: &str = "data/ban-data";

#[derive(Default, Clone)]
pub struct Control {
    inner: Arc<Mutex<Status>>,
    revision: Arc<AtomicU64>,
}

#[derive(Default, Clone)]
pub struct Status {
    pub available: bool,
    pub busy: bool,
    pub message: String,
    pub state: UpdateState,
    requested: bool,
}

impl Control {
    pub fn status(&self) -> Status {
        self.inner.lock().unwrap().clone()
    }

    /// Reserve one check immediately. Requests during a pending/running check are ignored.
    pub fn request_check(&self) -> bool {
        let mut status = self.inner.lock().unwrap();
        if !status.available || status.busy {
            return false;
        }
        status.busy = true;
        status.requested = true;
        status.message = "Ban data check queued...".into();
        true
    }

    fn begin(&self, due: bool) -> bool {
        let mut status = self.inner.lock().unwrap();
        if status.requested {
            status.requested = false;
        } else if !due || status.busy {
            return false;
        }
        status.busy = true;
        status.message = "Checking for new ban data...".into();
        true
    }

    fn message(&self, message: String) {
        self.inner.lock().unwrap().message = message;
    }
    fn changed(&self) {
        self.revision.fetch_add(1, Ordering::Release);
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    fn finish(&self, message: String, state: Option<UpdateState>) {
        let mut status = self.inner.lock().unwrap();
        status.busy = false;
        status.message = message;
        if let Some(state) = state {
            status.state = state;
        }
    }
}

pub fn start(bus: &Arc<Mutex<crate::appbus::AppBus>>) -> Result<()> {
    let control = bus.lock().unwrap().ban_data.clone();
    let result = (|| {
        let directory = Path::new(DIRECTORY);
        std::fs::create_dir_all(directory.join("staging"))?;
        let path = directory.join("bans.sqlite");
        let mut conn = storage::open(&path)?;
        let state = storage::state(&mut conn)?;
        control.finish(
            state.last_error.clone().unwrap_or_else(|| {
                state
                    .release
                    .as_ref()
                    .map_or("Ban data has not been downloaded yet".into(), |release| {
                        format!("Local ban data: {release}")
                    })
            }),
            Some(state),
        );
        crate::reputation::start(bus, path, control.clone())?;
        let updater_control = control.clone();
        let staging = directory.join("staging");
        std::thread::Builder::new()
            .name("ban-data-updater".into())
            .spawn(move || updater::run(conn, staging, updater_control))?;
        control.inner.lock().unwrap().available = true;
        Ok(())
    })();
    if let Err(error) = &result {
        control.finish(format!("Ban data unavailable: {error}"), None);
    }
    result
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn community() -> models::Community {
        serde_json::from_value(serde_json::json!({
            "id":"example", "name":"Example", "file":"example.json", "sha256":"a".repeat(64),
            "bytes":2, "gzip":{"file":"example.json.gz", "sha256":"b".repeat(64), "bytes":22},
            "banCount":1, "complete":true
        }))
        .unwrap()
    }

    pub fn ban() -> models::ExportBan {
        serde_json::from_value(serde_json::json!({
            "steam_id":"76561197960265729", "reason":"Cheating", "created_date":"2025-01-02"
        }))
        .unwrap()
    }

    #[test]
    fn manual_checks_bypass_interval_without_overlapping_or_queuing_duplicates() {
        let control = Control::default();
        assert!(!control.request_check());
        control.inner.lock().unwrap().available = true;
        assert!(!control.begin(false));
        assert!(control.request_check());
        assert!(!control.request_check());
        assert!(control.begin(false));
        assert!(!control.request_check());
        assert!(!control.begin(true));
        control.finish("Done".into(), None);
        assert!(!control.begin(false));
        assert!(control.begin(true));
    }

    #[test]
    fn manifest_rejects_unsupported_formats_unsafe_names_and_duplicate_communities() {
        let mut manifest = models::Manifest {
            manifest_version: 1,
            data_format_version: 2,
            communities: vec![community()],
        };
        manifest.validate().unwrap();
        manifest.manifest_version = 2;
        assert!(manifest.validate().is_err());
        manifest.manifest_version = 1;
        manifest.communities[0].gzip.file = "../example.json.gz".into();
        assert!(manifest.validate().is_err());
        manifest.communities = vec![community(), community()];
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn active_expired_and_removed_bans_are_distinct() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        let mut ban = ban();
        ban.validate().unwrap();
        assert!(ban.active_on(today));
        ban.expires_at = Some("2026-11-15".into());
        assert!(!ban.active_on(today));
        assert_eq!(ban.status_on(today), "Expired");
        ban.expires_at = None;
        ban.removed_at = Some("2026-10-01".into());
        assert!(!ban.active_on(today));
        assert_eq!(ban.status_on(today), "Removed");
        ban.remove_type = Some("expired".into());
        assert_eq!(ban.status_on(today), "Expired");
        ban.created_date = Some("invalid".into());
        assert!(ban.validate().is_err());
    }
}
