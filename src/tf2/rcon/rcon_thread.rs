use super::rcon_connection::{RConArgs, RConConnection};
use crate::appbus::AppBus;
use crate::models::app_settings::AppSettings;
use crate::tf2::rcon::g15_dumpplayer_parser::G15DumpPlayerParser;
use crate::utils::BoxResult;
use bus::BusReader;
use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{self, Duration},
};

/// The delay between RCON commands
const RCON_DELAY: Duration = time::Duration::from_millis(10);

/// The delay between loops in run()
const LOOP_DELAY: Duration = time::Duration::from_millis(5000);

/// Start the background thread for the rcon module
pub fn start(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>) -> thread::JoinHandle<()> {
    let mut rcon_thread = RconThread::new(settings, bus);

    thread::spawn(move || rcon_thread.run())
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
            bus: Arc::clone(bus),
            rcon_args,
            rcon_bus_rx,
        }
    }

    pub fn run(&mut self) {
        log::info!("Rcon background thread started");

        let g15_dumpplayer_parser = G15DumpPlayerParser::new();

        loop {
            if let Some(reply) = self.send_rcon_command("g15_dumpplayer") {
                // let start_time = std::time::Instant::now();

                let parsed_data = g15_dumpplayer_parser.parse(&reply);

                // let stop_time = std::time::Instant::now();
                // log::info!(
                //     "Parsing of g15_dumpplayer's {} chars reply took {:?}",
                //     reply.len(),
                //     stop_time - start_time
                // );

                // if !parsed_data.players.is_empty()
                {
                    // log::info!("Parsed g15_dumpplayer: {:?}", parsed_data);
                    self.bus
                        .lock()
                        .unwrap()
                        .g15_report_bus
                        .broadcast(parsed_data);
                }
            }

            self.process_bus();

            sleep(LOOP_DELAY);
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
