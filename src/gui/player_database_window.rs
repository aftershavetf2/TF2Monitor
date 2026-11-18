use super::theme;
use crate::{
    db::{
        entities::{Account, PlayerFlag},
        queries::{get_all_accounts, get_all_player_flags},
    },
    models::AppWin,
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

    ui.heading("Filters");

    ui.add_space(8.0);

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

    ui.add_space(15.0);

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

        if ui.button("Clear filters").clicked() {
            app_win.player_database_filters.clear();
            app_win.player_database_search.clear();
            app_win.player_database_page = 0;
        }
    });

    let filtered_accounts: Vec<&Account> = accounts
        .iter()
        .filter(|account| account_matches_filters(account, &flags_by_steamid, app_win))
        .collect();

    ui.add(egui::Separator::default().spacing(30.0));

    let page_count = filtered_accounts.len().div_ceil(PAGE_SIZE).max(1);
    app_win.player_database_page = app_win.player_database_page.min(page_count - 1);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                app_win.player_database_page > 0,
                egui::Button::new("Previous"),
            )
            .clicked()
        {
            app_win.player_database_page -= 1;
        }
        if ui
            .add_enabled(
                app_win.player_database_page + 1 < page_count,
                egui::Button::new("Next"),
            )
            .clicked()
        {
            app_win.player_database_page += 1;
        }
        ui.label(format!(
            "Page {} of {}",
            app_win.player_database_page + 1,
            page_count
        ));
    });

    let start = app_win.player_database_page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(filtered_accounts.len());
    ui.label(format!(
        "Showing {} - {} of {} matching players ({} total)",
        if filtered_accounts.is_empty() {
            0
        } else {
            start + 1
        },
        end,
        filtered_accounts.len(),
        accounts.len()
    ));

    ScrollArea::both()
        .id_salt(("player_database_scroll", app_win.player_database_page))
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

                        let response = ui.label(&account.name);
                        if response.double_clicked() {
                            app_win.set_selected_player(steamid);
                            app_win.open_player_details_window(steamid);
                        }

                        let response = ui.label(account.steam_id.to_string());
                        if response.double_clicked() {
                            app_win.set_selected_player(steamid);
                            app_win.open_player_details_window(steamid);
                        }

                        ui.horizontal_wrapped(|ui| {
                            ui.label(format_flags(flags_by_steamid.get(&account.steam_id)));
                            super::markings::add_tf2bd_badge(ui, steamid, &app_win.tf2bd_lists);
                        });
                        ui.end_row();
                    }
                });
        });
}

fn group_flags_by_steamid(player_flags: &[PlayerFlag]) -> HashMap<i64, HashSet<String>> {
    let mut grouped = HashMap::new();

    for player_flag in player_flags {
        grouped
            .entry(player_flag.steam_id)
            .or_insert_with(HashSet::new)
            .insert(player_flag.flag_type.clone());
    }

    grouped
}

fn account_matches_filters(
    account: &Account,
    flags_by_steamid: &HashMap<i64, HashSet<String>>,
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
        .any(|filter| player_flags.contains(&format!("{:?}", filter)))
}

fn format_flags(flags: Option<&HashSet<String>>) -> String {
    let Some(flags) = flags else {
        return "-".to_string();
    };

    if flags.is_empty() {
        return "-".to_string();
    }

    let mut flags: Vec<&str> = flags.iter().map(String::as_str).collect();
    flags.sort_unstable();
    flags.join(", ")
}
