//! Drawing a [`Terminal`] with egui and turning input into bytes.
//!
//! Cells are laid on a fixed grid measured from the font: runs of ASCII in
//! one style are drawn as one string, everything else (wide, CJK, Nerd Font,
//! combining) one cell at a time, so a fallback glyph of another width never
//! shifts the columns after it.

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::keys::{self, Key, Mods, MouseButton};
use super::Terminal;
use crate::fonts::TERM;
use crate::theme::Palette;

/// What one frame of input produced.
#[derive(Debug, Default)]
pub struct ViewOutput {
    /// Bytes for the remote side.
    pub bytes: Vec<u8>,
    /// The grid changed size: resize the PTY.
    pub resized: Option<(usize, usize)>,
    /// Text the person asked to copy.
    pub copy: Option<String>,
    /// The person tapped the terminal (show the keyboard).
    pub tapped: bool,
}

/// Modifiers latched by the on-screen key bar, applied to the next key.
#[derive(Debug, Default, Clone, Copy)]
pub struct Sticky {
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Default)]
pub struct ViewState {
    drag_accum: f32,
    selecting: bool,
}

fn color_of(c: Color, p: &Palette, ansi: &[Color32; 16], fg: bool, overrides: &alacritty_terminal::term::color::Colors) -> Color32 {
    match c {
        Color::Spec(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
        Color::Indexed(i) => {
            if let Some(rgb) = overrides[i as usize] {
                return Color32::from_rgb(rgb.r, rgb.g, rgb.b);
            }
            indexed(i, ansi)
        }
        Color::Named(n) => {
            if let Some(rgb) = overrides[n as usize] {
                return Color32::from_rgb(rgb.r, rgb.g, rgb.b);
            }
            match n {
                NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::Cursor => p.foreground,
                NamedColor::Background => p.background,
                NamedColor::DimForeground => p.muted,
                n if (n as usize) < 16 => ansi[n as usize],
                n if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize).contains(&(n as usize)) => {
                    ansi[n as usize - NamedColor::DimBlack as usize].gamma_multiply(0.7)
                }
                _ => {
                    if fg {
                        p.foreground
                    } else {
                        p.background
                    }
                }
            }
        }
    }
}

/// The xterm 256-color table.
pub fn indexed(i: u8, ansi: &[Color32; 16]) -> Color32 {
    match i {
        0..=15 => ansi[i as usize],
        16..=231 => {
            let i = i - 16;
            let v = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
            Color32::from_rgb(v(i / 36), v((i / 6) % 6), v(i % 6))
        }
        232..=255 => {
            let g = 8 + (i - 232) * 10;
            Color32::from_rgb(g, g, g)
        }
    }
}

/// Measures a cell of [font].
pub fn cell_size(ui: &egui::Ui, font: &FontId) -> Vec2 {
    ui.fonts_mut(|f| Vec2::new(f.glyph_width(font, 'M').max(1.0), f.row_height(font).max(1.0)))
}

