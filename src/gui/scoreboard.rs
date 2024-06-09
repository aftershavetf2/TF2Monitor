use super::{recently_left::add_recently_left_players, scoreboard_team::scoreboard_team};
use crate::{
    models::AppWin,
    tf2::lobby::{Player, Team},
};
use eframe::egui::{Color32, Ui};

pub fn add_scoreboard(app_win: &mut AppWin, ui: &mut Ui) {
    let mut sorted_players: Vec<Player> = app_win.lobby.players.clone();

    ui.horizontal(|ui| {
        if ui.button("Swap team colors").clicked() {
            app_win.swap_team_colors = !app_win.swap_team_colors;
        }
        ui.checkbox(&mut app_win.show_crits, "Show crits");
        ui.checkbox(&mut app_win.show_friendships, "Show friendships");
    });

    if app_win.swap_team_colors {
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

    let blu_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Invaders)
        .collect();

    let red_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Defendes)
        .collect();

    ui.columns(2, |ui| {
        scoreboard_team(app_win, &mut ui[0], "Blu", &blu_players, "blu");
        scoreboard_team(app_win, &mut ui[1], "Red", &red_players, "red");
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

    add_recently_left_players(app_win, ui);
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
