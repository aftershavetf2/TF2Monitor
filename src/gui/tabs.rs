use eframe::egui::Ui;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Game,
}

impl Tab {
    const ALL: &[Self] = &[Self::Game];

    fn label(self) -> &'static str {
        match self {
            Self::Game => "Game",
        }
    }
}

pub fn add_tab_bar(active_tab: &mut Tab, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for &tab in Tab::ALL {
            ui.selectable_value(active_tab, tab, tab.label());
        }
    });
}
