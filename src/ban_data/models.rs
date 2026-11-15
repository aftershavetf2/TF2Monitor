use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::Result;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub manifest_version: u32,
    pub data_format_version: u32,
    pub communities: Vec<Community>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Community {
    pub id: String,
    pub name: String,
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
    pub gzip: CompressedFile,
    pub ban_count: usize,
    pub complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct CompressedFile {
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBan {
    pub steam_id: String,
    pub player_name: Option<String>,
    pub reason: Option<String>,
    pub created_date: Option<String>,
    pub expires_at: Option<String>,
    pub admin_name: Option<String>,
    pub removed_at: Option<String>,
    pub remove_type: Option<String>,
    pub unban_reason: Option<String>,
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        if self.manifest_version != 1 || self.data_format_version != 2 {
            return Err("Unsupported ban data format; update TF2Monitor".into());
        }
        if self.communities.is_empty() {
            return Err("The manifest contains no communities".into());
        }
        let mut ids = HashSet::new();
        for community in &self.communities {
            if community.id.is_empty()
                || !community
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
                || !ids.insert(&community.id)
                || community.file != format!("{}.json", community.id)
                || community.gzip.file != format!("{}.json.gz", community.id)
                || !valid_hash(&community.sha256)
                || !valid_hash(&community.gzip.sha256)
                || community.gzip.bytes > 64 * 1024 * 1024
                || community.bytes > 256 * 1024 * 1024
            {
                return Err(format!("Invalid manifest entry: {}", community.id).into());
            }
        }
        Ok(())
    }
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

impl ExportBan {
    pub fn steam_id(&self) -> Result<i64> {
        let id = self.steam_id.parse::<i64>()?;
        if id < crate::models::steamid::MIN_STEAMID64 as i64 {
            return Err("Invalid SteamID64 in ban data".into());
        }
        Ok(id)
    }

    pub fn validate(&self) -> Result<()> {
        self.steam_id()?;
        for date in [&self.created_date, &self.expires_at, &self.removed_at]
            .into_iter()
            .flatten()
        {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
        }
        Ok(())
    }

    pub fn active_on(&self, today: NaiveDate) -> bool {
        self.removed_at.is_none()
            && self.remove_type.is_none()
            && self.expires_at.as_ref().is_none_or(|date| {
                NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok_and(|expires| expires > today)
            })
    }

    pub fn status_on(&self, today: NaiveDate) -> &'static str {
        if self.remove_type.as_deref() == Some("expired") {
            "Expired"
        } else if self.removed_at.is_some() || self.remove_type.is_some() {
            "Removed"
        } else if self.active_on(today) {
            "Active"
        } else {
            "Expired"
        }
    }
}
