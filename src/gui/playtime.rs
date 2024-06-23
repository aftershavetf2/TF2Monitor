use crate::tf2::lobby::{Player, Tf2PlayMinutes};
use eframe::egui::{Color32, Ui};

pub fn add_playtime(ui: &mut Ui, player: &Player) {
    match player.tf2_play_minutes {
        Tf2PlayMinutes::Loading => {
            ui.spinner();
        }
        Tf2PlayMinutes::PlayMinutes(minutes) => {
            ui.label(format!("{}h", minutes / 60));
        }
        Tf2PlayMinutes::Unknown => {
            ui.colored_label(Color32::GRAY, "-");
        }
    }
}
