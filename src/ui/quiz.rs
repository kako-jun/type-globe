use crate::game::QuizGame;
use crate::io::Storage;
use crate::jiwa_core::{RevealHandle, RevealOpts, Rgb};
use crate::types::{Language, Question, ScoreEntry};
use crate::ui::{HelpEntry, HelpLine, InputChannel, PaneFrame, RecvOutcome, StatusPane};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
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
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Top-N self-best entries kept per mode in `records_<lang>.json`.
const RECORDS_TOP_N: usize = 10;
/// Maximum characters the player can type into the name-entry field.
/// Sized to fit comfortably in the side pane / Records list rendering.
const NAME_MAX_CHARS: usize = 16;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_CHOICE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const INPUT_REJECT_FLASH_MS: u64 = 180;

/// High-level state machine for one Quiz session. Issue #70 removed the
/// per-question result interstitial, so playing → summary → records-name-
/// entry → done is the entire flow.
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
    phase: Phase,
    name_buffer: String,
    records_file_path: String,
    /// Set once the run's score has been saved to records, so a second
    /// Enter on the confirmation screen exits without writing a duplicate.
    saved: bool,
    /// Question-text reveal animation (typewriter + fade-in, #19/#20/#21).
    /// Lazily (re-)created each time the displayed question changes.
    reveal: Option<RevealHandle>,
    /// Question index the current `reveal` is anchored to. Used to detect
    /// when we need to reset the animation for the next question.
    reveal_for_question: Option<usize>,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
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
            phase: Phase::Playing,
            name_buffer: String::new(),
            records_file_path,
            saved: false,
            reveal: None,
            reveal_for_question: None,
            rejected_char: None,
            reject_flash_until: None,
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
        // Redraw cadence is short enough that the typewriter / fade
        // reveal (#19/#20/#21) renders smoothly (~33 fps). The actual
        // input read happens on a separate thread (#22) so a player who
        // already knows the answer can begin typing during the reveal
        // without waiting for the next redraw tick.
        const REDRAW: Duration = Duration::from_millis(30);

        let input = InputChannel::spawn();

        loop {
            terminal.draw(|f| self.ui(f))?;

            match input.recv_until(REDRAW) {
                RecvOutcome::Key(key) => {
                    if self.handle_key(key) {
                        break;
                    }
                }
                RecvOutcome::Timeout => {
                    // No input in this window — loop redraws so the
                    // reveal animation and the timer keep moving while
                    // the player is thinking.
                }
                RecvOutcome::Disconnected => break, // Worker thread exited.
            }
        }
        // `input` drops here → shutdown flag flipped → worker joins.

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

        match key.code {
            KeyCode::Enter => {}
            // The guard's side effect (skip_question) is intentional — Tab
            // skipping is the action, not a precondition. If skip fails
            // (no current question), the arm falls through to `_ => {}`.
            KeyCode::Tab if self.quiz_game.skip_question() => {
                self.input_buffer.clear();
                self.clear_reject_flash();
                if self.quiz_game.is_game_finished() {
                    self.phase = Phase::Summary;
                    return false;
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.clear_reject_flash();
            }
            // Drop any modifier-bearing chord (Ctrl+X / Alt+X) so it cannot
            // accidentally land in the typed answer.
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.handle_playing_char(c);
            }
            _ => {}
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
                    return false;
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
        if self.phase == Phase::Playing {
            self.ensure_reveal_for_current_question();
        }

        let frame = PaneFrame::quiz(f.area());

        self.render_main_pane(f, frame.main);
        self.render_status_pane(f, frame.side);
        self.render_input_echo(f, frame.input_echo);
        self.render_help_line(f, frame.help_line);
    }

    /// Lazy-create / reset the question-text reveal so the animation
    /// re-runs from the start whenever the player advances to a new
    /// question. Anchored to wall-clock `Instant::now()` because the
    /// rest of the loop already lives in real time.
    fn ensure_reveal_for_current_question(&mut self) {
        let (current_idx, _) = self.quiz_game.get_progress();
        if self.reveal_for_question == Some(current_idx) {
            return;
        }
        self.reveal = self.quiz_game.get_current_question().map(|question| {
            let text = self.quiz_game.get_question_text(question);
            RevealHandle::start(&text, RevealOpts::default_quiz())
        });
        self.reveal_for_question = Some(current_idx);
    }

    fn render_input_echo(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }
        let line = match self.phase {
            Phase::Playing => self.render_playing_input_line(),
            Phase::Summary => Line::from(""),
            Phase::NamingForRecord => Line::from(vec![
                Span::styled("name> ", STYLE_DIM),
                Span::styled(self.name_buffer.clone(), STYLE_INPUT_ECHO),
                Span::styled("_", STYLE_INPUT_ECHO),
            ]),
        };
        f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
    }

    fn render_playing_input_line(&self) -> Line<'static> {
        let flash_active = self.reject_flash_is_active();
        let prompt = if self.should_shake_input_echo() {
            " > "
        } else {
            "> "
        };
        let mut spans = vec![
            Span::styled(
                prompt.to_string(),
                if flash_active {
                    STYLE_INCORRECT
                } else {
                    STYLE_DIM
                },
            ),
            Span::styled(self.input_buffer.clone(), STYLE_CORRECT),
        ];
        if flash_active {
            if let Some(c) = self.rejected_char {
                spans.push(Span::styled(c.to_string(), STYLE_INCORRECT));
            }
            spans.push(Span::styled("_", STYLE_INCORRECT));
        } else {
            spans.push(Span::styled("_", STYLE_INPUT_ECHO));
        }
        Line::from(spans)
    }

    fn handle_playing_char(&mut self, c: char) {
        let mut attempted = self.input_buffer.clone();
        attempted.push(c);
        // Per Issue #70 only the correct answer's typings are accepted;
        // any divergence is treated as a mistype that flashes red and
        // resets the buffer to zero so the player retries from scratch.
        if !self.quiz_game.is_valid_correct_typed_prefix(&attempted) {
            self.note_rejected_char(c);
            self.input_buffer.clear();
            return;
        }

        self.input_buffer.push(c);
        self.clear_reject_flash();

        let typed = self.input_buffer.to_lowercase();
        if self
            .quiz_game
            .current_correct_typing_candidates()
            .iter()
            .any(|candidate| candidate == &typed)
        {
            self.submit_current_answer();
        }
    }

    /// Record the correct answer in the game state and advance immediately.
    /// Issue #70: there is no "Correct!" interstitial — the player flows
    /// straight into the next question, or into the summary screen if this
    /// was the final question (in which case `QuizGame` has already frozen
    /// the elapsed time).
    fn submit_current_answer(&mut self) {
        let typed = self.input_buffer.clone();
        if self.quiz_game.answer_question_typed(&typed).is_some() {
            self.input_buffer.clear();
            self.clear_reject_flash();
            if self.quiz_game.is_game_finished() {
                self.phase = Phase::Summary;
            }
        }
    }

    fn note_rejected_char(&mut self, c: char) {
        self.rejected_char = Some(c);
        self.reject_flash_until =
            Some(Instant::now() + Duration::from_millis(INPUT_REJECT_FLASH_MS));
    }

    fn clear_reject_flash(&mut self) {
        self.rejected_char = None;
        self.reject_flash_until = None;
    }

    fn reject_flash_is_active(&self) -> bool {
        self.reject_flash_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn should_shake_input_echo(&self) -> bool {
        self.reject_flash_until
            .map(|until| {
                let remaining_ticks =
                    until.saturating_duration_since(Instant::now()).as_millis() / 45;
                self.reject_flash_is_active() && remaining_ticks % 2 == 0
            })
            .unwrap_or(false)
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
            Phase::Playing => self.render_question(f, chunks[1]),
        }
    }

    /// Build the question's text as a styled `Line`, using the reveal
    /// snapshot when one is active. Each visible grapheme becomes one
    /// `Span` with its current per-grapheme RGB color so the typewriter
    /// and fade-in show up natively. Falls back to the plain text with a
    /// white foreground when no reveal is available — e.g. a brief race
    /// before `ensure_reveal_for_current_question` runs.
    fn question_reveal_line(&self, question: &Question) -> Line<'static> {
        if let Some(reveal) = self.reveal.as_ref() {
            let snapshot = reveal.snapshot(Instant::now());
            if !snapshot.is_empty() {
                let spans: Vec<Span<'static>> = snapshot
                    .into_iter()
                    .map(|g| {
                        let Rgb(r, gc, b) = g.color;
                        Span::styled(g.text, Style::new().fg(Color::Rgb(r, gc, b)))
                    })
                    .collect();
                return Line::from(spans);
            }
        }
        let text = self.quiz_game.get_question_text(question);
        Line::from(Span::styled(text, STYLE_NORMAL))
    }

    fn render_question(&self, f: &mut Frame, area: Rect) {
        if let Some(question) = self.quiz_game.get_current_question() {
            let choices = self.quiz_game.get_choice_texts(question);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(4)])
                .split(area);

            let question_line = self.question_reveal_line(question);
            let question_paragraph = Paragraph::new(question_line)
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
            Phase::Playing => HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Tab", "Skip"),
                HelpEntry::new("Auto", "Confirm"),
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
