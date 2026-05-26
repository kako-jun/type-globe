use crate::game::{Ta25LocalGame, Ta25SeatColor};
use crate::io::Storage;
use crate::types::TimeEntry;
use crate::ui::timestamp::now_rfc3339;
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
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const INPUT_REJECT_FLASH_MS: u64 = 180;
const REDRAW_TICK: Duration = Duration::from_millis(50);
/// Same upper bound as `QuizUI` (`NAME_MAX_CHARS`) so Records rows from
/// both modes line up under the 16-wide column in `ui/records.rs`.
const NAME_MAX_CHARS: usize = 16;

/// High-level state machine for one TA25 session. Mirrors `QuizUI::Phase`:
/// the 25 panels play out, the player sees a summary, and on Enter they
/// can stamp the run into Records (`time_attack_25`) — or Esc to skip
/// the save and head back to the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Playing,
    Summary,
    NamingForRecord,
}

pub struct TimeAttack25UI {
    game: Ta25LocalGame,
    input_buffer: String,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
    phase: Phase,
    name_buffer: String,
    /// Absolute path of `records_<lang>.yaml`. Passed in by `main.rs` so
    /// the UI never has to know the disk layout.
    records_file_path: String,
    /// Once the run's `TimeEntry` has been pushed and saved, a second
    /// Enter dismisses the confirmation screen instead of writing a
    /// duplicate row.
    saved: bool,
    /// Warnings collected during the run that must be surfaced to the
    /// user *after* the alt screen has been torn down (mirrors the same
    /// pattern in `QuizUI`). Used for `persist_record` disk-write
    /// failures so the message survives in the user's scrollback.
    pending_warnings: Vec<String>,
}

impl TimeAttack25UI {
    pub fn new(game: Ta25LocalGame, records_file_path: String) -> Self {
        Self {
            game,
            input_buffer: String::new(),
            rejected_char: None,
            reject_flash_until: None,
            phase: Phase::Playing,
            name_buffer: String::new(),
            records_file_path,
            saved: false,
            pending_warnings: Vec::new(),
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

        // Mirror `QuizUI::run`: flush deferred warnings only after the
        // alt screen is gone so any persist_record failure survives in
        // the user's scrollback instead of being painted over by the
        // next redraw.
        for w in self.pending_warnings.drain(..) {
            eprintln!("{w}");
        }

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        input: &impl KeyEventSource,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // CPU bots only advance while the panels are still being
            // played out. Once the 25th panel resolves the game freezes
            // its `finished_elapsed`, so polling further would be a
            // no-op; skipping it keeps the summary / name-entry phases
            // visually still.
            if self.phase == Phase::Playing {
                self.game.poll_cpu(Instant::now());
                // The game itself flips to `is_finished()` the moment
                // the final panel is claimed. Promote the UI to Summary
                // here (rather than only on the next keypress) so the
                // closing CPU capture transitions straight into the
                // results screen without an extra frame of stale board
                // rendering.
                self.promote_if_finished();
            }
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
        // Ctrl+C is always a global quit, matching `QuizUI`. Esc is
        // routed phase-by-phase so the player can skip Records save
        // (Summary / Naming) without it doubling as "quit immediately".
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        match self.phase {
            Phase::Playing => self.handle_key_playing(key),
            Phase::Summary => self.handle_key_summary(key),
            Phase::NamingForRecord => self.handle_key_naming(key),
        }
    }

    fn handle_key_playing(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Esc) {
            // Bail out of the run entirely. The session is unsaved by
            // design — only completed runs are eligible for Records.
            return true;
        }

        match key.code {
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Tab => {
                self.input_buffer.clear();
                self.game.forfeit_current_round(Instant::now());
                if self.game.is_finished() {
                    self.phase = Phase::Summary;
                    self.clear_reject_flash();
                }
            }
            KeyCode::Char(ch) => {
                self.push_input_char(ch);
                if self.game.is_finished() {
                    self.phase = Phase::Summary;
                    self.input_buffer.clear();
                    self.clear_reject_flash();
                }
            }
            _ => {}
        }

        false
    }

    fn handle_key_summary(&mut self, key: KeyEvent) -> bool {
        // Match `QuizUI::handle_key_summary`: Enter advances to name
        // entry, Esc skips the save and returns to the menu.
        match key.code {
            KeyCode::Enter => {
                self.phase = Phase::NamingForRecord;
                self.name_buffer.clear();
                false
            }
            KeyCode::Esc => true,
            _ => false,
        }
    }

