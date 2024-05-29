use crate::models::AppWin;
use eframe::egui::Ui;

use super::player_tooltip::add_player_tooltip;

pub fn add_recently_left_players(app_win: &mut AppWin, ui: &mut Ui) {
    if app_win.lobby.recently_left_players.is_empty() {
        return;
    }

    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.label("Recent players:\n");
        ui.separator();

        for player in &app_win.lobby.recently_left_players {
            // ui.horizontal_wrapped(|ui| {
            if let Some(steam_info) = &player.steam_info {
                ui.image(&steam_info.avatar)
                    .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));
            }

            if ui
                .label(player.name.as_str())
                .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player))
                .clicked()
            {
                app_win.selected_player = Some(player.steamid);
            }

            // ui.hyperlink_to(player.name.as_str(), player.steamid.steam_history_url())
            //     .on_hover_text("Click to view on Steam History")
            //     .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));
            // });
        }
    });
}
