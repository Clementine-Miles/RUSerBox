//! The terminal emulator: protocol interpretation (alacritty_terminal's VT
//! state machine and grid) kept apart from input encoding ([`keys`]) and from
//! drawing ([`view`]). Nothing here knows where the bytes come from; a
//! session feeds [`Terminal::feed`] and sends what [`Terminal::take_output`]
//! returns.

pub mod keys;
pub mod view;

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

/// What the emulator asks of its host, collected between frames.
#[derive(Default)]
struct Shared {
    /// Bytes to send back to the remote side (device status replies, …).
    output: Vec<u8>,
    title: Option<String>,
    bell: bool,
    /// OSC 52 clipboard writes.
    clipboard: Option<String>,
}

#[derive(Clone)]
pub struct Listener(Arc<Mutex<Shared>>);

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let mut s = self.0.lock().unwrap_or_else(|e| e.into_inner());
        match event {
            Event::PtyWrite(text) => s.output.extend_from_slice(text.as_bytes()),
            Event::Title(t) => s.title = Some(t),
            Event::ResetTitle => s.title = None,
            Event::Bell => s.bell = true,
            Event::ClipboardStore(_, text) => s.clipboard = Some(text),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    term: Term<Listener>,
    parser: Processor,
    shared: Arc<Mutex<Shared>>,
    size: Size,
    /// IME composition in progress, drawn at the cursor and never sent.
    pub preedit: String,
}

/// Lines of scrollback kept per session.
pub const SCROLLBACK: usize = 10_000;

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let size = Size { cols: cols.max(2), rows: rows.max(1) };
        let config = Config { scrolling_history: SCROLLBACK, ..Config::default() };
        let term = Term::new(config, &size, Listener(shared.clone()));
        Terminal { term, parser: Processor::new(), shared, size, preedit: String::new() }
    }

    /// Interprets bytes from the remote side.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn size(&self) -> Size {
        self.size
    }

    /// Resizes the grid; answers whether it changed (the PTY must follow).
    pub fn resize(&mut self, cols: usize, rows: usize) -> bool {
        let new = Size { cols: cols.max(2), rows: rows.max(1) };
        if new == self.size {
            return false;
        }
        self.size = new;
        self.term.resize(new);
        true
    }

    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }

    pub fn term(&self) -> &Term<Listener> {
        &self.term
    }

    /// Bytes the emulator wants sent back (answers to queries).
    pub fn take_output(&self) -> Vec<u8> {
        std::mem::take(&mut self.shared.lock().unwrap_or_else(|e| e.into_inner()).output)
    }

    pub fn title(&self) -> Option<String> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner()).title.clone()
    }

    pub fn take_bell(&self) -> bool {
        std::mem::take(&mut self.shared.lock().unwrap_or_else(|e| e.into_inner()).bell)
    }

    pub fn take_clipboard(&self) -> Option<String> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner()).clipboard.take()
    }

    /// Scrolls the view through scrollback; positive is up (older).
    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// A viewport cell to a grid point (accounting for scrollback).
    pub fn point_at(&self, col: usize, row: usize) -> Point {
        let offset = self.display_offset() as i32;
        Point::new(Line(row as i32 - offset), Column(col.min(self.size.cols - 1)))
    }

    pub fn start_selection(&mut self, col: usize, row: usize, kind: SelectionType) {
        let p = self.point_at(col, row);
        self.term.selection = Some(Selection::new(kind, p, Side::Left));
    }

    pub fn extend_selection(&mut self, col: usize, row: usize) {
        let p = self.point_at(col, row);
        if let Some(sel) = self.term.selection.as_mut() {
            sel.update(p, Side::Right);
        }
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    pub fn has_selection(&self) -> bool {
        self.term.selection.as_ref().is_some_and(|s| !s.is_empty())
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }

    /// The visible screen as text, one line per row (for tests and copy-all).
    pub fn screen_text(&self) -> String {
        let grid = self.term.grid();
        let mut out = String::new();
        for row in 0..self.size.rows {
            let line = Line(row as i32 - grid.display_offset() as i32);
            let mut s = String::new();
            for col in 0..self.size.cols {
                let cell = &grid[line][Column(col)];
                if cell.flags.contains(alacritty_terminal::term::cell::Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                s.push(cell.c);
                if let Some(z) = cell.zerowidth() {
                    s.extend(z.iter());
                }
            }
            out.push_str(s.trim_end());
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_and_cursor_motion() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"hello\r\nworld\x1b[1;3HX");
        let s = t.screen_text();
        assert_eq!(s.lines().next(), Some("heXlo"));
        assert_eq!(s.lines().nth(1), Some("world"));
    }

    #[test]
    fn colors_and_alternate_screen() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"main\x1b[?1049h\x1b[H\x1b[31malt\x1b[0m");
        assert!(t.mode().contains(TermMode::ALT_SCREEN));
        assert_eq!(t.screen_text().lines().next(), Some("alt"));
        t.feed(b"\x1b[?1049l");
        assert!(!t.mode().contains(TermMode::ALT_SCREEN));
        assert_eq!(t.screen_text().lines().next(), Some("main"));
    }

    #[test]
    fn wide_and_combining_characters() {
        let mut t = Terminal::new(10, 2);
        t.feed("中文e\u{301}".as_bytes());
        assert_eq!(t.screen_text().lines().next(), Some("中文e\u{301}"));
        // Two wide chars take four columns; the combining mark adds none.
        let grid = t.term().grid();
        assert_eq!(grid[Line(0)][Column(4)].c, 'e');
    }

    #[test]
    fn device_status_queries_are_answered() {
        let mut t = Terminal::new(10, 5);
        t.feed(b"\x1b[3;4H\x1b[6n");
        assert_eq!(t.take_output(), b"\x1b[3;4R");
    }

    #[test]
    fn scrollback_and_resize() {
        let mut t = Terminal::new(10, 3);
        for i in 0..10 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        t.scroll(3);
        assert_eq!(t.display_offset(), 3);
        t.scroll_to_bottom();
        assert!(t.resize(20, 5));
        assert!(!t.resize(20, 5));
        assert_eq!(t.size(), Size { cols: 20, rows: 5 });
    }

    #[test]
    fn selection_copies_text() {
        let mut t = Terminal::new(20, 2);
        t.feed(b"copy this text");
        t.start_selection(5, 0, SelectionType::Simple);
        t.extend_selection(8, 0);
        assert_eq!(t.selection_text().as_deref(), Some("this"));
    }

    #[test]
    fn titles_and_osc52() {
        let t0 = Terminal::new(10, 2);
        drop(t0);
        let mut t = Terminal::new(10, 2);
        t.feed(b"\x1b]0;my title\x07");
        assert_eq!(t.title().as_deref(), Some("my title"));
        t.feed(b"\x1b]52;c;aGVsbG8=\x07");
        assert_eq!(t.take_clipboard().as_deref(), Some("hello"));
    }
}
