use super::theme;
use crate::{
    db::{connection::DbPool, entities::Account, queries::get_all_accounts},
    models::{AppWin, player_markings::MarkingsSnapshot, steamid::SteamID},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{self, Grid, ScrollArea, TextEdit, Ui};
use std::collections::HashSet;

const PAGE_SIZE: usize = 50;

const PLAYER_DATABASE_FILTERS: [PlayerAttribute; 6] = [
    PlayerAttribute::Cheater,
    PlayerAttribute::Bot,
    PlayerAttribute::Suspicious,
    PlayerAttribute::Toxic,
    PlayerAttribute::Exploiter,
    PlayerAttribute::Cool,
];

pub fn show_player_database_window(app_win: &mut AppWin, ctx: &egui::Context) {
    if !app_win.player_database_window_open {
        return;
    }

    let mut window_open = app_win.player_database_window_open;

    super::theme::window(ctx, "Player Database")
        .open(&mut window_open)
        .resizable(true)
        .default_width(980.0)
        .default_height(700.0)
        .show(ctx, |ui| {
            // Fill the height allocated by the resizable window, including empty results.
            ui.set_min_height(ui.available_height());
            show_player_database_content(ui, app_win);
        });

    if !window_open {
        app_win.player_database_window_open = false;
    }
}

fn show_player_database_content(ui: &mut Ui, app_win: &mut AppWin) {
    let accounts = match load_cached_accounts(&app_win.db) {
        Ok(accounts) => accounts,
        Err(error) => {
            ui.colored_label(ui.visuals().error_fg_color, error);
            return;
        }
    };

    let markings = app_win.markings.snapshot();
    let players = merge_marked_players(accounts, &markings);
    show_player_database_filters(ui, app_win);
    show_player_database_results(ui, app_win, &players);
}

fn load_cached_accounts(db: &DbPool) -> Result<Vec<Account>, String> {
    let mut conn = db
        .get()
        .map_err(|error| format!("Database error: {error}"))?;
    get_all_accounts(&mut conn).map_err(|error| format!("Failed to load accounts: {error}"))
}

struct DatabasePlayer<'a> {
    steamid: SteamID,
    name: String,
    flags: &'a [PlayerAttribute],
}

fn merge_marked_players(
    accounts: Vec<Account>,
    markings: &MarkingsSnapshot,
) -> Vec<DatabasePlayer<'_>> {
    let cached_ids: HashSet<_> = accounts
        .iter()
        .map(|account| account.steam_id as u64)
        .collect();
    let mut players: Vec<_> = accounts
        .into_iter()
        .map(|account| {
            let id = account.steam_id as u64;
            DatabasePlayer {
                steamid: SteamID::from_u64(id),
                name: account.name,
                flags: markings
                    .get(&id)
                    .map_or(&[][..], |marking| marking.flags.as_slice()),
            }
        })
        .collect();

    for (&id, marking) in markings {
        if cached_ids.contains(&id) {
            continue;
        }
        let name = marking
            .last_seen
            .as_ref()
            .and_then(|seen| seen.player_name.clone())
            .unwrap_or_else(|| id.to_string());
        players.push(DatabasePlayer {
            steamid: SteamID::from_u64(id),
            name,
            flags: &marking.flags,
        });
    }

    // Cached accounts already arrive in display order.
    if players.len() > cached_ids.len() {
        players.sort_unstable_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.steamid.to_u64().cmp(&b.steamid.to_u64()))
        });
    }
    players
}

fn show_player_database_filters(ui: &mut Ui, app_win: &mut AppWin) {
    theme::section_heading(ui, "Filters");

    if ui
        .add(
            TextEdit::singleline(&mut app_win.player_database_search)
                .hint_text("Filter by name or SteamID64")
                .desired_width(280.0),
        )
        .changed()
    {
        app_win.player_database_page = 0;
    }

    ui.horizontal_wrapped(|ui| {
        for player_attribute in PLAYER_DATABASE_FILTERS {
            let mut enabled = app_win.player_database_filters.contains(&player_attribute);
            if theme::flag_checkbox(ui, &mut enabled, player_attribute).changed() {
                app_win.player_database_page = 0;
                if enabled {
                    app_win.player_database_filters.insert(player_attribute);
                } else {
                    app_win.player_database_filters.remove(&player_attribute);
                }
            }
        }

        let has_filters = !app_win.player_database_filters.is_empty()
            || !app_win.player_database_search.is_empty();
        if ui
            .add_enabled(has_filters, theme::secondary_button("Clear Filters"))
            .clicked()
        {
            app_win.player_database_filters.clear();
            app_win.player_database_search.clear();
            app_win.player_database_page = 0;
        }
    });
}

