//! Shared interface fonts, colors, and widget styling. Painting colors stay in the document model.
use std::sync::Arc;

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/inter/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/inter/Inter-SemiBold.ttf");
const JETBRAINS_MONO_MEDIUM: &[u8] =
    include_bytes!("../../assets/fonts/jetbrains-mono/JetBrainsMono-Medium.ttf");

pub const PANEL: Color32 = Color32::from_rgb(0x36, 0x31, 0x2B);
pub(super) const TABLE_ROW_SELF: Color32 = Color32::from_rgb(0x5F, 0x57, 0x4F);
pub(super) const TABLE_ROW_LIGHT: Color32 = Color32::from_rgb(0x41, 0x3C, 0x36);
pub(super) const TABLE_ROW_DARK: Color32 = Color32::from_rgb(0x2E, 0x29, 0x23);
const POPUP: Color32 = PANEL;
pub const CONTROL: Color32 = Color32::from_gray(49);
const ACCENT: Color32 = Color32::from_rgb(55, 142, 240);
const BUTTON: Color32 = Color32::from_rgb(29, 86, 153);
pub const HOVER: Color32 = Color32::from_rgb(42, 118, 202);
const PRESSED: Color32 = Color32::from_rgb(23, 69, 125);
pub const BORDER: Color32 = Color32::from_gray(66);
pub(super) const TEXT: Color32 = Color32::from_gray(230);
const MUTED_TEXT: Color32 = Color32::from_gray(166);
pub const SELECTION: Color32 = Color32::from_rgb(36, 104, 180);
const ERROR: Color32 = Color32::from_rgb(242, 132, 132);
const TOOLTIP_BACKGROUND: Color32 = Color32::from_rgb(240, 232, 211);
const TOOLTIP_TEXT: Color32 = Color32::from_rgb(48, 44, 35);
const TOOLTIP_BORDER: Color32 = Color32::from_rgb(184, 171, 139);
pub(super) const IMAGE_BACKGROUND: Color32 = Color32::from_gray(38);
pub(super) const IMAGE_BORDER: Color32 = Color32::from_gray(125);

pub(super) enum PanelEdge {
    Top,
    Bottom,
    Left,
}

/// Draw inside the panel so adjacent panel backgrounds cannot clip the separator.
pub(super) fn panel_separator(ctx: &egui::Context, response: &egui::Response, edge: PanelEdge) {
    let rect = response.rect;
    let pixels_per_point = ctx.pixels_per_point();
    let pixel = 1.0 / pixels_per_point;
    let mut line = egui::Rect::from_min_max(
        egui::pos2(
            (rect.min.x * pixels_per_point).ceil() / pixels_per_point,
            (rect.min.y * pixels_per_point).ceil() / pixels_per_point,
        ),
        egui::pos2(
            (rect.max.x * pixels_per_point).floor() / pixels_per_point,
            (rect.max.y * pixels_per_point).floor() / pixels_per_point,
        ),
    );
    match edge {
        PanelEdge::Top => line.max.y = line.min.y + pixel,
        PanelEdge::Bottom => line.min.y = line.max.y - pixel,
        PanelEdge::Left => line.max.x = line.min.x + pixel,
    }
    ctx.layer_painter(response.layer_id)
        .rect_filled(line, 0.0, BORDER);
}

pub(crate) fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "TF2 Classes".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/tf2-classfont/tf2-classfont.ttf"
        ))),
    );
    fonts.families.insert(
        FontFamily::Name("TF2 Classes".into()),
        vec!["TF2 Classes".into()],
    );
    for (name, data) in [
        ("Inter", INTER_REGULAR),
        ("Inter Semibold", INTER_SEMIBOLD),
        ("JetBrains Mono Medium", JETBRAINS_MONO_MEDIUM),
    ] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(data)));
    }
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "Inter".into());
    let mut semibold = proportional.clone();
    semibold.insert(0, "Inter Semibold".into());
    fonts
        .families
        .insert(FontFamily::Name("Inter Semibold".into()), semibold);
    let mut shortcut = fonts.families[&FontFamily::Monospace].clone();
    shortcut.insert(0, "JetBrains Mono Medium".into());
    fonts
        .families
        .insert(FontFamily::Name("JetBrains Mono Medium".into()), shortcut);
    ctx.set_fonts(fonts);

    let mut style = egui::Style {
        text_styles: [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Heading, heading_font()),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size = egui::vec2(28.0, 28.0);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(24);
    style.spacing.slider_width = 140.0;
    style.spacing.icon_width = 14.0;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = POPUP;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.extreme_bg_color = Color32::from_gray(26);
    visuals.faint_bg_color = Color32::from_gray(40);
    visuals.code_bg_color = CONTROL;
    visuals.text_edit_bg_color = Some(CONTROL);
    visuals.override_text_color = None;
    visuals.weak_text_color = Some(MUTED_TEXT);
    visuals.error_fg_color = ERROR;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = SELECTION;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 5],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.slider_trailing_fill = true;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    for (widget, background) in [
        (&mut visuals.widgets.inactive, CONTROL),
        (&mut visuals.widgets.hovered, HOVER),
        (&mut visuals.widgets.active, PRESSED),
        (&mut visuals.widgets.open, HOVER),
    ] {
        widget.bg_fill = background;
        widget.weak_bg_fill = background;
        widget.bg_stroke = Stroke::new(1.0_f32, BORDER);
        widget.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        widget.corner_radius = egui::CornerRadius::same(4);
        widget.expansion = 0.0;
    }
    // Button backgrounds use weak_bg_fill; input fields keep their neutral fill.
    // The menu style clears this resting fill, retaining blue only on hover/open.
    visuals.widgets.inactive.weak_bg_fill = BUTTON;
    style.visuals = visuals;
    // Keep the painting workspace neutral and consistent across OS appearances.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Inter Semibold".into()))
}

