use super::colors::color_for_flag;
use crate::{
    tf2::lobby::{player_attribute_description, Player},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::Ui;

pub fn add_flags(ui: &mut Ui, player: &Player) {
    if let Some(player_info) = &player.player_info {
        if player_info.attributes.is_empty() {
            return;
        }

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(140.0);

            let player_attributes_to_show = vec![
                PlayerAttribute::Cool,
                PlayerAttribute::Cheater,
                PlayerAttribute::Bot,
                PlayerAttribute::Suspicious,
                PlayerAttribute::Toxic,
                PlayerAttribute::Exploiter,
            ];

            for player_attribute in player_attributes_to_show {
                if player_info.attributes.contains(&player_attribute) {
                    add_flag(ui, player_attribute);
                }
            }
        });
    }
}

fn add_flag(ui: &mut Ui, player_attribute: PlayerAttribute) {
    ui.scope(|ui| {
        let (fgcolor, bgcolor) = color_for_flag(player_attribute);

        ui.style_mut().visuals.override_text_color = Some(fgcolor);

        ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = bgcolor;

        let text = player_attribute_description(player_attribute);

        let _ = ui.button(text);
    });
}
