use fs_err::create_dir_all;
use std::{
    fs::{metadata, write},
    path::PathBuf,
    time::SystemTime,
};

const BASE_DIR: &str = ".http-cache";

pub fn get_from_cache_or_fetch(
    domain: &str,
    key: &str,
    days_to_live: i32,
    url: &str,
) -> Option<String> {
    let mut path: PathBuf = [BASE_DIR, domain, key].iter().collect();
    ensure_directories_exist(&path);

    let filename = get_filename_from_url(url);
    path.push(filename.as_str());

    // Delete file if it is older than days_to_live
    delete_file_if_too_old(days_to_live, &path);

    // If the file exists, it's not too old, return its content
    if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        return Some(content);
    }

    // Fetch the URL contents and save it to the cache
    // log::info!("Data not in cache, fetching URL: {}", url);
    let html = get_html(url)?;

    match write(&path, &html) {
        Ok(_) => (),
        Err(e) => {
            log::error!(
                "XXX Failed to write to file: {}, full path = {}",
                e,
                path.display()
            );
        }
    }

    Some(html)
}

fn get_filename_from_url(url: &str) -> String {
    let url = url.replace("https://", "").replace("http://", "");
    let url = url.replace("/", "_");
    let url = url.replace(".", "_");
    let url = url.replace("?", "_");
    let url = url.replace("&", "_");
    let url = url.replace("%", "_");
    let url = url.replace(":", "_");
    url.replace("=", "_")
}

fn ensure_directories_exist(path: &PathBuf) {
    match create_dir_all(path) {
        Ok(_) => (),
        Err(e) => {
            log::error!("Failed to create directory: {}", e);
        }
    }
}

fn delete_file_if_too_old(days_to_live: i32, path: &PathBuf) {
    if !path.exists() {
        return;
    }

    // Delete the file if it is older than days_to_live
    let now = std::time::SystemTime::now();
    let modified_time = get_modified_time(path);

    if let Some(modified_time) = modified_time {
        if let Ok(duration) = now.duration_since(modified_time) {
            if duration.as_secs() > (days_to_live * 24 * 60 * 60) as u64 {
                std::fs::remove_file(path).unwrap_or_default();
            }
        }
    }
}

fn get_modified_time(path: &PathBuf) -> Option<SystemTime> {
    match metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified_time) => Some(modified_time),
            Err(e) => {
                log::error!(
                    "Failed to get modified time for file: {}. Error: {}",
                    path.display(),
                    e
                );
                None
            }
        },
        Err(e) => {
            log::error!(
                "Failed to get metadata for file: {}. Error: {}",
                path.display(),
                e
            );
            None
        }
    }
}

fn get_html(url: &str) -> Option<String> {
    match reqwest::blocking::get(url) {
        Ok(resp) => match resp.text() {
            Ok(text) => Some(text),
            Err(err) => {
                log::error!("http_cache: Failed to get text from response: {err}");
                None
            }
        },
        Err(err) => {
            log::error!("http_cache: Failed to get text from response: {err}");
            None
        }
    }
}
