//! Records browse screen (Issue #40).
//!
//! Loads `records_<lang>.json` and renders the local self-best list across
//! Quiz, Time Attack 25, and Listening RPG sections. Strictly read-only —
//! writing happens at the end of a Quiz run via `QuizUI::persist_record`.
//!
//! Per the kako-jun rule pinned in `docs/spec.md`, this screen displays
//! **Records** (offline self-bests). World ordering — "Ranking" — is
//! reserved for the v0.3.0+ Nostralgic Ranking integration in
//! `type-globe-online` and is not surfaced here.

use crate::io::Storage;
use crate::types::{Records, ScoreEntry, TimeEntry};
use crate::ui::{HelpEntry, HelpLine};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_SECTION: Style = Style::new().fg(Color::Cyan);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_HIGHLIGHT: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);

pub struct RecordsUI {
    records: Records,
    /// `ts` (epoch seconds) of the latest entry in each section. Each
    /// section's row carrying that ts is highlighted, so the user can
    /// spot "the run I just saved" without scrolling.
    latest_quiz_ts: Option<u64>,
    latest_ta25_ts: Option<u64>,
    latest_hack_ts: Option<u64>,
}

impl RecordsUI {
    /// Load records from disk for the given file path. A missing file
    /// produces an empty Records — the screen still renders (it just
    /// shows "(no records yet)" under each section).
    pub fn load(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let records = Storage::load_records(file_path)?;
        Ok(Self::from_records(records))
    }

    fn from_records(records: Records) -> Self {
        let latest_quiz_ts = records.quiz_mode.iter().map(|e| e.ts).max();
        let latest_ta25_ts = records.time_attack_25.iter().map(|e| e.ts).max();
        let latest_hack_ts = records.hack_and_slash_rpg.iter().map(|e| e.ts).max();
        Self {
            records,
            latest_quiz_ts,
            latest_ta25_ts,
            latest_hack_ts,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Same poll cadence as Quiz so terminal-resize redraws stay snappy.
        const TICK: Duration = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(TICK)? {
                if let Event::Key(key) = event::read()? {
                    if Self::is_quit(key) {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn is_quit(key: KeyEvent) -> bool {
        matches!(key.code, KeyCode::Esc | KeyCode::Enter)
            || (matches!(key.code, KeyCode::Char('c'))
                && key.modifiers.contains(KeyModifiers::CONTROL))
            || matches!(key.code, KeyCode::Char('q'))
    }

    fn ui(&self, f: &mut Frame) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.render_title(f, outer[0]);
        self.render_sections(f, outer[1]);
        self.help_line().render(f, outer[2]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("type-globe - Records")
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_sections(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .split(area);

        self.render_score_section(
            f,
            chunks[0],
            "Quiz (single-run)",
            &self.records.quiz_mode,
            self.latest_quiz_ts,
        );
        self.render_time_section(
            f,
            chunks[1],
            "Time Attack 25",
            &self.records.time_attack_25,
            self.latest_ta25_ts,
        );
        self.render_score_section(
            f,
            chunks[2],
            "Listening RPG",
            &self.records.hack_and_slash_rpg,
            self.latest_hack_ts,
        );
    }

    fn render_score_section(
        &self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        entries: &[ScoreEntry],
        highlight_ts: Option<u64>,
    ) {
        let lines = if entries.is_empty() {
            vec![Line::from(Span::styled("  (no records yet)", STYLE_DIM))]
        } else {
            entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let style = if Some(e.ts) == highlight_ts {
                        STYLE_HIGHLIGHT
                    } else {
                        STYLE_NORMAL
                    };
                    Line::from(Span::styled(
                        format!(
                            "  {rank:>2}. {name:<16}  Score {score:>6}   CPM {cpm:>4}   WPM {wpm:>3}",
                            rank = i + 1,
                            name = truncate_padded(&e.name, 16),
                            score = e.score,
                            cpm = e.cpm,
                            wpm = e.wpm,
                        ),
                        style,
                    ))
                })
                .collect()
        };

        let body = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(Span::styled(format!(" {title} "), STYLE_SECTION))
                .borders(Borders::ALL),
        );
        f.render_widget(body, area);
    }

    fn render_time_section(
        &self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        entries: &[TimeEntry],
        highlight_ts: Option<u64>,
    ) {
        let lines = if entries.is_empty() {
            vec![Line::from(Span::styled("  (no records yet)", STYLE_DIM))]
        } else {
            entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let style = if Some(e.ts) == highlight_ts {
                        STYLE_HIGHLIGHT
                    } else {
                        STYLE_NORMAL
                    };
                    let mins = e.time_seconds / 60;
                    let secs = e.time_seconds % 60;
                    Line::from(Span::styled(
                        format!(
                            "  {rank:>2}. {name:<16}  Time {mins}:{secs:02}",
                            rank = i + 1,
                            name = truncate_padded(&e.name, 16),
                        ),
                        style,
                    ))
                })
                .collect()
        };

        let body = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(Span::styled(format!(" {title} "), STYLE_SECTION))
                .borders(Borders::ALL),
        );
        f.render_widget(body, area);
    }

