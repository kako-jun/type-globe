//! Listening practice UI (#30 / #31).
//!
//! Single-prompt foundation that exercises the TTS engine (#28), the
//! listening data structure (#29), and the blind-input judge (#31)
//! end-to-end. The full hack-and-slash run (#32-#37) will compose
//! `ListeningSession` repeatedly inside this same 4-pane layout; for
//! now the side pane shows placeholder run info ("Practice — RPG run
//! lands in #32+").
//!
//! Per `docs/spec.md`, the audio is the only presentation: no text,
//! no choices. The visible elements are limited to:
//! - a pulsing `♪` (jiwa_core pulse) while audio is in flight,
//! - the input echo of what the player has typed,
//! - status placeholders (kind / Floor / Run time placeholder),
//! - a battle-log pane (used only on the result screen for v0.2.0).

use crate::audio::TtsEngine;
use crate::game::listening::{acceptable_listening_inputs, is_valid_listening_prefix};
use crate::game::{ListeningSession, SubmissionResult};
use crate::jiwa_core::{PulseHandle, PulseOpts, Rgb};
use crate::types::{AnswerKind, Language};
use crate::ui::{HelpEntry, HelpLine, InputChannel, PaneFrame, RecvOutcome};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_NORMAL: Style = Style::new().fg(Color::White);
const STYLE_DIM: Style = Style::new().fg(Color::DarkGray);
const STYLE_CORRECT: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_INCORRECT: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_INPUT_ECHO: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_LABEL: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const INPUT_REJECT_FLASH_MS: u64 = 180;

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Playing,
    Result,
}

pub struct ListenUI {
    session: ListeningSession,
    /// `None` when the caller passed `--no-tts` (#48).
    tts: Option<TtsEngine>,
    language: Language,
    phase: Phase,
    /// `♪` pulse for the active prompt — anchors per-frame color.
    /// `None` after submit; the result screen reveals the answer text
    /// instead of pulsing the symbol.
    pulse: Option<PulseHandle>,
    started_at: Instant,
    /// Number of times the player has triggered audio (initial play +
    /// each Space replay). Per spec there is no penalty, but exposing
    /// the count for the next UI iteration is cheap and mirrors what
    /// the hack-and-slash run is going to want for telemetry.
    plays: u32,
    rejected_char: Option<char>,
    reject_flash_until: Option<Instant>,
}

impl ListenUI {
    pub fn new(session: ListeningSession, tts: TtsEngine, language: Language) -> Self {
        Self {
            session,
            tts: Some(tts),
            language,
            phase: Phase::Playing,
            pulse: Some(PulseHandle::start("♪", PulseOpts::default_listening())),
            started_at: Instant::now(),
            plays: 0,
            rejected_char: None,
            reject_flash_until: None,
        }
    }

    /// Construct a `ListenUI` without a TTS engine. Audio calls are
    /// silently skipped. Activated by `rpg --no-tts` (#48).
    pub fn new_without_tts(session: ListeningSession, language: Language) -> Self {
        Self {
            session,
            tts: None,
            language,
            phase: Phase::Playing,
            pulse: Some(PulseHandle::start("♪", PulseOpts::default_listening())),
            started_at: Instant::now(),
            plays: 0,
            rejected_char: None,
            reject_flash_until: None,
        }
    }

    pub fn run(&mut self) -> Result<Option<SubmissionResult>, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Speak the prompt once on entry. Failure here is non-fatal —
        // the player can still try Space-replay, and the result screen
        // works even if no audio came out (helps debug TTS issues).
        if let Some(tts) = self.tts.as_mut() {
            if let Err(err) = tts.speak(&self.session.prompt().text, &self.language) {
                eprintln!("warning: initial TTS speak failed: {err}");
            } else {
                self.plays += 1;
            }
        }

        let result = self.run_app(&mut terminal);

