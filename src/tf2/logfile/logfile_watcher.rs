use super::line_parser::LogLineParser;
use crate::appbus::AppBus;
use crate::models::app_settings::AppSettings;
use crate::utils::BoxResult;
use fs_err as fs;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{thread, time};
use thread::sleep;

/// The delay between loops in run()
const LOOP_DELAY: Duration = time::Duration::from_millis(100);

// The delay between checking if the file exists
const FILE_NOT_EXIST_DELAY: Duration = time::Duration::from_millis(20 * 1000);

/// Start the logfile watcher thread to run in the background
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut watcher = LogfileWatcher {
        filename: settings.log_filename.clone(),
        last_pos: 0,
        bus: Arc::clone(bus),
    };

    remove_log_file(&watcher.filename);

    thread::spawn(move || watcher.run())
}

#[allow(dead_code)]
fn remove_log_file(filename: &str) {
    if Path::new(filename).exists() {
        log::info!("Removing log file: {}", filename);
        let _ = fs::remove_file(filename);
    } else {
        log::warn!(
            "Log file does not exist (yet?), or wrong path? {}",
            filename
        );
    }
}

pub struct LogfileWatcher {
    pub filename: String,
    pub last_pos: u64,

    bus: Arc<Mutex<AppBus>>,
}

impl LogfileWatcher {
    pub fn run(&mut self) {
        let parser = LogLineParser::default();
        log::info!(
            "Logfile watcher started. Will monitor file: {}",
            self.filename
        );

        self.goto_last_pos();

        loop {
            sleep(LOOP_DELAY);

            self.process_new_data(&parser);
        }
    }

    /// Go to the last position in the file
    fn goto_last_pos(&mut self) {
        self.last_pos = 0;

        let file = fs::File::open(self.filename.as_str());
        if let Ok(file) = file {
            if let Ok(metadata) = file.metadata() {
                self.last_pos = metadata.len();
            }
        }
    }

    pub fn process_new_data(&mut self, parser: &LogLineParser) {
        if !Path::new(&self.filename).exists() {
            log::warn!(
                "Log file does not exist (yet?), or wrong path? Waiting a bit... Filename: {}",
                self.filename
            );
            sleep(FILE_NOT_EXIST_DELAY);
            return;
        }

        // log::info!("Processing new data");
        let new_data = self.read_new_data();
        if let Ok(new_data) = new_data {
            let lines: Vec<&str> = new_data.lines().collect();
            log::debug!("Got {} new lines in the logfile", lines.len());
            for line in lines {
                let msg = parser.parse_line(line);
                if let Some(msg) = msg {
                    let mut bus = self.bus.lock().unwrap();
                    bus.send_logline(msg);
                }
            }
        } else {
            log::error!("Error reading new data. Moving to last position in file.");
            self.goto_last_pos();
        }
    }

    fn read_new_data(&mut self) -> BoxResult<String> {
        let mut file = fs::File::open(self.filename.as_str())?;

        // Get new file length, if same as old, we're done.
        let file_len = file.metadata()?.len();
        if file_len == self.last_pos {
            return Ok("".to_string());
        }

        // File was truncated, start from beginning
        if file_len < self.last_pos {
            self.last_pos = 0;
        }

        // Seek to the position we last read from
        file.seek(SeekFrom::Start(self.last_pos))?;

        // Read the portion of the file that is new
        let buf_size = (file_len - self.last_pos) as usize;
        let mut buf: Vec<u8> = vec![0; buf_size];
        file.read_exact(&mut buf)?;

        let s = String::from_utf8_lossy(&buf).to_string();

        self.last_pos += buf_size as u64;

        Ok(s)
    }
}
