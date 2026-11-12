use super::colors::{CHAT_BLU_COLOR, CHAT_RED_COLOR};
use crate::{
    models::AppWin,
    tf2::lobby::{LobbyKill, Team},
};
use eframe::egui::{Color32, ScrollArea, TextFormat, TextStyle, Ui, text::LayoutJob};

pub fn add_kill_feed(ui: &mut Ui, app_win: &mut AppWin) {
    ui.heading("Kill Feed");

    ui.push_id("kill_feed", |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.spacing_mut().interact_size.y = ui.text_style_height(&TextStyle::Body);
        ui.spacing_mut().button_padding.y = 1.0;

        // Entries can wrap or contain badges, so use their actual heights.
        ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &app_win.lobby.kill_feed.clone() {
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
            None => (&kill_row.killer_name, Team::Unknown),
        };
        let (victim_name, victim_team) = match victim {
            Some(player) => (&player.name, player.team),
            None => (&kill_row.victim_name, Team::Unknown),
        };

        let mut killer_color = match killer_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => super::theme::muted_text(),
        };
        let mut victim_color = match victim_team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => super::theme::muted_text(),
        };

        // If the killer or the victim is Me, use white color
        if app_win.is_me(Some(kill_row.killer)) {
            killer_color = Color32::WHITE;
        }
        if app_win.is_me(Some(kill_row.victim)) {
            victim_color = Color32::WHITE;
        }

        let mut job = LayoutJob::default();

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
                color: super::theme::muted_text(),
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
            " with ",
            0.0,
            TextFormat {
                color: super::theme::muted_text(),
                ..Default::default()
            },
        );

        job.append(
            &kill_row.weapon.replace('_', " "),
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        // Add "(crit)" if it was a critical hit
        if kill_row.crit {
            job.append(
                " (crit)",
                0.0,
                TextFormat {
                    color: super::theme::muted_text(),
                    ..Default::default()
                },
            );
        }

        ui.label(job)
    });
}
