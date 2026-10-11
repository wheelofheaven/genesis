//! Resolution-independent expedition pointer, with a contextual tool badge.
use crate::{interaction::Object, session::Session};
use egui::{Color32, Pos2, Stroke, Vec2};

pub fn paint(ctx: &egui::Context, session: &Session, world: bool) -> bool {
    let Some(p) = ctx.pointer_hover_pos() else {
        return false;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        "expedition_cursor".into(),
    ));
    let active =
        world && (session.placing || session.path.active || session.hovered_object.is_some());
    let color = if active {
        Color32::from_rgb(66, 218, 220)
    } else {
        Color32::from_rgb(239, 251, 252)
    };
    let stroke = Stroke::new(1.4, Color32::from_rgb(28, 58, 67));
    // Tip is the exact hit point; contrasting edge remains legible on sky and snow.
    painter.add(egui::Shape::convex_polygon(
        vec![
            p,
            p + Vec2::new(4., 20.),
            p + Vec2::new(9., 12.),
            p + Vec2::new(19., 9.),
        ],
        color,
        stroke,
    ));
    if world {
        let badge = if session.placing {
            Some(crate::ui::icons::Icon::House)
        } else if session.path.active {
            Some(crate::ui::icons::Icon::Path)
        } else {
            session.hovered_object.map(|o| match o {
                Object::Building(_) => crate::ui::icons::Icon::House,
                Object::Path(_) => crate::ui::icons::Icon::Path,
            })
        };
        if let Some(icon) = badge {
            let center = p + Vec2::new(27., 27.);
            painter.circle_filled(
                center,
                11.,
                Color32::from_rgba_unmultiplied(240, 252, 253, 235),
            );
            crate::ui::icons::paint(
                &painter,
                egui::Rect::from_center_size(center, Vec2::splat(16.)),
                icon,
                Color32::from_rgb(15, 100, 110),
            );
        }
        let label = if session.placing {
            session.hover.map(|_| {
                session
                    .sim
                    .validate_building(&session.candidate())
                    .err()
                    .map_or("Place dwelling".into(), |e| e.to_string())
            })
        } else if session.path.active {
            Some(if session.path.points.is_empty() {
                "Click to start path".into()
            } else {
                "Click waypoint · Enter builds".into()
            })
        } else {
            session
                .hovered_object
                .map(|o| format!("{} · Select", o.label()))
        };
        if let Some(label) = label {
            let galley = painter.layout_no_wrap(
                label,
                egui::FontId::proportional(12.),
                Color32::from_rgb(27, 65, 73),
            );
            let size = galley.size() + Vec2::new(16., 10.);
            let screen = ctx.content_rect();
            let pos = Pos2::new(
                (p.x + 42.)
                    .min(screen.right() - size.x - 6.)
                    .max(screen.left() + 6.),
                (p.y + 20.).min(screen.bottom() - size.y - 6.),
            );
            painter.rect_filled(
                egui::Rect::from_min_size(pos, size),
                5.,
                Color32::from_rgba_unmultiplied(241, 252, 252, 240),
            );
            painter.galley(pos + Vec2::new(8., 5.), galley, Color32::WHITE);
        }
    }
    true
}
