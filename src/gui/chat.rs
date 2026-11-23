use super::{
    colors::{CHAT_BLU_COLOR, CHAT_RED_COLOR},
    markings::{add_flags, add_tf2bd_badge},
};
use crate::{
    models::AppWin,
    tf2::lobby::{LobbyChat, Team},
};
use eframe::egui::{Color32, ScrollArea, TextFormat, TextStyle, Ui, text::LayoutJob};

pub fn add_chat(ui: &mut Ui, app_win: &mut AppWin) {
    ui.heading("Chat");

    ui.push_id("chat", |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.spacing_mut().interact_size.y = ui.text_style_height(&TextStyle::Body);
        ui.spacing_mut().button_padding.y = 1.0;

        // Entries can wrap or contain badges, so use their actual heights.
        ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &app_win.lobby.chat.clone() {
                    add_chat_row(ui, app_win, row);
                }
            });
    });
}

fn add_chat_row(ui: &mut Ui, app_win: &mut AppWin, chat_row: &LobbyChat) {
    ui.horizontal_wrapped(|ui| {
        let player = app_win.lobby.get_player(None, Some(chat_row.steamid));

        // In the case the player left, the name from the log file is used
        // And show an unknown team color
        let mut player_name = &chat_row.player_name;
        let mut team: Team = Team::Unknown;

        if let Some(player) = player {
            player_name = &player.name;
            team = player.team;

            add_flags(ui, player);
        }

        add_tf2bd_badge(ui, chat_row.steamid, &app_win.tf2bd_lists);

        let color = match team {
            Team::Blue => CHAT_BLU_COLOR,
            Team::Red => CHAT_RED_COLOR,
            _ => super::theme::muted_text(),
        };

        let mut job = LayoutJob::default();

        // Prefix player name with *DEAD* and (TEAM) if needed
        if chat_row.dead {
            job.append(
                "*DEAD* ",
                0.0,
                TextFormat {
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );
        }

        if chat_row.team {
            job.append(
                "(TEAM) ",
                0.0,
                TextFormat {
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );
        }

        // Player name
        job.append(
            player_name,
            0.0,
            TextFormat {
                color,
                ..Default::default()
            },
        );

        // The : between player name and the chat message
        job.append(
            ": ",
            0.0,
            TextFormat {
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );

        job.append(
            &chat_row.message,
            0.0,
            TextFormat {
                color: Color32::from_rgb(210, 210, 210),
                ..Default::default()
            },
        );

        if app_win.app_settings.translate_chat
            && let Some(translation) = chat_row.translated_message.as_deref()
            && !translation.is_empty()
            && translation != chat_row.message
        {
            job.append(
                &format!(" ({translation})"),
                0.0,
                TextFormat {
                    color: Color32::from_rgb(255, 255, 255),
                    background: Color32::from_rgb(20, 20, 20),
                    ..Default::default()
                },
            );
        }

        // Add the formatted text to the UI and make it clickable
        if ui
            .label(job)
            .on_hover_text(format!("Original message: {}", chat_row.message))
            .clicked()
        {
            app_win.set_selected_player(chat_row.steamid);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::{tests::run_ui, theme};
    use eframe::egui::{self, Shape};

    #[test]
    fn chat_displays_translations_only_when_enabled() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut app = crate::models::player_profile::test_app();
        let chat = LobbyChat {
            player_name: "Player".into(),
            message: "Hej".into(),
            translated_message: Some("Hello".into()),
            ..Default::default()
        };

        for (enabled, expected) in [(true, "Player: Hej (Hello)"), (false, "Player: Hej")] {
            app.app_settings.translate_chat = enabled;
            let output = run_ui(&ctx, egui::RawInput::default(), |ui| {
                add_chat_row(ui, &mut app, &chat);
            });
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, Shape::Text(text) if text.galley.text() == expected)
            }));
        }
    }
}
