use super::colors::{hexrgb, CHAT_BLU_COLOR, CHAT_RED_COLOR};
use crate::{
    models::AppWin,
    tf2::lobby::{LobbyKill, Team},
};
use eframe::egui::{text::LayoutJob, Color32, ScrollArea, TextFormat, TextStyle, Ui};

pub fn add_kill_feed(ui: &mut Ui, app_win: &mut AppWin) {
    ui.label("Kill feed");

    let text_style = TextStyle::Body;
    let row_height = ui.text_style_height(&text_style);
    let num_rows = app_win.lobby.kill_feed.len();

    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink(false)
        .show_rows(ui, row_height, num_rows, |ui, row_range| {
            ui.scope(|ui| {
                ui.style_mut().visuals.panel_fill = hexrgb(0xffffff);

                for row in row_range {
                    let row = &app_win.lobby.kill_feed[row].clone();
                    add_kill_row(ui, app_win, row);
                }
            });
        });
}

fn add_kill_row(ui: &mut Ui, app_win: &mut AppWin, kill_row: &LobbyKill) {
    ui.horizontal_wrapped(|ui| {
        let killer = app_win.lobby.get_player(None, Some(kill_row.killer));
        let victim = app_win.lobby.get_player(None, Some(kill_row.victim));

        let (killer_name, killer_team) = match killer {
            Some(player) => (&player.name, player.team),
            None => (&format!("{}", &kill_row.killer.to_u64()), Team::Unknown),
        };
        let (victim_name, victim_team) = match victim {
            Some(player) => (&player.name, player.team),
            None => (&format!("{}", &kill_row.victim.to_u64()), Team::Unknown),
        };

        let killer_color = match killer_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };
        let victim_color = match victim_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => Color32::GRAY,
        };

        let mut job = LayoutJob::default();

        // Uniceode skulls: "☠💀🕱"

        job.append(
            "💀 ",
            0.0,
            TextFormat {
                color: Color32::WHITE,
                ..Default::default()
            },
        );

        // Killer name in team color
        job.append(
            killer_name,
            0.0,
            TextFormat {
                color: killer_color,
                ..Default::default()
            },
        );

        job.append(
            " killed ",
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        // Victim name in team color
        job.append(
            victim_name,
            0.0,
            TextFormat {
                color: victim_color,
                ..Default::default()
            },
        );

        job.append(
            format!(" with {}", kill_row.weapon).as_str(),
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        ui.label(job)
    });
}
