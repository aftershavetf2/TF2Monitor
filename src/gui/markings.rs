use super::colors::color_for_flag;
use super::ui_utils::show_empty_value;
use crate::{
    tf2::lobby::{player_attribute_description, Player},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{Color32, Ui};

pub fn add_reputation(ui: &mut Ui, player: &Player) {
    ui.horizontal_wrapped(|ui| {
        // Show reputation (SourceBans)
        if player.reputation.is_none() {
            ui.spinner().on_hover_text("Fetching SourceBans...");
        } else if let Some(reputation) = &player.reputation {
            if reputation.has_bad_reputation {
                ui.scope(|ui| {
                    let fgcolor = Color32::BLACK;
                    let bgcolor = Color32::RED;
                    let text = "SB";
                    let tooltip = format!(
                        "SourceBans:\n{}",
                        reputation
                            .bans
                            .iter()
                            .map(|ban| format!("- {} for {}", ban.source, ban.reason))
                            .collect::<Vec<String>>()
                            .join("\n")
                            .as_str()
                    );

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
        }

        add_flags(ui, player, false);
    });
}

pub fn add_flags(ui: &mut Ui, player: &Player, add_empty_value: bool) {
    let player_attributes_to_show = vec![
        PlayerAttribute::Cool,
        PlayerAttribute::Cheater,
        PlayerAttribute::Bot,
        PlayerAttribute::Suspicious,
        PlayerAttribute::Toxic,
        PlayerAttribute::Exploiter,
    ];

    let mut added_flags = false;

    if let Some(player_info) = &player.player_info {
        ui.horizontal_wrapped(|ui| {
            // ui.set_max_width(140.0);
            for player_attribute in player_attributes_to_show {
                if player_info.attributes.contains(&player_attribute) {
                    add_flag(ui, player_attribute);
                    added_flags = true;
                }
            }
        });
    }

    if !added_flags && add_empty_value {
        show_empty_value(ui);
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

        let (text, tooltip) = player_attribute_description(player_attribute);

        let _ = ui.button(text).on_hover_text(tooltip);
    });
}
