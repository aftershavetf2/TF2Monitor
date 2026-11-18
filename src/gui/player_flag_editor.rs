use super::theme;
use crate::{
    appbus::AppEventMsg, models::AppWin, tf2::lobby::Player, tf2bd::models::PlayerAttribute,
};
use eframe::egui::Ui;

pub fn add_player_flag_editor(app_win: &AppWin, ui: &mut Ui, player: &Player) {
    ui.heading("Flags:");
    ui.horizontal_wrapped(|ui| {
        let player_attributes_to_show = vec![
            PlayerAttribute::Cool,
            PlayerAttribute::Cheater,
            PlayerAttribute::Bot,
            PlayerAttribute::Suspicious,
            PlayerAttribute::Toxic,
            PlayerAttribute::Exploiter,
        ];

        let actual_player_attributes = if let Some(player_info) = &player.player_info {
            &player_info.attributes
        } else {
            &vec![]
        };

        for player_attribute in player_attributes_to_show {
            let enable = actual_player_attributes.contains(&player_attribute);
            add_flag(app_win, ui, player_attribute, enable, player);
        }
    });
}

fn add_flag(
    app_win: &AppWin,
    ui: &mut Ui,
    player_attribute: PlayerAttribute,
    mut enable: bool,
    player: &Player,
) {
    if theme::flag_checkbox(ui, &mut enable, player_attribute).changed() {
        app_win
            .bus
            .lock()
            .unwrap()
            .app_event_bus
            .broadcast(AppEventMsg::SetPlayerFlag {
                steamid: player.steamid,
                name: player.name.clone(),
                flag: player_attribute,
                enable,
            });
    }
}
