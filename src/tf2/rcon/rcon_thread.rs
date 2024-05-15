use super::rcon_connection::{RConArgs, RConConnection};
use crate::models::app_settings::AppSettings;
use crate::utils::BoxResult;
use crate::{appbus::AppBus, tf2::logfile::LogLine};
use bus::BusReader;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{self, Duration},
};

/// The delay between RCON commands
const RCON_DELAY: Duration = time::Duration::from_millis(500);

/// The delay between loops in run()
const LOOP_DELAY: Duration = time::Duration::from_millis(1000);

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut rcon_thread = RconThread::new(settings, bus);

    thread::spawn(move || rcon_thread.run())
}

static LOBBY_DEBUG_RX: Lazy<Regex> = regex_static::lazy_regex!(
    r#"^\s{2}(Member|Pending)\[\d+]\s+(?P<sid>\[.+?]).+?TF_GC_TEAM_(?P<team>(DEFENDERS|INVADERS))\s{2}type\s=\sMATCH_PLAYER$"#
);

fn parse_lobby_debug_line(line: &str) -> Option<(String, String)> {
    if let Some(caps) = LOBBY_DEBUG_RX.captures(line) {
        let sid = caps.name("sid").unwrap().as_str().to_string();
        let team = caps.name("team").unwrap().as_str().to_string();
        return Some((sid, team));
    }
    None
}

pub struct RconThread {
    bus: Arc<Mutex<AppBus>>,
    rcon_args: RConArgs,
    rcon_bus_rx: BusReader<String>,
}

impl RconThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let mut rcon_args = RConArgs::new();
        rcon_args.ip.clone_from(&settings.rcon_ip);
        rcon_args.port = settings.rcon_port;
        rcon_args.password.clone_from(&settings.rcon_password);

        let rcon_bus_rx = bus.lock().unwrap().rcon_bus.add_rx();

        Self {
            bus: Arc::clone(&bus),
            rcon_args,
            rcon_bus_rx,
        }
    }

    pub fn run(&mut self) {
        log::info!("Rcon background thread started");

        loop {
            self.send_rcon_command("tf_lobby_debug", true);
            self.send_rcon_command("status", false);

            self.process_bus();

            sleep(LOOP_DELAY);
        }
    }

    fn send_rcon_command(&self, cmd: &str, log_result: bool) {
        match self.send_command_internal(cmd) {
            Ok(reply) => {
                log::debug!("RCON '{}' replied: '{}'", cmd, reply);
                if log_result {
                    self.send_as_logline(&reply);
                }
            }
            Err(error) => {
                log::error!("RCON: '{}' failed: '{:?}'", cmd, error);
            }
        }
    }

    fn process_bus(&mut self) {
        while let Ok(cmd) = self.rcon_bus_rx.try_recv() {
            self.send_rcon_command(&cmd, false);
        }
    }

    /// This takes the reply from tf_lobby_debug and
    /// sends it as bunch of LogLine::PlayerTeam
    /// to the listeners of the logfile bus
    fn send_as_logline(&self, msg: &str) {
        let lines = msg.lines();
        for line in lines {
            if let Some((steam_id32, team)) = parse_lobby_debug_line(line) {
                let logline = LogLine::PlayerTeam { steam_id32, team };
                self.bus.lock().unwrap().send_logline(logline);
            }
        }
    }

    fn send_command_internal(&self, cmd: &str) -> BoxResult<String> {
        sleep(RCON_DELAY);

        let mut rcon_client = RConConnection::new(&self.rcon_args)?;

        log::debug!("Sending RCON authorize: {}", cmd);
        rcon_client.authorize()?;

        log::debug!("Sending RCON command: {}", cmd);
        let reply = rcon_client.exec_command(cmd)?;
        return Ok(reply);
    }
}
