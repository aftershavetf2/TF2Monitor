use super::{
    account_age::add_account_age, markings::add_reputation,
    player_class_symbol::add_player_class_symbol, player_tooltip::add_player_tooltip,
    playtime::add_playtime, ui_utils::show_empty_value,
};
use crate::{
    models::{AppWin, steamid::SteamID},
    tf2::lobby::Player,
};
use eframe::egui::{
    self, Align, Color32, CursorIcon, Grid, Layout, Sense, TextFormat, Ui, text::LayoutJob,
};

const COLUMN_SPACING: f32 = 8.0;
const KILLS_COLUMN_MIN_WIDTH: f32 = 50.0;
const DEATHS_COLUMN_MIN_WIDTH: f32 = 50.0;
const HOURS_COLUMN_MIN_WIDTH: f32 = 60.0;

/// Draws a team scoreboard and returns the grid bounds.
pub fn scoreboard_team(
    app_win: &mut AppWin,
    ui: &mut Ui,
    title: &str,
    players: &Vec<&Player>,
) -> egui::Response {
    ui.heading(format!("{} - {} players", title, players.len()));

    ui.horizontal(|ui| {
        let total_kills = players.iter().map(|p| p.kills).sum::<u32>();
        let total_crit_kills = players.iter().map(|p| p.crit_kills).sum::<u32>();
        let total_deaths = players.iter().map(|p| p.deaths).sum::<u32>();
        let total_crits_deaths = players.iter().map(|p| p.crit_deaths).sum::<u32>();

        ui.label(format!("Kills: {}", total_kills));
        if app_win.app_settings.show_crits {
            ui.colored_label(
                super::theme::muted_text(),
                format!("({})", total_crit_kills),
            );
        }

        ui.label(format!("Deaths: {}", total_deaths));
        if app_win.app_settings.show_crits {
            ui.colored_label(
                super::theme::muted_text(),
                format!("({})", total_crits_deaths),
            );
        }
    });

    let mut num_columns = 7; // Base columns: team, player, kills, deaths, age, hours, info
    if app_win.app_settings.show_ping {
        num_columns += 1;
    }
    if app_win.app_settings.show_health {
        num_columns += 1;
    }
    let margin = 6.0 / ui.ctx().pixels_per_point();
    let outer_bounds = ui.available_rect_before_wrap();
    let bounds = outer_bounds.shrink2(egui::vec2(margin, 0.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(bounds), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(outer_bounds));
        let team_right = ui.max_rect().right();
        Grid::new(title)
            .striped(false)
            .spacing(egui::vec2(COLUMN_SPACING, ui.spacing().item_spacing.y))
            .min_col_width(0.0)
            .num_columns(num_columns)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_SPACING;
                // Paint full-width backgrounds behind the inset cell content.
                let mut row_index = 0;
                let header_top = ui.cursor().top();
                let header_background = ui.painter().add(egui::Shape::Noop);
                let mut headings = Vec::new();
                // Header row
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    header_label(ui, &mut headings, "");
                });
                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                    header_label(ui, &mut headings, "Player");
                });
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    ui.set_min_width(KILLS_COLUMN_MIN_WIDTH);
                    header_label(ui, &mut headings, "Kills");
                });
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    ui.set_min_width(DEATHS_COLUMN_MIN_WIDTH);
                    header_label(ui, &mut headings, "Deaths");
                });
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    // Age must measure its full text width, including on Grid's
                    // first measuring frame when the offered width can be zero.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    header_label(ui, &mut headings, "Age").on_hover_text("Account age");
                });
                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                    ui.set_min_width(HOURS_COLUMN_MIN_WIDTH);
                    header_label(ui, &mut headings, "Hours").on_hover_text("TF2 hours played");
                });
                if app_win.app_settings.show_ping {
                    ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                        header_label(ui, &mut headings, "Ping")
                            .on_hover_text("Ping in milliseconds");
                    });
                }
                if app_win.app_settings.show_health {
                    ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                        header_label(ui, &mut headings, "Health").on_hover_text("Current health");
                    });
                }
                // Keep weapons and reputation within this team's remaining space.
                let info_width = (team_right - ui.cursor().left()).max(0.0);
                wrapped_cell(ui, info_width, |ui| {
                    header_label(ui, &mut headings, "Info")
                        .on_hover_text("Last weapon used, reputation (SourceBans) and flags");
                });

                end_padded_row(
                    ui,
                    header_background,
                    header_top,
                    row_index,
                    outer_bounds,
                    false,
                );
                let header_center_y =
                    (header_top + ui.cursor().top() - ui.spacing().item_spacing.y) / 2.0;
                for heading in headings {
                    ui.painter().set(
                        heading.shape,
                        egui::Shape::galley(
                            egui::pos2(
                                heading.rect.left(),
                                header_center_y - heading.text.size().y / 2.0,
                            ),
                            heading.text,
                            ui.visuals().text_color(),
                        ),
                    );
                }
                row_index += 1;

                // Player rows
                for player in players {
                    let row_top = ui.cursor().top();
                    let background = ui.painter().add(egui::Shape::Noop);
                    // Estimated class symbol
                    add_team_symbol(app_win, ui, app_win.self_steamid, player);

                    add_player_name(app_win, ui, player);

                    // Player kills
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_min_width(KILLS_COLUMN_MIN_WIDTH);
                        let mut job = LayoutJob {
                            break_on_newline: false,
                            ..Default::default()
                        };

                        job.append(
                            format!("{:3}", player.kills).as_str(),
                            0.0,
                            TextFormat {
                                color: ui.style().visuals.text_color(),
                                ..Default::default()
                            },
                        );

                        if app_win.app_settings.show_crits {
                            job.append(
                                format!("({})", player.crit_kills).as_str(),
                                6.0,
                                TextFormat {
                                    color: super::theme::muted_text(),
                                    ..Default::default()
                                },
                            );
                        }
                        ui.label(job).on_hover_text("Number of kills (crit kills)");
                    });

                    // Player deaths
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_min_width(DEATHS_COLUMN_MIN_WIDTH);
                        let mut job = LayoutJob::default();

                        job.append(
                            format!("{}", player.deaths).as_str(),
                            0.0,
                            TextFormat {
                                color: ui.style().visuals.text_color(),
                                ..Default::default()
                            },
                        );

                        if app_win.app_settings.show_crits {
                            job.append(
                                format!(" ({})", player.crit_deaths).as_str(),
                                0.0,
                                TextFormat {
                                    color: super::theme::muted_text(),
                                    ..Default::default()
                                },
                            );
                        }
                        ui.label(job)
                            .on_hover_text("Number of deaths (crit deaths)");
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        add_account_age(player, ui);
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_min_width(HOURS_COLUMN_MIN_WIDTH);
                        add_playtime(ui, player);
                    });

                    if app_win.app_settings.show_ping {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(format!("{}", player.ping_ms))
                                .on_hover_text_at_pointer("ms");
                        });
                    }

                    if app_win.app_settings.show_health {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if player.alive && player.health.is_some() {
                                let health = player.health.unwrap();
                                ui.label(format!("{}", health))
                                    .on_hover_text_at_pointer("Current health");
                            } else {
                                show_empty_value(ui);
                            }
                        });
                    }

                    wrapped_cell(ui, info_width, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if app_win.app_settings.show_weapons {
                                if let Some(last_weapon) = player.kills_with.last() {
                                    ui.add(
                                        egui::Label::new(last_weapon.weapon.replace('_', " "))
                                            .wrap(),
                                    )
                                    .on_hover_text_at_pointer("Last weapon used");
                                } else {
                                    show_empty_value(ui);
                                }
                            }
                            add_reputation(ui, player);
                        });
                    });

                    end_padded_row(
                        ui,
                        background,
                        row_top,
                        row_index,
                        outer_bounds,
                        player.steamid == app_win.self_steamid,
                    );
                    row_index += 1;
                }

                // Add empty rows to fill the grid
                if players.len() < 12 {
                    for _ in 0..(12 - players.len()) {
                        let row_top = ui.cursor().top();
                        let background = ui.painter().add(egui::Shape::Noop);
                        for _ in 0..num_columns {
                            ui.label("");
                        }
                        end_padded_row(ui, background, row_top, row_index, outer_bounds, false);
                        row_index += 1;
                    }
                }
            })
            .response
    })
    .inner
}

