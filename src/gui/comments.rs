use crate::tf2::{lobby::Player, steamapi::SteamProfileComment};
use eframe::egui::{text::LayoutJob, Color32, OpenUrl, ScrollArea, TextFormat, TextStyle, Ui};

use super::colors::hex_to_rgb;

pub fn add_profile_comments(player: &Player, ui: &mut Ui) {
    // ui.heading("More info:");
    ui.vertical(|ui| {
        ui.heading("Profile Comments");

        if let Some(comments) = &player.profile_comments {
            let text_style = TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            let num_rows = comments.len();

            ui.push_id("comments", |ui| {
                ScrollArea::vertical()
                    .stick_to_bottom(false)
                    .auto_shrink(false)
                    .show_rows(ui, row_height, num_rows, |ui, row_range| {
                        ui.scope(|ui| {
                            ui.style_mut().visuals.panel_fill = hex_to_rgb(0xffffff);

                            for row in row_range {
                                let row = &comments[row].clone();
                                add_comment_row(ui, row);

                                // if ui.link(row.name.clone()).clicked() {
                                //     ui.ctx().open_url(OpenUrl {
                                //         url: row.steamid.steam_history_url(),
                                //         new_tab: true,
                                //     });
                                // }

                                // ui.label(format!("{}", row.comment));

                                // ui.separator();
                            }
                        });
                    });
            });
        } else {
            ui.label("Loading comments...");
        }
    });
}

fn add_comment_row(ui: &mut Ui, row: &SteamProfileComment) {
    ui.horizontal_wrapped(|ui| {
        let mut job = LayoutJob::default();

        job.append(
            "💬 ",
            0.0,
            TextFormat {
                color: Color32::WHITE,
                ..Default::default()
            },
        );

        // Player name
        job.append(
            &row.name,
            0.0,
            TextFormat {
                // color,
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
            &row.comment,
            0.0,
            TextFormat {
                color: Color32::from_rgb(210, 210, 210),
                ..Default::default()
            },
        );

        // let is_translated =
        //     row.translated_message.is_some() && row.translated_message != Some(row.message.clone());

        // // The chat message
        // if is_translated {
        //     job.append(
        //         " ",
        //         0.0,
        //         TextFormat {
        //             color: Color32::from_rgb(210, 210, 210),
        //             ..Default::default()
        //         },
        //     );
        //     job.append(
        //         format!("({})", &row.translated_message.as_ref().unwrap()).as_str(),
        //         0.0,
        //         TextFormat {
        //             color: Color32::from_rgb(255, 255, 255),
        //             background: Color32::from_rgb(20, 20, 20),
        //             ..Default::default()
        //         },
        //     );
        // }

        // Add the formatted text to the UI and make it clickable
        if ui
            .label(job)
            .on_hover_text("Click to view player on Steam History")
            .clicked()
        {
            ui.ctx().open_url(OpenUrl {
                url: row.steamid.steam_history_url(),
                new_tab: true,
            });
        }
    });
}
