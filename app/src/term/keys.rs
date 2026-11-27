//! Keys and mouse to the bytes a remote program expects (xterm encoding).

use alacritty_terminal::term::TermMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    /// xterm's modifier parameter (1 = none).
    fn param(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl)
    }
    fn any(self) -> bool {
        self.ctrl || self.alt || self.shift
    }
}

/// The keys that are not text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
    /// A printable character pressed with modifiers (Ctrl+C, Alt+x).
    Char(char),
}

/// Encodes [key] with [mods] under the terminal's current [mode].
pub fn encode(key: Key, mods: Mods, mode: TermMode) -> Option<Vec<u8>> {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let csi_mod = |final_: char| -> Vec<u8> {
        if mods.any() { format!("\x1b[1;{}{final_}", mods.param()).into_bytes() } else if app_cursor { format!("\x1bO{final_}").into_bytes() } else { format!("\x1b[{final_}").into_bytes() }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if mods.any() { format!("\x1b[{n};{}~", mods.param()).into_bytes() } else { format!("\x1b[{n}~").into_bytes() }
    };
    let alt = |mut b: Vec<u8>| -> Vec<u8> {
        if mods.alt {
            b.insert(0, 0x1b);
        }
        b
    };
    Some(match key {
        Key::Enter => alt(vec![b'\r']),
        Key::Backspace => alt(vec![if mods.ctrl { 0x08 } else { 0x7f }]),
        Key::Tab => {
            if mods.shift {
                b"\x1b[Z".to_vec()
            } else {
                alt(vec![b'\t'])
            }
        }
        Key::Escape => alt(vec![0x1b]),
        Key::Up => csi_mod('A'),
        Key::Down => csi_mod('B'),
        Key::Right => csi_mod('C'),
        Key::Left => csi_mod('D'),
        Key::Home => csi_mod('H'),
        Key::End => csi_mod('F'),
        Key::Insert => tilde(2),
        Key::Delete => tilde(3),
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F(n @ 1..=4) => {
            let f = b"PQRS"[usize::from(n - 1)] as char;
            if mods.any() { format!("\x1b[1;{}{f}", mods.param()).into_bytes() } else { format!("\x1bO{f}").into_bytes() }
        }
        Key::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)]),
        Key::F(_) => return None,
        Key::Char(c) => {
            let mut b = if mods.ctrl { ctrl_byte(c)?.to_le_bytes()[..1].to_vec() } else { c.to_string().into_bytes() };
            if mods.alt {
                b.insert(0, 0x1b);
            }
            b
        }
    })
}

/// Ctrl+[c] as a C0 byte, where there is one.
pub fn ctrl_byte(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        '@' | ' ' | '2' => Some(0),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' | '7' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

/// Text from the keyboard or IME, with sticky modifiers applied.
pub fn encode_text(text: &str, mods: Mods) -> Vec<u8> {
    if mods.ctrl {
        let mut out = Vec::new();
        for c in text.chars() {
            match ctrl_byte(c) {
                Some(b) => out.push(b),
                None => out.extend(c.to_string().as_bytes()),
            }
        }
        if mods.alt {
            out.insert(0, 0x1b);
        }
        return out;
    }
    let mut out = text.as_bytes().to_vec();
    if mods.alt {
        out.insert(0, 0x1b);
    }
    out
}

/// A paste, bracketed when the program asked for it, with the bracket's
/// end sequence stripped from the text so it cannot end the paste early.
pub fn encode_paste(text: &str, mode: TermMode) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if mode.contains(TermMode::BRACKETED_PASTE) {
        let clean = text.replace("\x1b[201~", "");
        format!("\x1b[200~{clean}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// A mouse report, when the program asked for one (SGR or X10 encoding).
pub fn encode_mouse(button: MouseButton, pressed: bool, col: usize, row: usize, mode: TermMode) -> Option<Vec<u8>> {
    if !mode.intersects(TermMode::MOUSE_MODE) {
        return None;
    }
    let code: u8 = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };
    let (x, y) = (col + 1, row + 1);
    if mode.contains(TermMode::SGR_MOUSE) {
        Some(format!("\x1b[<{code};{x};{y}{}", if pressed { 'M' } else { 'm' }).into_bytes())
    } else {
        if x > 223 || y > 223 {
            return None;
        }
        let b = if pressed { code } else { 3 };
        Some(vec![0x1b, b'[', b'M', 32 + b, 32 + x as u8, 32 + y as u8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_keys_follow_the_application_mode() {
        let none = Mods::default();
        assert_eq!(encode(Key::Up, none, TermMode::empty()).unwrap(), b"\x1b[A");
        assert_eq!(encode(Key::Up, none, TermMode::APP_CURSOR).unwrap(), b"\x1bOA");
        assert_eq!(encode(Key::Right, Mods { ctrl: true, ..none }, TermMode::empty()).unwrap(), b"\x1b[1;5C");
    }

    #[test]
    fn control_and_alt_chords() {
        assert_eq!(encode(Key::Char('c'), Mods { ctrl: true, ..Default::default() }, TermMode::empty()).unwrap(), [3]);
        assert_eq!(encode(Key::Char('x'), Mods { alt: true, ..Default::default() }, TermMode::empty()).unwrap(), b"\x1bx");
        assert_eq!(encode_text("d", Mods { ctrl: true, ..Default::default() }), [4]);
        assert_eq!(encode(Key::Backspace, Mods::default(), TermMode::empty()).unwrap(), [0x7f]);
        assert_eq!(encode(Key::F(5), Mods::default(), TermMode::empty()).unwrap(), b"\x1b[15~");
        assert_eq!(encode(Key::F(1), Mods::default(), TermMode::empty()).unwrap(), b"\x1bOP");
    }

    #[test]
    fn pastes_are_bracketed_and_sanitized() {
        assert_eq!(encode_paste("a\nb", TermMode::empty()), b"a\rb");
        assert_eq!(encode_paste("x\x1b[201~y", TermMode::BRACKETED_PASTE), b"\x1b[200~xy\x1b[201~");
    }

    #[test]
    fn mouse_reports_only_when_asked() {
        assert!(encode_mouse(MouseButton::Left, true, 0, 0, TermMode::empty()).is_none());
        let m = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        assert_eq!(encode_mouse(MouseButton::Left, true, 4, 1, m).unwrap(), b"\x1b[<0;5;2M");
        assert_eq!(encode_mouse(MouseButton::WheelDown, true, 0, 0, m).unwrap(), b"\x1b[<65;1;1M");
    }
}