        // Stop any in-flight utterance so the terminal returns silently.
        if let Some(tts) = self.tts.as_mut() {
            let _ = tts.stop();
        }
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<Option<SubmissionResult>, Box<dyn std::error::Error>> {
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
                RecvOutcome::Timeout => {}
                RecvOutcome::Disconnected => break,
            }
        }
        Ok(self.session.result().cloned())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Esc) {
            return true;
        }
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        match self.phase {
            Phase::Result => return self.handle_key_result(key),
            Phase::Playing => {}
        }

        match key.code {
            // Per `docs/spec.md`: `[Space] Replay sound`. The spec
            // doesn't reserve another key for "literal space inside a
            // phrase / sentence answer", so v0.2.0 foundation ships
            // word-only practice reliably; phrase / sentence input
            // mapping is revisited as part of the run-loop work in
            // #32-#37.
            KeyCode::Char(' ')
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.replay();
            }
            KeyCode::Enter => {}
            KeyCode::Backspace => {
                self.session.pop_char();
                self.clear_reject_flash();
            }
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

    fn handle_key_result(&mut self, key: KeyEvent) -> bool {
        matches!(key.code, KeyCode::Enter)
    }

    fn replay(&mut self) {
        if let Some(tts) = self.tts.as_mut() {
            if let Err(err) = tts.speak(&self.session.prompt().text, &self.language) {
                eprintln!("warning: TTS replay failed: {err}");
                return;
            }
            self.plays += 1;
        }
        // Restart the visual pulse on each replay so the breathing
        // anchors to the new utterance.
        self.pulse = Some(PulseHandle::start("♪", PulseOpts::default_listening()));
    }

    fn ui(&mut self, f: &mut Frame) {
        let frame = PaneFrame::hack(f.area());
        self.render_main_pane(f, frame.main);
        self.render_status_pane(f, frame.side);
        self.render_input_echo(f, frame.input_echo);
        if let Some(log) = frame.log {
            self.render_log_pane(f, log);
        }
        self.render_help_line(f, frame.help_line);
    }

    fn render_main_pane(&self, f: &mut Frame, area: Rect) {
        let title_text = match self.phase {
            Phase::Playing => "type-globe - Listening",
            Phase::Result => "type-globe - Listening",
        };

        let body_lines = match self.phase {
            Phase::Playing => self.playing_body_lines(),
            Phase::Result => self.result_body_lines(),
        };

        let para = Paragraph::new(body_lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(format!(" {title_text} "))
                    .title_style(STYLE_TITLE)
                    .borders(Borders::ALL)
                    .padding(Padding::uniform(1)),
            );
        f.render_widget(para, area);
    }

    fn playing_body_lines(&self) -> Vec<Line<'static>> {
        // Pulse symbol — color comes from the jiwa_core pulse so the
        // listening pane uses the same primitive as the spec example.
        let (symbol, color) = if let Some(pulse) = self.pulse.as_ref() {
            let frame = pulse.snapshot(Instant::now());
            (frame.text, frame.color)
        } else {
            ("♪".to_string(), Rgb(80, 200, 255))
        };
        let Rgb(r, g, b) = color;
        let symbol_span = Span::styled(symbol, Style::new().fg(Color::Rgb(r, g, b)));

        vec![
            Line::from(""),
            Line::from(""),
            Line::from(symbol_span),
            Line::from(""),
            Line::from(Span::styled("Listening...", STYLE_NORMAL)),
            Line::from(""),
            Line::from(Span::styled(
                "(audio only — exact match auto-confirms)",
                STYLE_DIM,
            )),
        ]
    }

    fn result_body_lines(&self) -> Vec<Line<'static>> {
        let Some(result) = self.session.result() else {
            return vec![Line::from("(no result)")];
        };
        let (verdict_text, verdict_style) = if result.is_correct {
            ("Correct!".to_string(), STYLE_CORRECT)
        } else {
            ("Wrong.".to_string(), STYLE_INCORRECT)
        };
        vec![
            Line::from(""),
            Line::from(Span::styled(verdict_text, verdict_style)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Expected: ", STYLE_LABEL),
                Span::styled(result.expected.clone(), STYLE_NORMAL),
            ]),
            Line::from(vec![
                Span::styled("You typed: ", STYLE_LABEL),
                Span::styled(self.session.input().to_string(), STYLE_INPUT_ECHO),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter or Esc to return to the menu.",
                STYLE_DIM,
            )),
        ]
    }

    fn render_status_pane(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.started_at.elapsed();
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;
        let kind = match self.session.prompt().kind {
            AnswerKind::Word => "word",
            AnswerKind::Phrase => "phrase",
            AnswerKind::Sentence => "sentence",
        };
        let lines = vec![
            Line::from(Span::styled("Practice", STYLE_LABEL)),
            Line::from(Span::styled("(RPG run: #32+)", STYLE_DIM)),
            Line::from(""),
            Line::from(format!("Kind   : {kind}")),
            Line::from(format!("Plays  : {}", self.plays)),
            Line::from(format!("Time   : {mins}:{secs:02}")),
        ];
        let para = Paragraph::new(lines).alignment(Alignment::Left).block(
            Block::default()
                .title(" Status ")
                .borders(Borders::ALL)
                .padding(Padding::uniform(1)),
        );
        f.render_widget(para, area);
    }

    fn render_input_echo(&self, f: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }
        let body = match self.phase {
            Phase::Playing => self.render_playing_input_line(),
            Phase::Result => Line::from(""),
        };
        f.render_widget(Paragraph::new(body).alignment(Alignment::Left), area);
    }

    fn render_log_pane(&self, f: &mut Frame, area: Rect) {
        let lines: Vec<Line<'static>> = match self.phase {
            Phase::Playing => vec![
                Line::from(Span::styled("(no events)", STYLE_DIM)),
                Line::from(Span::styled(
                    "Battle log fills in once #32-#37 land.",
                    STYLE_DIM,
                )),
            ],
            Phase::Result => match self.session.result() {
                Some(r) if r.is_correct => {
                    vec![
                        Line::from(Span::styled("▸ Hit!", STYLE_CORRECT)),
                        Line::from(Span::styled(
                            format!("▸ Plays this prompt: {}", self.plays),
                            STYLE_DIM,
                        )),
                    ]
                }
                Some(_) => {
                    vec![
                        Line::from(Span::styled("▸ Missed.", STYLE_INCORRECT)),
                        Line::from(Span::styled(
                            format!("▸ Plays this prompt: {}", self.plays),
                            STYLE_DIM,
                        )),
                    ]
                }
                None => vec![Line::from(Span::styled("(no result)", STYLE_DIM))],
            },
        };
        let para = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default().title(" Log ").borders(Borders::ALL));
        f.render_widget(para, area);
    }

    fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help = match self.phase {
            Phase::Playing => HelpLine::new(vec![
                HelpEntry::new("Esc", "Quit"),
                HelpEntry::new("Space", "Replay"),
                HelpEntry::new("Bksp", "Erase"),
            ]),
            Phase::Result => HelpLine::new(vec![HelpEntry::new("Enter", "Menu")]),
        };
        help.render(f, area);
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
            Span::styled(self.session.input().to_string(), STYLE_CORRECT),
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
        let mut attempted = self.session.input().to_string();
        attempted.push(c);
        if !is_valid_listening_prefix(&self.language, &attempted, &self.session.prompt().text) {
            self.note_rejected_char(c);
            return;
        }

        self.session.push_char(c);
        self.clear_reject_flash();

        let typed = self.session.input().to_lowercase();
        if acceptable_listening_inputs(&self.language, &self.session.prompt().text)
            .iter()
            .any(|candidate| candidate == &typed)
        {
            self.session.submit();
            self.pulse = None;
            self.phase = Phase::Result;
            if let Some(tts) = self.tts.as_mut() {
                let _ = tts.stop();
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
}

/// Build a one-line message for callers that want to surface a TTS
/// init failure to the player without crashing the run. Shared so the
/// menu and any future entry points format it consistently.
pub fn tts_unavailable_message(err: &dyn std::error::Error) -> String {
    format!(
        "Listening mode is unavailable on this system: {err}\n\
         (On Linux, install and start `speech-dispatcher`.)\n\
         Press Enter to return to the menu."
    )
}
