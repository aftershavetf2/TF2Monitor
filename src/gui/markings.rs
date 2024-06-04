use super::colors::color_for_flag;
use crate::tf2::lobby::{flag_description, flag_shortname, Player, PlayerFlag, PlayerMarking};
use eframe::egui::Ui;
use std::collections::{HashMap, HashSet};

pub struct Marking {
    pub sources: HashSet<String>,
    pub suggestion: HashSet<String>,
    pub flag: PlayerFlag,
}

pub fn transform_data(markings: &HashMap<String, PlayerMarking>) -> HashMap<PlayerFlag, Marking> {
    let mut result: HashMap<PlayerFlag, Marking> = HashMap::new();

    for (source, marking) in markings {
        for flag in &marking.flags {
            let entry = result.entry(*flag).or_insert(Marking {
                sources: HashSet::new(),
                suggestion: HashSet::new(),
                flag: *flag,
            });

            if marking.suggestion {
                entry.suggestion.insert(source.clone());
            } else {
                entry.sources.insert(source.clone());
            }
        }
    }

    result
}

pub fn add_flags(ui: &mut Ui, player: &Player) {
    let data = transform_data(&player.flags);
    ui.horizontal_wrapped(|ui| {
        ui.set_max_width(140.0);

        let flags = vec![
            PlayerFlag::Awesome,
            PlayerFlag::Cheater,
            PlayerFlag::Bot,
            PlayerFlag::Suspicious,
            PlayerFlag::Toxic,
            PlayerFlag::Exploiter,
        ];

        for flag in flags {
            if let Some(marking) = data.get(&flag) {
                add_flag(ui, marking);
            }
        }
    });
}

fn add_flag(ui: &mut Ui, marking: &Marking) {
    let flag = marking.flag;

    ui.scope(|ui| {
        let (fgcolor, bgcolor) = color_for_flag(flag);

        ui.style_mut().visuals.override_text_color = Some(fgcolor);

        // ui.style_mut().
        ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = bgcolor;

        let mut suggested_mark = "";

        if marking.sources.is_empty() && !marking.suggestion.is_empty() {
            suggested_mark = "?";
        }

        let text = format!("{}{}", flag_description(flag), suggested_mark);

        ui.button(text)
            .on_hover_ui(|ui| add_flag_tooltip(ui, marking));
    });
}

pub fn add_flag_tooltip(ui: &mut Ui, marking: &Marking) {
    let desc = flag_description(marking.flag);

    ui.heading(format!("{}", desc));

    if !marking.sources.is_empty() {
        ui.label(format!("{} claims the following sources:", desc));
        for source in &marking.sources {
            ui.label(format!("- {}", source));
        }
    }

    if !marking.suggestion.is_empty() {
        ui.label(format!("{} suggested by the following sources:", desc));
        for source in &marking.suggestion {
            ui.label(format!("- {}", source));
        }
    }
}
