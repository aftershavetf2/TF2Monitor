use super::{
    colors::color_for_flag,
    markings::{add_flag_tooltip, transform_data, Marking},
};
use crate::{
    appbus::AppEventMsg,
    models::{steamid::SteamID, AppWin},
    tf2::lobby::{flag_description, Player, PlayerFlag},
};
use eframe::egui::Ui;

pub fn add_player_flag_editor(app_win: &AppWin, ui: &mut Ui, player: &Player) {
    ui.horizontal_wrapped(|ui| {
        let flags = vec![
            PlayerFlag::Awesome,
            PlayerFlag::Cheater,
            PlayerFlag::Bot,
            PlayerFlag::Suspicious,
            PlayerFlag::Toxic,
            PlayerFlag::Exploiter,
        ];

        let data = transform_data(&player.flags);
        for flag in flags {
            if let Some(marking) = data.get(&flag) {
                add_flag(app_win, ui, marking, player.steamid);
            } else {
                let marking = Marking {
                    sources: Default::default(),
                    suggestion: Default::default(),
                    flag,
                };

                add_flag(app_win, ui, &marking, player.steamid);
            }
        }

        // if ui.button("Save").clicked() {
        //     // app_win.save_player_flags();
        // }
    });
}

fn add_flag(app_win: &AppWin, ui: &mut Ui, marking: &Marking, steamid: SteamID) {
    let flag = marking.flag;

    ui.scope(|ui| {
        ui.set_max_width(140.0);

        let (fgcolor, bgcolor) = color_for_flag(flag);

        // ui.style_mut().visuals.override_text_color = Some(fgcolor);

        // ui.style_mut().
        ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.bg_fill = bgcolor;

        let mut suggested_mark = "";

        if marking.sources.is_empty() && !marking.suggestion.is_empty() {
            suggested_mark = "?";
        }

        let text = format!("{}{}", flag_description(flag), suggested_mark);
        let mut enable = !marking.sources.is_empty();

        if ui
            .checkbox(&mut enable, text)
            .on_hover_ui(|ui| add_flag_tooltip(ui, marking))
            .clicked()
        {
            app_win
                .bus
                .lock()
                .unwrap()
                .app_event_bus
                .broadcast(AppEventMsg::SetPlayerFlag(steamid, flag, enable));
        }
    });
}