struct HeaderLabel {
    shape: egui::layers::ShapeIdx,
    rect: egui::Rect,
    text: std::sync::Arc<egui::Galley>,
}

/// Measure each heading now, then paint it once the entire row's height is known.
fn header_label(ui: &mut Ui, headings: &mut Vec<HeaderLabel>, label: &str) -> egui::Response {
    let wrap_width = if ui.style().wrap_mode == Some(egui::TextWrapMode::Wrap) {
        ui.available_width()
    } else {
        f32::INFINITY
    };
    let text = ui.painter().layout(
        label.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
        wrap_width,
    );
    let size = egui::vec2(
        text.size().x,
        text.size().y.max(ui.spacing().interact_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    headings.push(HeaderLabel {
        shape: ui.painter().add(egui::Shape::Noop),
        rect,
        text,
    });
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), label));
    response
}

/// Row backgrounds span the entire team width; content stays padded on both sides.
fn end_padded_row(
    ui: &mut Ui,
    background: egui::layers::ShapeIdx,
    top: f32,
    row: usize,
    outer: egui::Rect,
    is_self: bool,
) {
    ui.end_row();
    let half_spacing = ui.spacing().item_spacing.y / 2.0;
    let rect = egui::Rect::from_min_max(
        egui::pos2(outer.left(), top - half_spacing),
        egui::pos2(outer.right(), ui.cursor().top() - half_spacing),
    );
    let color = if is_self {
        super::theme::TABLE_ROW_SELF
    } else if row % 2 == 0 {
        super::theme::TABLE_ROW_LIGHT
    } else {
        super::theme::TABLE_ROW_DARK
    };
    ui.painter()
        .set(background, egui::Shape::rect_filled(rect, 2.0, color));
}

