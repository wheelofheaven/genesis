//! Extract the actual painted frame bounds, including popup/scroll clipping.
use egui::{Rect, Shape, epaint::ClippedShape};
use genesis_renderer::glass::Panel;

pub fn panels(shapes: &[ClippedShape], scale: f32) -> Vec<Panel> {
    fn collect(shape: &Shape, clip: Rect, scale: f32, result: &mut Vec<Panel>) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, clip, scale, result);
                }
            }
            Shape::Rect(shape)
                if shape.fill == super::theme::glass_tint()
                    && shape.rect.intersect(clip).is_positive() =>
            {
                let rect = shape.rect;
                let corners = shape.corner_radius;
                result.push(Panel {
                    bounds: [rect.left(), rect.top(), rect.right(), rect.bottom()]
                        .map(|v| v * scale),
                    corners: [corners.nw, corners.ne, corners.sw, corners.se]
                        .map(|v| v as f32 * scale),
                    clip: [clip.left(), clip.top(), clip.right(), clip.bottom()].map(|v| v * scale),
                });
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    for shape in shapes {
        collect(&shape.shape, shape.clip_rect, scale, &mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frosting_tracks_nested_shapes_clip_and_retina_scale() {
        let rect = Rect::from_min_max(egui::pos2(10., 20.), egui::pos2(110., 80.));
        let clip = Rect::from_min_max(egui::pos2(30., 0.), egui::pos2(300., 300.));
        let shapes = [ClippedShape {
            clip_rect: clip,
            shape: Shape::Vec(vec![
                Shape::rect_filled(rect, 12, super::super::theme::glass_tint()),
                Shape::rect_filled(rect, 0, egui::Color32::BLACK),
            ]),
        }];
        let panels = panels(&shapes, 2.);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].bounds, [20., 40., 220., 160.]);
        assert_eq!(panels[0].corners, [24.; 4]);
        assert_eq!(panels[0].clip, [60., 0., 600., 600.]);
    }
}
