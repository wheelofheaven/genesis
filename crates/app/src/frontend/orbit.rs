//! Cached orbital illustration: Natural Earth silhouettes with decorative shading.
//! This is navigation presentation, not a simulated planet or historical map.
// The orbital illustration has its own luminous palette, independent of HUD ink.
const TEXT: Color32 = Color32::from_rgb(225, 237, 246);
const MUTED: Color32 = Color32::from_rgb(180, 199, 213);
const ACCENT: Color32 = Color32::from_rgb(98, 222, 232);
const LINE: Color32 = Color32::from_rgb(86, 112, 130);
const HIGHLIGHT: Color32 = Color32::from_rgb(183, 174, 255);
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, Vec2};
use genesis_sim_core::worldgen::{
    Region,
    noise::{fbm, unit},
};
use glam::Vec3;

const MASK: &[u8] = include_bytes!("../../../../assets/maps/earth-land.bin");
const SIZE: usize = 768;
fn basis() -> (Vec3, Vec3, Vec3) {
    let lat = 18_f32.to_radians();
    let lon = 24_f32.to_radians();
    (
        Vec3::new(-lon.sin(), 0., lon.cos()),
        Vec3::new(-lat.sin() * lon.cos(), lat.cos(), -lat.sin() * lon.sin()),
        Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()),
    )
}
fn land(lon: f32, lat: f32) -> bool {
    let x = (((lon.to_degrees() + 180.) / 360. * 1024.) as usize).min(1023);
    let y = (((90. - lat.to_degrees()) / 180. * 512.) as usize).min(511);
    let i = y * 1024 + x;
    MASK[i / 8] & (1 << (i % 8)) != 0
}
fn texture(ctx: &egui::Context) -> TextureHandle {
    let mut pixels = vec![Color32::TRANSPARENT; SIZE * SIZE];
    let (right, up, forward) = basis();
    let light = Vec3::new(-0.8, 0.5, 0.7).normalize();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let nx = (x as f32 + 0.5) / SIZE as f32 * 2. - 1.;
            let ny = 1. - (y as f32 + 0.5) / SIZE as f32 * 2.;
            let r2 = nx * nx + ny * ny;
            if r2 > 1. {
                continue;
            }
            let nz = (1. - r2).sqrt();
            let p = right * nx + up * ny + forward * nz;
            let lon = p.z.atan2(p.x);
            let lat = p.y.asin();
            let texture = fbm(106, lon * 16., lat * 16.);
            let arid = (1. - ((lat.to_degrees().abs() - 24.) / 15.).abs()).clamp(0., 1.);
            let mut color = if land(lon, lat) {
                Vec3::new(49., 88., 77.).lerp(Vec3::new(157., 144., 102.), arid)
                    * (0.9 + texture * 0.25)
            } else {
                Vec3::new(15., 56., 88.) * (1. + texture * 0.15)
            };
            let cloud =
                ((fbm(812, lon * 5. + (lat * 5.).sin(), lat * 12.) - 0.08) * 2.2).clamp(0., 0.82);
            color = color.lerp(Vec3::new(213., 229., 234.), cloud);
            let illumination = (Vec3::new(nx, ny, nz).dot(light) * 0.9 + 0.22).clamp(0.065, 1.);
            color *= illumination;
            let atmosphere = (1. - nz).powf(3.) * 0.55;
            color = color.lerp(Vec3::new(57., 162., 208.) * illumination.sqrt(), atmosphere);
            let alpha = ((1. - r2) * SIZE as f32).clamp(0., 1.);
            pixels[y * SIZE + x] = Color32::from_rgba_unmultiplied(
                color.x as u8,
                color.y as u8,
                color.z as u8,
                (255. * alpha) as u8,
            );
        }
    }
    ctx.load_texture(
        "orbital-earth",
        egui::ColorImage::new([SIZE, SIZE], pixels),
        egui::TextureOptions::LINEAR,
    )
}
fn project(lat: f32, lon: f32, center: Pos2, radius: f32) -> Option<Pos2> {
    let (right, up, forward) = basis();
    let p = Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
    (p.dot(forward) > 0.).then(|| center + Vec2::new(p.dot(right), -p.dot(up)) * radius)
}

