use crate::tf2::lobby::{Player, Tf2PlayMinutes};
use eframe::egui::Ui;

use super::{markings::add_new_player_value, ui_utils::show_empty_value};

pub fn add_playtime(ui: &mut Ui, player: &Player) {
    match player.tf2_play_minutes {
        Tf2PlayMinutes::Loading => {
            ui.spinner().on_hover_text("Loading TF2 playtime...");
        }
        Tf2PlayMinutes::PlayMinutes(minutes) => {
            add_new_player_value(ui, format!("{}h", minutes / 60), minutes < 60 * 500);
        }
        Tf2PlayMinutes::Unknown => {
            show_empty_value(ui);
        }
    }
}