/// Draws the terminal into the available space and handles its input.
pub fn show(ui: &mut egui::Ui, id: egui::Id, t: &mut Terminal, p: &Palette, font_px: f32, sticky: &mut Sticky, st: &mut ViewState) -> ViewOutput {
    let mut out = ViewOutput::default();
    let font = FontId::new(font_px, FontFamily::Name(TERM.into()));
    let cell = cell_size(ui, &font);
    let avail = ui.available_size();
    let cols = ((avail.x / cell.x).floor() as usize).max(2);
    let rows = ((avail.y / cell.y).floor() as usize).max(1);
    if t.resize(cols, rows) {
        out.resized = Some((cols, rows));
    }
    let (rect, resp) = ui.allocate_exact_size(avail, Sense::click_and_drag());
    let resp = resp.on_hover_cursor(egui::CursorIcon::Text);
    let origin = rect.min;
    let to_cell = |pos: Pos2| -> (usize, usize) {
        let c = ((pos.x - origin.x) / cell.x).floor().max(0.0) as usize;
        let r = ((pos.y - origin.y) / cell.y).floor().max(0.0) as usize;
        (c.min(cols - 1), r.min(rows - 1))
    };
    let mode = t.mode();

    // ---- pointer
    if resp.clicked() {
        resp.request_focus();
        out.tapped = true;
        if t.has_selection() {
            t.clear_selection();
        } else if let Some(pos) = resp.interact_pointer_pos() {
            let (c, r) = to_cell(pos);
            if let Some(b) = keys::encode_mouse(MouseButton::Left, true, c, r, mode) {
                out.bytes.extend(b);
                out.bytes.extend(keys::encode_mouse(MouseButton::Left, false, c, r, mode).unwrap_or_default());
            }
        }
    }
    if resp.double_clicked()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let (c, r) = to_cell(pos);
        t.start_selection(c, r, SelectionType::Semantic);
        t.extend_selection(c, r);
    }
    if resp.long_touched()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let (c, r) = to_cell(pos);
        t.start_selection(c, r, SelectionType::Semantic);
        t.extend_selection(c, r);
        st.selecting = true;
    }
    let is_mouse = ui.input(|i| !i.any_touches());
    if resp.drag_started() {
        st.drag_accum = 0.0;
        // A mouse drag selects; a finger drag scrolls (a long press selects).
        if is_mouse && let Some(pos) = resp.interact_pointer_pos() {
            let (c, r) = to_cell(pos);
            t.start_selection(c, r, SelectionType::Simple);
            st.selecting = true;
        }
    }
    if resp.dragged() {
        if st.selecting {
            if let Some(pos) = resp.interact_pointer_pos() {
                let (c, r) = to_cell(pos);
                t.extend_selection(c, r);
            }
        } else {
            st.drag_accum += resp.drag_delta().y;
            let lines = (st.drag_accum / cell.y) as i32;
            if lines != 0 {
                st.drag_accum -= lines as f32 * cell.y;
                scroll_lines(t, lines, mode, &mut out);
            }
        }
    }
    if resp.drag_stopped() {
        st.selecting = false;
        if let Some(text) = t.selection_text() {
            out.copy = Some(text);
        }
    }
    if resp.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel.abs() > 0.0 {
            st.drag_accum += wheel;
            let lines = (st.drag_accum / cell.y) as i32;
            if lines != 0 {
                st.drag_accum -= lines as f32 * cell.y;
                scroll_lines(t, lines, mode, &mut out);
            }
        }
    }

    // ---- keyboard, IME, clipboard
    if resp.has_focus() {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
        let cursor = t.term().renderable_content().cursor.point;
        let crect = Rect::from_min_size(origin + Vec2::new(cursor.column.0 as f32 * cell.x, (cursor.line.0 + t.display_offset() as i32) as f32 * cell.y), cell);
        ui.ctx().output_mut(|o| o.ime = Some(egui::output::IMEOutput { purpose: egui::IMEPurpose::Terminal, rect, cursor_rect: crect, should_interrupt_composition: false }));
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            let mods_of = |m: egui::Modifiers| Mods { ctrl: m.ctrl || m.command && !cfg!(target_os = "macos") || sticky.ctrl, alt: m.alt || sticky.alt, shift: m.shift };
            match ev {
                egui::Event::Text(text) => {
                    let m = Mods { ctrl: sticky.ctrl, alt: sticky.alt, shift: false };
                    out.bytes.extend(keys::encode_text(&text, m));
                    *sticky = Sticky::default();
                    t.scroll_to_bottom();
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => t.preedit = text,
                egui::Event::Ime(egui::ImeEvent::Commit(s)) => {
                    t.preedit.clear();
                    out.bytes.extend(keys::encode_text(&s, Mods { ctrl: sticky.ctrl, alt: sticky.alt, shift: false }));
                    *sticky = Sticky::default();
                    t.scroll_to_bottom();
                }
                egui::Event::Paste(s) => out.bytes.extend(keys::encode_paste(&s, mode)),
                egui::Event::Copy => {
                    if let Some(s) = t.selection_text() {
                        out.copy = Some(s);
                    }
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let m = mods_of(modifiers);
                    let k = match key {
                        egui::Key::Enter => Some(Key::Enter),
                        egui::Key::Backspace => Some(Key::Backspace),
                        egui::Key::Tab => Some(Key::Tab),
                        egui::Key::Escape => Some(Key::Escape),
                        egui::Key::ArrowUp => Some(Key::Up),
                        egui::Key::ArrowDown => Some(Key::Down),
                        egui::Key::ArrowLeft => Some(Key::Left),
                        egui::Key::ArrowRight => Some(Key::Right),
                        egui::Key::Home => Some(Key::Home),
                        egui::Key::End => Some(Key::End),
                        egui::Key::PageUp if m.shift => {
                            t.scroll(rows as i32 / 2);
                            None
                        }
                        egui::Key::PageDown if m.shift => {
                            t.scroll(-(rows as i32) / 2);
                            None
                        }
                        egui::Key::PageUp => Some(Key::PageUp),
                        egui::Key::PageDown => Some(Key::PageDown),
                        egui::Key::Insert => Some(Key::Insert),
                        egui::Key::Delete => Some(Key::Delete),
                        k if (egui::Key::F1 as usize..=egui::Key::F12 as usize).contains(&(k as usize)) => Some(Key::F((k as usize - egui::Key::F1 as usize + 1) as u8)),
                        // Chords with a letter: the Text event does not come
                        // for those, so the key itself is encoded.
                        k if (m.ctrl || m.alt) && k.symbol_or_name().chars().count() == 1 => {
                            let c = k.symbol_or_name().chars().next().unwrap_or(' ').to_ascii_lowercase();
                            Some(Key::Char(c))
                        }
                        _ => None,
                    };
                    if let Some(k) = k
                        && let Some(b) = keys::encode(k, m, mode)
                    {
                        out.bytes.extend(b);
                        *sticky = Sticky::default();
                        t.scroll_to_bottom();
                    }
                }
                _ => {}
            }
        }
    }

    paint(ui, rect, t, p, &font, cell, resp.has_focus());
    out
}

