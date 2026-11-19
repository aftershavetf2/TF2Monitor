use super::theme;
use crate::{
    db::{
        entities::{Account, PlayerFlag},
        queries::{get_all_accounts, get_all_player_flags},
    },
    models::AppWin,
    tf2::lobby::player_attribute_description,
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{self, Grid, ScrollArea, TextEdit, Ui};
use std::collections::{HashMap, HashSet};

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
            show_player_database_content(ui, app_win);
        });

    if !window_open {
        app_win.player_database_window_open = false;
    }
}

fn show_player_database_content(ui: &mut Ui, app_win: &mut AppWin) {
    let mut conn = match app_win.db.get() {
        Ok(conn) => conn,
        Err(e) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Database error: {}", e),
            );
            return;
        }
    };

    let accounts = match get_all_accounts(&mut conn) {
        Ok(accounts) => accounts,
        Err(e) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Failed to load accounts: {}", e),
            );
            return;
        }
    };

    let player_flags = match get_all_player_flags(&mut conn) {
        Ok(player_flags) => player_flags,
        Err(e) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Failed to load player flags: {}", e),
            );
            return;
        }
    };

    let flags_by_steamid = group_flags_by_steamid(&player_flags);

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

    let filtered_accounts: Vec<&Account> = accounts
        .iter()
        .filter(|account| account_matches_filters(account, &flags_by_steamid, app_win))
        .collect();

    ui.add_space(ui.spacing().item_spacing.y);
    theme::section_heading(ui, "Results");

    let matching_count = filtered_accounts.len();
    if matching_count == 0 {
        app_win.player_database_page = 0;
        ui.label(if accounts.is_empty() {
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
    let total_count = accounts.len();
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

                    for account in &filtered_accounts[start..end] {
                        let steamid =
                            crate::models::steamid::SteamID::from_u64(account.steam_id as u64);

                        let name = ui.label(&account.name);
                        let steam_id = ui.label(account.steam_id.to_string());
                        if name.double_clicked() || steam_id.double_clicked() {
                            app_win.open_player_details_window(steamid);
                        }

                        ui.horizontal_wrapped(|ui| {
                            if let Some(flags) = flags_by_steamid.get(&account.steam_id) {
                                show_player_flags(ui, flags);
                            } else if !app_win.tf2bd_lists.has_match(steamid) {
                                ui.weak("—");
                            }
                            super::markings::add_tf2bd_badge(ui, steamid, &app_win.tf2bd_lists);
                        });
                        ui.end_row();
                    }
                });
        });
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

fn group_flags_by_steamid(player_flags: &[PlayerFlag]) -> HashMap<i64, HashSet<&str>> {
    let mut grouped = HashMap::new();

    for player_flag in player_flags {
        grouped
            .entry(player_flag.steam_id)
            .or_insert_with(HashSet::new)
            .insert(match player_flag.flag_type.as_str() {
                "Racist" => "Toxic",
                flag => flag,
            });
    }

    grouped
}

fn account_matches_filters(
    account: &Account,
    flags_by_steamid: &HashMap<i64, HashSet<&str>>,
    app_win: &AppWin,
) -> bool {
    let search_text = app_win.player_database_search.trim();
    if !search_text.is_empty() {
        let search_text_lower = search_text.to_ascii_lowercase();
        let name_matches = account
            .name
            .to_ascii_lowercase()
            .contains(&search_text_lower);
        let steamid_matches = account.steam_id.to_string().contains(search_text);

        if !name_matches && !steamid_matches {
            return false;
        }
    }

    if app_win.player_database_filters.is_empty() {
        return true;
    }

    let Some(player_flags) = flags_by_steamid.get(&account.steam_id) else {
        return false;
    };

    app_win
        .player_database_filters
        .iter()
        .any(|&filter| player_flags.contains(player_attribute_description(filter).1))
}

fn show_player_flags(ui: &mut Ui, flags: &HashSet<&str>) {
    let mut flags: Vec<_> = flags.iter().copied().collect();
    flags.sort_unstable();
    for flag in flags {
        let attribute = PLAYER_DATABASE_FILTERS
            .into_iter()
            .find(|&attribute| player_attribute_description(attribute).1 == flag);
        if let Some(attribute) = attribute {
            super::markings::add_flag(ui, attribute);
        } else {
            ui.label(flag);
        }
    }
}