fn wrapped_cell(ui: &mut Ui, width: f32, contents: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
        ui.set_width(width);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        contents(ui);
    });
}

fn add_player_name(app_win: &mut AppWin, ui: &mut Ui, player: &Player) {
    // Player icon and name
    ui.horizontal(|ui| {
        ui.push_id(player.steamid.to_u64(), |ui| {
            ui.scope(|ui| {
                // let color = match player.team {
                //     Team::Invaders => TEAM_BLU_COLOR,
                //     Team::Defendes => TEAM_RED_COLOR,
                //     _ => super::theme::muted_text(),
                // };
                // let color = Color32::WHITE;

                // ui.visuals_mut().override_text_color = Some(color);
                // ui.style_mut().visuals.override_text_color = Some(color);

                // Mark selected player and friends of selected player
                if let Some(steamid) = app_win.selected_player {
                    if steamid == player.steamid
                        || app_win
                            .lobby
                            .friendships
                            .are_friends(player.steamid, steamid)
                    {
                        let marked_color = Some(Color32::YELLOW);
                        ui.visuals_mut().override_text_color = marked_color;
                        ui.style_mut().visuals.override_text_color = marked_color;
                    }
                }

                // Player avatar and hover tooltip
                if let Some(steam_info) = &player.steam_info {
                    let response =
                        super::theme::image(ui, eframe::egui::Image::new(&steam_info.avatar_thumb))
                            .on_hover_cursor(CursorIcon::Help)
                            .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));

                    if response.clicked() {
                        app_win.set_selected_player(player.steamid);
                    }

                    if response.double_clicked() {
                        app_win.open_player_details_window(player.steamid);
                    }
                }

                // Player name prefixed with DEAD is not alive
                let name_color = if player.alive {
                    Color32::WHITE
                } else {
                    super::theme::muted_text()
                };

                // Player name and hover tooltip
                let response = ui
                    .colored_label(name_color, player.name.clone())
                    .on_hover_cursor(CursorIcon::Default)
                    .on_hover_ui_at_pointer(|ui| add_player_tooltip(ui, player));

                if response.clicked() {
                    app_win.set_selected_player(player.steamid);
                }

                if response.double_clicked() {
                    app_win.open_player_details_window(player.steamid);
                }
            });
        });
    });
}

