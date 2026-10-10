use super::{Frontend, Mode, Screen};
use crate::ui::theme::*;
use egui::{Color32, RichText};

pub(super) fn action(ui: &mut egui::Ui, text: &str, primary: bool) -> bool {
    let mut button =
        egui::Button::new(
            RichText::new(text)
                .size(16.)
                .color(if primary { PANEL } else { TEXT }),
        );
    if primary {
        button = button.fill(ACCENT);
    }
    let height = if ui.ctx().content_rect().height() < 650. {
        36.
    } else {
        46.
    };
    ui.add_sized([ui.available_width(), height], button)
        .clicked()
}
pub(super) fn heading(ui: &mut egui::Ui, section: &str, title: &str, subtitle: &str) {
    eyebrow(ui, section);
    ui.add_space(6.);
    ui.label(RichText::new(title).size(30.).color(TEXT));
    muted(ui, subtitle);
    ui.add_space(16.);
}

pub fn show(
    front: &mut Frontend,
    ctx: &egui::Context,
    session: &mut crate::session::Session,
    camera: &mut genesis_renderer::camera::Camera,
) -> Vec<egui::Rect> {
    let rect = ctx.content_rect();
    if front.screen == Screen::Arrival {
        return super::expedition::arrival(front, ctx, session, camera);
    }
    let width = (rect.width() * 0.37).clamp(350., 480.).min(rect.width());
    if front.surface_backdrop() {
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            rect,
            0.,
            Color32::from_white_alpha(8),
        );
    } else {
        let region = matches!(
            front.screen,
            Screen::Setup | Screen::Briefing | Screen::Loading | Screen::ConfirmNew
        )
        .then_some(front.region);
        super::orbit::show(ctx, &mut front.planet_texture, width, region);
    }
    egui::SidePanel::left("front_menu")
        .resizable(false).exact_width(width)
        .frame(egui::Frame::new().fill(if front.surface_backdrop() { glass_tint() } else { Color32::from_rgba_unmultiplied(235, 247, 252, 185) }).inner_margin(egui::Margin::symmetric(26, 22)))
        .show(ctx, |ui| {
            if rect.height() < 650. { ui.spacing_mut().item_spacing.y = 6.; }
            egui::ScrollArea::vertical().id_salt(("menu_scroll", front.screen))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match front.screen {
                        Screen::Home => home(front, ui),
                        Screen::Destination => super::expedition::destination(front, ui),
                        Screen::Setup => super::expedition::region(front, ui),
                        Screen::Briefing => super::expedition::briefing(front, ui),
                        Screen::Settings => settings(front, ui),
                        Screen::Controls => controls(front, ui),
                        Screen::Pause => pause(front, ui),
                        Screen::Loading => {
                            heading(ui, "EXPEDITION / SURFACE LINK", "Preparing descent", front.region.name());
                            ui.spinner();
                            ui.label("Preparing terrain, water systems, and the settlement site.");
                            ui.add_space(20.);
                            if action(ui, "Cancel deployment", false) { front.escape(); }
                        }
                        Screen::ConfirmNew => {
                            heading(ui, "NEW EXPEDITION", "Replace this session?", "Your current settlement will be lost. Saving sessions is not available yet.");
                            if action(ui, "Deploy new expedition", true) { front.start(); }
                            if action(ui, "Back to briefing", false) { front.screen = Screen::Briefing; }
                        }
                        Screen::ConfirmQuit => {
                            heading(ui, "DISCONNECT", "Quit Genesis?", "Your current session will be lost. Saving sessions is not available yet.");
                            if action(ui, "Quit game", true) { front.quit = true; }
                            if action(ui, "Stay here", false) { front.escape(); }
                        }
                        Screen::Playing | Screen::Arrival => {}
                    }
                    if !front.message.is_empty() {
                        ui.add_space(12.);
                        ui.label(RichText::new(&front.message).color(ERROR));
                    }
                    ui.add_space(24.);
                    ui.separator();
                    eyebrow(ui, "GENESIS / EARLY DEVELOPMENT");
                });
        });
    vec![rect]
}
fn home(front: &mut Frontend, ui: &mut egui::Ui) {
    ui.add_space(24.);
    eyebrow(ui, "EXPEDITION COMMAND");
    ui.add_space(10.);
    ui.label(RichText::new("GENESIS").size(48.).color(TEXT));
    ui.label(
        RichText::new("Beyond the stars. Among their lives.")
            .size(16.)
            .color(ACCENT),
    );
    ui.add_space(24.);
    if front.has_session && action(ui, "Continue expedition", true) {
        front.screen = Screen::Playing;
    }
    if action(ui, "Begin expedition", !front.has_session) {
        front.begin_expedition(Mode::Settlement);
    }
    if action(ui, "Settings", false) {
        front.enter_settings();
    }
    if action(ui, "Operations guide", false) {
        front.return_to = Screen::Home;
        front.screen = Screen::Controls;
    }
    if action(ui, "Quit", false) {
        front.request_quit();
    }
    ui.add_space(20.);
    eyebrow(ui, "SETTLEMENT / COLONY BUILDER");
    ui.label("Guide a community. Understand its environment. Shape what comes next.");
    muted(
        ui,
        "Your civilization crosses the stars. Your work begins with a place to live.",
    );
    ui.add_space(10.);
    if ui.link("World Builder sandbox").clicked() {
        front.begin_expedition(Mode::WorldBuilder);
    }
    if front.has_session {
        muted(ui, "Continue is available while the app remains open.");
    }
}
fn pause(front: &mut Frontend, ui: &mut egui::Ui) {
    heading(
        ui,
        "GENESIS",
        "Expedition paused",
        "Your world is suspended while you’re here.",
    );
    if action(ui, "Resume world", true) {
        front.screen = Screen::Playing;
    }
    if action(ui, "Settings", false) {
        front.enter_settings();
    }
    if action(ui, "Operations guide", false) {
        front.return_to = Screen::Pause;
        front.screen = Screen::Controls;
    }
    if action(ui, "Main menu", false) {
        front.screen = Screen::Home;
    }
    if action(ui, "Quit", false) {
        front.request_quit();
    }
    muted(
        ui,
        "Returning to the main menu keeps this session available. Closing the app loses the world; saving is not available yet.",
    );
}
fn settings(front: &mut Frontend, ui: &mut egui::Ui) {
    heading(
        ui,
        "PREFERENCES",
        "Settings",
        "Preferences are saved on this device.",
    );
    eyebrow(ui, "DISPLAY");
    ui.checkbox(&mut front.draft.fullscreen, "Borderless fullscreen");
    ui.add(
        egui::Slider::new(&mut front.draft.scale, 0.85..=1.35)
            .text("Interface scale")
            .step_by(0.05),
    );
    ui.horizontal(|ui| {
        ui.label("Frame limit");
        for fps in [30, 60, 120] {
            let selected = front.draft.fps == fps;
            if ui
                .selectable_label(
                    selected,
                    RichText::new(format!("{fps}")).color(if selected { PANEL } else { TEXT }),
                )
                .clicked()
            {
                front.draft.fps = fps;
            }
        }
    });
    ui.add_space(16.);
    eyebrow(ui, "NAVIGATION");
    ui.add(
        egui::Slider::new(&mut front.draft.sensitivity, 0.4..=2.)
            .text("Camera sensitivity")
            .step_by(0.1),
    );
    ui.checkbox(&mut front.draft.invert_zoom, "Reverse scroll zoom");
    muted(
        ui,
        "Sensitivity affects camera movement, dragging and scrolling.",
    );
    ui.add_space(20.);
    if action(ui, "Apply settings", true) {
        front.settings = front.draft.clone();
        match front.settings.save() {
            Ok(()) => {
                front.screen = front.return_to;
                front.message.clear();
            }
            Err(error) => {
                front.message =
                    format!("Applied for this session, but could not save preferences: {error}")
            }
        }
    }
    ui.columns(2, |columns| {
        if action(&mut columns[0], "Defaults", false) {
            front.draft = super::Settings::default();
        }
        if action(&mut columns[1], "Cancel", false) {
            front.escape();
        }
    });
}
fn controls(front: &mut Frontend, ui: &mut egui::Ui) {
    heading(
        ui,
        "OPERATIONS / FIELD GUIDE",
        "Find your way",
        "Explore freely. Pause whenever you need time to think.",
    );
    for (key, description) in [
        ("WASD / arrows", "Move across the landscape"),
        ("Shift", "Move faster / repeat building placement"),
        ("Right drag", "Orbit the camera"),
        ("Middle drag / Alt + drag", "Pan the camera"),
        ("Scroll", "Zoom toward the pointer"),
        ("Click / F", "Select a place / focus on it"),
        ("R", "Return to the region overview"),
        ("Space / 1, 2, 3", "Pause time / simulation speed"),
        ("B / Q, E", "Place a dwelling / rotate"),
        ("V", "Open hydrology and mineral surveys"),
        ("Escape", "Close a tool, then open the pause menu"),
    ] {
        ui.label(RichText::new(key).strong());
        muted(ui, description);
    }
    ui.add_space(12.);
    muted(
        ui,
        "Survey → Seafloor reveals underwater terrain. Landscape lets you raise, lower and plant the selected ground.",
    );
    if action(ui, "Back", true) {
        front.escape();
    }
}
