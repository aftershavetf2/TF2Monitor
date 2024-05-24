use crate::models::AppWin;
use eframe::egui::Ui;

pub fn add_recently_left_players(app_win: &mut AppWin, ui: &mut Ui) {
    if app_win.lobby.recently_left_players.is_empty() {
        return;
    }

    ui.separator();

    ui.horizontal_wrapped(|ui| {
        // if ui.button("Clear").clicked() {
        //     app_win.lobby.recently_left_players.clear();
        // }

        ui.label("Recent players:\n");
        ui.separator();

        let mut is_first = true;
        for player in &app_win.lobby.recently_left_players {
            if is_first {
                is_first = false;
            } else {
                ui.label(", ");
            }

            ui.hyperlink_to(player.name.as_str(), player.steamid.steam_history_url())
                .on_hover_text("View on Steam History");
        }
    });
}
