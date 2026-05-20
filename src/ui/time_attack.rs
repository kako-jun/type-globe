use crate::game::{Ta25LocalGame, Ta25SeatColor};
use crate::ui::{
    HelpEntry, HelpLine, InputChannel, KeyEventSource, PaneFrame, RecvOutcome, StatusItem,
    StatusPane,
};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_REJECTED: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const INPUT_REJECT_FLASH_MS: u64 = 180;
const REDRAW_TICK: Duration = Duration::from_millis(50);

pub struct TimeAttack25UI {
    game: Ta25LocalGame,
    input_buffer: String,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
}

impl TimeAttack25UI {
    pub fn new(game: Ta25LocalGame) -> Self {
        Self {
            game,
            input_buffer: String::new(),
            rejected_char: None,
            reject_flash_until: None,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let input = InputChannel::spawn();
        let result = self.run_app(&mut terminal, &input);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        input: &impl KeyEventSource,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.game.poll_cpu(Instant::now());
            terminal.draw(|f| self.ui(f))?;

            match input.recv_until(REDRAW_TICK) {
                RecvOutcome::Key(key) => {
                    if self.handle_key(key) {
                        return Ok(());
                    }
                }
                RecvOutcome::Timeout => {}
                RecvOutcome::Disconnected => return Ok(()),
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Esc)
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return true;
        }

        if self.game.is_finished() {
            return matches!(key.code, KeyCode::Enter | KeyCode::Esc);
        }

        match key.code {
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Tab => {
                self.input_buffer.clear();
                self.game.forfeit_current_round(Instant::now());
            }
            KeyCode::Char(ch) => {
                self.push_input_char(ch);
            }
            _ => {}
        }

        false
    }

    fn push_input_char(&mut self, ch: char) {
        let mut candidate = self.input_buffer.clone();
        candidate.push(ch);
        if self.game.is_valid_human_prefix(&candidate) {
            self.input_buffer = candidate;
            self.clear_reject_flash();
            if self.game.is_complete_human_answer(&self.input_buffer) {
                let answer = self.input_buffer.clone();
                let _ = self.game.submit_human_answer(&answer, Instant::now());
                self.input_buffer.clear();
            }
            return;
        }

        self.rejected_char = Some(ch);
        self.reject_flash_until =
            Some(Instant::now() + Duration::from_millis(INPUT_REJECT_FLASH_MS));
    }

    fn clear_reject_flash(&mut self) {
        self.rejected_char = None;
        self.reject_flash_until = None;
    }

    fn input_style(&self) -> Style {
        if self
            .reject_flash_until
            .is_some_and(|until| Instant::now() < until)
        {
            STYLE_REJECTED
        } else {
            STYLE_NORMAL
        }
    }

    fn ui(&self, f: &mut Frame) {
        let area = f.area();
        let frame = PaneFrame::quiz(area);
        self.render_main(f, frame.main);
        self.render_side(f, frame.side);
        self.render_input_echo(f, frame.input_echo);
        self.render_help_line(f, frame.help_line);
    }

    fn render_main(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(11),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        if self.game.is_finished() {
            self.render_summary(f, chunks[0]);
        } else {
            self.render_question(f, chunks[0]);
        }
        self.render_board(f, chunks[1]);
        self.render_action_line(f, chunks[2]);
    }

