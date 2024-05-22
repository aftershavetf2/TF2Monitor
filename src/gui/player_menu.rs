use crate::{appbus::AppBus, tf2::lobby::Player};
use eframe::egui::Ui;
use std::sync::{Arc, Mutex};

pub fn add_player_menu(ui: &mut Ui, bus: &Arc<Mutex<AppBus>>, player: &Player) {
    ui.menu_button("☰", |ui| {
        ui.heading(player.name.clone());
        ui.separator();

        // First section is for viewing player profiles
        fn make_link(ui: &mut Ui, url: String, text: &str) {
            if ui.hyperlink_to(text, url).clicked() {
                ui.close_menu();
            }
        }

        ui.label("View on");
        make_link(ui, player.steamid.steam_community_url(), "- SteamCommunity");
        make_link(ui, player.steamid.steam_history_url(), "- SteamHistory");
        make_link(ui, player.steamid.steam_rep_url(), "- SteamRep");
        make_link(ui, player.steamid.steam_id_url(), "- SteamID");

        ui.separator();

        // Second section is for kicking players
        ui.label("Kick for");
        if ui.button("- Cheating").clicked() {
            log::info!("Vote to kick player '{}' for cheating", player.name);
            let cmd = format!("callvote kick \"{} cheating\"", player.id);
            bus.lock().unwrap().send_rcon_cmd(cmd.as_str());
        }
    })
    .response
    .on_hover_text("Actions for this player");
}
