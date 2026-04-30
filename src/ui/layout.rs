//! Shared pane layout foundation for the v0.2.0 redesign.
//!
//! `PaneFrame` is a UI-library-agnostic geometry helper that splits a screen
//! `Rect` into the canonical regions used by every game mode. Renderers consume
//! the resulting `Rect`s and decide what widgets to draw inside each pane.
//!
//! ## v0.2.0 target structure (per `docs/spec.md`)
//!
//! - **Quiz** (3 panes): `main` (Q + choices) / `side` (Score / Time / CPM /
//!   WPM) / `input_echo` (typed-selection echo) + always-on `help_line`.
//! - **Hack-and-slash** (4 panes): `main` (listening + enemy) / `side`
//!   (Lv / EXP / HP / Floor) / `input_echo` / `log` (battle log) +
//!   always-on `help_line`.
//!
//! ## Current transitional layout
//!
//! Until #24 (typed selection) lands, the quiz still uses the legacy arrow /
//! number-key selection model, so no `input_echo` slot is allocated — the
//! `help_line` block currently absorbs the bottom region. When #18 (help
//! line component) and #24 ship, the bottom region will be split into a
//! 1-line input echo and a 1-line always-on help line.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Geometric breakdown of a mode's screen.
///
/// `main` and `side` are always present; `log` is filled only by hack-and-slash
/// runs (it carries the battle log per `docs/spec.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneFrame {
    pub main: Rect,
    pub side: Rect,
    /// Bottom region. Currently holds the help text directly; once #18 / #24
    /// land this will be subdivided into `input_echo` + 1-line help_line.
    pub help_line: Rect,
    /// Hack-and-slash battle log pane. `None` for quiz-style modes.
    pub log: Option<Rect>,
}

const SIDE_WIDTH: u16 = 24;
const MAIN_MIN_WIDTH: u16 = 40;
const QUIZ_BOTTOM_HEIGHT: u16 = 4;
const HACK_BOTTOM_HEIGHT: u16 = 4;
const HACK_LOG_HEIGHT: u16 = 5;

impl PaneFrame {
    /// 3-pane layout for quiz-style modes (Quiz, Time Attack 25, Records).
    pub fn quiz(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(10), Constraint::Length(QUIZ_BOTTOM_HEIGHT)])
            .split(area);

        let top = split_top(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            help_line: outer[1],
            log: None,
        }
    }

    /// 4-pane layout for the hack-and-slash listening RPG.
    ///
    /// TODO(#11): wire this up once the hack UI lands; the `#[allow(dead_code)]`
    /// can come off then.
    #[allow(dead_code)]
    pub fn hack(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(HACK_BOTTOM_HEIGHT),
                Constraint::Length(HACK_LOG_HEIGHT),
            ])
            .split(area);

        let top = split_top(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            help_line: outer[1],
            log: Some(outer[2]),
        }
    }
}

fn split_top(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(MAIN_MIN_WIDTH),
            Constraint::Length(SIDE_WIDTH),
        ])
        .split(area)
}

#[cfg(test)]
mod tests {
    use super::{PaneFrame, HACK_BOTTOM_HEIGHT, HACK_LOG_HEIGHT, QUIZ_BOTTOM_HEIGHT, SIDE_WIDTH};
    use ratatui::layout::Rect;

    const MARGIN: u16 = 1;

    #[test]
    fn quiz_layout_has_three_panes() {
        let area = Rect::new(0, 0, 100, 30);
        let frame = PaneFrame::quiz(area);

        let inner_width = area.width - MARGIN * 2;
        assert_eq!(frame.side.width, SIDE_WIDTH);
        assert_eq!(frame.main.width, inner_width - SIDE_WIDTH);
        assert_eq!(frame.help_line.height, QUIZ_BOTTOM_HEIGHT);
        assert_eq!(frame.help_line.width, inner_width);
        assert!(frame.log.is_none());
    }

    #[test]
    fn hack_layout_has_four_panes() {
        let area = Rect::new(0, 0, 100, 30);
        let frame = PaneFrame::hack(area);

        let inner_width = area.width - MARGIN * 2;
        assert_eq!(frame.side.width, SIDE_WIDTH);
        assert_eq!(frame.main.width, inner_width - SIDE_WIDTH);
        assert_eq!(frame.help_line.height, HACK_BOTTOM_HEIGHT);
        let log = frame.log.expect("hack log pane");
        assert_eq!(log.height, HACK_LOG_HEIGHT);
        assert_eq!(log.width, inner_width);
    }

    #[test]
    fn quiz_layout_does_not_panic_on_small_terminal() {
        let frame = PaneFrame::quiz(Rect::new(0, 0, 60, 20));
        assert!(frame.main.width >= 1);
        assert_eq!(frame.help_line.height, QUIZ_BOTTOM_HEIGHT);
    }

    #[test]
    fn hack_layout_does_not_panic_on_small_terminal() {
        let frame = PaneFrame::hack(Rect::new(0, 0, 60, 24));
        assert!(frame.main.width >= 1);
        assert!(frame.log.is_some());
    }
}
