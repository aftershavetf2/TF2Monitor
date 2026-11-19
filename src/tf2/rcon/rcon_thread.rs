use super::rcon_connection::{RConArgs, RConConnection};
use crate::config::{RCON_DELAY, RCON_LOOP_DELAY};
use crate::models::app_settings::AppSettings;
use crate::tf2::lobby::shared_lobby::SharedLobby;
use crate::utils::BoxResult;
use crate::{appbus::AppBus, tf2::rcon::g15_dumpplayer_parser::G15DumpPlayerParser};
use bus::BusReader;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
};

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut rcon_thread = RconThread::new(settings, bus);

    thread::spawn(move || rcon_thread.run())
}

pub struct RconThread {
    bus: Arc<Mutex<AppBus>>,
    rcon_args: RConArgs,
    rcon_bus_rx: BusReader<String>,
    shared_lobby: SharedLobby,
}

impl RconThread {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> Self {
        let mut rcon_args = RConArgs::new();
        rcon_args.ip.clone_from(&settings.rcon_ip);
        rcon_args.port = settings.rcon_port;
        rcon_args.password.clone_from(&settings.rcon_password);

        let mut app_bus = bus.lock().unwrap();
        let rcon_bus_rx = app_bus.rcon_bus.add_rx();
        let shared_lobby = app_bus.shared_lobby.clone();

        Self {
            bus: Arc::clone(bus),
            rcon_args,
            rcon_bus_rx,
            shared_lobby,
        }
    }

    pub fn run(&mut self) {
        log::info!("Rcon background thread started");

        let mut g15_dumpplayer_parser = G15DumpPlayerParser::new();

        loop {
            self.poll_g15(&mut g15_dumpplayer_parser);
            self.process_bus();

            sleep(RCON_LOOP_DELAY);
        }
    }

    fn poll_g15(&self, parser: &mut G15DumpPlayerParser) {
        let generation = self.shared_lobby.g15_generation();
        parser.begin_generation(generation);
        let Some(reply) = self.send_rcon_command("g15_dumpplayer") else {
            return;
        };
        if self.shared_lobby.g15_generation() != generation {
            return;
        }

        let parsed_data = parser.parse(&reply);
        if !parsed_data.players.is_empty() {
            self.bus
                .lock()
                .unwrap()
                .g15_report_bus
                .broadcast(parsed_data);
        }
    }

    fn send_rcon_command(&self, cmd: &str) -> Option<String> {
        match self.send_command_internal(cmd) {
            Ok(reply) => {
                // log::info!("RCON '{}' replied: start'{}'end", cmd, reply);
                // log::info!("RCON command '{}' replied with {} chars", cmd, reply.len());

                Some(reply)
            }
            Err(error) => {
                log::info!("RCON: '{}' failed: '{:?}'", cmd, error);
                log::warn!("Could not talk to TF2 using RCON.");

                None
            }
        }
    }

    fn process_bus(&mut self) {
        while let Ok(cmd) = self.rcon_bus_rx.try_recv() {
            self.send_rcon_command(&cmd);
        }
    }

    fn send_command_internal(&self, cmd: &str) -> BoxResult<String> {
        sleep(RCON_DELAY);

        let mut rcon_client = RConConnection::new(&self.rcon_args)?;

        log::debug!("Sending RCON authorize: {}", cmd);
        rcon_client.authorize()?;

        log::debug!("Sending RCON command: {}", cmd);
        let reply = rcon_client.exec_command(cmd)?;
        Ok(reply)
    }
}
