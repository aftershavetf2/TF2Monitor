use super::theme;
use crate::{
    models::{AppWin, player_markings::MarkingCommand},
    tf2::lobby::Player,
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::Ui;

pub fn add_player_flag_editor(app_win: &AppWin, ui: &mut Ui, player: &Player) {
    ui.heading("Flags:");
    if let Some(error) = app_win.marking_error.borrow().as_ref() {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
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
        let result = app_win.send_marking_command(MarkingCommand::SetFlag {
            steamid: player.steamid,
            name: player.name.clone(),
            flag: player_attribute,
            enable,
        });
        app_win.marking_error.replace(result.err());
    }
}
