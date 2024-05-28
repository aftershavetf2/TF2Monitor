use crate::tf2::lobby::Player;
use eframe::egui::{Image, Ui};

// pub const IMAGE_URL: &str =
//     "https://avatars.cloudflare.steamstatic.com/f39ba23bc07d2de9b77abcabae13ee2541f9c938_full.jpg";

pub fn add_player_tooltip(ui: &mut Ui, player: &Player) {
    ui.heading(format!("{} ({})", player.name, player.id));

    // ui.heading(format!("({}) {}", player.id, &player.name));

    if let Some(steam_info) = &player.steam_info {
        let image = Image::from_uri(&steam_info.avatarfull)
            .max_width(100.0)
            .rounding(3.0);

        ui.add(image);

        ui.label(format!(
            "Account created: {}",
            steam_info.get_account_created()
        ));
    }

    if let Some(playtime) = player.tf2_play_minutes {
        ui.label(format!("TF2 playtime: {} hours", playtime / 60));
    } else {
        ui.label("TF2 playtime: Loading...");
    }

    ui.label("");

    if let Some(friends) = &player.friends {
        ui.label(format!("{} friends", friends.len()));
    } else {
        ui.label("Loading friends...");
    }

    if let Some(reason) = player.has_steam_bans() {
        ui.label(reason);
    } else {
        ui.label("No Steam bans");
    }
}
