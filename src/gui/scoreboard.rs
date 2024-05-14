use crate::{
    models::steamid::SteamID,
    tf2::lobby::{Lobby, Player, Team},
};
use eframe::egui::{Color32, Ui};

use super::scoreboard_team::scoreboard_team;

const IMAGE_URL: &str =
    "https://avatars.cloudflare.steamstatic.com/f39ba23bc07d2de9b77abcabae13ee2541f9c938_full.jpg";

// const TEAM_NAMES: [&str; 2] = [" BLU ", " RED "];

pub fn add_scoreboard(
    ui: &mut Ui,
    self_steamid: SteamID,
    lobby: &mut Lobby,
    swap_team_colors: &mut bool,
    show_crits: &mut bool,
) {
    // ui.heading("Scoreboard");

    let mut sorted_players: Vec<Player> = lobby.players.clone();

    ui.horizontal(|ui| {
        if ui.button("Swap team colors").clicked() {
            *swap_team_colors = !*swap_team_colors;
        }
        ui.checkbox(show_crits, "Show crits");
    });

    if *swap_team_colors {
        sorted_players.iter_mut().for_each(|p| {
            p.team = match p.team {
                Team::Invaders => Team::Defendes,
                Team::Defendes => Team::Invaders,
                x => x,
            }
        });
    }

    ui.separator();

    sorted_players.sort_by(cmp_for_scoreboard);

    let blu: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Invaders)
        .collect();

    let red: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Defendes)
        .collect();

    ui.columns(2, |ui| {
        scoreboard_team(
            &mut ui[0],
            "Blu",
            self_steamid,
            &blu,
            "blu",
            swap_team_colors,
            show_crits,
        );
        scoreboard_team(
            &mut ui[1],
            "Red",
            self_steamid,
            &red,
            "red",
            swap_team_colors,
            show_crits,
        );
    });

    let spectator_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Spec)
        .collect();
    let unknown_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Unknown)
        .collect();

    if !spectator_players.is_empty() {
        let player_names: String = spectator_players
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        ui.colored_label(Color32::GRAY, format!("Spectators: {}", player_names));

        if !unknown_players.is_empty() {
            ui.separator();
        }
    }

    let unknown_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Unknown)
        .collect();
    if !spectator_players.is_empty() {
        let player_names: String = unknown_players
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        ui.colored_label(Color32::GRAY, format!("Joined: {}", player_names));
    }
}

fn cmp_for_scoreboard(a: &Player, b: &Player) -> std::cmp::Ordering {
    // Sort by team, kills(desc), deaths(desc), and lastly player name
    if a.team != b.team {
        return a.team.cmp(&b.team);
    }

    if a.kills != b.kills {
        return a.kills.cmp(&b.kills).reverse();
    }

    if a.deaths != b.deaths {
        return a.deaths.cmp(&b.deaths);
    }

    a.name.cmp(&b.name)
}