fn scroll_lines(t: &mut Terminal, lines: i32, mode: TermMode, out: &mut ViewOutput) {
    if mode.intersects(TermMode::MOUSE_MODE) {
        let b = if lines > 0 { MouseButton::WheelUp } else { MouseButton::WheelDown };
        for _ in 0..lines.unsigned_abs() {
            out.bytes.extend(keys::encode_mouse(b, true, 0, 0, mode).unwrap_or_default());
        }
    } else if mode.contains(TermMode::ALT_SCREEN) {
        // Full-screen programs without mouse reporting get arrow keys, as
        // other mobile terminals send.
        let k = if lines > 0 { Key::Up } else { Key::Down };
        for _ in 0..lines.unsigned_abs() {
            out.bytes.extend(keys::encode(k, Mods::default(), mode).unwrap_or_default());
        }
    } else {
        t.scroll(lines);
    }
}

fn paint(ui: &egui::Ui, rect: Rect, t: &Terminal, p: &Palette, font: &FontId, cell: Vec2, focused: bool) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, p.background);
    let ansi = p.ansi16();
    let content = t.term().renderable_content();
    let offset = content.display_offset as i32;
    let sel = content.selection;
    let cursor = content.cursor;
    let colors = content.colors;
    let origin = rect.min;
    let at = |pt: Point| origin + Vec2::new(pt.column.0 as f32 * cell.x, (pt.line.0 + offset) as f32 * cell.y);

    struct Run {
        start: Pos2,
        text: String,
        color: Color32,
    }
    let mut run: Option<Run> = None;
    let flush = |run: &mut Option<Run>| {
        if let Some(r) = run.take() {
            painter.text(r.start, Align2::LEFT_TOP, r.text, font.clone(), r.color);
        }
    };

    for item in content.display_iter {
        let pt = item.point;
        let c = item.cell;
        if c.flags.contains(Flags::WIDE_CHAR_SPACER) || c.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) {
            flush(&mut run);
            continue;
        }
        let mut fg = color_of(c.fg, p, &ansi, true, colors);
        let mut bg = color_of(c.bg, p, &ansi, false, colors);
        if c.flags.contains(Flags::BOLD)
            && let Color::Named(n) = c.fg
            && (n as usize) < 8
        {
            fg = ansi[n as usize + 8];
        }
        if c.flags.contains(Flags::DIM) {
            fg = fg.gamma_multiply(0.66);
        }
        let selected = sel.is_some_and(|s| s.contains(pt));
        if c.flags.contains(Flags::INVERSE) ^ selected {
            std::mem::swap(&mut fg, &mut bg);
        }
        if c.flags.contains(Flags::HIDDEN) {
            fg = bg;
        }
        let width = if c.flags.contains(Flags::WIDE_CHAR) { 2.0 } else { 1.0 };
        let pos = at(pt);
        if bg != p.background {
            painter.rect_filled(Rect::from_min_size(pos, Vec2::new(cell.x * width, cell.y)), 0.0, bg);
        }
        if c.flags.intersects(Flags::ALL_UNDERLINES) {
            let y = pos.y + cell.y - 1.5;
            painter.line_segment([Pos2::new(pos.x, y), Pos2::new(pos.x + cell.x * width, y)], Stroke::new(1.0, fg));
        }
        if c.flags.contains(Flags::STRIKEOUT) {
            let y = pos.y + cell.y / 2.0;
            painter.line_segment([Pos2::new(pos.x, y), Pos2::new(pos.x + cell.x * width, y)], Stroke::new(1.0, fg));
        }
        let ch = c.c;
        let zw = c.zerowidth();
        let plain_ascii = ch.is_ascii_graphic() && zw.is_none() && width == 1.0;
        if ch == ' ' || ch == '\0' {
            flush(&mut run);
            continue;
        }
        if plain_ascii {
            match &mut run {
                Some(r) if r.color == fg && (r.start.y - pos.y).abs() < 0.5 && (r.start.x + r.text.len() as f32 * cell.x - pos.x).abs() < 0.5 => r.text.push(ch),
                _ => {
                    flush(&mut run);
                    run = Some(Run { start: pos, text: ch.to_string(), color: fg });
                }
            }
        } else {
            flush(&mut run);
            let mut s = ch.to_string();
            if let Some(z) = zw {
                s.extend(z.iter());
            }
            let center = pos + Vec2::new(cell.x * width / 2.0, 0.0);
            painter.text(center, Align2::CENTER_TOP, s, font.clone(), fg);
        }
    }
    flush(&mut run);

    // Cursor, and the IME composition over it.
    if content.mode.contains(TermMode::SHOW_CURSOR) && offset == 0 && cursor.shape != CursorShape::Hidden {
        let pos = at(cursor.point);
        let r = Rect::from_min_size(pos, cell);
        let color = p.foreground;
        match (cursor.shape, focused) {
            (_, false) | (CursorShape::HollowBlock, _) => {
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Inside);
            }
            (CursorShape::Beam, true) => {
                painter.rect_filled(Rect::from_min_size(pos, Vec2::new(2.0, cell.y)), 0.0, color);
            }
            (CursorShape::Underline, true) => {
                painter.rect_filled(Rect::from_min_size(pos + Vec2::new(0.0, cell.y - 2.0), Vec2::new(cell.x, 2.0)), 0.0, color);
            }
            _ => {
                painter.rect_filled(r, 0.0, color.gamma_multiply(0.75));
                let grid = t.term().grid();
                let ch = grid[Line(cursor.point.line.0)][Column(cursor.point.column.0)].c;
                if ch != ' ' && ch != '\0' && cursor.point.line.0 < grid.screen_lines() as i32 {
                    painter.text(pos + Vec2::new(cell.x / 2.0, 0.0), Align2::CENTER_TOP, ch.to_string(), font.clone(), p.background);
                }
            }
        }
        if !t.preedit.is_empty() {
            let g = painter.layout_no_wrap(t.preedit.clone(), font.clone(), p.foreground);
            let box_ = Rect::from_min_size(pos, g.size());
            painter.rect_filled(box_, 2.0, p.raised);
            painter.line_segment([box_.left_bottom(), box_.right_bottom()], Stroke::new(1.0, p.yellow));
            painter.galley(pos, g, p.foreground);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_256_color_table() {
        let ansi = crate::theme::Palette::of(crate::theme::Preset::Material, true).ansi16();
        assert_eq!(indexed(16, &ansi), Color32::from_rgb(0, 0, 0));
        assert_eq!(indexed(231, &ansi), Color32::from_rgb(255, 255, 255));
        assert_eq!(indexed(196, &ansi), Color32::from_rgb(255, 0, 0));
        assert_eq!(indexed(232, &ansi), Color32::from_rgb(8, 8, 8));
        assert_eq!(indexed(1, &ansi), ansi[1]);
    }
}
