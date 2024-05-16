use super::player_tooltip::add_player_tooltip;
use crate::{
    models::steamid::SteamID,
    tf2::lobby::{Player, Team},
};
use eframe::egui::{Align, Color32, Grid, Layout, Sense, Ui, Vec2};

pub fn scoreboard_team(
    ui: &mut Ui,
    title: &str,
    self_steamid: SteamID,
    players: &Vec<&Player>,
    team_name: &str,
    show_crits: &mut bool,
) {
    ui.heading(format!("{} - {} players", title, players.len()));

    ui.horizontal(|ui| {
        let total_kills = players.iter().map(|p| p.kills).sum::<u32>();
        let total_crit_kills = players.iter().map(|p| p.crit_kills).sum::<u32>();
        let total_deaths = players.iter().map(|p| p.deaths).sum::<u32>();
        let total_crits_deaths = players.iter().map(|p| p.crit_deaths).sum::<u32>();

        ui.label(format!("Kills: {}", total_kills));
        if *show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crit_kills));
        }

        ui.label(format!("Deaths: {}", total_deaths));
        if *show_crits {
            ui.colored_label(Color32::GRAY, format!("({})", total_crits_deaths));
        }
    });

    Grid::new(team_name).striped(true).show(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
            ui.label("");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Player").strong());
            ui.label("Player");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Kills").strong());
            ui.label("Kills");
        });
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Deaths").strong());
            ui.label("Deaths");
        });
        // ui.label("Flags");
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            // ui.label(RichText::new("Links").strong());
            ui.label("Links");
        });

        ui.end_row();

        for player in players {
            // Team color box
            add_team_symbol(ui, self_steamid, player);

            add_player_name(ui, player);

            // Player kills
            ui.horizontal(|ui| {
                ui.label(format!("{:3}", player.kills));
                if *show_crits {
                    ui.colored_label(Color32::GRAY, format!("({})", player.crit_kills));
                }
            });

            // Player deaths
            ui.horizontal(|ui| {
                ui.label(format!("{:3}", player.deaths));
                if *show_crits {
                    ui.colored_label(Color32::GRAY, format!("({})", player.crit_deaths));
                }
            });

            // add_flags(ui, &player);
            add_links(ui, player);

            ui.end_row();
        }
    });
}

fn add_player_name(ui: &mut Ui, player: &Player) {
    // Player icon and name
    ui.horizontal(|ui| {
        if let Some(steam_info) = &player.steam_info {
            ui.image(&steam_info.avatar)
                .on_hover_ui(|ui| add_player_tooltip(ui, player));
        }

        ui.label(player.name.clone())
            .on_hover_ui(|ui| add_player_tooltip(ui, player));
    });
}

fn add_team_symbol(ui: &mut Ui, self_steamid: SteamID, player: &Player) {
    let invader_color = super::colors::TEAM_BLU_COLOR;
    let defender_color = super::colors::TEAM_RED_COLOR;

    let color = match player.team {
        Team::Invaders => invader_color,
        Team::Defendes => defender_color,
        _ => Color32::GRAY,
    };

    ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
        ui.horizontal(|ui| {
            let size = Vec2::splat(2.0 * 10.0 * 0.5 + 5.0);

            let (rect, _response) = ui.allocate_at_least(size, Sense::hover());
            ui.painter().rect_filled(rect, 3.0f32, color);

            if player.steamid == self_steamid {
                let (rect, _response) = ui.allocate_at_least(size, Sense::hover());
                ui.painter().rect_filled(rect, 3.0f32, Color32::WHITE);
            }

            if let Some(steam_info) = &player.steam_info {
                if steam_info.is_account_new() {
                    let (rect, _response) = ui.allocate_at_least(size, Sense::hover());
                    ui.painter().rect_filled(rect, 3.0f32, Color32::LIGHT_GREEN);
                }
            }
        });
    });
}

// #[allow(dead_code)]
// fn add_flags(ui: &mut Ui, player: &Player) {
//     ui.horizontal_wrapped(|ui| {
//         ui.set_max_width(140.0);

//         let flags = vec![
//             PlayerFlags::Cheater,
//             PlayerFlags::Bot,
//             PlayerFlags::Sus,
//             PlayerFlags::New,
//             PlayerFlags::Racist,
//             PlayerFlags::Exploiter,
//         ];

//         for flag in flags {
//             let is_active = player.flags.contains(&flag);

//             let (fgcolor, bgcolor) = if is_active {
//                 color_for_flag(flag)
//             } else {
//                 (Color32::WHITE, Color32::DARK_GRAY)
//             };

//             let text = flag_shortname(flag);
//             let tooltip = format!("{}. Click to toggle", flag_description(flag));

//             ui.scope(|ui| {
//                 ui.style_mut().visuals.override_text_color = Some(fgcolor);

//                 // ui.style_mut().
//                 ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
//                 ui.style_mut().visuals.widgets.active.weak_bg_fill = bgcolor;
//                 ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
//                 ui.style_mut().visuals.widgets.inactive.weak_bg_fill = bgcolor;
//                 ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
//                 ui.style_mut().visuals.widgets.hovered.weak_bg_fill = bgcolor;

//                 if ui.button(text).on_hover_text(tooltip).clicked() {
//                     if is_active {
//                         log::trace!("Removing flag: {:?}", flag);
//                         // player.flags.retain(|&f| f != flag);
//                     } else {
//                         log::trace!("Adding flag: {:?}", flag);
//                         // player.flags.push(flag);
//                     }
//                 }
//             });

//             // let frame = egui::containers::Frame::default()
//             //     .fill(bgcolor)
//             //     .stroke(egui::Stroke::new(1.0, Color32::BLACK))
//             //     .inner_margin(4.0)
//             //     .rounding(3.0);

//             // frame.show(ui, |ui| {
//             //     ui.scope(|ui| {
//             //         ui.style_mut().wrap = Some(false);
//             //         let rich_text = RichText::new(text).color(fgcolor).strong();

//             //         ui.label(rich_text).on_hover_text_at_pointer(tooltip)
//             //     });
//             // });

//             //     Frame::default()
//             //         .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
//             //         .rounding(ui.visuals().widgets.noninteractive.rounding)
//             //         .show(ui, |ui| {
//             //             self.frame.show(ui, |ui| {
//             //                 ui.style_mut().wrap = Some(false);
//             //                 ui.label(egui::RichText::new("Content").color(egui::Color32::WHITE));
//             //             });
//             //         });
//         }
//     });
// }

fn add_links(ui: &mut Ui, player: &Player) {
    fn make_link(ui: &mut Ui, url: String, text: &str) {
        if ui.hyperlink_to(text, url).clicked() {
            ui.close_menu();
        }
    }

    ui.horizontal(|ui| {
        ui.menu_button("☰ View", |ui| {
            // ui.heading("View player on");
            make_link(ui, player.steamid.steam_community_url(), "SteamCommunity");
            make_link(ui, player.steamid.steam_history_url(), "SteamHistory");
            make_link(ui, player.steamid.steam_rep_url(), "SteamRep");
            make_link(ui, player.steamid.steam_id_url(), "SteamID");
        });
    });
}
