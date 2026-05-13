use crate::audio::{Cue, CueEngine};
use crate::game::QuizGame;
use crate::io::Storage;
use crate::jiwa_core::{lerp_rgb, RevealHandle, RevealOpts, Rgb};
use crate::types::{Language, Question, ScoreEntry};
use crate::ui::inline_code;
use crate::ui::{
    DemoInputSource, HelpEntry, HelpLine, InputChannel, KeyEventSource, MultiplexedSource,
    PaneFrame, RecvOutcome, StatusPane,
};
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
/// Foreground color applied to Markdown inline-code spans in question and
/// choice text (Issue #97). The Bold modifier is added on top per-span;
/// this constant only carries the color so the reveal animation can mix
/// it with the per-grapheme fade color.
// Issue #97 nit: shifted toward orange so the inline-code highlight is
// unambiguously distinct from `Color::Yellow` (the input-echo color).
const INLINE_CODE_COLOR: Color = Color::Rgb(255, 200, 60);
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

/// Result of one demo session (#106 review: M-2/S-1).
///
/// The caller (`run_quiz_demo` in `main.rs`) needs to distinguish three
/// outcomes so it can react correctly inside `--demo-loop`:
///   - normal completion → keep looping
///   - the user pressed Esc / Ctrl+C → break out of the outer loop too
///   - no typing target could be derived for the current question
///     (data hole) → count it toward the consecutive-abort breaker so a
///     pathological pool can't busy-loop the kiosk forever
///
/// Carrying these flags as data (rather than printing `eprintln!`
/// straight from inside the alt screen, which used to corrupt the TUI
/// output) lets `run_quiz_demo` emit warnings *after* the terminal has
/// been restored.
#[derive(Debug, Clone, Default)]
pub struct DemoOutcome {
    /// Final score of the run. Currently unused by `run_quiz_demo` (the
    /// kiosk demo intentionally discards scores) but kept on the
    /// outcome so future callers — e.g. a "demo + record to disk"
    /// mode — don't have to break the return type again.
    #[allow(dead_code)]
    pub score: u32,
    /// `true` when the player hit Esc / Ctrl+C during the session.
    pub user_aborted: bool,
    /// `true` when the demo had to abort the session because no typing
    /// target was available for the current question (ja_typings empty
    /// and label not convertible). The loop driver should warn the user
    /// after leaving the alt screen, and break after enough consecutive
    /// hits to avoid an infinite kiosk loop on a broken question pool.
    pub no_target_abort: bool,
    /// Optional human-readable warnings collected during the run that
    /// the caller should `eprintln!` *after* the alt screen has been
    /// torn down. Currently used for `persist_record` failures so the
    /// disk error reaches the user without corrupting the TUI.
    pub warnings: Vec<String>,
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
    /// Grapheme index ranges of Markdown inline-code spans in the current
    /// question text *after* backticks are stripped (Issue #97). Aligned
    /// with the reveal snapshot indices so we can style code graphemes
    /// without disturbing the per-character fade-in.
    code_ranges: Vec<(usize, usize)>,
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
    /// Warnings collected during the run that must be surfaced to the
    /// user *after* the alt screen has been torn down (M-2). Examples:
    /// `persist_record` disk-write failures. Plain `eprintln!` while
    /// the alt screen is active would either be hidden by ratatui's
    /// next redraw or visibly tear the TUI when it scrolls into view.
    pending_warnings: Vec<String>,
    /// Set to `true` when the demo path discovered the current question
    /// has no usable typing target. Signals `run_app` to break out of
    /// the loop without delivering more synthetic input. Distinct from
    /// `user_aborted` so the loop driver can apply the consecutive-
    /// abort breaker (M-3).
    no_target_abort: bool,
    /// Set to `true` when the player pressed Esc / Ctrl+C. Propagated
    /// up through `DemoOutcome` so `--demo-loop` can break the outer
    /// loop too (S-1) rather than restarting another session right
    /// after the user already asked to leave.
    user_aborted: bool,
}

impl QuizUI {
    /// Build a UI by sampling a 10-question run from `pool`. Mirrors
    /// `QuizGame::from_pool` so main.rs doesn't have to reach into the
    /// game module directly.
    pub fn from_pool(pool: &[Question], language: Language, records_file_path: String) -> Self {
        let mut quiz_game = QuizGame::from_pool(pool, language);
        quiz_game.start();
        Self::wrap_started_game(quiz_game, records_file_path)
    }

