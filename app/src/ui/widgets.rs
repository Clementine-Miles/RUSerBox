//! Small drawing helpers shared by the pages.

use egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

use crate::theme::Palette;

/// Bytes as `1.5 GiB`.
pub fn bytes(n: u64) -> String {
    const U: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", U[i]) }
}

/// A rate as `1.5 MiB/s`.
pub fn rate(per_sec: f64) -> String {
    format!("{}/s", bytes(per_sec.max(0.0) as u64))
}

/// A rounded card with a title row.
pub fn card<R>(ui: &mut Ui, p: &Palette, title: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(p.surface)
        .corner_radius(14)
        .inner_margin(egui::Margin::same(14))
        .outer_margin(egui::Margin::symmetric(0, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(egui::RichText::new(title).strong().color(p.muted));
                ui.add_space(6.0);
            }
            body(ui)
        })
        .inner
}

/// A horizontal usage bar, 0–1.
pub fn bar(ui: &mut Ui, p: &Palette, frac: f64, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, p.raised);
    let f = frac.clamp(0.0, 1.0) as f32;
    if f > 0.0 {
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::new(rect.width() * f, rect.height())), 4.0, color);
    }
    resp
}

/// The color for a usage fraction: green, yellow, red.
pub fn level(p: &Palette, frac: f64) -> Color32 {
    if frac > 0.9 {
        p.red
    } else if frac > 0.7 {
        p.yellow
    } else {
        p.green
    }
}

/// A line chart of [values] scaled to [max] (or their own maximum).
pub fn sparkline(ui: &mut Ui, p: &Palette, values: &[f64], max: Option<f64>, color: Color32, height: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, p.background);
    if values.len() >= 2 {
        let hi = max.unwrap_or_else(|| values.iter().copied().fold(0.0, f64::max)).max(1e-9);
        let n = crate::core::HISTORY.max(values.len()) - 1;
        let pts: Vec<Pos2> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = rect.right() - (values.len() - 1 - i) as f32 / n as f32 * rect.width();
                let y = rect.bottom() - 2.0 - (v / hi).clamp(0.0, 1.0) as f32 * (rect.height() - 4.0);
                Pos2::new(x, y)
            })
            .collect();
        let mut fill = pts.clone();
        fill.push(Pos2::new(pts[pts.len() - 1].x, rect.bottom()));
        fill.push(Pos2::new(pts[0].x, rect.bottom()));
        painter.add(Shape::convex_polygon(fill, color.gamma_multiply(0.18), Stroke::NONE));
        painter.add(Shape::line(pts, Stroke::new(1.5, color)));
    }
    resp
}

/// A full-width button, comfortable to tap.
pub fn wide_button(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> Response {
    ui.add_sized([ui.available_width(), 44.0], egui::Button::new(text))
}

/// A labelled single-line field.
pub fn field(ui: &mut Ui, label: &str, value: &mut String, password: bool) -> Response {
    ui.label(label);
    let r = ui.add(egui::TextEdit::singleline(value).password(password).desired_width(f32::INFINITY).margin(egui::Margin::symmetric(8, 8)));
    ui.add_space(4.0);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_formatting() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1.5 KiB");
        assert_eq!(bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
        assert_eq!(rate(2048.0), "2.0 KiB/s");
    }
}
