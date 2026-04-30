use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneFrame {
    pub main: Rect,
    pub side: Rect,
    pub bottom: Rect,
    pub extra: Option<Rect>,
}

impl PaneFrame {
    pub fn quiz(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(10), Constraint::Length(4)])
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(24), Constraint::Length(24)])
            .split(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            bottom: outer[1],
            extra: None,
        }
    }

    #[allow(dead_code)]
    pub fn hack(area: Rect) -> Self {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(12),
                Constraint::Length(4),
                Constraint::Length(5),
            ])
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(24), Constraint::Length(24)])
            .split(outer[0]);

        Self {
            main: top[0],
            side: top[1],
            bottom: outer[1],
            extra: Some(outer[2]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PaneFrame;
    use ratatui::layout::Rect;

    #[test]
    fn quiz_layout_has_three_panes() {
        let frame = PaneFrame::quiz(Rect::new(0, 0, 100, 30));

        assert!(frame.main.width > frame.side.width);
        assert_eq!(frame.bottom.height, 4);
        assert!(frame.extra.is_none());
    }

    #[test]
    fn hack_layout_has_four_panes() {
        let frame = PaneFrame::hack(Rect::new(0, 0, 100, 30));

        assert!(frame.main.width > frame.side.width);
        assert_eq!(frame.bottom.height, 4);
        assert_eq!(frame.extra.expect("hack extra pane").height, 5);
    }
}
