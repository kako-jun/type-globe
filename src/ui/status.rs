//! Mode-aware status pane.
//!
//! Renders the right-hand side column for every game mode. The component is
//! a value-type built from current game state every frame, so updates flow
//! naturally — the renderer just rebuilds a `StatusPane` from the live
//! `QuizGame` (or future `HackGame`) and calls `render`.
//!
//! Per `docs/spec.md`:
//! - Quiz modes show **Score / Time / CPM / WPM**.
//! - Hack-and-slash shows **Lv / EXP / HP / Floor**.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use std::time::Duration;

const STYLE_LABEL: Style = Style::new().fg(Color::DarkGray);
const STYLE_VALUE: Style = Style::new().fg(Color::Cyan);
const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

/// A simple progress bar specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressBar {
    pub label: String,
    pub current: u32,
    pub max: u32,
}

impl ProgressBar {
    pub fn ratio(&self) -> f64 {
        if self.max == 0 {
            0.0
        } else {
            (f64::from(self.current) / f64::from(self.max)).clamp(0.0, 1.0)
        }
    }
}

/// One row in the status pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusItem {
    Value {
        label: String,
        value: String,
    },
    /// Used by hack-and-slash; quiz mode never produces bars.
    /// TODO(#11): drop this allow when hack UI lands.
    #[allow(dead_code)]
    Bar(ProgressBar),
}

impl StatusItem {
    pub fn value(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Value {
            label: label.into(),
            value: value.into(),
        }
    }

    /// TODO(#11): drop this `allow(dead_code)` once the hack UI wires it up.
    #[allow(dead_code)]
    pub fn bar(label: impl Into<String>, current: u32, max: u32) -> Self {
        Self::Bar(ProgressBar {
            label: label.into(),
            current,
            max,
        })
    }

    pub fn height(&self) -> u16 {
        match self {
            StatusItem::Value { .. } => 1,
            StatusItem::Bar(_) => 3,
        }
    }
}

/// The full status pane for a single frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusPane {
    pub title: String,
    pub items: Vec<StatusItem>,
}

impl StatusPane {
    pub fn new(title: impl Into<String>, items: Vec<StatusItem>) -> Self {
        Self {
            title: title.into(),
            items,
        }
    }

    /// Quiz status: Score / Time / CPM / WPM (per `docs/spec.md`).
    pub fn quiz(score: u32, elapsed: Duration, cpm: u32, wpm: u32) -> Self {
        Self::new(
            "Stats",
            vec![
                StatusItem::value("Score", format_score(score)),
                StatusItem::value("Time", format_time(elapsed)),
                StatusItem::value("CPM", cpm.to_string()),
                StatusItem::value("WPM", wpm.to_string()),
            ],
        )
    }

    /// Hack-and-slash status: Lv / EXP / HP / Floor / Run time
    /// (per `docs/spec.md`).
    ///
    /// TODO(#11): drop this `allow(dead_code)` once the hack UI wires it up.
    #[allow(dead_code)]
    pub fn hack(
        level: u32,
        exp: ProgressBar,
        hp: ProgressBar,
        floor: u32,
        floor_max: u32,
        run_time: Duration,
    ) -> Self {
        Self::new(
            "Run",
            vec![
                StatusItem::value("Lv.", level.to_string()),
                StatusItem::Bar(exp),
                StatusItem::Bar(hp),
                StatusItem::value("Floor", format!("{floor}/{floor_max}")),
                StatusItem::value("Run time", format_time(run_time)),
            ],
        )
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(self.title.clone(), STYLE_TITLE))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.items.is_empty() || inner.height == 0 {
            return;
        }

        // Trailing Min(0) absorbs leftover height so items keep their natural
        // size instead of being stretched to fill the pane.
        let constraints: Vec<Constraint> = self
            .items
            .iter()
            .map(|item| Constraint::Length(item.height()))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (item, area) in self.items.iter().zip(chunks.iter()) {
            render_item(f, *area, item);
        }
    }
}

fn render_item(f: &mut Frame, area: Rect, item: &StatusItem) {
    if area.height == 0 {
        return;
    }
    match item {
        StatusItem::Value { label, value } => {
            let line = Line::from(vec![
                Span::styled(format!("{label} "), STYLE_LABEL),
                Span::styled(value.clone(), STYLE_VALUE),
            ]);
            f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
        }
        StatusItem::Bar(bar) => {
            let label = format!("{} {}/{}", bar.label, bar.current, bar.max);
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Blue))
                .ratio(bar.ratio())
                .label(label);
            f.render_widget(gauge, area);
        }
    }
}

