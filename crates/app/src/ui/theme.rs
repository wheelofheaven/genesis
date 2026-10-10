use egui::{Color32, FontId, RichText, Stroke, TextStyle};

pub const PANEL: Color32 = Color32::from_rgb(239, 248, 252);
/// Unique fill marks surfaces that receive a GPU-blurred backdrop.
pub fn glass_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(235, 247, 252, 72)
}
pub const TEXT: Color32 = Color32::from_rgb(15, 31, 39);
pub const MUTED: Color32 = Color32::from_rgb(35, 57, 65);
pub const LINE: Color32 = Color32::from_rgb(104, 130, 139);
pub const ACCENT: Color32 = Color32::from_rgb(0, 99, 120);
pub const HIGHLIGHT: Color32 = Color32::from_rgb(76, 64, 133);
pub const ERROR: Color32 = Color32::from_rgb(157, 49, 35);

pub fn install(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::light(),
        ..Default::default()
    };
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(11.));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(20.));
    style.spacing.button_padding = egui::vec2(12., 9.);
    style.spacing.item_spacing = egui::vec2(8., 8.);
    style.spacing.interact_size = egui::vec2(36., 34.);
    style.animation_time = 0.08;
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.window_fill = glass_tint();
    style.visuals.panel_fill = glass_tint();
    style.visuals.extreme_bg_color = Color32::from_black_alpha(12);
    style.visuals.window_stroke = Stroke::new(1., LINE);
    style.visuals.window_corner_radius = 12.into();
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1., PANEL);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1., LINE);
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_white_alpha(40);
    style.visuals.widgets.noninteractive.weak_bg_fill = Color32::from_white_alpha(16);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., MUTED);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    // Strong fills are used by slider rails and checkbox interiors; buttons use weak_bg_fill.
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(35, 85, 105, 65);
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(65);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(35, 85, 105, 100);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., Color32::from_rgb(0, 99, 120));
    style.visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(105);
    style.visuals.widgets.active.bg_fill = ACCENT;
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = 7.into();
        widget.fg_stroke = Stroke::new(1., TEXT);
    }
    ctx.set_style(style);
}
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(glass_tint())
        .stroke(Stroke::new(1., Color32::from_white_alpha(125)))
        .corner_radius(14)
        .inner_margin(14)
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(22),
        })
}
pub fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.).strong().color(MUTED));
}
pub fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(12.).color(MUTED));
}
pub fn button(ui: &mut egui::Ui, label: &str, active: bool, tooltip: &str) -> bool {
    let text = RichText::new(label).color(if active { ACCENT } else { TEXT });
    let mut button = egui::Button::new(text);
    if active {
        button = button
            .fill(Color32::from_white_alpha(100))
            .stroke(Stroke::new(
                1.,
                Color32::from_rgba_unmultiplied(0, 99, 120, 90),
            ));
    }
    ui.add(button).on_hover_text(tooltip).clicked()
}