    fn render_question(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                format!(
                    " {}/{}  Panel {:02} ",
                    self.game.current_question_number(),
                    self.game.total_questions(),
                    self.game.current_panel_number()
                ),
                STYLE_TITLE,
            )]),
            Line::from(""),
            Line::from(self.game.question_text()),
            Line::from(""),
        ];

        for (index, choice) in self.game.choice_texts().iter().enumerate() {
            let label = match index {
                0 => "A",
                1 => "B",
                2 => "C",
                3 => "D",
                _ => "?",
            };
            lines.push(Line::from(format!("{label}) {choice}")));
        }

        let block = Block::default()
            .title(Span::styled(" Current Question ", STYLE_TITLE))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_summary(&self, f: &mut Frame, area: Rect) {
        let counts = self.game.counts_by_color();
        let mut lines = vec![
            Line::from(vec![Span::styled(" 25 panels resolved ", STYLE_TITLE)]),
            Line::from(""),
            Line::from(self.game.leader_summary()),
            Line::from(format!(
                "Final time: {}",
                format_time(self.game.elapsed(Instant::now()))
            )),
            Line::from(""),
        ];
        for (color, count) in counts {
            let seat_name = self
                .game
                .roster()
                .seat_for_color(color)
                .map(|seat| seat.display_name.as_str())
                .unwrap_or(color.as_str());
            lines.push(Line::from(vec![
                Span::styled(format!("{seat_name:<8} "), seat_color_style(color)),
                Span::styled(format!("{count:>2} panels"), STYLE_NORMAL),
            ]));
        }
        let block = Block::default()
            .title(Span::styled(" Summary ", STYLE_TITLE))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_board(&self, f: &mut Frame, area: Rect) {
        let mut lines = Vec::with_capacity(5);
        for row in 0..5 {
            let mut spans = Vec::with_capacity(5 * 3);
            for col in 0..5 {
                let index = row * 5 + col;
                if col > 0 {
                    spans.push(Span::raw(" "));
                }
                let cell = format!("{:02}", index + 1);
                let style = match self.game.board()[index] {
                    Some(color) => seat_color_style(color).add_modifier(Modifier::BOLD),
                    None if !self.game.is_finished()
                        && index + 1 == self.game.current_panel_number() =>
                    {
                        Style::new().fg(Color::Black).bg(Color::White)
                    }
                    None => STYLE_DIM,
                };
                spans.push(Span::styled(cell, style));
            }
            lines.push(Line::from(spans));
            lines.push(Line::from(""));
        }

        let block = Block::default()
            .title(Span::styled(" 5x5 Board ", STYLE_TITLE))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_action_line(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(Span::styled(" Pace ", STYLE_TITLE))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        f.render_widget(
            Paragraph::new(self.game.last_action())
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_side(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.game.elapsed(Instant::now());
        let counts = self.game.counts_by_color();
        let mut items = vec![
            StatusItem::value(
                "Round",
                format!(
                    "{}/{}",
                    self.game.current_question_number(),
                    self.game.total_questions()
                ),
            ),
            StatusItem::value("Time", format_time(elapsed)),
            StatusItem::value("Lead", self.game.leader_summary()),
        ];

        for (color, count) in counts {
            let seat_name = self
                .game
                .roster()
                .seat_for_color(color)
                .map(|seat| seat.display_name.as_str())
                .unwrap_or(color.as_str());
            items.push(StatusItem::value(seat_name, count.to_string()));
        }

        let pane = StatusPane::new("TA25", items);
        pane.render(f, area);
    }

    fn render_input_echo(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![Span::styled("> ", self.input_style())];
        if self.input_buffer.is_empty() {
            spans.push(Span::styled("type the correct answer", STYLE_DIM));
        } else {
            spans.push(Span::styled(self.input_buffer.clone(), self.input_style()));
        }
        if let Some(ch) = self.rejected_char {
            if self
                .reject_flash_until
                .is_some_and(|until| Instant::now() < until)
            {
                spans.push(Span::styled(format!("  x {ch}"), STYLE_REJECTED));
            }
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help = if self.game.is_finished() {
            HelpLine::new(vec![
                HelpEntry::new("Enter", "Menu"),
                HelpEntry::new("Esc", "Quit"),
            ])
        } else {
            HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Tab", "Forfeit"),
                HelpEntry::new("Backspace", "Edit"),
            ])
        };
        help.render(f, area);
    }
}

fn seat_color_style(color: Ta25SeatColor) -> Style {
    match color {
        Ta25SeatColor::Red => Style::new().fg(Color::Red),
        Ta25SeatColor::Blue => Style::new().fg(Color::Blue),
        Ta25SeatColor::Green => Style::new().fg(Color::Green),
        Ta25SeatColor::Yellow => Style::new().fg(Color::Yellow),
    }
}

fn format_time(d: Duration) -> String {
    let total = d.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Choice, Question};
    use std::collections::HashMap;

    fn sample_question(id: usize) -> Question {
        Question {
            id: format!("q-{id:02}"),
            genre: "test".into(),
            question_text: HashMap::from([("en".to_string(), format!("Question {id}"))]),
            question_text_reading: HashMap::new(),
            choices: vec![
                Choice {
                    labels: HashMap::from([("en".to_string(), "alpha".to_string())]),
                    ja_typings: Vec::new(),
                },
                Choice {
                    labels: HashMap::from([("en".to_string(), "bravo".to_string())]),
                    ja_typings: Vec::new(),
                },
            ],
            correct_answer_index: 0,
            image_path: None,
            ja_reviewed: true,
        }
    }

    fn make_ui() -> TimeAttack25UI {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let game =
            Ta25LocalGame::from_pool(&pool, crate::types::Language::English, "You").expect("game");
        TimeAttack25UI::new(game)
    }

    #[test]
    fn rejects_wrong_prefix_and_clears_buffer() {
        let mut ui = make_ui();
        ui.input_buffer = "alp".to_string();
        ui.push_input_char('z');
        assert_eq!(ui.input_buffer, "alp");
        assert_eq!(ui.rejected_char, Some('z'));
    }

    #[test]
    fn help_line_switches_on_finish() {
        let mut ui = make_ui();
        ui.game.forfeit_current_round(Instant::now());
        while !ui.game.is_finished() {
            ui.game.forfeit_current_round(Instant::now());
        }
        let backend = ratatui::backend::TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| ui.render_help_line(f, Rect::new(0, 0, 60, 1)))
            .unwrap();
        let mut out = String::new();
        let buf = terminal.backend().buffer();
        for x in 0..buf.area.width {
            out.push_str(buf[(x, 0)].symbol());
        }
        assert!(out.contains("[Enter]"));
    }
}