    fn handle_key_naming(&mut self, key: KeyEvent) -> bool {
        if self.saved {
            // After a successful save the name is locked in. Treat
            // Backspace and other editing keys as no-ops so the player
            // cannot mutate the already-persisted record.
            //
            // Esc is intentionally excluded from the dismiss set to
            // match `QuizUI::handle_key_naming`: once saved, Esc would
            // mean two things at once ("skip the save" and "dismiss"),
            // so we keep it inert. Enter / printable keys dismiss the
            // confirmation screen back to the menu.
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(_)) {
                return true;
            }
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                // Explicit "skip Records save" — leaves the run
                // unrecorded and returns to the menu.
                true
            }
            KeyCode::Enter => {
                if self.name_buffer.trim().is_empty() {
                    return false;
                }
                if let Err(err) = self.persist_record() {
                    // Defer the error to `run`'s post-alt-screen flush
                    // so the message survives in the user's scrollback
                    // instead of being repainted away.
                    self.pending_warnings
                        .push(format!("warning: failed to save records: {err}"));
                    return false;
                }
                self.saved = true;
                false
            }
            KeyCode::Backspace => {
                self.name_buffer.pop();
                false
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && self.name_buffer.chars().count() < NAME_MAX_CHARS
                    // #122: drop a leading Space (mirror `QuizUI`). The
                    // buffer is only `trim()`-ed at save time, so a name
                    // starting with whitespace made typed-vs-saved char
                    // counts diverge against NAME_MAX_CHARS. Mid/trailing
                    // spaces stay allowed; the save-time trim absorbs them.
                    && !(c == ' ' && self.name_buffer.is_empty()) =>
            {
                self.name_buffer.push(c);
                false
            }
            _ => false,
        }
    }

    /// Persist the just-finished run as a `TimeEntry` row in the local
    /// Records file. Mirrors `QuizUI::persist_record` so both modes use
    /// the same load → push → save pattern and end up reading back
    /// identically in `ui/records.rs`.
    ///
    /// `time_seconds` is taken from `game.elapsed(now)` which the game
    /// freezes to `finished_elapsed` as soon as the 25th panel resolves
    /// — so the value is stable regardless of how long the player
    /// lingers on the Summary / Naming screens.
    fn persist_record(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut records = Storage::load_records(&self.records_file_path)?;
        let elapsed = self.game.elapsed(Instant::now());
        let entry = TimeEntry {
            name: self.name_buffer.trim().to_string(),
            time_seconds: elapsed.as_secs() as u32,
            ts: now_rfc3339(),
        };
        records.push_ta25(entry);
        Storage::save_records(&self.records_file_path, &records)?;
        Ok(())
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

    /// If we are still in Playing but the game has just finished, flip
    /// the phase to Summary and tidy up the Playing-only state (input
    /// echo / reject flash). Idempotent: if the phase is already past
    /// Playing this is a no-op, so `run_app` can call it on every tick
    /// and tests can call it directly without bookkeeping.
    fn promote_if_finished(&mut self) {
        if self.phase == Phase::Playing && self.game.is_finished() {
            self.phase = Phase::Summary;
            self.input_buffer.clear();
            self.clear_reject_flash();
        }
    }

    /// Test-only thin wrapper around the `run_app` top-of-loop tick.
    /// `run_app` itself requires a live `Terminal<CrosstermBackend>` and
    /// is therefore not exercisable from unit tests, so this helper lets
    /// the test module drive `poll_cpu → promote_if_finished` without
    /// reaching into `Ta25LocalGame` directly.
    #[cfg(test)]
    pub(super) fn tick_for_test(&mut self) {
        if self.phase == Phase::Playing {
            self.game.poll_cpu(Instant::now());
            self.promote_if_finished();
        }
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

        match self.phase {
            // The Summary card lives in the question slot — same
            // dimensions, just different content. Naming reuses the
            // same slot so the board stays visible and the player can
            // glance at their finished panel layout while typing in a
            // name.
            Phase::Summary => self.render_summary(f, chunks[0]),
            Phase::NamingForRecord => self.render_naming(f, chunks[0]),
            Phase::Playing => self.render_question(f, chunks[0]),
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
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to save your record (Esc to skip).",
            STYLE_NORMAL,
        )));
        let block = Block::default()
            .title(Span::styled(" Summary ", STYLE_TITLE))
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_naming(&self, f: &mut Frame, area: Rect) {
        let lines = if self.saved {
            vec![
                Line::from(Span::styled(" Record saved. ", STYLE_TITLE)),
                Line::from(""),
                Line::from(format!("  Name : {}", self.name_buffer.trim())),
                Line::from(format!(
                    "  Time : {}",
                    format_time(self.game.elapsed(Instant::now()))
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to return to the menu.",
                    STYLE_NORMAL,
                )),
            ]
        } else {
            vec![
                Line::from(Span::styled(" Records entry ", STYLE_TITLE)),
                Line::from(""),
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
        let block = Block::default()
            .title(Span::styled(" Records ", STYLE_TITLE))
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
        // Mirror `QuizUI::help_line` (src/ui/quiz.rs:966-987): drive the
        // help footer from `self.phase` (plus `saved` while in Naming)
        // so each screen advertises only the keys that actually do
        // something on it. The previous `is_finished()` switch lumped
        // Summary and Naming together and hid the Enter/Esc choices.
        let help = match self.phase {
            Phase::Playing => HelpLine::new(vec![
                HelpEntry::new("Backspace", "Erase"),
                HelpEntry::new("Tab", "Skip panel"),
                HelpEntry::new("Esc", "Quit"),
            ]),
            Phase::Summary => HelpLine::new(vec![
                HelpEntry::new("Enter", "Save record"),
                HelpEntry::new("Esc", "Skip & menu"),
            ]),
            Phase::NamingForRecord if self.saved => HelpLine::new(vec![
                // After save, Esc is intentionally inert (see
                // `handle_key_naming`), so advertise the keys that
                // actually dismiss the confirmation screen.
                HelpEntry::new("Enter", "Menu"),
                HelpEntry::new("Ctrl+C", "Quit"),
            ]),
            Phase::NamingForRecord => HelpLine::new(vec![
                HelpEntry::new("Enter", "Save"),
                HelpEntry::new("Esc", "Skip"),
                HelpEntry::new("Backspace", "Edit"),
            ]),
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
    use tempfile::tempdir;

    /// Return `(TempDir, path)` where `path` points to a file inside the
    /// tempdir that does **not yet exist**. We can't use
    /// `NamedTempFile` here because it leaves a zero-byte file on disk,
    /// and `Storage::load_records` insists on a valid YAML document if
    /// the path exists (the empty-file case routes to deserialize and
    /// fails). Using a non-existent path triggers the
    /// "return Records::default()" fast path on first load.
    fn fresh_records_path() -> (tempfile::TempDir, String) {
        let dir = tempdir().expect("tempdir");
        let path = dir
            .path()
            .join("records.yaml")
            .to_string_lossy()
            .to_string();
        (dir, path)
    }

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
        TimeAttack25UI::new(game, String::new())
    }

    /// Variant of [`make_ui`] that points `records_file_path` at a real
    /// file on disk so persist-record paths can be exercised end-to-end.
    fn make_ui_with_records_path(path: String) -> TimeAttack25UI {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let game =
            Ta25LocalGame::from_pool(&pool, crate::types::Language::English, "You").expect("game");
        TimeAttack25UI::new(game, path)
    }

    /// Force the game to its terminal state by forfeiting every remaining
    /// round. `forfeit_current_round` is a no-op once `is_finished()` is
    /// true, so this is safe to over-call.
    fn finish_game(ui: &mut TimeAttack25UI) {
        while !ui.game.is_finished() {
            ui.game.forfeit_current_round(Instant::now());
        }
    }

    /// Bring the game one panel away from finishing, leaving the UI in
    /// Playing phase. Used by tests that need to exercise the "last
    /// panel resolves" promotion path through a single key press.
    fn play_until_last_panel(ui: &mut TimeAttack25UI) {
        // 25 panels total → forfeit 24 so the 25th remains open. The
        // TA25 run length is a fixed product constant; mirroring it as
        // a literal keeps the test independent of game-internal paths.
        for _ in 0..24 {
            ui.game.forfeit_current_round(Instant::now());
        }
        assert!(!ui.game.is_finished(), "setup: game should still be live");
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn rejects_wrong_prefix_and_clears_buffer() {
        let mut ui = make_ui();
        ui.input_buffer = "alp".to_string();
        ui.push_input_char('z');
        assert_eq!(ui.input_buffer, "alp");
        assert_eq!(ui.rejected_char, Some('z'));
    }

    /// Helper: render `render_help_line` to a 80×1 TestBackend and dump
    /// the row as a `String`. The 80-cell width matches the widest help
    /// hint string (`[Backspace] Erase  [Tab] Skip panel  [Esc] Quit`).
    fn render_help_to_string(ui: &TimeAttack25UI) -> String {
        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| ui.render_help_line(f, Rect::new(0, 0, 80, 1)))
            .unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for x in 0..buf.area.width {
            out.push_str(buf[(x, 0)].symbol());
        }
        out
    }

    #[test]
    fn help_line_switches_per_phase() {
        // Playing: Backspace / Tab / Esc, but no Enter (typing the
        // answer auto-confirms — there is no Enter affordance).
        let ui_playing = make_ui();
        let out = render_help_to_string(&ui_playing);
        assert!(
            out.contains("[Backspace]"),
            "playing missing Backspace: {out}"
        );
        assert!(out.contains("[Tab]"), "playing missing Tab: {out}");
        assert!(out.contains("[Esc]"), "playing missing Esc: {out}");
        assert!(
            !out.contains("[Enter]"),
            "playing must not advertise Enter: {out}"
        );

        // Summary: Enter saves, Esc skips.
        let mut ui_summary = make_ui();
        finish_game(&mut ui_summary);
        ui_summary.phase = Phase::Summary;
        let out = render_help_to_string(&ui_summary);
        assert!(out.contains("[Enter]"), "summary missing Enter: {out}");
        assert!(out.contains("[Esc]"), "summary missing Esc: {out}");

        // NamingForRecord (saved=false): Enter / Esc / Backspace.
        let mut ui_naming = make_ui();
        finish_game(&mut ui_naming);
        ui_naming.phase = Phase::NamingForRecord;
        let out = render_help_to_string(&ui_naming);
        assert!(out.contains("[Enter]"), "naming missing Enter: {out}");
        assert!(out.contains("[Esc]"), "naming missing Esc: {out}");
        assert!(
            out.contains("[Backspace]"),
            "naming missing Backspace: {out}"
        );

        // NamingForRecord (saved=true): Enter dismisses; Esc no-op so
        // it must NOT be advertised.
        let mut ui_saved = make_ui();
        finish_game(&mut ui_saved);
        ui_saved.phase = Phase::NamingForRecord;
        ui_saved.saved = true;
        let out = render_help_to_string(&ui_saved);
        assert!(out.contains("[Enter]"), "saved missing Enter: {out}");
        assert!(out.contains("[Ctrl+C]"), "saved missing Ctrl+C hint: {out}");
        assert!(
            !out.contains("[Esc]"),
            "saved must not advertise Esc: {out}"
        );
    }

    // -------------------------------------------------------------------
    // #1: auto-promote inside the run loop
    // -------------------------------------------------------------------
    #[test]
    fn test_auto_promote_to_summary_on_finish() {
        let mut ui = make_ui();
        // Seed leftover Playing-phase residue that the auto-promote
        // must clean up.
        ui.input_buffer = "alp".to_string();
        ui.rejected_char = Some('z');
        ui.reject_flash_until = Some(Instant::now() + Duration::from_secs(60));
        finish_game(&mut ui);
        assert_eq!(ui.phase, Phase::Playing, "precondition: still Playing");

        ui.tick_for_test();

        assert_eq!(ui.phase, Phase::Summary);
        assert!(ui.input_buffer.is_empty());
        assert!(ui.reject_flash_until.is_none());
        assert!(ui.rejected_char.is_none());
    }

    // -------------------------------------------------------------------
    // #2: Char input that resolves the 25th panel promotes immediately
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_playing_promotes_to_summary_when_last_panel_resolves() {
        let mut ui = make_ui();
        play_until_last_panel(&mut ui);

        // "alpha" is the correct typing for sample_question — submitting
        // it claims the final panel as Red and finishes the game.
        for ch in "alpha".chars() {
            let quit = ui.handle_key(key(KeyCode::Char(ch)));
            assert!(!quit, "char input must not quit during play");
        }

        assert!(ui.game.is_finished(), "final panel must resolve");
        assert_eq!(ui.phase, Phase::Summary);
        assert!(ui.input_buffer.is_empty());
        assert!(ui.reject_flash_until.is_none());
    }

    // -------------------------------------------------------------------
    // #3: Tab forfeit on the last round promotes to Summary
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_playing_tab_forfeit_promotes_to_summary_on_finish() {
        let mut ui = make_ui();
        play_until_last_panel(&mut ui);

        let quit = ui.handle_key(key(KeyCode::Tab));
        assert!(!quit);
        assert!(ui.game.is_finished());
        assert_eq!(ui.phase, Phase::Summary);
        assert!(ui.input_buffer.is_empty());
    }

    // -------------------------------------------------------------------
    // #4: Summary + Enter → NamingForRecord + buffer clear
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_summary_enter_transitions_to_naming_and_clears_buffer() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::Summary;
        ui.name_buffer = "stale".to_string();

        let quit = ui.handle_key(key(KeyCode::Enter));

        assert!(!quit);
        assert_eq!(ui.phase, Phase::NamingForRecord);
        assert!(ui.name_buffer.is_empty());
    }

    // -------------------------------------------------------------------
    // #5: Summary + Esc → quit
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_summary_esc_returns_quit() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::Summary;

        assert!(ui.handle_key(key(KeyCode::Esc)));
    }

    // -------------------------------------------------------------------
    // #6: Summary ignores Char / Backspace / Tab
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_summary_ignores_other_keys() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::Summary;

        for code in [KeyCode::Char('x'), KeyCode::Backspace, KeyCode::Tab] {
            let quit = ui.handle_key(key(code));
            assert!(!quit, "{code:?} should not quit");
            assert_eq!(ui.phase, Phase::Summary, "{code:?} should not change phase");
        }
    }

    // -------------------------------------------------------------------
    // #7: Naming + Enter persists record and sets saved=true
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_enter_persists_record_and_sets_saved() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        let quit = ui.handle_key(key(KeyCode::Enter));

        assert!(!quit);
        assert!(
            ui.saved,
            "saved flag should be true after successful persist"
        );
        assert!(ui.pending_warnings.is_empty());
        let loaded = Storage::load_records(&path).expect("load records");
        assert_eq!(loaded.time_attack_25.len(), 1);
        assert_eq!(loaded.time_attack_25[0].name, "Alice");
    }

    // -------------------------------------------------------------------
    // #8: Naming accepts up to NAME_MAX_CHARS=16, rejects the 17th
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_accepts_up_to_16_chars_then_rejects() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;

        for ch in "abcdefghijklmnop".chars() {
            assert!(!ui.handle_key(key(KeyCode::Char(ch))));
        }
        assert_eq!(ui.name_buffer.chars().count(), NAME_MAX_CHARS);

        let before = ui.name_buffer.clone();
        let _ = ui.handle_key(key(KeyCode::Char('q')));
        assert_eq!(ui.name_buffer, before, "17th char must be ignored");
    }

    // -------------------------------------------------------------------
    // #122: a leading Space is dropped; mid / trailing spaces are kept.
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_rejects_leading_space_keeps_inner() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;

        // Space on an empty buffer is dropped — no leading whitespace.
        let _ = ui.handle_key(key(KeyCode::Char(' ')));
        assert!(
            ui.name_buffer.is_empty(),
            "leading Space must not enter the buffer"
        );
        // Repeated leading Spaces stay dropped (buffer never leaves empty).
        let _ = ui.handle_key(key(KeyCode::Char(' ')));
        assert!(ui.name_buffer.is_empty(), "second leading Space dropped");

        // Once there is content, an inner Space is accepted...
        for ch in "ab".chars() {
            let _ = ui.handle_key(key(KeyCode::Char(ch)));
        }
        let _ = ui.handle_key(key(KeyCode::Char(' ')));
        let _ = ui.handle_key(key(KeyCode::Char('c')));
        // ...and a trailing Space is accepted too (trim absorbs it on save).
        let _ = ui.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(ui.name_buffer, "ab c ", "inner/trailing spaces kept");

        // Backspace all the way to empty, then a Space is dropped again —
        // the guard reads current state every keypress, it is not a
        // one-shot "first character" flag.
        for _ in 0..ui.name_buffer.chars().count() {
            let _ = ui.handle_key(key(KeyCode::Backspace));
        }
        assert!(ui.name_buffer.is_empty(), "buffer emptied via Backspace");
        let _ = ui.handle_key(key(KeyCode::Char(' ')));
        assert!(
            ui.name_buffer.is_empty(),
            "Space after Backspace-to-empty must be dropped"
        );
    }

    // -------------------------------------------------------------------
    // #9: chars().count() counts grapheme-ish, not bytes — JP works
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_counts_japanese_chars_by_grapheme_not_bytes() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;

        // 16 Japanese characters → each ~3 bytes in UTF-8, but the
        // limit is per-`char`, so all 16 must fit.
        let jp: Vec<char> = "あいうえおかきくけこさしすせそた".chars().collect();
        assert_eq!(jp.len(), NAME_MAX_CHARS);
        for ch in &jp {
            assert!(!ui.handle_key(key(KeyCode::Char(*ch))));
        }
        assert_eq!(ui.name_buffer.chars().count(), NAME_MAX_CHARS);

        // 17th JP char rejected, byte length irrelevant.
        let _ = ui.handle_key(key(KeyCode::Char('ち')));
        assert_eq!(ui.name_buffer.chars().count(), NAME_MAX_CHARS);
    }

    // -------------------------------------------------------------------
    // #10: empty name + Enter is a no-op (does not persist, stays in Naming)
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_enter_empty_name_is_noop() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer.clear();

        let quit = ui.handle_key(key(KeyCode::Enter));
        assert!(!quit);
        assert!(!ui.saved);
        assert_eq!(ui.phase, Phase::NamingForRecord);
        // Nothing should have been written.
        let loaded = Storage::load_records(&path).expect("load");
        assert!(loaded.time_attack_25.is_empty());
    }

    // -------------------------------------------------------------------
    // #11: whitespace-only name + Enter is also a no-op
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_enter_whitespace_only_name_is_noop() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "   ".to_string();

        let quit = ui.handle_key(key(KeyCode::Enter));
        assert!(!quit);
        assert!(!ui.saved);
        assert_eq!(ui.phase, Phase::NamingForRecord);
        let loaded = Storage::load_records(&path).expect("load");
        assert!(loaded.time_attack_25.is_empty());
    }

    // -------------------------------------------------------------------
    // #12: Backspace pops last char in name_buffer
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_backspace_pops_last_char() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        let quit = ui.handle_key(key(KeyCode::Backspace));
        assert!(!quit);
        assert_eq!(ui.name_buffer, "Alic");
    }

    // -------------------------------------------------------------------
    // #13: Ctrl+A and Alt+B do NOT push into name_buffer
    //      (Ctrl+C is special-cased upstream as global quit — see #18.)
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_ignores_modified_char_keys() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;

        let _ = ui.handle_key(key_with(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert!(ui.name_buffer.is_empty(), "Ctrl+A must not push");
        let _ = ui.handle_key(key_with(KeyCode::Char('b'), KeyModifiers::ALT));
        assert!(ui.name_buffer.is_empty(), "Alt+B must not push");
    }

    // -------------------------------------------------------------------
    // #14: After saved=true, Enter dismisses (quit=true) and does NOT
    //      append another entry to the records file.
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_after_saved_enter_dismisses_without_re_persisting() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();
        // First Enter → persist & saved=true.
        assert!(!ui.handle_key(key(KeyCode::Enter)));
        assert!(ui.saved);
        let count_after_save = Storage::load_records(&path).unwrap().time_attack_25.len();

        // Second Enter → quit, no re-persist.
        let quit = ui.handle_key(key(KeyCode::Enter));
        assert!(quit);
        let count_after_dismiss = Storage::load_records(&path).unwrap().time_attack_25.len();
        assert_eq!(count_after_dismiss, count_after_save);
    }

    // -------------------------------------------------------------------
    // #15: After saved=true, Enter / Char dismiss (quit=true).
    //      Esc は意図的に除外（後続 #15b の no-op 観点で確認）。Quiz と
    //      挙動を揃え、saved 後の Esc は「save をスキップ」と「dismiss」
    //      の意味が二重化しないよう no-op にする。
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_after_saved_any_dismiss_key_returns_quit() {
        for dismiss_code in [KeyCode::Enter, KeyCode::Char('x')] {
            let mut ui = make_ui();
            finish_game(&mut ui);
            ui.phase = Phase::NamingForRecord;
            ui.saved = true;

            let quit = ui.handle_key(key(dismiss_code));
            assert!(quit, "{dismiss_code:?} should dismiss when saved=true");
        }
    }

    // -------------------------------------------------------------------
    // #15b: After saved=true, Esc is a no-op (quit=false).
    //      Mirrors `QuizUI::handle_key_naming` — Esc is reserved for
    //      "skip the save" during un-saved naming, so once saved it
    //      must not double as a dismiss key.
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_after_saved_esc_is_noop() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.saved = true;
        ui.name_buffer = "Alice".to_string();

        let quit = ui.handle_key(key(KeyCode::Esc));
        assert!(!quit, "Esc must not dismiss when saved=true");
        assert_eq!(ui.name_buffer, "Alice", "Esc must not mutate the buffer");
    }

    // -------------------------------------------------------------------
    // #16: After saved=true, Backspace is a no-op (quit=false).
    //      "要確認" 観点 → 現状の挙動を assertion で固定する。
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_after_saved_non_dismiss_key_is_noop() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.saved = true;
        ui.name_buffer = "Alice".to_string();

        let quit = ui.handle_key(key(KeyCode::Backspace));
        assert!(!quit, "Backspace must not dismiss when saved=true");
        // And it must NOT mutate the buffer either.
        assert_eq!(ui.name_buffer, "Alice");
    }

    // -------------------------------------------------------------------
    // #17: Esc during Naming (saved=false) skips the save and quits,
    //      leaving saved=false.
    // -------------------------------------------------------------------
    #[test]
    fn test_handle_key_naming_esc_skips_without_saving() {
        let mut ui = make_ui();
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();
        assert!(!ui.saved);

        let quit = ui.handle_key(key(KeyCode::Esc));
        assert!(quit);
        assert!(!ui.saved);
    }

    // -------------------------------------------------------------------
    // #18: Ctrl+C quits from every phase.
    // -------------------------------------------------------------------
    #[test]
    fn test_ctrl_c_quits_in_every_phase() {
        for phase in [Phase::Playing, Phase::Summary, Phase::NamingForRecord] {
            let mut ui = make_ui();
            if phase != Phase::Playing {
                finish_game(&mut ui);
            }
            ui.phase = phase;
            let quit = ui.handle_key(key_with(KeyCode::Char('c'), KeyModifiers::CONTROL));
            assert!(quit, "Ctrl+C must quit from {phase:?}");
        }
    }

    // -------------------------------------------------------------------
    // #19: persist_record failure → pending_warnings += 1, saved stays false
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_failure_appends_pending_warning_and_keeps_saved_false() {
        // Point at a file under a tempdir that has been dropped, so the
        // parent dir is guaranteed not to exist on whichever filesystem
        // CI happens to be running. This is more robust than a
        // hard-coded `/tmp/...nonexistent...` path because we can't
        // assume `/tmp` semantics (or that no prior run left an
        // identically-named dir behind).
        let bogus = {
            let tmp = tempdir().expect("tempdir");
            tmp.path()
                .join("records.yaml")
                .to_string_lossy()
                .to_string()
            // tmp drops here → directory disappears.
        };
        let mut ui = make_ui_with_records_path(bogus);
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        let quit = ui.handle_key(key(KeyCode::Enter));
        assert!(!quit, "failed persist must not quit");
        assert!(!ui.saved);
        assert_eq!(ui.pending_warnings.len(), 1);
        assert!(ui.pending_warnings[0].contains("warning"));
    }

    // -------------------------------------------------------------------
    // #20: repeated persist failure accumulates warnings (no dedupe).
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_repeated_failure_appends_multiple_warnings() {
        // Same drop-the-tempdir trick as #19 — see that test for why.
        let bogus = {
            let tmp = tempdir().expect("tempdir");
            tmp.path()
                .join("records.yaml")
                .to_string_lossy()
                .to_string()
        };
        let mut ui = make_ui_with_records_path(bogus);
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        let _ = ui.handle_key(key(KeyCode::Enter));
        let _ = ui.handle_key(key(KeyCode::Enter));

        assert_eq!(ui.pending_warnings.len(), 2);
        assert!(!ui.saved);
    }

    // -------------------------------------------------------------------
    // #21: persist_record trims surrounding whitespace from the name.
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_trims_name_whitespace() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "  Alice  ".to_string();

        assert!(!ui.handle_key(key(KeyCode::Enter)));
        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.time_attack_25.len(), 1);
        assert_eq!(loaded.time_attack_25[0].name, "Alice");
    }

    // -------------------------------------------------------------------
    // #22: TimeEntry.time_seconds matches game.elapsed() in seconds.
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_writes_time_seconds_from_game_elapsed() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        let expected_secs = ui.game.elapsed(Instant::now()).as_secs() as u32;
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        assert!(!ui.handle_key(key(KeyCode::Enter)));

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.time_attack_25.len(), 1);
        // Game freezes `finished_elapsed` at finish, so the stored value
        // must equal the elapsed we captured pre-save.
        assert_eq!(loaded.time_attack_25[0].time_seconds, expected_secs);
    }

    // -------------------------------------------------------------------
    // #23: TimeEntry.ts is RFC3339 (YYYY-MM-DDTHH:MM:SSZ, 20 chars).
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_writes_rfc3339_timestamp() {
        let (_tmp, path) = fresh_records_path();
        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();

        assert!(!ui.handle_key(key(KeyCode::Enter)));

        let loaded = Storage::load_records(&path).expect("load");
        let ts = &loaded.time_attack_25[0].ts;
        assert_eq!(ts.len(), 20, "unexpected ts length: {ts}");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], "Z");
    }

    // -------------------------------------------------------------------
    // #24: pre-existing TA25 entries are preserved on append.
    // -------------------------------------------------------------------
    #[test]
    fn test_persist_record_preserves_existing_entries_and_pushes_new() {
        let dir = tempdir().expect("tempdir");
        let path = dir
            .path()
            .join("records.yaml")
            .to_string_lossy()
            .to_string();

        // Pre-populate the records file with an existing TA25 entry.
        let mut seed = crate::types::Records::default();
        seed.time_attack_25.push(TimeEntry {
            name: "Existing".into(),
            time_seconds: 999,
            ts: "2020-01-01T00:00:00Z".into(),
        });
        Storage::save_records(&path, &seed).expect("seed");

        let mut ui = make_ui_with_records_path(path.clone());
        finish_game(&mut ui);
        ui.phase = Phase::NamingForRecord;
        ui.name_buffer = "Alice".to_string();
        assert!(!ui.handle_key(key(KeyCode::Enter)));

        let loaded = Storage::load_records(&path).expect("load");
        assert_eq!(loaded.time_attack_25.len(), 2);
        let names: Vec<&str> = loaded
            .time_attack_25
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"Existing"));
        assert!(names.contains(&"Alice"));
    }

    // -------------------------------------------------------------------
    // #25: Constructor stores the supplied records_file_path verbatim.
    //      White-box regression for the `new(game, records_file_path)`
    //      shape so a future refactor cannot silently drop the field.
    // -------------------------------------------------------------------
    #[test]
    fn test_new_stores_records_file_path() {
        let pool = (1..40).map(sample_question).collect::<Vec<_>>();
        let game =
            Ta25LocalGame::from_pool(&pool, crate::types::Language::English, "You").expect("game");
        let ui = TimeAttack25UI::new(game, "/tmp/records-marker.yaml".to_string());
        assert_eq!(ui.records_file_path, "/tmp/records-marker.yaml");
    }

    // -------------------------------------------------------------------
    // #26: parametrised — Char and Tab input paths both lift the UI out
    //      of Playing as soon as the game finishes (no stale Playing).
    // -------------------------------------------------------------------
    #[test]
    fn test_no_stale_playing_phase_after_finish_via_each_input_path() {
        // Variant A: final panel resolved by typing the answer.
        {
            let mut ui = make_ui();
            play_until_last_panel(&mut ui);
            for ch in "alpha".chars() {
                let _ = ui.handle_key(key(KeyCode::Char(ch)));
            }
            assert!(ui.game.is_finished());
            assert_ne!(
                ui.phase,
                Phase::Playing,
                "Char path must not leave UI in Playing after finish"
            );
            assert_eq!(ui.phase, Phase::Summary);
        }
        // Variant B: final panel resolved by Tab forfeit.
        {
            let mut ui = make_ui();
            play_until_last_panel(&mut ui);
            let _ = ui.handle_key(key(KeyCode::Tab));
            assert!(ui.game.is_finished());
            assert_ne!(
                ui.phase,
                Phase::Playing,
                "Tab path must not leave UI in Playing after finish"
            );
            assert_eq!(ui.phase, Phase::Summary);
        }
    }
}
