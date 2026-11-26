//! Bounded log console shared by all background tasks.

use std::collections::VecDeque;

pub const MAX_LINES: usize = 5_000;

#[derive(Default)]
pub struct Console {
    lines: VecDeque<String>,
}

/// Remove ANSI escape sequences (colours from cargo / probe-rs) that egui would
/// otherwise render as boxes.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            // CSI: parameters/intermediates until a final byte in '@'..='~'.
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }
    out
}

impl Console {
    pub fn push(&mut self, line: impl Into<String>) {
        if self.lines.len() >= MAX_LINES {
            self.lines.pop_front();
        }
        let line = line.into();
        let line = if line.contains('\x1b') {
            strip_ansi(&line)
        } else {
            line
        };
        self.lines.push_back(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_colour_codes() {
        let mut console = Console::default();
        console.push("\x1b[33m WARN\x1b[0m \x1b[2mprobe_rs::probe\x1b[0m: hi");
        assert_eq!(console.lines().next(), Some(" WARN probe_rs::probe: hi"));
    }

    #[test]
    fn keeps_only_the_most_recent_lines() {
        let mut console = Console::default();
        for i in 0..MAX_LINES + 10 {
            console.push(format!("line {i}"));
        }
        assert_eq!(console.len(), MAX_LINES);
        assert_eq!(console.lines().next(), Some("line 10"));
        console.clear();
        assert_eq!(console.len(), 0);
    }
}