fn format_time(d: Duration) -> String {
    let total = d.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

fn format_score(score: u32) -> String {
    let s = score.to_string();
    let bytes = s.as_bytes();
    let comma_count = bytes.len().saturating_sub(1) / 3;
    let mut out = String::with_capacity(s.len() + comma_count);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render(pane: &StatusPane, width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| pane.render(f, Rect::new(0, 0, width, height)))
            .unwrap();
        terminal
    }

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
    fn quiz_pane_has_four_value_items() {
        let pane = StatusPane::quiz(12_340, Duration::from_secs(42), 230, 46);
        assert_eq!(pane.items.len(), 4);
        assert!(pane
            .items
            .iter()
            .all(|i| matches!(i, StatusItem::Value { .. })));
    }

    #[test]
    fn hack_pane_mixes_values_and_bars() {
        let pane = StatusPane::hack(
            5,
            ProgressBar {
                label: "EXP".into(),
                current: 60,
                max: 100,
            },
            ProgressBar {
                label: "HP".into(),
                current: 80,
                max: 100,
            },
            3,
            10,
            Duration::from_secs(80),
        );
        assert_eq!(pane.items.len(), 5);
        assert_eq!(
            pane.items
                .iter()
                .filter(|i| matches!(i, StatusItem::Bar(_)))
                .count(),
            2
        );
    }

    #[test]
    fn status_item_bar_constructor_matches_struct_form() {
        assert_eq!(
            StatusItem::bar("HP", 80, 100),
            StatusItem::Bar(ProgressBar {
                label: "HP".into(),
                current: 80,
                max: 100,
            })
        );
    }

    #[test]
    fn quiz_pane_renders_score_and_time() {
        let pane = StatusPane::quiz(12_340, Duration::from_secs(42), 230, 46);
        let terminal = render(&pane, 24, 12);
        let out = dump(&terminal);
        assert!(out.contains("Score"));
        assert!(out.contains("12,340"));
        assert!(out.contains("0:42"));
        assert!(out.contains("CPM"));
        assert!(out.contains("230"));
        assert!(out.contains("WPM"));
    }

    #[test]
    fn hack_pane_renders_lv_and_floor() {
        let pane = StatusPane::hack(
            5,
            ProgressBar {
                label: "EXP".into(),
                current: 60,
                max: 100,
            },
            ProgressBar {
                label: "HP".into(),
                current: 80,
                max: 100,
            },
            3,
            10,
            Duration::from_secs(80),
        );
        let terminal = render(&pane, 24, 16);
        let out = dump(&terminal);
        assert!(out.contains("Lv."));
        assert!(out.contains("Floor"));
        assert!(out.contains("3/10"));
        assert!(out.contains("Run time"));
        assert!(out.contains("1:20"));
    }

    #[test]
    fn progress_bar_ratio_is_clamped() {
        assert!(
            (ProgressBar {
                label: "X".into(),
                current: 0,
                max: 0,
            }
            .ratio()
                - 0.0)
                .abs()
                < f64::EPSILON
        );
        assert!(
            (ProgressBar {
                label: "X".into(),
                current: 200,
                max: 100,
            }
            .ratio()
                - 1.0)
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn empty_pane_does_not_panic() {
        let pane = StatusPane::default();
        let _ = render(&pane, 24, 6);
    }

    #[test]
    fn very_small_area_does_not_panic() {
        let pane = StatusPane::quiz(0, Duration::from_secs(0), 0, 0);
        let _ = render(&pane, 8, 2);
    }

    #[test]
    fn format_score_groups_thousands() {
        assert_eq!(format_score(0), "0");
        assert_eq!(format_score(7), "7");
        assert_eq!(format_score(123), "123");
        assert_eq!(format_score(1234), "1,234");
        assert_eq!(format_score(12_340), "12,340");
        assert_eq!(format_score(1_234_567), "1,234,567");
    }

    #[test]
    fn format_time_renders_mmss() {
        assert_eq!(format_time(Duration::from_secs(0)), "0:00");
        assert_eq!(format_time(Duration::from_secs(42)), "0:42");
        assert_eq!(format_time(Duration::from_secs(125)), "2:05");
    }
}
