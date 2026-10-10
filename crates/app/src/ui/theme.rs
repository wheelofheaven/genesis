use egui::{Color32, FontId, RichText, Stroke, TextStyle};

pub const PANEL: Color32 = Color32::from_rgb(16, 27, 40);
pub const TEXT: Color32 = Color32::from_rgb(225, 237, 246);
pub const MUTED: Color32 = Color32::from_rgb(146, 169, 189);
pub const LINE: Color32 = Color32::from_rgb(47, 70, 90);
pub const ACCENT: Color32 = Color32::from_rgb(98, 222, 232);
pub const HIGHLIGHT: Color32 = Color32::from_rgb(183, 174, 255);
pub const ERROR: Color32 = Color32::from_rgb(255, 137, 121);

pub fn install(ctx: &egui::Context) {
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
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
    style.spacing.item_spacing = egui::vec2(8., 10.);
    style.spacing.interact_size = egui::vec2(36., 34.);
    style.animation_time = 0.08;
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.window_fill = PANEL;
    style.visuals.panel_fill = PANEL;
    style.visuals.window_stroke = Stroke::new(1., LINE);
    style.visuals.window_corner_radius = 4.into();
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1., PANEL);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1., LINE);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., MUTED);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(25, 43, 60);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(25, 43, 60);
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(35, 60, 79);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(35, 60, 79);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., Color32::from_rgb(98, 222, 232));
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(47, 77, 96);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(47, 77, 96);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = 3.into();
        widget.fg_stroke = Stroke::new(1., TEXT);
    }
    ctx.set_style(style);
}
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1., LINE))
        .corner_radius(4)
        .inner_margin(14)
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(28),
        })
}
pub fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.).strong().color(MUTED));
}
pub fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(12.).color(MUTED));
}
pub fn button(ui: &mut egui::Ui, label: &str, active: bool, tooltip: &str) -> bool {
    let text = RichText::new(label).color(if active { PANEL } else { TEXT });
    let mut button = egui::Button::new(text);
    if active {
        button = button.fill(ACCENT);
    }
    ui.add(button).on_hover_text(tooltip).clicked()
}
pub fn key_hint(ui: &mut egui::Ui, key: &str, action: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(29, 47, 63))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(5, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(key).size(10.).strong().color(TEXT));
        });
    muted(ui, action);
}
