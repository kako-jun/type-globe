use crate::game::{QuizGame, QuizResult};
use crate::io::Storage;
use crate::types::{Language, Question, ScoreEntry};
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Top-N self-best entries kept per mode in `records_<lang>.json`.
const RECORDS_TOP_N: usize = 10;
/// Maximum characters the player can type into the name-entry field.
/// Sized to fit comfortably in the side pane / Records list rendering.
const NAME_MAX_CHARS: usize = 16;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_CHOICE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

/// High-level state machine for one Quiz session. The per-question
/// "show_result" flag still lives separately; this enum captures the flow
/// across the run: playing → summary → records-name-entry → done.
#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Playing,
    Summary,
    NamingForRecord,
}

pub struct QuizUI {
    quiz_game: QuizGame,
    /// Characters the player has typed for the current question. Per
    /// `docs/spec.md`, this is the only source of truth for which choice is
    /// being picked — there is no arrow / number-key fallback.
    input_buffer: String,
    current_result: Option<QuizResult>,
    show_result: bool,
    phase: Phase,
    name_buffer: String,
    records_file_path: String,
    /// Set once the run's score has been saved to records, so a second
    /// Enter on the confirmation screen exits without writing a duplicate.
    saved: bool,
}

impl QuizUI {
    /// Build a UI by sampling a 10-question run from `pool`. Mirrors
    /// `QuizGame::from_pool` so main.rs doesn't have to reach into the
    /// game module directly.
    pub fn from_pool(pool: &[Question], language: Language, records_file_path: String) -> Self {
        let mut quiz_game = QuizGame::from_pool(pool, language);
        quiz_game.start();
        Self {
            quiz_game,
            input_buffer: String::new(),
            current_result: None,
            show_result: false,
            phase: Phase::Playing,
            name_buffer: String::new(),
            records_file_path,
            saved: false,
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

        match self.phase {
            Phase::Summary => return self.handle_key_summary(key),
            Phase::NamingForRecord => return self.handle_key_naming(key),
            Phase::Playing => {}
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
            // The guard's side effect (skip_question) is intentional — Tab
            // skipping is the action, not a precondition. If skip fails
            // (no current question), the arm falls through to `_ => {}`.
            KeyCode::Tab if self.quiz_game.skip_question() => {
                self.input_buffer.clear();
                if self.quiz_game.is_game_finished() {
                    return true;
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
                self.phase = Phase::Summary;
                self.show_result = false;
                self.current_result = None;
                self.input_buffer.clear();
                return false;
            }
            self.show_result = false;
            self.current_result = None;
            self.input_buffer.clear();
        }
        false
    }

    fn handle_key_summary(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Enter {
            self.phase = Phase::NamingForRecord;
            self.name_buffer.clear();
        }
        false
    }

    /// Name-entry phase: printable chars accumulate, Backspace deletes,
    /// Enter on a non-empty name saves and exits, Enter on an empty name
    /// is a no-op (forces an explicit choice — Esc still skips the save).
    fn handle_key_naming(&mut self, key: KeyEvent) -> bool {
        if self.saved {
            // Once saved, any Enter / printable key dismisses the
            // confirmation screen and returns to the menu.
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(_)) {
                return true;
            }
            return false;
        }

        match key.code {
            KeyCode::Enter => {
                if self.name_buffer.trim().is_empty() {
                    return false;
                }
                if let Err(err) = self.persist_record() {
                    // Surfacing the failure inline keeps the run from
                    // silently dropping on a disk error — Esc / next Enter
                    // still exits regardless.
                    eprintln!("warning: failed to save records: {err}");
                }
                self.saved = true;
            }
            KeyCode::Backspace => {
                self.name_buffer.pop();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && self.name_buffer.chars().count() < NAME_MAX_CHARS =>
            {
                self.name_buffer.push(c);
            }
            _ => {}
        }
        false
    }

    fn persist_record(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut records = Storage::load_records(&self.records_file_path)?;
        let entry = ScoreEntry {
            name: self.name_buffer.trim().to_string(),
            score: self.quiz_game.get_final_score(),
            cpm: self.quiz_game.get_cpm(),
            wpm: self.quiz_game.get_wpm(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        records.quiz_mode.push(entry);
        // Highest score first, then ts descending as a stable tiebreaker
        // so the most recent attempt at a tied score wins display order.
        records
            .quiz_mode
            .sort_by(|a, b| b.score.cmp(&a.score).then(b.ts.cmp(&a.ts)));
        records.quiz_mode.truncate(RECORDS_TOP_N);
        Storage::save_records(&self.records_file_path, &records)?;
        Ok(())
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
        let body = match self.phase {
            // Hide the echo on result / summary — keeps the screen calm
            // during the "correct/wrong" reveal and final score reveal.
            Phase::Playing if self.show_result => String::new(),
            Phase::Playing => format!("> {}_", self.input_buffer),
            Phase::Summary => String::new(),
            Phase::NamingForRecord => format!("name> {}_", self.name_buffer),
        };
        let line = Paragraph::new(body)
            .style(STYLE_INPUT_ECHO)
            .alignment(Alignment::Left);
        f.render_widget(line, area);
    }

    fn render_status_pane(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.quiz_game.get_total_time().unwrap_or(Duration::ZERO);
        let pane = StatusPane::quiz(
            self.quiz_game.get_final_score(),
            elapsed,
            self.quiz_game.get_cpm(),
            self.quiz_game.get_wpm(),
        );
        pane.render(f, area);
    }

    fn render_main_pane(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(area);

        let (current, total) = self.quiz_game.get_progress();
        let title_text = match self.phase {
            Phase::Playing => format!("TypeGlobe — Quiz  Q{current}/{total}"),
            Phase::Summary => "TypeGlobe — Quiz  Run complete".to_string(),
            Phase::NamingForRecord => "TypeGlobe — Quiz  Records entry".to_string(),
        };
        let title = Paragraph::new(title_text)
            .style(STYLE_TITLE)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        match self.phase {
            Phase::Summary => self.render_summary(f, chunks[1]),
            Phase::NamingForRecord => self.render_naming(f, chunks[1]),
            Phase::Playing if self.show_result => self.render_result(f, chunks[1]),
            Phase::Playing => self.render_question(f, chunks[1]),
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

    fn render_summary(&self, f: &mut Frame, area: Rect) {
        let total = self.quiz_game.get_progress().1 as u32;
        let correct = self.quiz_game.get_correct_count();
        let accuracy_pct = (self.quiz_game.get_accuracy() * 100.0).round() as u32;
        let elapsed = self.quiz_game.get_total_time().unwrap_or(Duration::ZERO);
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;

        let lines = vec![
            Line::from(Span::styled("Run complete", STYLE_CORRECT)),
            Line::from(""),
            Line::from(format!("  Score    : {}", self.quiz_game.get_final_score())),
            Line::from(format!("  Correct  : {correct} / {total}")),
            Line::from(format!("  Accuracy : {accuracy_pct}%")),
            Line::from(format!("  CPM      : {}", self.quiz_game.get_cpm())),
            Line::from(format!("  WPM      : {}", self.quiz_game.get_wpm())),
            Line::from(format!("  Time     : {mins}:{secs:02}")),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to register a record (Esc to skip).",
                STYLE_NORMAL,
            )),
        ];

        let body = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default().title("Summary").borders(Borders::ALL));
        f.render_widget(body, area);
    }

    fn render_naming(&self, f: &mut Frame, area: Rect) {
        let lines = if self.saved {
            vec![
                Line::from(Span::styled("Record saved.", STYLE_CORRECT)),
                Line::from(""),
                Line::from(format!("  Name  : {}", self.name_buffer.trim())),
                Line::from(format!("  Score : {}", self.quiz_game.get_final_score())),
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to return to the menu.",
                    STYLE_NORMAL,
                )),
            ]
        } else {
            vec![
                Line::from("Enter a name for your records entry."),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  name : {}_", self.name_buffer),
                    STYLE_INPUT_ECHO,
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("(max {NAME_MAX_CHARS} chars; Enter saves, Esc skips)"),
                    STYLE_NORMAL,
                )),
            ]
        };

        let body = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title("Records entry")
                .borders(Borders::ALL),
        );
        f.render_widget(body, area);
    }

    fn help_line(&self) -> HelpLine {
        match self.phase {
            Phase::Playing if self.show_result => {
                let next_label = if self.quiz_game.is_game_finished() {
                    "Summary"
                } else {
                    "Next"
                };
                HelpLine::new(vec![
                    HelpEntry::new("Esc", "Quit"),
                    HelpEntry::new("Enter", next_label),
                ])
            }
            Phase::Playing => HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Tab", "Skip"),
                HelpEntry::new("Enter", "Confirm"),
                HelpEntry::new("Bksp", "Erase"),
            ]),
            Phase::Summary => HelpLine::new(vec![
                HelpEntry::new("Esc", "Skip"),
                HelpEntry::new("Enter", "Register"),
            ]),
            Phase::NamingForRecord if self.saved => {
                HelpLine::new(vec![HelpEntry::new("Enter", "Menu")])
            }
            Phase::NamingForRecord => HelpLine::new(vec![
                HelpEntry::new("Esc", "Skip"),
                HelpEntry::new("Enter", "Save"),
                HelpEntry::new("Bksp", "Erase"),
            ]),
        }
    }
}
