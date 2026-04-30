//! Always-on help line.
//!
//! Renders a single line of bracketed key hints at the very bottom of the
//! screen, e.g. `[Esc] Quit  [Tab] Skip  [F5] Restart` — matching the format
//! used in `docs/spec.md`.
//!
//! The component is a value type rebuilt every frame from the current mode /
//! state, so help-line content can change as the UI transitions (e.g. quiz
//! "answering" → "result" → "finished").

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const STYLE_KEY: Style = Style::new().fg(Color::Yellow);
const STYLE_ACTION: Style = Style::new().fg(Color::Gray);
const STYLE_SEP: Style = Style::new().fg(Color::DarkGray);

/// One key-action pair shown in the help line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpEntry {
    pub key: &'static str,
    pub action: &'static str,
}

impl HelpEntry {
    pub const fn new(key: &'static str, action: &'static str) -> Self {
        Self { key, action }
    }
}

/// A frame-local help-line definition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HelpLine {
    pub entries: Vec<HelpEntry>,
}

impl HelpLine {
    pub fn new(entries: Vec<HelpEntry>) -> Self {
        Self { entries }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 || self.entries.is_empty() {
            return;
        }
        let mut spans: Vec<Span<'_>> = Vec::with_capacity(self.entries.len() * 4);
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", STYLE_SEP));
            }
            spans.push(Span::styled(format!("[{}]", entry.key), STYLE_KEY));
            spans.push(Span::styled(format!(" {}", entry.action), STYLE_ACTION));
        }
        let paragraph = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }

    /// Plain text form, useful for tests and logs.
    #[cfg(test)]
    pub fn as_text(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("[{}] {}", e.key, e.action))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn dump(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn as_text_matches_spec_format() {
        let help = HelpLine::new(vec![
            HelpEntry::new("Esc", "Quit"),
            HelpEntry::new("Tab", "Skip"),
            HelpEntry::new("F5", "Restart"),
        ]);
        assert_eq!(help.as_text(), "[Esc] Quit  [Tab] Skip  [F5] Restart");
    }

    #[test]
    fn renders_each_entry_to_buffer() {
        let help = HelpLine::new(vec![
            HelpEntry::new("Esc", "Quit"),
            HelpEntry::new("Tab", "Skip"),
        ]);
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| help.render(f, Rect::new(0, 0, 60, 1)))
            .unwrap();
        let out = dump(&terminal);
        assert!(out.contains("[Esc]"));
        assert!(out.contains("Quit"));
        assert!(out.contains("[Tab]"));
        assert!(out.contains("Skip"));
    }

    #[test]
    fn empty_help_line_does_not_panic() {
        let help = HelpLine::default();
        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| help.render(f, Rect::new(0, 0, 40, 1)))
            .unwrap();
    }

    #[test]
    fn zero_height_area_does_not_panic() {
        let help = HelpLine::new(vec![HelpEntry::new("Esc", "Quit")]);
        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| help.render(f, Rect::new(0, 0, 40, 0)))
            .unwrap();
    }
}
