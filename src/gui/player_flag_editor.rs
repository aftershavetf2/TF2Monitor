use super::colors::color_for_flag;
use crate::{
    appbus::AppEventMsg,
    models::AppWin,
    tf2::lobby::{player_attribute_description, Player},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::{Checkbox, Ui, Widget};

pub fn add_player_flag_editor(app_win: &AppWin, ui: &mut Ui, player: &Player) {
    ui.horizontal_wrapped(|ui| {
        let player_attributes_to_show = vec![
            PlayerAttribute::Cool,
            PlayerAttribute::Cheater,
            PlayerAttribute::Bot,
            PlayerAttribute::Suspicious,
            PlayerAttribute::Toxic,
            PlayerAttribute::Exploiter,
        ];

        let actual_player_attributes = if let Some(player_info) = &player.player_info {
            &player_info.attributes
        } else {
            &vec![]
        };

        for player_attribute in player_attributes_to_show {
            let enable = actual_player_attributes.contains(&player_attribute);
            add_flag(app_win, ui, player_attribute, enable, player);
        }
    });
}

fn add_flag(
    app_win: &AppWin,
    ui: &mut Ui,
    player_attribute: PlayerAttribute,
    mut enable: bool,
    player: &Player,
) {
    ui.scope(|ui| {
        ui.set_max_width(140.0);

        let (fgcolor, bgcolor) = color_for_flag(player_attribute);

        ui.style_mut().visuals.panel_fill = bgcolor;
        
        ui.style_mut().visuals.widgets.active.bg_fill = bgcolor;

        // ui.style_mut().visuals.override_text_color = Some(fgcolor);

        // ui.style_mut().
        // ui.style_mut().visuals.widgets.active.fg_stroke.color = fgcolor;
        // ui.style_mut().visuals.widgets.active.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.active.bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.inactive.bg_fill = bgcolor;
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.bg_stroke.color = fgcolor;
        ui.style_mut().visuals.widgets.hovered.bg_fill = bgcolor;

        // ui.style_mut()
        //     .visuals
        //     .widgets
        //     .noninteractive
        //     .fg_stroke
        //     .color = fgcolor;

        let (_text, tooltip) = player_attribute_description(player_attribute);
        let checkbox = Checkbox::new(&mut enable, tooltip);

        let response = ui.add(checkbox);

        if response.clicked() {
            app_win
                .bus
                .lock()
                .unwrap()
                .app_event_bus
                .broadcast(AppEventMsg::SetPlayerFlag(
                    player.clone(),
                    player_attribute,
                    enable,
                ));
        }
    });
}