pub fn show(
    ctx: &egui::Context,
    cached: &mut Option<TextureHandle>,
    panel_width: f32,
    region: Option<Region>,
) {
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, 0., Color32::from_rgb(4, 10, 19));
    for i in 0..180 {
        let p = rect.min + Vec2::new(unit(831, i) * rect.width(), unit(529, i) * rect.height());
        let brightness = (45. + unit(818, i) * 95.) as u8;
        painter.circle_filled(
            p,
            0.4 + unit(74, i) * 0.6,
            Color32::from_rgb(brightness, brightness, brightness.saturating_add(20)),
        );
    }
    let space = Rect::from_min_max(egui::pos2(panel_width + 12., rect.top()), rect.max);
    let painter = painter.with_clip_rect(space);
    let radius = (space.width() * 0.46).min(rect.height() * 0.39);
    let center = egui::pos2(space.center().x, rect.center().y - 14.);
    for i in (1..=12).rev() {
        painter.circle_stroke(
            center,
            radius + i as f32 * 1.4,
            Stroke::new(
                2.,
                Color32::from_rgba_unmultiplied(63, 170, 220, (17 - i) as u8),
            ),
        );
    }
    let tex = cached.get_or_insert_with(|| texture(ctx));
    painter.image(
        tex.id(),
        Rect::from_center_size(center, Vec2::splat(radius * 2.)),
        Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
        Color32::WHITE,
    );
    if let Some(region) = region {
        let stroke = Stroke::new(0.6, Color32::from_rgba_unmultiplied(146, 211, 229, 40));
        for lat in [-60_f32, -30., 0., 30., 60.] {
            let mut last = None;
            for lon in (-180..=180).step_by(3) {
                let current = project(lat.to_radians(), (lon as f32).to_radians(), center, radius);
                if let (Some(a), Some(b)) = (last, current) {
                    painter.line_segment([a, b], stroke);
                }
                last = current;
            }
        }
        if let Some(anchor) = region.anchor()
            && let Some(p) = project(
                (anchor.latitude as f32).to_radians(),
                (anchor.longitude as f32).to_radians(),
                center,
                radius,
            )
        {
            painter.circle_stroke(p, 11., Stroke::new(1., ACCENT));
            painter.circle_filled(p, 3., Color32::WHITE);
            let end = p + Vec2::new(38., -32.);
            painter.line_segment([p, end], Stroke::new(1., ACCENT));
            painter.line_segment([end, end + Vec2::new(50., 0.)], Stroke::new(1., ACCENT));
            painter.text(
                end + Vec2::new(2., -7.),
                egui::Align2::LEFT_BOTTOM,
                "LEVANT",
                FontId::monospace(11.),
                TEXT,
            );
        }
    }
    let margin = if space.width() > 500. { 40. } else { 12. };
    painter.text(
        egui::pos2(space.left() + margin, 34.),
        egui::Align2::LEFT_TOP,
        "SOL / EARTH",
        FontId::monospace(13.),
        ACCENT,
    );
    painter.text(
        egui::pos2(space.left() + margin, 56.),
        egui::Align2::LEFT_TOP,
        "ORBITAL OBSERVATION",
        FontId::monospace(10.),
        MUTED,
    );
    // Foreground observation vessel: restrained structural geometry and emissive strips.
    vessel(
        &painter,
        egui::pos2(space.right() - margin, rect.bottom() - 52.),
        space.width().min(440.) * 0.65,
    );
    painter.text(
        egui::pos2(space.left() + margin, rect.bottom() - 28.),
        egui::Align2::LEFT_BOTTOM,
        "EXPEDITION / GENESIS",
        FontId::monospace(10.),
        MUTED,
    );
}
fn vessel(p: &Painter, origin: Pos2, size: f32) {
    let point = |x, y| origin + Vec2::new(x * size, y * size);
    // Flat radiator wings sit behind an axial observation hull.
    for (points, fill) in [
        (
            vec![
                point(-0.8, -0.06),
                point(-0.85, -0.32),
                point(-0.34, -0.44),
                point(-0.30, -0.18),
            ],
            Color32::from_rgb(24, 47, 66),
        ),
        (
            vec![
                point(-0.74, 0.02),
                point(-0.5, 0.18),
                point(-0.03, 0.02),
                point(-0.29, -0.14),
            ],
            Color32::from_rgb(24, 47, 66),
        ),
        (
            vec![
                point(-1., 0.),
                point(-0.76, -0.15),
                point(-0.2, -0.24),
                point(0., -0.16),
                point(-0.13, -0.05),
                point(-0.76, 0.08),
            ],
            Color32::from_rgb(93, 116, 133),
        ),
        (
            vec![
                point(-0.76, 0.08),
                point(-0.13, -0.05),
                point(0., -0.16),
                point(-0.07, -0.02),
                point(-0.76, 0.12),
            ],
            Color32::from_rgb(30, 48, 65),
        ),
    ] {
        p.add(egui::Shape::convex_polygon(
            points,
            fill,
            Stroke::new(1., LINE),
        ));
    }
    for i in 0..6 {
        let t = i as f32 / 6.;
        p.line_segment(
            [
                point(-0.80 + t * 0.42, -0.29 - t * 0.1),
                point(-0.78 + t * 0.42, -0.17 - t * 0.1),
            ],
            Stroke::new(1., Color32::from_rgb(68, 117, 142)),
        );
    }
    p.line_segment(
        [point(-0.72, -0.04), point(-0.3, -0.13)],
        Stroke::new(2., ACCENT),
    );
    p.line_segment(
        [point(-0.93, 0.02), point(-0.81, 0.055)],
        Stroke::new(3., HIGHLIGHT),
    );
    p.circle_filled(point(-0.21, -0.15), 2.5, TEXT);
}
