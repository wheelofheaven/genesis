use egui::{Color32, FontId, RichText, Stroke, TextStyle};

pub const PAPER: Color32 = Color32::from_rgb(246, 241, 226);
pub const INK: Color32 = Color32::from_rgb(42, 58, 51);
pub const MUTED: Color32 = Color32::from_rgb(101, 111, 96);
pub const LINE: Color32 = Color32::from_rgb(211, 210, 190);
pub const ACCENT: Color32 = Color32::from_rgb(57, 88, 66);
pub const GOLD: Color32 = Color32::from_rgb(152, 103, 38);
pub const ERROR: Color32 = Color32::from_rgb(158, 62, 40);

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
    style.spacing.item_spacing = egui::vec2(8., 10.);
    style.spacing.interact_size = egui::vec2(36., 34.);
    style.animation_time = 0.1;
    style.visuals.override_text_color = Some(INK);
    style.visuals.window_fill = PAPER;
    style.visuals.panel_fill = PAPER;
    style.visuals.window_stroke = Stroke::new(1., LINE);
    style.visuals.window_corner_radius = 12.into();
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1., PAPER);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1., LINE);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., MUTED);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(233, 231, 214);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(233, 231, 214);
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(219, 224, 202);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(219, 224, 202);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., Color32::from_rgb(164, 177, 149));
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(197, 210, 182);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(197, 210, 182);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = 7.into();
        widget.fg_stroke = Stroke::new(1., INK);
    }
    ctx.set_style(style);
}
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(PAPER)
        .stroke(Stroke::new(1., LINE))
        .corner_radius(12)
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
    let text = RichText::new(label).color(if active { PAPER } else { INK });
    let mut button = egui::Button::new(text);
    if active {
        button = button.fill(ACCENT);
    }
    ui.add(button).on_hover_text(tooltip).clicked()
}
pub fn key_hint(ui: &mut egui::Ui, key: &str, action: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(230, 229, 213))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(5, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(key).size(10.).strong().color(INK));
        });
    muted(ui, action);
}
