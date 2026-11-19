use super::theme;
use crate::{
    models::AppWin, tf2::lobby::player_attribute_description, tf2bd::player_lists::ATTRIBUTES,
};
use eframe::egui::{self, ScrollArea, Ui};

// Public TF2BD exports listed by SteamHistory, plus the check.tf export.
const KNOWN_LISTS: &[(&str, &str)] = &[
    (
        "TF2BD Official List",
        "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/staging/cfg/playerlist.official.json",
    ),
    ("TF2BD Trusted List", "https://trusted.roto.lol/v1/steamids"),
    (
        "Vorobey-HackerPolice",
        "https://raw.githubusercontent.com/AveraFox/Tom/refs/heads/main/playerlist.vorobey-hackerpolice.json",
    ),
    ("check.tf", "https://api.check.tf/v1/playerlist"),
    (
        "H0xton1337 NaughtyList",
        "https://raw.githubusercontent.com/H0xton1337/naughtylist-/main/playerlist.naughtylist.json",
    ),
    (
        "Milenko List",
        "https://raw.githubusercontent.com/incontestableness/milenko-lists/master/playerlist.milenko-cumulative.json",
    ),
    (
        "minein4 ShitList",
        "https://raw.githubusercontent.com/minein4/Shitlist/main/playerlist.shitlist.json",
    ),
    (
        "sleepylist",
        "https://raw.githubusercontent.com/surepy/tf2db-sleepy-list/main/playerlist.sleepy.json",
    ),
    (
        "sleepylist — Bots",
        "https://raw.githubusercontent.com/surepy/tf2db-sleepy-list/main/playerlist.sleepy-bots.merged.json",
    ),
    (
        "sleepylist — NoProof",
        "https://raw.githubusercontent.com/surepy/tf2db-sleepy-list/main/playerlist.sleepy-no-proof.json",
    ),
    (
        "wget List",
        "https://gist.githubusercontent.com/wgetJane/0bc01bd46d7695362253c5a2fa49f2e9/raw/playerlist.biglist.json",
    ),
];

#[derive(Default)]
pub struct ListsWindow {
    pub open: bool,
    url: String,
    known_list: Option<usize>,
    error: Option<String>,
}

