//! A small, consistent vector icon set. No font glyph or emoji dependencies.
use super::theme::*;
use egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Explore,
    Person,
    House,
    Path,
    Terrain,
    Survey,
    Globe,
    Help,
    Menu,
    Play,
    Pause,
    Step,
    Plus,
    Minus,
    Focus,
    Home,
    Water,
}

pub fn paint(p: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let point = |x: f32, y: f32| rect.min + Vec2::new(x, y) * rect.size();
    let stroke = Stroke::new(1.6, color);
    let line = |a: [f32; 2], b: [f32; 2]| {
        p.line_segment([point(a[0], a[1]), point(b[0], b[1])], stroke);
    };
    let path = |points: &[[f32; 2]]| {
        p.add(Shape::line(
            points.iter().map(|v| point(v[0], v[1])).collect(),
            stroke,
        ));
    };
    match icon {
        Icon::Path => {
            path(&[[0.18, 0.9], [0.22, 0.64], [0.57, 0.42], [0.58, 0.12]]);
            path(&[[0.43, 0.9], [0.44, 0.75], [0.79, 0.52], [0.82, 0.12]]);
        }
        Icon::Person => {
            p.circle_stroke(point(0.5, 0.2), rect.width() * 0.11, stroke);
            path(&[[0.25, 0.53], [0.5, 0.4], [0.75, 0.53]]);
            path(&[[0.5, 0.4], [0.5, 0.65], [0.32, 0.9]]);
            line([0.5, 0.65], [0.68, 0.9]);
        }
        Icon::House | Icon::Home => {
            path(&[[0.1, 0.45], [0.5, 0.12], [0.9, 0.45]]);
            path(&[
                [0.22, 0.39],
                [0.22, 0.86],
                [0.42, 0.86],
                [0.42, 0.61],
                [0.61, 0.61],
                [0.61, 0.86],
                [0.78, 0.86],
                [0.78, 0.39],
            ]);
        }
        Icon::Explore => {
            path(&[
                [0.22, 0.12],
                [0.8, 0.51],
                [0.55, 0.57],
                [0.4, 0.86],
                [0.22, 0.12],
            ]);
        }
        Icon::Terrain => {
            path(&[
                [0.06, 0.81],
                [0.38, 0.21],
                [0.6, 0.58],
                [0.74, 0.38],
                [0.95, 0.81],
                [0.06, 0.81],
            ]);
            path(&[[0.26, 0.44], [0.38, 0.5], [0.48, 0.39]]);
        }
        Icon::Survey | Icon::Globe => {
            p.circle_stroke(rect.center(), rect.width() * 0.39, stroke);
            if matches!(icon, Icon::Survey) {
                p.circle_stroke(rect.center(), rect.width() * 0.21, stroke);
                line([0.5, 0.5], [0.8, 0.19]);
                p.circle_filled(point(0.35, 0.62), 2., color);
            } else {
                line([0.11, 0.5], [0.89, 0.5]);
                path(&[
                    [0.5, 0.11],
                    [0.34, 0.3],
                    [0.31, 0.5],
                    [0.34, 0.7],
                    [0.5, 0.89],
                    [0.66, 0.7],
                    [0.69, 0.5],
                    [0.66, 0.3],
                    [0.5, 0.11],
                ]);
            }
        }
        Icon::Help => {
            p.circle_stroke(rect.center(), rect.width() * 0.39, stroke);
            path(&[
                [0.35, 0.35],
                [0.4, 0.27],
                [0.57, 0.27],
                [0.64, 0.35],
                [0.61, 0.44],
                [0.5, 0.51],
                [0.5, 0.57],
            ]);
            p.circle_filled(point(0.5, 0.71), 1.2, color);
        }
        Icon::Menu => {
            for y in [0.28, 0.5, 0.72] {
                line([0.19, y], [0.81, y]);
            }
        }
        Icon::Play | Icon::Step => {
            p.add(Shape::convex_polygon(
                vec![point(0.28, 0.2), point(0.74, 0.5), point(0.28, 0.8)],
                color,
                Stroke::NONE,
            ));
            if matches!(icon, Icon::Step) {
                line([0.85, 0.2], [0.85, 0.8]);
            }
        }
        Icon::Pause => {
            for x in [0.35, 0.65] {
                p.line_segment([point(x, 0.23), point(x, 0.77)], Stroke::new(3., color));
            }
        }
        Icon::Plus | Icon::Minus => {
            line([0.23, 0.5], [0.77, 0.5]);
            if matches!(icon, Icon::Plus) {
                line([0.5, 0.23], [0.5, 0.77]);
            }
        }
        Icon::Focus => {
            for (x, y, dx, dy) in [
                (0.15, 0.15, 1., 1.),
                (0.85, 0.15, -1., 1.),
                (0.15, 0.85, 1., -1.),
                (0.85, 0.85, -1., -1.),
            ] {
                path(&[[x, y + dy * 0.2], [x, y], [x + dx * 0.2, y]]);
            }
            p.circle_stroke(rect.center(), rect.width() * 0.13, stroke);
        }
        Icon::Water => {
            path(&[
                [0.5, 0.1],
                [0.25, 0.47],
                [0.22, 0.65],
                [0.31, 0.82],
                [0.5, 0.88],
                [0.69, 0.82],
                [0.78, 0.65],
                [0.75, 0.47],
                [0.5, 0.1],
            ]);
        }
    }
}

pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    active: bool,
    caption: bool,
    tooltip: &str,
) -> Response {
    let size = if caption {
        Vec2::new(
            if ui.ctx().content_rect().width() < 650. {
                63.
            } else {
                76.
            },
            58.,
        )
    } else {
        Vec2::splat(32.)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("icon_button", label)), rect));
    let color = if !ui.is_enabled() {
        MUTED.gamma_multiply(0.45)
    } else if active {
        ACCENT
    } else {
        TEXT
    };
    let p = ui.painter();
    if active || response.hovered() || response.has_focus() {
        p.rect_filled(
            rect,
            9.,
            Color32::from_white_alpha(if active { 100 } else { 60 }),
        );
    }
    if response.has_focus() {
        p.rect_stroke(rect, 9., Stroke::new(1., ACCENT), egui::StrokeKind::Inside);
    }
    let center = if caption {
        Pos2::new(rect.center().x, rect.top() + 20.)
    } else {
        rect.center()
    };
    paint(
        p,
        Rect::from_center_size(center, Vec2::splat(22.)),
        icon,
        color,
    );
    if caption {
        p.text(
            Pos2::new(rect.center().x, rect.bottom() - 12.),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.),
            color,
        );
    }
    if active {
        p.line_segment(
            [
                rect.center_bottom() + Vec2::new(-8., -2.),
                rect.center_bottom() + Vec2::new(8., -2.),
            ],
            Stroke::new(2., ACCENT),
        );
    }
    response.on_hover_text(tooltip)
}
