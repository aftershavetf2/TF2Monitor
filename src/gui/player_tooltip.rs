use eframe::egui::{Image, Ui};

use crate::tf2::lobby::{Player, PlayerKill};

// pub const IMAGE_URL: &str =
//     "https://avatars.cloudflare.steamstatic.com/f39ba23bc07d2de9b77abcabae13ee2541f9c938_full.jpg";

pub fn add_player_tooltip(ui: &mut Ui, player: &Player) {
    ui.heading(player.name.clone());

    // let image = Image::from_uri(IMAGE_URL).max_width(100.0).rounding(3.0);

    // ui.add(image);

    // Get the last 10 kill's weapon names
    let kills: Vec<&PlayerKill> = player.kills_with.iter().rev().take(10).collect();

    ui.label("Latest kills used:");
    for player_kill in kills {
        if player_kill.crit {
            ui.label(format!("{} (crit)", player_kill.weapon));
        } else {
            ui.label(player_kill.weapon.clone());
        }
    }
}