pub(super) fn shortcut_font() -> FontId {
    FontId::new(11.0, FontFamily::Name("JetBrains Mono Medium".into()))
}

pub(super) fn modal_frame(ctx: &egui::Context) -> egui::Frame {
    let style = ctx.style();
    egui::Frame::window(&style).inner_margin(style.spacing.window_margin)
}

pub(super) fn heading_font() -> FontId {
    semibold(17.0)
}

pub(super) fn menu_bar() -> egui::MenuBar {
    egui::MenuBar::new()
        .style(|style: &mut egui::Style| {
            egui::containers::menu::menu_style(style);
            style.spacing.button_padding = egui::vec2(10.0, 4.0);
            style.spacing.interact_size.y = 26.0;
        })
        .config(
            egui::containers::menu::MenuConfig::new().style(|style: &mut egui::Style| {
                egui::containers::menu::menu_style(style);
                style.spacing.button_padding = egui::vec2(10.0, 5.0);
                style.spacing.interact_size.y = 27.0;
                style.spacing.item_spacing.y = 3.0;
                style.visuals.weak_text_color = Some(Color32::WHITE);
            }),
        )
}

pub(super) fn menu_contents(ui: &mut egui::Ui) {
    ui.set_min_width(220.0);
}

/// Menu hints sit outside the menu instead of obscuring the following rows.
pub(super) trait MenuTooltip {
    fn menu_tooltip(self, text: impl Into<egui::WidgetText>) -> Self;
}

impl MenuTooltip for egui::Response {
    fn menu_tooltip(self, text: impl Into<egui::WidgetText>) -> Self {
        let menu = self
            .ctx
            .memory(|memory| memory.area_rect(self.layer_id.id))
            .unwrap_or(self.rect);
        let anchor = egui::Rect::from_min_max(
            egui::pos2(menu.left().min(self.rect.left()), self.rect.top()),
            egui::pos2(menu.right().max(self.rect.right()), self.rect.bottom()),
        );
        let frame = egui::Frame::popup(&self.ctx.style())
            .fill(TOOLTIP_BACKGROUND)
            .stroke(Stroke::new(1.0_f32, TOOLTIP_BORDER))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .corner_radius(6);
        let mut tooltip = egui::Tooltip::for_enabled(&self);
        tooltip.popup = tooltip
            .popup
            .anchor(anchor)
            .align(egui::emath::RectAlign::RIGHT_START)
            .align_alternatives(&[
                egui::emath::RectAlign::LEFT_START,
                egui::emath::RectAlign::RIGHT_END,
                egui::emath::RectAlign::LEFT_END,
            ])
            .gap(12.0)
            .frame(frame)
            .width(280.0);
        tooltip.show(|ui| {
            ui.visuals_mut().override_text_color = Some(TOOLTIP_TEXT);
            ui.set_max_width(280.0);
            ui.add(egui::Label::new(text).wrap());
        });
        self
    }
}

pub fn muted_text() -> Color32 {
    MUTED_TEXT
}

/// egui 0.33 popups supply dismissal and navigation; add menu-bar hover switching.
pub fn menu_header<R>(
    ui: &mut egui::Ui,
    label: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let group = ui.id().with("open_menu_header");
    let previous = ui.ctx().data(|d| d.get_temp::<egui::Id>(group));
    let inactive = ui.visuals().widgets.inactive;
    if previous.is_some_and(|id| {
        id == ui.next_auto_id().with("popup") && egui::Popup::is_id_open(ui.ctx(), id)
    }) {
        ui.visuals_mut().widgets.inactive = ui.visuals().widgets.open;
    }
    let response = ui.button(label);
    ui.visuals_mut().widgets.inactive = inactive;
    let id = egui::Popup::default_response_id(&response);
    if response.enabled()
        && response.hovered()
        && previous.is_some_and(|p| p != id && egui::Popup::is_id_open(ui.ctx(), p))
    {
        egui::Popup::open_id(ui.ctx(), id);
    }
    let config = egui::containers::menu::MenuConfig::find(ui);
    let popup = egui::Popup::menu(&response)
        .width(220.0)
        .style(config.style.clone())
        .close_behavior(config.close_behavior)
        .info(
            egui::UiStackInfo::new(egui::UiKind::Menu)
                .with_tag_value(egui::containers::menu::MenuConfig::MENU_CONFIG_TAG, config),
        )
        .show(contents);
    if egui::Popup::is_id_open(ui.ctx(), id) {
        ui.ctx().data_mut(|d| d.insert_temp(group, id));
    }
    egui::InnerResponse::new(popup.map(|p| p.inner), response)
}

/// Common image surface, including the outline on top of loaded pixels.
pub fn image(ui: &mut egui::Ui, image: egui::Image<'_>) -> egui::Response {
    let response = ui.add(image.bg_fill(IMAGE_BACKGROUND));
    ui.painter().rect_stroke(
        response.rect,
        3.0,
        Stroke::new(1.0_f32, IMAGE_BORDER),
        egui::StrokeKind::Inside,
    );
    response
}
