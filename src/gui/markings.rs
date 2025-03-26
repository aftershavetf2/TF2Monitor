use super::colors::{color_for_flag, hexrgb};
use crate::{
    tf2::lobby::{player_attribute_description, Player},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{Color32, Ui};

pub fn add_flags(ui: &mut Ui, player: &Player) {
    ui.horizontal_wrapped(|ui| {
        ui.set_max_width(140.0);

        if let Some(player_info) = &player.player_info {
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
        }

        add_reputation(ui, player);
    });
}

fn add_reputation(ui: &mut Ui, player: &Player) {
    ui.scope(|ui| {
        let mut text = "rep?";
        let mut tooltip = "Still loading reputation...".to_string();
        let mut fgcolor = Color32::BLACK;
        let mut bgcolor = hexrgb(0xaaaaaa);

        if let Some(reputation) = &player.reputation {
            if reputation.has_bad_reputation {
                fgcolor = Color32::BLACK;
                bgcolor = Color32::RED;
                text = "-rep";
                tooltip = format!(
                    "Bans:\n{}",
                    reputation
                        .bans
                        .iter()
                        .map(|ban| format!("- {} for {}", ban.source, ban.reason))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .as_str()
                );
            } else {
                fgcolor = Color32::BLACK;
                bgcolor = Color32::GREEN;
                text = "+rep";
                tooltip = "No SourceBans".to_string();
            }
        }

        ui.style_mut().visuals.override_text_color = Some(fgcolor);

        ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = bgcolor;

        let _ = ui.button(text).on_hover_text(tooltip);
    });
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

        let (text, tooltip) = player_attribute_description(player_attribute);

        let _ = ui.button(text).on_hover_text(tooltip);
    });
}