    fn help_line(&self) -> HelpLine {
        HelpLine::new(vec![
            HelpEntry::new("Esc", "Menu"),
            HelpEntry::new("Enter", "Menu"),
            HelpEntry::new("q", "Menu"),
        ])
    }
}

/// Pad / truncate `s` so the displayed width is exactly `width` characters
/// (UTF-8 char count, not bytes — wide-character widths are still rough,
/// but the records list does not promise pixel-perfect alignment).
fn truncate_padded(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if count == width {
        s.to_string()
    } else if count > width {
        s.chars().take(width).collect()
    } else {
        let mut out = s.to_string();
        for _ in count..width {
            out.push(' ');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, score: u32, ts: u64) -> ScoreEntry {
        ScoreEntry {
            name: name.into(),
            score,
            cpm: 0,
            wpm: 0,
            ts,
        }
    }

    #[test]
    fn latest_ts_is_max_per_section() {
        let mut records = Records::default();
        records.quiz_mode.push(entry("a", 100, 10));
        records.quiz_mode.push(entry("b", 200, 30));
        records.quiz_mode.push(entry("c", 150, 20));
        records.hack_and_slash_rpg.push(entry("z", 50, 5));

        let ui = RecordsUI::from_records(records);
        assert_eq!(ui.latest_quiz_ts, Some(30));
        assert_eq!(ui.latest_hack_ts, Some(5));
        assert_eq!(ui.latest_ta25_ts, None);
    }

    #[test]
    fn latest_ts_is_none_for_empty_records() {
        let ui = RecordsUI::from_records(Records::default());
        assert_eq!(ui.latest_quiz_ts, None);
        assert_eq!(ui.latest_ta25_ts, None);
        assert_eq!(ui.latest_hack_ts, None);
    }

    #[test]
    fn truncate_padded_pads_short_strings() {
        assert_eq!(truncate_padded("ab", 5), "ab   ");
    }

    #[test]
    fn truncate_padded_truncates_long_strings() {
        assert_eq!(truncate_padded("abcdef", 4), "abcd");
    }

    #[test]
    fn truncate_padded_handles_multibyte_chars() {
        // 3 chars (ten-letter ja) vs 5 chars width → 2 trailing spaces.
        assert_eq!(truncate_padded("あいう", 5), "あいう  ");
    }

    #[test]
    fn is_quit_recognises_expected_keys() {
        let mk = |code: KeyCode, mods: KeyModifiers| KeyEvent::new(code, mods);
        assert!(RecordsUI::is_quit(mk(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(RecordsUI::is_quit(mk(KeyCode::Enter, KeyModifiers::NONE)));
        assert!(RecordsUI::is_quit(mk(
            KeyCode::Char('q'),
            KeyModifiers::NONE
        )));
        assert!(RecordsUI::is_quit(mk(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
        assert!(!RecordsUI::is_quit(mk(
            KeyCode::Char('a'),
            KeyModifiers::NONE
        )));
    }
}
