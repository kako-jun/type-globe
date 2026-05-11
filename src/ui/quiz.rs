use crate::audio::{Cue, CueEngine};
use crate::game::QuizGame;
use crate::io::Storage;
use crate::jiwa_core::{lerp_rgb, RevealHandle, RevealOpts, Rgb};
use crate::types::{Language, Question, ScoreEntry};
use crate::ui::{HelpEntry, HelpLine, InputChannel, PaneFrame, RecvOutcome, StatusPane};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as RFC3339 UTC: YYYY-MM-DDTHH:MM:SSZ
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Days since Unix epoch → Gregorian date (proleptic)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    // Uses i64 internally to avoid underflow in intermediate subtractions.
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m as u64, d as u64)
}

/// Maximum characters the player can type into the name-entry field.
/// Sized to fit comfortably in the side pane / Records list rendering.
const NAME_MAX_CHARS: usize = 16;

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const INPUT_REJECT_FLASH_MS: u64 = 180;
/// How long after the question reveal starts before the choices block begins
/// fading in (Issue #72). Roughly the time it takes for the eye to land on
/// the question line, so the choices show up just as the player is ready
/// to look down.
const CHOICES_REVEAL_DELAY_MS: u64 = 500;
/// How long the choices block takes to fade from invisible to its final
/// color. The four choices appear as a single group (Issue #72), so this
/// is the only timing knob the choices reveal needs.
const CHOICES_FADE_MS: u64 = 320;

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
    /// Permutation of the active question's choices (Issue #72). Indices
    /// reference the original `question.choices`; `correct_answer_index`
    /// stays meaningful because the typing match is identity-based, not
    /// position-based.
    choice_order: Vec<usize>,
    /// Wall-clock instant at which the choices block begins fading in
    /// (Issue #72). Set when a new question's reveal is anchored, so the
    /// choices appear `CHOICES_REVEAL_DELAY_MS` after the question text.
    choices_reveal_starts_at: Option<Instant>,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
    /// Sound-effect engine (Issue #73). `None` when audio output is
    /// unavailable — Quiz still works silently in that case.
    cues: Option<CueEngine>,
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
            choice_order: Vec::new(),
            choices_reveal_starts_at: None,
            rejected_char: None,
            reject_flash_until: None,
            cues: CueEngine::new(),
        }
    }

    fn play_cue(&self, cue: Cue) {
        if let Some(engine) = self.cues.as_ref() {
            engine.play(cue);
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
                // Skipping a question counts as the "wrong" outcome for
                // sound purposes (Issue #73): the player gave up rather
                // than landing the correct answer, so play "ブブー".
                self.play_cue(Cue::Wrong);
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
            ts: now_rfc3339(),
        };
        records.push_quiz(entry);
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
        // New question is about to be revealed — fire the "ダダン" cue
        // (Issue #73) before the typewriter starts, so the audio leads
        // the visual by a frame.
        if self.quiz_game.get_current_question().is_some() {
            self.play_cue(Cue::QuestionReveal);
        }
        let now = Instant::now();
        self.reveal = self.quiz_game.get_current_question().map(|question| {
            let text = self.quiz_game.get_question_text(question);
            RevealHandle::start_at(&text, RevealOpts::default_quiz(), now)
        });
        // Issue #72: shuffle the four choices each question and stagger
        // their reveal so the player reads the question first, then the
        // choices fade in together a moment later.
        if let Some(question) = self.quiz_game.get_current_question() {
            let mut order: Vec<usize> = (0..question.choices.len()).collect();
            order.shuffle(&mut rand::thread_rng());
            self.choice_order = order;
            self.choices_reveal_starts_at =
                Some(now + Duration::from_millis(CHOICES_REVEAL_DELAY_MS));
        } else {
            self.choice_order.clear();
            self.choices_reveal_starts_at = None;
        }
        self.reveal_for_question = Some(current_idx);
    }

    /// Linear fade-in alpha for the choices block. Returns 0.0 while the
    /// reveal is still scheduled in the future, ramps to 1.0 over
    /// `CHOICES_FADE_MS`, and stays pinned at 1.0 afterwards.
    fn choices_fade_alpha(&self) -> f32 {
        let Some(starts_at) = self.choices_reveal_starts_at else {
            return 1.0;
        };
        let now = Instant::now();
        if now < starts_at {
            return 0.0;
        }
        let elapsed_ms = now.saturating_duration_since(starts_at).as_millis() as u64;
        if CHOICES_FADE_MS == 0 {
            return 1.0;
        }
        (elapsed_ms as f32 / CHOICES_FADE_MS as f32).clamp(0.0, 1.0)
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
            // Mistype cue (Issue #73) — slightly louder than the
            // keystroke tick so the player can tell them apart.
            self.play_cue(Cue::Mistype);
            return;
        }

        self.input_buffer.push(c);
        self.clear_reject_flash();
        // Quiet typing tick (Issue #73). Played only on accepted
        // characters so the mistype cue can stand alone.
        self.play_cue(Cue::Keystroke);

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
            // Issue #70 limits this code path to correct answers (the
            // input layer rejects wrong ones), so this is always the
            // "ピンポーン" cue. Issue #73.
            self.play_cue(Cue::Correct);
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
            Phase::Playing => format!("type-globe - Quiz {current}/{total}"),
            Phase::Summary => "type-globe - Quiz".to_string(),
            Phase::NamingForRecord => "type-globe - Quiz".to_string(),
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
                // Length(5) keeps 1 visible row of question text after
                // the 2-row border and Issue #76's 2-row vertical padding
                // (border + padding eats 4 rows). Choices stays Min so
                // it absorbs whatever vertical space is left.
                .constraints([Constraint::Length(5), Constraint::Min(6)])
                .split(area);

            let question_line = self.question_reveal_line(question);
            let question_paragraph = Paragraph::new(question_line)
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .title(" Question ")
                        .borders(Borders::ALL)
                        .padding(Padding::uniform(1)),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(question_paragraph, chunks[0]);

            // Plain non-highlighted list — typed selection means there is no
            // "currently focused" choice. Labels A/B/C/D match docs/spec.md.
            // Issue #72: display order is the shuffled `choice_order`, not
            // the original index, so the answer's position varies per
            // question and the four choices fade in together after the
            // question text is on screen.
            const LABELS: [&str; 4] = ["A", "B", "C", "D"];
            let alpha = self.choices_fade_alpha();
            let label_color = lerp_rgb_color(Rgb(20, 60, 80), Rgb(80, 200, 255), alpha);
            let text_color = lerp_rgb_color(Rgb(20, 20, 20), Rgb(255, 255, 255), alpha);
            let label_style = Style::new().fg(label_color).add_modifier(Modifier::BOLD);
            let text_style = Style::new().fg(text_color);
            let order: Vec<usize> = if self.choice_order.len() == choices.len() {
                self.choice_order.clone()
            } else {
                (0..choices.len()).collect()
            };
            let choice_items: Vec<ListItem> = order
                .iter()
                .enumerate()
                .filter_map(|(display_idx, &orig_idx)| {
                    let label = LABELS.get(display_idx).copied().unwrap_or("?");
                    let choice = choices.get(orig_idx)?.clone();
                    Some(ListItem::new(Line::from(vec![
                        Span::styled(format!("{label}) "), label_style),
                        Span::styled(choice, text_style),
                    ])))
                })
                .collect();

            let choices_list = List::new(choice_items).block(
                Block::default()
                    .title(" Choices ")
                    .borders(Borders::ALL)
                    .padding(Padding::uniform(1)),
            );

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

        let body = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(" Summary ")
                .borders(Borders::ALL)
                .padding(Padding::uniform(1)),
        );
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
                .title(" Records entry ")
                .borders(Borders::ALL)
                .padding(Padding::uniform(1)),
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

