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
//! ## Bottom stack
//!
//! Both quiz and rpg frames carry a 1-line `input_echo` (renders the
//! typed answer / sentence) immediately below the top panes, and an
//! always-on 1-line `help_line` at the very bottom rendered by the
//! `ui::HelpLine` component (#18). For rpg mode, a 5-line `log` slot
//! sits between the input echo and the help line.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Geometric breakdown of a mode's screen.
///
/// `main`, `side`, `input_echo`, and `help_line` are always present;
/// `log` is filled only by RPG runs (it carries the battle log
/// per `docs/spec.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneFrame {
    pub main: Rect,
    pub side: Rect,
    /// 1-line input echo (typed answer / sentence) per `docs/spec.md`.
    pub input_echo: Rect,
    /// 1-line always-on help line, rendered by `ui::HelpLine` (#18).
    pub help_line: Rect,
    /// Hack-and-slash battle log pane. `None` for quiz-style modes.
    pub log: Option<Rect>,
}

const SIDE_WIDTH: u16 = 24;
const MAIN_MIN_WIDTH: u16 = 40;
const INPUT_ECHO_HEIGHT: u16 = 1;
const HELP_LINE_HEIGHT: u16 = 1;
const HACK_LOG_HEIGHT: u16 = 5;

impl PaneFrame {
    /// 3-pane layout for quiz-style modes (Quiz, Time Attack 25, Records).
    pub fn quiz(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(INPUT_ECHO_HEIGHT),
                Constraint::Length(HELP_LINE_HEIGHT),
            ])
            .split(area);

        let top = split_top(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            input_echo: outer[1],
            help_line: outer[2],
            log: None,
        }
    }

    /// 4-pane layout for the listening RPG.
    ///
    /// TODO(#11): wire this up once the rpg UI lands; the `#[allow(dead_code)]`
    /// can come off then.
    #[allow(dead_code)]
    pub fn rpg(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(INPUT_ECHO_HEIGHT),
                Constraint::Length(HACK_LOG_HEIGHT),
                Constraint::Length(HELP_LINE_HEIGHT),
            ])
            .split(area);

        let top = split_top(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            input_echo: outer[1],
            log: Some(outer[2]),
            help_line: outer[3],
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
    use super::{PaneFrame, HACK_LOG_HEIGHT, HELP_LINE_HEIGHT, INPUT_ECHO_HEIGHT, SIDE_WIDTH};
    use ratatui::layout::Rect;

    const MARGIN: u16 = 1;

    #[test]
    fn quiz_layout_has_three_panes() {
        let area = Rect::new(0, 0, 100, 30);
        let frame = PaneFrame::quiz(area);

        let inner_width = area.width - MARGIN * 2;
        assert_eq!(frame.side.width, SIDE_WIDTH);
        assert_eq!(frame.main.width, inner_width - SIDE_WIDTH);
        assert_eq!(frame.input_echo.height, INPUT_ECHO_HEIGHT);
        assert_eq!(frame.input_echo.width, inner_width);
        assert_eq!(frame.help_line.height, HELP_LINE_HEIGHT);
        assert_eq!(frame.help_line.width, inner_width);
        assert!(frame.input_echo.y < frame.help_line.y);
        assert!(frame.log.is_none());
    }

    #[test]
    fn rpg_layout_has_four_panes() {
        let area = Rect::new(0, 0, 100, 30);
        let frame = PaneFrame::rpg(area);

        let inner_width = area.width - MARGIN * 2;
        assert_eq!(frame.side.width, SIDE_WIDTH);
        assert_eq!(frame.main.width, inner_width - SIDE_WIDTH);
        assert_eq!(frame.input_echo.height, INPUT_ECHO_HEIGHT);
        assert_eq!(frame.help_line.height, HELP_LINE_HEIGHT);
        let log = frame.log.expect("rpg log pane");
        assert_eq!(log.height, HACK_LOG_HEIGHT);
        assert_eq!(log.width, inner_width);
        assert!(frame.input_echo.y < log.y);
        assert!(log.y < frame.help_line.y);
    }

    #[test]
    fn quiz_layout_does_not_panic_on_small_terminal() {
        let frame = PaneFrame::quiz(Rect::new(0, 0, 60, 20));
        assert!(frame.main.width >= 1);
        assert_eq!(frame.help_line.height, HELP_LINE_HEIGHT);
    }

    #[test]
    fn rpg_layout_does_not_panic_on_small_terminal() {
        let frame = PaneFrame::rpg(Rect::new(0, 0, 60, 24));
        assert!(frame.main.width >= 1);
        assert!(frame.log.is_some());
    }
}