fn add_team_symbol(app_win: &mut AppWin, ui: &mut Ui, self_steamid: SteamID, player: &Player) {
    ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
        ui.horizontal(|ui| {
            let response = add_player_class_symbol(ui, player);
            let pos = response.rect.center();
            app_win.friendship_positions.insert(player.steamid, pos);

            if app_win
                .lobby
                .friendships
                .are_friends(self_steamid, player.steamid)
            {
                ui.colored_label(Color32::RED, "❤")
                    .on_hover_text(format!("{} is in your friendlist", player.name));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tf2::lobby::Team;
    use crate::{appbus::AppBus, models::app_settings::AppSettings};
    use diesel::{
        SqliteConnection,
        r2d2::{ConnectionManager, Pool},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn team_grids_fill_their_columns_with_each_optional_column_combination() {
        let settings = AppSettings::default();
        let bus = Arc::new(Mutex::new(AppBus::new(settings.self_steamid64)));
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
            .unwrap();
        let mut app = AppWin::new(&settings, &bus, Arc::new(pool));
        let team: Vec<_> = (0..12)
            .map(|index| Player {
                name: format!("Player {index}"),
                steamid: SteamID::from_u64(index + 1),
                team: Team::Blue,
                account_age: match index % 4 {
                    0 => crate::tf2::lobby::AccountAge::Unknown,
                    1 => crate::tf2::lobby::AccountAge::Private,
                    2 => crate::tf2::lobby::AccountAge::Loaded(
                        chrono::Local::now()
                            - chrono::Duration::days(if index < 4 { 30 } else { 3650 }),
                    ),
                    _ => crate::tf2::lobby::AccountAge::Approx(
                        chrono::Local::now()
                            - chrono::Duration::days(if index < 4 { 30 } else { 3650 }),
                    ),
                },
                tf2_play_minutes: match index % 3 {
                    0 => crate::tf2::lobby::Tf2PlayMinutes::Unknown,
                    1 => crate::tf2::lobby::Tf2PlayMinutes::PlayMinutes(60 * 100),
                    _ => crate::tf2::lobby::Tf2PlayMinutes::PlayMinutes(60 * 500),
                },
                player_info: Some(crate::tf2bd::models::PlayerInfo {
                    attributes: vec![
                        crate::tf2bd::models::PlayerAttribute::Cool,
                        crate::tf2bd::models::PlayerAttribute::Cheater,
                        crate::tf2bd::models::PlayerAttribute::Bot,
                        crate::tf2bd::models::PlayerAttribute::Suspicious,
                        crate::tf2bd::models::PlayerAttribute::Toxic,
                        crate::tf2bd::models::PlayerAttribute::Exploiter,
                    ],
                    ..Default::default()
                }),
                kills_with: vec![crate::tf2::lobby::PlayerKill {
                    weapon: "a_very_long_weapon_name_that_must_wrap_inside_the_team_column".into(),
                    crit: false,
                }],
                ..Default::default()
            })
            .collect();
        for scale in [1.0, 1.25, 1.5] {
            let ctx = egui::Context::default();
            super::super::theme::install(&ctx);
            for flags in 0..8 {
                app.app_settings.show_ping = flags & 1 != 0;
                app.app_settings.show_health = flags & 2 != 0;
                app.app_settings.show_weapons = flags & 4 != 0;
                for count in [0, 1, 12] {
                    let players = team.iter().take(count).collect();
                    let mut bounds = [(egui::Rect::NOTHING, egui::Rect::NOTHING); 2];
                    // Grid measures its contents across frames; also exercise resizing in place.
                    for width in [1000.0, 1280.0, 1800.0, 2200.0, 1000.0] {
                        for frame in 0..4 {
                            let mut input = egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    egui::vec2(width, 900.0),
                                )),
                                ..Default::default()
                            };
                            input
                                .viewports
                                .get_mut(&egui::ViewportId::ROOT)
                                .unwrap()
                                .native_pixels_per_point = Some(scale);
                            let output = ctx.run(input, |ctx| {
                                egui::CentralPanel::default().show(ctx, |ui| {
                                    ui.columns(2, |columns| {
                                        for (index, ui) in columns.iter_mut().enumerate() {
                                            let expected = ui.max_rect().shrink2(egui::vec2(
                                                6.0 / ctx.pixels_per_point(),
                                                0.0,
                                            ));
                                            let response = scoreboard_team(
                                                &mut app,
                                                ui,
                                                if index == 0 { "Blu" } else { "Red" },
                                                &players,
                                            );
                                            bounds[index] = (response.rect, expected);
                                        }
                                    });
                                });
                            });
                            if frame == 3 {
                                let header_center = output
                                    .shapes
                                    .iter()
                                    .find_map(|shape| match &shape.shape {
                                        egui::Shape::Rect(rect)
                                            if rect.fill
                                                == super::super::theme::TABLE_ROW_LIGHT =>
                                        {
                                            Some(rect.rect.center().y)
                                        }
                                        _ => None,
                                    })
                                    .unwrap();
                                for shape in &output.shapes {
                                    if let egui::Shape::Text(text) = &shape.shape {
                                        if [
                                            "Player", "Kills", "Deaths", "Age", "Hours", "Ping",
                                            "Health", "Info",
                                        ]
                                        .contains(&text.galley.text())
                                        {
                                            assert!(
                                                (text.pos.y + text.galley.size().y / 2.0
                                                    - header_center)
                                                    .abs()
                                                    < 0.01,
                                                "Heading must be vertically centered: {}",
                                                text.galley.text()
                                            );
                                        }
                                    }
                                }
                                let mut stripe_count = 0;
                                for shape in &output.shapes {
                                    if let egui::Shape::Rect(stripe) = &shape.shape {
                                        if [
                                            super::super::theme::TABLE_ROW_LIGHT,
                                            super::super::theme::TABLE_ROW_DARK,
                                        ]
                                        .contains(&stripe.fill)
                                        {
                                            let (_, content) = bounds
                                                .iter()
                                                .min_by(|a, b| {
                                                    (a.1.center().x - stripe.rect.center().x)
                                                        .abs()
                                                        .total_cmp(
                                                            &(b.1.center().x
                                                                - stripe.rect.center().x)
                                                                .abs(),
                                                        )
                                                })
                                                .unwrap();
                                            let pixels = ctx.pixels_per_point();
                                            assert!(
                                                ((content.left() - stripe.rect.left()) * pixels
                                                    - 6.0)
                                                    .abs()
                                                    < 0.01
                                            );
                                            assert!(
                                                ((stripe.rect.right() - content.right()) * pixels
                                                    - 6.0)
                                                    .abs()
                                                    < 0.01
                                            );
                                            stripe_count += 1;
                                        }
                                    }
                                }
                                assert!(stripe_count >= 2);
                            }
                            for shape in &output.shapes {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    if ["Age", "Unknown", "Private"].contains(&text.galley.text()) {
                                        assert_eq!(
                                            text.galley.rows.len(),
                                            1,
                                            "Age must never wrap: {}",
                                            text.galley.text()
                                        );
                                    }
                                }
                            }
                            if frame == 3 && count > 0 && flags & 4 != 0 && width == 1000.0 {
                                let weapons: Vec<_> = output
                                    .shapes
                                    .iter()
                                    .filter_map(|shape| match &shape.shape {
                                        egui::Shape::Text(text)
                                            if text
                                                .galley
                                                .text()
                                                .starts_with("a very long weapon") =>
                                        {
                                            Some(text)
                                        }
                                        _ => None,
                                    })
                                    .collect();
                                assert!(!weapons.is_empty());
                                for weapon in weapons {
                                    assert!(weapon.galley.rows.len() > 1);
                                }
                                // Compact badges may fit on one line, but each stays intact.
                                let badges: Vec<_> = output
                                    .shapes
                                    .iter()
                                    .filter_map(|shape| match &shape.shape {
                                        egui::Shape::Text(text)
                                            if ["C", "B", "S", "T", "E"]
                                                .contains(&text.galley.text()) =>
                                        {
                                            Some(text)
                                        }
                                        _ => None,
                                    })
                                    .collect();
                                assert!(!badges.is_empty());
                                assert!(badges.iter().all(|badge| badge.galley.rows.len() == 1));
                            }
                        }
                        for (grid, expected) in bounds {
                            assert!(
                                (grid.right() - expected.right()).abs() <= 1.0
                                    && (grid.left() - expected.left()).abs() <= 1.0,
                                "flags={flags}, count={count}, width={width}, scale={scale}: grid={grid:?}, expected={expected:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