pub fn show(app_win: &mut AppWin, ctx: &egui::Context) {
    let window = &mut app_win.tf2bd_lists_window;
    if !window.open {
        return;
    }

    theme::window(ctx, "TF2BD Player Lists")
        .open(&mut window.open)
        .default_width(620.0)
        .show(ctx, |ui| {
            let lists = &mut app_win.tf2bd_lists;
            if let Some(error) = &lists.load_error {
                show_error(ui, &format!("Could not load list settings: {error}"));
                return;
            }

            theme::section_heading(ui, "Add Lists");
            let source_width = 260.0;
            ui.add_enabled_ui(!lists.busy(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let selected_name = window
                        .known_list
                        .map_or("Choose a known list", |index| KNOWN_LISTS[index].0);
                    egui::ComboBox::from_id_salt("known_tf2bd_lists")
                        .selected_text(selected_name)
                        .width(source_width)
                        .show_ui(ui, |ui| {
                            for (index, &(name, url)) in KNOWN_LISTS.iter().enumerate() {
                                if lists.has_source_url(url) {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::selectable(false, format!("{name} (added)")),
                                    );
                                } else {
                                    ui.selectable_value(&mut window.known_list, Some(index), name)
                                        .on_hover_text(url);
                                }
                            }
                        });
                    let selected_url = window
                        .known_list
                        .map(|index| KNOWN_LISTS[index].1)
                        .filter(|url| !lists.has_source_url(url));
                    if ui
                        .add_enabled(selected_url.is_some(), egui::Button::new("Add"))
                        .clicked()
                        && let Some(url) = selected_url
                    {
                        window.error = lists.add_from_url(url).err();
                    }

                    let urls = KNOWN_LISTS.iter().map(|&(_, url)| url);
                    let has_missing = urls.clone().any(|url| !lists.has_source_url(url));
                    if ui
                        .add_enabled(
                            has_missing && !lists.busy(),
                            theme::secondary_button("Add All"),
                        )
                        .clicked()
                    {
                        window.error = lists.add_from_urls(urls).err();
                    }
                });
            });

            ui.add_enabled_ui(!lists.busy(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut window.url)
                            .hint_text("Player list URL")
                            .desired_width(source_width),
                    );
                    let submit =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui
                        .add_enabled(
                            !window.url.trim().is_empty(),
                            egui::Button::new("Add from URL"),
                        )
                        .clicked()
                        || (submit && !window.url.trim().is_empty())
                    {
                        match lists.add_from_url(&window.url) {
                            Ok(()) => {
                                window.error = None;
                                window.url.clear();
                            }
                            Err(error) => window.error = Some(error),
                        }
                    }
                });
            });
            let import = &lists.import;
            if import.total > 0 {
                let completed = import.added + import.errors.len();
                if completed < import.total && lists.busy() {
                    ui.label(format!("Importing lists… {completed}/{}", import.total));
                } else {
                    ui.label(format!("Added {} of {} lists.", import.added, import.total));
                }
                if !import.errors.is_empty() {
                    egui::CollapsingHeader::new(format!(
                        "Failed imports ({})",
                        import.errors.len()
                    ))
                    .default_open(import.total == 1)
                    .show(ui, |ui| {
                        for error in &import.errors {
                            show_error(ui, error);
                        }
                    });
                }
            }
            if let Some(error) = &window.error {
                show_error(ui, error);
            }

            let section_spacing = ui.spacing().item_spacing.y;
            ui.add_space(section_spacing);
            theme::section_heading(ui, "TF2BD Badges");
            let mut filters = lists.filters().to_vec();
            let mut filters_changed = false;
            ui.horizontal_wrapped(|ui| {
                for attribute in ATTRIBUTES {
                    let mut selected = filters.contains(&attribute);
                    if theme::flag_checkbox(ui, &mut selected, attribute).changed() {
                        filters_changed = true;
                        if selected {
                            filters.push(attribute);
                        } else {
                            filters.retain(|&value| value != attribute);
                        }
                    }
                }
            });
            if filters_changed {
                filters.sort_by_key(|attribute| {
                    ATTRIBUTES.iter().position(|value| value == attribute)
                });
                window.error = lists.set_filters(filters).err();
            }

            ui.add_space(section_spacing);
            let has_lists = lists.summaries().next().is_some();
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if ui
                        .add_enabled(!lists.busy() && has_lists, egui::Button::new("Update All"))
                        .clicked()
                    {
                        window.error = lists.update_all().err();
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        theme::section_heading(ui, "Installed Lists");
                    });
                },
            );
            if !has_lists {
                ui.weak("No player lists added.");
            }
            let busy = lists.busy();
            let mut change_enabled = None;
            let mut remove = None;
            ui.scope(|ui| {
                ui.spacing_mut().scroll.floating = false;
                ScrollArea::vertical().show(ui, |ui| {
                    let card = egui::Frame::new()
                        .fill(theme::CARD_FILL)
                        .corner_radius(6)
                        .inner_margin(10);
                    for (index, list) in lists.summaries().enumerate() {
                        ui.push_id(list.filename, |ui| {
                            card.show(ui, |ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_enabled(!busy, theme::secondary_button("Remove"))
                                            .clicked()
                                        {
                                            remove = Some(index);
                                        }
                                        let mut enabled = list.enabled;
                                        if ui.checkbox(&mut enabled, "Enabled").changed() {
                                            change_enabled = Some((index, enabled));
                                        }
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| show_list_title(ui, list.title, list.url),
                                        );
                                    },
                                );
                                if let Some((players, counts)) = list.counts {
                                    ui.label(format!("{players} unique SteamIDs"));
                                    ui.horizontal_wrapped(|ui| {
                                        for (attribute, &count) in ATTRIBUTES.iter().zip(counts) {
                                            ui.label(format!(
                                                "{} {count}",
                                                player_attribute_description(*attribute).1
                                            ));
                                        }
                                    });
                                }
                                if list.failed {
                                    show_error(ui, list.status);
                                } else if !list.status.is_empty() {
                                    ui.label(list.status);
                                }
                            });
                        });
                    }
                });
            });
            if let Some((index, enabled)) = change_enabled {
                window.error = lists.set_enabled(index, enabled).err();
            }
            if let Some(index) = remove {
                window.error = lists.remove(index).err();
            }
        });
}

fn show_list_title(ui: &mut Ui, title: &str, url: &str) {
    let icon_size = ui.spacing().interact_size.y;
    let mut title_rect = ui.available_rect_before_wrap();
    title_rect.max.x =
        (title_rect.max.x - icon_size - ui.spacing().item_spacing.x).max(title_rect.min.x);
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        theme::card_title(ui, title);
    });

    let label = "Open list source";
    let response = ui.add(theme::secondary_button(()).min_size(egui::Vec2::splat(icon_size)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, ui.is_enabled(), label));
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_ui(|ui| {
            ui.label(label);
            ui.label(url);
        });

    if ui.is_rect_visible(response.rect) {
        let center = response.rect.center();
        let point = |x, y| center + egui::vec2(x, y);
        let stroke = egui::Stroke::new(1.5, ui.style().interact(&response).fg_stroke.color);
        let painter = ui.painter();
        painter.line(
            vec![
                point(0.0, -4.5),
                point(-5.5, -4.5),
                point(-5.5, 5.5),
                point(4.5, 5.5),
                point(4.5, 0.0),
            ],
            stroke,
        );
        painter.line_segment([point(-1.0, 1.0), point(5.5, -5.5)], stroke);
        painter.line(
            vec![point(1.0, -5.5), point(5.5, -5.5), point(5.5, -1.0)],
            stroke,
        );
    }

    let new_tab = response.clicked_with_open_in_background();
    if new_tab || response.clicked() {
        ui.open_url(egui::OpenUrl {
            url: url.to_owned(),
            new_tab,
        });
    }
}

fn show_error(ui: &mut Ui, error: &str) {
    ui.colored_label(ui.visuals().error_fg_color, error);
}
