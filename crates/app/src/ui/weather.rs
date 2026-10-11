use genesis_renderer::atmosphere::Atmosphere;

pub(super) fn show(ui: &mut egui::Ui, atmosphere: &mut Atmosphere) {
    ui.collapsing("Sky and wind", |ui| {
        super::theme::muted(ui, "Visual preview · weather simulation is in development");
        ui.horizontal(|ui| {
            for (name, label) in [
                ("clear", "Clear"),
                ("fair", "Fair"),
                ("overcast", "Overcast"),
            ] {
                if ui.button(label).clicked()
                    && let Some(profile) = Atmosphere::preview(name)
                {
                    *atmosphere = profile;
                }
            }
        });
        ui.add(egui::Slider::new(&mut atmosphere.clouds.cover, 0.0..=1.0).text("Cloud cover"));
        let wind = atmosphere.clouds.wind_metres_per_second;
        let mut speed = wind.length();
        // Heading describes motion towards a compass point, not wind's source.
        let mut heading = wind.x.atan2(-wind.y).to_degrees().rem_euclid(360.);
        let speed_changed = ui
            .add(egui::Slider::new(&mut speed, 0.0..=40.0).text("Wind · m/s"))
            .changed();
        let heading_changed = ui
            .add(egui::Slider::new(&mut heading, 0.0..=360.0).text("Towards · °"))
            .changed();
        if speed_changed || heading_changed {
            let angle = heading.to_radians();
            atmosphere.clouds.wind_metres_per_second =
                glam::Vec2::new(angle.sin(), -angle.cos()) * speed;
        }
        super::theme::muted(ui, "0° N · 90° E · 180° S · 270° W");
        if ui.button("Calm wind").clicked() {
            atmosphere.clouds.wind_metres_per_second = glam::Vec2::ZERO;
        }
    });
}
