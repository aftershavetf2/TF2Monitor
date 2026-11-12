use super::colors::color_for_flag;
use crate::{
    tf2::lobby::{Player, player_attribute_description},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{self, Color32, FontId, Response, Sense, TextStyle, Ui};

const BADGE_FONT_SIZE: f32 = 10.0;
const BADGE_PADDING: egui::Vec2 = egui::vec2(6.0, 4.0);
const BADGE_CORNER_RADIUS: f32 = 3.0;

pub fn add_reputation(ui: &mut Ui, player: &Player) {
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
    if let Some(tooltip) = &player.has_vac_bans() {
        add_badge(ui, "VAC", Color32::WHITE, Color32::DARK_RED, tooltip);
    }

    if let Some(tooltip) = &player.has_game_bans() {
        add_badge(ui, "GB", Color32::WHITE, Color32::DARK_RED, tooltip);
    }
}

fn add_badge(ui: &mut Ui, text: &str, fgcolor: Color32, bgcolor: Color32, tooltip: &str) {
    colored_box(
        ui,
        text,
        fgcolor,
        bgcolor,
        FontId::proportional(BADGE_FONT_SIZE),
    )
    .on_hover_text(tooltip);
}

pub fn add_new_player_value(ui: &mut Ui, text: String, highlight: bool) -> Response {
    if highlight {
        colored_box(
            ui,
            &text,
            Color32::WHITE,
            Color32::DARK_GREEN,
            TextStyle::Body.resolve(ui.style()),
        )
    } else {
        ui.label(text)
    }
}

fn colored_box(
    ui: &mut Ui,
    text: &str,
    fgcolor: Color32,
    bgcolor: Color32,
    font: FontId,
) -> Response {
    let text = ui.painter().layout_no_wrap(text.to_owned(), font, fgcolor);
    // Allocate directly in the wrapping row so each badge stays together.
    let (rect, response) =
        ui.allocate_exact_size(text.size() + 2.0 * BADGE_PADDING, Sense::hover());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text.text())
    });
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, BADGE_CORNER_RADIUS, bgcolor);
        let text_pos = rect.center() - text.size() / 2.0;
        ui.painter().galley(text_pos, text, fgcolor);
    }
    response
}
