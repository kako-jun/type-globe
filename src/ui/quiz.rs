use crate::game::{QuizGame, QuizResult};
use crate::types::{Language, Question};
use crate::ui::{HelpEntry, HelpLine, PaneFrame, StatusPane};
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_CHOICE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub struct QuizUI {
    quiz_game: QuizGame,
    /// Characters the player has typed for the current question. Per
    /// `docs/spec.md`, this is the only source of truth for which choice is
    /// being picked — there is no arrow / number-key fallback.
    input_buffer: String,
    current_result: Option<QuizResult>,
    show_result: bool,
}

impl QuizUI {
    pub fn new(questions: Vec<Question>, language: Language) -> Self {
        let mut quiz_game = QuizGame::new(questions, language);
        quiz_game.start();

        Self {
            quiz_game,
            input_buffer: String::new(),
            current_result: None,
            show_result: false,
        }
    }

    pub fn run(&mut self) -> Result<u32, Box<dyn std::error::Error>> {
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
    ) -> Result<u32, Box<dyn std::error::Error>> {
        // Poll cadence: small enough that the side-pane Time advances within
        // a noticeable fraction of a second (#54), large enough not to spin.
        const TICK: Duration = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(TICK)? {
                if let Event::Key(key) = event::read()? {
                    if self.handle_key(key) {
                        break;
                    }
                }
            }
            // No event in this tick — loop redraws so Time / CPM / WPM keep
            // moving even while the player is thinking.
        }

        Ok(self.quiz_game.get_final_score())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Quit keys: Esc, and Ctrl+C as the standard terminal escape.
        // Printable characters (including 'q') are part of typed selection
        // and must reach the buffer.
        if matches!(key.code, KeyCode::Esc) {
            return true;
        }
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        if self.show_result {
            return self.handle_key_result(key);
        }

        match key.code {
            KeyCode::Enter => {
                let typed = self.input_buffer.clone();
                if let Some(result) = self.quiz_game.answer_question_typed(&typed) {
                    self.current_result = Some(result);
                    self.show_result = true;
                }
            }
            KeyCode::Tab => {
                if self.quiz_game.skip_question() {
                    self.input_buffer.clear();
                    if self.quiz_game.is_game_finished() {
                        return true;
                    }
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            // Drop any modifier-bearing chord (Ctrl+X / Alt+X) so it cannot
            // accidentally land in the typed answer.
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        false
    }

    fn handle_key_result(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Enter {
            if self.quiz_game.is_game_finished() {
                return true;
            }
            self.show_result = false;
            self.current_result = None;
            self.input_buffer.clear();
        }
        false
    }

    fn ui(&mut self, f: &mut Frame) {
        let frame = PaneFrame::quiz(f.area());

        self.render_main_pane(f, frame.main);
        self.render_status_pane(f, frame.side);
        self.render_input_echo(f, frame.input_echo);
        self.render_help_line(f, frame.help_line);
    }

    fn render_input_echo(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }
        // Hide the echo while showing a result — keeps the screen calm
        // during the "correct/wrong" reveal.
        let body = if self.show_result {
            String::new()
        } else {
            format!("> {}_", self.input_buffer)
        };
        let line = Paragraph::new(body)
            .style(STYLE_INPUT_ECHO)
            .alignment(Alignment::Left);
        f.render_widget(line, area);
    }

    fn render_status_pane(&self, f: &mut Frame, area: Rect) {
        // CPM / WPM are 0 until typed selection (#24) lands and we begin
        // measuring keystrokes; the pane structure already reserves the slots.
        let elapsed = self.quiz_game.get_total_time().unwrap_or(Duration::ZERO);
        let pane = StatusPane::quiz(self.quiz_game.get_final_score(), elapsed, 0, 0);
        pane.render(f, area);
    }

    fn render_main_pane(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(area);

        let (current, total) = self.quiz_game.get_progress();
        let title_text = format!("TypeGlobe — Quiz  Q{current}/{total}");
        let title = Paragraph::new(title_text)
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        if self.show_result {
            self.render_result(f, chunks[1]);
        } else {
            self.render_question(f, chunks[1]);
        }
    }

    fn render_question(&self, f: &mut Frame, area: Rect) {
        if let Some(question) = self.quiz_game.get_current_question() {
            let question_text = self.quiz_game.get_question_text(question);
            let choices = self.quiz_game.get_choice_texts(question);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(4)])
                .split(area);

            let question_paragraph = Paragraph::new(question_text)
                .style(STYLE_NORMAL)
                .alignment(Alignment::Left)
                .block(Block::default().title("Question").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(question_paragraph, chunks[0]);

            // Plain non-highlighted list — typed selection means there is no
            // "currently focused" choice. Labels A/B/C/D match docs/spec.md.
            const LABELS: [&str; 4] = ["A", "B", "C", "D"];
            let choice_items: Vec<ListItem> = choices
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    let label = LABELS.get(i).copied().unwrap_or("?");
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{label}) "), STYLE_CHOICE_LABEL),
                        Span::styled(choice.clone(), STYLE_NORMAL),
                    ]))
                })
                .collect();

            let choices_list = List::new(choice_items)
                .block(Block::default().title("Choices").borders(Borders::ALL));

            f.render_widget(choices_list, chunks[1]);
        } else {
            let no_question = Paragraph::new("No question available")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(no_question, area);
        }
    }

    fn render_result(&self, f: &mut Frame, area: Rect) {
        let (Some(result), Some(question)) =
            (&self.current_result, self.quiz_game.get_current_question())
        else {
            return;
        };

        let choices = self.quiz_game.get_choice_texts(question);

        let (result_text, result_style) = if result.is_correct {
            ("Correct!".to_string(), STYLE_CORRECT)
        } else {
            let correct_text = choices
                .get(result.correct_answer_index)
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            (format!("Wrong. Answer: {correct_text}"), STYLE_INCORRECT)
        };

        let result_paragraph = Paragraph::new(result_text)
            .style(result_style)
            .alignment(Alignment::Center)
            .block(Block::default().title("Result").borders(Borders::ALL));
        f.render_widget(result_paragraph, area);
    }

    fn render_help_line(&self, f: &mut Frame, area: Rect) {
        self.help_line().render(f, area);
    }

    fn help_line(&self) -> HelpLine {
        if self.show_result {
            let next_label = if self.quiz_game.is_game_finished() {
                "Finish"
            } else {
                "Next"
            };
            HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Enter", next_label),
            ])
        } else {
            // Spec form per docs/spec.md (`[Esc] Quit  [Tab] Skip  [F5] Restart`),
            // augmented with Enter / Bksp for typed selection. F5 Restart is
            // not wired yet.
            HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Tab", "Skip"),
                HelpEntry::new("Enter", "Confirm"),
                HelpEntry::new("Bksp", "Erase"),
            ])
        }
    }
}