    /// Variant of [`from_pool`] that lets the caller pin the run length.
    /// The auto-demo (#106) uses this to honour `--demo-count`.
    pub fn from_pool_with_count(
        pool: &[Question],
        language: Language,
        records_file_path: String,
        count: usize,
    ) -> Self {
        let mut quiz_game = QuizGame::from_pool_with_count(pool, language, count);
        quiz_game.start();
        Self::wrap_started_game(quiz_game, records_file_path)
    }

    fn wrap_started_game(quiz_game: QuizGame, records_file_path: String) -> Self {
        Self {
            quiz_game,
            input_buffer: String::new(),
            phase: Phase::Playing,
            name_buffer: String::new(),
            records_file_path,
            saved: false,
            reveal: None,
            reveal_for_question: None,
            code_ranges: Vec::new(),
            choice_order: Vec::new(),
            choices_reveal_starts_at: None,
            rejected_char: None,
            reject_flash_until: None,
            cues: CueEngine::new(),
            pending_warnings: Vec::new(),
            no_target_abort: false,
            user_aborted: false,
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

        let input = InputChannel::spawn();
        let result = self.run_app(&mut terminal, &input, None);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // M-2: flush warnings (e.g. persist_record disk failure) only
        // after the alt screen is gone so the message survives in the
        // user's scrollback.
        for w in self.pending_warnings.drain(..) {
            eprintln!("{w}");
        }

        result
    }

    /// Run the quiz under auto-demo control (#106). The `demo` source
    /// drives typing; a real-keyboard `InputChannel` is multiplexed on
    /// top so Esc / Ctrl+C still aborts. The session itself is unchanged —
    /// reveal, sound, auto-confirm and result screen all flow through
    /// the same code path as a human run.
    ///
    /// Returns a [`DemoOutcome`] carrying the score plus abort flags and
    /// any deferred warnings the caller should print after restoring
    /// the terminal (M-2/S-1).
    pub fn run_with_demo(
        &mut self,
        demo: DemoInputSource,
    ) -> Result<DemoOutcome, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let human = InputChannel::spawn();
        let source = MultiplexedSource { a: human, b: demo };
        let result = self.run_app(&mut terminal, &source, Some(&source.b));

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        let score = result?;
        Ok(DemoOutcome {
            score,
            user_aborted: self.user_aborted,
            no_target_abort: self.no_target_abort,
            warnings: std::mem::take(&mut self.pending_warnings),
        })
    }

    fn run_app<S: KeyEventSource>(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        input: &S,
        demo: Option<&DemoInputSource>,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        // Redraw cadence is short enough that the typewriter / fade
        // reveal (#19/#20/#21) renders smoothly (~33 fps). The actual
        // input read happens on a separate thread (#22) so a player who
        // already knows the answer can begin typing during the reveal
        // without waiting for the next redraw tick.
        const REDRAW: Duration = Duration::from_millis(30);

        // Track which question the demo has already been primed for, so
        // we re-prime exactly once per question (right after the reveal
        // anchor is established for the new question).
        let mut demo_primed_for: Option<usize> = None;

        loop {
            terminal.draw(|f| self.ui(f))?;

            // Demo: keep the synthetic input source pointed at the
            // currently-active question. We re-prime whenever the
            // question index changes; once the buffer is consumed the
            // session will auto-confirm and move on naturally via the
            // existing correct-answer path.
            if let Some(demo) = demo {
                if self.phase == Phase::Playing {
                    let (current_idx, _) = self.quiz_game.get_progress();
                    // Defer priming until the question-text reveal has finished
                    // animating, so `--demo-wait-ms` measures the gap *after*
                    // the text is fully visible instead of starting in
                    // parallel with the typewriter. Without this, short
                    // questions consumed most of the wait window during
                    // reveal — visually "typing starts before the text is
                    // done", and the gap shrank as questions got shorter.
                    let reveal_done = self
                        .reveal
                        .as_ref()
                        .map(|r| r.is_done(Instant::now()))
                        .unwrap_or(true);
                    if demo_primed_for != Some(current_idx) && reveal_done {
                        match self.demo_target_for_current_question() {
                            Some(target) => {
                                demo.set_target(&target);
                                demo_primed_for = Some(current_idx);
                            }
                            None => {
                                // R-2 フェイルセーフ (M-2): 該当
                                // question の打鍵対象が生成できない
                                // (ja_typings 未登録 + 漢字ラベルなど)。
                                // MultiplexedSource は human 入力を
                                // 待ち続けるため、無人だと無限ハングに
                                // 倒れる。session を即時中断する。
                                //
                                // 警告メッセージは alt screen 内で
                                // `eprintln!` すると次の redraw に
                                // 上書きされたり TUI を縦に汚すので、
                                // `pending_warnings` に積んで
                                // `run_with_demo` が alt screen を
                                // 抜けた後に呼び出し側で表示する。
                                self.pending_warnings.push(format!(
                                    "warn: demo skipped — no typing target for question (idx {current_idx}); aborting session"
                                ));
                                self.no_target_abort = true;
                                break;
                            }
                        }
                    }
                }
            }

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

            // Demo: in Summary / Naming phases, the session has nothing
            // left to do — exit immediately so the demo loop driver can
            // start the next run (or end if non-looping).
            if demo.is_some() && matches!(self.phase, Phase::Summary | Phase::NamingForRecord) {
                break;
            }
        }

        Ok(self.quiz_game.get_final_score())
    }