/// Interpolate two `Rgb` triples and return a ratatui `Color`. Thin
/// wrapper over `jiwa_core::lerp_rgb` so the choices fade-in (Issue #72)
/// uses the same channel math as the question text reveal.
fn lerp_rgb_color(from: Rgb, to: Rgb, t: f32) -> Color {
    let Rgb(r, g, b) = lerp_rgb(from, to, t);
    Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_rfc3339_matches_format() {
        let ts = now_rfc3339();
        // Expected: YYYY-MM-DDTHH:MM:SSZ (20 chars)
        assert_eq!(ts.len(), 20, "unexpected length: {ts}");
        assert_eq!(&ts[4..5], "-", "missing dash after year: {ts}");
        assert_eq!(&ts[7..8], "-", "missing dash after month: {ts}");
        assert_eq!(&ts[10..11], "T", "missing T separator: {ts}");
        assert_eq!(&ts[13..14], ":", "missing colon after hour: {ts}");
        assert_eq!(&ts[16..17], ":", "missing colon after minute: {ts}");
        assert_eq!(&ts[19..20], "Z", "missing Z suffix: {ts}");
        // All digit positions must be numeric
        for pos in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
            let ch = ts.as_bytes()[pos];
            assert!(ch.is_ascii_digit(), "non-digit at pos {pos} in {ts}");
        }
    }
}
