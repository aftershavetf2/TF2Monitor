#![allow(dead_code)]
use eframe::egui;
use eframe::egui::Color32;

use crate::tf2bd::models::PlayerAttribute;

// Colors taken from https://lospec.com/palette-list/team-fortress-2-official

pub const DARK_BLUE: Color32 = hex_to_rgb(0x395c78);
pub const BLUE: Color32 = hex_to_rgb(0x5b7a8c);
pub const GREEN_BLUE: Color32 = hex_to_rgb(0x5b7a8c);
pub const GRAY: Color32 = hex_to_rgb(0x6b6a65);
pub const DARK_BROWN_GRAY: Color32 = hex_to_rgb(0x34302d);

pub const DARK_BROWN: Color32 = hex_to_rgb(0x462d26);
pub const BROWN: Color32 = hex_to_rgb(0x6a4535);
pub const RED_BROWN: Color32 = hex_to_rgb(0x913a1e);
pub const RED: Color32 = hex_to_rgb(0xbd3b3b);
pub const DARK_RED: Color32 = hex_to_rgb(0x9d312f);

pub const RED_ORANGE: Color32 = hex_to_rgb(0xf08149);
pub const ORANGE: Color32 = hex_to_rgb(0xef9849);
pub const PINK: Color32 = hex_to_rgb(0xf5ad87);
pub const BEIGE: Color32 = hex_to_rgb(0xf6b98a);
pub const PINK_WHITE: Color32 = hex_to_rgb(0xf5e7de);

pub const LIGHT_BROWN: Color32 = hex_to_rgb(0xc1a18a);
pub const LIGHT_BEIGE: Color32 = hex_to_rgb(0xdabdab);

pub const TEAM_BLU_COLOR: Color32 = BLUE;
pub const TEAM_RED_COLOR: Color32 = RED;

pub const CHAT_BLU_COLOR: Color32 = hex_to_rgb(0x99CCFF);
pub const CHAT_RED_COLOR: Color32 = hex_to_rgb(0xFF4040);

pub const PANEL_FILL: Color32 = hex_to_rgb(0x36312B);
// pub const WIDGET_FILL: Color32 = DARK_BROWN_GRAY;
pub const TEXT_COLOR: Color32 = hex_to_rgb(0xEBE2CA);

pub fn set_style(ctx: &egui::Context) {
    let mut style = egui::Visuals::dark().clone();

    style.dark_mode = true;

    style.panel_fill = PANEL_FILL;
    style.override_text_color = Some(TEXT_COLOR);
    // // style.widgets.inactive.bg_fill = Color32::BLACK;
    // style.widgets.inactive.weak_bg_fill = hexrgb(0x756B5E);
    // style.widgets.inactive.bg_fill = hexrgb(0x756B5E);

    ctx.set_visuals(style);
}

/// Returns the (text color, background color) for a given PlayerAttribute.
pub fn color_for_flag(player_attribute: PlayerAttribute) -> (Color32, Color32) {
    match player_attribute {
        PlayerAttribute::Cool => (Color32::BLACK, Color32::GOLD),
        PlayerAttribute::Cheater => (Color32::BLACK, hex_to_rgb(0xff006e)),
        PlayerAttribute::Bot => (Color32::BLACK, hex_to_rgb(0xff0000)),
        PlayerAttribute::Suspicious => (Color32::BLACK, Color32::from_rgb(0xf0, 0x81, 0x49)),
        PlayerAttribute::Toxic => (Color32::BLACK, Color32::WHITE),
        PlayerAttribute::Exploiter => (Color32::BLACK, Color32::LIGHT_GREEN),
    }
}

pub const fn hex_to_rgb(hex: u32) -> Color32 {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}
