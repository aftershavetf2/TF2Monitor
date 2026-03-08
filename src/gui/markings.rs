use super::colors::color_for_flag;
use crate::{
    tf2::lobby::{Player, player_attribute_description},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{Button, Color32, TextWrapMode, Ui};

pub fn add_reputation(ui: &mut Ui, player: &Player) {
    ui.horizontal_wrapped(|ui| {
        add_team_symbol_basic_rep(ui, player);
        add_flags(ui, player);

        // Show reputation (SourceBans)
        if player.reputation.is_none() {
            ui.spinner().on_hover_text("Fetching SourceBans...");
        } else if let Some(reputation) = &player.reputation {
            if reputation.has_bad_reputation {
                let text = "SB";
                let tooltip = format!(
                    "SourceBans:\n{}",
                    reputation
                        .source_bans
                        .iter()
                        .map(|ban| format!("- {} for {}", ban.source, ban.reason))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .as_str()
                );

                add_badge(ui, text, Color32::BLACK, Color32::ORANGE, tooltip.as_str());
            }
        }
    });
}

pub fn add_flags(ui: &mut Ui, player: &Player) {
    let player_attributes_to_show = vec![
        PlayerAttribute::Cool,
        PlayerAttribute::Cheater,
        PlayerAttribute::Bot,
        PlayerAttribute::Suspicious,
        PlayerAttribute::Toxic,
        PlayerAttribute::Exploiter,
    ];

    if let Some(player_info) = &player.player_info {
        for player_attribute in player_attributes_to_show {
            if player_info.attributes.contains(&player_attribute) {
                add_flag(ui, player_attribute);
            }
        }
    }
}

fn add_flag(ui: &mut Ui, player_attribute: PlayerAttribute) {
    let (fgcolor, bgcolor) = color_for_flag(player_attribute);

    let (text, tooltip) = player_attribute_description(player_attribute);

    add_badge(ui, text, fgcolor, bgcolor, tooltip);
}

fn add_team_symbol_basic_rep(ui: &mut Ui, player: &Player) {
    if let Some(tooltip) = &player.is_newbie() {
        add_badge(ui, "NEW", Color32::WHITE, Color32::DARK_GREEN, tooltip);
    }

    if let Some(tooltip) = &player.has_vac_bans() {
        add_badge(ui, "VAC", Color32::WHITE, Color32::DARK_RED, tooltip);
    }

    if let Some(tooltip) = &player.has_game_bans() {
        add_badge(ui, "GB", Color32::WHITE, Color32::DARK_RED, tooltip);
    }
}

fn add_badge(ui: &mut Ui, text: &str, fgcolor: Color32, bgcolor: Color32, tooltip: &str) {
    ui.scope(|ui| {
        ui.style_mut().visuals.override_text_color = Some(fgcolor);

        ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = bgcolor;

        // ui.style_mut().override_text_style = Some(eframe::egui::TextStyle::Small);

        let _ = ui
            .add(Button::new(text).wrap_mode(TextWrapMode::Extend))
            .on_hover_text(tooltip);
    });
}
