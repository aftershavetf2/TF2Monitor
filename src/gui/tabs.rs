use eframe::egui::{self, Key, Ui};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Game,
    Overview,
}

impl Tab {
    const ALL: &[Self] = &[Self::Game, Self::Overview];

    fn label(self) -> &'static str {
        match self {
            Self::Game => "Game",
            Self::Overview => "Overview",
        }
    }

    fn shortcut(self) -> (Key, &'static str) {
        match self {
            Self::Game => (Key::Num1, "1"),
            Self::Overview => (Key::Num2, "2"),
        }
    }
}

pub fn add_menu_section(active_tab: &mut Tab, ui: &mut Ui) {
    ui.label("Tabs");
    for &tab in Tab::ALL {
        if super::menu_item::command_with_keys(ui, tab.label(), &[tab.shortcut().1]).clicked() {
            *active_tab = tab;
        }
    }
}

pub fn handle_shortcuts(active_tab: &mut Tab, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() || ctx.memory(|memory| memory.top_modal_layer().is_some()) {
        return;
    }
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
                && modifiers.is_none()
                && let Some(&tab) = Tab::ALL.iter().find(|tab| tab.shortcut().0 == *key)
            {
                *active_tab = tab;
                return false;
            }
            true
        });
    });
}

pub fn add_tab_bar(active_tab: &mut Tab, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for &tab in Tab::ALL {
            ui.selectable_value(active_tab, tab, tab.label());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_shortcuts_switch_tabs_but_respect_editors_modals_and_modifiers() {
        for (editor, modal, modifiers) in [
            (false, false, egui::Modifiers::NONE),
            (true, false, egui::Modifiers::NONE),
            (false, true, egui::Modifiers::NONE),
            (false, false, egui::Modifiers::COMMAND),
            (false, false, egui::Modifiers::SHIFT),
            (false, false, egui::Modifiers::ALT),
        ] {
            let ctx = egui::Context::default();
            let mut tab = Tab::Game;
            let mut text = String::new();
            for key in [None, Some(Key::Num2), Some(Key::Num1)] {
                let _ = super::super::tests::run_ui(
                    &ctx,
                    egui::RawInput {
                        events: key
                            .map(|key| egui::Event::Key {
                                key,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers,
                            })
                            .into_iter()
                            .collect(),
                        ..Default::default()
                    },
                    |root_ui| {
                        let ctx = &root_ui.ctx().clone();
                        egui::CentralPanel::default().show(root_ui, |ui| {
                            if editor {
                                ui.text_edit_singleline(&mut text).request_focus();
                            }
                        });
                        if modal {
                            egui::Modal::new(egui::Id::unique("tab_shortcut_modal")).show(
                                ctx,
                                |ui| {
                                    ui.label("Dialog");
                                },
                            );
                        }
                        handle_shortcuts(&mut tab, ctx);
                    },
                );
                let switches = !editor && !modal && modifiers.is_none();
                assert_eq!(
                    tab,
                    if switches && key == Some(Key::Num2) {
                        Tab::Overview
                    } else {
                        Tab::Game
                    }
                );
            }
        }
    }
}