    /// Pick the string the auto-demo should type for the current
    /// question. Returns the first entry of
    /// `current_correct_typing_candidates()`, which is the same set the
    /// input validator (`is_valid_correct_typed_prefix`) checks against
    /// — so whatever we hand the demo here is guaranteed to be
    /// accepted by the prefix matcher.
    ///
    /// Returns `None` when no candidate exists (e.g. `ja_typings` empty
    /// and the JA label has no Hepburn conversion). The caller is
    /// expected to treat `None` as "ja_typings 未登録の問題" and abort
    /// the demo session — otherwise the synthetic source would type a
    /// string the validator rejects, causing an infinite mistype flash.
    ///
    /// Note (S-3/S-6): an earlier revision had an `en` label fallback
    /// (lowercase + whitespace stripped) so demo could at least "type
    /// something" when typing candidates were missing. That fallback
    /// was removed because the typed string then bypassed the
    /// validator's candidate list, producing exactly the infinite-loop
    /// failure mode the fallback was supposed to avoid.
    fn demo_target_for_current_question(&self) -> Option<String> {
        self.quiz_game
            .current_correct_typing_candidates()
            .into_iter()
            .find(|s| !s.is_empty())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Quit keys: Esc, and Ctrl+C as the standard terminal escape.
        // Printable characters (including 'q') are part of typed selection
        // and must reach the buffer.
        //
        // S-1: record the explicit user abort so `run_with_demo` can
        // forward the signal up to the `--demo-loop` driver and break
        // the outer loop, rather than restarting another session right
        // after the user already asked to leave.
        if matches!(key.code, KeyCode::Esc) {
            self.user_aborted = true;
            return true;
        }
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.user_aborted = true;
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
                    // M-2: alt screen 内で `eprintln!` すると次の redraw
                    // で隠れたり画面が縦に汚れるので、警告を
                    // `pending_warnings` に積んで run path 終了後に
                    // 呼び出し側で表示する。Esc / 次の Enter で
                    // session を抜ける動作は維持する。
                    self.pending_warnings
                        .push(format!("warning: failed to save records: {err}"));
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
        // Issue #97: strip Markdown inline-code backticks before handing
        // the text to the reveal handle, but remember where the code
        // spans live (in grapheme indices) so `question_reveal_line` can
        // restyle them on the way out. The typing-match path is not
        // routed through here — backticks don't appear in `ja_typings`
        // or in `current_correct_typing_candidates`, so input validation
        // is unaffected.
        let mut next_code_ranges: Vec<(usize, usize)> = Vec::new();
        self.reveal = self.quiz_game.get_current_question().map(|question| {
            let text = self.quiz_game.get_question_text(question);
            let (stripped, ranges) = inline_code::strip_and_locate(&text);
            next_code_ranges = ranges;
            RevealHandle::start_at(&stripped, RevealOpts::default_quiz(), now)
        });
        self.code_ranges = next_code_ranges;
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
        // Per Issue #70 only the correct answer's typings are accepted.
        // Issue #94: a mistype flashes red but the input buffer is *kept*
        // at the last valid prefix; clearing it back to zero forced the
        // player to retype a long word from scratch for a single slip,
        // which made long answers feel hostile. The buffer is already at
        // the last valid prefix at this point (we only push validated
        // characters), so leaving it alone is the correct behaviour.
        if !self.quiz_game.is_valid_correct_typed_prefix(&attempted) {
            self.note_rejected_char(c);
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

        // Auto-confirm on canonical equality so IME 別経路 (記号 `/`,
        // 拗音 `kixya`, 促音 `ltsu`, ファ系 `huxa` 等) も最後のキーで
        // 自動確定する。旧実装は raw 等値 だけで判定していたため、
        // prefix 入力は通っているのに最後で確定しない症状 (`burendann/aiku`、
        // `rokeltsuto` 等) が発生していた。
        let typed = self.input_buffer.to_lowercase();
        if self.quiz_game.is_complete_correct_typed(&typed) {
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
                // Issue #97: keep the per-grapheme reveal color so the
                // fade-in/typewriter animation still plays out, but mark
                // inline-code graphemes with the Bold modifier so the
                // code span is visually distinct from the start. Once
                // the reveal is fully settled, the fg color of code
                // graphemes is replaced with `INLINE_CODE_COLOR` for a
                // stronger highlight in the steady-state view.
                //
                // Followup idea (#97): interpolate the code fg toward
                // `INLINE_CODE_COLOR` over the final fade window so the
                // settle reads as a deliberate "pop" instead of a hard
                // swap. Out of scope for the initial implementation.
                let settled = reveal.is_done(Instant::now());
                let spans: Vec<Span<'static>> = snapshot
                    .into_iter()
                    .enumerate()
                    .map(|(i, g)| {
                        let in_code = self.code_ranges.iter().any(|&(s, e)| i >= s && i < e);
                        let style = if in_code {
                            let fg = if settled {
                                INLINE_CODE_COLOR
                            } else {
                                let Rgb(r, gc, b) = g.color;
                                Color::Rgb(r, gc, b)
                            };
                            Style::new().fg(fg).add_modifier(Modifier::BOLD)
                        } else {
                            let Rgb(r, gc, b) = g.color;
                            Style::new().fg(Color::Rgb(r, gc, b))
                        };
                        Span::styled(g.text, style)
                    })
                    .collect();
                return Line::from(spans);
            }
        }
        // Fallback path (no active reveal): parse the raw text directly
        // so backticks are stripped and code spans get the inline-code
        // style applied.
        let text = self.quiz_game.get_question_text(question);
        Line::from(spans_from_inline_code(&text, STYLE_NORMAL))
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
                    // Issue #97: parse Markdown inline-code in the choice
                    // text so backticks are stripped and `code` spans get
                    // the inline-code highlight. Non-code text inherits
                    // the choices-fade `text_style` so the fade-in still
                    // works.
                    let mut line_spans = vec![Span::styled(format!("{label}) "), label_style)];
                    line_spans.extend(spans_from_inline_code(&choice, text_style));
                    Some(ListItem::new(Line::from(line_spans)))
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

/// Build a `Vec<Span>` for `text`, stripping Markdown inline-code
/// backticks and styling each code span with `INLINE_CODE_COLOR` + Bold
/// while keeping non-code text styled with `base_style` (Issue #97).
/// Used by both the question fallback path (no active reveal) and the
/// choices renderer.
fn spans_from_inline_code(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let segments = inline_code::parse_inline_code(text);
    // Empty input → empty span list. `Line::from(vec![])` and
    // `List::new` both tolerate an empty Vec, so we don't need to emit a
    // placeholder empty-text span just to keep callers happy.
    if segments.is_empty() {
        return Vec::new();
    }
    segments
        .into_iter()
        .map(|seg| {
            if seg.is_code {
                Span::styled(
                    seg.text,
                    Style::new()
                        .fg(INLINE_CODE_COLOR)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(seg.text, base_style)
            }
        })
        .collect()
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
    fn spans_from_inline_code_strips_backticks() {
        // Issue #97: backticks must not leak into the rendered spans.
        let spans = spans_from_inline_code("HTMLの `alt` 属性", STYLE_NORMAL);
        let all_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !all_text.contains('`'),
            "backtick leaked into rendered spans: {all_text}"
        );
        assert_eq!(all_text, "HTMLの alt 属性");
        // The code segment must carry the inline-code style.
        let code_span = spans.iter().find(|s| s.content.as_ref() == "alt").unwrap();
        assert_eq!(code_span.style.fg, Some(INLINE_CODE_COLOR));
        assert!(code_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn spans_from_inline_code_plain_text_unchanged() {
        let spans = spans_from_inline_code("just text", STYLE_NORMAL);
        let all_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(all_text, "just text");
        assert!(spans.iter().all(|s| s.style.fg == STYLE_NORMAL.fg));
    }

    #[test]
    fn spans_from_inline_code_multiple_spans() {
        let spans = spans_from_inline_code("use `let` and `mut` here", STYLE_NORMAL);
        let all_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(!all_text.contains('`'));
        assert_eq!(all_text, "use let and mut here");
        let code_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.style.fg == Some(INLINE_CODE_COLOR))
            .collect();
        assert_eq!(code_spans.len(), 2);
    }

    /// Build a single-question `QuizUI` for the demo-target fallback tests.
    /// `ja_typings`, `ja` label, `en` label の任意組合せを差し込んで
    /// `demo_target_for_current_question()` の挙動を直接観察する。
    fn make_quiz_ui_with_choice(
        ja_label: &str,
        en_label: &str,
        ja_typings: Vec<String>,
        language: Language,
    ) -> QuizUI {
        use crate::types::{Choice, Question};
        use std::collections::HashMap;

        let mut labels = HashMap::new();
        if !ja_label.is_empty() {
            labels.insert("ja".to_string(), ja_label.to_string());
        }
        if !en_label.is_empty() {
            labels.insert("en".to_string(), en_label.to_string());
        }
        let correct = Choice { labels, ja_typings };
        // Add a second dummy choice so multiple-choice display still works
        // in case any code path peeks at choices.len(); not strictly
        // required for the target lookup which only reads
        // correct_answer_index.
        let dummy = Choice {
            labels: HashMap::from([("ja".to_string(), "dummy".to_string())]),
            ja_typings: vec!["dummy".to_string()],
        };

        let mut question_text = HashMap::new();
        question_text.insert("ja".to_string(), "テスト".to_string());
        question_text.insert("en".to_string(), "test".to_string());

        let question = Question {
            id: "q-demo-fallback".into(),
            genre: "test".into(),
            question_text,
            question_text_reading: HashMap::new(),
            choices: vec![correct, dummy],
            correct_answer_index: 0,
            image_path: None,
            ja_reviewed: false,
        };

        QuizUI::from_pool_with_count(&[question], language, "/tmp/records-demo.yaml".into(), 1)
    }

    #[test]
    fn demo_target_returns_none_when_ja_typings_empty_and_label_not_convertible() {
        // S-3/S-6: 漢字のみの ja ラベル + ja_typings 空 のとき、
        // `current_correct_typing_candidates` は empty を返す。
        // 旧実装は en ラベルから「打鍵だけ進む」フォールバックを生成
        // していたが、その文字列は validator の candidate list には
        // 含まれず無限ミスタイプフラッシュの原因になっていた。
        // 現実装ではこのケースを None として返し、run_with_demo 側で
        // session abort に倒す。
        let ui = make_quiz_ui_with_choice("漢字", "Hello World", Vec::new(), Language::Japanese);
        assert!(
            ui.demo_target_for_current_question().is_none(),
            "ja_typings 空 + 変換不能ラベルは fallback せず None"
        );
    }

    #[test]
    fn demo_target_returns_none_when_both_typings_and_en_empty() {
        // S-3/S-6: ja_typings 空 + ja は変換不能の漢字のみ + en ラベル空 =
        // どこからも打鍵対象を取り出せない。run_with_demo 側でこれを
        // 検出して session を中断する。
        let ui = make_quiz_ui_with_choice("漢字", "", Vec::new(), Language::Japanese);
        assert!(
            ui.demo_target_for_current_question().is_none(),
            "no usable target → None"
        );
    }

    #[test]
    fn handle_key_esc_sets_user_aborted_flag() {
        // S-7: pressing Esc must flag `user_aborted` so run_with_demo
        // propagates the abort up to the --demo-loop driver, which can
        // then break the outer loop rather than restarting another
        // session right after the user asked to leave.
        let mut ui = make_quiz_ui_with_choice(
            "東京",
            "Tokyo",
            vec!["tokyo".to_string()],
            Language::Japanese,
        );
        assert!(!ui.user_aborted, "user_aborted starts false");
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let quit = ui.handle_key(esc);
        assert!(quit, "Esc must request quit");
        assert!(ui.user_aborted, "Esc must record user abort");
    }

    #[test]
    fn handle_key_ctrl_c_sets_user_aborted_flag() {
        // S-7: Ctrl+C is the other documented quit binding; it must
        // set the same flag so the loop driver treats it identically.
        let mut ui = make_quiz_ui_with_choice(
            "東京",
            "Tokyo",
            vec!["tokyo".to_string()],
            Language::Japanese,
        );
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let quit = ui.handle_key(ctrl_c);
        assert!(quit, "Ctrl+C must request quit");
        assert!(ui.user_aborted, "Ctrl+C must record user abort");
    }

    #[test]
    fn demo_target_uses_ja_typings_when_registered() {
        // ja_typings が登録されていれば通常パスでそれを採用する。
        // S-3/S-6 の fallback 撤廃でも通常パスは壊れていないことを保証。
        let ui = make_quiz_ui_with_choice(
            "東京",
            "Tokyo",
            vec!["tokyo".to_string()],
            Language::Japanese,
        );
        let target = ui
            .demo_target_for_current_question()
            .expect("ja_typings path must yield a target");
        assert_eq!(target, "tokyo");
    }

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
