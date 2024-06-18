use super::{recently_left::add_recently_left_players, scoreboard_team::scoreboard_team};
use crate::{
    appbus::AppEventMsg,
    models::AppWin,
    tf2::lobby::{Player, Team},
};
use eframe::egui::{Color32, Ui};

pub fn add_scoreboard(app_win: &mut AppWin, ui: &mut Ui) {
    // Row with buttons
    ui.horizontal(|ui| {
        if ui.button("Swap team colors").clicked() {
            app_win.swap_team_colors = !app_win.swap_team_colors;
        }
        ui.checkbox(&mut app_win.show_crits, "Show crits");
        ui.checkbox(&mut app_win.show_friendships, "Show friendships");

        if ui
            .checkbox(&mut app_win.kick_cheaters, "Kick cheaters")
            .changed()
        {
            app_win
                .bus
                .lock()
                .unwrap()
                .app_event_bus
                .broadcast(AppEventMsg::KickCheaters(app_win.kick_cheaters))
        }

        if ui.checkbox(&mut app_win.kick_bots, "Kick bots").changed() {
            app_win
                .bus
                .lock()
                .unwrap()
                .app_event_bus
                .broadcast(AppEventMsg::KickBots(app_win.kick_bots))
        }
    });

    ui.separator();

    // Player list, also check if there are teams at all
    let mut sorted_players: Vec<Player> = app_win.lobby.players.clone();
    sorted_players.sort_by(cmp_for_scoreboard);

    if app_win.swap_team_colors {
        sorted_players.iter_mut().for_each(|p| {
            p.team = match p.team {
                Team::Invaders => Team::Defendes,
                Team::Defendes => Team::Invaders,
                x => x,
            }
        });
    }

    let blu_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Invaders)
        .collect();
    let red_players: Vec<&Player> = sorted_players
        .iter()
        .filter(|p| p.team == Team::Defendes)
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
