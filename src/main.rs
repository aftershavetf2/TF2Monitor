mod appbus;
mod gui;
mod models;
mod tf2;
mod utils;

use appbus::AppBus;
use models::app_settings::AppSettings;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    simple_logger::SimpleLogger::new().init().unwrap();

    let settings = AppSettings::load_or_default();
    let bus = Arc::new(Mutex::new(AppBus::default()));

    tf2::start(&settings, &bus);

    gui::run(&settings, &bus)
}

// fn main() {
//     simple_logger::SimpleLogger::new().init().unwrap();

//     let settings = AppSettings::load_or_default();
//     let buses = Arc::new(Mutex::new(AppBus::default()));

//     test_steam_api(&settings);
// }

// fn test_steam_api(settings: &AppSettings) {
//     use crate::models::steamid::SteamID;

//     let mut steam_api = tf2::steam::SteamApi::new(settings);

//     // 76561199119901587,76561199289898291,76561199234573637,76561199191957545,76561199243399574,76561198064076891,76561199197733316,76561199174058886,76561198076719730,76561198884330277,76561199156370643,76561198090231678,76561198899495757,76561198147668557,76561198370670319,76561199467708430,76561198080281312,76561199379427015,76561198451145010,76561199400218364,76561199559924950,76561198999562072,76561199664152002,76561197974228301
//     let steamid64 = models::steamid::SteamID::from_steam_id32("[U:1:169802]");
//     let players = steam_api.get_player_summaries(vec![
//         SteamID::from_u64(76561199119901587),
//         SteamID::from_u64(76561199289898291),
//         SteamID::from_u64(76561199234573637),
//         SteamID::from_u64(76561199191957545),
//         SteamID::from_u64(76561199243399574),
//         SteamID::from_u64(76561198064076891),
//         SteamID::from_u64(76561199197733316),
//         SteamID::from_u64(76561199174058886),
//         SteamID::from_u64(76561198076719730),
//         SteamID::from_u64(76561198884330277),
//         SteamID::from_u64(76561199156370643),
//         SteamID::from_u64(76561198090231678),
//         SteamID::from_u64(76561198899495757),
//         SteamID::from_u64(76561198147668557),
//         SteamID::from_u64(76561198370670319),
//         SteamID::from_u64(76561199467708430),
//         SteamID::from_u64(76561198080281312),
//         SteamID::from_u64(76561199379427015),
//         SteamID::from_u64(76561198451145010),
//         SteamID::from_u64(76561199400218364),
//         SteamID::from_u64(76561199559924950),
//         SteamID::from_u64(76561198999562072),
//         SteamID::from_u64(76561199664152002),
//         SteamID::from_u64(76561197974228301),
//     ]);
//     println!("Players: {:?}", players);
// }