fn show_player_database_results(ui: &mut Ui, app_win: &mut AppWin, players: &[DatabasePlayer<'_>]) {
    let total_count = players.len();
    let filtered_players: Vec<_> = players
        .iter()
        .filter(|player| player_matches_filters(player, app_win))
        .collect();

    ui.add_space(ui.spacing().item_spacing.y);
    theme::section_heading(ui, "Results");

    let matching_count = filtered_players.len();
    if matching_count == 0 {
        app_win.player_database_page = 0;
        ui.label(if total_count == 0 {
            "No players in the database."
        } else {
            "No players match your filters."
        });
        return;
    }

    let page_count = matching_count.div_ceil(PAGE_SIZE);
    let page = app_win.player_database_page.min(page_count - 1);
    let start = page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(matching_count);
    let first = start + 1;
    let count = if matching_count == total_count {
        format!("Showing {first}–{end} of {total_count} players")
    } else {
        format!("Showing {first}–{end} of {matching_count} matching players ({total_count} total)")
    };
    let (_, selected_page) = egui::containers::Sides::new().shrink_left().wrap().show(
        ui,
        |ui| ui.label(count),
        |ui| show_pagination(ui, page, page_count),
    );
    app_win.player_database_page = selected_page;
    show_player_grid(ui, app_win, page, &filtered_players[start..end]);
}

fn show_player_grid(
    ui: &mut Ui,
    app_win: &mut AppWin,
    page: usize,
    players: &[&DatabasePlayer<'_>],
) {
    ScrollArea::both()
        .id_salt(("player_database_scroll", page))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let column_width =
                ((ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0).max(180.0);
            Grid::new("player_database_grid")
                .min_col_width(column_width)
                .max_col_width(column_width)
                .with_row_color(|row, _style| {
                    Some(if row.is_multiple_of(2) {
                        super::theme::TABLE_ROW_LIGHT
                    } else {
                        super::theme::TABLE_ROW_DARK
                    })
                })
                .num_columns(3)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("SteamID64");
                    ui.strong("Flags");
                    ui.end_row();

                    for player in players {
                        show_player_row(ui, app_win, player);
                    }
                });
        });
}

fn show_player_row(ui: &mut Ui, app_win: &mut AppWin, player: &DatabasePlayer<'_>) {
    let name = ui.add(egui::Label::new(&player.name).sense(egui::Sense::click()));
    let steam_id =
        ui.add(egui::Label::new(player.steamid.to_u64().to_string()).sense(egui::Sense::click()));
    if name.double_clicked() || steam_id.double_clicked() {
        app_win.open_player_details_window(player.steamid);
    }

    ui.horizontal_wrapped(|ui| {
        for flag in PLAYER_DATABASE_FILTERS {
            if player.flags.contains(&flag) {
                super::markings::add_flag(ui, flag);
            }
        }
        if player.flags.is_empty() && !app_win.tf2bd_lists.has_match(player.steamid) {
            ui.weak("—");
        }
        super::markings::add_tf2bd_badge(ui, player.steamid, &app_win.tf2bd_lists);
    });
    ui.end_row();
}

fn show_pagination(ui: &mut Ui, page: usize, page_count: usize) -> usize {
    if page_count == 1 {
        return page;
    }

    let next = ui
        .add_enabled(page + 1 < page_count, theme::secondary_button("Next"))
        .clicked();
    ui.label(format!("Page {} of {page_count}", page + 1));
    let previous = ui
        .add_enabled(page > 0, theme::secondary_button("Previous"))
        .clicked();
    page + usize::from(next) - usize::from(previous)
}

fn player_matches_filters(player: &DatabasePlayer<'_>, app_win: &AppWin) -> bool {
    let search = app_win.player_database_search.trim();
    if !search.is_empty()
        && !player
            .name
            .to_ascii_lowercase()
            .contains(&search.to_ascii_lowercase())
        && !player.steamid.to_u64().to_string().contains(search)
    {
        return false;
    }
    app_win
        .player_database_filters
        .iter()
        .all(|flag| player.flags.contains(flag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player_profile::{seed_account, test_app};

    #[test]
    fn database_window_keeps_the_height_set_by_the_user() {
        let mut app = test_app();
        app.player_database_window_open = true;
        seed_account(&app, SteamID::from_u64(76561197960265729), "Player");

        let ctx = egui::Context::default();
        theme::install(&ctx);
        let render = |app: &mut AppWin, events| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                events,
                ..Default::default()
            };
            super::super::tests::run_ui(&ctx, input, |_| {
                show_player_database_window(app, &ctx);
            });
            let layer = ctx.top_layer_id().expect("Player Database window is open");
            ctx.memory(|memory| memory.area_rect(layer.id))
                .expect("Player Database window has bounds")
        };

        render(&mut app, vec![]);
        let initial = render(&mut app, vec![]);
        app.player_database_search = "no matching players".into();
        let empty = render(&mut app, vec![]);
        assert!((empty.height() - initial.height()).abs() < 1.0);

        let bottom = empty.center_bottom();
        let destination = bottom - egui::vec2(0.0, 160.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        render(&mut app, vec![egui::Event::PointerMoved(bottom)]);
        render(&mut app, vec![button(bottom, true)]);
        render(&mut app, vec![egui::Event::PointerMoved(destination)]);
        render(&mut app, vec![button(destination, false)]);
        let resized = render(&mut app, vec![]);
        assert!(resized.height() < empty.height() - 100.0);

        app.player_database_search.clear();
        let with_results = render(&mut app, vec![]);
        assert!((with_results.height() - resized.height()).abs() < 1.0);
    }

    #[test]
    fn flag_filter_requires_every_selected_flag() {
        use PlayerAttribute::{Cheater, Cool, Suspicious};

        let mut app = test_app();
        let mut player = DatabasePlayer {
            steamid: SteamID::from_u64(76561197960265729),
            name: "Player".into(),
            flags: &[],
        };
        assert!(player_matches_filters(&player, &app));

        app.player_database_filters.extend([Cheater, Suspicious]);
        assert!(!player_matches_filters(&player, &app));
        player.flags = &[Cheater];
        assert!(!player_matches_filters(&player, &app));
        player.flags = &[Suspicious];
        assert!(!player_matches_filters(&player, &app));
        player.flags = &[Cheater, Suspicious];
        assert!(player_matches_filters(&player, &app));
        player.flags = &[Cheater, Suspicious, Cool];
        assert!(player_matches_filters(&player, &app));
    }
}
