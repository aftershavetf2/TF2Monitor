use super::{recently_left::add_recently_left_players, scoreboard_team::scoreboard_team};
use crate::{
    models::AppWin,
    tf2::lobby::{Player, Team},
};
use eframe::egui::{self, Color32, Ui};

pub fn add_scoreboard(app_win: &mut AppWin, ui: &mut Ui) {
    // Player list, also check if there are teams at all
    let mut sorted_players: Vec<Player> = app_win.lobby.players.clone();
    sorted_players.sort_by(cmp_for_scoreboard);

    let blu_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Blue)
        .collect();
    let red_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Red)
        .collect();
    let spectator_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Spec)
        .collect();
    let unknown_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Unknown)
        .collect();

    // If there's a lobby with red/blu teams, show the scoreboard
    if !blu_players.is_empty() || !red_players.is_empty() {
        ui.columns(2, |ui| {
            scoreboard_team(app_win, &mut ui[0], "Blu", &blu_players);
            scoreboard_team(app_win, &mut ui[1], "Red", &red_players);
        });
    } else if !unknown_players.is_empty() {
        // Make two teams of a maximum of 12 players each,
        // the player list is sorted so after 24 players there are players
        // with low scores
        let team1: Vec<&Player> = unknown_players.iter().take(12).copied().collect();
        let team2: Vec<&Player> = unknown_players.iter().skip(12).take(12).copied().collect();
        let rest_team: Vec<&Player> = unknown_players.iter().skip(24).copied().collect();
        ui.columns(2, |ui| {
            scoreboard_team(app_win, &mut ui[0], "Players 1-12", &team1);
            scoreboard_team(app_win, &mut ui[1], "Players 13-24", &team2);
        });

        if !rest_team.is_empty() {
            ui.separator();
            let player_names: String = rest_team
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<String>>()
                .join(", ");
            ui.colored_label(Color32::GRAY, format!("Unknown team: {}", player_names));
        }
    }

    if !spectator_players.is_empty() {
        let player_names: String = spectator_players
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<String>>()
            .join(", ");
        ui.colored_label(Color32::WHITE, format!("Spectators: {}", player_names));

        if !unknown_players.is_empty() {
            ui.separator();
        }
    }

    add_recently_left_players(app_win, ui);
}

fn cmp_for_scoreboard(a: &Player, b: &Player) -> std::cmp::Ordering {
    // Sort by team, score(desc), kills(desc), deaths(asc), and lastly player name
    if a.team != b.team {
        return a.team.cmp(&b.team);
    }

    // if a.score != b.score {
    //     return a.score.cmp(&b.score).reverse();
    // }

    if a.kills != b.kills {
        return a.kills.cmp(&b.kills).reverse();
    }

    if a.deaths != b.deaths {
        return a.deaths.cmp(&b.deaths);
    }

    a.name.cmp(&b.name)
}
